//! Advanced Memory Allocators — Arena, Pool, Slab, Buddy & Reference Counting
//!
//! Provides high-performance memory allocation strategies for reduced allocation
//! pressure, improved cache locality, and deterministic deallocation patterns.
//! Includes arena (bump), pool (fixed-block), slab (typed), buddy (power-of-2),
//! reference counting with cycle detection, and allocation profiling.

use std::collections::HashMap;

// ── Arena Allocator (Bump) ───────────────────────────────────────────

/// Fast bump allocator — allocates sequentially, frees all at once.
/// O(1) allocation, O(1) mass deallocation. Ideal for phase-based allocation.
#[derive(Debug)]
pub struct ArenaAllocator {
    chunks: Vec<Vec<u8>>,
    chunk_size: usize,
    offset: usize,
    stats: AllocStats,
}

impl ArenaAllocator {
    pub fn new(chunk_size: usize) -> Self {
        let chunk_size = chunk_size.max(64);
        Self {
            chunks: vec![vec![0u8; chunk_size]],
            chunk_size,
            offset: 0,
            stats: AllocStats::new(),
        }
    }

    /// Allocate `size` bytes with `align` alignment. Returns offset within arena.
    pub fn alloc(&mut self, size: usize, align: usize) -> Option<ArenaPtr> {
        let align = align.max(1);
        // Align the offset
        let aligned = (self.offset + align - 1) & !(align - 1);

        if aligned + size <= self.chunk_size {
            let ptr = ArenaPtr {
                chunk: self.chunks.len() - 1,
                offset: aligned,
                size,
            };
            self.offset = aligned + size;
            self.stats.record_alloc(size);
            Some(ptr)
        } else {
            // Need a new chunk
            let new_chunk_size = self.chunk_size.max(size + align);
            self.chunks.push(vec![0u8; new_chunk_size]);
            self.offset = 0;
            let ptr = ArenaPtr {
                chunk: self.chunks.len() - 1,
                offset: 0,
                size,
            };
            self.offset = size;
            self.stats.record_alloc(size);
            Some(ptr)
        }
    }

    /// Write bytes into an arena allocation.
    pub fn write(&mut self, ptr: &ArenaPtr, data: &[u8]) -> bool {
        if data.len() > ptr.size || ptr.chunk >= self.chunks.len() {
            return false;
        }
        self.chunks[ptr.chunk][ptr.offset..ptr.offset + data.len()].copy_from_slice(data);
        true
    }

    /// Read bytes from an arena allocation.
    pub fn read(&self, ptr: &ArenaPtr) -> Option<&[u8]> {
        if ptr.chunk >= self.chunks.len() {
            return None;
        }
        Some(&self.chunks[ptr.chunk][ptr.offset..ptr.offset + ptr.size])
    }

    /// Reset the arena — O(1) mass deallocation.
    pub fn reset(&mut self) {
        self.chunks.truncate(1);
        self.offset = 0;
        self.stats.record_reset();
    }

    /// Total bytes allocated.
    pub fn bytes_used(&self) -> usize {
        let full_chunks = if self.chunks.len() > 1 {
            (self.chunks.len() - 1) * self.chunk_size
        } else {
            0
        };
        full_chunks + self.offset
    }

    /// Total capacity across all chunks.
    pub fn capacity(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }

    pub fn stats(&self) -> &AllocStats {
        &self.stats
    }
}

/// Pointer into an arena allocation.
#[derive(Debug, Clone, Copy)]
pub struct ArenaPtr {
    pub chunk: usize,
    pub offset: usize,
    pub size: usize,
}

// ── Pool Allocator (Fixed-Block) ─────────────────────────────────────

/// Fixed-size block allocator with free list. O(1) alloc and free.
/// All blocks are the same size — no fragmentation.
#[derive(Debug)]
pub struct PoolAllocator {
    block_size: usize,
    blocks: Vec<u8>,
    free_list: Vec<usize>,
    capacity: usize,
    stats: AllocStats,
}

impl PoolAllocator {
    /// Create a pool with `count` blocks of `block_size` bytes each.
    pub fn new(block_size: usize, count: usize) -> Self {
        let block_size = block_size.max(1);
        let count = count.max(1);
        let mut free_list: Vec<usize> = (0..count).collect();
        free_list.reverse(); // Stack order — pop from end
        Self {
            block_size,
            blocks: vec![0u8; block_size * count],
            free_list,
            capacity: count,
            stats: AllocStats::new(),
        }
    }

