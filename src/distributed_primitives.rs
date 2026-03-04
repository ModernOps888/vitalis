//! Distributed systems primitives for Vitalis.
//!
//! Implements foundational building blocks for distributed systems:
//! - **CRDTs**: G-Counter, PN-Counter, G-Set, OR-Set, LWW-Register
//! - **Vector clocks**: Causal ordering and conflict detection
//! - **Consistent hashing**: Ring-based partitioning with virtual nodes
//! - **Circuit breaker**: Fail-fast with half-open recovery
//! - **Bulkhead**: Resource isolation via semaphore-based limits
//! - **Retry with backoff**: Exponential backoff + jitter
//! - **Saga orchestrator**: Compensating transaction coordination

use std::collections::{BTreeMap, HashMap, HashSet};

// ── Vector Clock ─────────────────────────────────────────────────────

pub type ReplicaId = u64;

/// A vector clock for tracking causal ordering across replicas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorClock {
    pub clocks: BTreeMap<ReplicaId, u64>,
}

/// Causal ordering between two vector clocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalOrder {
    Before,
    After,
    Concurrent,
    Equal,
}

impl VectorClock {
    pub fn new() -> Self {
        Self { clocks: BTreeMap::new() }
    }

    /// Increment the clock for a given replica.
    pub fn increment(&mut self, replica: ReplicaId) {
        let counter = self.clocks.entry(replica).or_insert(0);
        *counter += 1;
    }

    /// Get the counter for a replica.
    pub fn get(&self, replica: ReplicaId) -> u64 {
        self.clocks.get(&replica).copied().unwrap_or(0)
    }

    /// Merge with another vector clock (point-wise max).
    pub fn merge(&mut self, other: &VectorClock) {
        for (&replica, &count) in &other.clocks {
            let entry = self.clocks.entry(replica).or_insert(0);
            *entry = (*entry).max(count);
        }
    }

    /// Compare causal ordering with another clock.
    pub fn compare(&self, other: &VectorClock) -> CausalOrder {
        let all_keys: HashSet<ReplicaId> = self.clocks.keys()
            .chain(other.clocks.keys())
            .copied()
            .collect();

        let mut self_le = true;
        let mut other_le = true;

        for key in all_keys {
            let a = self.get(key);
            let b = other.get(key);
            if a > b { self_le = false; }
            if b > a { other_le = false; }
        }

        match (self_le, other_le) {
            (true, true) => CausalOrder::Equal,
            (true, false) => CausalOrder::Before,
            (false, true) => CausalOrder::After,
            (false, false) => CausalOrder::Concurrent,
        }
    }
}

// ── CRDTs ────────────────────────────────────────────────────────────

/// G-Counter: grow-only counter CRDT.
#[derive(Debug, Clone)]
pub struct GCounter {
    pub counts: HashMap<ReplicaId, u64>,
}

impl GCounter {
    pub fn new() -> Self {
        Self { counts: HashMap::new() }
    }

    pub fn increment(&mut self, replica: ReplicaId) {
        *self.counts.entry(replica).or_insert(0) += 1;
    }

    pub fn value(&self) -> u64 {
        self.counts.values().sum()
    }

    pub fn merge(&mut self, other: &GCounter) {
        for (&replica, &count) in &other.counts {
            let entry = self.counts.entry(replica).or_insert(0);
            *entry = (*entry).max(count);
        }
    }
}

/// PN-Counter: increment/decrement counter CRDT.
#[derive(Debug, Clone)]
pub struct PNCounter {
    pub positive: GCounter,
    pub negative: GCounter,
}

impl PNCounter {
    pub fn new() -> Self {
        Self {
            positive: GCounter::new(),
            negative: GCounter::new(),
        }
    }

    pub fn increment(&mut self, replica: ReplicaId) {
        self.positive.increment(replica);
    }

    pub fn decrement(&mut self, replica: ReplicaId) {
        self.negative.increment(replica);
    }

    pub fn value(&self) -> i64 {
        self.positive.value() as i64 - self.negative.value() as i64
    }

    pub fn merge(&mut self, other: &PNCounter) {
        self.positive.merge(&other.positive);
        self.negative.merge(&other.negative);
    }
}

/// G-Set: grow-only set CRDT.
#[derive(Debug, Clone)]
pub struct GSet<T: std::hash::Hash + Eq + Clone> {
    pub elements: HashSet<T>,
}

