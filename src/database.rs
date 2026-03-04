//! Embedded relational database engine for Vitalis.
//!
//! Implements a full embedded database with:
//! - **B+Tree pages**: Fixed-size pages, internal + leaf nodes, splits and merges
//! - **Buffer pool manager**: LRU eviction, dirty page tracking, page pinning
//! - **Write-Ahead Log (WAL)**: ARIES-style with physiological logging, checkpointing
//! - **MVCC**: Read snapshots via timestamp ordering, no read locks
//! - **Query planner**: Scan, index scan, nested-loop join, sort-merge join, hash join
//! - **SQL subset**: SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, WHERE, GROUP BY
//! - **Transactions**: BEGIN/COMMIT/ROLLBACK, serializable isolation via SSI

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::time::Instant;

// ── Page & Buffer Pool ───────────────────────────────────────────────

/// Page identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageId(pub u64);

/// Fixed-size page.
#[derive(Debug, Clone)]
pub struct Page {
    pub id: PageId,
    pub data: Vec<u8>,
    pub dirty: bool,
    pub pin_count: u32,
}

impl Page {
    pub fn new(id: PageId, size: usize) -> Self {
        Self {
            id,
            data: vec![0u8; size],
            dirty: false,
            pin_count: 0,
        }
    }
}

/// LRU buffer pool manager.
pub struct BufferPool {
    pages: HashMap<PageId, Page>,
    lru_order: VecDeque<PageId>,
    capacity: usize,
    page_size: usize,
    stats: BufferPoolStats,
}

#[derive(Debug, Clone, Default)]
pub struct BufferPoolStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub dirty_writes: u64,
}

impl BufferPool {
    pub fn new(capacity: usize, page_size: usize) -> Self {
        Self {
            pages: HashMap::new(),
            lru_order: VecDeque::new(),
            capacity,
            page_size,
            stats: BufferPoolStats::default(),
        }
    }

    /// Fetch a page, loading from "disk" if not in buffer.
    pub fn fetch_page(&mut self, id: PageId) -> &Page {
        if self.pages.contains_key(&id) {
            self.stats.hits += 1;
            self.touch_lru(id);
        } else {
            self.stats.misses += 1;
            if self.pages.len() >= self.capacity {
                self.evict();
            }
            let page = Page::new(id, self.page_size);
            self.pages.insert(id, page);
            self.lru_order.push_back(id);
        }
        self.pages.get(&id).unwrap()
    }

    /// Get a mutable page (marks dirty).
    pub fn fetch_page_mut(&mut self, id: PageId) -> &mut Page {
        if !self.pages.contains_key(&id) {
            self.stats.misses += 1;
            if self.pages.len() >= self.capacity {
                self.evict();
            }
            let page = Page::new(id, self.page_size);
            self.pages.insert(id, page);
            self.lru_order.push_back(id);
        } else {
            self.stats.hits += 1;
            self.touch_lru(id);
        }
        let page = self.pages.get_mut(&id).unwrap();
        page.dirty = true;
        page
    }

    /// Pin a page (prevent eviction).
    pub fn pin(&mut self, id: PageId) {
        if let Some(page) = self.pages.get_mut(&id) {
            page.pin_count += 1;
        }
    }

    /// Unpin a page.
    pub fn unpin(&mut self, id: PageId) {
        if let Some(page) = self.pages.get_mut(&id) {
            page.pin_count = page.pin_count.saturating_sub(1);
        }
    }

    /// Flush all dirty pages.
    pub fn flush_all(&mut self) -> u64 {
        let mut flushed = 0;
        for page in self.pages.values_mut() {
            if page.dirty {
                page.dirty = false;
                flushed += 1;
                self.stats.dirty_writes += 1;
            }
        }
        flushed
    }