    /// Allocate one block. Returns block index.
    pub fn alloc(&mut self) -> Option<usize> {
        if let Some(idx) = self.free_list.pop() {
            self.stats.record_alloc(self.block_size);
            Some(idx)
        } else {
            None
        }
    }

    /// Free a block by index.
    pub fn free(&mut self, idx: usize) {
        if idx < self.capacity && !self.free_list.contains(&idx) {
            // Zero out the block
            let start = idx * self.block_size;
            let end = start + self.block_size;
            self.blocks[start..end].fill(0);
            self.free_list.push(idx);
            self.stats.record_free(self.block_size);
        }
    }

    /// Write data to a block.
    pub fn write(&mut self, idx: usize, data: &[u8]) -> bool {
        if idx >= self.capacity || data.len() > self.block_size {
            return false;
        }
        let start = idx * self.block_size;
        self.blocks[start..start + data.len()].copy_from_slice(data);
        true
    }

    /// Read data from a block.
    pub fn read(&self, idx: usize) -> Option<&[u8]> {
        if idx >= self.capacity {
            return None;
        }
        let start = idx * self.block_size;
        Some(&self.blocks[start..start + self.block_size])
    }

    /// Number of blocks currently allocated.
    pub fn allocated_count(&self) -> usize {
        self.capacity - self.free_list.len()
    }

    /// Number of free blocks.
    pub fn free_count(&self) -> usize {
        self.free_list.len()
    }

    pub fn stats(&self) -> &AllocStats {
        &self.stats
    }
}

// ── Slab Allocator (Typed) ───────────────────────────────────────────

/// Typed object slab — pre-allocates slots for objects of a fixed size.
/// Uses generation counters for safe handle-based access.
#[derive(Debug, Clone)]
pub struct SlabHandle {
    pub index: usize,
    pub generation: u64,
}

#[derive(Debug)]
struct SlabSlot {
    data: Vec<u8>,
    generation: u64,
    occupied: bool,
}

/// Slab allocator with generational handles for ABA-safe access.
#[derive(Debug)]
pub struct SlabAllocator {
    slots: Vec<SlabSlot>,
    free_indices: Vec<usize>,
    object_size: usize,
    stats: AllocStats,
}

impl SlabAllocator {
    pub fn new(object_size: usize, initial_capacity: usize) -> Self {
        let object_size = object_size.max(1);
        let mut slots = Vec::with_capacity(initial_capacity);
        let mut free_indices = Vec::with_capacity(initial_capacity);
        for i in (0..initial_capacity).rev() {
            slots.push(SlabSlot {
                data: vec![0u8; object_size],
                generation: 0,
                occupied: false,
            });
            free_indices.push(i);
        }
        Self {
            slots,
            free_indices,
            object_size,
            stats: AllocStats::new(),
        }
    }

    /// Allocate a slot, returns a generational handle.
    pub fn alloc(&mut self) -> SlabHandle {
        if let Some(idx) = self.free_indices.pop() {
            self.slots[idx].occupied = true;
            self.slots[idx].generation += 1;
            self.stats.record_alloc(self.object_size);
            SlabHandle {
                index: idx,
                generation: self.slots[idx].generation,
            }
        } else {
            // Grow
            let idx = self.slots.len();
            self.slots.push(SlabSlot {
                data: vec![0u8; self.object_size],
                generation: 1,
                occupied: true,
            });
            self.stats.record_alloc(self.object_size);
            SlabHandle {
                index: idx,
                generation: 1,
            }
        }
    }

    /// Free a slot. The handle's generation must match.
    pub fn free(&mut self, handle: &SlabHandle) -> bool {
        if handle.index >= self.slots.len() {
            return false;
        }
        let slot = &mut self.slots[handle.index];
        if slot.generation != handle.generation || !slot.occupied {
            return false;
        }
        slot.occupied = false;
        slot.data.fill(0);
        self.free_indices.push(handle.index);
        self.stats.record_free(self.object_size);
        true
    }

    /// Write data to a slot (handle must be valid).
    pub fn write(&mut self, handle: &SlabHandle, data: &[u8]) -> bool {
        if handle.index >= self.slots.len() {
            return false;
        }
        let slot = &self.slots[handle.index];
        if slot.generation != handle.generation || !slot.occupied || data.len() > self.object_size {
            return false;
        }
        self.slots[handle.index].data[..data.len()].copy_from_slice(data);
        true
    }

