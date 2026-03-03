//! Structured concurrency primitives for Vitalis.
//!
//! Provides Mutex, RwLock, channels (bounded/unbounded MPSC), Select multiplexer,
//! WaitGroup synchronization, atomic operations, and scoped tasks for structured
//! concurrency patterns.
//!
//! Modeled after Go channels, Rust std::sync, and Swift structured concurrency.

use std::collections::VecDeque;
use std::fmt;

// ── Error Types ──────────────────────────────────────────────────────

/// Errors that can occur during concurrency operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ConcurrencyError {
    /// Mutex is already locked (try_lock failed).
    MutexLocked,
    /// Mutex was poisoned by a panic in a critical section.
    MutexPoisoned,
    /// RwLock has active writers (try_read failed).
    RwLockWriteLocked,
    /// RwLock has active readers (try_write failed).
    RwLockReadLocked,
    /// Channel is closed, no more messages can be sent.
    ChannelClosed,
    /// Channel buffer is full (bounded channel).
    ChannelFull,
    /// Channel is empty, no messages available.
    ChannelEmpty,
    /// Timeout expired waiting for operation.
    Timeout,
    /// Select had no ready channels.
    SelectEmpty,
    /// WaitGroup counter went negative.
    WaitGroupNegative,
    /// Deadlock detected.
    DeadlockDetected,
    /// Task was cancelled.
    TaskCancelled,
    /// Invalid operation on a concurrency primitive.
    InvalidOperation(String),
}

impl fmt::Display for ConcurrencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MutexLocked => write!(f, "mutex is already locked"),
            Self::MutexPoisoned => write!(f, "mutex was poisoned"),
            Self::RwLockWriteLocked => write!(f, "rwlock has active writer"),
            Self::RwLockReadLocked => write!(f, "rwlock has active readers"),
            Self::ChannelClosed => write!(f, "channel is closed"),
            Self::ChannelFull => write!(f, "channel buffer is full"),
            Self::ChannelEmpty => write!(f, "channel is empty"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::SelectEmpty => write!(f, "select has no ready channels"),
            Self::WaitGroupNegative => write!(f, "wait group counter went negative"),
            Self::DeadlockDetected => write!(f, "deadlock detected"),
            Self::TaskCancelled => write!(f, "task was cancelled"),
            Self::InvalidOperation(msg) => write!(f, "invalid operation: {msg}"),
        }
    }
}

// ── Concurrency Values ───────────────────────────────────────────────

/// Values that can be sent through channels and stored in sync primitives.
#[derive(Debug, Clone, PartialEq)]
pub enum ConcValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<ConcValue>),
    Void,
}

impl fmt::Display for ConcValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(n) => write!(f, "{n}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Str(s) => write!(f, "{s}"),
            Self::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Self::Void => write!(f, "()"),
        }
    }
}

// ── Mutex ────────────────────────────────────────────────────────────

/// State of a Mutex lock.
#[derive(Debug, Clone, PartialEq)]
pub enum MutexState {
    Unlocked,
    Locked { holder: String },
    Poisoned,
}

/// A mutual exclusion lock protecting a value.
#[derive(Debug, Clone)]
pub struct Mutex {
    pub name: String,
    pub value: ConcValue,
    pub state: MutexState,
    pub wait_queue: Vec<String>,
}

impl Mutex {
    pub fn new(name: &str, value: ConcValue) -> Self {
        Self {
            name: name.to_string(),
            value,
            state: MutexState::Unlocked,
            wait_queue: Vec::new(),
        }
    }

    /// Acquire the lock. Returns error if already locked or poisoned.
    pub fn lock(&mut self, holder: &str) -> Result<&ConcValue, ConcurrencyError> {
        match &self.state {
            MutexState::Unlocked => {
                self.state = MutexState::Locked { holder: holder.to_string() };
                Ok(&self.value)
            }
            MutexState::Locked { .. } => {
                self.wait_queue.push(holder.to_string());
                Err(ConcurrencyError::MutexLocked)
            }
            MutexState::Poisoned => Err(ConcurrencyError::MutexPoisoned),
        }
    }

    /// Try to acquire the lock without blocking.
    pub fn try_lock(&mut self, holder: &str) -> Result<&ConcValue, ConcurrencyError> {
        match &self.state {
            MutexState::Unlocked => {
                self.state = MutexState::Locked { holder: holder.to_string() };
                Ok(&self.value)
            }
            MutexState::Locked { .. } => Err(ConcurrencyError::MutexLocked),
            MutexState::Poisoned => Err(ConcurrencyError::MutexPoisoned),
        }
    }

    /// Release the lock. Promotes next waiter if any.
    pub fn unlock(&mut self, holder: &str) -> Result<(), ConcurrencyError> {
        match &self.state {
            MutexState::Locked { holder: h } if h == holder => {
                if let Some(next) = self.wait_queue.first().cloned() {
                    self.wait_queue.remove(0);
                    self.state = MutexState::Locked { holder: next };
                } else {
                    self.state = MutexState::Unlocked;
                }
                Ok(())
            }
            MutexState::Locked { .. } => {
                Err(ConcurrencyError::InvalidOperation("not the lock holder".into()))
            }
            MutexState::Unlocked => {
                Err(ConcurrencyError::InvalidOperation("mutex is not locked".into()))
            }
            MutexState::Poisoned => Err(ConcurrencyError::MutexPoisoned),
        }
    }

    /// Update the protected value while holding the lock.
    pub fn set_value(&mut self, holder: &str, value: ConcValue) -> Result<(), ConcurrencyError> {
        match &self.state {
            MutexState::Locked { holder: h } if h == holder => {
                self.value = value;
                Ok(())
            }
            MutexState::Locked { .. } => {
                Err(ConcurrencyError::InvalidOperation("not the lock holder".into()))
            }
            _ => Err(ConcurrencyError::MutexLocked),
        }
    }

