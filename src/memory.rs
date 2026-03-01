//! Memory Primitives — Native memory management for Vitalis.
//!
//! This module implements the memory layer for Vitalis's self-evolving system.
//! Unlike external databases, these are *native* memory operations that run at
//! Rust speed with sophisticated retrieval, decay, and consolidation.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────┐
//! │                    Memory System                          │
//! │                                                           │
//! │  ┌────────────────┐  ┌────────────────┐                   │
//! │  │  Engram Store   │  │ Pattern Index  │                   │
//! │  │  (episodic)     │  │  (recurring)   │                   │
//! │  │                 │  │                │                   │
//! │  │  store()        │  │  detect()      │                   │
//! │  │  recall()       │  │  frequency()   │                   │
//! │  │  forget()       │  │  correlate()   │                   │
//! │  └────────┬───────┘  └───────┬────────┘                   │
//! │           │                  │                             │
//! │  ┌────────▼──────────────────▼────────┐                   │
//! │  │         Consolidation Engine       │                   │
//! │  │  decay()  merge()  compress()      │                   │
//! │  │  importance_score()  prune()       │                   │
//! │  └────────────────────────────────────┘                   │
//! └───────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════
//  ENGRAM — a single memory trace
// ═══════════════════════════════════════════════════════════════════════

/// An engram — the fundamental unit of memory.
/// Named after the hypothetical physical trace left by learning in the brain.
#[derive(Debug, Clone)]
pub struct Engram {
    /// Unique identifier
    pub id: u64,
    /// What kind of memory is this?
    pub kind: EngramKind,
    /// The content of the memory
    pub content: String,
    /// Tags for retrieval
    pub tags: Vec<String>,
    /// When this was stored (cycle number)
    pub stored_at: u64,
    /// When this was last accessed
    pub last_accessed: u64,
    /// How many times this has been recalled
    pub access_count: u64,
    /// Importance score (0.0 = trivial, 1.0 = critical)
    pub importance: f64,
    /// Current strength (decays over time, strengthens on recall)
    pub strength: f64,
    /// Association links to other engrams
    pub associations: Vec<u64>,
    /// Context when stored (e.g., what function, what signal)
    pub context: String,
}

/// Categories of memory.
#[derive(Debug, Clone, PartialEq)]
pub enum EngramKind {
    /// An event that happened — "I evolved function X at cycle N"
    Episodic,
    /// A learned fact — "function X works best with approach Y"
    Semantic,
    /// A skill pattern — "when I see pattern X, do Y"
    Procedural,
    /// A working memory item — temporary, high-importance
    Working,
    /// An emotional memory — how something made the system "feel"
    Emotional,
}

impl EngramKind {
    pub fn name(&self) -> &'static str {
        match self {
            EngramKind::Episodic => "episodic",
            EngramKind::Semantic => "semantic",
            EngramKind::Procedural => "procedural",
            EngramKind::Working => "working",
            EngramKind::Emotional => "emotional",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  MEMORY STORE — the engram database
// ═══════════════════════════════════════════════════════════════════════

/// The memory store — manages all engrams with storage, retrieval, decay.
pub struct MemoryStore {
    /// All engrams indexed by ID
    engrams: HashMap<u64, Engram>,
    /// Tag index for fast retrieval
    tag_index: HashMap<String, Vec<u64>>,
    /// Kind index
    kind_index: HashMap<String, Vec<u64>>,
    /// Next engram ID
    next_id: u64,
    /// Total engrams ever stored (including forgotten)
    total_stored: u64,
    /// Total engrams forgotten
    total_forgotten: u64,
    /// Total recalls performed
    total_recalls: u64,
    /// Consolidation count
    consolidations: u64,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            engrams: HashMap::new(),
            tag_index: HashMap::new(),
            kind_index: HashMap::new(),
            next_id: 1,
            total_stored: 0,
            total_forgotten: 0,
            total_recalls: 0,
            consolidations: 0,
        }
    }

