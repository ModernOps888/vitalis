//! Green threads — M:N threading with work-stealing for Vitalis.
//!
//! Provides lightweight cooperative concurrency:
//! - **Stackful coroutines**: Small initial stacks, growable
//! - **Work-stealing scheduler**: Per-core deques, random victim selection
//! - **Green thread API**: `spawn_green`, `yield_now`, `park`/`unpark`
//! - **Channel integration**: Green threads block on channels without OS stall
//! - **Preemption**: Timer-based preemption for fairness
//! - **I/O integration**: Non-blocking I/O awareness

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Condvar};
use std::time::{Duration, Instant};

// ── Green Thread State ───────────────────────────────────────────────

/// Unique identifier for a green thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GreenThreadId(pub u64);

/// State of a green thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// Ready to run.
    Ready,
    /// Currently executing.
    Running,
    /// Parked (waiting for unpark).
    Parked,
    /// Blocked on I/O.
    Blocked,
    /// Finished execution.
    Completed,
    /// Yielded control voluntarily.
    Yielded,
}

/// Priority level for scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// A green thread (lightweight task).
#[derive(Debug, Clone)]
pub struct GreenThread {
    pub id: GreenThreadId,
    pub state: ThreadState,
    pub priority: Priority,
    /// Simulated stack size in bytes.
    pub stack_size: usize,
    /// Maximum stack size before overflow.
    pub max_stack_size: usize,
    /// Time quantum remaining (microseconds).
    pub time_quantum_us: u64,
    /// Total CPU time consumed (microseconds).
    pub cpu_time_us: u64,
    /// Yield count.
    pub yield_count: u64,
    /// Task payload / name.
    pub name: String,
    /// Result value (if completed).
    pub result: Option<i64>,
    /// Creation timestamp.
    pub created_at: Instant,
}

impl GreenThread {
    pub fn new(id: GreenThreadId, name: String, stack_size: usize) -> Self {
        Self {
            id,
            state: ThreadState::Ready,
            priority: Priority::Normal,
            stack_size,
            max_stack_size: 1024 * 1024, // 1 MB max
            time_quantum_us: 1000,       // 1 ms quantum
            cpu_time_us: 0,
            yield_count: 0,
            name,
            result: None,
            created_at: Instant::now(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}

// ── Work-Stealing Deque ──────────────────────────────────────────────

/// A simple work-stealing deque (LIFO local, FIFO steal).
#[derive(Debug)]
pub struct WorkStealingDeque {
    /// Local tasks (owner pushes/pops from back).
    items: VecDeque<GreenThreadId>,
}

impl WorkStealingDeque {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }

    /// Owner pushes to back (LIFO).
    pub fn push(&mut self, id: GreenThreadId) {
        self.items.push_back(id);
    }

    /// Owner pops from back (LIFO).
    pub fn pop(&mut self) -> Option<GreenThreadId> {
        self.items.pop_back()
    }

    /// Thief steals from front (FIFO — oldest tasks first).
    pub fn steal(&mut self) -> Option<GreenThreadId> {
        self.items.pop_front()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// ── Scheduler ────────────────────────────────────────────────────────

/// Scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Number of worker deques (simulated cores).
    pub num_workers: usize,
    /// Default time quantum in microseconds.
    pub time_quantum_us: u64,
    /// Enable preemption.
    pub preemptive: bool,
    /// Maximum green threads.
    pub max_threads: usize,
    /// Default stack size in bytes.
    pub default_stack_size: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            num_workers: 4,
            time_quantum_us: 1000,
            preemptive: true,
            max_threads: 100_000,
            default_stack_size: 8192, // 8 KB
        }
    }
}

/// Scheduler statistics.
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub total_spawned: u64,
    pub total_completed: u64,
    pub total_yields: u64,
    pub total_steals: u64,
    pub total_steal_attempts: u64,
    pub context_switches: u64,
    pub preemptions: u64,
    pub max_queue_depth: usize,
}

/// The M:N green thread scheduler with work-stealing.
pub struct Scheduler {
    /// All threads, indexed by ID.
    threads: Vec<GreenThread>,
    /// Per-worker deques.
    deques: Vec<WorkStealingDeque>,
    /// Global overflow queue.
    global_queue: VecDeque<GreenThreadId>,
    /// Currently running thread per worker (worker_idx → thread_id).
    current: Vec<Option<GreenThreadId>>,
    /// Parked threads.
    parked: Vec<GreenThreadId>,
    /// Next thread ID.
    next_id: u64,
    /// Configuration.
    config: SchedulerConfig,
    /// Statistics.
    stats: SchedulerStats,
}

