//! Advanced Data Structures Module for Vitalis v30.0
//!
//! Best-in-class implementations of fundamental data structures:
//! - B-Tree (order-configurable, O(log n) search/insert/delete)
//! - Skip List (probabilistic O(log n), lock-free friendly)
//! - Ring Buffer (fixed-capacity circular buffer, O(1) push/pop)
//! - Union-Find / Disjoint Set (path compression + union by rank, ≈O(α(n)))
//! - Interval Tree (augmented BST, O(log n + k) query)
//! - LRU Cache (O(1) get/put via HashMap + doubly-linked list simulation)

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::collections::HashMap;

// ─── B-Tree ─────────────────────────────────────────────────────────

/// A B-Tree with configurable order (minimum degree t).
/// Each node has at most 2t-1 keys and 2t children.
struct BTree {
    root: Option<BTreeNode>,
    min_degree: usize,  // t
    len: usize,
}

#[derive(Clone)]
struct BTreeNode {
    keys: Vec<i64>,
    values: Vec<i64>,
    children: Vec<BTreeNode>,
    leaf: bool,
}

impl BTree {
    fn new(min_degree: usize) -> Self {
        let t = if min_degree < 2 { 2 } else { min_degree };
        Self {
            root: None,
            min_degree: t,
            len: 0,
        }
    }

    fn search(&self, key: i64) -> Option<i64> {
        self.root.as_ref().and_then(|node| Self::search_node(node, key))
    }

    fn search_node(node: &BTreeNode, key: i64) -> Option<i64> {
        let mut i = 0;
        while i < node.keys.len() && key > node.keys[i] {
            i += 1;
        }
        if i < node.keys.len() && node.keys[i] == key {
            return Some(node.values[i]);
        }
        if node.leaf {
            return None;
        }
        Self::search_node(&node.children[i], key)
    }

    fn insert(&mut self, key: i64, value: i64) {
        if let Some(root) = &self.root {
            if root.keys.len() == 2 * self.min_degree - 1 {
                // Root is full — split it
                let old_root = self.root.take().unwrap();
                let mut new_root = BTreeNode {
                    keys: vec![],
                    values: vec![],
                    children: vec![old_root],
                    leaf: false,
                };
                Self::split_child(&mut new_root, 0, self.min_degree);
                Self::insert_non_full(&mut new_root, key, value, self.min_degree);
                self.root = Some(new_root);
            } else {
                let root = self.root.as_mut().unwrap();
                Self::insert_non_full(root, key, value, self.min_degree);
            }
        } else {
            self.root = Some(BTreeNode {
                keys: vec![key],
                values: vec![value],
                children: vec![],
                leaf: true,
            });
        }
        self.len += 1;
    }

    fn split_child(parent: &mut BTreeNode, idx: usize, t: usize) {
        let child = &mut parent.children[idx];
        let mid = t - 1;

        let new_node = BTreeNode {
            keys: child.keys.split_off(mid + 1),
            values: child.values.split_off(mid + 1),
            children: if child.leaf {
                vec![]
            } else {
                child.children.split_off(mid + 1)
            },
            leaf: child.leaf,
        };
        let _ = &new_node; // suppress unused warning

        let mid_key = child.keys.pop().unwrap();
        let mid_val = child.values.pop().unwrap();

        parent.keys.insert(idx, mid_key);
        parent.values.insert(idx, mid_val);
        parent.children.insert(idx + 1, new_node);
    }

    fn insert_non_full(node: &mut BTreeNode, key: i64, value: i64, t: usize) {
        let mut i = node.keys.len();
        if node.leaf {
            // Find insertion point
            while i > 0 && key < node.keys[i - 1] {
                i -= 1;
            }
            // Update if key exists
            if i > 0 && node.keys[i - 1] == key {
                node.values[i - 1] = value;
                return;
            }
            node.keys.insert(i, key);
            node.values.insert(i, value);
        } else {
            while i > 0 && key < node.keys[i - 1] {
                i -= 1;
            }
            if i > 0 && node.keys[i - 1] == key {
                node.values[i - 1] = value;
                return;
            }
            if node.children[i].keys.len() == 2 * t - 1 {
                Self::split_child(node, i, t);
                if key > node.keys[i] {
                    i += 1;
                } else if key == node.keys[i] {
                    node.values[i] = value;
                    return;
                }
            }
            Self::insert_non_full(&mut node.children[i], key, value, t);
        }
    }

