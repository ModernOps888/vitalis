//! Time-travel debugging for Vitalis.
//!
//! Record-replay debugging with reverse stepping:
//! - **Execution recording**: Instruction-level traces with memory snapshots
//! - **Deterministic replay**: Replay non-deterministic events from recorded trace
//! - **Reverse stepping**: Step backwards through execution history
//! - **Reverse watchpoints**: Find when a variable last changed
//! - **Trace diffing**: Compare two traces to find divergence
//! - **Snapshot compression**: Delta-compressed memory snapshots

use std::collections::HashMap;

// ── Trace Events ────────────────────────────────────────────────────

/// Unique identifier for a trace point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TracePointId(pub u64);

/// A recorded event in the execution trace.
#[derive(Debug, Clone)]
pub enum TraceEvent {
    /// Function entry.
    FunctionEnter { name: String, args: Vec<TraceValue> },
    /// Function exit.
    FunctionExit { name: String, result: TraceValue },
    /// Variable assignment.
    VarWrite { name: String, old: TraceValue, new: TraceValue },
    /// Memory read.
    MemRead { addr: u64, size: u32, value: Vec<u8> },
    /// Memory write.
    MemWrite { addr: u64, size: u32, old: Vec<u8>, new: Vec<u8> },
    /// Branch taken.
    Branch { location: SourceLocation, taken: bool },
    /// I/O event (non-deterministic).
    IoEvent { kind: IoKind, data: Vec<u8> },
    /// Allocation.
    Alloc { addr: u64, size: u64 },
    /// Deallocation.
    Dealloc { addr: u64 },
    /// Snapshot marker.
    Snapshot { id: u32 },
}

/// A recorded value.
#[derive(Debug, Clone, PartialEq)]
pub enum TraceValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Ptr(u64),
    Void,
    Bytes(Vec<u8>),
}

/// I/O event kind.
#[derive(Debug, Clone, PartialEq)]
pub enum IoKind {
    StdoutWrite,
    StdinRead,
    FileRead,
    FileWrite,
    NetworkSend,
    NetworkRecv,
    RandomSeed,
    TimestampQuery,
}

/// Source location.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

// ── Memory Snapshot ─────────────────────────────────────────────────

/// A memory snapshot at a point in time.
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub id: u32,
    pub trace_point: TracePointId,
    pub regions: Vec<MemoryRegion>,
}

/// A contiguous memory region.
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub base: u64,
    pub data: Vec<u8>,
}

/// Delta-compressed snapshot (stores only changes from previous).
#[derive(Debug, Clone)]
pub struct DeltaSnapshot {
    pub base_snapshot_id: u32,
    pub trace_point: TracePointId,
    pub deltas: Vec<MemoryDelta>,
}

/// A single memory delta.
#[derive(Debug, Clone)]
pub struct MemoryDelta {
    pub addr: u64,
    pub old_bytes: Vec<u8>,
    pub new_bytes: Vec<u8>,
}

impl DeltaSnapshot {
    /// Compute compressed size (sum of delta sizes).
    pub fn compressed_size(&self) -> usize {
        self.deltas.iter().map(|d| d.old_bytes.len() + d.new_bytes.len() + 8).sum()
    }

    /// Number of changed regions.
    pub fn change_count(&self) -> usize {
        self.deltas.len()
    }
}

// ── Execution Trace ─────────────────────────────────────────────────

/// A recorded execution trace.
pub struct ExecutionTrace {
    events: Vec<(TracePointId, TraceEvent)>,
    snapshots: Vec<MemorySnapshot>,
    delta_snapshots: Vec<DeltaSnapshot>,
    next_id: u64,
    snapshot_interval: u64,
}

impl ExecutionTrace {
    pub fn new(snapshot_interval: u64) -> Self {
        Self {
            events: Vec::new(),
            snapshots: Vec::new(),
            delta_snapshots: Vec::new(),
            next_id: 0,
            snapshot_interval,
        }
    }

    /// Record a trace event.
    pub fn record(&mut self, event: TraceEvent) -> TracePointId {
        let id = TracePointId(self.next_id);
        self.next_id += 1;

        // Take snapshot at intervals.
        if self.next_id % self.snapshot_interval == 0 {
            let snapshot_id = self.snapshots.len() as u32;
            self.events.push((id, TraceEvent::Snapshot { id: snapshot_id }));
        }

        self.events.push((id, event));
        id
    }

