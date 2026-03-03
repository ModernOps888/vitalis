//! Debug Adapter Protocol (DAP) Support
//!
//! Implements the core DAP types and session logic for IDE debugging:
//! - Breakpoint management
//! - Variable inspection
//! - Call stack frames
//! - Step operations (in, over, out, continue)
//! - Expression evaluation during debug

use std::collections::HashMap;

/// Source location for breakpoints and stack frames
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// Breakpoint state
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub id: u32,
    pub location: SourceLocation,
    pub enabled: bool,
    pub condition: Option<String>,
    pub hit_count: u32,
}

/// A call stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub id: u32,
    pub name: String,
    pub location: SourceLocation,
    pub locals: HashMap<String, DebugValue>,
}

/// Debug value representation
#[derive(Debug, Clone)]
pub enum DebugValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<DebugValue>),
    Struct { name: String, fields: Vec<(String, DebugValue)> },
    Null,
}

impl std::fmt::Display for DebugValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugValue::Int(v) => write!(f, "{}", v),
            DebugValue::Float(v) => write!(f, "{:.6}", v),
            DebugValue::Bool(v) => write!(f, "{}", v),
            DebugValue::Str(v) => write!(f, "\"{}\"", v),
            DebugValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            DebugValue::Struct { name, fields } => {
                write!(f, "{} {{ ", name)?;
                for (i, (key, val)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", key, val)?;
                }
                write!(f, " }}")
            }
            DebugValue::Null => write!(f, "null"),
        }
    }
}

/// Debug execution state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionState {
    Running,
    Paused,
    Stopped,
    SteppingIn,
    SteppingOver,
    SteppingOut,
}

/// DAP event types
#[derive(Debug, Clone)]
pub enum DapEvent {
    Initialized,
    Stopped { reason: StopReason },
    Continued,
    Exited { code: i32 },
    Output { text: String, category: OutputCategory },
    Breakpoint { id: u32, verified: bool },
}

/// Reason execution stopped
#[derive(Debug, Clone)]
pub enum StopReason {
    Breakpoint(u32),
    Step,
    Pause,
    Exception(String),
    Entry,
}

/// Output category
#[derive(Debug, Clone)]
pub enum OutputCategory {
    Console,
    Stdout,
    Stderr,
}

/// The debug session
pub struct DebugSession {
    pub breakpoints: Vec<Breakpoint>,
    pub stack: Vec<StackFrame>,
    pub state: ExecutionState,
    pub event_log: Vec<DapEvent>,
    next_bp_id: u32,
    next_frame_id: u32,
    watched_expressions: Vec<String>,
}