    /// Poison the mutex (simulates panic in critical section).
    pub fn poison(&mut self) {
        self.state = MutexState::Poisoned;
    }

    pub fn is_locked(&self) -> bool {
        matches!(self.state, MutexState::Locked { .. })
    }

    pub fn is_poisoned(&self) -> bool {
        matches!(self.state, MutexState::Poisoned)
    }

    pub fn waiter_count(&self) -> usize {
        self.wait_queue.len()
    }
}

// ── RwLock ───────────────────────────────────────────────────────────

/// State of a RwLock.
#[derive(Debug, Clone, PartialEq)]
pub enum RwLockState {
    Unlocked,
    ReadLocked { readers: Vec<String> },
    WriteLocked { writer: String },
}

/// A reader-writer lock allowing multiple concurrent readers or one exclusive writer.
#[derive(Debug, Clone)]
pub struct RwLock {
    pub name: String,
    pub value: ConcValue,
    pub state: RwLockState,
}

impl RwLock {
    pub fn new(name: &str, value: ConcValue) -> Self {
        Self {
            name: name.to_string(),
            value,
            state: RwLockState::Unlocked,
        }
    }

    /// Acquire a read lock. Multiple readers allowed.
    pub fn read_lock(&mut self, reader: &str) -> Result<&ConcValue, ConcurrencyError> {
        match &mut self.state {
            RwLockState::Unlocked => {
                self.state = RwLockState::ReadLocked {
                    readers: vec![reader.to_string()],
                };
                Ok(&self.value)
            }
            RwLockState::ReadLocked { readers } => {
                readers.push(reader.to_string());
                Ok(&self.value)
            }
            RwLockState::WriteLocked { .. } => Err(ConcurrencyError::RwLockWriteLocked),
        }
    }

    /// Acquire a write lock. Exclusive access required.
    pub fn write_lock(&mut self, writer: &str) -> Result<&mut ConcValue, ConcurrencyError> {
        match &self.state {
            RwLockState::Unlocked => {
                self.state = RwLockState::WriteLocked { writer: writer.to_string() };
                Ok(&mut self.value)
            }
            RwLockState::ReadLocked { .. } => Err(ConcurrencyError::RwLockReadLocked),
            RwLockState::WriteLocked { .. } => Err(ConcurrencyError::RwLockWriteLocked),
        }
    }

    /// Release a read lock.
    pub fn read_unlock(&mut self, reader: &str) -> Result<(), ConcurrencyError> {
        match &mut self.state {
            RwLockState::ReadLocked { readers } => {
                readers.retain(|r| r != reader);
                if readers.is_empty() {
                    self.state = RwLockState::Unlocked;
                }
                Ok(())
            }
            _ => Err(ConcurrencyError::InvalidOperation("not read-locked".into())),
        }
    }

    /// Release a write lock.
    pub fn write_unlock(&mut self, writer: &str) -> Result<(), ConcurrencyError> {
        match &self.state {
            RwLockState::WriteLocked { writer: w } if w == writer => {
                self.state = RwLockState::Unlocked;
                Ok(())
            }
            _ => Err(ConcurrencyError::InvalidOperation("not write-locked by this writer".into())),
        }
    }

    pub fn reader_count(&self) -> usize {
        match &self.state {
            RwLockState::ReadLocked { readers } => readers.len(),
            _ => 0,
        }
    }

    pub fn is_write_locked(&self) -> bool {
        matches!(self.state, RwLockState::WriteLocked { .. })
    }

    pub fn is_read_locked(&self) -> bool {
        matches!(self.state, RwLockState::ReadLocked { .. })
    }
}

// ── Channel ──────────────────────────────────────────────────────────

/// Channel type: bounded with a capacity, or unbounded.
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelKind {
    Bounded(usize),
    Unbounded,
}

/// A message-passing channel (MPSC-style).
#[derive(Debug, Clone)]
pub struct Channel {
    pub name: String,
    pub kind: ChannelKind,
    pub buffer: VecDeque<ConcValue>,
    pub closed: bool,
    pub send_count: u64,
    pub recv_count: u64,
}

impl Channel {
    /// Create a bounded channel with the given capacity.
    pub fn bounded(name: &str, capacity: usize) -> Self {
        Self {
            name: name.to_string(),
            kind: ChannelKind::Bounded(capacity),
            buffer: VecDeque::new(),
            closed: false,
            send_count: 0,
            recv_count: 0,
        }
    }

    /// Create an unbounded channel.
    pub fn unbounded(name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: ChannelKind::Unbounded,
            buffer: VecDeque::new(),
            closed: false,
            send_count: 0,
            recv_count: 0,
        }
    }

    /// Send a value into the channel.
    pub fn send(&mut self, value: ConcValue) -> Result<(), ConcurrencyError> {
        if self.closed {
            return Err(ConcurrencyError::ChannelClosed);
        }
        if let ChannelKind::Bounded(cap) = &self.kind {
            if self.buffer.len() >= *cap {
                return Err(ConcurrencyError::ChannelFull);
            }
        }
        self.buffer.push_back(value);
        self.send_count += 1;
        Ok(())
    }

    /// Receive a value from the channel.
    pub fn recv(&mut self) -> Result<ConcValue, ConcurrencyError> {
        if let Some(value) = self.buffer.pop_front() {
            self.recv_count += 1;
            Ok(value)
        } else if self.closed {
            Err(ConcurrencyError::ChannelClosed)
        } else {
            Err(ConcurrencyError::ChannelEmpty)
        }
    }

    /// Try to receive without blocking.
    pub fn try_recv(&mut self) -> Result<ConcValue, ConcurrencyError> {
        self.recv()
    }

    /// Close the channel. No more sends allowed.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Check if the channel has pending messages.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Number of pending messages in the buffer.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Remaining capacity (for bounded channels).
    pub fn remaining_capacity(&self) -> Option<usize> {
        match &self.kind {
            ChannelKind::Bounded(cap) => Some(cap - self.buffer.len()),
            ChannelKind::Unbounded => None,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Drain all pending messages from the channel.
    pub fn drain(&mut self) -> Vec<ConcValue> {
        let items: Vec<ConcValue> = self.buffer.drain(..).collect();
        self.recv_count += items.len() as u64;
        items
    }
}

// ── Select Statement ─────────────────────────────────────────────────

/// A case in a select statement, referencing a channel by name.
#[derive(Debug, Clone)]
pub enum SelectCase {
    /// Receive from a channel.
    Recv { channel_name: String },
    /// Send a value to a channel.
    Send { channel_name: String, value: ConcValue },
    /// Default case if no channels are ready.
    Default,
}

/// Result of a select operation.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectResult {
    /// A message was received from the named channel.
    Received { channel_name: String, value: ConcValue },
    /// A message was sent to the named channel.
    Sent { channel_name: String },
    /// The default case was chosen.
    DefaultChosen,
    /// No channels were ready and no default was provided.
    NoneReady,
}