    /// Add a full memory snapshot.
    pub fn add_snapshot(&mut self, snapshot: MemorySnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Add a delta snapshot.
    pub fn add_delta_snapshot(&mut self, delta: DeltaSnapshot) {
        self.delta_snapshots.push(delta);
    }

    /// Total number of trace events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get event at a trace point.
    pub fn get_event(&self, id: TracePointId) -> Option<&TraceEvent> {
        self.events.iter().find(|(tp, _)| *tp == id).map(|(_, e)| e)
    }

    /// Get all events in range [from, to] inclusive.
    pub fn events_in_range(&self, from: TracePointId, to: TracePointId) -> Vec<&(TracePointId, TraceEvent)> {
        self.events.iter()
            .filter(|(tp, _)| tp.0 >= from.0 && tp.0 <= to.0)
            .collect()
    }

    /// Find last write to a variable before a given trace point.
    pub fn reverse_watchpoint(&self, var_name: &str, before: TracePointId) -> Option<TracePointId> {
        self.events.iter().rev()
            .filter(|(tp, _)| tp.0 < before.0)
            .find_map(|(tp, event)| {
                if let TraceEvent::VarWrite { name, .. } = event {
                    if name == var_name {
                        return Some(*tp);
                    }
                }
                None
            })
    }

    /// Find all writes to a variable.
    pub fn var_history(&self, var_name: &str) -> Vec<(TracePointId, &TraceValue, &TraceValue)> {
        self.events.iter().filter_map(|(tp, event)| {
            if let TraceEvent::VarWrite { name, old, new } = event {
                if name == var_name {
                    return Some((*tp, old, new));
                }
            }
            None
        }).collect()
    }

    /// Get the call stack at a given trace point.
    pub fn call_stack_at(&self, point: TracePointId) -> Vec<String> {
        let mut stack = Vec::new();
        for (tp, event) in &self.events {
            if tp.0 > point.0 { break; }
            match event {
                TraceEvent::FunctionEnter { name, .. } => stack.push(name.clone()),
                TraceEvent::FunctionExit { .. } => { stack.pop(); }
                _ => {}
            }
        }
        stack
    }
}

// ── Trace Diffing ───────────────────────────────────────────────────

/// Result of comparing two execution traces.
#[derive(Debug, Clone)]
pub struct TraceDiff {
    pub divergence_point: Option<u64>,
    pub left_only_events: usize,
    pub right_only_events: usize,
    pub common_prefix_length: u64,
}

/// Compare two execution traces and find where they diverge.
pub fn diff_traces(left: &ExecutionTrace, right: &ExecutionTrace) -> TraceDiff {
    let mut common = 0u64;

    let min_len = left.events.len().min(right.events.len());
    for i in 0..min_len {
        if !events_match(&left.events[i].1, &right.events[i].1) {
            return TraceDiff {
                divergence_point: Some(i as u64),
                left_only_events: left.events.len() - i,
                right_only_events: right.events.len() - i,
                common_prefix_length: common,
            };
        }
        common += 1;
    }

    TraceDiff {
        divergence_point: if left.events.len() != right.events.len() {
            Some(min_len as u64)
        } else {
            None
        },
        left_only_events: left.events.len().saturating_sub(min_len),
        right_only_events: right.events.len().saturating_sub(min_len),
        common_prefix_length: common,
    }
}

fn events_match(a: &TraceEvent, b: &TraceEvent) -> bool {
    match (a, b) {
        (TraceEvent::FunctionEnter { name: n1, .. }, TraceEvent::FunctionEnter { name: n2, .. }) => n1 == n2,
        (TraceEvent::FunctionExit { name: n1, .. }, TraceEvent::FunctionExit { name: n2, .. }) => n1 == n2,
        (TraceEvent::VarWrite { name: n1, new: v1, .. }, TraceEvent::VarWrite { name: n2, new: v2, .. }) => n1 == n2 && v1 == v2,
        (TraceEvent::Branch { taken: t1, .. }, TraceEvent::Branch { taken: t2, .. }) => t1 == t2,
        _ => false,
    }
}

// ── Replay Controller ───────────────────────────────────────────────

/// Controls replaying an execution trace.
pub struct ReplayController {
    trace: ExecutionTrace,
    position: usize,
    breakpoints: Vec<ReplayBreakpoint>,
    watchpoints: Vec<String>,
}

/// A replay breakpoint.
#[derive(Debug, Clone)]
pub struct ReplayBreakpoint {
    pub id: u32,
    pub kind: BreakpointKind,
    pub enabled: bool,
    pub hit_count: u32,
}

/// Breakpoint targeting.
#[derive(Debug, Clone)]
pub enum BreakpointKind {
    TracePoint(TracePointId),
    Function(String),
    VarChange(String),
    Location(SourceLocation),
}

impl ReplayController {
    pub fn new(trace: ExecutionTrace) -> Self {
        Self {
            trace,
            position: 0,
            breakpoints: Vec::new(),
            watchpoints: Vec::new(),
        }
    }