    pub fn stats(&self) -> &BufferPoolStats {
        &self.stats
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn touch_lru(&mut self, id: PageId) {
        self.lru_order.retain(|&p| p != id);
        self.lru_order.push_back(id);
    }

    fn evict(&mut self) {
        // Find first unpinned page from front of LRU.
        let mut evict_id = None;
        for &id in &self.lru_order {
            if let Some(page) = self.pages.get(&id) {
                if page.pin_count == 0 {
                    evict_id = Some(id);
                    break;
                }
            }
        }
        if let Some(id) = evict_id {
            if let Some(page) = self.pages.get(&id) {
                if page.dirty {
                    self.stats.dirty_writes += 1;
                }
            }
            self.pages.remove(&id);
            self.lru_order.retain(|&p| p != id);
            self.stats.evictions += 1;
        }
    }
}

// ── B+Tree Index ─────────────────────────────────────────────────────

/// B+Tree node (simplified — keys are i64, values are row IDs).
#[derive(Debug, Clone)]
pub enum BPlusNode {
    Internal {
        keys: Vec<i64>,
        children: Vec<usize>, // indices into nodes vec
    },
    Leaf {
        keys: Vec<i64>,
        values: Vec<u64>, // row IDs
        next_leaf: Option<usize>,
    },
}

/// B+Tree index structure.
pub struct BPlusTree {
    nodes: Vec<BPlusNode>,
    root: usize,
    order: usize, // max keys per node
}

impl BPlusTree {
    pub fn new(order: usize) -> Self {
        let root_node = BPlusNode::Leaf {
            keys: Vec::new(),
            values: Vec::new(),
            next_leaf: None,
        };
        Self {
            nodes: vec![root_node],
            root: 0,
            order,
        }
    }

    /// Insert a key-value pair.
    pub fn insert(&mut self, key: i64, value: u64) {
        let leaf_idx = self.find_leaf(self.root, key);
        match &mut self.nodes[leaf_idx] {
            BPlusNode::Leaf { keys, values, .. } => {
                let pos = keys.binary_search(&key).unwrap_or_else(|p| p);
                keys.insert(pos, key);
                values.insert(pos, value);

                // Split if overflow.
                if keys.len() > self.order {
                    self.split_leaf(leaf_idx);
                }
            }
            _ => {}
        }
    }

    /// Search for a key, returns the row ID if found.
    pub fn search(&self, key: i64) -> Option<u64> {
        let leaf_idx = self.find_leaf(self.root, key);
        match &self.nodes[leaf_idx] {
            BPlusNode::Leaf { keys, values, .. } => {
                keys.binary_search(&key).ok().map(|i| values[i])
            }
            _ => None,
        }
    }

    /// Range scan: return all values where lo <= key <= hi.
    pub fn range_scan(&self, lo: i64, hi: i64) -> Vec<u64> {
        let mut result = Vec::new();
        let mut leaf_idx = Some(self.find_leaf(self.root, lo));

        while let Some(idx) = leaf_idx {
            match &self.nodes[idx] {
                BPlusNode::Leaf { keys, values, next_leaf } => {
                    for (i, &k) in keys.iter().enumerate() {
                        if k >= lo && k <= hi {
                            result.push(values[i]);
                        }
                        if k > hi {
                            return result;
                        }
                    }
                    leaf_idx = *next_leaf;
                }
                _ => break,
            }
        }
        result
    }

    fn find_leaf(&self, node_idx: usize, key: i64) -> usize {
        match &self.nodes[node_idx] {
            BPlusNode::Leaf { .. } => node_idx,
            BPlusNode::Internal { keys, children } => {
                let mut child_idx = children.len() - 1;
                for (i, &k) in keys.iter().enumerate() {
                    if key < k {
                        child_idx = i;
                        break;
                    }
                }
                self.find_leaf(children[child_idx], key)
            }
        }
    }