/// A select multiplexer that evaluates multiple channel operations.
#[derive(Debug, Clone)]
pub struct Select {
    pub cases: Vec<SelectCase>,
}

impl Select {
    pub fn new() -> Self {
        Self { cases: Vec::new() }
    }

    pub fn add_recv(&mut self, channel_name: &str) {
        self.cases.push(SelectCase::Recv {
            channel_name: channel_name.to_string(),
        });
    }

    pub fn add_send(&mut self, channel_name: &str, value: ConcValue) {
        self.cases.push(SelectCase::Send {
            channel_name: channel_name.to_string(),
            value,
        });
    }

    pub fn add_default(&mut self) {
        self.cases.push(SelectCase::Default);
    }

    /// Evaluate the select statement against a set of channels.
    /// Returns the first ready case.
    pub fn evaluate(&self, channels: &mut ChannelRegistry) -> SelectResult {
        // Try recv cases first
        for case in &self.cases {
            match case {
                SelectCase::Recv { channel_name } => {
                    if let Some(ch) = channels.get_mut(channel_name) {
                        if let Ok(value) = ch.recv() {
                            return SelectResult::Received {
                                channel_name: channel_name.clone(),
                                value,
                            };
                        }
                    }
                }
                SelectCase::Send { channel_name, value } => {
                    if let Some(ch) = channels.get_mut(channel_name) {
                        if ch.send(value.clone()).is_ok() {
                            return SelectResult::Sent {
                                channel_name: channel_name.clone(),
                            };
                        }
                    }
                }
                SelectCase::Default => {}
            }
        }
        // Check for default
        for case in &self.cases {
            if matches!(case, SelectCase::Default) {
                return SelectResult::DefaultChosen;
            }
        }
        SelectResult::NoneReady
    }

    pub fn case_count(&self) -> usize {
        self.cases.len()
    }

    pub fn has_default(&self) -> bool {
        self.cases.iter().any(|c| matches!(c, SelectCase::Default))
    }
}

impl Default for Select {
    fn default() -> Self {
        Self::new()
    }
}

// ── Channel Registry ─────────────────────────────────────────────────

/// Registry of named channels for use with Select and scoped tasks.
#[derive(Debug, Clone)]
pub struct ChannelRegistry {
    channels: Vec<(String, Channel)>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self { channels: Vec::new() }
    }

    pub fn register(&mut self, channel: Channel) {
        self.channels.push((channel.name.clone(), channel));
    }

    pub fn get(&self, name: &str) -> Option<&Channel> {
        self.channels.iter().find(|(n, _)| n == name).map(|(_, c)| c)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|(n, _)| n == name).map(|(_, c)| c)
    }

    pub fn remove(&mut self, name: &str) -> Option<Channel> {
        if let Some(pos) = self.channels.iter().position(|(n, _)| n == name) {
            Some(self.channels.remove(pos).1)
        } else {
            None
        }
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    pub fn channel_names(&self) -> Vec<&str> {
        self.channels.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Close all channels in the registry.
    pub fn close_all(&mut self) {
        for (_, ch) in &mut self.channels {
            ch.close();
        }
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── WaitGroup ────────────────────────────────────────────────────────

/// A synchronization primitive that waits for a group of tasks to complete.
#[derive(Debug, Clone)]
pub struct WaitGroup {
    pub name: String,
    pub counter: i64,
    pub completed: Vec<String>,
}

impl WaitGroup {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            counter: 0,
            completed: Vec::new(),
        }
    }

    /// Add delta to the counter. Typically called before launching a task.
    pub fn add(&mut self, delta: i64) -> Result<(), ConcurrencyError> {
        self.counter += delta;
        if self.counter < 0 {
            return Err(ConcurrencyError::WaitGroupNegative);
        }
        Ok(())
    }

    /// Mark one task as done. Decrements the counter.
    pub fn done(&mut self, task_name: &str) -> Result<(), ConcurrencyError> {
        self.counter -= 1;
        self.completed.push(task_name.to_string());
        if self.counter < 0 {
            return Err(ConcurrencyError::WaitGroupNegative);
        }
        Ok(())
    }

    /// Check if all tasks are done (counter == 0).
    pub fn is_done(&self) -> bool {
        self.counter == 0
    }

    /// Get the current counter value.
    pub fn pending(&self) -> i64 {
        self.counter
    }

    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }
}

// ── Atomic Operations ────────────────────────────────────────────────

/// Memory ordering for atomic operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ordering {
    Relaxed,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}

impl fmt::Display for Ordering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Relaxed => write!(f, "Relaxed"),
            Self::Acquire => write!(f, "Acquire"),
            Self::Release => write!(f, "Release"),
            Self::AcqRel => write!(f, "AcqRel"),
            Self::SeqCst => write!(f, "SeqCst"),
        }
    }
}

/// An atomic integer supporting load, store, swap, and compare-and-swap.
#[derive(Debug, Clone)]
pub struct AtomicInt {
    pub name: String,
    pub value: i64,
    pub operation_count: u64,
}

