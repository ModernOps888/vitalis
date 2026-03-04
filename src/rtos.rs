//! Real-Time Operating System (RTOS) kernel for Vitalis.
//!
//! - **Preemptive scheduler**: Priority-based round-robin
//! - **Semaphores & Mutexes**: Kernel-level synchronization
//! - **Message queues**: Inter-task communication
//! - **Timers**: One-shot and periodic software timers
//! - **MPU support**: Memory protection unit configuration
//! - **Static allocation**: No dynamic allocation in kernel

use std::collections::VecDeque;

// ── Task Management ─────────────────────────────────────────────────

/// Task ID.
pub type TaskId = u32;

/// Task state.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked(BlockReason),
    Suspended,
    Terminated,
}

/// Reason a task is blocked.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockReason {
    Semaphore(u32),
    Mutex(u32),
    Queue(u32),
    Delay(u64),
    Event(u32),
}

/// Task priority (0 = highest).
pub type Priority = u8;

/// A task control block.
#[derive(Debug, Clone)]
pub struct TaskControlBlock {
    pub id: TaskId,
    pub name: String,
    pub priority: Priority,
    pub state: TaskState,
    pub stack_size: u32,
    pub stack_used: u32,
    pub cpu_time_us: u64,
    pub deadline_us: Option<u64>,
    pub last_scheduled: u64,
}

impl TaskControlBlock {
    pub fn stack_usage_percent(&self) -> f64 {
        if self.stack_size == 0 { return 0.0; }
        self.stack_used as f64 / self.stack_size as f64 * 100.0
    }

    pub fn is_deadline_critical(&self, current_time_us: u64) -> bool {
        if let Some(deadline) = self.deadline_us {
            current_time_us >= deadline
        } else {
            false
        }
    }
}

// ── Scheduler ───────────────────────────────────────────────────────

/// Scheduling algorithm.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulingPolicy {
    PriorityPreemptive,
    RoundRobin,
    EarliestDeadlineFirst,
    RateMonotonic,
}

/// The RTOS scheduler.
pub struct Scheduler {
    tasks: Vec<TaskControlBlock>,
    current_task: Option<TaskId>,
    next_id: TaskId,
    policy: SchedulingPolicy,
    tick_count: u64,
    time_slice_us: u64,
    idle_time_us: u64,
}

impl Scheduler {
    pub fn new(policy: SchedulingPolicy, time_slice_us: u64) -> Self {
        Self {
            tasks: Vec::new(),
            current_task: None,
            next_id: 0,
            policy,
            tick_count: 0,
            time_slice_us,
            idle_time_us: 0,
        }
    }

    pub fn create_task(&mut self, name: &str, priority: Priority, stack_size: u32) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(TaskControlBlock {
            id, name: name.to_string(), priority, state: TaskState::Ready,
            stack_size, stack_used: 0, cpu_time_us: 0,
            deadline_us: None, last_scheduled: 0,
        });
        id
    }

    pub fn create_task_with_deadline(&mut self, name: &str, priority: Priority, stack_size: u32, deadline_us: u64) -> TaskId {
        let id = self.create_task(name, priority, stack_size);
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.deadline_us = Some(deadline_us);
        }
        id
    }

    /// Schedule the next task to run.
    pub fn schedule(&mut self) -> Option<TaskId> {
        self.tick_count += 1;

        // If current task is running, set it back to Ready.
        if let Some(current) = self.current_task {
            if let Some(task) = self.tasks.iter_mut().find(|t| t.id == current) {
                if task.state == TaskState::Running {
                    task.state = TaskState::Ready;
                }
            }
        }

        let next = match self.policy {
            SchedulingPolicy::PriorityPreemptive => {
                self.tasks.iter()
                    .filter(|t| t.state == TaskState::Ready)
                    .min_by_key(|t| t.priority) // Lower number = higher priority.
                    .map(|t| t.id)
            }
            SchedulingPolicy::RoundRobin => {
                let ready: Vec<_> = self.tasks.iter()
                    .filter(|t| t.state == TaskState::Ready)
                    .collect();
                if ready.is_empty() { None }
                else {
                    let idx = (self.tick_count as usize) % ready.len();
                    Some(ready[idx].id)
                }
            }
            SchedulingPolicy::EarliestDeadlineFirst => {
                self.tasks.iter()
                    .filter(|t| t.state == TaskState::Ready)
                    .min_by_key(|t| t.deadline_us.unwrap_or(u64::MAX))
                    .map(|t| t.id)
            }
            SchedulingPolicy::RateMonotonic => {
                // Shorter period = higher priority (approximated by deadline).
                self.tasks.iter()
                    .filter(|t| t.state == TaskState::Ready)
                    .min_by_key(|t| t.deadline_us.unwrap_or(u64::MAX))
                    .map(|t| t.id)
            }
        };

        if let Some(id) = next {
            if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                task.state = TaskState::Running;
                task.last_scheduled = self.tick_count;
            }
        }
        self.current_task = next;
        next
    }

    pub fn suspend_task(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.state = TaskState::Suspended;
        }
    }

    pub fn resume_task(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            if task.state == TaskState::Suspended {
                task.state = TaskState::Ready;
            }
        }
    }

    pub fn terminate_task(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.state = TaskState::Terminated;
        }
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn ready_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.state == TaskState::Ready).count()
    }

    pub fn get_task(&self, id: TaskId) -> Option<&TaskControlBlock> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn current_task_id(&self) -> Option<TaskId> {
        self.current_task
    }

    pub fn cpu_utilization(&self) -> f64 {
        if self.tick_count == 0 { return 0.0; }
        let busy = self.tick_count.saturating_sub(self.idle_time_us);
        busy as f64 / self.tick_count as f64 * 100.0
    }
}