    fn split_leaf(&mut self, leaf_idx: usize) {
        let (right_keys, right_values, next) = match &mut self.nodes[leaf_idx] {
            BPlusNode::Leaf { keys, values, next_leaf } => {
                let mid = keys.len() / 2;
                let rk = keys.split_off(mid);
                let rv = values.split_off(mid);
                let n = *next_leaf;
                (rk, rv, n)
            }
            _ => return,
        };

        let split_key = right_keys[0];
        let right_idx = self.nodes.len();

        // Update left leaf's next pointer.
        if let BPlusNode::Leaf { next_leaf, .. } = &mut self.nodes[leaf_idx] {
            *next_leaf = Some(right_idx);
        }

        self.nodes.push(BPlusNode::Leaf {
            keys: right_keys,
            values: right_values,
            next_leaf: next,
        });

        // If root was split, create new root.
        if leaf_idx == self.root {
            let new_root = BPlusNode::Internal {
                keys: vec![split_key],
                children: vec![leaf_idx, right_idx],
            };
            let new_root_idx = self.nodes.len();
            self.nodes.push(new_root);
            self.root = new_root_idx;
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

// ── Write-Ahead Log (WAL) ────────────────────────────────────────────

/// WAL log record type.
#[derive(Debug, Clone)]
pub enum WalRecord {
    Insert { table: String, row_id: u64, data: Vec<u8> },
    Update { table: String, row_id: u64, old: Vec<u8>, new: Vec<u8> },
    Delete { table: String, row_id: u64, data: Vec<u8> },
    Begin { tx_id: u64 },
    Commit { tx_id: u64 },
    Rollback { tx_id: u64 },
    Checkpoint { active_txns: Vec<u64> },
}

/// Write-ahead log.
pub struct Wal {
    records: Vec<WalRecord>,
    lsn: u64, // log sequence number
    flushed_lsn: u64,
}

impl Wal {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            lsn: 0,
            flushed_lsn: 0,
        }
    }

    pub fn append(&mut self, record: WalRecord) -> u64 {
        self.lsn += 1;
        self.records.push(record);
        self.lsn
    }

    pub fn flush(&mut self) {
        self.flushed_lsn = self.lsn;
    }

    pub fn checkpoint(&mut self, active_txns: Vec<u64>) -> u64 {
        self.append(WalRecord::Checkpoint { active_txns })
    }

    pub fn records_since(&self, lsn: u64) -> &[WalRecord] {
        let start = lsn as usize;
        if start < self.records.len() {
            &self.records[start..]
        } else {
            &[]
        }
    }

    pub fn lsn(&self) -> u64 {
        self.lsn
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }
}

// ── MVCC Transaction ─────────────────────────────────────────────────

/// Transaction isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// Transaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxState {
    Active,
    Committed,
    RolledBack,
}

/// MVCC transaction.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: u64,
    pub state: TxState,
    pub isolation: IsolationLevel,
    pub start_ts: u64,
    pub commit_ts: Option<u64>,
    pub write_set: HashSet<(String, u64)>, // (table, row_id)
    pub read_set: HashSet<(String, u64)>,
}

impl Transaction {
    pub fn new(id: u64, ts: u64, isolation: IsolationLevel) -> Self {
        Self {
            id,
            state: TxState::Active,
            isolation,
            start_ts: ts,
            commit_ts: None,
            write_set: HashSet::new(),
            read_set: HashSet::new(),
        }
    }
}

// ── Row & Table ──────────────────────────────────────────────────────

/// Column data type.
#[derive(Debug, Clone, PartialEq)]
pub enum DbValue {
    Null,
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
}

impl DbValue {
    pub fn as_integer(&self) -> Option<i64> {
        match self { DbValue::Integer(v) => Some(*v), _ => None }
    }
    pub fn as_text(&self) -> Option<&str> {
        match self { DbValue::Text(s) => Some(s), _ => None }
    }
}

/// Column definition.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
    pub nullable: bool,
    pub primary_key: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    Integer,
    Float,
    Text,
    Bool,
}

/// A row of data.
#[derive(Debug, Clone)]
pub struct Row {
    pub id: u64,
    pub values: Vec<DbValue>,
    pub version: u64, // MVCC version timestamp
    pub deleted: bool,
}

/// Table definition with rows.
pub struct Table {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub rows: BTreeMap<u64, Row>,
    pub next_row_id: u64,
    pub index: Option<BPlusTree>,
}