impl AtomicInt {
    pub fn new(name: &str, value: i64) -> Self {
        Self {
            name: name.to_string(),
            value,
            operation_count: 0,
        }
    }

    pub fn load(&mut self, _ordering: Ordering) -> i64 {
        self.operation_count += 1;
        self.value
    }

    pub fn store(&mut self, value: i64, _ordering: Ordering) {
        self.operation_count += 1;
        self.value = value;
    }

    pub fn swap(&mut self, new: i64, _ordering: Ordering) -> i64 {
        self.operation_count += 1;
        let old = self.value;
        self.value = new;
        old
    }

    /// Compare-and-swap: if current == expected, set to new and return Ok(old).
    pub fn compare_and_swap(&mut self, expected: i64, new: i64, _ordering: Ordering) -> Result<i64, i64> {
        self.operation_count += 1;
        if self.value == expected {
            let old = self.value;
            self.value = new;
            Ok(old)
        } else {
            Err(self.value)
        }
    }

    pub fn fetch_add(&mut self, val: i64, _ordering: Ordering) -> i64 {
        self.operation_count += 1;
        let old = self.value;
        self.value += val;
        old
    }

    pub fn fetch_sub(&mut self, val: i64, _ordering: Ordering) -> i64 {
        self.operation_count += 1;
        let old = self.value;
        self.value -= val;
        old
    }

    pub fn fetch_and(&mut self, val: i64, _ordering: Ordering) -> i64 {
        self.operation_count += 1;
        let old = self.value;
        self.value &= val;
        old
    }

    pub fn fetch_or(&mut self, val: i64, _ordering: Ordering) -> i64 {
        self.operation_count += 1;
        let old = self.value;
        self.value |= val;
        old
    }

    pub fn fetch_xor(&mut self, val: i64, _ordering: Ordering) -> i64 {
        self.operation_count += 1;
        let old = self.value;
        self.value ^= val;
        old
    }
}

/// An atomic boolean.
#[derive(Debug, Clone)]
pub struct AtomicBool {
    pub name: String,
    pub value: bool,
    pub operation_count: u64,
}

impl AtomicBool {
    pub fn new(name: &str, value: bool) -> Self {
        Self {
            name: name.to_string(),
            value,
            operation_count: 0,
        }
    }

    pub fn load(&mut self, _ordering: Ordering) -> bool {
        self.operation_count += 1;
        self.value
    }

    pub fn store(&mut self, value: bool, _ordering: Ordering) {
        self.operation_count += 1;
        self.value = value;
    }

    pub fn swap(&mut self, new: bool, _ordering: Ordering) -> bool {
        self.operation_count += 1;
        let old = self.value;
        self.value = new;
        old
    }

    pub fn compare_and_swap(&mut self, expected: bool, new: bool, _ordering: Ordering) -> Result<bool, bool> {
        self.operation_count += 1;
        if self.value == expected {
            let old = self.value;
            self.value = new;
            Ok(old)
        } else {
            Err(self.value)
        }
    }
}

// ── Scoped Task ──────────────────────────────────────────────────────

/// Status of a scoped task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed(ConcValue),
    Failed(String),
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed(v) => write!(f, "completed({v})"),
            Self::Failed(e) => write!(f, "failed({e})"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A scoped task that runs within a structured concurrency scope.
#[derive(Debug, Clone)]
pub struct ScopedTask {
    pub id: u64,
    pub name: String,
    pub status: TaskStatus,
    pub parent_scope: Option<String>,
}

impl ScopedTask {
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            status: TaskStatus::Pending,
            parent_scope: None,
        }
    }

    pub fn with_scope(mut self, scope: &str) -> Self {
        self.parent_scope = Some(scope.to_string());
        self
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
    }

    pub fn complete(&mut self, result: ConcValue) {
        self.status = TaskStatus::Completed(result);
    }

    pub fn fail(&mut self, error: &str) {
        self.status = TaskStatus::Failed(error.to_string());
    }

    pub fn cancel(&mut self) -> Result<(), ConcurrencyError> {
        match &self.status {
            TaskStatus::Pending | TaskStatus::Running => {
                self.status = TaskStatus::Cancelled;
                Ok(())
            }
            _ => Err(ConcurrencyError::InvalidOperation(
                "cannot cancel a completed/failed/cancelled task".into(),
            )),
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed(_) | TaskStatus::Failed(_) | TaskStatus::Cancelled
        )
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Completed(_))
    }
}

// ── Task Scope ───────────────────────────────────────────────────────

/// A structured concurrency scope that manages a group of tasks.
/// All tasks must complete before the scope exits.
#[derive(Debug, Clone)]
pub struct TaskScope {
    pub name: String,
    pub tasks: Vec<ScopedTask>,
    next_id: u64,
}

impl TaskScope {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    /// Spawn a new task in this scope.
    pub fn spawn(&mut self, task_name: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let task = ScopedTask::new(id, task_name).with_scope(&self.name);
        self.tasks.push(task);
        id
    }