    /// Step forward one event.
    pub fn step_forward(&mut self) -> Option<&TraceEvent> {
        if self.position < self.trace.events.len() {
            let event = &self.trace.events[self.position].1;
            self.position += 1;
            Some(event)
        } else {
            None
        }
    }

    /// Step backward one event.
    pub fn step_backward(&mut self) -> Option<&TraceEvent> {
        if self.position > 0 {
            self.position -= 1;
            Some(&self.trace.events[self.position].1)
        } else {
            None
        }
    }

    /// Continue forward to next breakpoint.
    pub fn continue_forward(&mut self) -> Option<&TraceEvent> {
        while self.position < self.trace.events.len() {
            let event = &self.trace.events[self.position];
            self.position += 1;
            if self.hits_breakpoint(&event.1) {
                return Some(&event.1);
            }
        }
        None
    }

    /// Continue backward to previous breakpoint.
    pub fn continue_backward(&mut self) -> Option<&TraceEvent> {
        while self.position > 0 {
            self.position -= 1;
            let event = &self.trace.events[self.position];
            if self.hits_breakpoint(&event.1) {
                return Some(&event.1);
            }
        }
        None
    }

    /// Add a breakpoint.
    pub fn add_breakpoint(&mut self, kind: BreakpointKind) -> u32 {
        let id = self.breakpoints.len() as u32;
        self.breakpoints.push(ReplayBreakpoint {
            id,
            kind,
            enabled: true,
            hit_count: 0,
        });
        id
    }

    /// Add a watchpoint for variable changes.
    pub fn add_watchpoint(&mut self, var_name: String) {
        self.watchpoints.push(var_name);
    }

    /// Current position in the trace.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Total events in trace.
    pub fn total_events(&self) -> usize {
        self.trace.events.len()
    }

    /// Jump to a specific trace point.
    pub fn jump_to(&mut self, point: TracePointId) -> bool {
        if let Some(pos) = self.trace.events.iter().position(|(tp, _)| *tp == point) {
            self.position = pos;
            true
        } else {
            false
        }
    }

    /// Get call stack at current position.
    pub fn current_call_stack(&self) -> Vec<String> {
        if self.position == 0 {
            return Vec::new();
        }
        let current_tp = self.trace.events[self.position - 1].0;
        self.trace.call_stack_at(current_tp)
    }