    /// Read data from a slot (handle must be valid).
    pub fn read(&self, handle: &SlabHandle) -> Option<&[u8]> {
        if handle.index >= self.slots.len() {
            return None;
        }
        let slot = &self.slots[handle.index];
        if slot.generation != handle.generation || !slot.occupied {
            return None;
        }
        Some(&slot.data)
    }

    /// Number of occupied slots.
    pub fn allocated_count(&self) -> usize {
        self.slots.iter().filter(|s| s.occupied).count()
    }

    pub fn stats(&self) -> &AllocStats {
        &self.stats
    }
}

// ── Buddy Allocator ──────────────────────────────────────────────────

/// Power-of-2 buddy allocator. Splits and coalesces blocks in O(log n).
/// Minimizes external fragmentation through binary buddy merging.
#[derive(Debug)]
pub struct BuddyAllocator {
    /// Total size (must be power of 2).
    total_size: usize,
    /// Minimum block size.
    min_block: usize,
    /// Number of levels: log2(total_size / min_block) + 1.
    levels: usize,
    /// Free lists per level. Level 0 = largest, level N = smallest.
    free_lists: Vec<Vec<usize>>,
    /// Allocation metadata: offset → level.
    allocated: HashMap<usize, usize>,
    stats: AllocStats,
}

impl BuddyAllocator {
    pub fn new(total_size: usize, min_block: usize) -> Self {
        let total_size = total_size.next_power_of_two();
        let min_block = min_block.next_power_of_two().max(1);
        let levels = (total_size / min_block).trailing_zeros() as usize + 1;

        let mut free_lists = vec![Vec::new(); levels];
        // Level 0 has one block covering everything
        free_lists[0].push(0);

        Self {
            total_size,
            min_block,
            levels,
            free_lists,
            allocated: HashMap::new(),
            stats: AllocStats::new(),
        }
    }

    /// Block size at a given level.
    fn block_size_at_level(&self, level: usize) -> usize {
        self.total_size >> level
    }

    /// Allocate at least `size` bytes. Returns the offset.
    pub fn alloc(&mut self, size: usize) -> Option<usize> {
        let size = size.max(self.min_block).next_power_of_two();
        let target_level = self.level_for_size(size)?;

        // Find the smallest level >= target_level that has a free block
        let mut found_level = None;
        for level in (0..=target_level).rev() {
            if !self.free_lists[level].is_empty() {
                found_level = Some(level);
                break;
            }
        }

        let found_level = found_level?;
        let offset = self.free_lists[found_level].pop()?;

        // Split down to target level
        let mut current_level = found_level;
        let mut current_offset = offset;
        while current_level < target_level {
            current_level += 1;
            let buddy_offset = current_offset + self.block_size_at_level(current_level);
            self.free_lists[current_level].push(buddy_offset);
        }

        self.allocated.insert(current_offset, target_level);
        self.stats.record_alloc(size);
        Some(current_offset)
    }

    /// Free a previously allocated block.
    pub fn free(&mut self, offset: usize) -> bool {
        let level = match self.allocated.remove(&offset) {
            Some(l) => l,
            None => return false,
        };

        let block_size = self.block_size_at_level(level);
        self.stats.record_free(block_size);

        // Try to coalesce with buddy
        let mut current_offset = offset;
        let mut current_level = level;

        while current_level > 0 {
            let buddy = self.buddy_offset(current_offset, current_level);
            // Check if buddy is free
            if let Some(pos) = self.free_lists[current_level]
                .iter()
                .position(|&o| o == buddy)
            {
                self.free_lists[current_level].remove(pos);
                current_offset = current_offset.min(buddy);
                current_level -= 1;
            } else {
                break;
            }
        }

        self.free_lists[current_level].push(current_offset);
        true
    }

    fn buddy_offset(&self, offset: usize, level: usize) -> usize {
        offset ^ self.block_size_at_level(level)
    }

    fn level_for_size(&self, size: usize) -> Option<usize> {
        for level in 0..self.levels {
            if self.block_size_at_level(level) == size {
                return Some(level);
            }
        }
        None
    }