impl<T: std::hash::Hash + Eq + Clone> GSet<T> {
    pub fn new() -> Self {
        Self { elements: HashSet::new() }
    }

    pub fn insert(&mut self, value: T) {
        self.elements.insert(value);
    }

    pub fn contains(&self, value: &T) -> bool {
        self.elements.contains(value)
    }

    pub fn merge(&mut self, other: &GSet<T>) {
        for elem in &other.elements {
            self.elements.insert(elem.clone());
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }
}

/// OR-Set: observed-remove set CRDT.
#[derive(Debug, Clone)]
pub struct ORSet<T: std::hash::Hash + Eq + Clone> {
    elements: HashMap<T, HashSet<u64>>, // value → set of unique tags
    next_tag: u64,
    tombstones: HashMap<T, HashSet<u64>>, // removed tags
}

impl<T: std::hash::Hash + Eq + Clone> ORSet<T> {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            next_tag: 0,
            tombstones: HashMap::new(),
        }
    }

    pub fn insert(&mut self, value: T) {
        let tag = self.next_tag;
        self.next_tag += 1;
        self.elements.entry(value).or_default().insert(tag);
    }

    pub fn remove(&mut self, value: &T) {
        if let Some(tags) = self.elements.remove(value) {
            self.tombstones.entry(value.clone()).or_default().extend(tags);
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        self.elements.get(value).map(|tags| !tags.is_empty()).unwrap_or(false)
    }

    pub fn values(&self) -> Vec<&T> {
        self.elements.keys().filter(|k| self.contains(k)).collect()
    }

    pub fn merge(&mut self, other: &ORSet<T>) {
        for (val, tags) in &other.elements {
            let entry = self.elements.entry(val.clone()).or_default();
            entry.extend(tags);
        }
        for (val, tags) in &other.tombstones {
            let entry = self.tombstones.entry(val.clone()).or_default();
            entry.extend(tags);
        }
        // Remove tombstoned tags.
        for (val, dead_tags) in &self.tombstones {
            if let Some(live_tags) = self.elements.get_mut(val) {
                for tag in dead_tags {
                    live_tags.remove(tag);
                }
                if live_tags.is_empty() {
                    // Mark for cleanup.
                }
            }
        }
        self.elements.retain(|_, tags| !tags.is_empty());
    }
}

/// LWW-Register: last-writer-wins register CRDT.
#[derive(Debug, Clone)]
pub struct LWWRegister<T: Clone> {
    pub value: Option<T>,
    pub timestamp: u64,
}

impl<T: Clone> LWWRegister<T> {
    pub fn new() -> Self {
        Self { value: None, timestamp: 0 }
    }

    pub fn set(&mut self, value: T, timestamp: u64) {
        if timestamp >= self.timestamp {
            self.value = Some(value);
            self.timestamp = timestamp;
        }
    }

    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn merge(&mut self, other: &LWWRegister<T>) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        }
    }
}

// ── Consistent Hashing ──────────────────────────────────────────────

/// Consistent hashing ring with virtual nodes.
#[derive(Debug, Clone)]
pub struct HashRing {
    ring: BTreeMap<u64, String>, // hash → node name
    virtual_nodes: usize,
}

impl HashRing {
    pub fn new(virtual_nodes: usize) -> Self {
        Self {
            ring: BTreeMap::new(),
            virtual_nodes,
        }
    }

    /// Add a node to the ring.
    pub fn add_node(&mut self, node: &str) {
        for i in 0..self.virtual_nodes {
            let key = format!("{node}:{i}");
            let hash = Self::hash(&key);
            self.ring.insert(hash, node.to_string());
        }
    }

    /// Remove a node from the ring.
    pub fn remove_node(&mut self, node: &str) {
        self.ring.retain(|_, v| v != node);
    }

    /// Get the node responsible for a key.
    pub fn get_node(&self, key: &str) -> Option<&str> {
        if self.ring.is_empty() {
            return None;
        }
        let hash = Self::hash(key);
        // Find the first node >= hash (clockwise).
        if let Some((_, node)) = self.ring.range(hash..).next() {
            Some(node.as_str())
        } else {
            // Wrap around to the first node.
            self.ring.values().next().map(|s| s.as_str())
        }
    }