    fn hits_breakpoint(&self, event: &TraceEvent) -> bool {
        for bp in &self.breakpoints {
            if !bp.enabled { continue; }
            match (&bp.kind, event) {
                (BreakpointKind::Function(name), TraceEvent::FunctionEnter { name: fn_name, .. }) => {
                    if name == fn_name { return true; }
                }
                (BreakpointKind::VarChange(name), TraceEvent::VarWrite { name: var_name, .. }) => {
                    if name == var_name { return true; }
                }
                _ => {}
            }
        }
        // Check watchpoints.
        if let TraceEvent::VarWrite { name, .. } = event {
            if self.watchpoints.iter().any(|w| w == name) {
                return true;
            }
        }
        false
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_record_events() {
        let mut trace = ExecutionTrace::new(100);
        let id = trace.record(TraceEvent::FunctionEnter {
            name: "main".into(),
            args: vec![],
        });
        assert_eq!(id, TracePointId(0));
        assert_eq!(trace.event_count(), 1);
    }

    #[test]
    fn test_trace_var_write() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::VarWrite {
            name: "x".into(),
            old: TraceValue::Int(0),
            new: TraceValue::Int(42),
        });
        let history = trace.var_history("x");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].2, &TraceValue::Int(42));
    }

    #[test]
    fn test_reverse_watchpoint() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::VarWrite {
            name: "x".into(),
            old: TraceValue::Int(0),
            new: TraceValue::Int(1),
        });
        let tp2 = trace.record(TraceEvent::VarWrite {
            name: "x".into(),
            old: TraceValue::Int(1),
            new: TraceValue::Int(2),
        });
        trace.record(TraceEvent::VarWrite {
            name: "y".into(),
            old: TraceValue::Int(0),
            new: TraceValue::Int(10),
        });
        let result = trace.reverse_watchpoint("x", TracePointId(3));
        assert_eq!(result, Some(tp2));
    }

    #[test]
    fn test_call_stack() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::FunctionEnter { name: "main".into(), args: vec![] });
        trace.record(TraceEvent::FunctionEnter { name: "foo".into(), args: vec![] });
        let stack = trace.call_stack_at(TracePointId(1));
        assert_eq!(stack, vec!["main", "foo"]);
    }

    #[test]
    fn test_call_stack_after_exit() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::FunctionEnter { name: "main".into(), args: vec![] });
        trace.record(TraceEvent::FunctionEnter { name: "foo".into(), args: vec![] });
        trace.record(TraceEvent::FunctionExit { name: "foo".into(), result: TraceValue::Void });
        let stack = trace.call_stack_at(TracePointId(2));
        assert_eq!(stack, vec!["main"]);
    }

    #[test]
    fn test_events_in_range() {
        let mut trace = ExecutionTrace::new(100);
        for i in 0..10 {
            trace.record(TraceEvent::VarWrite {
                name: format!("v{i}"),
                old: TraceValue::Int(0),
                new: TraceValue::Int(i),
            });
        }
        let range = trace.events_in_range(TracePointId(3), TracePointId(6));
        assert_eq!(range.len(), 4);
    }

    #[test]
    fn test_trace_diff_identical() {
        let mut t1 = ExecutionTrace::new(100);
        let mut t2 = ExecutionTrace::new(100);
        t1.record(TraceEvent::FunctionEnter { name: "f".into(), args: vec![] });
        t2.record(TraceEvent::FunctionEnter { name: "f".into(), args: vec![] });
        let diff = diff_traces(&t1, &t2);
        assert!(diff.divergence_point.is_none());
        assert_eq!(diff.common_prefix_length, 1);
    }

    #[test]
    fn test_trace_diff_divergent() {
        let mut t1 = ExecutionTrace::new(100);
        let mut t2 = ExecutionTrace::new(100);
        t1.record(TraceEvent::FunctionEnter { name: "f".into(), args: vec![] });
        t2.record(TraceEvent::FunctionEnter { name: "g".into(), args: vec![] });
        let diff = diff_traces(&t1, &t2);
        assert_eq!(diff.divergence_point, Some(0));
    }

    #[test]
    fn test_replay_step_forward() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::FunctionEnter { name: "main".into(), args: vec![] });
        trace.record(TraceEvent::FunctionExit { name: "main".into(), result: TraceValue::Void });
        let mut ctrl = ReplayController::new(trace);
        assert!(ctrl.step_forward().is_some());
        assert!(ctrl.step_forward().is_some());
        assert!(ctrl.step_forward().is_none());
    }

    #[test]
    fn test_replay_step_backward() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::FunctionEnter { name: "main".into(), args: vec![] });
        trace.record(TraceEvent::FunctionExit { name: "main".into(), result: TraceValue::Void });
        let mut ctrl = ReplayController::new(trace);
        ctrl.step_forward();
        ctrl.step_forward();
        assert!(ctrl.step_backward().is_some());
        assert!(ctrl.step_backward().is_some());
        assert!(ctrl.step_backward().is_none());
    }

    #[test]
    fn test_replay_breakpoint() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::VarWrite { name: "x".into(), old: TraceValue::Int(0), new: TraceValue::Int(1) });
        trace.record(TraceEvent::FunctionEnter { name: "target".into(), args: vec![] });
        trace.record(TraceEvent::VarWrite { name: "y".into(), old: TraceValue::Int(0), new: TraceValue::Int(2) });
        let mut ctrl = ReplayController::new(trace);
        ctrl.add_breakpoint(BreakpointKind::Function("target".into()));
        let hit = ctrl.continue_forward();
        assert!(hit.is_some());
    }

    #[test]
    fn test_replay_watchpoint() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::VarWrite { name: "a".into(), old: TraceValue::Int(0), new: TraceValue::Int(1) });
        trace.record(TraceEvent::VarWrite { name: "b".into(), old: TraceValue::Int(0), new: TraceValue::Int(2) });
        trace.record(TraceEvent::VarWrite { name: "a".into(), old: TraceValue::Int(1), new: TraceValue::Int(3) });
        let mut ctrl = ReplayController::new(trace);
        ctrl.add_watchpoint("a".into());
        let hit = ctrl.continue_forward();
        assert!(hit.is_some());
        assert_eq!(ctrl.position(), 1);
    }

    #[test]
    fn test_replay_jump_to() {
        let mut trace = ExecutionTrace::new(100);
        for i in 0..5 {
            trace.record(TraceEvent::VarWrite { name: format!("v{i}"), old: TraceValue::Int(0), new: TraceValue::Int(i) });
        }
        let mut ctrl = ReplayController::new(trace);
        assert!(ctrl.jump_to(TracePointId(3)));
        assert_eq!(ctrl.position(), 3);
    }

    #[test]
    fn test_delta_snapshot() {
        let delta = DeltaSnapshot {
            base_snapshot_id: 0,
            trace_point: TracePointId(100),
            deltas: vec![
                MemoryDelta { addr: 0x1000, old_bytes: vec![0; 4], new_bytes: vec![1; 4] },
                MemoryDelta { addr: 0x2000, old_bytes: vec![0; 8], new_bytes: vec![2; 8] },
            ],
        };
        assert_eq!(delta.change_count(), 2);
        assert!(delta.compressed_size() > 0);
    }

    #[test]
    fn test_trace_value_equality() {
        assert_eq!(TraceValue::Int(42), TraceValue::Int(42));
        assert_ne!(TraceValue::Int(1), TraceValue::Float(1.0));
    }

    #[test]
    fn test_io_kind_variants() {
        let kinds = vec![
            IoKind::StdoutWrite, IoKind::StdinRead, IoKind::FileRead,
            IoKind::FileWrite, IoKind::NetworkSend, IoKind::NetworkRecv,
            IoKind::RandomSeed, IoKind::TimestampQuery,
        ];
        assert_eq!(kinds.len(), 8);
    }

    #[test]
    fn test_memory_snapshot() {
        let snap = MemorySnapshot {
            id: 0,
            trace_point: TracePointId(50),
            regions: vec![
                MemoryRegion { base: 0x1000, data: vec![0xAA; 64] },
                MemoryRegion { base: 0x2000, data: vec![0xBB; 128] },
            ],
        };
        assert_eq!(snap.regions.len(), 2);
        assert_eq!(snap.regions[0].data.len(), 64);
    }

    #[test]
    fn test_replay_current_call_stack() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::FunctionEnter { name: "main".into(), args: vec![] });
        trace.record(TraceEvent::FunctionEnter { name: "helper".into(), args: vec![] });
        let mut ctrl = ReplayController::new(trace);
        ctrl.step_forward();
        ctrl.step_forward();
        let stack = ctrl.current_call_stack();
        assert_eq!(stack, vec!["main", "helper"]);
    }

    #[test]
    fn test_var_history_multiple() {
        let mut trace = ExecutionTrace::new(100);
        trace.record(TraceEvent::VarWrite { name: "x".into(), old: TraceValue::Int(0), new: TraceValue::Int(1) });
        trace.record(TraceEvent::VarWrite { name: "y".into(), old: TraceValue::Int(0), new: TraceValue::Int(5) });
        trace.record(TraceEvent::VarWrite { name: "x".into(), old: TraceValue::Int(1), new: TraceValue::Int(2) });
        trace.record(TraceEvent::VarWrite { name: "x".into(), old: TraceValue::Int(2), new: TraceValue::Int(3) });
        let hist = trace.var_history("x");
        assert_eq!(hist.len(), 3);
    }
}