    /// Start a task by ID.
    pub fn start_task(&mut self, id: u64) -> Result<(), ConcurrencyError> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.start();
            Ok(())
        } else {
            Err(ConcurrencyError::InvalidOperation(format!("task {id} not found")))
        }
    }

    /// Complete a task by ID with a result value.
    pub fn complete_task(&mut self, id: u64, result: ConcValue) -> Result<(), ConcurrencyError> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.complete(result);
            Ok(())
        } else {
            Err(ConcurrencyError::InvalidOperation(format!("task {id} not found")))
        }
    }

    /// Fail a task by ID.
    pub fn fail_task(&mut self, id: u64, error: &str) -> Result<(), ConcurrencyError> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.fail(error);
            Ok(())
        } else {
            Err(ConcurrencyError::InvalidOperation(format!("task {id} not found")))
        }
    }

    /// Cancel all tasks in the scope.
    pub fn cancel_all(&mut self) {
        for task in &mut self.tasks {
            if !task.is_done() {
                let _ = task.cancel();
            }
        }
    }

    /// Check if all tasks in the scope are done.
    pub fn all_done(&self) -> bool {
        self.tasks.iter().all(|t| t.is_done())
    }

    /// Count tasks by status.
    pub fn count_by_status(&self, status_match: &str) -> usize {
        self.tasks.iter().filter(|t| {
            match status_match {
                "pending" => matches!(t.status, TaskStatus::Pending),
                "running" => matches!(t.status, TaskStatus::Running),
                "completed" => matches!(t.status, TaskStatus::Completed(_)),
                "failed" => matches!(t.status, TaskStatus::Failed(_)),
                "cancelled" => matches!(t.status, TaskStatus::Cancelled),
                _ => false,
            }
        }).count()
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Collect results from all completed tasks.
    pub fn results(&self) -> Vec<(u64, &ConcValue)> {
        self.tasks.iter().filter_map(|t| {
            if let TaskStatus::Completed(ref v) = t.status {
                Some((t.id, v))
            } else {
                None
            }
        }).collect()
    }

    /// Collect errors from all failed tasks.
    pub fn errors(&self) -> Vec<(u64, &str)> {
        self.tasks.iter().filter_map(|t| {
            if let TaskStatus::Failed(ref e) = t.status {
                Some((t.id, e.as_str()))
            } else {
                None
            }
        }).collect()
    }
}

// ── Deadlock Detector ────────────────────────────────────────────────

/// A simple cycle-based deadlock detector for lock wait graphs.
#[derive(Debug, Clone)]
pub struct DeadlockDetector {
    /// Edges in the wait-for graph: (waiter, holder).
    edges: Vec<(String, String)>,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Record that `waiter` is waiting for a resource held by `holder`.
    pub fn add_wait(&mut self, waiter: &str, holder: &str) {
        self.edges.push((waiter.to_string(), holder.to_string()));
    }

    /// Remove a wait edge.
    pub fn remove_wait(&mut self, waiter: &str, holder: &str) {
        self.edges.retain(|(w, h)| w != waiter || h != holder);
    }

    /// Clear all edges.
    pub fn clear(&mut self) {
        self.edges.clear();
    }

    /// Detect if there is a cycle in the wait-for graph (deadlock).
    pub fn has_cycle(&self) -> bool {
        // DFS-based cycle detection
        let nodes: Vec<&str> = self.edges.iter()
            .flat_map(|(w, h)| vec![w.as_str(), h.as_str()])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for start in &nodes {
            let mut visited = std::collections::HashSet::new();
            if self.dfs_cycle(start, &mut visited) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle<'a>(&'a self, node: &str, visited: &mut std::collections::HashSet<String>) -> bool {
        if visited.contains(node) {
            return true;
        }
        visited.insert(node.to_string());
        for (w, h) in &self.edges {
            if w == node && self.dfs_cycle(h, visited) {
                return true;
            }
        }
        visited.remove(node);
        false
    }

    /// Find all nodes involved in a deadlock cycle, if any.
    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let nodes: Vec<String> = self.edges.iter()
            .flat_map(|(w, h)| vec![w.clone(), h.clone()])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for start in &nodes {
            let mut path = Vec::new();
            let mut visited = std::collections::HashSet::new();
            if self.dfs_find_cycle(start, &mut visited, &mut path) {
                return Some(path);
            }
        }
        None
    }

