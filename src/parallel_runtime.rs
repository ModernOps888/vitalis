//! Parallel runtime for Vitalis.
//!
//! Implements a parallel execution runtime:
//! - **Thread pool**: Fixed-size pool with work distribution
//! - **Parallel for**: Data-parallel loop with chunked scheduling
//! - **Parallel reduce**: Reduction with associative combiner
//! - **Parallel scan**: Prefix sum (inclusive/exclusive)
//! - **Task graph**: DAG-based task scheduling with dependencies
//! - **Fork-join**: Recursive parallel decomposition
//! - **Barrier**: Synchronization primitive

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering}};

// ── Task ────────────────────────────────────────────────────────────

pub type TaskId = u64;

/// A task in the parallel runtime.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub status: TaskStatus,
    pub priority: usize,
    pub result: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
}

impl Task {
    pub fn new(id: TaskId, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            status: TaskStatus::Pending,
            priority: 0,
            result: None,
        }
    }
}

// ── Thread Pool ─────────────────────────────────────────────────────

/// A simulated thread pool for task execution.
pub struct ThreadPool {
    num_workers: usize,
    task_queue: VecDeque<Task>,
    completed: Vec<Task>,
    active_count: AtomicUsize,
    total_submitted: AtomicU64,
    shutdown: AtomicBool,
}

impl ThreadPool {
    pub fn new(num_workers: usize) -> Self {
        Self {
            num_workers,
            task_queue: VecDeque::new(),
            completed: Vec::new(),
            active_count: AtomicUsize::new(0),
            total_submitted: AtomicU64::new(0),
            shutdown: AtomicBool::new(false),
        }
    }