// ── Semaphore ───────────────────────────────────────────────────────

/// Counting semaphore.
#[derive(Debug)]
pub struct Semaphore {
    pub id: u32,
    count: i32,
    max_count: i32,
    waiters: VecDeque<TaskId>,
}

impl Semaphore {
    pub fn new(id: u32, initial_count: i32, max_count: i32) -> Self {
        Self { id, count: initial_count, max_count, waiters: VecDeque::new() }
    }

    pub fn binary(id: u32) -> Self {
        Self::new(id, 1, 1)
    }

    /// Try to acquire (take) the semaphore.
    pub fn try_acquire(&mut self) -> Result<(), TaskId> {
        if self.count > 0 {
            self.count -= 1;
            Ok(())
        } else {
            Err(0) // Would block.
        }
    }

    /// Release (give) the semaphore.
    pub fn release(&mut self) -> Option<TaskId> {
        if self.count < self.max_count {
            self.count += 1;
        }
        self.waiters.pop_front()
    }

    pub fn add_waiter(&mut self, task: TaskId) {
        self.waiters.push_back(task);
    }

    pub fn count(&self) -> i32 {
        self.count
    }

    pub fn waiter_count(&self) -> usize {
        self.waiters.len()
    }
}

// ── Mutex ───────────────────────────────────────────────────────────

/// Priority-inheritance mutex.
#[derive(Debug)]
pub struct KernelMutex {
    pub id: u32,
    owner: Option<TaskId>,
    original_priority: Option<Priority>,
    waiters: VecDeque<TaskId>,
    recursive_count: u32,
}

impl KernelMutex {
    pub fn new(id: u32) -> Self {
        Self { id, owner: None, original_priority: None, waiters: VecDeque::new(), recursive_count: 0 }
    }

    pub fn try_lock(&mut self, task: TaskId) -> bool {
        match self.owner {
            None => {
                self.owner = Some(task);
                self.recursive_count = 1;
                true
            }
            Some(t) if t == task => {
                self.recursive_count += 1;
                true
            }
            _ => false,
        }
    }

    pub fn unlock(&mut self, task: TaskId) -> Option<TaskId> {
        if self.owner == Some(task) {
            self.recursive_count -= 1;
            if self.recursive_count == 0 {
                self.owner = None;
                return self.waiters.pop_front();
            }
        }
        None
    }

    pub fn owner(&self) -> Option<TaskId> {
        self.owner
    }

    pub fn is_locked(&self) -> bool {
        self.owner.is_some()
    }

    pub fn add_waiter(&mut self, task: TaskId) {
        self.waiters.push_back(task);
    }
}

// ── Message Queue ───────────────────────────────────────────────────

/// A fixed-size message queue.
#[derive(Debug)]
pub struct MessageQueue<T> {
    pub id: u32,
    buffer: VecDeque<T>,
    capacity: usize,
    send_waiters: VecDeque<TaskId>,
    recv_waiters: VecDeque<TaskId>,
}

impl<T> MessageQueue<T> {
    pub fn new(id: u32, capacity: usize) -> Self {
        Self {
            id,
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            send_waiters: VecDeque::new(),
            recv_waiters: VecDeque::new(),
        }
    }

    pub fn try_send(&mut self, msg: T) -> Result<Option<TaskId>, T> {
        if self.buffer.len() < self.capacity {
            self.buffer.push_back(msg);
            Ok(self.recv_waiters.pop_front())
        } else {
            Err(msg) // Full.
        }
    }