impl DebugSession {
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
            stack: Vec::new(),
            state: ExecutionState::Stopped,
            event_log: Vec::new(),
            next_bp_id: 1,
            next_frame_id: 1,
            watched_expressions: Vec::new(),
        }
    }

    /// Add a breakpoint at a source location
    pub fn add_breakpoint(&mut self, file: &str, line: u32) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.breakpoints.push(Breakpoint {
            id,
            location: SourceLocation {
                file: file.to_string(),
                line,
                column: 0,
            },
            enabled: true,
            condition: None,
            hit_count: 0,
        });
        self.emit(DapEvent::Breakpoint { id, verified: true });
        id
    }

    /// Add a conditional breakpoint
    pub fn add_conditional_breakpoint(&mut self, file: &str, line: u32, condition: &str) -> u32 {
        let id = self.add_breakpoint(file, line);
        if let Some(bp) = self.breakpoints.iter_mut().find(|b| b.id == id) {
            bp.condition = Some(condition.to_string());
        }
        id
    }

    /// Remove a breakpoint by ID
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        let len_before = self.breakpoints.len();
        self.breakpoints.retain(|bp| bp.id != id);
        self.breakpoints.len() < len_before
    }

    /// Toggle breakpoint enabled/disabled
    pub fn toggle_breakpoint(&mut self, id: u32) -> bool {
        if let Some(bp) = self.breakpoints.iter_mut().find(|b| b.id == id) {
            bp.enabled = !bp.enabled;
            true
        } else {
            false
        }
    }

    /// Check if we should stop at the given location
    pub fn should_stop(&self, file: &str, line: u32) -> bool {
        self.breakpoints.iter().any(|bp| {
            bp.enabled && bp.location.file == file && bp.location.line == line
        })
    }

    /// Hit a breakpoint at a location
    pub fn hit_breakpoint(&mut self, file: &str, line: u32) -> Option<u32> {
        let bp = self.breakpoints.iter_mut()
            .find(|bp| bp.enabled && bp.location.file == file && bp.location.line == line)?;
        bp.hit_count += 1;
        let id = bp.id;
        self.state = ExecutionState::Paused;
        self.emit(DapEvent::Stopped { reason: StopReason::Breakpoint(id) });
        Some(id)
    }

    /// Push a stack frame
    pub fn push_frame(&mut self, name: &str, file: &str, line: u32) -> u32 {
        let id = self.next_frame_id;
        self.next_frame_id += 1;
        self.stack.push(StackFrame {
            id,
            name: name.to_string(),
            location: SourceLocation {
                file: file.to_string(),
                line,
                column: 0,
            },
            locals: HashMap::new(),
        });
        id
    }

    /// Pop the top stack frame
    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        self.stack.pop()
    }

    /// Set a local variable in the top frame
    pub fn set_local(&mut self, name: &str, value: DebugValue) {
        if let Some(frame) = self.stack.last_mut() {
            frame.locals.insert(name.to_string(), value);
        }
    }

    /// Get a local variable from the call stack (searches top-down)
    pub fn get_local(&self, name: &str) -> Option<&DebugValue> {
        for frame in self.stack.iter().rev() {
            if let Some(val) = frame.locals.get(name) {
                return Some(val);
            }
        }
        None
    }

    /// Continue execution
    pub fn continue_execution(&mut self) {
        self.state = ExecutionState::Running;
        self.emit(DapEvent::Continued);
    }

    /// Step into next function call
    pub fn step_in(&mut self) {
        self.state = ExecutionState::SteppingIn;
    }

    /// Step over current line
    pub fn step_over(&mut self) {
        self.state = ExecutionState::SteppingOver;
    }

    /// Step out of current function
    pub fn step_out(&mut self) {
        self.state = ExecutionState::SteppingOut;
    }

    /// Pause execution
    pub fn pause(&mut self) {
        self.state = ExecutionState::Paused;
        self.emit(DapEvent::Stopped { reason: StopReason::Pause });
    }

    /// Stop the debug session
    pub fn stop(&mut self, exit_code: i32) {
        self.state = ExecutionState::Stopped;
        self.stack.clear();
        self.emit(DapEvent::Exited { code: exit_code });
    }

    /// Add a watch expression
    pub fn add_watch(&mut self, expr: &str) {
        self.watched_expressions.push(expr.to_string());
    }

    /// Get all watch expressions
    pub fn watches(&self) -> &[String] {
        &self.watched_expressions
    }

    /// Log output
    pub fn output(&mut self, text: &str, category: OutputCategory) {
        self.emit(DapEvent::Output {
            text: text.to_string(),
            category,
        });
    }

    fn emit(&mut self, event: DapEvent) {
        self.event_log.push(event);
    }

    /// Get the number of events logged
    pub fn event_count(&self) -> usize {
        self.event_log.len()
    }

    /// Get current call depth
    pub fn call_depth(&self) -> usize {
        self.stack.len()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dap_session_creation() {
        let session = DebugSession::new();
        assert_eq!(session.state, ExecutionState::Stopped);
        assert!(session.breakpoints.is_empty());
        assert!(session.stack.is_empty());
    }

    #[test]
    fn test_dap_add_breakpoint() {
        let mut session = DebugSession::new();
        let id = session.add_breakpoint("main.sl", 10);
        assert_eq!(id, 1);
        assert_eq!(session.breakpoints.len(), 1);
        assert_eq!(session.breakpoints[0].location.line, 10);
    }

    #[test]
    fn test_dap_conditional_breakpoint() {
        let mut session = DebugSession::new();
        let id = session.add_conditional_breakpoint("main.sl", 5, "x > 10");
        let bp = session.breakpoints.iter().find(|b| b.id == id).unwrap();
        assert_eq!(bp.condition.as_deref(), Some("x > 10"));
    }

    #[test]
    fn test_dap_remove_breakpoint() {
        let mut session = DebugSession::new();
        let id = session.add_breakpoint("main.sl", 10);
        assert!(session.remove_breakpoint(id));
        assert!(session.breakpoints.is_empty());
    }

    #[test]
    fn test_dap_toggle_breakpoint() {
        let mut session = DebugSession::new();
        let id = session.add_breakpoint("main.sl", 10);
        assert!(session.breakpoints[0].enabled);
        session.toggle_breakpoint(id);
        assert!(!session.breakpoints[0].enabled);
        session.toggle_breakpoint(id);
        assert!(session.breakpoints[0].enabled);
    }

    #[test]
    fn test_dap_should_stop() {
        let mut session = DebugSession::new();
        session.add_breakpoint("main.sl", 10);
        assert!(session.should_stop("main.sl", 10));
        assert!(!session.should_stop("main.sl", 11));
        assert!(!session.should_stop("other.sl", 10));
    }

    #[test]
    fn test_dap_hit_breakpoint() {
        let mut session = DebugSession::new();
        session.add_breakpoint("main.sl", 10);
        let id = session.hit_breakpoint("main.sl", 10);
        assert_eq!(id, Some(1));
        assert_eq!(session.state, ExecutionState::Paused);
        assert_eq!(session.breakpoints[0].hit_count, 1);
    }

    #[test]
    fn test_dap_stack_frames() {
        let mut session = DebugSession::new();
        session.push_frame("main", "main.sl", 1);
        session.push_frame("helper", "main.sl", 20);
        assert_eq!(session.call_depth(), 2);
        let frame = session.pop_frame().unwrap();
        assert_eq!(frame.name, "helper");
        assert_eq!(session.call_depth(), 1);
    }

    #[test]
    fn test_dap_locals() {
        let mut session = DebugSession::new();
        session.push_frame("main", "main.sl", 1);
        session.set_local("x", DebugValue::Int(42));
        let val = session.get_local("x").unwrap();
        assert!(matches!(val, DebugValue::Int(42)));
    }

    #[test]
    fn test_dap_continue() {
        let mut session = DebugSession::new();
        session.state = ExecutionState::Paused;
        session.continue_execution();
        assert_eq!(session.state, ExecutionState::Running);
    }

    #[test]
    fn test_dap_step_in() {
        let mut session = DebugSession::new();
        session.step_in();
        assert_eq!(session.state, ExecutionState::SteppingIn);
    }

    #[test]
    fn test_dap_step_over() {
        let mut session = DebugSession::new();
        session.step_over();
        assert_eq!(session.state, ExecutionState::SteppingOver);
    }

    #[test]
    fn test_dap_step_out() {
        let mut session = DebugSession::new();
        session.step_out();
        assert_eq!(session.state, ExecutionState::SteppingOut);
    }

    #[test]
    fn test_dap_pause() {
        let mut session = DebugSession::new();
        session.state = ExecutionState::Running;
        session.pause();
        assert_eq!(session.state, ExecutionState::Paused);
    }

    #[test]
    fn test_dap_stop() {
        let mut session = DebugSession::new();
        session.push_frame("main", "main.sl", 1);
        session.stop(0);
        assert_eq!(session.state, ExecutionState::Stopped);
        assert!(session.stack.is_empty());
    }

    #[test]
    fn test_dap_watches() {
        let mut session = DebugSession::new();
        session.add_watch("x + y");
        session.add_watch("arr.len()");
        assert_eq!(session.watches().len(), 2);
    }

    #[test]
    fn test_dap_debug_value_display() {
        assert_eq!(format!("{}", DebugValue::Int(42)), "42");
        assert_eq!(format!("{}", DebugValue::Bool(true)), "true");
        assert_eq!(format!("{}", DebugValue::Str("hi".into())), "\"hi\"");
        assert_eq!(format!("{}", DebugValue::Null), "null");
    }

    #[test]
    fn test_dap_debug_value_list_display() {
        let list = DebugValue::List(vec![
            DebugValue::Int(1),
            DebugValue::Int(2),
            DebugValue::Int(3),
        ]);
        assert_eq!(format!("{}", list), "[1, 2, 3]");
    }

    #[test]
    fn test_dap_debug_value_struct_display() {
        let s = DebugValue::Struct {
            name: "Point".into(),
            fields: vec![
                ("x".into(), DebugValue::Int(10)),
                ("y".into(), DebugValue::Int(20)),
            ],
        };
        assert_eq!(format!("{}", s), "Point { x: 10, y: 20 }");
    }

    #[test]
    fn test_dap_event_count() {
        let mut session = DebugSession::new();
        session.add_breakpoint("main.sl", 1);
        session.continue_execution();
        session.pause();
        assert_eq!(session.event_count(), 3); // bp verified + continued + stopped
    }

    #[test]
    fn test_dap_output_event() {
        let mut session = DebugSession::new();
        session.output("Hello world", OutputCategory::Stdout);
        assert_eq!(session.event_count(), 1);
    }

    #[test]
    fn test_dap_remove_nonexistent_bp() {
        let mut session = DebugSession::new();
        assert!(!session.remove_breakpoint(99));
    }

    #[test]
    fn test_dap_toggle_nonexistent_bp() {
        let mut session = DebugSession::new();
        assert!(!session.toggle_breakpoint(99));
    }

    #[test]
    fn test_dap_hit_no_breakpoint() {
        let mut session = DebugSession::new();
        assert!(session.hit_breakpoint("main.sl", 99).is_none());
    }

    #[test]
    fn test_dap_disabled_bp_no_stop() {
        let mut session = DebugSession::new();
        let id = session.add_breakpoint("main.sl", 10);
        session.toggle_breakpoint(id); // disable
        assert!(!session.should_stop("main.sl", 10));
    }

    #[test]
    fn test_dap_multiple_breakpoints() {
        let mut session = DebugSession::new();
        session.add_breakpoint("main.sl", 5);
        session.add_breakpoint("main.sl", 10);
        session.add_breakpoint("utils.sl", 3);
        assert_eq!(session.breakpoints.len(), 3);
    }

    #[test]
    fn test_dap_locals_search_stack() {
        let mut session = DebugSession::new();
        session.push_frame("outer", "main.sl", 1);
        session.set_local("x", DebugValue::Int(10));
        session.push_frame("inner", "main.sl", 5);
        session.set_local("y", DebugValue::Int(20));
        // y found in top frame
        assert!(matches!(session.get_local("y"), Some(DebugValue::Int(20))));
        // x found by searching down
        assert!(matches!(session.get_local("x"), Some(DebugValue::Int(10))));
    }

    #[test]
    fn test_dap_float_display() {
        let val = DebugValue::Float(3.14);
        let s = format!("{}", val);
        assert!(s.starts_with("3.14"));
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation {
            file: "test.sl".into(),
            line: 42,
            column: 5,
        };
        assert_eq!(loc.file, "test.sl");
        assert_eq!(loc.line, 42);
    }
}