    /// Submit a task to the pool.
    pub fn submit(&mut self, mut task: Task) {
        task.status = TaskStatus::Ready;
        self.task_queue.push_back(task);
        self.total_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Execute one batch of tasks (up to num_workers).
    pub fn execute_batch(&mut self) {
        let batch_size = self.task_queue.len().min(self.num_workers);
        for _ in 0..batch_size {
            if let Some(mut task) = self.task_queue.pop_front() {
                self.active_count.fetch_add(1, Ordering::Relaxed);
                task.status = TaskStatus::Running;
                // Simulate execution.
                task.status = TaskStatus::Completed;
                task.result = Some(task.id as i64);
                self.active_count.fetch_sub(1, Ordering::Relaxed);
                self.completed.push(task);
            }
        }
    }

    /// Execute all submitted tasks.
    pub fn execute_all(&mut self) {
        while !self.task_queue.is_empty() {
            self.execute_batch();
        }
    }

    pub fn pending_count(&self) -> usize {
        self.task_queue.len()
    }

    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    pub fn num_workers(&self) -> usize {
        self.num_workers
    }

    pub fn completed_tasks(&self) -> &[Task] {
        &self.completed
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }
}

// ── Parallel For ────────────────────────────────────────────────────

/// Scheduling strategy for parallel loops.
#[derive(Debug, Clone, Copy)]
pub enum Schedule {
    Static,        // Equal-sized chunks.
    Dynamic(usize), // Chunk size for dynamic distribution.
    Guided,        // Decreasing chunk sizes.
}

/// Result of a parallel for execution.
#[derive(Debug, Clone)]
pub struct ParallelForResult {
    pub total_iterations: usize,
    pub chunks_executed: usize,
    pub results: Vec<i64>,
}

/// Execute a parallel for loop.
pub fn parallel_for(
    start: i64,
    end: i64,
    num_workers: usize,
    schedule: Schedule,
    f: &dyn Fn(i64) -> i64,
) -> ParallelForResult {
    let total = (end - start) as usize;
    let chunks = make_chunks(start, end, num_workers, schedule);
    let mut results = Vec::with_capacity(total);

    for (lo, hi) in &chunks {
        for i in *lo..*hi {
            results.push(f(i));
        }
    }

    ParallelForResult {
        total_iterations: total,
        chunks_executed: chunks.len(),
        results,
    }
}

fn make_chunks(start: i64, end: i64, num_workers: usize, schedule: Schedule) -> Vec<(i64, i64)> {
    let total = (end - start) as usize;
    match schedule {
        Schedule::Static => {
            let chunk_size = (total + num_workers - 1) / num_workers;
            let mut chunks = Vec::new();
            let mut pos = start;
            while pos < end {
                let hi = (pos + chunk_size as i64).min(end);
                chunks.push((pos, hi));
                pos = hi;
            }
            chunks
        }
        Schedule::Dynamic(chunk_size) => {
            let mut chunks = Vec::new();
            let mut pos = start;
            while pos < end {
                let hi = (pos + chunk_size as i64).min(end);
                chunks.push((pos, hi));
                pos = hi;
            }
            chunks
        }
        Schedule::Guided => {
            let mut chunks = Vec::new();
            let mut remaining = total;
            let mut pos = start;
            while remaining > 0 {
                let chunk = (remaining / num_workers).max(1);
                let hi = (pos + chunk as i64).min(end);
                chunks.push((pos, hi));
                remaining -= (hi - pos) as usize;
                pos = hi;
            }
            chunks
        }
    }
}

// ── Parallel Reduce ─────────────────────────────────────────────────

/// Parallel reduction with an associative combiner.
pub fn parallel_reduce<T: Clone + Send>(
    data: &[T],
    identity: T,
    combine: &dyn Fn(&T, &T) -> T,
    num_workers: usize,
) -> T {
    if data.is_empty() {
        return identity;
    }

    let chunk_size = (data.len() + num_workers - 1) / num_workers;
    let mut partial_results = Vec::new();

    for chunk in data.chunks(chunk_size) {
        let mut acc = identity.clone();
        for item in chunk {
            acc = combine(&acc, item);
        }
        partial_results.push(acc);
    }

    // Final reduction.
    let mut result = identity;
    for partial in &partial_results {
        result = combine(&result, partial);
    }
    result
}

// ── Parallel Scan (Prefix Sum) ──────────────────────────────────────

/// Inclusive prefix scan.
pub fn parallel_scan_inclusive<T: Clone + Send>(
    data: &[T],
    combine: &dyn Fn(&T, &T) -> T,
    _num_workers: usize,
) -> Vec<T> {
    if data.is_empty() {
        return Vec::new();
    }

    // Blelloch-style scan (sequential here, but structured for parallelism).
    let mut result = Vec::with_capacity(data.len());
    result.push(data[0].clone());
    for i in 1..data.len() {
        let prev = &result[i - 1];
        result.push(combine(prev, &data[i]));
    }
    result
}

/// Exclusive prefix scan.
pub fn parallel_scan_exclusive<T: Clone + Send>(
    data: &[T],
    identity: T,
    combine: &dyn Fn(&T, &T) -> T,
    _num_workers: usize,
) -> Vec<T> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(data.len());
    result.push(identity.clone());
    for i in 1..data.len() {
        let prev = &result[i - 1];
        result.push(combine(prev, &data[i - 1]));
    }
    result
}

// ── Task Graph ──────────────────────────────────────────────────────