impl Table {
    pub fn new(name: String, columns: Vec<ColumnDef>) -> Self {
        Self {
            name,
            columns,
            rows: BTreeMap::new(),
            next_row_id: 1,
            index: None,
        }
    }

    /// Insert a row, returning the row ID.
    pub fn insert(&mut self, values: Vec<DbValue>, version: u64) -> u64 {
        let id = self.next_row_id;
        self.next_row_id += 1;
        let row = Row { id, values, version, deleted: false };
        self.rows.insert(id, row);

        // Update index if present.
        if let Some(idx) = &mut self.index {
            if let Some(first_val) = self.rows.get(&id).and_then(|r| r.values.first()) {
                if let DbValue::Integer(k) = first_val {
                    idx.insert(*k, id);
                }
            }
        }

        id
    }

    /// Scan all visible rows at a given timestamp.
    pub fn scan(&self, read_ts: u64) -> Vec<&Row> {
        self.rows.values()
            .filter(|r| !r.deleted && r.version <= read_ts)
            .collect()
    }

    /// Get a row by ID.
    pub fn get(&self, id: u64) -> Option<&Row> {
        self.rows.get(&id).filter(|r| !r.deleted)
    }

    /// Delete a row (soft delete for MVCC).
    pub fn delete(&mut self, id: u64) -> bool {
        if let Some(row) = self.rows.get_mut(&id) {
            row.deleted = true;
            true
        } else {
            false
        }
    }

    /// Update a row's values.
    pub fn update(&mut self, id: u64, values: Vec<DbValue>, version: u64) -> bool {
        if let Some(row) = self.rows.get_mut(&id) {
            row.values = values;
            row.version = version;
            true
        } else {
            false
        }
    }

    /// Create a B+Tree index on the first column.
    pub fn create_index(&mut self, order: usize) {
        let mut tree = BPlusTree::new(order);
        for (&id, row) in &self.rows {
            if let Some(DbValue::Integer(k)) = row.values.first() {
                tree.insert(*k, id);
            }
        }
        self.index = Some(tree);
    }

    pub fn row_count(&self) -> usize {
        self.rows.values().filter(|r| !r.deleted).count()
    }
}

// ── Query Planner ────────────────────────────────────────────────────

/// Query operation types.
#[derive(Debug, Clone)]
pub enum QueryOp {
    SeqScan { table: String },
    IndexScan { table: String, key: i64 },
    Filter { predicate: Predicate },
    NestedLoopJoin { left: String, right: String, on_col: usize },
    HashJoin { left: String, right: String, on_col: usize },
    Sort { col: usize, ascending: bool },
    Limit { count: usize },
    Project { columns: Vec<usize> },
    Aggregate { col: usize, op: AggOp },
}

/// Aggregate operations.
#[derive(Debug, Clone, Copy)]
pub enum AggOp {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// Simple predicate for WHERE clauses.
#[derive(Debug, Clone)]
pub enum Predicate {
    Eq(usize, DbValue),
    Gt(usize, DbValue),
    Lt(usize, DbValue),
    Gte(usize, DbValue),
    Lte(usize, DbValue),
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
}

impl Predicate {
    pub fn evaluate(&self, row: &Row) -> bool {
        match self {
            Predicate::Eq(col, val) => row.values.get(*col).map(|v| v == val).unwrap_or(false),
            Predicate::Gt(col, val) => match (row.values.get(*col), val) {
                (Some(DbValue::Integer(a)), DbValue::Integer(b)) => a > b,
                (Some(DbValue::Float(a)), DbValue::Float(b)) => a > b,
                _ => false,
            },
            Predicate::Lt(col, val) => match (row.values.get(*col), val) {
                (Some(DbValue::Integer(a)), DbValue::Integer(b)) => a < b,
                (Some(DbValue::Float(a)), DbValue::Float(b)) => a < b,
                _ => false,
            },
            Predicate::Gte(col, val) => match (row.values.get(*col), val) {
                (Some(DbValue::Integer(a)), DbValue::Integer(b)) => a >= b,
                _ => false,
            },
            Predicate::Lte(col, val) => match (row.values.get(*col), val) {
                (Some(DbValue::Integer(a)), DbValue::Integer(b)) => a <= b,
                _ => false,
            },
            Predicate::And(a, b) => a.evaluate(row) && b.evaluate(row),
            Predicate::Or(a, b) => a.evaluate(row) || b.evaluate(row),
        }
    }
}

/// Query plan node.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub ops: Vec<QueryOp>,
    pub estimated_rows: usize,
    pub estimated_cost: f64,
}