    fn in_order(&self) -> Vec<(i64, i64)> {
        let mut result = Vec::new();
        if let Some(root) = &self.root {
            Self::in_order_node(root, &mut result);
        }
        result
    }

    fn in_order_node(node: &BTreeNode, result: &mut Vec<(i64, i64)>) {
        for i in 0..node.keys.len() {
            if !node.leaf && i < node.children.len() {
                Self::in_order_node(&node.children[i], result);
            }
            result.push((node.keys[i], node.values[i]));
        }
        if !node.leaf && node.children.len() > node.keys.len() {
            Self::in_order_node(&node.children[node.keys.len()], result);
        }
    }
}

// ─── Skip List ──────────────────────────────────────────────────────

/// Probabilistic skip list with O(log n) expected operations.
/// Uses geometric distribution for level selection (p=0.5).
struct SkipList {
    heads: Vec<Option<usize>>, // head pointer at each level
    nodes: Vec<SkipNode>,
    max_level: usize,
    len: usize,
    rng_state: u64,
}

struct SkipNode {
    key: i64,
    value: i64,
    forward: Vec<Option<usize>>, // next node at each level
}

impl SkipList {
    fn new(max_level: usize) -> Self {
        let max_level = if max_level < 1 { 16 } else { max_level };
        Self {
            heads: vec![None; max_level],
            nodes: Vec::new(),
            max_level,
            len: 0,
            rng_state: 0x12345678_9abcdef0,
        }
    }

    fn random_level(&mut self) -> usize {
        let mut level = 1;
        while level < self.max_level {
            // Xorshift for fast random bits
            self.rng_state ^= self.rng_state << 13;
            self.rng_state ^= self.rng_state >> 7;
            self.rng_state ^= self.rng_state << 17;
            if self.rng_state & 1 == 0 {
                break;
            }
            level += 1;
        }
        level
    }

    fn search(&self, key: i64) -> Option<i64> {
        let mut current = None;
        for level in (0..self.max_level).rev() {
            let start = if current.is_some() { current } else { self.heads[level] };
            let mut node_idx = start;
            while let Some(idx) = node_idx {
                if self.nodes[idx].key == key {
                    return Some(self.nodes[idx].value);
                }
                if self.nodes[idx].key > key {
                    break;
                }
                current = node_idx;
                node_idx = self.nodes[idx].forward.get(level).copied().flatten();
            }
        }
        None
    }

    fn insert(&mut self, key: i64, value: i64) {
        let level = self.random_level();
        let node_idx = self.nodes.len();
        self.nodes.push(SkipNode {
            key,
            value,
            forward: vec![None; level],
        });

        for lv in 0..level {
            // Find insertion point at this level
            let mut prev = None;
            let mut curr = self.heads[lv];
            while let Some(idx) = curr {
                if self.nodes[idx].key >= key {
                    break;
                }
                prev = curr;
                curr = self.nodes[idx].forward[lv];
            }

            // Update existing key
            if let Some(idx) = curr {
                if self.nodes[idx].key == key {
                    self.nodes[idx].value = value;
                    if lv == 0 {
                        // Remove the node we just added
                        self.nodes.pop();
                        return;
                    }
                    continue;
                }
            }

            self.nodes[node_idx].forward[lv] = curr;
            if let Some(prev_idx) = prev {
                self.nodes[prev_idx].forward[lv] = Some(node_idx);
            } else {
                self.heads[lv] = Some(node_idx);
            }
        }
        self.len += 1;
    }

    fn to_sorted_vec(&self) -> Vec<(i64, i64)> {
        let mut result = Vec::new();
        let mut curr = self.heads[0];
        while let Some(idx) = curr {
            result.push((self.nodes[idx].key, self.nodes[idx].value));
            curr = self.nodes[idx].forward[0];
        }
        result
    }
}

// ─── Ring Buffer ────────────────────────────────────────────────────

/// Fixed-capacity circular buffer with O(1) push/pop from both ends.
struct RingBuffer {
    data: Vec<i64>,
    head: usize,  // read position
    tail: usize,  // write position
    len: usize,
    capacity: usize,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        let capacity = if capacity < 1 { 16 } else { capacity };
        Self {
            data: vec![0; capacity],
            head: 0,
            tail: 0,
            len: 0,
            capacity,
        }
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    fn push_back(&mut self, value: i64) -> bool {
        if self.is_full() {
            return false;
        }
        self.data[self.tail] = value;
        self.tail = (self.tail + 1) % self.capacity;
        self.len += 1;
        true
    }