/// A DAG-based task scheduler.
pub struct TaskGraph {
    pub tasks: HashMap<TaskId, Task>,
    pub edges: HashMap<TaskId, Vec<TaskId>>,   // task → dependencies
    pub reverse: HashMap<TaskId, Vec<TaskId>>,  // task → dependents
    next_id: TaskId,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            edges: HashMap::new(),
            reverse: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add a task to the graph.
    pub fn add_task(&mut self, name: &str) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.insert(id, Task::new(id, name));
        self.edges.insert(id, Vec::new());
        self.reverse.insert(id, Vec::new());
        id
    }

    /// Add a dependency: `dependent` depends on `dependency`.
    pub fn add_dependency(&mut self, dependent: TaskId, dependency: TaskId) {
        self.edges.entry(dependent).or_default().push(dependency);
        self.reverse.entry(dependency).or_default().push(dependent);
    }

    /// Get tasks that are ready to execute (all dependencies completed).
    pub fn ready_tasks(&self) -> Vec<TaskId> {
        self.tasks.iter()
            .filter(|(_, t)| t.status == TaskStatus::Pending || t.status == TaskStatus::Ready)
            .filter(|(id, _)| {
                self.edges.get(id)
                    .map(|deps| deps.iter().all(|d| {
                        self.tasks.get(d)
                            .map(|t| t.status == TaskStatus::Completed)
                            .unwrap_or(true)
                    }))
                    .unwrap_or(true)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Mark a task as completed.
    pub fn complete_task(&mut self, id: TaskId, result: i64) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.status = TaskStatus::Completed;
            task.result = Some(result);
        }
    }

    /// Execute all tasks in topological order.
    pub fn execute_all(&mut self) {
        loop {
            let ready = self.ready_tasks();
            if ready.is_empty() {
                break;
            }
            for id in ready {
                if let Some(task) = self.tasks.get_mut(&id) {
                    task.status = TaskStatus::Running;
                    task.status = TaskStatus::Completed;
                    task.result = Some(task.id as i64);
                }
            }
        }
    }

    /// Topological sort of the task graph.
    pub fn topological_sort(&self) -> Vec<TaskId> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();

        for &id in self.tasks.keys() {
            self.topo_dfs(id, &mut visited, &mut order);
        }

        order
    }

    fn topo_dfs(&self, id: TaskId, visited: &mut HashSet<TaskId>, order: &mut Vec<TaskId>) {
        if visited.contains(&id) {
            return;
        }
        visited.insert(id);
        if let Some(deps) = self.edges.get(&id) {
            for &dep in deps {
                self.topo_dfs(dep, visited, order);
            }
        }
        order.push(id);
    }

    /// Detect cycles in the task graph.
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();

        for &id in self.tasks.keys() {
            if self.cycle_dfs(id, &mut visited, &mut in_stack) {
                return true;
            }
        }
        false
    }

    fn cycle_dfs(
        &self,
        id: TaskId,
        visited: &mut HashSet<TaskId>,
        in_stack: &mut HashSet<TaskId>,
    ) -> bool {
        if in_stack.contains(&id) {
            return true;
        }
        if visited.contains(&id) {
            return false;
        }
        visited.insert(id);
        in_stack.insert(id);
        if let Some(deps) = self.edges.get(&id) {
            for &dep in deps {
                if self.cycle_dfs(dep, visited, in_stack) {
                    return true;
                }
            }
        }
        in_stack.remove(&id);
        false
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn completed_count(&self) -> usize {
        self.tasks.values().filter(|t| t.status == TaskStatus::Completed).count()
    }
}

// ── Fork-Join ───────────────────────────────────────────────────────

/// A fork-join computation tree.
#[derive(Debug, Clone)]
pub enum ForkJoin<T: Clone> {
    Leaf(T),
    Fork(Box<ForkJoin<T>>, Box<ForkJoin<T>>),
}

impl<T: Clone> ForkJoin<T> {
    pub fn leaf(value: T) -> Self {
        ForkJoin::Leaf(value)
    }

    pub fn fork(left: ForkJoin<T>, right: ForkJoin<T>) -> Self {
        ForkJoin::Fork(Box::new(left), Box::new(right))
    }

    /// Execute the fork-join tree with a combine function.
    pub fn execute(&self, combine: &dyn Fn(&T, &T) -> T) -> T {
        match self {
            ForkJoin::Leaf(v) => v.clone(),
            ForkJoin::Fork(left, right) => {
                let l = left.execute(combine);
                let r = right.execute(combine);
                combine(&l, &r)
            }
        }
    }

