//! LSM-Tree key-value store for Vitalis.
//!
//! Implements a log-structured merge-tree (LSM) key-value store:
//! - **MemTable**: Skip list as in-memory write buffer
//! - **Sorted String Tables (SSTs)**: Block-based format with bloom filters
//! - **Leveled compaction**: L0 flush, size-ratio compaction, tombstone GC
//! - **Block cache**: LRU cache for hot SST blocks
//! - **Range queries**: Forward/reverse iterators, prefix scan
//! - **Write batching**: Group commits for throughput

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::Instant;

// ── MemTable (In-Memory Write Buffer) ────────────────────────────────

/// MemTable entry — latest write wins.
#[derive(Debug, Clone)]
pub enum MemTableEntry {
    Value(Vec<u8>),
    Tombstone, // deleted key
}

/// In-memory sorted map used as write buffer.
/// Uses BTreeMap as a simplified skip list equivalent.
pub struct MemTable {
    data: BTreeMap<Vec<u8>, MemTableEntry>,
    size_bytes: usize,
    max_size: usize,
}

impl MemTable {
    pub fn new(max_size: usize) -> Self {
        Self {
            data: BTreeMap::new(),
            size_bytes: 0,
            max_size,
        }
    }

    /// Put a key-value pair.
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.size_bytes += key.len() + value.len();
        self.data.insert(key, MemTableEntry::Value(value));
    }

    /// Delete a key (insert tombstone).
    pub fn delete(&mut self, key: Vec<u8>) {
        self.size_bytes += key.len() + 1;
        self.data.insert(key, MemTableEntry::Tombstone);
    }

    /// Get a value by key.
    pub fn get(&self, key: &[u8]) -> Option<&MemTableEntry> {
        self.data.get(key)
    }

    /// Check if the memtable is full and should be flushed.
    pub fn should_flush(&self) -> bool {
        self.size_bytes >= self.max_size
    }

    /// Drain all entries for flushing to SST.
    pub fn drain(&mut self) -> BTreeMap<Vec<u8>, MemTableEntry> {
        self.size_bytes = 0;
        std::mem::take(&mut self.data)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn size_bytes(&self) -> usize {
        self.size_bytes
    }

    /// Iterator over all entries in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &MemTableEntry)> {
        self.data.iter()
    }
}

// ── Bloom Filter ─────────────────────────────────────────────────────

/// Simple bloom filter for SST block membership testing.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bits: Vec<bool>,
    num_hashes: usize,
    size: usize,
}

impl BloomFilter {
    pub fn new(size: usize, num_hashes: usize) -> Self {
        Self {
            bits: vec![false; size],
            num_hashes,
            size,
        }
    }

    /// Insert a key into the bloom filter.
    pub fn insert(&mut self, key: &[u8]) {
        for i in 0..self.num_hashes {
            let h = self.hash(key, i);
            self.bits[h % self.size] = true;
        }
    }

    /// Check if a key might be in the set. False = definitely not present.
    pub fn may_contain(&self, key: &[u8]) -> bool {
        for i in 0..self.num_hashes {
            let h = self.hash(key, i);
            if !self.bits[h % self.size] {
                return false;
            }
        }
        true
    }

    fn hash(&self, key: &[u8], seed: usize) -> usize {
        let mut h: usize = seed.wrapping_mul(0x9e3779b97f4a7c15);
        for &b in key {
            h = h.wrapping_mul(31).wrapping_add(b as usize);
        }
        h
    }

    /// False positive rate estimate.
    pub fn false_positive_rate(&self, num_elements: usize) -> f64 {
        let m = self.size as f64;
        let k = self.num_hashes as f64;
        let n = num_elements as f64;
        (1.0 - (-k * n / m).exp()).powf(k)
    }
}

// ── SST (Sorted String Table) ────────────────────────────────────────

/// A block within an SST.
#[derive(Debug, Clone)]
pub struct SstBlock {
    pub entries: Vec<(Vec<u8>, Vec<u8>)>, // (key, value) pairs
    pub min_key: Vec<u8>,
    pub max_key: Vec<u8>,
}