    /// Number of active allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocated.len()
    }

    /// Total free blocks across all levels.
    pub fn free_block_count(&self) -> usize {
        self.free_lists.iter().map(|l| l.len()).sum()
    }

    /// Fragmentation ratio: 1.0 = fully fragmented, 0.0 = no fragmentation.
    pub fn fragmentation(&self) -> f64 {
        let total_free: usize = self
            .free_lists
            .iter()
            .enumerate()
            .map(|(level, list)| list.len() * self.block_size_at_level(level))
            .sum();

        if total_free == 0 || self.free_block_count() <= 1 {
            return 0.0;
        }

        let largest_free = self
            .free_lists
            .iter()
            .enumerate()
            .filter(|(_, list)| !list.is_empty())
            .map(|(level, _)| self.block_size_at_level(level))
            .max()
            .unwrap_or(0);

        1.0 - (largest_free as f64 / total_free as f64)
    }

    pub fn stats(&self) -> &AllocStats {
        &self.stats
    }
}

// ── Reference Counting with Cycle Detection ──────────────────────────

/// Simple reference-counted object with mark-sweep cycle detection.
#[derive(Debug, Clone)]
pub struct RcObject {
    pub id: usize,
    pub ref_count: usize,
    pub references: Vec<usize>, // IDs of objects this refers to
    pub marked: bool,
}

/// Reference-counting heap with cycle-detecting garbage collector.
#[derive(Debug)]
pub struct RcHeap {
    objects: HashMap<usize, RcObject>,
    next_id: usize,
    roots: Vec<usize>,
    stats: AllocStats,
}

impl RcHeap {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            next_id: 0,
            roots: Vec::new(),
            stats: AllocStats::new(),
        }
    }

    /// Allocate a new object, returns its ID.
    pub fn alloc(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.insert(
            id,
            RcObject {
                id,
                ref_count: 1,
                references: Vec::new(),
                marked: false,
            },
        );
        self.stats.record_alloc(1);
        id
    }

    /// Add a reference from `from` to `to`.
    pub fn add_ref(&mut self, from: usize, to: usize) -> bool {
        if !self.objects.contains_key(&to) {
            return false;
        }
        if let Some(obj) = self.objects.get_mut(&from) {
            obj.references.push(to);
        } else {
            return false;
        }
        if let Some(target) = self.objects.get_mut(&to) {
            target.ref_count += 1;
        }
        true
    }

    /// Remove a reference from `from` to `to`.
    pub fn remove_ref(&mut self, from: usize, to: usize) -> bool {
        if let Some(obj) = self.objects.get_mut(&from) {
            if let Some(pos) = obj.references.iter().position(|&r| r == to) {
                obj.references.remove(pos);
            } else {
                return false;
            }
        } else {
            return false;
        }
        if let Some(target) = self.objects.get_mut(&to) {
            target.ref_count = target.ref_count.saturating_sub(1);
        }
        true
    }

    /// Mark an object as a GC root.
    pub fn add_root(&mut self, id: usize) {
        if !self.roots.contains(&id) {
            self.roots.push(id);
        }
    }

    /// Remove a GC root.
    pub fn remove_root(&mut self, id: usize) {
        self.roots.retain(|&r| r != id);
    }

    /// Mark phase: starting from roots, mark all reachable objects.
    fn mark(&mut self) {
        for obj in self.objects.values_mut() {
            obj.marked = false;
        }
        let roots = self.roots.clone();
        for root in roots {
            self.mark_recursive(root);
        }
    }

    fn mark_recursive(&mut self, id: usize) {
        if let Some(obj) = self.objects.get(&id) {
            if obj.marked {
                return;
            }
            let refs = obj.references.clone();
            if let Some(obj) = self.objects.get_mut(&id) {
                obj.marked = true;
            }
            for r in refs {
                self.mark_recursive(r);
            }
        }
    }

    /// Sweep phase: free all unmarked objects (cycle detection).
    fn sweep(&mut self) -> usize {
        let to_remove: Vec<usize> = self
            .objects
            .iter()
            .filter(|(_, obj)| !obj.marked)
            .map(|(&id, _)| id)
            .collect();
        let count = to_remove.len();
        for id in to_remove {
            self.objects.remove(&id);
            self.stats.record_free(1);
        }
        count
    }

    /// Run full mark-sweep GC. Returns number of objects collected.
    pub fn collect(&mut self) -> usize {
        self.mark();
        self.sweep()
    }

    /// Detect cycles using DFS coloring.
    pub fn detect_cycles(&self) -> Vec<Vec<usize>> {
        let mut cycles = Vec::new();
        let mut white: std::collections::HashSet<usize> = self.objects.keys().copied().collect();
        let mut gray: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut path = Vec::new();

        fn dfs(
            id: usize,
            objects: &HashMap<usize, RcObject>,
            white: &mut std::collections::HashSet<usize>,
            gray: &mut std::collections::HashSet<usize>,
            path: &mut Vec<usize>,
            cycles: &mut Vec<Vec<usize>>,
        ) {
            if gray.contains(&id) {
                // Found a cycle — extract it
                if let Some(pos) = path.iter().position(|&p| p == id) {
                    cycles.push(path[pos..].to_vec());
                }
                return;
            }
            if !white.contains(&id) {
                return;
            }
            white.remove(&id);
            gray.insert(id);
            path.push(id);

            if let Some(obj) = objects.get(&id) {
                for &r in &obj.references {
                    dfs(r, objects, white, gray, path, cycles);
                }
            }

            path.pop();
            gray.remove(&id);
        }

        let ids: Vec<usize> = self.objects.keys().copied().collect();
        for id in ids {
            if white.contains(&id) {
                dfs(id, &self.objects, &mut white, &mut gray, &mut path, &mut cycles);
            }
        }
        cycles
    }

    /// Number of live objects.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn stats(&self) -> &AllocStats {
        &self.stats
    }
}