    fn dfs_find_cycle(&self, node: &str, visited: &mut std::collections::HashSet<String>, path: &mut Vec<String>) -> bool {
        if visited.contains(node) {
            path.push(node.to_string());
            return true;
        }
        visited.insert(node.to_string());
        path.push(node.to_string());
        for (w, h) in &self.edges {
            if w == node && self.dfs_find_cycle(h, visited, path) {
                return true;
            }
        }
        path.pop();
        visited.remove(node);
        false
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for DeadlockDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Error Display ────────────────────────────────────────────────

    #[test]
    fn test_error_display() {
        assert_eq!(format!("{}", ConcurrencyError::MutexLocked), "mutex is already locked");
        assert_eq!(format!("{}", ConcurrencyError::ChannelClosed), "channel is closed");
        assert_eq!(format!("{}", ConcurrencyError::Timeout), "operation timed out");
        assert_eq!(
            format!("{}", ConcurrencyError::InvalidOperation("test".into())),
            "invalid operation: test"
        );
    }

    // ── ConcValue ────────────────────────────────────────────────────

    #[test]
    fn test_conc_value_display() {
        assert_eq!(format!("{}", ConcValue::Int(42)), "42");
        assert_eq!(format!("{}", ConcValue::Bool(true)), "true");
        assert_eq!(format!("{}", ConcValue::Str("hello".into())), "hello");
        assert_eq!(format!("{}", ConcValue::Void), "()");
        assert_eq!(
            format!("{}", ConcValue::List(vec![ConcValue::Int(1), ConcValue::Int(2)])),
            "[1, 2]"
        );
    }

    // ── Mutex ────────────────────────────────────────────────────────

    #[test]
    fn test_mutex_lock_unlock() {
        let mut m = Mutex::new("m1", ConcValue::Int(10));
        assert!(!m.is_locked());
        assert!(m.lock("t1").is_ok());
        assert!(m.is_locked());
        assert!(m.unlock("t1").is_ok());
        assert!(!m.is_locked());
    }

    #[test]
    fn test_mutex_reentrant_fails() {
        let mut m = Mutex::new("m1", ConcValue::Int(10));
        assert!(m.lock("t1").is_ok());
        assert_eq!(m.lock("t2"), Err(ConcurrencyError::MutexLocked));
        assert_eq!(m.waiter_count(), 1);
    }

    #[test]
    fn test_mutex_try_lock() {
        let mut m = Mutex::new("m1", ConcValue::Int(0));
        assert!(m.try_lock("t1").is_ok());
        assert_eq!(m.try_lock("t2"), Err(ConcurrencyError::MutexLocked));
        // try_lock does NOT add to wait queue
        assert_eq!(m.waiter_count(), 0);
    }

    #[test]
    fn test_mutex_unlock_promotes_waiter() {
        let mut m = Mutex::new("m1", ConcValue::Int(0));
        assert!(m.lock("t1").is_ok());
        let _ = m.lock("t2"); // t2 waits
        let _ = m.lock("t3"); // t3 waits
        assert_eq!(m.waiter_count(), 2);

        m.unlock("t1").unwrap();
        // t2 should now hold the lock
        assert!(m.is_locked());
        assert_eq!(m.waiter_count(), 1);
    }

    #[test]
    fn test_mutex_set_value() {
        let mut m = Mutex::new("m1", ConcValue::Int(10));
        m.lock("t1").unwrap();
        assert!(m.set_value("t1", ConcValue::Int(42)).is_ok());
        assert_eq!(m.value, ConcValue::Int(42));
        assert!(m.set_value("t2", ConcValue::Int(0)).is_err());
    }

    #[test]
    fn test_mutex_poison() {
        let mut m = Mutex::new("m1", ConcValue::Int(0));
        m.poison();
        assert!(m.is_poisoned());
        assert_eq!(m.lock("t1"), Err(ConcurrencyError::MutexPoisoned));
    }

    #[test]
    fn test_mutex_unlock_wrong_holder() {
        let mut m = Mutex::new("m1", ConcValue::Int(0));
        m.lock("t1").unwrap();
        assert!(m.unlock("t2").is_err());
    }

    // ── RwLock ───────────────────────────────────────────────────────

    #[test]
    fn test_rwlock_multiple_readers() {
        let mut rw = RwLock::new("rw1", ConcValue::Int(100));
        assert!(rw.read_lock("r1").is_ok());
        assert!(rw.read_lock("r2").is_ok());
        assert!(rw.read_lock("r3").is_ok());
        assert_eq!(rw.reader_count(), 3);
        assert!(rw.is_read_locked());
    }

    #[test]
    fn test_rwlock_write_blocks_read() {
        let mut rw = RwLock::new("rw1", ConcValue::Int(0));
        assert!(rw.write_lock("w1").is_ok());
        assert!(rw.is_write_locked());
        assert_eq!(rw.read_lock("r1"), Err(ConcurrencyError::RwLockWriteLocked));
    }

    #[test]
    fn test_rwlock_read_blocks_write() {
        let mut rw = RwLock::new("rw1", ConcValue::Int(0));
        assert!(rw.read_lock("r1").is_ok());
        assert_eq!(rw.write_lock("w1"), Err(ConcurrencyError::RwLockReadLocked));
    }

    #[test]
    fn test_rwlock_unlock_readers() {
        let mut rw = RwLock::new("rw1", ConcValue::Int(0));
        rw.read_lock("r1").unwrap();
        rw.read_lock("r2").unwrap();
        rw.read_unlock("r1").unwrap();
        assert_eq!(rw.reader_count(), 1);
        rw.read_unlock("r2").unwrap();
        assert_eq!(rw.reader_count(), 0);
        // Now write should succeed
        assert!(rw.write_lock("w1").is_ok());
    }

    #[test]
    fn test_rwlock_write_unlock() {
        let mut rw = RwLock::new("rw1", ConcValue::Int(0));
        rw.write_lock("w1").unwrap();
        assert!(rw.write_unlock("w1").is_ok());
        assert!(!rw.is_write_locked());
    }

    // ── Channel ──────────────────────────────────────────────────────

    #[test]
    fn test_channel_bounded_send_recv() {
        let mut ch = Channel::bounded("ch1", 3);
        assert!(ch.send(ConcValue::Int(1)).is_ok());
        assert!(ch.send(ConcValue::Int(2)).is_ok());
        assert!(ch.send(ConcValue::Int(3)).is_ok());
        assert_eq!(ch.send(ConcValue::Int(4)), Err(ConcurrencyError::ChannelFull));

        assert_eq!(ch.recv(), Ok(ConcValue::Int(1)));
        assert_eq!(ch.recv(), Ok(ConcValue::Int(2)));
        assert_eq!(ch.len(), 1);
    }

    #[test]
    fn test_channel_unbounded() {
        let mut ch = Channel::unbounded("ch1");
        for i in 0..100 {
            assert!(ch.send(ConcValue::Int(i)).is_ok());
        }
        assert_eq!(ch.len(), 100);
        assert_eq!(ch.recv(), Ok(ConcValue::Int(0)));
        assert_eq!(ch.remaining_capacity(), None);
    }

    #[test]
    fn test_channel_close() {
        let mut ch = Channel::bounded("ch1", 10);
        ch.send(ConcValue::Int(1)).unwrap();
        ch.close();
        assert!(ch.is_closed());
        assert_eq!(ch.send(ConcValue::Int(2)), Err(ConcurrencyError::ChannelClosed));
        // Can still read buffered messages
        assert_eq!(ch.recv(), Ok(ConcValue::Int(1)));
        // But then closed
        assert_eq!(ch.recv(), Err(ConcurrencyError::ChannelClosed));
    }

    #[test]
    fn test_channel_drain() {
        let mut ch = Channel::unbounded("ch1");
        ch.send(ConcValue::Int(1)).unwrap();
        ch.send(ConcValue::Int(2)).unwrap();
        ch.send(ConcValue::Int(3)).unwrap();
        let drained = ch.drain();
        assert_eq!(drained.len(), 3);
        assert!(ch.is_empty());
    }

    #[test]
    fn test_channel_stats() {
        let mut ch = Channel::bounded("ch1", 10);
        ch.send(ConcValue::Int(1)).unwrap();
        ch.send(ConcValue::Int(2)).unwrap();
        ch.recv().unwrap();
        assert_eq!(ch.send_count, 2);
        assert_eq!(ch.recv_count, 1);
    }

    // ── Select ───────────────────────────────────────────────────────

    #[test]
    fn test_select_recv() {
        let mut registry = ChannelRegistry::new();
        let mut ch = Channel::unbounded("ch1");
        ch.send(ConcValue::Int(42)).unwrap();
        registry.register(ch);

        let mut sel = Select::new();
        sel.add_recv("ch1");

        let result = sel.evaluate(&mut registry);
        assert_eq!(result, SelectResult::Received {
            channel_name: "ch1".into(),
            value: ConcValue::Int(42),
        });
    }

    #[test]
    fn test_select_default() {
        let mut registry = ChannelRegistry::new();
        registry.register(Channel::unbounded("ch1")); // empty

        let mut sel = Select::new();
        sel.add_recv("ch1");
        sel.add_default();
        assert!(sel.has_default());

        let result = sel.evaluate(&mut registry);
        assert_eq!(result, SelectResult::DefaultChosen);
    }

    #[test]
    fn test_select_send() {
        let mut registry = ChannelRegistry::new();
        registry.register(Channel::bounded("ch1", 5));

        let mut sel = Select::new();
        sel.add_send("ch1", ConcValue::Int(99));

        let result = sel.evaluate(&mut registry);
        assert_eq!(result, SelectResult::Sent { channel_name: "ch1".into() });
    }

    #[test]
    fn test_select_none_ready() {
        let mut registry = ChannelRegistry::new();
        registry.register(Channel::unbounded("ch1")); // empty

        let mut sel = Select::new();
        sel.add_recv("ch1");
        // No default

        let result = sel.evaluate(&mut registry);
        assert_eq!(result, SelectResult::NoneReady);
    }

    // ── Channel Registry ─────────────────────────────────────────────

    #[test]
    fn test_channel_registry() {
        let mut reg = ChannelRegistry::new();
        reg.register(Channel::bounded("a", 10));
        reg.register(Channel::unbounded("b"));
        assert_eq!(reg.channel_count(), 2);
        assert!(reg.get("a").is_some());
        assert!(reg.get("c").is_none());

        let names = reg.channel_names();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));