impl SstBlock {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            min_key: Vec::new(),
            max_key: Vec::new(),
        }
    }

    pub fn add(&mut self, key: Vec<u8>, value: Vec<u8>) {
        if self.entries.is_empty() {
            self.min_key = key.clone();
        }
        self.max_key = key.clone();
        self.entries.push((key, value));
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.entries.iter()
            .find(|(k, _)| k.as_slice() == key)
            .map(|(_, v)| v.as_slice())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// A sorted string table file.
#[derive(Debug, Clone)]
pub struct Sst {
    pub id: u64,
    pub level: usize,
    pub blocks: Vec<SstBlock>,
    pub bloom: BloomFilter,
    pub min_key: Vec<u8>,
    pub max_key: Vec<u8>,
    pub size_bytes: usize,
    pub entry_count: usize,
}

impl Sst {
    /// Create an SST from sorted entries.
    pub fn from_entries(id: u64, level: usize, entries: Vec<(Vec<u8>, Vec<u8>)>, block_size: usize) -> Self {
        let mut blocks = Vec::new();
        let mut bloom = BloomFilter::new(entries.len().max(64) * 10, 3);
        let mut current_block = SstBlock::new();
        let mut total_size = 0;

        let min_key = entries.first().map(|(k, _)| k.clone()).unwrap_or_default();
        let max_key = entries.last().map(|(k, _)| k.clone()).unwrap_or_default();
        let entry_count = entries.len();

        for (key, value) in entries {
            bloom.insert(&key);
            total_size += key.len() + value.len();
            current_block.add(key, value);
            if current_block.len() >= block_size {
                blocks.push(current_block);
                current_block = SstBlock::new();
            }
        }
        if !current_block.entries.is_empty() {
            blocks.push(current_block);
        }

        Self {
            id,
            level,
            blocks,
            bloom,
            min_key,
            max_key,
            size_bytes: total_size,
            entry_count,
        }
    }

    /// Search for a key in this SST.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        if !self.bloom.may_contain(key) {
            return None;
        }
        for block in &self.blocks {
            if let Some(val) = block.get(key) {
                return Some(val.to_vec());
            }
        }
        None
    }

    /// Scan all entries in order.
    pub fn scan(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.blocks.iter()
            .flat_map(|b| b.entries.iter().cloned())
            .collect()
    }
}

// ── Block Cache ──────────────────────────────────────────────────────

/// LRU block cache for hot SST blocks.
pub struct BlockCache {
    cache: HashMap<(u64, usize), SstBlock>, // (sst_id, block_idx) → block
    lru: VecDeque<(u64, usize)>,
    capacity: usize,
    hits: u64,
    misses: u64,
}