    fn pop_front(&mut self) -> Option<i64> {
        if self.is_empty() {
            return None;
        }
        let value = self.data[self.head];
        self.head = (self.head + 1) % self.capacity;
        self.len -= 1;
        Some(value)
    }

    fn push_front(&mut self, value: i64) -> bool {
        if self.is_full() {
            return false;
        }
        self.head = if self.head == 0 { self.capacity - 1 } else { self.head - 1 };
        self.data[self.head] = value;
        self.len += 1;
        true
    }

    fn pop_back(&mut self) -> Option<i64> {
        if self.is_empty() {
            return None;
        }
        self.tail = if self.tail == 0 { self.capacity - 1 } else { self.tail - 1 };
        let value = self.data[self.tail];
        self.len -= 1;
        Some(value)
    }

    fn peek_front(&self) -> Option<i64> {
        if self.is_empty() { None } else { Some(self.data[self.head]) }
    }

    fn peek_back(&self) -> Option<i64> {
        if self.is_empty() {
            None
        } else {
            let idx = if self.tail == 0 { self.capacity - 1 } else { self.tail - 1 };
            Some(self.data[idx])
        }
    }

    fn to_vec(&self) -> Vec<i64> {
        let mut result = Vec::with_capacity(self.len);
        let mut i = self.head;
        for _ in 0..self.len {
            result.push(self.data[i]);
            i = (i + 1) % self.capacity;
        }
        result
    }
}

// ─── Union-Find / Disjoint Set ──────────────────────────────────────

/// Union-Find with path compression + union by rank.
/// Nearly O(α(n)) amortized per operation (inverse Ackermann).
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
    count: usize, // number of disjoint sets
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
            count: n,
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        // Path compression (iterative)
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]]; // path halving
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, x: usize, y: usize) -> bool {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return false; // already in same set
        }
        // Union by rank
        match self.rank[rx].cmp(&self.rank[ry]) {
            std::cmp::Ordering::Less => self.parent[rx] = ry,
            std::cmp::Ordering::Greater => self.parent[ry] = rx,
            std::cmp::Ordering::Equal => {
                self.parent[ry] = rx;
                self.rank[rx] += 1;
            }
        }
        self.count -= 1;
        true
    }

    fn connected(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }

    fn set_count(&self) -> usize {
        self.count
    }
}

// ─── Interval Tree ──────────────────────────────────────────────────

/// Augmented BST for interval queries in O(log n + k) time.
struct IntervalTree {
    nodes: Vec<IntervalNode>,
    root: Option<usize>,
}

struct IntervalNode {
    low: i64,
    high: i64,
    max_high: i64,      // maximum high in this subtree
    left: Option<usize>,
    right: Option<usize>,
}

impl IntervalTree {
    fn new() -> Self {
        Self { nodes: Vec::new(), root: None }
    }

    fn insert(&mut self, low: i64, high: i64) {
        let node_idx = self.nodes.len();
        self.nodes.push(IntervalNode {
            low,
            high,
            max_high: high,
            left: None,
            right: None,
        });
        if self.root.is_none() {
            self.root = Some(node_idx);
            return;
        }
        self.root = Some(Self::insert_recursive(&mut self.nodes, self.root.unwrap(), node_idx));
    }

    fn insert_recursive(nodes: &mut Vec<IntervalNode>, current: usize, new_idx: usize) -> usize {
        // Update max_high
        if nodes[new_idx].high > nodes[current].max_high {
            nodes[current].max_high = nodes[new_idx].high;
        }

        if nodes[new_idx].low < nodes[current].low {
            if let Some(left) = nodes[current].left {
                let updated = Self::insert_recursive(nodes, left, new_idx);
                nodes[current].left = Some(updated);
            } else {
                nodes[current].left = Some(new_idx);
            }
        } else {
            if let Some(right) = nodes[current].right {
                let updated = Self::insert_recursive(nodes, right, new_idx);
                nodes[current].right = Some(updated);
            } else {
                nodes[current].right = Some(new_idx);
            }
        }
        current
    }

    /// Find all intervals that overlap with [low, high].
    fn query(&self, low: i64, high: i64) -> Vec<(i64, i64)> {
        let mut results = Vec::new();
        if let Some(root) = self.root {
            Self::query_recursive(&self.nodes, root, low, high, &mut results);
        }
        results
    }

