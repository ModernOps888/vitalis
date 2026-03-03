//! Vitalis Async/Await Runtime — cooperative task scheduler.
//!
//! Provides a lightweight async runtime for Vitalis programs:
//! - `async fn` → creates a Future that can be awaited
//! - `await expr` → suspends current task until the future completes
//! - `spawn(expr)` → schedules a future for concurrent execution
//! - Built-in executor with round-robin task scheduling
//!
//! The runtime uses a simple cooperative model: each async function
//! is compiled to a state machine, and the executor polls tasks
//! until all are complete.

use std::collections::VecDeque;
use std::fmt;

// ─── Task State ─────────────────────────────────────────────────────────

/// Unique identifier for a spawned task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Task({})", self.0)
    }
}

/// The result of polling a task.
#[derive(Debug, Clone, PartialEq)]
pub enum PollResult {
    /// Task completed with an i64 value.
    Ready(i64),
    /// Task is not yet complete — needs to be polled again.
    Pending,
    /// Task encountered an error.
    Error(String),
}

/// A task's execution state.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    /// Task is ready to be polled.
    Runnable,
    /// Task is waiting on another task.
    Waiting(TaskId),
    /// Task completed.
    Completed(i64),
    /// Task failed.
    Failed(String),
}

/// A spawned asynchronous task.
#[derive(Debug)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub state: TaskState,
    /// The function name to call when polling.
    pub func_name: String,
    /// Current state-machine state index (for multi-await functions).
    pub resume_point: u32,
    /// Local variables saved across await points.
    pub saved_locals: Vec<i64>,
    /// The final result once completed.
    pub result: Option<i64>,
}

impl Task {
    pub fn new(id: TaskId, name: String, func_name: String) -> Self {
        Self {
            id,
            name,
            state: TaskState::Runnable,
            func_name,
            resume_point: 0,
            saved_locals: Vec::new(),
            result: None,
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self.state, TaskState::Completed(_) | TaskState::Failed(_))
    }
}

// ─── Executor ───────────────────────────────────────────────────────────

/// The async executor — drives tasks to completion.
pub struct Executor {
    tasks: VecDeque<Task>,
    next_id: u64,
    completed: Vec<(TaskId, i64)>,
    max_iterations: u64,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            next_id: 1,
            completed: Vec::new(),
            max_iterations: 100_000,
        }
    }

    /// Spawn a new task onto the executor.
    pub fn spawn(&mut self, name: &str, func_name: &str) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;
        let task = Task::new(id, name.to_string(), func_name.to_string());
        self.tasks.push_back(task);
        id
    }

    /// Spawn a task that immediately completes with a value.
    pub fn spawn_ready(&mut self, name: &str, value: i64) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;
        let mut task = Task::new(id, name.to_string(), String::new());
        task.state = TaskState::Completed(value);
        task.result = Some(value);
        self.completed.push((id, value));
        self.tasks.push_back(task);
        id
    }

    /// Get the result of a completed task.
    pub fn get_result(&self, id: TaskId) -> Option<i64> {
        self.completed.iter()
            .find(|(tid, _)| *tid == id)
            .map(|(_, val)| *val)
    }

    /// Check if a task has completed.
    pub fn is_complete(&self, id: TaskId) -> bool {
        self.completed.iter().any(|(tid, _)| *tid == id)
    }

    /// Mark a task as completed with a value (called by codegen'd poll functions).
    pub fn complete_task(&mut self, id: TaskId, value: i64) {
        for task in self.tasks.iter_mut() {
            if task.id == id {
                task.state = TaskState::Completed(value);
                task.result = Some(value);
                break;
            }
        }
        self.completed.push((id, value));
        // Wake any tasks waiting on this one
        for task in self.tasks.iter_mut() {
            if task.state == TaskState::Waiting(id) {
                task.state = TaskState::Runnable;
            }
        }
    }

    /// Run all tasks to completion using round-robin scheduling.
    pub fn run_all(&mut self) -> Vec<(TaskId, i64)> {
        let mut iterations: u64 = 0;

        while self.has_runnable() && iterations < self.max_iterations {
            if let Some(mut task) = self.tasks.pop_front() {
                match &task.state {
                    TaskState::Runnable => {
                        // In a real implementation, this would call the JIT'd poll function.
                        // For the runtime framework, we mark single-step tasks as complete.
                        if task.resume_point == 0 && task.func_name.is_empty() {
                            // Already completed (spawn_ready)
                        } else {
                            // The codegen layer will set up actual polling.
                            // Default behavior: complete with 0 after first poll.
                            task.state = TaskState::Completed(0);
                            task.result = Some(0);
                            self.completed.push((task.id, 0));
                        }
                    }
                    TaskState::Waiting(_) => {
                        // Still waiting, re-queue
                    }
                    TaskState::Completed(_) | TaskState::Failed(_) => {
                        // Done, don't re-queue
                        self.tasks.push_back(task);
                        iterations += 1;
                        continue;
                    }
                }
                self.tasks.push_back(task);
            }
            iterations += 1;
        }

        self.completed.clone()
    }

    /// Check if there are any runnable (non-completed) tasks.
    fn has_runnable(&self) -> bool {
        self.tasks.iter().any(|t| matches!(t.state, TaskState::Runnable))
    }

    /// Total number of tasks (including completed).
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Number of completed tasks.
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Future Type ────────────────────────────────────────────────────────