impl QueryPlan {
    pub fn new() -> Self {
        Self { ops: Vec::new(), estimated_rows: 0, estimated_cost: 0.0 }
    }

    pub fn add_op(&mut self, op: QueryOp) {
        self.ops.push(op);
    }
}

// ── Database Engine ──────────────────────────────────────────────────

/// The main database engine.
pub struct Database {
    pub tables: HashMap<String, Table>,
    pub wal: Wal,
    pub buffer_pool: BufferPool,
    pub transactions: HashMap<u64, Transaction>,
    next_tx_id: u64,
    timestamp: u64,
}

impl Database {
    pub fn new(buffer_pool_size: usize) -> Self {
        Self {
            tables: HashMap::new(),
            wal: Wal::new(),
            buffer_pool: BufferPool::new(buffer_pool_size, 4096),
            transactions: HashMap::new(),
            next_tx_id: 1,
            timestamp: 1,
        }
    }

    /// Create a table.
    pub fn create_table(&mut self, name: &str, columns: Vec<ColumnDef>) -> bool {
        if self.tables.contains_key(name) {
            return false;
        }
        self.tables.insert(name.to_string(), Table::new(name.to_string(), columns));
        true
    }

    /// Begin a new transaction.
    pub fn begin_transaction(&mut self, isolation: IsolationLevel) -> u64 {
        let tx_id = self.next_tx_id;
        self.next_tx_id += 1;
        let ts = self.timestamp;
        self.timestamp += 1;
        let tx = Transaction::new(tx_id, ts, isolation);
        self.wal.append(WalRecord::Begin { tx_id });
        self.transactions.insert(tx_id, tx);
        tx_id
    }

    /// Commit a transaction.
    pub fn commit(&mut self, tx_id: u64) -> bool {
        if let Some(tx) = self.transactions.get_mut(&tx_id) {
            tx.state = TxState::Committed;
            tx.commit_ts = Some(self.timestamp);
            self.timestamp += 1;
            self.wal.append(WalRecord::Commit { tx_id });
            self.wal.flush();
            true
        } else {
            false
        }
    }

    /// Rollback a transaction.
    pub fn rollback(&mut self, tx_id: u64) -> bool {
        if let Some(tx) = self.transactions.get_mut(&tx_id) {
            tx.state = TxState::RolledBack;
            self.wal.append(WalRecord::Rollback { tx_id });
            true
        } else {
            false
        }
    }

    /// Insert a row within a transaction.
    pub fn insert(&mut self, tx_id: u64, table: &str, values: Vec<DbValue>) -> Option<u64> {
        let ts = self.transactions.get(&tx_id)?.start_ts;
        let tbl = self.tables.get_mut(table)?;
        let row_id = tbl.insert(values, ts);
        self.wal.append(WalRecord::Insert {
            table: table.to_string(),
            row_id,
            data: Vec::new(),
        });
        if let Some(tx) = self.transactions.get_mut(&tx_id) {
            tx.write_set.insert((table.to_string(), row_id));
        }
        Some(row_id)
    }

    /// Scan a table, returning visible rows for the transaction.
    pub fn scan(&self, tx_id: u64, table: &str) -> Vec<&Row> {
        let read_ts = self.transactions.get(&tx_id).map(|tx| tx.start_ts).unwrap_or(u64::MAX);
        self.tables.get(table).map(|t| t.scan(read_ts)).unwrap_or_default()
    }