    /// Store a new engram. Returns the engram ID.
    pub fn store(&mut self, kind: EngramKind, content: &str, tags: &[&str],
                 importance: f64, context: &str, cycle: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.total_stored += 1;

        let tag_strings: Vec<String> = tags.iter().map(|t| t.to_string()).collect();

        let engram = Engram {
            id,
            kind: kind.clone(),
            content: content.to_string(),
            tags: tag_strings.clone(),
            stored_at: cycle,
            last_accessed: cycle,
            access_count: 0,
            importance,
            strength: 1.0,
            associations: Vec::new(),
            context: context.to_string(),
        };

        // Update indices
        for tag in &tag_strings {
            self.tag_index.entry(tag.clone()).or_default().push(id);
        }
        self.kind_index.entry(kind.name().to_string()).or_default().push(id);

        self.engrams.insert(id, engram);
        id
    }

    /// Recall engrams by tag. Strengthens recalled memories.
    pub fn recall_by_tag(&mut self, tag: &str, current_cycle: u64) -> Vec<&Engram> {
        self.total_recalls += 1;

        let ids: Vec<u64> = self.tag_index.get(tag)
            .map(|v| v.clone())
            .unwrap_or_default();

        // Strengthen recalled memories
        for &id in &ids {
            if let Some(engram) = self.engrams.get_mut(&id) {
                engram.access_count += 1;
                engram.last_accessed = current_cycle;
                // Spaced repetition effect: strength increases with recall
                engram.strength = (engram.strength + 0.1).min(1.0);
            }
        }

        // Return sorted by importance * strength
        let mut results: Vec<&Engram> = ids.iter()
            .filter_map(|id| self.engrams.get(id))
            .collect();
        results.sort_by(|a, b| {
            let score_a = a.importance * a.strength;
            let score_b = b.importance * b.strength;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Recall engrams by kind.
    pub fn recall_by_kind(&mut self, kind: &EngramKind, current_cycle: u64) -> Vec<&Engram> {
        self.total_recalls += 1;

        let ids: Vec<u64> = self.kind_index.get(kind.name())
            .map(|v| v.clone())
            .unwrap_or_default();

        for &id in &ids {
            if let Some(engram) = self.engrams.get_mut(&id) {
                engram.access_count += 1;
                engram.last_accessed = current_cycle;
                engram.strength = (engram.strength + 0.05).min(1.0);
            }
        }

        let mut results: Vec<&Engram> = ids.iter()
            .filter_map(|id| self.engrams.get(id))
            .collect();
        results.sort_by(|a, b| {
            let score_a = a.importance * a.strength;
            let score_b = b.importance * b.strength;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Recall the most important memories. Returns top N by importance * strength.
    pub fn recall_top(&mut self, n: usize, current_cycle: u64) -> Vec<&Engram> {
        self.total_recalls += 1;

        // Touch accessed memories
        let mut scored: Vec<(u64, f64)> = self.engrams.iter()
            .map(|(&id, e)| (id, e.importance * e.strength))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(n);

        for &(id, _) in &scored {
            if let Some(engram) = self.engrams.get_mut(&id) {
                engram.access_count += 1;
                engram.last_accessed = current_cycle;
            }
        }

        scored.iter()
            .filter_map(|(id, _)| self.engrams.get(id))
            .collect()
    }

    /// Forget a specific engram by ID.
    pub fn forget(&mut self, id: u64) -> bool {
        if let Some(engram) = self.engrams.remove(&id) {
            self.total_forgotten += 1;

            // Clean up indices
            for tag in &engram.tags {
                if let Some(ids) = self.tag_index.get_mut(tag) {
                    ids.retain(|&eid| eid != id);
                }
            }
            if let Some(ids) = self.kind_index.get_mut(engram.kind.name()) {
                ids.retain(|&eid| eid != id);
            }

            true
        } else {
            false
        }
    }

    /// Apply time-based decay to all memories.
    /// Older, less-accessed, less-important memories fade.
    pub fn decay(&mut self, current_cycle: u64) {
        let mut to_forget = Vec::new();

        for (&id, engram) in self.engrams.iter_mut() {
            let age = current_cycle.saturating_sub(engram.stored_at) as f64;
            let recency = current_cycle.saturating_sub(engram.last_accessed) as f64;

            // Ebbinghaus-inspired forgetting curve
            // strength *= e^(-recency / (access_count + 1) * decay_rate)
            let decay_rate = match engram.kind {
                EngramKind::Working => 2.0,      // Working memory decays fast
                EngramKind::Episodic => 0.5,      // Episodes fade slowly
                EngramKind::Semantic => 0.1,      // Facts are stable
                EngramKind::Procedural => 0.05,   // Skills barely fade
                EngramKind::Emotional => 0.3,     // Emotional memories are sticky
            };

            let retention_factor = (engram.access_count as f64 + 1.0).ln() + 1.0;
            let decay = (-recency / (retention_factor * 100.0) * decay_rate).exp();
            engram.strength *= decay;

            // Importance protects from decay
            engram.strength = engram.strength.max(engram.importance * 0.1);

            // Forget memories that have effectively faded to nothing
            if engram.strength < 0.01 && engram.importance < 0.2 {
                to_forget.push(id);
            }

            // Also forget very old working memories
            if engram.kind == EngramKind::Working && age > 100.0 {
                to_forget.push(id);
            }
        }

        for id in to_forget {
            self.forget(id);
        }
    }

    /// Consolidate memories — merge similar engrams, compress content.
    pub fn consolidate(&mut self, current_cycle: u64) -> ConsolidationResult {
        self.consolidations += 1;
        let mut merged = 0;
        let mut pruned = 0;
        let mut strengthened = 0;

        // 1. Prune dead memories (strength < 0.05)
        let to_prune: Vec<u64> = self.engrams.iter()
            .filter(|(_, e)| e.strength < 0.05)
            .map(|(&id, _)| id)
            .collect();
        for id in to_prune {
            self.forget(id);
            pruned += 1;
        }

        // 2. Merge similar episodic memories (same tags, similar content)
        let episodic_ids: Vec<u64> = self.engrams.iter()
            .filter(|(_, e)| e.kind == EngramKind::Episodic)
            .map(|(&id, _)| id)
            .collect();

        let mut merge_pairs: Vec<(u64, u64)> = Vec::new();
        for i in 0..episodic_ids.len() {
            for j in (i + 1)..episodic_ids.len() {
                let a = &self.engrams[&episodic_ids[i]];
                let b = &self.engrams[&episodic_ids[j]];
                // Merge if they share > 50% of tags
                let shared_tags = a.tags.iter()
                    .filter(|t| b.tags.contains(t))
                    .count();
                let total_tags = a.tags.len().max(b.tags.len()).max(1);
                if shared_tags as f64 / total_tags as f64 > 0.5 && a.tags.len() > 0 {
                    merge_pairs.push((episodic_ids[i], episodic_ids[j]));
                }
            }
            // Limit merge operations
            if merge_pairs.len() > 20 { break; }
        }

        for (keep_id, remove_id) in merge_pairs {
            if let (Some(_keep), Some(remove)) = (
                self.engrams.get(&keep_id).cloned(),
                self.engrams.get(&remove_id).cloned(),
            ) {
                // Merge into the more important one
                if let Some(survivor) = self.engrams.get_mut(&keep_id) {
                    survivor.importance = survivor.importance.max(remove.importance);
                    survivor.strength = (survivor.strength + remove.strength) / 2.0;
                    survivor.access_count += remove.access_count;
                    survivor.content = format!("{} [+consolidated]", survivor.content);

                    // Add associations
                    for assoc in &remove.associations {
                        if !survivor.associations.contains(assoc) {
                            survivor.associations.push(*assoc);
                        }
                    }

                    // Merge unique tags
                    for tag in &remove.tags {
                        if !survivor.tags.contains(tag) {
                            survivor.tags.push(tag.clone());
                        }
                    }

                    survivor.last_accessed = current_cycle;
                    merged += 1;
                }
                self.forget(remove_id);
            }
        }

        // 3. Strengthen frequently accessed memories
        for engram in self.engrams.values_mut() {
            if engram.access_count > 10 {
                engram.strength = (engram.strength + 0.01).min(1.0);
                strengthened += 1;
            }
        }

        ConsolidationResult {
            merged,
            pruned,
            strengthened,
            total_remaining: self.engrams.len(),
            consolidation_number: self.consolidations,
        }
    }

    /// Add an association link between two engrams.
    pub fn associate(&mut self, id_a: u64, id_b: u64) {
        if let Some(a) = self.engrams.get_mut(&id_a) {
            if !a.associations.contains(&id_b) {
                a.associations.push(id_b);
            }
        }
        if let Some(b) = self.engrams.get_mut(&id_b) {
            if !b.associations.contains(&id_a) {
                b.associations.push(id_a);
            }
        }
    }

    /// Find engrams associated with a given engram.
    pub fn recall_associations(&self, id: u64) -> Vec<&Engram> {
        self.engrams.get(&id)
            .map(|e| {
                e.associations.iter()
                    .filter_map(|assoc_id| self.engrams.get(assoc_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get memory statistics.
    pub fn stats_json(&self) -> String {
        let by_kind: HashMap<String, usize> = self.engrams.iter()
            .fold(HashMap::new(), |mut acc, (_, e)| {
                *acc.entry(e.kind.name().to_string()).or_insert(0) += 1;
                acc
            });

        let avg_strength = if !self.engrams.is_empty() {
            self.engrams.values().map(|e| e.strength).sum::<f64>() / self.engrams.len() as f64
        } else {
            0.0
        };

        let avg_importance = if !self.engrams.is_empty() {
            self.engrams.values().map(|e| e.importance).sum::<f64>() / self.engrams.len() as f64
        } else {
            0.0
        };

        let kind_json: Vec<String> = by_kind.iter()
            .map(|(k, v)| format!("\"{}\":{}", k, v))
            .collect();

        format!(
            concat!(
                "{{",
                "\"total_stored\":{},",
                "\"total_forgotten\":{},",
                "\"total_recalls\":{},",
                "\"active_engrams\":{},",
                "\"consolidations\":{},",
                "\"avg_strength\":{:.4},",
                "\"avg_importance\":{:.4},",
                "\"by_kind\":{{{}}}",
                "}}"
            ),
            self.total_stored,
            self.total_forgotten,
            self.total_recalls,
            self.engrams.len(),
            self.consolidations,
            avg_strength,
            avg_importance,
            kind_json.join(","),
        )
    }

    /// Total active engrams.
    pub fn count(&self) -> usize {
        self.engrams.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  CONSOLIDATION RESULT
// ═══════════════════════════════════════════════════════════════════════

/// Result of a memory consolidation cycle.
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    pub merged: usize,
    pub pruned: usize,
    pub strengthened: usize,
    pub total_remaining: usize,
    pub consolidation_number: u64,
}

impl ConsolidationResult {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"merged\":{},\"pruned\":{},\"strengthened\":{},\"remaining\":{},\"consolidation\":{}}}",
            self.merged, self.pruned, self.strengthened,
            self.total_remaining, self.consolidation_number
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  PATTERN DETECTOR — finds recurring patterns in memory
// ═══════════════════════════════════════════════════════════════════════

/// A detected pattern — a recurring theme in memories.
#[derive(Debug, Clone)]
pub struct DetectedPattern {
    /// The pattern identifier (tag or content fragment)
    pub pattern: String,
    /// How many times this pattern appears
    pub frequency: usize,
    /// The engram IDs that contain this pattern
    pub engram_ids: Vec<u64>,
    /// Confidence that this is a real pattern (vs. noise)
    pub confidence: f64,
}

/// Detect patterns in the memory store.
pub fn detect_patterns(store: &MemoryStore) -> Vec<DetectedPattern> {
    let mut tag_freq: HashMap<String, Vec<u64>> = HashMap::new();

    for (&id, engram) in &store.engrams {
        for tag in &engram.tags {
            tag_freq.entry(tag.clone()).or_default().push(id);
        }
    }

    let mut patterns: Vec<DetectedPattern> = tag_freq.into_iter()
        .filter(|(_, ids)| ids.len() >= 3) // At least 3 occurrences
        .map(|(tag, ids)| {
            let frequency = ids.len();
            let confidence = (frequency as f64 / store.engrams.len().max(1) as f64).min(1.0);
            DetectedPattern {
                pattern: tag,
                frequency,
                engram_ids: ids,
                confidence,
            }
        })
        .collect();

    patterns.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    patterns.truncate(20); // Top 20 patterns
    patterns
}

// ═══════════════════════════════════════════════════════════════════════
//  GLOBAL MEMORY STORE (thread-local)
// ═══════════════════════════════════════════════════════════════════════

use std::cell::RefCell;

thread_local! {
    static GLOBAL_MEMORY: RefCell<MemoryStore> = RefCell::new(MemoryStore::new());
}

/// Access the global memory store.
pub fn with_memory<F, R>(f: F) -> R
where
    F: FnOnce(&mut MemoryStore) -> R,
{
    GLOBAL_MEMORY.with(|m| f(&mut m.borrow_mut()))
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_count() {
        let mut store = MemoryStore::new();
        let id = store.store(
            EngramKind::Episodic,
            "Evolved function alpha successfully",
            &["evolution", "alpha", "success"],
            0.7,
            "cycle 5",
            5,
        );
        assert_eq!(id, 1);
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn test_store_multiple_kinds() {
        let mut store = MemoryStore::new();
        store.store(EngramKind::Episodic, "event 1", &["a"], 0.5, "", 1);
        store.store(EngramKind::Semantic, "fact 1", &["b"], 0.8, "", 2);
        store.store(EngramKind::Procedural, "skill 1", &["c"], 0.9, "", 3);
        store.store(EngramKind::Working, "temp 1", &["d"], 0.3, "", 4);
        store.store(EngramKind::Emotional, "feeling 1", &["e"], 0.6, "", 5);
        assert_eq!(store.count(), 5);
    }

    #[test]
    fn test_recall_by_tag() {
        let mut store = MemoryStore::new();
        store.store(EngramKind::Episodic, "event A", &["evolution"], 0.5, "", 1);
        store.store(EngramKind::Episodic, "event B", &["evolution"], 0.8, "", 2);
        store.store(EngramKind::Episodic, "event C", &["compile"], 0.3, "", 3);

        let recalled = store.recall_by_tag("evolution", 5);
        assert_eq!(recalled.len(), 2);
        // Should be sorted by importance*strength (highest first)
        assert!(recalled[0].importance >= recalled[1].importance);
    }

    #[test]
    fn test_recall_by_kind() {
        let mut store = MemoryStore::new();
        store.store(EngramKind::Semantic, "fact", &["f"], 0.7, "", 1);
        store.store(EngramKind::Episodic, "event", &["e"], 0.5, "", 2);
        store.store(EngramKind::Semantic, "fact 2", &["f"], 0.9, "", 3);

        let facts = store.recall_by_kind(&EngramKind::Semantic, 5);
        assert_eq!(facts.len(), 2);
    }

    #[test]
    fn test_recall_strengthens_memory() {
        let mut store = MemoryStore::new();
        let id = store.store(EngramKind::Episodic, "test", &["tag"], 0.5, "", 1);

        let strength_before = store.engrams[&id].strength;
        store.recall_by_tag("tag", 5);
        let strength_after = store.engrams[&id].strength;

        assert!(strength_after >= strength_before, "Recall should strengthen memory");
        assert_eq!(store.engrams[&id].access_count, 1);
    }

    #[test]
    fn test_recall_top() {
        let mut store = MemoryStore::new();
        store.store(EngramKind::Episodic, "low", &["a"], 0.1, "", 1);
        store.store(EngramKind::Episodic, "high", &["b"], 0.9, "", 2);
        store.store(EngramKind::Episodic, "mid", &["c"], 0.5, "", 3);

        let top = store.recall_top(2, 5);
        assert_eq!(top.len(), 2);
        assert!(top[0].importance > top[1].importance);
    }

    #[test]
    fn test_forget() {
        let mut store = MemoryStore::new();
        let id = store.store(EngramKind::Episodic, "forget me", &["tmp"], 0.1, "", 1);
        assert_eq!(store.count(), 1);

        assert!(store.forget(id));
        assert_eq!(store.count(), 0);
        assert_eq!(store.total_forgotten, 1);
    }

    #[test]
    fn test_forget_nonexistent() {
        let mut store = MemoryStore::new();
        assert!(!store.forget(999));
    }

    #[test]
    fn test_decay() {
        let mut store = MemoryStore::new();
        // Store working memory (decays fast)
        store.store(EngramKind::Working, "temp", &["work"], 0.1, "", 1);
        // Store procedural memory (decays slow)
        store.store(EngramKind::Procedural, "skill", &["proc"], 0.8, "", 1);

        // Apply decay at a much later cycle
        store.decay(200);

        // Working memory might have been forgotten, procedural should survive
        let proc_memories = store.recall_by_kind(&EngramKind::Procedural, 200);
        assert!(!proc_memories.is_empty(), "Procedural memory should survive decay");
    }

    #[test]
    fn test_consolidation() {
        let mut store = MemoryStore::new();

        // Store some similar memories (same tags)
        store.store(EngramKind::Episodic, "evolved alpha v1", &["evolution", "alpha"], 0.5, "", 1);
        store.store(EngramKind::Episodic, "evolved alpha v2", &["evolution", "alpha"], 0.6, "", 2);
        store.store(EngramKind::Episodic, "evolved alpha v3", &["evolution", "alpha"], 0.7, "", 3);
        // Different memory
        store.store(EngramKind::Semantic, "fact about beta", &["beta", "fact"], 0.8, "", 4);

        let before_count = store.count();
        let result = store.consolidate(10);

        // Should have merged some similar episodic memories
        assert!(result.merged > 0 || result.pruned > 0 || result.strengthened > 0,
            "Consolidation should do something");
        assert!(store.count() <= before_count);
    }

    #[test]
    fn test_associations() {
        let mut store = MemoryStore::new();
        let id1 = store.store(EngramKind::Semantic, "cause", &["a"], 0.8, "", 1);
        let id2 = store.store(EngramKind::Semantic, "effect", &["b"], 0.7, "", 2);

        store.associate(id1, id2);

        let assocs = store.recall_associations(id1);
        assert_eq!(assocs.len(), 1);
        assert_eq!(assocs[0].id, id2);

        // Bidirectional
        let assocs2 = store.recall_associations(id2);
        assert_eq!(assocs2.len(), 1);
        assert_eq!(assocs2[0].id, id1);
    }

    #[test]
    fn test_pattern_detection() {
        let mut store = MemoryStore::new();

        // Create a pattern — "evolution" tag appears many times
        for i in 0..10 {
            store.store(
                EngramKind::Episodic,
                &format!("evolution event {}", i),
                &["evolution", &format!("func_{}", i % 3)],
                0.5,
                "",
                i as u64,
            );
        }

        let patterns = detect_patterns(&store);
        assert!(!patterns.is_empty());
        assert!(patterns[0].frequency >= 3);
    }

    #[test]
    fn test_stats_json() {
        let mut store = MemoryStore::new();
        store.store(EngramKind::Episodic, "test", &["a"], 0.5, "", 1);
        store.store(EngramKind::Semantic, "fact", &["b"], 0.8, "", 2);

        let json = store.stats_json();
        assert!(json.contains("\"total_stored\":2"));
        assert!(json.contains("\"active_engrams\":2"));
        assert!(json.contains("\"avg_strength\""));
    }

    #[test]
    fn test_global_memory() {
        with_memory(|m| {
            m.store(EngramKind::Episodic, "global test", &["global"], 0.5, "", 1);
            assert_eq!(m.count(), 1);
        });
    }

    #[test]
    fn test_consolidation_result_json() {
        let result = ConsolidationResult {
            merged: 3,
            pruned: 2,
            strengthened: 5,
            total_remaining: 10,
            consolidation_number: 1,
        };
        let json = result.to_json();
        assert!(json.contains("\"merged\":3"));
        assert!(json.contains("\"remaining\":10"));
    }

    #[test]
    fn test_memory_lifecycle() {
        let mut store = MemoryStore::new();

        // 1. Store memories
        let id1 = store.store(EngramKind::Episodic, "learned X", &["learning", "X"], 0.7, "cycle 1", 1);
        let id2 = store.store(EngramKind::Semantic, "X works with Y", &["X", "Y"], 0.9, "cycle 2", 2);
        let _id3 = store.store(EngramKind::Working, "temp note", &["temp"], 0.2, "cycle 3", 3);

        // 2. Create associations
        store.associate(id1, id2);

        // 3. Recall
        let recalled = store.recall_by_tag("X", 5);
        assert_eq!(recalled.len(), 2);

        // 4. Decay
        store.decay(200);

        // 5. Working memory should decay faster
        // (but might not be fully gone depending on parameters)

        // 6. Consolidate
        let result = store.consolidate(200);
        assert!(result.total_remaining <= 3);

        // Check stats
        let json = store.stats_json();
        assert!(json.contains("\"total_recalls\":"));
    }
}