        reg.remove("a");
        assert_eq!(reg.channel_count(), 1);
    }

    #[test]
    fn test_channel_registry_close_all() {
        let mut reg = ChannelRegistry::new();
        reg.register(Channel::bounded("a", 10));
        reg.register(Channel::bounded("b", 10));
        reg.close_all();
        assert!(reg.get("a").unwrap().is_closed());
        assert!(reg.get("b").unwrap().is_closed());
    }

    // ── WaitGroup ────────────────────────────────────────────────────

    #[test]
    fn test_waitgroup_basic() {
        let mut wg = WaitGroup::new("wg1");
        wg.add(3).unwrap();
        assert_eq!(wg.pending(), 3);
        assert!(!wg.is_done());

        wg.done("task1").unwrap();
        wg.done("task2").unwrap();
        wg.done("task3").unwrap();
        assert!(wg.is_done());
        assert_eq!(wg.completed_count(), 3);
    }

    #[test]
    fn test_waitgroup_negative_fails() {
        let mut wg = WaitGroup::new("wg1");
        assert_eq!(wg.done("t1"), Err(ConcurrencyError::WaitGroupNegative));
    }

    // ── Atomic Int ───────────────────────────────────────────────────

    #[test]
    fn test_atomic_int_load_store() {
        let mut a = AtomicInt::new("counter", 0);
        a.store(42, Ordering::SeqCst);
        assert_eq!(a.load(Ordering::SeqCst), 42);
        assert_eq!(a.operation_count, 2);
    }

    #[test]
    fn test_atomic_int_swap() {
        let mut a = AtomicInt::new("counter", 10);
        let old = a.swap(20, Ordering::SeqCst);
        assert_eq!(old, 10);
        assert_eq!(a.load(Ordering::SeqCst), 20);
    }

    #[test]
    fn test_atomic_int_cas_success() {
        let mut a = AtomicInt::new("counter", 10);
        assert_eq!(a.compare_and_swap(10, 20, Ordering::SeqCst), Ok(10));
        assert_eq!(a.load(Ordering::SeqCst), 20);
    }

    #[test]
    fn test_atomic_int_cas_failure() {
        let mut a = AtomicInt::new("counter", 10);
        assert_eq!(a.compare_and_swap(5, 20, Ordering::SeqCst), Err(10));
        assert_eq!(a.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_atomic_int_fetch_ops() {
        let mut a = AtomicInt::new("v", 10);
        assert_eq!(a.fetch_add(5, Ordering::SeqCst), 10);
        assert_eq!(a.load(Ordering::SeqCst), 15);
        assert_eq!(a.fetch_sub(3, Ordering::SeqCst), 15);
        assert_eq!(a.load(Ordering::SeqCst), 12);
    }

    #[test]
    fn test_atomic_int_bitwise() {
        let mut a = AtomicInt::new("v", 0b1010);
        a.fetch_and(0b1100, Ordering::SeqCst);
        assert_eq!(a.load(Ordering::SeqCst), 0b1000);
        a.fetch_or(0b0011, Ordering::SeqCst);
        assert_eq!(a.load(Ordering::SeqCst), 0b1011);
        a.fetch_xor(0b1111, Ordering::SeqCst);
        assert_eq!(a.load(Ordering::SeqCst), 0b0100);
    }

    // ── Atomic Bool ──────────────────────────────────────────────────

    #[test]
    fn test_atomic_bool() {
        let mut ab = AtomicBool::new("flag", false);
        ab.store(true, Ordering::SeqCst);
        assert!(ab.load(Ordering::SeqCst));
        let old = ab.swap(false, Ordering::SeqCst);
        assert!(old);
        assert!(!ab.load(Ordering::SeqCst));
    }

    #[test]
    fn test_atomic_bool_cas() {
        let mut ab = AtomicBool::new("flag", false);
        assert_eq!(ab.compare_and_swap(false, true, Ordering::SeqCst), Ok(false));
        assert!(ab.load(Ordering::SeqCst));
        assert_eq!(ab.compare_and_swap(false, true, Ordering::SeqCst), Err(true));
    }

    // ── Scoped Task ──────────────────────────────────────────────────

    #[test]
    fn test_scoped_task_lifecycle() {
        let mut task = ScopedTask::new(1, "worker");
        assert!(!task.is_done());
        task.start();
        assert!(!task.is_done());
        task.complete(ConcValue::Int(42));
        assert!(task.is_done());
        assert!(task.is_success());
    }

    #[test]
    fn test_scoped_task_cancel() {
        let mut task = ScopedTask::new(1, "worker");
        task.start();
        assert!(task.cancel().is_ok());
        assert!(task.is_done());
        assert!(!task.is_success());
        // Cannot cancel again
        assert!(task.cancel().is_err());
    }

    #[test]
    fn test_scoped_task_failure() {
        let mut task = ScopedTask::new(1, "worker");
        task.start();
        task.fail("some error");
        assert!(task.is_done());
        assert!(!task.is_success());
        assert_eq!(format!("{}", task.status), "failed(some error)");
    }

    // ── Task Scope ───────────────────────────────────────────────────

    #[test]
    fn test_task_scope_spawn_and_complete() {
        let mut scope = TaskScope::new("main");
        let t1 = scope.spawn("worker1");
        let t2 = scope.spawn("worker2");
        assert_eq!(scope.task_count(), 2);
        assert!(!scope.all_done());

        scope.start_task(t1).unwrap();
        scope.start_task(t2).unwrap();
        scope.complete_task(t1, ConcValue::Int(1)).unwrap();
        scope.complete_task(t2, ConcValue::Int(2)).unwrap();
        assert!(scope.all_done());

        let results = scope.results();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_task_scope_cancel_all() {
        let mut scope = TaskScope::new("main");
        let t1 = scope.spawn("w1");
        let t2 = scope.spawn("w2");
        scope.start_task(t1).unwrap();
        scope.start_task(t2).unwrap();
        scope.cancel_all();
        assert!(scope.all_done());
        assert_eq!(scope.count_by_status("cancelled"), 2);
    }

    #[test]
    fn test_task_scope_mixed_results() {
        let mut scope = TaskScope::new("main");
        let t1 = scope.spawn("w1");
        let t2 = scope.spawn("w2");
        let t3 = scope.spawn("w3");

        scope.start_task(t1).unwrap();
        scope.start_task(t2).unwrap();
        scope.start_task(t3).unwrap();

        scope.complete_task(t1, ConcValue::Str("ok".into())).unwrap();
        scope.fail_task(t2, "timeout").unwrap();
        let _ = scope.tasks.iter_mut().find(|t| t.id == t3).unwrap().cancel();

        assert!(scope.all_done());
        assert_eq!(scope.count_by_status("completed"), 1);
        assert_eq!(scope.count_by_status("failed"), 1);
        assert_eq!(scope.count_by_status("cancelled"), 1);
        assert_eq!(scope.errors().len(), 1);
    }

    // ── Deadlock Detector ────────────────────────────────────────────

    #[test]
    fn test_deadlock_no_cycle() {
        let mut dd = DeadlockDetector::new();
        dd.add_wait("t1", "t2");
        dd.add_wait("t2", "t3");
        assert!(!dd.has_cycle());
    }

    #[test]
    fn test_deadlock_cycle() {
        let mut dd = DeadlockDetector::new();
        dd.add_wait("t1", "t2");
        dd.add_wait("t2", "t3");
        dd.add_wait("t3", "t1");
        assert!(dd.has_cycle());
        let cycle = dd.find_cycle();
        assert!(cycle.is_some());
    }

    #[test]
    fn test_deadlock_self_cycle() {
        let mut dd = DeadlockDetector::new();
        dd.add_wait("t1", "t1");
        assert!(dd.has_cycle());
    }

    #[test]
    fn test_deadlock_remove_breaks_cycle() {
        let mut dd = DeadlockDetector::new();
        dd.add_wait("t1", "t2");
        dd.add_wait("t2", "t1");
        assert!(dd.has_cycle());
        dd.remove_wait("t2", "t1");
        assert!(!dd.has_cycle());
    }

    #[test]
    fn test_deadlock_clear() {
        let mut dd = DeadlockDetector::new();
        dd.add_wait("t1", "t2");
        dd.add_wait("t2", "t1");
        dd.clear();
        assert_eq!(dd.edge_count(), 0);
        assert!(!dd.has_cycle());
    }

    // ── Ordering Display ─────────────────────────────────────────────

    #[test]
    fn test_ordering_display() {
        assert_eq!(format!("{}", Ordering::SeqCst), "SeqCst");
        assert_eq!(format!("{}", Ordering::Relaxed), "Relaxed");
        assert_eq!(format!("{}", Ordering::AcqRel), "AcqRel");
    }

    // ── Integration: Channel + Select + WaitGroup ────────────────────

    #[test]
    fn test_integration_channel_select_waitgroup() {
        let mut registry = ChannelRegistry::new();
        registry.register(Channel::bounded("results", 10));
        registry.register(Channel::bounded("errors", 10));

        let mut wg = WaitGroup::new("wg");
        wg.add(2).unwrap();

        // Simulate two tasks producing results
        registry.get_mut("results").unwrap().send(ConcValue::Int(1)).unwrap();
        wg.done("task1").unwrap();
        registry.get_mut("errors").unwrap().send(ConcValue::Str("fail".into())).unwrap();
        wg.done("task2").unwrap();

        assert!(wg.is_done());

        // Select from results first
        let mut sel = Select::new();
        sel.add_recv("results");
        sel.add_recv("errors");
        let r = sel.evaluate(&mut registry);
        assert_eq!(r, SelectResult::Received {
            channel_name: "results".into(),
            value: ConcValue::Int(1),
        });
    }
}