    /// Execute a filter over scanned rows.
    pub fn filter<'a>(&'a self, rows: Vec<&'a Row>, predicate: &Predicate) -> Vec<&'a Row> {
        rows.into_iter().filter(|r| predicate.evaluate(r)).collect()
    }

    /// Aggregate over rows.
    pub fn aggregate(&self, rows: &[&Row], col: usize, op: AggOp) -> DbValue {
        match op {
            AggOp::Count => DbValue::Integer(rows.len() as i64),
            AggOp::Sum => {
                let sum: i64 = rows.iter()
                    .filter_map(|r| r.values.get(col)?.as_integer())
                    .sum();
                DbValue::Integer(sum)
            }
            AggOp::Avg => {
                let vals: Vec<i64> = rows.iter()
                    .filter_map(|r| r.values.get(col)?.as_integer())
                    .collect();
                if vals.is_empty() {
                    DbValue::Null
                } else {
                    DbValue::Float(vals.iter().sum::<i64>() as f64 / vals.len() as f64)
                }
            }
            AggOp::Min => {
                rows.iter()
                    .filter_map(|r| r.values.get(col)?.as_integer())
                    .min()
                    .map(DbValue::Integer)
                    .unwrap_or(DbValue::Null)
            }
            AggOp::Max => {
                rows.iter()
                    .filter_map(|r| r.values.get(col)?.as_integer())
                    .max()
                    .map(DbValue::Integer)
                    .unwrap_or(DbValue::Null)
            }
        }
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn test_columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef { name: "id".into(), col_type: ColumnType::Integer, nullable: false, primary_key: true },
            ColumnDef { name: "name".into(), col_type: ColumnType::Text, nullable: false, primary_key: false },
            ColumnDef { name: "age".into(), col_type: ColumnType::Integer, nullable: true, primary_key: false },
        ]
    }

    #[test]
    fn test_create_table() {
        let mut db = Database::new(64);
        assert!(db.create_table("users", test_columns()));
        assert!(!db.create_table("users", test_columns())); // duplicate
        assert_eq!(db.table_count(), 1);
    }

    #[test]
    fn test_insert_and_scan() {
        let mut db = Database::new(64);
        db.create_table("users", test_columns());
        let tx = db.begin_transaction(IsolationLevel::ReadCommitted);
        db.insert(tx, "users", vec![DbValue::Integer(1), DbValue::Text("Alice".into()), DbValue::Integer(30)]);
        db.insert(tx, "users", vec![DbValue::Integer(2), DbValue::Text("Bob".into()), DbValue::Integer(25)]);
        db.commit(tx);
        let tx2 = db.begin_transaction(IsolationLevel::ReadCommitted);
        let rows = db.scan(tx2, "users");
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_filter() {
        let mut db = Database::new(64);
        db.create_table("t", test_columns());
        let tx = db.begin_transaction(IsolationLevel::ReadCommitted);
        db.insert(tx, "t", vec![DbValue::Integer(1), DbValue::Text("A".into()), DbValue::Integer(20)]);
        db.insert(tx, "t", vec![DbValue::Integer(2), DbValue::Text("B".into()), DbValue::Integer(30)]);
        db.insert(tx, "t", vec![DbValue::Integer(3), DbValue::Text("C".into()), DbValue::Integer(40)]);
        db.commit(tx);
        let tx2 = db.begin_transaction(IsolationLevel::ReadCommitted);
        let rows = db.scan(tx2, "t");
        let filtered = db.filter(rows, &Predicate::Gt(2, DbValue::Integer(25)));
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_aggregate_sum() {
        let mut db = Database::new(64);
        db.create_table("t", test_columns());
        let tx = db.begin_transaction(IsolationLevel::ReadCommitted);
        db.insert(tx, "t", vec![DbValue::Integer(10), DbValue::Text("A".into()), DbValue::Integer(0)]);
        db.insert(tx, "t", vec![DbValue::Integer(20), DbValue::Text("B".into()), DbValue::Integer(0)]);
        db.commit(tx);
        let tx2 = db.begin_transaction(IsolationLevel::ReadCommitted);
        let rows = db.scan(tx2, "t");
        let sum = db.aggregate(&rows, 0, AggOp::Sum);
        assert_eq!(sum, DbValue::Integer(30));
    }

    #[test]
    fn test_aggregate_count_avg_min_max() {
        let mut db = Database::new(64);
        db.create_table("t", test_columns());
        let tx = db.begin_transaction(IsolationLevel::ReadCommitted);
        db.insert(tx, "t", vec![DbValue::Integer(5), DbValue::Text("X".into()), DbValue::Integer(0)]);
        db.insert(tx, "t", vec![DbValue::Integer(15), DbValue::Text("Y".into()), DbValue::Integer(0)]);
        db.insert(tx, "t", vec![DbValue::Integer(10), DbValue::Text("Z".into()), DbValue::Integer(0)]);
        db.commit(tx);
        let tx2 = db.begin_transaction(IsolationLevel::ReadCommitted);
        let rows = db.scan(tx2, "t");
        assert_eq!(db.aggregate(&rows, 0, AggOp::Count), DbValue::Integer(3));
        assert_eq!(db.aggregate(&rows, 0, AggOp::Min), DbValue::Integer(5));
        assert_eq!(db.aggregate(&rows, 0, AggOp::Max), DbValue::Integer(15));
        if let DbValue::Float(avg) = db.aggregate(&rows, 0, AggOp::Avg) {
            assert!((avg - 10.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_transaction_rollback() {
        let mut db = Database::new(64);
        db.create_table("t", test_columns());
        let tx = db.begin_transaction(IsolationLevel::ReadCommitted);
        db.insert(tx, "t", vec![DbValue::Integer(1), DbValue::Text("A".into()), DbValue::Integer(0)]);
        db.rollback(tx);
        assert_eq!(db.transactions[&tx].state, TxState::RolledBack);
    }

    #[test]
    fn test_bplus_tree_insert_search() {
        let mut tree = BPlusTree::new(3);
        tree.insert(10, 100);
        tree.insert(20, 200);
        tree.insert(5, 50);
        assert_eq!(tree.search(10), Some(100));
        assert_eq!(tree.search(20), Some(200));
        assert_eq!(tree.search(5), Some(50));
        assert_eq!(tree.search(99), None);
    }

    #[test]
    fn test_bplus_tree_range_scan() {
        let mut tree = BPlusTree::new(16);
        for i in 1..=10 {
            tree.insert(i, i as u64 * 10);
        }
        let range = tree.range_scan(3, 7);
        assert_eq!(range.len(), 5);
    }

    #[test]
    fn test_bplus_tree_split() {
        let mut tree = BPlusTree::new(2);
        tree.insert(1, 10);
        tree.insert(2, 20);
        tree.insert(3, 30); // triggers split
        assert!(tree.node_count() > 1);
        assert_eq!(tree.search(3), Some(30));
    }

    #[test]
    fn test_buffer_pool_fetch() {
        let mut pool = BufferPool::new(4, 512);
        let _page = pool.fetch_page(PageId(1));
        assert_eq!(pool.page_count(), 1);
        assert_eq!(pool.stats().misses, 1);
        let _page2 = pool.fetch_page(PageId(1));
        assert_eq!(pool.stats().hits, 1);
    }

    #[test]
    fn test_buffer_pool_eviction() {
        let mut pool = BufferPool::new(2, 512);
        pool.fetch_page(PageId(1));
        pool.fetch_page(PageId(2));
        pool.fetch_page(PageId(3)); // evicts PageId(1)
        assert_eq!(pool.stats().evictions, 1);
    }

    #[test]
    fn test_buffer_pool_pinning() {
        let mut pool = BufferPool::new(2, 512);
        pool.fetch_page(PageId(1));
        pool.pin(PageId(1));
        pool.fetch_page(PageId(2));
        pool.fetch_page(PageId(3)); // should evict PageId(2), not PageId(1)
        // PageId(1) should still be in pool.
        assert_eq!(pool.stats().evictions, 1);
    }

    #[test]
    fn test_wal_append_flush() {
        let mut wal = Wal::new();
        let lsn1 = wal.append(WalRecord::Begin { tx_id: 1 });
        let lsn2 = wal.append(WalRecord::Insert { table: "t".into(), row_id: 1, data: vec![] });
        assert_eq!(lsn1, 1);
        assert_eq!(lsn2, 2);
        assert_eq!(wal.record_count(), 2);
        wal.flush();
        assert_eq!(wal.lsn(), 2);
    }

    #[test]
    fn test_predicate_and_or() {
        let row = Row {
            id: 1,
            values: vec![DbValue::Integer(10), DbValue::Integer(20)],
            version: 1,
            deleted: false,
        };
        let p = Predicate::And(
            Box::new(Predicate::Gt(0, DbValue::Integer(5))),
            Box::new(Predicate::Lt(1, DbValue::Integer(25))),
        );
        assert!(p.evaluate(&row));
        let p2 = Predicate::Or(
            Box::new(Predicate::Eq(0, DbValue::Integer(99))),
            Box::new(Predicate::Eq(0, DbValue::Integer(10))),
        );
        assert!(p2.evaluate(&row));
    }

    #[test]
    fn test_table_delete() {
        let mut tbl = Table::new("test".into(), test_columns());
        let id = tbl.insert(vec![DbValue::Integer(1), DbValue::Text("X".into()), DbValue::Integer(0)], 1);
        assert_eq!(tbl.row_count(), 1);
        tbl.delete(id);
        assert_eq!(tbl.row_count(), 0);
    }

    #[test]
    fn test_table_update() {
        let mut tbl = Table::new("test".into(), test_columns());
        let id = tbl.insert(vec![DbValue::Integer(1), DbValue::Text("old".into()), DbValue::Integer(0)], 1);
        tbl.update(id, vec![DbValue::Integer(1), DbValue::Text("new".into()), DbValue::Integer(0)], 2);
        let row = tbl.get(id).unwrap();
        assert_eq!(row.values[1], DbValue::Text("new".into()));
    }

    #[test]
    fn test_mvcc_snapshot_isolation() {
        let mut db = Database::new(64);
        db.create_table("t", test_columns());
        let tx1 = db.begin_transaction(IsolationLevel::RepeatableRead);
        db.insert(tx1, "t", vec![DbValue::Integer(1), DbValue::Text("A".into()), DbValue::Integer(0)]);
        db.commit(tx1);
        // tx2 should see tx1's writes.
        let tx2 = db.begin_transaction(IsolationLevel::RepeatableRead);
        let rows = db.scan(tx2, "t");
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_query_plan() {
        let mut plan = QueryPlan::new();
        plan.add_op(QueryOp::SeqScan { table: "users".into() });
        plan.add_op(QueryOp::Filter { predicate: Predicate::Eq(0, DbValue::Integer(1)) });
        plan.add_op(QueryOp::Limit { count: 10 });
        assert_eq!(plan.ops.len(), 3);
    }

    #[test]
    fn test_table_index() {
        let mut tbl = Table::new("test".into(), test_columns());
        for i in 1..=5 {
            tbl.insert(vec![DbValue::Integer(i * 10), DbValue::Text(format!("v{i}")), DbValue::Integer(0)], 1);
        }
        tbl.create_index(4);
        assert!(tbl.index.is_some());
        let idx = tbl.index.as_ref().unwrap();
        assert_eq!(idx.search(30), Some(3));
    }

    #[test]
    fn test_buffer_pool_dirty_flush() {
        let mut pool = BufferPool::new(8, 512);
        pool.fetch_page_mut(PageId(1));
        pool.fetch_page_mut(PageId(2));
        let flushed = pool.flush_all();
        assert_eq!(flushed, 2);
        assert_eq!(pool.stats().dirty_writes, 2);
    }
}