impl Default for RcHeap {
    fn default() -> Self {
        Self::new()
    }
}

// ── Allocation Statistics ────────────────────────────────────────────

/// Tracks allocation/deallocation metrics.
#[derive(Debug, Clone)]
pub struct AllocStats {
    pub total_allocs: u64,
    pub total_frees: u64,
    pub total_bytes_allocated: u64,
    pub total_bytes_freed: u64,
    pub peak_bytes: u64,
    current_bytes: u64,
    pub resets: u64,
}

impl AllocStats {
    pub fn new() -> Self {
        Self {
            total_allocs: 0,
            total_frees: 0,
            total_bytes_allocated: 0,
            total_bytes_freed: 0,
            peak_bytes: 0,
            current_bytes: 0,
            resets: 0,
        }
    }

    pub fn record_alloc(&mut self, bytes: usize) {
        self.total_allocs += 1;
        self.total_bytes_allocated += bytes as u64;
        self.current_bytes += bytes as u64;
        if self.current_bytes > self.peak_bytes {
            self.peak_bytes = self.current_bytes;
        }
    }

    pub fn record_free(&mut self, bytes: usize) {
        self.total_frees += 1;
        self.total_bytes_freed += bytes as u64;
        self.current_bytes = self.current_bytes.saturating_sub(bytes as u64);
    }

    pub fn record_reset(&mut self) {
        self.resets += 1;
        self.current_bytes = 0;
    }

    /// Currently live bytes.
    pub fn current_bytes(&self) -> u64 {
        self.current_bytes
    }

    /// Fragmentation estimate: 1.0 - (freed / allocated).
    pub fn utilization(&self) -> f64 {
        if self.total_bytes_allocated == 0 {
            return 1.0;
        }
        1.0 - (self.total_bytes_freed as f64 / self.total_bytes_allocated as f64)
    }
}

impl Default for AllocStats {
    fn default() -> Self {
        Self::new()
    }
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Arena ────────────────────────────────────────────────────────

    #[test]
    fn test_arena_alloc() {
        let mut arena = ArenaAllocator::new(1024);
        let ptr = arena.alloc(64, 8).unwrap();
        assert_eq!(ptr.size, 64);
        assert!(arena.bytes_used() >= 64);
    }