    fn query_recursive(
        nodes: &[IntervalNode],
        idx: usize,
        low: i64,
        high: i64,
        results: &mut Vec<(i64, i64)>,
    ) {
        let node = &nodes[idx];

        // Check if current interval overlaps with query
        if node.low <= high && node.high >= low {
            results.push((node.low, node.high));
        }

        // Check left subtree
        if let Some(left) = node.left {
            if nodes[left].max_high >= low {
                Self::query_recursive(nodes, left, low, high, results);
            }
        }

        // Check right subtree
        if let Some(right) = node.right {
            if nodes[right].low <= high {
                Self::query_recursive(nodes, right, low, high, results);
            }
        }
    }
}

// ─── LRU Cache ──────────────────────────────────────────────────────

/// O(1) LRU cache using HashMap + doubly-linked list (via Vec indices).
struct LruCache {
    capacity: usize,
    map: HashMap<i64, usize>,    // key -> node index
    entries: Vec<LruEntry>,
    head: Option<usize>,         // most recently used
    tail: Option<usize>,         // least recently used
    free_list: Vec<usize>,
}

struct LruEntry {
    key: i64,
    value: i64,
    prev: Option<usize>,
    next: Option<usize>,
    active: bool,
}

impl LruCache {
    fn new(capacity: usize) -> Self {
        let capacity = if capacity < 1 { 16 } else { capacity };
        Self {
            capacity,
            map: HashMap::new(),
            entries: Vec::new(),
            head: None,
            tail: None,
            free_list: Vec::new(),
        }
    }

    fn get(&mut self, key: i64) -> Option<i64> {
        if let Some(&idx) = self.map.get(&key) {
            let value = self.entries[idx].value;
            self.move_to_front(idx);
            Some(value)
        } else {
            None
        }
    }

    fn put(&mut self, key: i64, value: i64) {
        if let Some(&idx) = self.map.get(&key) {
            self.entries[idx].value = value;
            self.move_to_front(idx);
            return;
        }

        // Evict if at capacity
        if self.map.len() >= self.capacity {
            if let Some(tail_idx) = self.tail {
                let evicted_key = self.entries[tail_idx].key;
                self.remove_node(tail_idx);
                self.entries[tail_idx].active = false;
                self.free_list.push(tail_idx);
                self.map.remove(&evicted_key);
            }
        }

        let idx = if let Some(free_idx) = self.free_list.pop() {
            self.entries[free_idx] = LruEntry {
                key,
                value,
                prev: None,
                next: None,
                active: true,
            };
            free_idx
        } else {
            let idx = self.entries.len();
            self.entries.push(LruEntry {
                key,
                value,
                prev: None,
                next: None,
                active: true,
            });
            idx
        };

        self.map.insert(key, idx);
        self.push_front(idx);
    }

    fn remove_node(&mut self, idx: usize) {
        let prev = self.entries[idx].prev;
        let next = self.entries[idx].next;

        if let Some(p) = prev {
            self.entries[p].next = next;
        } else {
            self.head = next;
        }

        if let Some(n) = next {
            self.entries[n].prev = prev;
        } else {
            self.tail = prev;
        }

        self.entries[idx].prev = None;
        self.entries[idx].next = None;
    }

    fn push_front(&mut self, idx: usize) {
        self.entries[idx].next = self.head;
        self.entries[idx].prev = None;
        if let Some(head) = self.head {
            self.entries[head].prev = Some(idx);
        }
        self.head = Some(idx);
        if self.tail.is_none() {
            self.tail = Some(idx);
        }
    }