    /// Get N nodes for replication.
    pub fn get_nodes(&self, key: &str, n: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        if self.ring.is_empty() {
            return result;
        }
        let hash = Self::hash(key);
        for (_, node) in self.ring.range(hash..).chain(self.ring.iter()) {
            if seen.insert(node.clone()) {
                result.push(node.clone());
                if result.len() >= n {
                    break;
                }
            }
        }
        result
    }

    fn hash(key: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for byte in key.bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn node_count(&self) -> usize {
        let unique: HashSet<&String> = self.ring.values().collect();
        unique.len()
    }
}

// ── Circuit Breaker ─────────────────────────────────────────────────

/// Circuit breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker for fail-fast fault tolerance.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub half_open_max: u32,
    pub total_requests: u64,
    pub total_failures: u64,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold,
            half_open_max: 1,
            total_requests: 0,
            total_failures: 0,
        }
    }

    /// Check if a request is allowed.
    pub fn allow_request(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful call.
    pub fn record_success(&mut self) {
        self.total_requests += 1;
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed call.
    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.total_failures += 1;
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Attempt to transition from Open → HalfOpen (called after timeout).
    pub fn try_half_open(&mut self) {
        if self.state == CircuitState::Open {
            self.state = CircuitState::HalfOpen;
            self.success_count = 0;
        }
    }

    pub fn failure_rate(&self) -> f64 {
        if self.total_requests == 0 { 0.0 }
        else { self.total_failures as f64 / self.total_requests as f64 }
    }
}

// ── Bulkhead ────────────────────────────────────────────────────────

/// Bulkhead for resource isolation.
#[derive(Debug, Clone)]
pub struct Bulkhead {
    pub name: String,
    pub max_concurrent: usize,
    pub active: usize,
    pub queue_size: usize,
    pub queued: usize,
    pub rejected: u64,
}

impl Bulkhead {
    pub fn new(name: &str, max_concurrent: usize, queue_size: usize) -> Self {
        Self {
            name: name.to_string(),
            max_concurrent,
            active: 0,
            queue_size,
            queued: 0,
            rejected: 0,
        }
    }

    /// Try to acquire a permit. Returns true if acquired.
    pub fn try_acquire(&mut self) -> bool {
        if self.active < self.max_concurrent {
            self.active += 1;
            true
        } else if self.queued < self.queue_size {
            self.queued += 1;
            true
        } else {
            self.rejected += 1;
            false
        }
    }

    /// Release a permit.
    pub fn release(&mut self) {
        if self.active > 0 {
            self.active -= 1;
            if self.queued > 0 {
                self.queued -= 1;
                self.active += 1;
            }
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_concurrent == 0 { 0.0 }
        else { self.active as f64 / self.max_concurrent as f64 }
    }
}

// ── Retry with Backoff ──────────────────────────────────────────────

/// Retry strategy configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30_000,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Calculate delay for a given attempt number.
    pub fn delay_ms(&self, attempt: u32) -> u64 {
        let delay = self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let delay = delay.min(self.max_delay_ms as f64) as u64;
        if self.jitter {
            // Simple deterministic "jitter" for testability.
            let jitter = (attempt as u64 * 17) % (delay / 4 + 1);
            delay + jitter
        } else {
            delay
        }
    }

    /// Check if we should retry after `attempt` failures.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

/// Retry execution result.
#[derive(Debug, Clone)]
pub struct RetryResult<T> {
    pub result: Result<T, String>,
    pub attempts: u32,
    pub total_delay_ms: u64,
}

/// Execute a function with retry logic.
pub fn retry_sync<T, F: FnMut() -> Result<T, String>>(
    policy: &RetryPolicy,
    mut f: F,
) -> RetryResult<T> {
    let mut attempts = 0;
    let mut total_delay = 0;

    loop {
        match f() {
            Ok(v) => {
                return RetryResult {
                    result: Ok(v),
                    attempts: attempts + 1,
                    total_delay_ms: total_delay,
                };
            }
            Err(e) => {
                if !policy.should_retry(attempts) {
                    return RetryResult {
                        result: Err(e),
                        attempts: attempts + 1,
                        total_delay_ms: total_delay,
                    };
                }
                total_delay += policy.delay_ms(attempts);
                attempts += 1;
            }
        }
    }
}

// ── Saga Orchestrator ───────────────────────────────────────────────

/// A saga step with action and compensating action.
#[derive(Debug, Clone)]
pub struct SagaStep {
    pub name: String,
    pub status: SagaStepStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SagaStepStatus {
    Pending,
    Completed,
    Compensated,
    Failed,
}

/// Result of executing a saga step.
#[derive(Debug, Clone)]
pub enum StepResult {
    Success,
    Failure(String),
}

/// The saga orchestrator coordinates a distributed transaction.
#[derive(Debug, Clone)]
pub struct Saga {
    pub name: String,
    pub steps: Vec<SagaStep>,
    pub status: SagaStatus,
    pub completed_count: usize,
    pub compensated_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SagaStatus {
    NotStarted,
    InProgress,
    Completed,
    Compensating,
    Compensated,
    Failed,
}

impl Saga {
    pub fn new(name: &str, step_names: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            steps: step_names.iter().map(|n| SagaStep {
                name: n.to_string(),
                status: SagaStepStatus::Pending,
            }).collect(),
            status: SagaStatus::NotStarted,
            completed_count: 0,
            compensated_count: 0,
        }
    }

    /// Execute the next pending step. Returns the step index.
    pub fn execute_step(&mut self, result: StepResult) -> Option<usize> {
        if self.status == SagaStatus::NotStarted {
            self.status = SagaStatus::InProgress;
        }
        if self.status != SagaStatus::InProgress {
            return None;
        }

        let idx = self.completed_count;
        if idx >= self.steps.len() {
            self.status = SagaStatus::Completed;
            return None;
        }

        match result {
            StepResult::Success => {
                self.steps[idx].status = SagaStepStatus::Completed;
                self.completed_count += 1;
                if self.completed_count == self.steps.len() {
                    self.status = SagaStatus::Completed;
                }
                Some(idx)
            }
            StepResult::Failure(err) => {
                self.steps[idx].status = SagaStepStatus::Failed;
                self.status = SagaStatus::Compensating;
                let _ = err;
                Some(idx)
            }
        }
    }

    /// Compensate the next step (in reverse order).
    pub fn compensate_step(&mut self) -> Option<usize> {
        if self.status != SagaStatus::Compensating {
            return None;
        }

        if self.completed_count == 0 {
            self.status = SagaStatus::Compensated;
            return None;
        }

        self.completed_count -= 1;
        let idx = self.completed_count;
        self.steps[idx].status = SagaStepStatus::Compensated;
        self.compensated_count += 1;

        if self.completed_count == 0 {
            self.status = SagaStatus::Compensated;
        }

        Some(idx)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock_increment() {
        let mut vc = VectorClock::new();
        vc.increment(1);
        vc.increment(1);
        vc.increment(2);
        assert_eq!(vc.get(1), 2);
        assert_eq!(vc.get(2), 1);
    }

    #[test]
    fn test_vector_clock_merge() {
        let mut vc1 = VectorClock::new();
        vc1.increment(1);
        vc1.increment(1);
        let mut vc2 = VectorClock::new();
        vc2.increment(2);
        vc2.increment(1);
        vc1.merge(&vc2);
        assert_eq!(vc1.get(1), 2);
        assert_eq!(vc1.get(2), 1);
    }

    #[test]
    fn test_vector_clock_compare() {
        let mut a = VectorClock::new();
        a.increment(1);
        let mut b = VectorClock::new();
        b.increment(1);
        b.increment(2);
        assert_eq!(a.compare(&b), CausalOrder::Before);
        assert_eq!(b.compare(&a), CausalOrder::After);
    }

    #[test]
    fn test_vector_clock_concurrent() {
        let mut a = VectorClock::new();
        a.increment(1);
        let mut b = VectorClock::new();
        b.increment(2);
        assert_eq!(a.compare(&b), CausalOrder::Concurrent);
    }

    #[test]
    fn test_g_counter() {
        let mut c1 = GCounter::new();
        let mut c2 = GCounter::new();
        c1.increment(1);
        c1.increment(1);
        c2.increment(2);
        c1.merge(&c2);
        assert_eq!(c1.value(), 3);
    }

    #[test]
    fn test_pn_counter() {
        let mut c = PNCounter::new();
        c.increment(1);
        c.increment(1);
        c.decrement(1);
        assert_eq!(c.value(), 1);
    }

    #[test]
    fn test_g_set() {
        let mut s1: GSet<String> = GSet::new();
        let mut s2: GSet<String> = GSet::new();
        s1.insert("a".to_string());
        s2.insert("b".to_string());
        s1.merge(&s2);
        assert!(s1.contains(&"a".to_string()));
        assert!(s1.contains(&"b".to_string()));
        assert_eq!(s1.len(), 2);
    }

    #[test]
    fn test_or_set() {
        let mut s: ORSet<String> = ORSet::new();
        s.insert("x".to_string());
        assert!(s.contains(&"x".to_string()));
        s.remove(&"x".to_string());
        assert!(!s.contains(&"x".to_string()));
    }

    #[test]
    fn test_lww_register() {
        let mut reg: LWWRegister<String> = LWWRegister::new();
        reg.set("first".to_string(), 1);
        reg.set("second".to_string(), 2);
        reg.set("stale".to_string(), 1); // ignored (older)
        assert_eq!(reg.get(), Some(&"second".to_string()));
    }

    #[test]
    fn test_hash_ring() {
        let mut ring = HashRing::new(10);
        ring.add_node("node1");
        ring.add_node("node2");
        ring.add_node("node3");
        assert_eq!(ring.node_count(), 3);
        let node = ring.get_node("mykey").unwrap();
        assert!(["node1", "node2", "node3"].contains(&node));
    }

    #[test]
    fn test_hash_ring_replication() {
        let mut ring = HashRing::new(10);
        ring.add_node("a");
        ring.add_node("b");
        ring.add_node("c");
        let nodes = ring.get_nodes("key", 2);
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_hash_ring_remove() {
        let mut ring = HashRing::new(5);
        ring.add_node("n1");
        ring.add_node("n2");
        ring.remove_node("n1");
        assert_eq!(ring.node_count(), 1);
    }

    #[test]
    fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(3, 2);
        assert!(cb.allow_request());
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens() {
        let mut cb = CircuitBreaker::new(3, 2);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_circuit_breaker_half_open_recovery() {
        let mut cb = CircuitBreaker::new(2, 2);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
        cb.try_half_open();
        assert_eq!(cb.state, CircuitState::HalfOpen);
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn test_bulkhead() {
        let mut bh = Bulkhead::new("api", 2, 1);
        assert!(bh.try_acquire());
        assert!(bh.try_acquire());
        assert!(bh.try_acquire()); // queued
        assert!(!bh.try_acquire()); // rejected
        bh.release();
        assert!(bh.try_acquire());
    }

    #[test]
    fn test_retry_success() {
        let policy = RetryPolicy::default();
        let mut attempt = 0;
        let result = retry_sync(&policy, || {
            attempt += 1;
            if attempt < 3 {
                Err("fail".to_string())
            } else {
                Ok(42)
            }
        });
        assert!(result.result.is_ok());
        assert_eq!(result.result.unwrap(), 42);
        assert_eq!(result.attempts, 3);
    }

    #[test]
    fn test_retry_exhausted() {
        let policy = RetryPolicy { max_retries: 2, ..Default::default() };
        let result: RetryResult<i32> = retry_sync(&policy, || {
            Err("always fails".to_string())
        });
        assert!(result.result.is_err());
    }

    #[test]
    fn test_saga_complete() {
        let mut saga = Saga::new("order", &["reserve", "charge", "ship"]);
        saga.execute_step(StepResult::Success);
        saga.execute_step(StepResult::Success);
        saga.execute_step(StepResult::Success);
        assert_eq!(saga.status, SagaStatus::Completed);
    }

    #[test]
    fn test_saga_compensate() {
        let mut saga = Saga::new("order", &["reserve", "charge", "ship"]);
        saga.execute_step(StepResult::Success);
        saga.execute_step(StepResult::Failure("card declined".into()));
        assert_eq!(saga.status, SagaStatus::Compensating);
        saga.compensate_step(); // compensate "reserve"
        assert_eq!(saga.status, SagaStatus::Compensated);
    }

    #[test]
    fn test_backoff_policy() {
        let policy = RetryPolicy {
            initial_delay_ms: 100,
            backoff_multiplier: 2.0,
            jitter: false,
            ..Default::default()
        };
        assert_eq!(policy.delay_ms(0), 100);
        assert_eq!(policy.delay_ms(1), 200);
        assert_eq!(policy.delay_ms(2), 400);
    }
}