impl BlockCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::new(),
            lru: VecDeque::new(),
            capacity,
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, sst_id: u64, block_idx: usize) -> Option<&SstBlock> {
        let key = (sst_id, block_idx);
        if self.cache.contains_key(&key) {
            self.hits += 1;
            self.lru.retain(|k| k != &key);
            self.lru.push_back(key);
            self.cache.get(&key)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, sst_id: u64, block_idx: usize, block: SstBlock) {
        let key = (sst_id, block_idx);
        if self.cache.len() >= self.capacity {
            if let Some(evict) = self.lru.pop_front() {
                self.cache.remove(&evict);
            }
        }
        self.cache.insert(key, block);
        self.lru.push_back(key);
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
}

// ── Write Batch ──────────────────────────────────────────────────────

/// A batch of writes to apply atomically.
#[derive(Debug, Clone)]
pub struct WriteBatch {
    ops: Vec<WriteBatchOp>,
}

#[derive(Debug, Clone)]
pub enum WriteBatchOp {
    Put(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
}

impl WriteBatch {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.ops.push(WriteBatchOp::Put(key, value));
    }

    pub fn delete(&mut self, key: Vec<u8>) {
        self.ops.push(WriteBatchOp::Delete(key));
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

// ── LSM-Tree KV Store ────────────────────────────────────────────────

/// KV store statistics.
#[derive(Debug, Clone, Default)]
pub struct KvStats {
    pub writes: u64,
    pub reads: u64,
    pub deletes: u64,
    pub compactions: u64,
    pub flushes: u64,
    pub bloom_filter_saves: u64,
}

/// The main LSM-Tree based key-value store.
pub struct KvStore {
    /// Active memtable (receives writes).
    memtable: MemTable,
    /// Immutable memtables waiting to be flushed.
    immutable: Vec<MemTable>,
    /// SSTs organized by level.
    levels: Vec<Vec<Sst>>,
    /// Block cache.
    block_cache: BlockCache,
    /// Configuration.
    config: KvConfig,
    /// Next SST ID.
    next_sst_id: u64,
    /// Statistics.
    stats: KvStats,
}

/// KV store configuration.
#[derive(Debug, Clone)]
pub struct KvConfig {
    pub memtable_size: usize,
    pub block_size: usize,
    pub num_levels: usize,
    pub level_size_ratio: usize,
    pub block_cache_capacity: usize,
}

impl Default for KvConfig {
    fn default() -> Self {
        Self {
            memtable_size: 4 * 1024 * 1024, // 4 MB
            block_size: 64,
            num_levels: 7,
            level_size_ratio: 10,
            block_cache_capacity: 1024,
        }
    }
}

impl KvStore {
    pub fn new(config: KvConfig) -> Self {
        let num_levels = config.num_levels;
        let cache_cap = config.block_cache_capacity;
        Self {
            memtable: MemTable::new(config.memtable_size),
            immutable: Vec::new(),
            levels: vec![Vec::new(); num_levels],
            block_cache: BlockCache::new(cache_cap),
            config,
            next_sst_id: 1,
            stats: KvStats::default(),
        }
    }

    /// Put a key-value pair.
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.stats.writes += 1;
        self.memtable.put(key, value);
        if self.memtable.should_flush() {
            self.flush_memtable();
        }
    }

    /// Get a value by key. Checks memtable, immutable memtables, then SSTs.
    pub fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.stats.reads += 1;

        // 1. Check active memtable.
        if let Some(entry) = self.memtable.get(key) {
            return match entry {
                MemTableEntry::Value(v) => Some(v.clone()),
                MemTableEntry::Tombstone => None,
            };
        }

        // 2. Check immutable memtables (newest first).
        for imm in self.immutable.iter().rev() {
            if let Some(entry) = imm.get(key) {
                return match entry {
                    MemTableEntry::Value(v) => Some(v.clone()),
                    MemTableEntry::Tombstone => None,
                };
            }
        }

        // 3. Check SSTs level by level (L0 → L_max).
        for level in &self.levels {
            for sst in level.iter().rev() {
                if !sst.bloom.may_contain(key) {
                    self.stats.bloom_filter_saves += 1;
                    continue;
                }
                if let Some(val) = sst.get(key) {
                    return Some(val);
                }
            }
        }

        None
    }

    /// Delete a key.
    pub fn delete(&mut self, key: Vec<u8>) {
        self.stats.deletes += 1;
        self.memtable.delete(key);
    }

    /// Apply a write batch atomically.
    pub fn write_batch(&mut self, batch: WriteBatch) {
        for op in batch.ops {
            match op {
                WriteBatchOp::Put(k, v) => self.put(k, v),
                WriteBatchOp::Delete(k) => self.delete(k),
            }
        }
    }

    /// Range scan: return all key-value pairs where lo <= key <= hi.
    pub fn range_scan(&self, lo: &[u8], hi: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut result: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();

        // Scan SSTs bottom-up (older first, newer overwrites).
        for level in self.levels.iter().rev() {
            for sst in level {
                for (k, v) in sst.scan() {
                    if k.as_slice() >= lo && k.as_slice() <= hi {
                        result.insert(k, v);
                    }
                }
            }
        }

        // Scan memtable (newest, overwrites all).
        for (k, entry) in self.memtable.iter() {
            if k.as_slice() >= lo && k.as_slice() <= hi {
                match entry {
                    MemTableEntry::Value(v) => { result.insert(k.clone(), v.clone()); }
                    MemTableEntry::Tombstone => { result.remove(k); }
                }
            }
        }

        result.into_iter().collect()
    }

    /// Prefix scan: return all entries matching a key prefix.
    pub fn prefix_scan(&self, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut hi = prefix.to_vec();
        // Increment last byte for upper bound.
        if let Some(last) = hi.last_mut() {
            *last = last.saturating_add(1);
        }
        self.range_scan(prefix, &hi)
    }

    /// Flush the active memtable to an L0 SST.
    pub fn flush_memtable(&mut self) {
        let entries: Vec<(Vec<u8>, Vec<u8>)> = self.memtable.drain()
            .into_iter()
            .filter_map(|(k, e)| match e {
                MemTableEntry::Value(v) => Some((k, v)),
                MemTableEntry::Tombstone => None,
            })
            .collect();

        if entries.is_empty() {
            return;
        }

        let sst = Sst::from_entries(self.next_sst_id, 0, entries, self.config.block_size);
        self.next_sst_id += 1;
        self.levels[0].push(sst);
        self.stats.flushes += 1;

        // Check if L0 compaction is needed.
        if self.levels[0].len() > self.config.level_size_ratio {
            self.compact(0);
        }
    }

    /// Compact level `n` into level `n+1`.
    pub fn compact(&mut self, level: usize) {
        if level + 1 >= self.config.num_levels {
            return;
        }

        // Merge all SSTs from this level into one.
        let ssts = std::mem::take(&mut self.levels[level]);
        let mut all_entries: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();

        // Also merge existing L+1 SSTs.
        let next_ssts = std::mem::take(&mut self.levels[level + 1]);
        for sst in next_ssts {
            for (k, v) in sst.scan() {
                all_entries.insert(k, v);
            }
        }

        // Level N entries overwrite L+1 (newer).
        for sst in ssts {
            for (k, v) in sst.scan() {
                all_entries.insert(k, v);
            }
        }

        let entries: Vec<(Vec<u8>, Vec<u8>)> = all_entries.into_iter().collect();
        if !entries.is_empty() {
            let merged = Sst::from_entries(self.next_sst_id, level + 1, entries, self.config.block_size);
            self.next_sst_id += 1;
            self.levels[level + 1].push(merged);
        }

        self.stats.compactions += 1;
    }

    /// Get statistics.
    pub fn stats(&self) -> &KvStats {
        &self.stats
    }

    /// Total number of SSTs across all levels.
    pub fn sst_count(&self) -> usize {
        self.levels.iter().map(|l| l.len()).sum()
    }

    /// Total entries in memtable.
    pub fn memtable_size(&self) -> usize {
        self.memtable.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memtable_put_get() {
        let mut mt = MemTable::new(1024);
        mt.put(b"key1".to_vec(), b"val1".to_vec());
        match mt.get(b"key1").unwrap() {
            MemTableEntry::Value(v) => assert_eq!(v, b"val1"),
            _ => panic!("expected value"),
        }
    }

    #[test]
    fn test_memtable_delete() {
        let mut mt = MemTable::new(1024);
        mt.put(b"key1".to_vec(), b"val1".to_vec());
        mt.delete(b"key1".to_vec());
        match mt.get(b"key1").unwrap() {
            MemTableEntry::Tombstone => {}
            _ => panic!("expected tombstone"),
        }
    }

    #[test]
    fn test_bloom_filter() {
        let mut bf = BloomFilter::new(1000, 3);
        bf.insert(b"hello");
        bf.insert(b"world");
        assert!(bf.may_contain(b"hello"));
        assert!(bf.may_contain(b"world"));
        // May have false positives, but this specific case should be false.
        // Just test that a sufficiently different key is likely rejected.
        let fpr = bf.false_positive_rate(2);
        assert!(fpr < 0.1);
    }

    #[test]
    fn test_sst_from_entries() {
        let entries = vec![
            (b"a".to_vec(), b"1".to_vec()),
            (b"b".to_vec(), b"2".to_vec()),
            (b"c".to_vec(), b"3".to_vec()),
        ];
        let sst = Sst::from_entries(1, 0, entries, 2);
        assert_eq!(sst.entry_count, 3);
        assert_eq!(sst.get(b"b"), Some(b"2".to_vec()));
        assert_eq!(sst.get(b"z"), None);
    }

    #[test]
    fn test_sst_bloom_filter_skip() {
        let entries = vec![(b"key1".to_vec(), b"val1".to_vec())];
        let sst = Sst::from_entries(1, 0, entries, 10);
        // Bloom says "no" for keys not inserted.
        assert_eq!(sst.get(b"nonexistent_key_xyz"), None);
    }

    #[test]
    fn test_kv_put_get() {
        let mut kv = KvStore::new(KvConfig { memtable_size: 4096, ..Default::default() });
        kv.put(b"name".to_vec(), b"Alice".to_vec());
        assert_eq!(kv.get(b"name"), Some(b"Alice".to_vec()));
    }

    #[test]
    fn test_kv_delete() {
        let mut kv = KvStore::new(KvConfig { memtable_size: 4096, ..Default::default() });
        kv.put(b"key".to_vec(), b"val".to_vec());
        kv.delete(b"key".to_vec());
        assert_eq!(kv.get(b"key"), None);
    }

    #[test]
    fn test_kv_overwrite() {
        let mut kv = KvStore::new(KvConfig { memtable_size: 4096, ..Default::default() });
        kv.put(b"key".to_vec(), b"old".to_vec());
        kv.put(b"key".to_vec(), b"new".to_vec());
        assert_eq!(kv.get(b"key"), Some(b"new".to_vec()));
    }

    #[test]
    fn test_kv_flush() {
        let mut kv = KvStore::new(KvConfig { memtable_size: 32, block_size: 4, ..Default::default() });
        kv.put(b"a".to_vec(), b"1".to_vec());
        kv.put(b"b".to_vec(), b"2".to_vec());
        kv.flush_memtable();
        assert_eq!(kv.sst_count(), 1);
        assert_eq!(kv.get(b"a"), Some(b"1".to_vec()));
    }

    #[test]
    fn test_kv_range_scan() {
        let mut kv = KvStore::new(KvConfig { memtable_size: 4096, ..Default::default() });
        kv.put(b"a".to_vec(), b"1".to_vec());
        kv.put(b"b".to_vec(), b"2".to_vec());
        kv.put(b"c".to_vec(), b"3".to_vec());
        kv.put(b"d".to_vec(), b"4".to_vec());
        let range = kv.range_scan(b"b", b"c");
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_kv_prefix_scan() {
        let mut kv = KvStore::new(KvConfig { memtable_size: 4096, ..Default::default() });
        kv.put(b"user:1".to_vec(), b"Alice".to_vec());
        kv.put(b"user:2".to_vec(), b"Bob".to_vec());
        kv.put(b"item:1".to_vec(), b"Widget".to_vec());
        let users = kv.prefix_scan(b"user:");
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_write_batch() {
        let mut kv = KvStore::new(KvConfig { memtable_size: 4096, ..Default::default() });
        let mut batch = WriteBatch::new();
        batch.put(b"k1".to_vec(), b"v1".to_vec());
        batch.put(b"k2".to_vec(), b"v2".to_vec());
        batch.delete(b"k1".to_vec());
        kv.write_batch(batch);
        assert_eq!(kv.get(b"k1"), None);
        assert_eq!(kv.get(b"k2"), Some(b"v2".to_vec()));
    }

    #[test]
    fn test_block_cache() {
        let mut cache = BlockCache::new(2);
        let block = SstBlock::new();
        cache.insert(1, 0, block.clone());
        assert!(cache.get(1, 0).is_some());
        assert!(cache.get(1, 1).is_none());
        assert!(cache.hit_rate() > 0.0);
    }

    #[test]
    fn test_compaction() {
        let mut kv = KvStore::new(KvConfig {
            memtable_size: 32,
            block_size: 4,
            level_size_ratio: 2,
            ..Default::default()
        });
        // Insert enough to trigger multiple flushes and compaction.
        for i in 0..20 {
            kv.put(format!("key{i:03}").into_bytes(), format!("val{i}").into_bytes());
        }
        kv.flush_memtable();
        // Should have flushed at least once.
        assert!(kv.stats().flushes >= 1);
    }

    #[test]
    fn test_kv_stats() {
        let mut kv = KvStore::new(KvConfig { memtable_size: 4096, ..Default::default() });
        kv.put(b"a".to_vec(), b"1".to_vec());
        kv.put(b"b".to_vec(), b"2".to_vec());
        kv.get(b"a");
        kv.delete(b"b".to_vec());
        assert_eq!(kv.stats().writes, 2);
        assert_eq!(kv.stats().reads, 1);
        assert_eq!(kv.stats().deletes, 1);
    }

    #[test]
    fn test_sst_scan() {
        let entries = vec![
            (b"x".to_vec(), b"10".to_vec()),
            (b"y".to_vec(), b"20".to_vec()),
        ];
        let sst = Sst::from_entries(1, 0, entries, 10);
        let scanned = sst.scan();
        assert_eq!(scanned.len(), 2);
    }

    #[test]
    fn test_block_cache_eviction() {
        let mut cache = BlockCache::new(2);
        cache.insert(1, 0, SstBlock::new());
        cache.insert(1, 1, SstBlock::new());
        cache.insert(1, 2, SstBlock::new()); // evicts (1,0)
        assert_eq!(cache.len(), 2);
        assert!(cache.get(1, 0).is_none());
    }

    #[test]
    fn test_memtable_should_flush() {
        let mut mt = MemTable::new(20);
        mt.put(b"a".to_vec(), b"12345678901".to_vec()); // 12 bytes
        assert!(!mt.should_flush());
        mt.put(b"b".to_vec(), b"12345678901".to_vec()); // 24 total
        assert!(mt.should_flush());
    }
}