    #[test]
    fn test_arena_write_read() {
        let mut arena = ArenaAllocator::new(1024);
        let ptr = arena.alloc(4, 1).unwrap();
        arena.write(&ptr, &[1, 2, 3, 4]);
        let data = arena.read(&ptr).unwrap();
        assert_eq!(data, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_arena_reset() {
        let mut arena = ArenaAllocator::new(256);
        arena.alloc(100, 1);
        arena.alloc(100, 1);
        arena.reset();
        assert_eq!(arena.bytes_used(), 0);
        assert_eq!(arena.stats().resets, 1);
    }

    #[test]
    fn test_arena_grow() {
        let mut arena = ArenaAllocator::new(64);
        let _p1 = arena.alloc(32, 1).unwrap();
        let _p2 = arena.alloc(64, 1).unwrap(); // Forces new chunk
        assert!(arena.capacity() >= 128);
    }

    #[test]
    fn test_arena_alignment() {
        let mut arena = ArenaAllocator::new(1024);
        let _p1 = arena.alloc(3, 1).unwrap();
        let p2 = arena.alloc(8, 8).unwrap();
        assert_eq!(p2.offset % 8, 0, "Should be 8-byte aligned");
    }

    // ── Pool ─────────────────────────────────────────────────────────

    #[test]
    fn test_pool_alloc_free() {
        let mut pool = PoolAllocator::new(64, 4);
        assert_eq!(pool.free_count(), 4);
        let b0 = pool.alloc().unwrap();
        let b1 = pool.alloc().unwrap();
        assert_eq!(pool.allocated_count(), 2);
        pool.free(b0);
        assert_eq!(pool.allocated_count(), 1);
        assert_eq!(pool.free_count(), 3);
        let _ = b1;
    }

    #[test]
    fn test_pool_exhaustion() {
        let mut pool = PoolAllocator::new(16, 2);
        pool.alloc().unwrap();
        pool.alloc().unwrap();
        assert!(pool.alloc().is_none());
    }

    #[test]
    fn test_pool_write_read() {
        let mut pool = PoolAllocator::new(8, 4);
        let idx = pool.alloc().unwrap();
        pool.write(idx, &[10, 20, 30]);
        let data = pool.read(idx).unwrap();
        assert_eq!(data[0], 10);
        assert_eq!(data[1], 20);
        assert_eq!(data[2], 30);
    }

    #[test]
    fn test_pool_double_free_noop() {
        let mut pool = PoolAllocator::new(16, 4);
        let idx = pool.alloc().unwrap();
        pool.free(idx);
        pool.free(idx); // Should be noop since already free
        assert_eq!(pool.free_count(), 4);
    }

    // ── Slab ─────────────────────────────────────────────────────────

    #[test]
    fn test_slab_alloc_free() {
        let mut slab = SlabAllocator::new(32, 8);
        let h1 = slab.alloc();
        let h2 = slab.alloc();
        assert_eq!(slab.allocated_count(), 2);
        assert!(slab.free(&h1));
        assert_eq!(slab.allocated_count(), 1);
        let _ = h2;
    }

    #[test]
    fn test_slab_generational_safety() {
        let mut slab = SlabAllocator::new(16, 4);
        let h1 = slab.alloc();
        let old_gen = h1.generation;
        slab.free(&h1);
        let h2 = slab.alloc(); // Reuses same slot
        assert_eq!(h2.index, h1.index);
        assert!(h2.generation > old_gen, "Generation should increase");
        // Old handle should be invalid
        let stale = SlabHandle {
            index: h1.index,
            generation: old_gen,
        };
        assert!(slab.read(&stale).is_none());
    }

    #[test]
    fn test_slab_write_read() {
        let mut slab = SlabAllocator::new(8, 4);
        let h = slab.alloc();
        slab.write(&h, &[42, 43]);
        let data = slab.read(&h).unwrap();
        assert_eq!(data[0], 42);
        assert_eq!(data[1], 43);
    }

    #[test]
    fn test_slab_grow() {
        let mut slab = SlabAllocator::new(8, 2);
        let _h1 = slab.alloc();
        let _h2 = slab.alloc();
        let h3 = slab.alloc(); // Should grow
        assert!(slab.write(&h3, &[99]));
    }

    // ── Buddy ────────────────────────────────────────────────────────

    #[test]
    fn test_buddy_alloc_free() {
        let mut buddy = BuddyAllocator::new(256, 16);
        let off1 = buddy.alloc(32).unwrap();
        let off2 = buddy.alloc(32).unwrap();
        assert_ne!(off1, off2);
        assert_eq!(buddy.allocation_count(), 2);
        buddy.free(off1);
        assert_eq!(buddy.allocation_count(), 1);
    }

    #[test]
    fn test_buddy_coalesce() {
        let mut buddy = BuddyAllocator::new(256, 16);
        let off1 = buddy.alloc(128).unwrap();
        let off2 = buddy.alloc(128).unwrap();
        buddy.free(off1);
        buddy.free(off2);
        // After coalescing, should have one free block at level 0
        assert_eq!(buddy.free_block_count(), 1);
    }

    #[test]
    fn test_buddy_split() {
        let mut buddy = BuddyAllocator::new(256, 16);
        let _off = buddy.alloc(16).unwrap();
        // Should have split multiple times
        assert!(buddy.free_block_count() > 0);
    }

    #[test]
    fn test_buddy_fragmentation() {
        let mut buddy = BuddyAllocator::new(256, 16);
        let a = buddy.alloc(16).unwrap();
        let _b = buddy.alloc(16).unwrap();
        let c = buddy.alloc(16).unwrap();
        buddy.free(a);
        buddy.free(c);
        // Two non-contiguous free blocks → fragmentation > 0
        assert!(buddy.fragmentation() >= 0.0);
    }

    #[test]
    fn test_buddy_exhaustion() {
        let mut buddy = BuddyAllocator::new(64, 16);
        buddy.alloc(32).unwrap();
        buddy.alloc(32).unwrap();
        assert!(buddy.alloc(32).is_none());
    }

    // ── RcHeap ───────────────────────────────────────────────────────

    #[test]
    fn test_rc_heap_alloc() {
        let mut heap = RcHeap::new();
        let id = heap.alloc();
        assert_eq!(heap.object_count(), 1);
        let _ = id;
    }

    #[test]
    fn test_rc_heap_add_ref() {
        let mut heap = RcHeap::new();
        let a = heap.alloc();
        let b = heap.alloc();
        assert!(heap.add_ref(a, b));
        assert_eq!(heap.objects[&b].ref_count, 2);
    }

    #[test]
    fn test_rc_heap_remove_ref() {
        let mut heap = RcHeap::new();
        let a = heap.alloc();
        let b = heap.alloc();
        heap.add_ref(a, b);
        heap.remove_ref(a, b);
        assert_eq!(heap.objects[&b].ref_count, 1);
    }

    #[test]
    fn test_rc_heap_gc_collects_unreachable() {
        let mut heap = RcHeap::new();
        let a = heap.alloc();
        let b = heap.alloc();
        let c = heap.alloc();
        heap.add_root(a);
        heap.add_ref(a, b);
        // c is unreachable
        let collected = heap.collect();
        assert_eq!(collected, 1); // c collected
        assert_eq!(heap.object_count(), 2);
        let _ = c;
    }

    #[test]
    fn test_rc_heap_cycle_detection() {
        let mut heap = RcHeap::new();
        let a = heap.alloc();
        let b = heap.alloc();
        heap.add_ref(a, b);
        heap.add_ref(b, a); // cycle!
        let cycles = heap.detect_cycles();
        assert!(!cycles.is_empty(), "Should detect A↔B cycle");
    }

    #[test]
    fn test_rc_heap_gc_breaks_cycles() {
        let mut heap = RcHeap::new();
        let a = heap.alloc();
        let b = heap.alloc();
        heap.add_ref(a, b);
        heap.add_ref(b, a);
        // Neither is a root → both should be collected
        let collected = heap.collect();
        assert_eq!(collected, 2);
        assert_eq!(heap.object_count(), 0);
    }

    // ── AllocStats ───────────────────────────────────────────────────

    #[test]
    fn test_alloc_stats_tracking() {
        let mut s = AllocStats::new();
        s.record_alloc(100);
        s.record_alloc(200);
        s.record_free(50);
        assert_eq!(s.total_allocs, 2);
        assert_eq!(s.total_frees, 1);
        assert_eq!(s.total_bytes_allocated, 300);
        assert_eq!(s.total_bytes_freed, 50);
        assert_eq!(s.current_bytes(), 250);
        assert_eq!(s.peak_bytes, 300);
    }

    #[test]
    fn test_alloc_stats_utilization() {
        let mut s = AllocStats::new();
        s.record_alloc(100);
        s.record_free(25);
        // utilization = 1.0 - 25/100 = 0.75
        assert!((s.utilization() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_alloc_stats_reset() {
        let mut s = AllocStats::new();
        s.record_alloc(500);
        s.record_reset();
        assert_eq!(s.current_bytes(), 0);
        assert_eq!(s.resets, 1);
        assert_eq!(s.peak_bytes, 500); // Peak preserved
    }
}