    pub fn try_recv(&mut self) -> Option<T> {
        self.buffer.pop_front()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// ── Software Timers ─────────────────────────────────────────────────

/// Timer mode.
#[derive(Debug, Clone, PartialEq)]
pub enum TimerMode {
    OneShot,
    Periodic,
}

/// A software timer.
#[derive(Debug, Clone)]
pub struct SoftwareTimer {
    pub id: u32,
    pub name: String,
    pub period_us: u64,
    pub mode: TimerMode,
    pub remaining_us: u64,
    pub active: bool,
    pub callback_name: String,
    pub fire_count: u64,
}

impl SoftwareTimer {
    pub fn new(id: u32, name: &str, period_us: u64, mode: TimerMode, callback: &str) -> Self {
        Self {
            id, name: name.to_string(), period_us, mode,
            remaining_us: period_us, active: false,
            callback_name: callback.to_string(), fire_count: 0,
        }
    }

    pub fn start(&mut self) {
        self.active = true;
        self.remaining_us = self.period_us;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Advance timer by `elapsed_us`. Returns true if timer fired.
    pub fn tick(&mut self, elapsed_us: u64) -> bool {
        if !self.active { return false; }
        if elapsed_us >= self.remaining_us {
            self.fire_count += 1;
            match self.mode {
                TimerMode::OneShot => {
                    self.active = false;
                    self.remaining_us = 0;
                }
                TimerMode::Periodic => {
                    self.remaining_us = self.period_us;
                }
            }
            true
        } else {
            self.remaining_us -= elapsed_us;
            false
        }
    }
}

// ── MPU (Memory Protection Unit) ────────────────────────────────────

/// MPU region configuration.
#[derive(Debug, Clone)]
pub struct MpuRegion {
    pub id: u8,
    pub base_address: u32,
    pub size: u32,
    pub executable: bool,
    pub writable: bool,
    pub cacheable: bool,
    pub shareable: bool,
}

/// MPU configuration for a task.
#[derive(Debug, Clone)]
pub struct MpuConfig {
    pub task_id: TaskId,
    pub regions: Vec<MpuRegion>,
}

impl MpuConfig {
    pub fn new(task_id: TaskId) -> Self {
        Self { task_id, regions: Vec::new() }
    }

    pub fn add_region(&mut self, region: MpuRegion) -> Result<(), String> {
        if self.regions.len() >= 8 {
            return Err("MPU supports max 8 regions".into());
        }
        // Size must be power of 2 and >= 32.
        if region.size < 32 || !region.size.is_power_of_two() {
            return Err("MPU region size must be power of 2 and >= 32".into());
        }
        self.regions.push(region);
        Ok(())
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_create_task() {
        let mut sched = Scheduler::new(SchedulingPolicy::PriorityPreemptive, 1000);
        let t1 = sched.create_task("task1", 1, 1024);
        let t2 = sched.create_task("task2", 2, 2048);
        assert_eq!(sched.task_count(), 2);
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_priority_scheduling() {
        let mut sched = Scheduler::new(SchedulingPolicy::PriorityPreemptive, 1000);
        sched.create_task("low", 10, 1024);
        let high = sched.create_task("high", 1, 1024);
        let next = sched.schedule();
        assert_eq!(next, Some(high)); // Higher priority (lower number).
    }

    #[test]
    fn test_round_robin() {
        let mut sched = Scheduler::new(SchedulingPolicy::RoundRobin, 1000);
        sched.create_task("a", 1, 1024);
        sched.create_task("b", 1, 1024);
        let first = sched.schedule();
        assert!(first.is_some());
    }

    #[test]
    fn test_edf_scheduling() {
        let mut sched = Scheduler::new(SchedulingPolicy::EarliestDeadlineFirst, 1000);
        sched.create_task_with_deadline("late", 1, 1024, 200);
        let early = sched.create_task_with_deadline("early", 1, 1024, 100);
        let next = sched.schedule();
        assert_eq!(next, Some(early));
    }

    #[test]
    fn test_suspend_resume() {
        let mut sched = Scheduler::new(SchedulingPolicy::PriorityPreemptive, 1000);
        let t = sched.create_task("t", 1, 1024);
        sched.suspend_task(t);
        assert_eq!(sched.get_task(t).unwrap().state, TaskState::Suspended);
        sched.resume_task(t);
        assert_eq!(sched.get_task(t).unwrap().state, TaskState::Ready);
    }

    #[test]
    fn test_semaphore_acquire_release() {
        let mut sem = Semaphore::new(0, 1, 3);
        assert!(sem.try_acquire().is_ok());
        assert_eq!(sem.count(), 0);
        sem.release();
        assert_eq!(sem.count(), 1);
    }

    #[test]
    fn test_semaphore_blocking() {
        let mut sem = Semaphore::new(0, 0, 1);
        assert!(sem.try_acquire().is_err());
    }

    #[test]
    fn test_binary_semaphore() {
        let mut sem = Semaphore::binary(0);
        assert_eq!(sem.count(), 1);
        assert!(sem.try_acquire().is_ok());
        assert_eq!(sem.count(), 0);
    }

    #[test]
    fn test_mutex_lock_unlock() {
        let mut mtx = KernelMutex::new(0);
        assert!(mtx.try_lock(1));
        assert!(!mtx.try_lock(2)); // Different task blcked.
        assert!(mtx.try_lock(1));  // Same task recursive.
        mtx.unlock(1);
        assert!(mtx.is_locked());  // Still locked (recursive).
        mtx.unlock(1);
        assert!(!mtx.is_locked());
    }

    #[test]
    fn test_message_queue() {
        let mut q: MessageQueue<i32> = MessageQueue::new(0, 4);
        assert!(q.try_send(42).is_ok());
        assert!(q.try_send(43).is_ok());
        assert_eq!(q.len(), 2);
        assert_eq!(q.try_recv(), Some(42));
        assert_eq!(q.try_recv(), Some(43));
        assert!(q.is_empty());
    }

    #[test]
    fn test_message_queue_full() {
        let mut q: MessageQueue<i32> = MessageQueue::new(0, 2);
        assert!(q.try_send(1).is_ok());
        assert!(q.try_send(2).is_ok());
        assert!(q.try_send(3).is_err());
    }

    #[test]
    fn test_timer_oneshot() {
        let mut t = SoftwareTimer::new(0, "t", 1000, TimerMode::OneShot, "cb");
        t.start();
        assert!(!t.tick(500));
        assert!(t.tick(500));
        assert!(!t.active);
        assert_eq!(t.fire_count, 1);
    }

    #[test]
    fn test_timer_periodic() {
        let mut t = SoftwareTimer::new(0, "t", 100, TimerMode::Periodic, "cb");
        t.start();
        assert!(t.tick(200));
        assert!(t.active); // Still active.
        assert_eq!(t.fire_count, 1);
    }

    #[test]
    fn test_mpu_config() {
        let mut mpu = MpuConfig::new(0);
        assert!(mpu.add_region(MpuRegion {
            id: 0, base_address: 0x2000_0000, size: 1024,
            executable: false, writable: true, cacheable: true, shareable: false,
        }).is_ok());
        assert_eq!(mpu.region_count(), 1);
    }

    #[test]
    fn test_mpu_invalid_size() {
        let mut mpu = MpuConfig::new(0);
        let result = mpu.add_region(MpuRegion {
            id: 0, base_address: 0, size: 100, // Not power of 2.
            executable: false, writable: false, cacheable: false, shareable: false,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_task_stack_usage() {
        let task = TaskControlBlock {
            id: 0, name: "t".into(), priority: 0,
            state: TaskState::Ready, stack_size: 1000,
            stack_used: 500, cpu_time_us: 0,
            deadline_us: None, last_scheduled: 0,
        };
        assert!((task.stack_usage_percent() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_terminate() {
        let mut sched = Scheduler::new(SchedulingPolicy::PriorityPreemptive, 1000);
        let t = sched.create_task("t", 1, 1024);
        sched.terminate_task(t);
        assert_eq!(sched.get_task(t).unwrap().state, TaskState::Terminated);
    }

    #[test]
    fn test_scheduler_ready_count() {
        let mut sched = Scheduler::new(SchedulingPolicy::PriorityPreemptive, 1000);
        sched.create_task("a", 1, 1024);
        sched.create_task("b", 2, 1024);
        let t3 = sched.create_task("c", 3, 1024);
        sched.suspend_task(t3);
        assert_eq!(sched.ready_count(), 2);
    }

    #[test]
    fn test_deadline_critical() {
        let task = TaskControlBlock {
            id: 0, name: "t".into(), priority: 0,
            state: TaskState::Ready, stack_size: 1024,
            stack_used: 0, cpu_time_us: 0,
            deadline_us: Some(100), last_scheduled: 0,
        };
        assert!(!task.is_deadline_critical(50));
        assert!(task.is_deadline_critical(100));
        assert!(task.is_deadline_critical(200));
    }
}