    /// Count the total number of leaves.
    pub fn leaf_count(&self) -> usize {
        match self {
            ForkJoin::Leaf(_) => 1,
            ForkJoin::Fork(l, r) => l.leaf_count() + r.leaf_count(),
        }
    }
}

/// Build a fork-join tree from a slice using recursive decomposition.
pub fn fork_join_from_slice<T: Clone>(data: &[T], threshold: usize) -> ForkJoin<T> {
    if data.len() <= threshold {
        // Base case: create leaves.
        if data.len() == 1 {
            return ForkJoin::leaf(data[0].clone());
        }
        // Multi-element leaf — fold into pairs.
        let mid = data.len() / 2;
        ForkJoin::fork(
            fork_join_from_slice(&data[..mid], threshold),
            fork_join_from_slice(&data[mid..], threshold),
        )
    } else {
        let mid = data.len() / 2;
        ForkJoin::fork(
            fork_join_from_slice(&data[..mid], threshold),
            fork_join_from_slice(&data[mid..], threshold),
        )
    }
}

// ── Barrier ─────────────────────────────────────────────────────────

/// A barrier for synchronizing parallel tasks.
pub struct Barrier {
    total: usize,
    arrived: AtomicUsize,
    generation: AtomicU64,
}

impl Barrier {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            arrived: AtomicUsize::new(0),
            generation: AtomicU64::new(0),
        }
    }

    /// Arrive at the barrier. Returns true if this was the last arrival.
    pub fn arrive(&self) -> bool {
        let prev = self.arrived.fetch_add(1, Ordering::AcqRel);
        if prev + 1 >= self.total {
            self.arrived.store(0, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
            true
        } else {
            false
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    pub fn arrived_count(&self) -> usize {
        self.arrived.load(Ordering::Acquire)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_basic() {
        let mut pool = ThreadPool::new(4);
        pool.submit(Task::new(1, "task1"));
        pool.submit(Task::new(2, "task2"));
        pool.execute_all();
        assert_eq!(pool.completed_count(), 2);
        assert_eq!(pool.pending_count(), 0);
    }

    #[test]
    fn test_thread_pool_batch() {
        let mut pool = ThreadPool::new(2);
        for i in 0..5 {
            pool.submit(Task::new(i + 1, &format!("t{i}")));
        }
        pool.execute_batch(); // 2 tasks
        assert_eq!(pool.completed_count(), 2);
        assert_eq!(pool.pending_count(), 3);
    }

    #[test]
    fn test_parallel_for_static() {
        let result = parallel_for(0, 10, 4, Schedule::Static, &|i| i * i);
        assert_eq!(result.total_iterations, 10);
        assert_eq!(result.results.len(), 10);
        assert_eq!(result.results[3], 9);
    }

    #[test]
    fn test_parallel_for_dynamic() {
        let result = parallel_for(0, 8, 2, Schedule::Dynamic(3), &|i| i + 1);
        assert_eq!(result.results, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_parallel_for_guided() {
        let result = parallel_for(0, 10, 3, Schedule::Guided, &|i| i);
        assert_eq!(result.results.len(), 10);
    }

    #[test]
    fn test_parallel_reduce() {
        let data: Vec<i64> = (1..=100).collect();
        let sum = parallel_reduce(&data, 0, &|a, b| a + b, 4);
        assert_eq!(sum, 5050);
    }

    #[test]
    fn test_parallel_reduce_empty() {
        let data: Vec<i64> = vec![];
        let sum = parallel_reduce(&data, 0, &|a, b| a + b, 4);
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_scan_inclusive() {
        let data = vec![1, 2, 3, 4, 5];
        let result = parallel_scan_inclusive(&data, &|a, b| a + b, 2);
        assert_eq!(result, vec![1, 3, 6, 10, 15]);
    }

    #[test]
    fn test_scan_exclusive() {
        let data = vec![1, 2, 3, 4, 5];
        let result = parallel_scan_exclusive(&data, 0, &|a, b| a + b, 2);
        assert_eq!(result, vec![0, 1, 3, 6, 10]);
    }

    #[test]
    fn test_task_graph_basic() {
        let mut graph = TaskGraph::new();
        let a = graph.add_task("A");
        let b = graph.add_task("B");
        let c = graph.add_task("C");
        graph.add_dependency(c, a);
        graph.add_dependency(c, b);

        // A and B should be ready, C should not.
        let ready = graph.ready_tasks();
        assert!(ready.contains(&a));
        assert!(ready.contains(&b));
        assert!(!ready.contains(&c));
    }

    #[test]
    fn test_task_graph_execute_all() {
        let mut graph = TaskGraph::new();
        let a = graph.add_task("A");
        let b = graph.add_task("B");
        let c = graph.add_task("C");
        graph.add_dependency(c, a);
        graph.add_dependency(c, b);
        graph.execute_all();
        assert_eq!(graph.completed_count(), 3);
    }

    #[test]
    fn test_task_graph_topological_sort() {
        let mut graph = TaskGraph::new();
        let a = graph.add_task("A");
        let b = graph.add_task("B");
        let c = graph.add_task("C");
        graph.add_dependency(b, a);
        graph.add_dependency(c, b);
        let order = graph.topological_sort();
        let pos_a = order.iter().position(|&x| x == a).unwrap();
        let pos_b = order.iter().position(|&x| x == b).unwrap();
        let pos_c = order.iter().position(|&x| x == c).unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_task_graph_no_cycle() {
        let mut graph = TaskGraph::new();
        let a = graph.add_task("A");
        let b = graph.add_task("B");
        graph.add_dependency(b, a);
        assert!(!graph.has_cycle());
    }

    #[test]
    fn test_fork_join_sum() {
        let tree = ForkJoin::fork(
            ForkJoin::fork(ForkJoin::leaf(1), ForkJoin::leaf(2)),
            ForkJoin::fork(ForkJoin::leaf(3), ForkJoin::leaf(4)),
        );
        let sum = tree.execute(&|a, b| a + b);
        assert_eq!(sum, 10);
    }

    #[test]
    fn test_fork_join_from_slice() {
        let data: Vec<i64> = (1..=8).collect();
        let tree = fork_join_from_slice(&data, 2);
        let sum = tree.execute(&|a, b| a + b);
        assert_eq!(sum, 36);
        assert_eq!(tree.leaf_count(), 8);
    }

    #[test]
    fn test_barrier() {
        let barrier = Barrier::new(3);
        assert!(!barrier.arrive());
        assert!(!barrier.arrive());
        assert!(barrier.arrive()); // last one triggers
        assert_eq!(barrier.generation(), 1);
    }

    #[test]
    fn test_thread_pool_shutdown() {
        let pool = ThreadPool::new(2);
        assert!(!pool.is_shutdown());
        pool.shutdown();
        assert!(pool.is_shutdown());
    }

    #[test]
    fn test_task_status_transitions() {
        let mut task = Task::new(1, "test");
        assert_eq!(task.status, TaskStatus::Pending);
        task.status = TaskStatus::Ready;
        assert_eq!(task.status, TaskStatus::Ready);
        task.status = TaskStatus::Running;
        task.status = TaskStatus::Completed;
        task.result = Some(42);
        assert_eq!(task.result, Some(42));
    }

    #[test]
    fn test_parallel_reduce_product() {
        let data: Vec<i64> = vec![1, 2, 3, 4, 5];
        let product = parallel_reduce(&data, 1, &|a, b| a * b, 2);
        assert_eq!(product, 120);
    }

    #[test]
    fn test_scan_empty() {
        let data: Vec<i64> = vec![];
        let result = parallel_scan_inclusive(&data, &|a, b| a + b, 2);
        assert!(result.is_empty());
    }
}
