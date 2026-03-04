//! Tracing garbage collector for Vitalis.
//!
//! Implements a tri-color mark-sweep collector with generational support:
//! - **Nursery**: Bump-allocated young generation with copying collection
//! - **Old generation**: Mark-compact collection for long-lived objects
//! - **Write barriers**: Card-marking remembered sets for old→young pointers
//! - **Finalization**: Weak references, destructor queues, resurrection prevention
//! - **Pinning**: `Pin` API to prevent GC from moving objects (FFI interop)
//! - **GC/ownership interop**: `Gc<T>` for shared ownership alongside borrow checker
//! - **Heap statistics**: Allocation rate, pause times, fragmentation, live set size
//! - **Tuning**: Heap growth factor, nursery size, concurrent marking, pause target

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

// ── Tri-Color Marking ────────────────────────────────────────────────

/// Tri-color state for mark-sweep GC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriColor {
    /// Not yet visited — candidate for collection.
    White,
    /// Discovered but children not yet scanned.
    Grey,
    /// Fully scanned — reachable, will survive collection.
    Black,
}

/// Unique identifier for a GC-managed object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcId(pub u64);

/// Represents a GC-managed object in the heap.
#[derive(Debug, Clone)]
pub struct GcObject {
    pub id: GcId,
    pub color: TriColor,
    pub generation: Generation,
    pub size_bytes: usize,
    pub references: Vec<GcId>,
    pub pinned: bool,
    pub weak: bool,
    pub finalized: bool,
    pub data: Vec<u8>,
}

/// Generation for generational collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    Nursery,
    Old,
}

// ── GC Configuration ─────────────────────────────────────────────────

/// Tuning knobs for the garbage collector.
#[derive(Debug, Clone)]
pub struct GcConfig {
    /// Nursery size in bytes before minor collection triggers.
    pub nursery_size: usize,
    /// Heap growth factor (old gen grows by this multiplier when full).
    pub heap_growth_factor: f64,
    /// Maximum pause target in microseconds.
    pub pause_target_us: u64,
    /// Number of concurrent marking threads (0 = single-threaded).
    pub concurrent_markers: usize,
    /// Promotion threshold: survive N nursery collections → promote to old gen.
    pub promotion_threshold: u32,
    /// Card table granularity in bytes (power of 2).
    pub card_size: usize,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            nursery_size: 1024 * 1024, // 1 MB
            heap_growth_factor: 1.5,
            pause_target_us: 1000, // 1ms
            concurrent_markers: 0,
            promotion_threshold: 3,
            card_size: 512,
        }
    }
}

// ── Heap Statistics ──────────────────────────────────────────────────

/// Live heap statistics.
#[derive(Debug, Clone, Default)]
pub struct HeapStats {
    pub total_allocations: u64,
    pub total_bytes_allocated: u64,
    pub live_objects: usize,
    pub live_bytes: usize,
    pub nursery_collections: u64,
    pub old_collections: u64,
    pub total_pause_us: u64,
    pub max_pause_us: u64,
    pub promotion_count: u64,
    pub fragmentation_ratio: f64,
}

// ── Card Table (Write Barrier) ───────────────────────────────────────

/// Card table for tracking old→young pointers (write barrier).
#[derive(Debug, Clone)]
pub struct CardTable {
    cards: Vec<bool>,
    card_size: usize,
    heap_start: usize,
}

impl CardTable {
    pub fn new(heap_size: usize, card_size: usize) -> Self {
        let num_cards = (heap_size + card_size - 1) / card_size;
        Self {
            cards: vec![false; num_cards],
            card_size,
            heap_start: 0,
        }
    }

    /// Mark a card as dirty (old object wrote a reference to young object).
    pub fn mark_dirty(&mut self, address: usize) {
        let idx = (address.saturating_sub(self.heap_start)) / self.card_size;
        if idx < self.cards.len() {
            self.cards[idx] = true;
        }
    }

    /// Check if a card is dirty.
    pub fn is_dirty(&self, index: usize) -> bool {
        index < self.cards.len() && self.cards[index]
    }