/// A Future handle that can be awaited.
#[derive(Debug, Clone)]
pub struct VitalisFuture {
    pub task_id: TaskId,
    pub name: String,
}

impl VitalisFuture {
    pub fn new(task_id: TaskId, name: String) -> Self {
        Self { task_id, name }
    }
}

// ─── Channel (for async communication) ──────────────────────────────────

/// A simple bounded channel for inter-task communication.
#[derive(Debug)]
pub struct Channel {
    buffer: VecDeque<i64>,
    capacity: usize,
    closed: bool,
}

impl Channel {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            closed: false,
        }
    }

    /// Send a value into the channel. Returns false if full or closed.
    pub fn send(&mut self, value: i64) -> bool {
        if self.closed || self.buffer.len() >= self.capacity {
            return false;
        }
        self.buffer.push_back(value);
        true
    }

    /// Try to receive a value. Returns None if empty.
    pub fn recv(&mut self) -> Option<i64> {
        self.buffer.pop_front()
    }

    /// Close the channel.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Check if the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Check if the channel is closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_task() {
        let mut exec = Executor::new();
        let id = exec.spawn("test_task", "test_func");
        assert_eq!(id.0, 1);
        assert_eq!(exec.task_count(), 1);
    }

    #[test]
    fn test_spawn_ready() {
        let mut exec = Executor::new();
        let id = exec.spawn_ready("ready_task", 42);
        assert!(exec.is_complete(id));
        assert_eq!(exec.get_result(id), Some(42));
    }

    #[test]
    fn test_run_all_completes() {
        let mut exec = Executor::new();
        exec.spawn_ready("t1", 10);
        exec.spawn_ready("t2", 20);
        let results = exec.run_all();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_multiple_spawns() {
        let mut exec = Executor::new();
        let id1 = exec.spawn("a", "func_a");
        let id2 = exec.spawn("b", "func_b");
        assert_ne!(id1, id2);
        assert_eq!(exec.task_count(), 2);
    }

    #[test]
    fn test_executor_default() {
        let exec = Executor::default();
        assert_eq!(exec.task_count(), 0);
    }

    #[test]
    fn test_complete_task() {
        let mut exec = Executor::new();
        let id = exec.spawn("task", "func");
        exec.complete_task(id, 99);
        assert!(exec.is_complete(id));
        assert_eq!(exec.get_result(id), Some(99));
    }

    #[test]
    fn test_waiting_task_wakes() {
        let mut exec = Executor::new();
        let id1 = exec.spawn("blocker", "func1");
        let id2 = exec.spawn("waiter", "func2");
        // Make task 2 wait on task 1
        for task in exec.tasks.iter_mut() {
            if task.id == id2 {
                task.state = TaskState::Waiting(id1);
            }
        }
        // Complete task 1 — should wake task 2
        exec.complete_task(id1, 42);
        let waiter = exec.tasks.iter().find(|t| t.id == id2).unwrap();
        assert_eq!(waiter.state, TaskState::Runnable);
    }

    #[test]
    fn test_channel_send_recv() {
        let mut ch = Channel::new(10);
        assert!(ch.send(1));
        assert!(ch.send(2));
        assert_eq!(ch.recv(), Some(1));
        assert_eq!(ch.recv(), Some(2));
        assert_eq!(ch.recv(), None);
    }

    #[test]
    fn test_channel_capacity() {
        let mut ch = Channel::new(2);
        assert!(ch.send(1));
        assert!(ch.send(2));
        assert!(!ch.send(3)); // Full
    }

    #[test]
    fn test_channel_close() {
        let mut ch = Channel::new(10);
        ch.close();
        assert!(!ch.send(1)); // Closed
        assert!(ch.is_closed());
    }

    #[test]
    fn test_task_display() {
        let id = TaskId(42);
        assert_eq!(format!("{}", id), "Task(42)");
    }

    #[test]
    fn test_future_creation() {
        let f = VitalisFuture::new(TaskId(1), "my_future".to_string());
        assert_eq!(f.task_id, TaskId(1));
        assert_eq!(f.name, "my_future");
    }

    #[test]
    fn test_run_spawned_tasks() {
        let mut exec = Executor::new();
        exec.spawn("worker", "do_work");
        let results = exec.run_all();
        // Default poll completes with 0
        assert!(!results.is_empty());
    }

    #[test]
    fn test_poll_result_variants() {
        let ready = PollResult::Ready(42);
        let pending = PollResult::Pending;
        let err = PollResult::Error("oops".into());
        assert_eq!(ready, PollResult::Ready(42));
        assert_eq!(pending, PollResult::Pending);
        assert_eq!(err, PollResult::Error("oops".into()));
    }
}