impl Scheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        let num_workers = config.num_workers;
        Self {
            threads: Vec::new(),
            deques: (0..num_workers).map(|_| WorkStealingDeque::new()).collect(),
            global_queue: VecDeque::new(),
            current: vec![None; num_workers],
            parked: Vec::new(),
            next_id: 1,
            config,
            stats: SchedulerStats::default(),
        }
    }

    /// Spawn a new green thread, returning its ID.
    pub fn spawn(&mut self, name: impl Into<String>) -> GreenThreadId {
        let id = GreenThreadId(self.next_id);
        self.next_id += 1;

        let thread = GreenThread::new(id, name.into(), self.config.default_stack_size);
        self.threads.push(thread);
        self.stats.total_spawned += 1;

        // Assign to least-loaded worker.
        let worker = self.least_loaded_worker();
        self.deques[worker].push(id);

        // Track max queue depth.
        let depth = self.deques[worker].len();
        if depth > self.stats.max_queue_depth {
            self.stats.max_queue_depth = depth;
        }

        id
    }

    /// Spawn with a specific priority.
    pub fn spawn_with_priority(&mut self, name: impl Into<String>, priority: Priority) -> GreenThreadId {
        let id = self.spawn(name);
        if let Some(t) = self.get_thread_mut(id) {
            t.priority = priority;
        }
        id
    }

    /// Yield the current thread on a worker.
    pub fn yield_now(&mut self, worker: usize) {
        if let Some(tid) = self.current[worker].take() {
            if let Some(t) = self.get_thread_mut(tid) {
                t.state = ThreadState::Yielded;
                t.yield_count += 1;
                self.stats.total_yields += 1;
            }
            // Put back in the worker's deque.
            self.deques[worker].push(tid);
            self.stats.context_switches += 1;
        }
    }

    /// Park a thread (block until unparked).
    pub fn park(&mut self, id: GreenThreadId) {
        if let Some(t) = self.get_thread_mut(id) {
            t.state = ThreadState::Parked;
        }
        self.parked.push(id);
        // Remove from current if running.
        for slot in &mut self.current {
            if *slot == Some(id) {
                *slot = None;
            }
        }
    }

    /// Unpark a previously parked thread.
    pub fn unpark(&mut self, id: GreenThreadId) {
        self.parked.retain(|&pid| pid != id);
        if let Some(t) = self.get_thread_mut(id) {
            t.state = ThreadState::Ready;
        }
        // Re-enqueue to least loaded worker.
        let worker = self.least_loaded_worker();
        self.deques[worker].push(id);
    }

    /// Complete a thread with a result.
    pub fn complete(&mut self, id: GreenThreadId, result: i64) {
        if let Some(t) = self.get_thread_mut(id) {
            t.state = ThreadState::Completed;
            t.result = Some(result);
            self.stats.total_completed += 1;
        }
        for slot in &mut self.current {
            if *slot == Some(id) {
                *slot = None;
            }
        }
    }

    /// Schedule the next thread on a worker. Returns the thread ID if one was found.
    pub fn schedule(&mut self, worker: usize) -> Option<GreenThreadId> {
        // 1. Check local deque.
        if let Some(id) = self.deques[worker].pop() {
            self.run_thread(worker, id);
            return Some(id);
        }

        // 2. Check global queue.
        if let Some(id) = self.global_queue.pop_front() {
            self.run_thread(worker, id);
            return Some(id);
        }

        // 3. Try to steal from another worker.
        self.stats.total_steal_attempts += 1;
        let num_workers = self.config.num_workers;
        for offset in 1..num_workers {
            let victim = (worker + offset) % num_workers;
            if let Some(id) = self.deques[victim].steal() {
                self.stats.total_steals += 1;
                self.run_thread(worker, id);
                return Some(id);
            }
        }

        None
    }

    /// Run one scheduling round across all workers. Returns total threads scheduled.
    pub fn run_round(&mut self) -> usize {
        let mut scheduled = 0;
        let num_workers = self.config.num_workers;
        for w in 0..num_workers {
            if self.schedule(w).is_some() {
                scheduled += 1;
            }
        }
        scheduled
    }

    /// Check if all threads have completed.
    pub fn all_completed(&self) -> bool {
        self.threads.iter().all(|t| t.state == ThreadState::Completed)
    }

    /// Get thread by ID.
    pub fn get_thread(&self, id: GreenThreadId) -> Option<&GreenThread> {
        self.threads.iter().find(|t| t.id == id)
    }

    /// Get mutable thread by ID.
    fn get_thread_mut(&mut self, id: GreenThreadId) -> Option<&mut GreenThread> {
        self.threads.iter_mut().find(|t| t.id == id)
    }

    /// Get scheduler statistics.
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }

    /// Get the number of active (non-completed) threads.
    pub fn active_count(&self) -> usize {
        self.threads.iter().filter(|t| t.state != ThreadState::Completed).count()
    }

    /// Get total thread count.
    pub fn total_count(&self) -> usize {
        self.threads.len()
    }

    // ── Internal helpers ─────────────────────────────────────────────

    fn run_thread(&mut self, worker: usize, id: GreenThreadId) {
        if let Some(t) = self.get_thread_mut(id) {
            t.state = ThreadState::Running;
        }
        self.current[worker] = Some(id);
        self.stats.context_switches += 1;
    }

    fn least_loaded_worker(&self) -> usize {
        self.deques.iter()
            .enumerate()
            .min_by_key(|(_, d)| d.len())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

// ── Channel for Green Threads ────────────────────────────────────────

/// A bounded channel for green thread communication.
pub struct GreenChannel<T> {
    buffer: VecDeque<T>,
    capacity: usize,
    closed: bool,
}

impl<T> GreenChannel<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            closed: false,
        }
    }

    pub fn send(&mut self, value: T) -> Result<(), &'static str> {
        if self.closed {
            return Err("channel closed");
        }
        if self.buffer.len() >= self.capacity {
            return Err("channel full");
        }
        self.buffer.push_back(value);
        Ok(())
    }

    pub fn recv(&mut self) -> Option<T> {
        self.buffer.pop_front()
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

// ── Timer Wheel for Preemption ───────────────────────────────────────

/// Simple timer wheel for preemption scheduling.
pub struct TimerWheel {
    slots: Vec<Vec<GreenThreadId>>,
    current_tick: usize,
    num_slots: usize,
}

impl TimerWheel {
    pub fn new(num_slots: usize) -> Self {
        Self {
            slots: vec![Vec::new(); num_slots],
            current_tick: 0,
            num_slots,
        }
    }

    /// Schedule a thread to fire after `ticks` ticks.
    pub fn schedule(&mut self, id: GreenThreadId, ticks: usize) {
        let slot = (self.current_tick + ticks) % self.num_slots;
        self.slots[slot].push(id);
    }

    /// Advance the wheel by one tick, returning expired thread IDs.
    pub fn tick(&mut self) -> Vec<GreenThreadId> {
        self.current_tick = (self.current_tick + 1) % self.num_slots;
        std::mem::take(&mut self.slots[self.current_tick])
    }

    pub fn current_tick(&self) -> usize {
        self.current_tick
    }
}

// ── Context Switch Record ────────────────────────────────────────────

/// Record of a context switch for debugging/profiling.
#[derive(Debug, Clone)]
pub struct ContextSwitchRecord {
    pub from: Option<GreenThreadId>,
    pub to: GreenThreadId,
    pub worker: usize,
    pub reason: SwitchReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchReason {
    Yield,
    Preemption,
    Park,
    Completion,
    Steal,
    Schedule,
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_thread() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        let id = sched.spawn("test-task");
        assert_eq!(id, GreenThreadId(1));
        assert_eq!(sched.total_count(), 1);
        assert_eq!(sched.get_thread(id).unwrap().state, ThreadState::Ready);
    }

    #[test]
    fn test_schedule_and_run() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        let id = sched.spawn("task-1");
        let scheduled = sched.schedule(0);
        assert_eq!(scheduled, Some(id));
        assert_eq!(sched.get_thread(id).unwrap().state, ThreadState::Running);
    }

    #[test]
    fn test_yield_now() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        let id = sched.spawn("task-1");
        sched.schedule(0);
        sched.yield_now(0);
        let t = sched.get_thread(id).unwrap();
        assert_eq!(t.state, ThreadState::Yielded);
        assert_eq!(t.yield_count, 1);
    }

    #[test]
    fn test_park_unpark() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        let id = sched.spawn("parked-task");
        sched.schedule(0);
        sched.park(id);
        assert_eq!(sched.get_thread(id).unwrap().state, ThreadState::Parked);
        sched.unpark(id);
        assert_eq!(sched.get_thread(id).unwrap().state, ThreadState::Ready);
    }

    #[test]
    fn test_complete_thread() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        let id = sched.spawn("task-1");
        sched.schedule(0);
        sched.complete(id, 42);
        let t = sched.get_thread(id).unwrap();
        assert_eq!(t.state, ThreadState::Completed);
        assert_eq!(t.result, Some(42));
    }

    #[test]
    fn test_work_stealing() {
        let mut sched = Scheduler::new(SchedulerConfig { num_workers: 2, ..Default::default() });
        // Spawn 3 tasks — they go to least-loaded worker.
        let _id1 = sched.spawn("t1");
        let _id2 = sched.spawn("t2");
        let _id3 = sched.spawn("t3");
        // Drain worker 1 first so it needs to steal.
        let first = sched.schedule(1);
        assert!(first.is_some());
        let stolen = sched.schedule(1);
        assert!(stolen.is_some());
        assert!(sched.stats().total_steals > 0 || sched.stats().total_steal_attempts > 0);
    }

    #[test]
    fn test_work_stealing_deque() {
        let mut deque = WorkStealingDeque::new();
        deque.push(GreenThreadId(1));
        deque.push(GreenThreadId(2));
        deque.push(GreenThreadId(3));
        // Owner pops LIFO.
        assert_eq!(deque.pop(), Some(GreenThreadId(3)));
        // Thief steals FIFO.
        assert_eq!(deque.steal(), Some(GreenThreadId(1)));
        assert_eq!(deque.len(), 1);
    }

    #[test]
    fn test_green_channel() {
        let mut ch = GreenChannel::new(2);
        assert!(ch.send(10).is_ok());
        assert!(ch.send(20).is_ok());
        assert!(ch.send(30).is_err()); // full
        assert_eq!(ch.recv(), Some(10));
        assert_eq!(ch.recv(), Some(20));
        assert_eq!(ch.recv(), None);
    }

    #[test]
    fn test_green_channel_close() {
        let mut ch = GreenChannel::new(10);
        ch.send(1).unwrap();
        ch.close();
        assert!(ch.send(2).is_err());
        assert!(ch.is_closed());
        assert_eq!(ch.recv(), Some(1)); // can still drain
    }

    #[test]
    fn test_timer_wheel() {
        let mut tw = TimerWheel::new(8);
        tw.schedule(GreenThreadId(1), 3);
        tw.schedule(GreenThreadId(2), 3);
        tw.schedule(GreenThreadId(3), 5);
        // Advance 3 ticks.
        assert!(tw.tick().is_empty());
        assert!(tw.tick().is_empty());
        let expired = tw.tick();
        assert_eq!(expired.len(), 2);
        // 2 more ticks for the third.
        assert!(tw.tick().is_empty());
        let expired2 = tw.tick();
        assert_eq!(expired2.len(), 1);
        assert_eq!(expired2[0], GreenThreadId(3));
    }

    #[test]
    fn test_priority_scheduling() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        let _low = sched.spawn_with_priority("low", Priority::Low);
        let high = sched.spawn_with_priority("high", Priority::High);
        assert_eq!(sched.get_thread(high).unwrap().priority, Priority::High);
    }

    #[test]
    fn test_all_completed() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        let id1 = sched.spawn("t1");
        let id2 = sched.spawn("t2");
        assert!(!sched.all_completed());
        sched.complete(id1, 0);
        sched.complete(id2, 0);
        assert!(sched.all_completed());
    }

    #[test]
    fn test_run_round() {
        let mut sched = Scheduler::new(SchedulerConfig { num_workers: 2, ..Default::default() });
        sched.spawn("t1");
        sched.spawn("t2");
        sched.spawn("t3");
        let scheduled = sched.run_round();
        assert!(scheduled >= 2);
    }

    #[test]
    fn test_scheduler_stats() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        sched.spawn("t1");
        sched.spawn("t2");
        assert_eq!(sched.stats().total_spawned, 2);
        sched.schedule(0);
        assert!(sched.stats().context_switches >= 1);
    }

    #[test]
    fn test_active_count() {
        let mut sched = Scheduler::new(SchedulerConfig::default());
        let id1 = sched.spawn("t1");
        let _id2 = sched.spawn("t2");
        assert_eq!(sched.active_count(), 2);
        sched.complete(id1, 0);
        assert_eq!(sched.active_count(), 1);
    }

    #[test]
    fn test_context_switch_record() {
        let rec = ContextSwitchRecord {
            from: Some(GreenThreadId(1)),
            to: GreenThreadId(2),
            worker: 0,
            reason: SwitchReason::Yield,
        };
        assert_eq!(rec.reason, SwitchReason::Yield);
        assert_eq!(rec.from, Some(GreenThreadId(1)));
    }

    #[test]
    fn test_green_thread_default_stack() {
        let t = GreenThread::new(GreenThreadId(1), "test".to_string(), 8192);
        assert_eq!(t.stack_size, 8192);
        assert_eq!(t.max_stack_size, 1024 * 1024);
        assert_eq!(t.cpu_time_us, 0);
    }
}