    fn move_to_front(&mut self, idx: usize) {
        if self.head == Some(idx) {
            return;
        }
        self.remove_node(idx);
        self.push_front(idx);
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

// ─── FFI Layer ──────────────────────────────────────────────────────

// B-Tree FFI
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_btree_create(min_degree: i64) -> *mut BTree {
    let tree = Box::new(BTree::new(min_degree.max(2) as usize));
    Box::into_raw(tree)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_btree_insert(tree: *mut BTree, key: i64, value: i64) {
    let tree = unsafe { &mut *tree };
    tree.insert(key, value);
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_btree_search(tree: *mut BTree, key: i64) -> i64 {
    let tree = unsafe { &*tree };
    tree.search(key).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_btree_len(tree: *mut BTree) -> i64 {
    let tree = unsafe { &*tree };
    tree.len as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_btree_free(tree: *mut BTree) {
    if !tree.is_null() {
        unsafe { drop(Box::from_raw(tree)); }
    }
}

// Ring Buffer FFI
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ringbuf_create(capacity: i64) -> *mut RingBuffer {
    let rb = Box::new(RingBuffer::new(capacity.max(1) as usize));
    Box::into_raw(rb)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ringbuf_push_back(rb: *mut RingBuffer, value: i64) -> i64 {
    let rb = unsafe { &mut *rb };
    if rb.push_back(value) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ringbuf_pop_front(rb: *mut RingBuffer) -> i64 {
    let rb = unsafe { &mut *rb };
    rb.pop_front().unwrap_or(i64::MIN)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ringbuf_len(rb: *mut RingBuffer) -> i64 {
    let rb = unsafe { &*rb };
    rb.len as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ringbuf_free(rb: *mut RingBuffer) {
    if !rb.is_null() {
        unsafe { drop(Box::from_raw(rb)); }
    }
}

// Union-Find FFI
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_uf_create(n: i64) -> *mut UnionFind {
    let uf = Box::new(UnionFind::new(n.max(1) as usize));
    Box::into_raw(uf)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_uf_union(uf: *mut UnionFind, x: i64, y: i64) -> i64 {
    let uf = unsafe { &mut *uf };
    if uf.union(x as usize, y as usize) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_uf_find(uf: *mut UnionFind, x: i64) -> i64 {
    let uf = unsafe { &mut *uf };
    uf.find(x as usize) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_uf_connected(uf: *mut UnionFind, x: i64, y: i64) -> i64 {
    let uf = unsafe { &mut *uf };
    if uf.connected(x as usize, y as usize) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_uf_set_count(uf: *mut UnionFind) -> i64 {
    let uf = unsafe { &*uf };
    uf.set_count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_uf_free(uf: *mut UnionFind) {
    if !uf.is_null() {
        unsafe { drop(Box::from_raw(uf)); }
    }
}

// LRU Cache FFI
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lru_create(capacity: i64) -> *mut LruCache {
    let cache = Box::new(LruCache::new(capacity.max(1) as usize));
    Box::into_raw(cache)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lru_put(cache: *mut LruCache, key: i64, value: i64) {
    let cache = unsafe { &mut *cache };
    cache.put(key, value);
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lru_get(cache: *mut LruCache, key: i64) -> i64 {
    let cache = unsafe { &mut *cache };
    cache.get(key).unwrap_or(i64::MIN)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lru_len(cache: *mut LruCache) -> i64 {
    let cache = unsafe { &*cache };
    cache.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lru_free(cache: *mut LruCache) {
    if !cache.is_null() {
        unsafe { drop(Box::from_raw(cache)); }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // B-Tree tests
    #[test]
    fn test_btree_insert_search() {
        let mut tree = BTree::new(2);
        tree.insert(10, 100);
        tree.insert(20, 200);
        tree.insert(5, 50);
        assert_eq!(tree.search(10), Some(100));
        assert_eq!(tree.search(20), Some(200));
        assert_eq!(tree.search(5), Some(50));
        assert_eq!(tree.search(15), None);
    }

    #[test]
    fn test_btree_many_inserts() {
        let mut tree = BTree::new(3);
        for i in 0..100 {
            tree.insert(i, i * 10);
        }
        for i in 0..100 {
            assert_eq!(tree.search(i), Some(i * 10));
        }
        assert_eq!(tree.len, 100);
    }

    #[test]
    fn test_btree_in_order() {
        let mut tree = BTree::new(2);
        let keys = [50, 30, 70, 10, 40, 60, 80];
        for &k in &keys {
            tree.insert(k, k * 10);
        }
        let sorted = tree.in_order();
        let sorted_keys: Vec<i64> = sorted.iter().map(|&(k, _)| k).collect();
        let mut expected = keys.to_vec();
        expected.sort();
        assert_eq!(sorted_keys, expected);
    }

    #[test]
    fn test_btree_update_value() {
        let mut tree = BTree::new(2);
        tree.insert(5, 50);
        tree.insert(5, 500);
        assert_eq!(tree.search(5), Some(500));
    }

    #[test]
    fn test_btree_reverse_insert() {
        let mut tree = BTree::new(2);
        for i in (0..50).rev() {
            tree.insert(i, i);
        }
        let sorted = tree.in_order();
        let keys: Vec<i64> = sorted.iter().map(|&(k, _)| k).collect();
        assert_eq!(keys, (0..50).collect::<Vec<_>>());
    }

    // Skip List tests
    #[test]
    fn test_skiplist_insert_search() {
        let mut sl = SkipList::new(8);
        sl.insert(10, 100);
        sl.insert(20, 200);
        sl.insert(5, 50);
        assert_eq!(sl.search(10), Some(100));
        assert_eq!(sl.search(20), Some(200));
        assert_eq!(sl.search(5), Some(50));
        assert_eq!(sl.search(15), None);
    }

    #[test]
    fn test_skiplist_sorted_output() {
        let mut sl = SkipList::new(8);
        for i in [50, 30, 70, 10, 40, 60, 80] {
            sl.insert(i, i * 10);
        }
        let sorted = sl.to_sorted_vec();
        let keys: Vec<i64> = sorted.iter().map(|&(k, _)| k).collect();
        for w in keys.windows(2) {
            assert!(w[0] <= w[1], "Not sorted: {} > {}", w[0], w[1]);
        }
    }

    #[test]
    fn test_skiplist_many() {
        let mut sl = SkipList::new(16);
        for i in 0..200 {
            sl.insert(i, i * 2);
        }
        for i in 0..200 {
            assert_eq!(sl.search(i), Some(i * 2));
        }
    }

    // Ring Buffer tests
    #[test]
    fn test_ringbuf_basic() {
        let mut rb = RingBuffer::new(4);
        assert!(rb.is_empty());
        assert!(rb.push_back(1));
        assert!(rb.push_back(2));
        assert!(rb.push_back(3));
        assert!(rb.push_back(4));
        assert!(rb.is_full());
        assert!(!rb.push_back(5)); // should fail
    }

    #[test]
    fn test_ringbuf_pop() {
        let mut rb = RingBuffer::new(4);
        rb.push_back(10);
        rb.push_back(20);
        rb.push_back(30);
        assert_eq!(rb.pop_front(), Some(10));
        assert_eq!(rb.pop_front(), Some(20));
        assert_eq!(rb.pop_front(), Some(30));
        assert_eq!(rb.pop_front(), None);
    }

    #[test]
    fn test_ringbuf_wraparound() {
        let mut rb = RingBuffer::new(3);
        rb.push_back(1);
        rb.push_back(2);
        rb.push_back(3);
        rb.pop_front(); // removes 1
        rb.push_back(4); // wraps around
        assert_eq!(rb.to_vec(), vec![2, 3, 4]);
    }

    #[test]
    fn test_ringbuf_deque_ops() {
        let mut rb = RingBuffer::new(4);
        rb.push_back(2);
        rb.push_back(3);
        rb.push_front(1);
        assert_eq!(rb.to_vec(), vec![1, 2, 3]);
        assert_eq!(rb.pop_back(), Some(3));
        assert_eq!(rb.to_vec(), vec![1, 2]);
    }

    #[test]
    fn test_ringbuf_peek() {
        let mut rb = RingBuffer::new(4);
        rb.push_back(10);
        rb.push_back(20);
        assert_eq!(rb.peek_front(), Some(10));
        assert_eq!(rb.peek_back(), Some(20));
    }

    // Union-Find tests
    #[test]
    fn test_uf_basic() {
        let mut uf = UnionFind::new(5);
        assert_eq!(uf.set_count(), 5);
        assert!(!uf.connected(0, 1));
        uf.union(0, 1);
        assert!(uf.connected(0, 1));
        assert_eq!(uf.set_count(), 4);
    }

    #[test]
    fn test_uf_transitive() {
        let mut uf = UnionFind::new(5);
        uf.union(0, 1);
        uf.union(1, 2);
        assert!(uf.connected(0, 2));
        assert_eq!(uf.set_count(), 3);
    }

    #[test]
    fn test_uf_many_unions() {
        let mut uf = UnionFind::new(100);
        for i in 0..99 {
            uf.union(i, i + 1);
        }
        assert_eq!(uf.set_count(), 1);
        for i in 0..100 {
            for j in 0..100 {
                assert!(uf.connected(i, j));
            }
        }
    }

    #[test]
    fn test_uf_duplicate_union() {
        let mut uf = UnionFind::new(3);
        assert!(uf.union(0, 1));
        assert!(!uf.union(0, 1)); // already connected
        assert_eq!(uf.set_count(), 2);
    }

    // Interval Tree tests
    #[test]
    fn test_interval_tree_basic() {
        let mut tree = IntervalTree::new();
        tree.insert(10, 20);
        tree.insert(5, 15);
        tree.insert(25, 35);

        let overlapping = tree.query(12, 18);
        assert!(overlapping.contains(&(10, 20)));
        assert!(overlapping.contains(&(5, 15)));
    }

    #[test]
    fn test_interval_tree_no_overlap() {
        let mut tree = IntervalTree::new();
        tree.insert(1, 5);
        tree.insert(10, 15);
        let overlapping = tree.query(6, 9);
        assert!(overlapping.is_empty());
    }

    #[test]
    fn test_interval_tree_point_query() {
        let mut tree = IntervalTree::new();
        tree.insert(1, 10);
        tree.insert(5, 20);
        tree.insert(15, 25);
        let at_12 = tree.query(12, 12);
        assert!(at_12.contains(&(5, 20)));
        assert!(!at_12.contains(&(1, 10)));
    }

    // LRU Cache tests
    #[test]
    fn test_lru_basic() {
        let mut cache = LruCache::new(3);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        assert_eq!(cache.get(1), Some(10));
        assert_eq!(cache.get(2), Some(20));
        assert_eq!(cache.get(3), Some(30));
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = LruCache::new(2);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30); // evicts 1 (LRU)
        assert_eq!(cache.get(1), None);
        assert_eq!(cache.get(2), Some(20));
        assert_eq!(cache.get(3), Some(30));
    }

    #[test]
    fn test_lru_access_prevents_eviction() {
        let mut cache = LruCache::new(2);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.get(1); // access 1, making 2 the LRU
        cache.put(3, 30); // evicts 2 (LRU)
        assert_eq!(cache.get(1), Some(10));
        assert_eq!(cache.get(2), None);
        assert_eq!(cache.get(3), Some(30));
    }

    #[test]
    fn test_lru_update() {
        let mut cache = LruCache::new(2);
        cache.put(1, 10);
        cache.put(1, 100);
        assert_eq!(cache.get(1), Some(100));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_lru_capacity_one() {
        let mut cache = LruCache::new(1);
        cache.put(1, 10);
        cache.put(2, 20);
        assert_eq!(cache.get(1), None);
        assert_eq!(cache.get(2), Some(20));
    }

    // FFI tests
    #[test]
    fn test_ffi_btree() {
        let tree = vitalis_btree_create(2);
        vitalis_btree_insert(tree, 5, 50);
        vitalis_btree_insert(tree, 3, 30);
        vitalis_btree_insert(tree, 7, 70);
        assert_eq!(vitalis_btree_search(tree, 5), 50);
        assert_eq!(vitalis_btree_search(tree, 99), -1);
        assert_eq!(vitalis_btree_len(tree), 3);
        vitalis_btree_free(tree);
    }

    #[test]
    fn test_ffi_ringbuf() {
        let rb = vitalis_ringbuf_create(4);
        assert_eq!(vitalis_ringbuf_push_back(rb, 10), 1);
        assert_eq!(vitalis_ringbuf_push_back(rb, 20), 1);
        assert_eq!(vitalis_ringbuf_len(rb), 2);
        assert_eq!(vitalis_ringbuf_pop_front(rb), 10);
        vitalis_ringbuf_free(rb);
    }

    #[test]
    fn test_ffi_union_find() {
        let uf = vitalis_uf_create(5);
        assert_eq!(vitalis_uf_set_count(uf), 5);
        vitalis_uf_union(uf, 0, 1);
        assert_eq!(vitalis_uf_connected(uf, 0, 1), 1);
        assert_eq!(vitalis_uf_connected(uf, 0, 2), 0);
        assert_eq!(vitalis_uf_set_count(uf), 4);
        vitalis_uf_free(uf);
    }

    #[test]
    fn test_ffi_lru() {
        let cache = vitalis_lru_create(2);
        vitalis_lru_put(cache, 1, 10);
        vitalis_lru_put(cache, 2, 20);
        assert_eq!(vitalis_lru_get(cache, 1), 10);
        vitalis_lru_put(cache, 3, 30);
        assert_eq!(vitalis_lru_get(cache, 2), i64::MIN); // evicted
        vitalis_lru_free(cache);
    }
}