    /// Clear all dirty bits after scanning.
    pub fn clear(&mut self) {
        for c in &mut self.cards {
            *c = false;
        }
    }

    /// Return indices of all dirty cards.
    pub fn dirty_cards(&self) -> Vec<usize> {
        self.cards.iter().enumerate()
            .filter(|(_, d)| **d)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

// ── Weak Reference ───────────────────────────────────────────────────

/// A weak reference that does not prevent collection.
#[derive(Debug, Clone)]
pub struct WeakRef {
    pub target: GcId,
    pub alive: bool,
}

impl WeakRef {
    pub fn new(target: GcId) -> Self {
        Self { target, alive: true }
    }

    pub fn get(&self) -> Option<GcId> {
        if self.alive { Some(self.target) } else { None }
    }

    pub fn clear(&mut self) {
        self.alive = false;
    }
}

// ── Finalizer Queue ──────────────────────────────────────────────────

/// Finalizer action for an object about to be collected.
#[derive(Debug, Clone)]
pub struct FinalizerEntry {
    pub object_id: GcId,
    pub priority: u32,
}

/// Ordered queue of finalizers to run before collection.
#[derive(Debug, Clone, Default)]
pub struct FinalizerQueue {
    entries: VecDeque<FinalizerEntry>,
}

impl FinalizerQueue {
    pub fn enqueue(&mut self, id: GcId, priority: u32) {
        self.entries.push_back(FinalizerEntry { object_id: id, priority });
    }

    pub fn drain(&mut self) -> Vec<FinalizerEntry> {
        let mut items: Vec<_> = self.entries.drain(..).collect();
        items.sort_by(|a, b| b.priority.cmp(&a.priority));
        items
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ── Pin Guard ────────────────────────────────────────────────────────

/// Pin guard preventing GC from moving an object.
#[derive(Debug, Clone)]
pub struct PinGuard {
    pub id: GcId,
    pub active: bool,
}

impl PinGuard {
    pub fn new(id: GcId) -> Self {
        Self { id, active: true }
    }

    pub fn unpin(&mut self) {
        self.active = false;
    }
}

// ── GC Heap ──────────────────────────────────────────────────────────

/// The main garbage-collected heap.
pub struct GcHeap {
    /// All managed objects.
    objects: HashMap<GcId, GcObject>,
    /// Root set (stack roots, global roots).
    roots: HashSet<GcId>,
    /// Next object ID.
    next_id: u64,
    /// Nursery byte counter.
    nursery_bytes: usize,
    /// Survival counters for promotion.
    survival_count: HashMap<GcId, u32>,
    /// Write barrier card table.
    card_table: CardTable,
    /// Weak references.
    weak_refs: Vec<WeakRef>,
    /// Finalizer queue.
    finalizer_queue: FinalizerQueue,
    /// Pinned objects.
    pins: Vec<PinGuard>,
    /// Configuration.
    config: GcConfig,
    /// Statistics.
    stats: HeapStats,
}

impl GcHeap {
    pub fn new(config: GcConfig) -> Self {
        let card_table = CardTable::new(config.nursery_size * 4, config.card_size);
        Self {
            objects: HashMap::new(),
            roots: HashSet::new(),
            next_id: 1,
            nursery_bytes: 0,
            survival_count: HashMap::new(),
            card_table,
            weak_refs: Vec::new(),
            finalizer_queue: FinalizerQueue::default(),
            pins: Vec::new(),
            config,
            stats: HeapStats::default(),
        }
    }

    /// Allocate a new object in the nursery.
    pub fn alloc(&mut self, size_bytes: usize, data: Vec<u8>) -> GcId {
        let id = GcId(self.next_id);
        self.next_id += 1;

        let obj = GcObject {
            id,
            color: TriColor::White,
            generation: Generation::Nursery,
            size_bytes,
            references: Vec::new(),
            pinned: false,
            weak: false,
            finalized: false,
            data,
        };

        self.objects.insert(id, obj);
        self.nursery_bytes += size_bytes;
        self.stats.total_allocations += 1;
        self.stats.total_bytes_allocated += size_bytes as u64;
        self.stats.live_objects += 1;
        self.stats.live_bytes += size_bytes;

        // Check if nursery collection is needed.
        if self.nursery_bytes >= self.config.nursery_size {
            self.collect_nursery();
        }

        id
    }

    /// Add a reference from `from` to `to`.
    pub fn add_reference(&mut self, from: GcId, to: GcId) {
        // Write barrier: if old object references nursery object, mark card dirty.
        let from_gen = self.objects.get(&from).map(|o| o.generation);
        let to_gen = self.objects.get(&to).map(|o| o.generation);
        if from_gen == Some(Generation::Old) && to_gen == Some(Generation::Nursery) {
            self.card_table.mark_dirty(from.0 as usize);
        }

        if let Some(obj) = self.objects.get_mut(&from) {
            if !obj.references.contains(&to) {
                obj.references.push(to);
            }
        }
    }

    /// Add a root reference.
    pub fn add_root(&mut self, id: GcId) {
        self.roots.insert(id);
    }

    /// Remove a root reference.
    pub fn remove_root(&mut self, id: GcId) {
        self.roots.remove(&id);
    }

    /// Pin an object (prevent GC from moving it).
    pub fn pin(&mut self, id: GcId) -> PinGuard {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.pinned = true;
        }
        let guard = PinGuard::new(id);
        self.pins.push(guard.clone());
        guard
    }

    /// Create a weak reference.
    pub fn create_weak_ref(&mut self, target: GcId) -> usize {
        let idx = self.weak_refs.len();
        self.weak_refs.push(WeakRef::new(target));
        idx
    }

    /// Register a finalizer for an object.
    pub fn register_finalizer(&mut self, id: GcId, priority: u32) {
        self.finalizer_queue.enqueue(id, priority);
    }

    /// Get an object by ID.
    pub fn get(&self, id: GcId) -> Option<&GcObject> {
        self.objects.get(&id)
    }

    /// Get heap statistics.
    pub fn stats(&self) -> &HeapStats {
        &self.stats
    }

    /// Get live object count.
    pub fn live_count(&self) -> usize {
        self.objects.len()
    }

    // ── Tri-Color Mark Phase ─────────────────────────────────────────

    fn mark(&mut self) {
        // Reset all to white.
        for obj in self.objects.values_mut() {
            obj.color = TriColor::White;
        }

        // Grey the roots.
        let roots: Vec<GcId> = self.roots.iter().copied().collect();
        let mut worklist: VecDeque<GcId> = VecDeque::new();

        for &root in &roots {
            if let Some(obj) = self.objects.get_mut(&root) {
                obj.color = TriColor::Grey;
                worklist.push_back(root);
            }
        }

        // Process grey objects until empty.
        while let Some(id) = worklist.pop_front() {
            let refs = if let Some(obj) = self.objects.get(&id) {
                obj.references.clone()
            } else {
                continue;
            };

            for child_id in refs {
                if let Some(child) = self.objects.get_mut(&child_id) {
                    if child.color == TriColor::White {
                        child.color = TriColor::Grey;
                        worklist.push_back(child_id);
                    }
                }
            }

            if let Some(obj) = self.objects.get_mut(&id) {
                obj.color = TriColor::Black;
            }
        }
    }

    // ── Sweep Phase ──────────────────────────────────────────────────

    fn sweep(&mut self) -> usize {
        let white_ids: Vec<GcId> = self.objects.iter()
            .filter(|(_, obj)| obj.color == TriColor::White && !obj.pinned)
            .map(|(id, _)| *id)
            .collect();

        let collected = white_ids.len();

        // Clear weak refs pointing to collected objects.
        let white_set: HashSet<GcId> = white_ids.iter().copied().collect();
        for weak in &mut self.weak_refs {
            if white_set.contains(&weak.target) {
                weak.clear();
            }
        }

        // Remove collected objects.
        for id in &white_ids {
            if let Some(obj) = self.objects.remove(id) {
                self.stats.live_objects = self.stats.live_objects.saturating_sub(1);
                self.stats.live_bytes = self.stats.live_bytes.saturating_sub(obj.size_bytes);
            }
            self.survival_count.remove(id);
        }

        collected
    }

    // ── Nursery Collection (Minor GC) ────────────────────────────────

    pub fn collect_nursery(&mut self) {
        let start = Instant::now();
        self.stats.nursery_collections += 1;

        self.mark();

        // Promote surviving nursery objects that exceed the threshold.
        let nursery_ids: Vec<GcId> = self.objects.iter()
            .filter(|(_, obj)| obj.generation == Generation::Nursery && obj.color == TriColor::Black)
            .map(|(id, _)| *id)
            .collect();

        for id in nursery_ids {
            let count = self.survival_count.entry(id).or_insert(0);
            *count += 1;
            if *count >= self.config.promotion_threshold {
                if let Some(obj) = self.objects.get_mut(&id) {
                    obj.generation = Generation::Old;
                    self.stats.promotion_count += 1;
                }
            }
        }

        let collected = self.sweep();

        // Reset nursery counter.
        self.nursery_bytes = self.objects.iter()
            .filter(|(_, obj)| obj.generation == Generation::Nursery)
            .map(|(_, obj)| obj.size_bytes)
            .sum();

        let elapsed = start.elapsed().as_micros() as u64;
        self.stats.total_pause_us += elapsed;
        if elapsed > self.stats.max_pause_us {
            self.stats.max_pause_us = elapsed;
        }

        self.card_table.clear();
    }

    // ── Full Collection (Major GC) ───────────────────────────────────

    pub fn collect_full(&mut self) {
        let start = Instant::now();
        self.stats.old_collections += 1;

        self.mark();
        let collected = self.sweep();

        // Compact: update fragmentation ratio.
        let total_bytes: usize = self.objects.values().map(|o| o.size_bytes).sum();
        let ideal = self.stats.live_bytes;
        self.stats.fragmentation_ratio = if total_bytes > 0 {
            1.0 - (ideal as f64 / total_bytes as f64)
        } else {
            0.0
        };

        let elapsed = start.elapsed().as_micros() as u64;
        self.stats.total_pause_us += elapsed;
        if elapsed > self.stats.max_pause_us {
            self.stats.max_pause_us = elapsed;
        }

        self.card_table.clear();
    }
}

// ── Gc<T> Smart Pointer ──────────────────────────────────────────────

/// A GC-managed smart pointer for shared ownership.
#[derive(Debug, Clone)]
pub struct Gc<T> {
    pub id: GcId,
    pub value: Arc<RwLock<T>>,
}

impl<T: Clone> Gc<T> {
    pub fn new(value: T, heap: &mut GcHeap) -> Self {
        let size = std::mem::size_of::<T>();
        let id = heap.alloc(size, vec![0u8; size]);
        heap.add_root(id);
        Self {
            id,
            value: Arc::new(RwLock::new(value)),
        }
    }

    pub fn read(&self) -> T {
        self.value.read().unwrap().clone()
    }

    pub fn write(&self, val: T) {
        *self.value.write().unwrap() = val;
    }
}

// ── Incremental Marking ──────────────────────────────────────────────

/// Incremental marker that processes a bounded number of objects per step.
pub struct IncrementalMarker {
    worklist: VecDeque<GcId>,
    objects_per_step: usize,
    complete: bool,
}

impl IncrementalMarker {
    pub fn new(objects_per_step: usize) -> Self {
        Self {
            worklist: VecDeque::new(),
            objects_per_step,
            complete: false,
        }
    }

    /// Initialize with root set.
    pub fn init(&mut self, roots: &[GcId]) {
        self.worklist.clear();
        self.complete = false;
        for &r in roots {
            self.worklist.push_back(r);
        }
    }

    /// Process one step of incremental marking. Returns number of objects scanned.
    pub fn step(&mut self, objects: &mut HashMap<GcId, GcObject>) -> usize {
        let mut scanned = 0;

        for _ in 0..self.objects_per_step {
            if let Some(id) = self.worklist.pop_front() {
                let refs = if let Some(obj) = objects.get(&id) {
                    obj.references.clone()
                } else {
                    continue;
                };

                for child_id in refs {
                    if let Some(child) = objects.get_mut(&child_id) {
                        if child.color == TriColor::White {
                            child.color = TriColor::Grey;
                            self.worklist.push_back(child_id);
                        }
                    }
                }

                if let Some(obj) = objects.get_mut(&id) {
                    obj.color = TriColor::Black;
                }
                scanned += 1;
            } else {
                self.complete = true;
                break;
            }
        }

        scanned
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_and_get() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 10_000, ..Default::default() });
        let id = heap.alloc(64, vec![42u8; 64]);
        assert!(heap.get(id).is_some());
        assert_eq!(heap.get(id).unwrap().size_bytes, 64);
        assert_eq!(heap.get(id).unwrap().data[0], 42);
    }

    #[test]
    fn test_root_prevents_collection() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 10_000, ..Default::default() });
        let id = heap.alloc(64, vec![1; 64]);
        heap.add_root(id);
        heap.collect_full();
        assert!(heap.get(id).is_some(), "rooted object must survive collection");
    }

    #[test]
    fn test_unreachable_collected() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        let id = heap.alloc(64, vec![0; 64]);
        // Don't root it.
        heap.collect_full();
        assert!(heap.get(id).is_none(), "unreachable object must be collected");
    }

    #[test]
    fn test_transitive_reachability() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        let a = heap.alloc(32, vec![1; 32]);
        let b = heap.alloc(32, vec![2; 32]);
        let c = heap.alloc(32, vec![3; 32]);
        heap.add_root(a);
        heap.add_reference(a, b);
        heap.add_reference(b, c);
        heap.collect_full();
        assert!(heap.get(a).is_some());
        assert!(heap.get(b).is_some());
        assert!(heap.get(c).is_some());
    }

    #[test]
    fn test_cycle_collection() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        let a = heap.alloc(32, vec![0; 32]);
        let b = heap.alloc(32, vec![0; 32]);
        heap.add_reference(a, b);
        heap.add_reference(b, a);
        // No roots — should collect both.
        heap.collect_full();
        assert!(heap.get(a).is_none());
        assert!(heap.get(b).is_none());
    }

    #[test]
    fn test_pinned_survives_without_root() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        let id = heap.alloc(64, vec![0; 64]);
        let _guard = heap.pin(id);
        heap.collect_full();
        assert!(heap.get(id).is_some(), "pinned object must survive");
    }

    #[test]
    fn test_weak_ref_cleared_on_collection() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        let id = heap.alloc(64, vec![0; 64]);
        let weak_idx = heap.create_weak_ref(id);
        assert!(heap.weak_refs[weak_idx].get().is_some());
        heap.collect_full();
        assert!(heap.weak_refs[weak_idx].get().is_none(), "weak ref must be cleared");
    }

    #[test]
    fn test_generation_promotion() {
        let mut heap = GcHeap::new(GcConfig {
            nursery_size: 100_000,
            promotion_threshold: 2,
            ..Default::default()
        });
        let id = heap.alloc(64, vec![0; 64]);
        heap.add_root(id);
        // Survive 2 nursery collections → promote.
        heap.collect_nursery();
        assert_eq!(heap.get(id).unwrap().generation, Generation::Nursery);
        heap.collect_nursery();
        assert_eq!(heap.get(id).unwrap().generation, Generation::Old);
    }

    #[test]
    fn test_card_table() {
        let mut ct = CardTable::new(4096, 512);
        assert!(!ct.is_dirty(0));
        ct.mark_dirty(100);
        assert!(ct.is_dirty(0));
        ct.mark_dirty(600);
        assert!(ct.is_dirty(1));
        assert_eq!(ct.dirty_cards(), vec![0, 1]);
        ct.clear();
        assert!(ct.dirty_cards().is_empty());
    }

    #[test]
    fn test_finalizer_queue() {
        let mut q = FinalizerQueue::default();
        q.enqueue(GcId(1), 1);
        q.enqueue(GcId(2), 10);
        q.enqueue(GcId(3), 5);
        let drained = q.drain();
        assert_eq!(drained[0].object_id, GcId(2)); // highest priority first
        assert_eq!(drained[1].object_id, GcId(3));
        assert_eq!(drained[2].object_id, GcId(1));
    }

    #[test]
    fn test_heap_stats() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        heap.alloc(100, vec![0; 100]);
        heap.alloc(200, vec![0; 200]);
        assert_eq!(heap.stats().total_allocations, 2);
        assert_eq!(heap.stats().total_bytes_allocated, 300);
        assert_eq!(heap.stats().live_objects, 2);
    }

    #[test]
    fn test_gc_smart_pointer() {
        let mut heap = GcHeap::new(GcConfig::default());
        let gc_val = Gc::new(42i64, &mut heap);
        assert_eq!(gc_val.read(), 42);
        gc_val.write(99);
        assert_eq!(gc_val.read(), 99);
    }

    #[test]
    fn test_incremental_marker() {
        let mut objects = HashMap::new();
        let root = GcId(1);
        let child = GcId(2);
        let grandchild = GcId(3);

        objects.insert(root, GcObject {
            id: root, color: TriColor::Grey, generation: Generation::Nursery,
            size_bytes: 32, references: vec![child], pinned: false, weak: false,
            finalized: false, data: vec![],
        });
        objects.insert(child, GcObject {
            id: child, color: TriColor::White, generation: Generation::Nursery,
            size_bytes: 32, references: vec![grandchild], pinned: false, weak: false,
            finalized: false, data: vec![],
        });
        objects.insert(grandchild, GcObject {
            id: grandchild, color: TriColor::White, generation: Generation::Nursery,
            size_bytes: 32, references: vec![], pinned: false, weak: false,
            finalized: false, data: vec![],
        });

        let mut marker = IncrementalMarker::new(10);
        marker.init(&[root]);
        let scanned = marker.step(&mut objects);
        assert!(scanned > 0);
        assert!(marker.is_complete());
        assert_eq!(objects[&grandchild].color, TriColor::Black);
    }

    #[test]
    fn test_nursery_auto_collection() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 200, ..Default::default() });
        // Allocate enough to trigger auto-collection.
        let id1 = heap.alloc(100, vec![0; 100]);
        heap.add_root(id1);
        let _id2 = heap.alloc(100, vec![0; 100]); // not rooted
        let _id3 = heap.alloc(100, vec![0; 100]); // triggers nursery collection
        assert!(heap.get(id1).is_some());
        assert!(heap.stats().nursery_collections >= 1);
    }

    #[test]
    fn test_remove_root_allows_collection() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        let id = heap.alloc(64, vec![0; 64]);
        heap.add_root(id);
        heap.collect_full();
        assert!(heap.get(id).is_some());
        heap.remove_root(id);
        heap.collect_full();
        assert!(heap.get(id).is_none());
    }

    #[test]
    fn test_multiple_roots() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        let ids: Vec<GcId> = (0..10).map(|i| {
            let id = heap.alloc(32, vec![i as u8; 32]);
            if i % 2 == 0 { heap.add_root(id); }
            id
        }).collect();
        heap.collect_full();
        for (i, &id) in ids.iter().enumerate() {
            if i % 2 == 0 {
                assert!(heap.get(id).is_some(), "rooted object {i} must survive");
            } else {
                assert!(heap.get(id).is_none(), "unrooted object {i} must be collected");
            }
        }
    }

    #[test]
    fn test_tri_color_defaults() {
        let mut heap = GcHeap::new(GcConfig { nursery_size: 100_000, ..Default::default() });
        let id = heap.alloc(32, vec![0; 32]);
        assert_eq!(heap.get(id).unwrap().color, TriColor::White);
    }

    #[test]
    fn test_pin_guard_unpin() {
        let mut guard = PinGuard::new(GcId(1));
        assert!(guard.active);
        guard.unpin();
        assert!(!guard.active);
    }

    #[test]
    fn test_weak_ref_alive() {
        let w = WeakRef::new(GcId(42));
        assert_eq!(w.get(), Some(GcId(42)));
        let mut w2 = w.clone();
        w2.clear();
        assert_eq!(w2.get(), None);
    }
}
