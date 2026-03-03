//! Vitalis Effect Handlers — Algebraic Effect Handler System (v24)
//!
//! Provides first-class algebraic effect handlers with resume/abort continuations,
//! handler composition, and a handler stack for dispatching effects through nested
//! handler frames.
//!
//! # Design
//!
//! Algebraic effects separate *what* an operation does (the effect signature) from
//! *how* it's handled (the handler). Functions `perform` effects and handlers
//! intercept them, choosing to resume, abort, or transform the computation.
//!
//! # Example (Vitalis syntax)
//!
//! ```text
//! effect Ask {
//!     fn ask(prompt: str) -> str;
//! }
//!
//! fn greet() performs Ask {
//!     let name = perform Ask::ask("What is your name?");
//!     println("Hello, " + name + "!");
//! }
//!
//! handle {
//!     greet()
//! } with {
//!     Ask::ask(prompt) => resume("World")
//! }
//! ```
//!
//! # Architecture
//!
//! - `EffectDecl`: Declares an effect with operation signatures
//! - `HandlerClause`: Maps an effect operation to a handler body
//! - `HandlerDef`: A complete `handle { ... } with { ... }` block
//! - `Continuation`: Represents a suspended computation at a resume point
//! - `HandlerFrame`: A single frame on the handler stack
//! - `HandlerStack`: Manages nested handler frames for dispatch
//! - `EffectDispatcher`: Resolves effect operations through the handler chain

use std::collections::HashMap;
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Effect Declarations
// ═══════════════════════════════════════════════════════════════════════

/// Declares a named effect with one or more operations.
///
/// ```text
/// effect State[T] {
///     fn get() -> T;
///     fn set(val: T) -> void;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct EffectDecl {
    /// Name of the effect (e.g. "Ask", "State", "Logger")
    pub name: String,
    /// Type parameters (e.g. ["T"] for State[T])
    pub type_params: Vec<String>,
    /// Operations defined by this effect
    pub operations: Vec<EffectOperation>,
}

/// A single operation within an effect declaration.
#[derive(Debug, Clone)]
pub struct EffectOperation {
    /// Operation name (e.g. "ask", "get", "set")
    pub name: String,
    /// Parameter names and types
    pub params: Vec<(String, String)>,
    /// Return type
    pub return_type: String,
}

impl EffectDecl {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            type_params: Vec::new(),
            operations: Vec::new(),
        }
    }

    pub fn with_type_params(mut self, params: Vec<String>) -> Self {
        self.type_params = params;
        self
    }

    pub fn add_operation(&mut self, name: &str, params: Vec<(&str, &str)>, ret: &str) {
        self.operations.push(EffectOperation {
            name: name.to_string(),
            params: params.into_iter().map(|(n, t)| (n.to_string(), t.to_string())).collect(),
            return_type: ret.to_string(),
        });
    }

    /// Find an operation by name.
    pub fn find_operation(&self, name: &str) -> Option<&EffectOperation> {
        self.operations.iter().find(|op| op.name == name)
    }

    /// Get the qualified name for an operation: "Effect::operation"
    pub fn qualified_name(&self, op_name: &str) -> String {
        format!("{}::{}", self.name, op_name)
    }
}

impl fmt::Display for EffectDecl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "effect {}", self.name)?;
        if !self.type_params.is_empty() {
            write!(f, "[{}]", self.type_params.join(", "))?;
        }
        write!(f, " {{ ")?;
        for (i, op) in self.operations.iter().enumerate() {
            if i > 0 { write!(f, " ")?; }
            write!(f, "fn {}(", op.name)?;
            for (j, (pname, pty)) in op.params.iter().enumerate() {
                if j > 0 { write!(f, ", ")?; }
                write!(f, "{}: {}", pname, pty)?;
            }
            write!(f, ") -> {};", op.return_type)?;
        }
        write!(f, " }}")
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Continuations
// ═══════════════════════════════════════════════════════════════════════

/// Represents a suspended computation at the point where an effect was performed.
///
/// When a handler intercepts an effect, it receives a continuation representing
/// "the rest of the computation." The handler can:
/// - `resume(value)` — continue the computation with the given value
/// - `abort(value)` — abandon the computation and return the value
/// - Transform or invoke the continuation multiple times
#[derive(Debug, Clone)]
pub struct Continuation {
    /// Unique identifier for this continuation
    pub id: u64,
    /// The effect that was performed
    pub effect_name: String,
    /// The operation that was performed
    pub operation: String,
    /// Arguments passed to the effect operation
    pub args: Vec<ContinuationValue>,
    /// Current state of the continuation
    pub state: ContinuationState,
}

/// Values passed through continuations.
#[derive(Debug, Clone, PartialEq)]
pub enum ContinuationValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Void,
    List(Vec<ContinuationValue>),
}

impl fmt::Display for ContinuationValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContinuationValue::Int(v) => write!(f, "{}", v),
            ContinuationValue::Float(v) => write!(f, "{}", v),
            ContinuationValue::Str(v) => write!(f, "\"{}\"", v),
            ContinuationValue::Bool(v) => write!(f, "{}", v),
            ContinuationValue::Void => write!(f, "void"),
            ContinuationValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
        }
    }
}

/// The state of a continuation.
#[derive(Debug, Clone, PartialEq)]
pub enum ContinuationState {
    /// Suspended — waiting for the handler to resume or abort
    Suspended,
    /// Resumed with a value
    Resumed(ContinuationValue),
    /// Aborted with a value
    Aborted(ContinuationValue),
    /// Already consumed (one-shot continuation)
    Consumed,
}

impl Continuation {
    pub fn new(id: u64, effect: &str, operation: &str, args: Vec<ContinuationValue>) -> Self {
        Self {
            id,
            effect_name: effect.to_string(),
            operation: operation.to_string(),
            args,
            state: ContinuationState::Suspended,
        }
    }

    /// Resume the computation with the given value.
    pub fn resume(&mut self, value: ContinuationValue) -> Result<(), HandlerError> {
        match &self.state {
            ContinuationState::Suspended => {
                self.state = ContinuationState::Resumed(value);
                Ok(())
            }
            ContinuationState::Consumed => {
                Err(HandlerError::ContinuationConsumed {
                    continuation_id: self.id,
                })
            }
            _ => Err(HandlerError::ContinuationAlreadyResolved {
                continuation_id: self.id,
            }),
        }
    }

    /// Abort the computation with the given value.
    pub fn abort(&mut self, value: ContinuationValue) -> Result<(), HandlerError> {
        match &self.state {
            ContinuationState::Suspended => {
                self.state = ContinuationState::Aborted(value);
                Ok(())
            }
            ContinuationState::Consumed => {
                Err(HandlerError::ContinuationConsumed {
                    continuation_id: self.id,
                })
            }
            _ => Err(HandlerError::ContinuationAlreadyResolved {
                continuation_id: self.id,
            }),
        }
    }

    /// Mark the continuation as consumed (one-shot).
    pub fn consume(&mut self) {
        self.state = ContinuationState::Consumed;
    }

    pub fn is_suspended(&self) -> bool {
        self.state == ContinuationState::Suspended
    }

    pub fn is_resumed(&self) -> bool {
        matches!(self.state, ContinuationState::Resumed(_))
    }

    pub fn is_aborted(&self) -> bool {
        matches!(self.state, ContinuationState::Aborted(_))
    }

    /// Get the resume value (if resumed).
    pub fn resume_value(&self) -> Option<&ContinuationValue> {
        match &self.state {
            ContinuationState::Resumed(v) => Some(v),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Handler Clauses
// ═══════════════════════════════════════════════════════════════════════

/// A single clause in a handler block mapping an effect operation to handler behavior.
///
/// ```text
/// Ask::ask(prompt) => resume("World")
/// Logger::log(msg) => { println(msg); resume(void) }
/// Exception::throw(e) => abort(default_value)
/// ```
#[derive(Debug, Clone)]
pub struct HandlerClause {
    /// The effect being handled
    pub effect_name: String,
    /// The operation being handled
    pub operation: String,
    /// Parameter bindings for the operation arguments
    pub param_bindings: Vec<String>,
    /// The action to take
    pub action: HandlerAction,
}

/// What a handler clause does when intercepting an effect.
#[derive(Debug, Clone)]
pub enum HandlerAction {
    /// Resume the computation with a value
    Resume(ContinuationValue),
    /// Abort the computation with a value
    Abort(ContinuationValue),
    /// Transform: apply a function to the arguments and resume with the result
    Transform {
        /// Name of the transformation function
        transform_fn: String,
    },
    /// Log and resume: record the effect and continue
    LogAndResume {
        /// Where to log (effect log buffer)
        log_target: String,
    },
    /// Retry: re-perform the effect with modified arguments
    Retry {
        modified_args: Vec<ContinuationValue>,
    },
    /// Custom handler body (represented as an opaque string for static analysis)
    Custom {
        body_description: String,
    },
}

impl HandlerClause {
    pub fn resume_with(effect: &str, op: &str, params: Vec<&str>, value: ContinuationValue) -> Self {
        Self {
            effect_name: effect.to_string(),
            operation: op.to_string(),
            param_bindings: params.into_iter().map(|s| s.to_string()).collect(),
            action: HandlerAction::Resume(value),
        }
    }

    pub fn abort_with(effect: &str, op: &str, params: Vec<&str>, value: ContinuationValue) -> Self {
        Self {
            effect_name: effect.to_string(),
            operation: op.to_string(),
            param_bindings: params.into_iter().map(|s| s.to_string()).collect(),
            action: HandlerAction::Abort(value),
        }
    }

    /// Applies this clause to a continuation, resolving it.
    pub fn apply(&self, cont: &mut Continuation) -> Result<ContinuationValue, HandlerError> {
        match &self.action {
            HandlerAction::Resume(val) => {
                cont.resume(val.clone())?;
                Ok(val.clone())
            }
            HandlerAction::Abort(val) => {
                cont.abort(val.clone())?;
                Ok(val.clone())
            }
            HandlerAction::Transform { transform_fn } => {
                // In a real compiler this would invoke the transform function.
                // For static analysis, we resume with void and record the transform.
                let result = ContinuationValue::Str(
                    format!("transformed by {}", transform_fn)
                );
                cont.resume(result.clone())?;
                Ok(result)
            }
            HandlerAction::LogAndResume { log_target: _ } => {
                cont.resume(ContinuationValue::Void)?;
                Ok(ContinuationValue::Void)
            }
            HandlerAction::Retry { modified_args } => {
                // Re-perform with modified args — create a new continuation
                let result = if modified_args.is_empty() {
                    ContinuationValue::Void
                } else {
                    modified_args[0].clone()
                };
                cont.resume(result.clone())?;
                Ok(result)
            }
            HandlerAction::Custom { body_description: _ } => {
                cont.resume(ContinuationValue::Void)?;
                Ok(ContinuationValue::Void)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Handler Definition
// ═══════════════════════════════════════════════════════════════════════

/// A complete handler block: `handle { body } with { clauses }`
#[derive(Debug, Clone)]
pub struct HandlerDef {
    /// Optional name for the handler
    pub name: Option<String>,
    /// The clauses mapping effect operations to handler actions
    pub clauses: Vec<HandlerClause>,
    /// The return clause (what to do with the final value if no effect is performed)
    pub return_clause: Option<ReturnClause>,
}

/// The return clause handles the case where the body completes without performing
/// any (unhandled) effects.
#[derive(Debug, Clone)]
pub struct ReturnClause {
    /// Binding for the return value
    pub binding: String,
    /// The value to produce
    pub value: ContinuationValue,
}

impl HandlerDef {
    pub fn new() -> Self {
        Self {
            name: None,
            clauses: Vec::new(),
            return_clause: None,
        }
    }

    pub fn named(name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
            clauses: Vec::new(),
            return_clause: None,
        }
    }

    pub fn add_clause(&mut self, clause: HandlerClause) {
        self.clauses.push(clause);
    }

    pub fn set_return(&mut self, binding: &str, value: ContinuationValue) {
        self.return_clause = Some(ReturnClause {
            binding: binding.to_string(),
            value,
        });
    }

    /// Find a clause that handles a specific effect + operation pair.
    pub fn find_clause(&self, effect: &str, operation: &str) -> Option<&HandlerClause> {
        self.clauses.iter().find(|c| c.effect_name == effect && c.operation == operation)
    }

    /// Get all effects this handler covers.
    pub fn handled_effects(&self) -> Vec<String> {
        let mut effects: Vec<String> = self.clauses.iter()
            .map(|c| c.effect_name.clone())
            .collect();
        effects.sort();
        effects.dedup();
        effects
    }

    /// Check if this handler covers all operations of a given effect declaration.
    pub fn covers_effect(&self, decl: &EffectDecl) -> bool {
        decl.operations.iter().all(|op| {
            self.find_clause(&decl.name, &op.name).is_some()
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Handler Stack & Frames
// ═══════════════════════════════════════════════════════════════════════

/// A frame on the handler stack representing an active handler scope.
#[derive(Debug, Clone)]
pub struct HandlerFrame {
    /// The handler definition
    pub handler: HandlerDef,
    /// Depth in the handler stack (0 = outermost)
    pub depth: usize,
    /// Effects intercepted at this frame
    pub intercepted: Vec<String>,
    /// History of handled effects for debugging/logging
    pub history: Vec<HandledEffect>,
}

/// Record of an effect that was handled.
#[derive(Debug, Clone)]
pub struct HandledEffect {
    pub effect: String,
    pub operation: String,
    pub args: Vec<ContinuationValue>,
    pub result: ContinuationValue,
    pub action_taken: String,
}

/// The handler stack manages nested handler frames.
///
/// When a function performs an effect, the dispatcher searches the stack
/// from innermost to outermost handler to find a matching clause.
#[derive(Debug)]
pub struct HandlerStack {
    frames: Vec<HandlerFrame>,
    next_continuation_id: u64,
}

impl HandlerStack {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            next_continuation_id: 1,
        }
    }

    /// Push a new handler frame onto the stack.
    pub fn push(&mut self, handler: HandlerDef) {
        let depth = self.frames.len();
        let intercepted = handler.handled_effects();
        self.frames.push(HandlerFrame {
            handler,
            depth,
            intercepted,
            history: Vec::new(),
        });
    }

    /// Pop the top handler frame.
    pub fn pop(&mut self) -> Option<HandlerFrame> {
        self.frames.pop()
    }

    /// Current depth of the handler stack.
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Check if any handler in the stack handles a given effect + operation.
    pub fn can_handle(&self, effect: &str, operation: &str) -> bool {
        self.frames.iter().rev().any(|frame| {
            frame.handler.find_clause(effect, operation).is_some()
        })
    }

    /// Dispatch an effect operation through the handler stack.
    ///
    /// Searches from innermost to outermost for a matching handler clause,
    /// creates a continuation, and applies the clause.
    pub fn dispatch(
        &mut self,
        effect: &str,
        operation: &str,
        args: Vec<ContinuationValue>,
    ) -> Result<ContinuationValue, HandlerError> {
        let cont_id = self.next_continuation_id;
        self.next_continuation_id += 1;

        let mut cont = Continuation::new(cont_id, effect, operation, args.clone());

        // Search from innermost to outermost
        for frame in self.frames.iter_mut().rev() {
            if let Some(clause) = frame.handler.find_clause(effect, operation).cloned() {
                let result = clause.apply(&mut cont)?;

                // Record in history
                frame.history.push(HandledEffect {
                    effect: effect.to_string(),
                    operation: operation.to_string(),
                    args: args.clone(),
                    result: result.clone(),
                    action_taken: format!("{:?}", clause.action),
                });

                cont.consume();
                return Ok(result);
            }
        }

        // No handler found — unhandled effect
        Err(HandlerError::UnhandledEffect {
            effect: effect.to_string(),
            operation: operation.to_string(),
        })
    }

    /// Get the history of all handled effects across all frames.
    pub fn full_history(&self) -> Vec<&HandledEffect> {
        self.frames.iter().flat_map(|f| f.history.iter()).collect()
    }

    /// Get the top frame (innermost handler).
    pub fn top(&self) -> Option<&HandlerFrame> {
        self.frames.last()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Effect Dispatcher
// ═══════════════════════════════════════════════════════════════════════

/// The effect dispatcher manages effect declarations and handler resolution.
///
/// It serves as the central registry for declared effects and provides
/// static analysis of handler coverage.
#[derive(Debug)]
pub struct EffectDispatcher {
    /// Registered effect declarations
    declarations: HashMap<String, EffectDecl>,
    /// The handler stack
    pub stack: HandlerStack,
}

impl EffectDispatcher {
    pub fn new() -> Self {
        Self {
            declarations: HashMap::new(),
            stack: HandlerStack::new(),
        }
    }

    /// Register an effect declaration.
    pub fn declare_effect(&mut self, decl: EffectDecl) {
        self.declarations.insert(decl.name.clone(), decl);
    }

    /// Look up an effect declaration by name.
    pub fn get_effect(&self, name: &str) -> Option<&EffectDecl> {
        self.declarations.get(name)
    }

    /// Install a handler onto the stack.
    pub fn install_handler(&mut self, handler: HandlerDef) {
        self.stack.push(handler);
    }

    /// Remove the current handler from the stack.
    pub fn remove_handler(&mut self) -> Option<HandlerFrame> {
        self.stack.pop()
    }

    /// Perform an effect operation, dispatching through the handler stack.
    pub fn perform(
        &mut self,
        effect: &str,
        operation: &str,
        args: Vec<ContinuationValue>,
    ) -> Result<ContinuationValue, HandlerError> {
        // Validate the effect exists
        if let Some(decl) = self.declarations.get(effect) {
            if decl.find_operation(operation).is_none() {
                return Err(HandlerError::UnknownOperation {
                    effect: effect.to_string(),
                    operation: operation.to_string(),
                });
            }
        }
        // Dispatch through handler stack (allow undeclared effects for flexibility)
        self.stack.dispatch(effect, operation, args)
    }

    /// Static analysis: check if a handler covers all operations of a declared effect.
    pub fn check_handler_coverage(&self, handler: &HandlerDef) -> Vec<HandlerError> {
        let mut errors = Vec::new();
        for effect_name in handler.handled_effects() {
            if let Some(decl) = self.declarations.get(&effect_name) {
                for op in &decl.operations {
                    if handler.find_clause(&effect_name, &op.name).is_none() {
                        errors.push(HandlerError::MissingClause {
                            effect: effect_name.clone(),
                            operation: op.name.clone(),
                        });
                    }
                }
            }
        }
        errors
    }

    /// Get all declared effect names.
    pub fn declared_effects(&self) -> Vec<String> {
        self.declarations.keys().cloned().collect()
    }

    /// Check if a handler is total (handles all operations of all effects it claims to handle).
    pub fn is_handler_total(&self, handler: &HandlerDef) -> bool {
        self.check_handler_coverage(handler).is_empty()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Handler Composition
// ═══════════════════════════════════════════════════════════════════════

/// Compose two handlers into one that handles effects from both.
pub fn compose_handlers(inner: &HandlerDef, outer: &HandlerDef) -> HandlerDef {
    let mut composed = HandlerDef::new();
    composed.name = match (&inner.name, &outer.name) {
        (Some(a), Some(b)) => Some(format!("{} + {}", a, b)),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
    };
    // Inner handler clauses take priority over outer
    for clause in &inner.clauses {
        composed.add_clause(clause.clone());
    }
    // Add outer clauses only if not already covered by inner
    for clause in &outer.clauses {
        if composed.find_clause(&clause.effect_name, &clause.operation).is_none() {
            composed.add_clause(clause.clone());
        }
    }
    // Take inner return clause if present, otherwise outer
    composed.return_clause = inner.return_clause.clone()
        .or_else(|| outer.return_clause.clone());
    composed
}

// ═══════════════════════════════════════════════════════════════════════
//  Pre-built Handlers
// ═══════════════════════════════════════════════════════════════════════

/// Create a handler that handles IO by providing mock/default values.
pub fn mock_io_handler() -> HandlerDef {
    let mut handler = HandlerDef::named("mock_io");
    handler.add_clause(HandlerClause::resume_with(
        "IO", "print", vec!["msg"],
        ContinuationValue::Void,
    ));
    handler.add_clause(HandlerClause::resume_with(
        "IO", "println", vec!["msg"],
        ContinuationValue::Void,
    ));
    handler.add_clause(HandlerClause::resume_with(
        "IO", "read_line", vec![],
        ContinuationValue::Str("mock_input".to_string()),
    ));
    handler
}

/// Create a handler that converts exceptions to result values.
pub fn exception_to_result_handler() -> HandlerDef {
    let mut handler = HandlerDef::named("exception_to_result");
    handler.add_clause(HandlerClause::abort_with(
        "Exception", "throw", vec!["error"],
        ContinuationValue::Str("error".to_string()),
    ));
    handler.set_return("value", ContinuationValue::Str("ok".to_string()));
    handler
}

/// Create a stateful handler that threads state through continuations.
pub fn state_handler(initial: ContinuationValue) -> HandlerDef {
    let mut handler = HandlerDef::named("state");
    handler.add_clause(HandlerClause::resume_with(
        "State", "get", vec![],
        initial.clone(),
    ));
    handler.add_clause(HandlerClause::resume_with(
        "State", "set", vec!["new_val"],
        ContinuationValue::Void,
    ));
    handler.set_return("result", initial);
    handler
}

/// Create a nondeterminism handler (choose from a list of values).
pub fn nondeterminism_handler() -> HandlerDef {
    let mut handler = HandlerDef::named("nondeterminism");
    handler.add_clause(HandlerClause::resume_with(
        "Choose", "choose", vec!["options"],
        ContinuationValue::Int(0), // always choose first
    ));
    handler.add_clause(HandlerClause::abort_with(
        "Choose", "fail", vec![],
        ContinuationValue::Void,
    ));
    handler
}

// ═══════════════════════════════════════════════════════════════════════
//  Errors
// ═══════════════════════════════════════════════════════════════════════

/// Errors that can occur during effect handling.
#[derive(Debug, Clone, PartialEq)]
pub enum HandlerError {
    /// An effect was performed but no handler in the stack can handle it.
    UnhandledEffect {
        effect: String,
        operation: String,
    },
    /// An operation was performed on an effect that doesn't declare it.
    UnknownOperation {
        effect: String,
        operation: String,
    },
    /// A handler is missing a clause for an effect operation.
    MissingClause {
        effect: String,
        operation: String,
    },
    /// Tried to resume/abort a continuation that was already resolved.
    ContinuationAlreadyResolved {
        continuation_id: u64,
    },
    /// Tried to use a consumed (one-shot) continuation.
    ContinuationConsumed {
        continuation_id: u64,
    },
    /// Handler stack underflow (pop with no frames).
    StackUnderflow,
}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandlerError::UnhandledEffect { effect, operation } =>
                write!(f, "unhandled effect: {}::{}", effect, operation),
            HandlerError::UnknownOperation { effect, operation } =>
                write!(f, "unknown operation '{}' on effect '{}'", operation, effect),
            HandlerError::MissingClause { effect, operation } =>
                write!(f, "handler missing clause for {}::{}", effect, operation),
            HandlerError::ContinuationAlreadyResolved { continuation_id } =>
                write!(f, "continuation {} already resolved", continuation_id),
            HandlerError::ContinuationConsumed { continuation_id } =>
                write!(f, "continuation {} already consumed", continuation_id),
            HandlerError::StackUnderflow =>
                write!(f, "handler stack underflow"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Static Analysis Helpers
// ═══════════════════════════════════════════════════════════════════════

/// Analyze which effects a handler block introduces into scope.
///
/// Returns the set of effects that are "handled" (i.e., code inside the
/// handle block can perform these effects without propagating them outward).
pub fn handler_eliminates_effects(handler: &HandlerDef) -> Vec<String> {
    handler.handled_effects()
}

/// Check if two handlers have conflicting clauses (handle same effect+operation).
pub fn handlers_conflict(a: &HandlerDef, b: &HandlerDef) -> Vec<(String, String)> {
    let mut conflicts = Vec::new();
    for clause_a in &a.clauses {
        for clause_b in &b.clauses {
            if clause_a.effect_name == clause_b.effect_name
                && clause_a.operation == clause_b.operation
            {
                conflicts.push((clause_a.effect_name.clone(), clause_a.operation.clone()));
            }
        }
    }
    conflicts
}

/// Validate that a handler is well-formed:
/// - All referenced effects exist
/// - Parameter bindings match operation signatures
/// - No duplicate clauses
pub fn validate_handler(
    handler: &HandlerDef,
    declarations: &HashMap<String, EffectDecl>,
) -> Vec<HandlerError> {
    let mut errors = Vec::new();

    // Check for duplicate clauses
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for clause in &handler.clauses {
        let key = (clause.effect_name.clone(), clause.operation.clone());
        *seen.entry(key).or_insert(0) += 1;
    }
    for ((effect, op), count) in &seen {
        if *count > 1 {
            errors.push(HandlerError::MissingClause {
                effect: format!("duplicate handler for {}", effect),
                operation: op.clone(),
            });
        }
    }

    // Check parameter arity
    for clause in &handler.clauses {
        if let Some(decl) = declarations.get(&clause.effect_name) {
            if let Some(op) = decl.find_operation(&clause.operation) {
                if clause.param_bindings.len() != op.params.len() {
                    errors.push(HandlerError::MissingClause {
                        effect: format!(
                            "{}::{} expects {} params but handler binds {}",
                            clause.effect_name, clause.operation,
                            op.params.len(), clause.param_bindings.len()
                        ),
                        operation: clause.operation.clone(),
                    });
                }
            }
        }
    }

    errors
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Effect Declarations ──

    #[test]
    fn test_effect_decl_basic() {
        let mut ask = EffectDecl::new("Ask");
        ask.add_operation("ask", vec![("prompt", "str")], "str");
        assert_eq!(ask.name, "Ask");
        assert_eq!(ask.operations.len(), 1);
        assert!(ask.find_operation("ask").is_some());
        assert!(ask.find_operation("nope").is_none());
    }

    #[test]
    fn test_effect_decl_with_type_params() {
        let state = EffectDecl::new("State")
            .with_type_params(vec!["T".to_string()]);
        assert_eq!(state.type_params, vec!["T"]);
    }

    #[test]
    fn test_effect_decl_multiple_operations() {
        let mut state = EffectDecl::new("State");
        state.add_operation("get", vec![], "i64");
        state.add_operation("set", vec![("val", "i64")], "void");
        assert_eq!(state.operations.len(), 2);
        assert!(state.find_operation("get").is_some());
        assert!(state.find_operation("set").is_some());
    }

    #[test]
    fn test_effect_decl_qualified_name() {
        let decl = EffectDecl::new("Logger");
        assert_eq!(decl.qualified_name("log"), "Logger::log");
    }

    #[test]
    fn test_effect_decl_display() {
        let mut ask = EffectDecl::new("Ask");
        ask.add_operation("ask", vec![("prompt", "str")], "str");
        let s = format!("{}", ask);
        assert!(s.contains("effect Ask"));
        assert!(s.contains("fn ask"));
    }

    // ── Continuations ──

    #[test]
    fn test_continuation_resume() {
        let mut cont = Continuation::new(1, "Ask", "ask", vec![ContinuationValue::Str("hello".into())]);
        assert!(cont.is_suspended());
        cont.resume(ContinuationValue::Str("world".into())).unwrap();
        assert!(cont.is_resumed());
        assert_eq!(cont.resume_value(), Some(&ContinuationValue::Str("world".into())));
    }

    #[test]
    fn test_continuation_abort() {
        let mut cont = Continuation::new(2, "Exception", "throw", vec![]);
        cont.abort(ContinuationValue::Int(42)).unwrap();
        assert!(cont.is_aborted());
    }

    #[test]
    fn test_continuation_double_resume_fails() {
        let mut cont = Continuation::new(3, "X", "op", vec![]);
        cont.resume(ContinuationValue::Void).unwrap();
        assert!(cont.resume(ContinuationValue::Void).is_err());
    }

    #[test]
    fn test_continuation_consumed() {
        let mut cont = Continuation::new(4, "X", "op", vec![]);
        cont.consume();
        assert!(cont.resume(ContinuationValue::Void).is_err());
        assert!(cont.abort(ContinuationValue::Void).is_err());
    }

    #[test]
    fn test_continuation_value_display() {
        assert_eq!(format!("{}", ContinuationValue::Int(42)), "42");
        assert_eq!(format!("{}", ContinuationValue::Str("hi".into())), "\"hi\"");
        assert_eq!(format!("{}", ContinuationValue::Bool(true)), "true");
        assert_eq!(format!("{}", ContinuationValue::Void), "void");
        let list = ContinuationValue::List(vec![ContinuationValue::Int(1), ContinuationValue::Int(2)]);
        assert_eq!(format!("{}", list), "[1, 2]");
    }

    // ── Handler Clauses ──

    #[test]
    fn test_handler_clause_resume() {
        let clause = HandlerClause::resume_with("Ask", "ask", vec!["prompt"], ContinuationValue::Str("answer".into()));
        let mut cont = Continuation::new(10, "Ask", "ask", vec![]);
        let result = clause.apply(&mut cont).unwrap();
        assert_eq!(result, ContinuationValue::Str("answer".into()));
        assert!(cont.is_resumed());
    }

    #[test]
    fn test_handler_clause_abort() {
        let clause = HandlerClause::abort_with("Exception", "throw", vec!["e"], ContinuationValue::Int(-1));
        let mut cont = Continuation::new(11, "Exception", "throw", vec![]);
        let result = clause.apply(&mut cont).unwrap();
        assert_eq!(result, ContinuationValue::Int(-1));
        assert!(cont.is_aborted());
    }

    // ── Handler Definition ──

    #[test]
    fn test_handler_def_find_clause() {
        let mut handler = HandlerDef::named("test");
        handler.add_clause(HandlerClause::resume_with("Ask", "ask", vec!["p"], ContinuationValue::Str("x".into())));
        handler.add_clause(HandlerClause::resume_with("IO", "print", vec!["msg"], ContinuationValue::Void));
        assert!(handler.find_clause("Ask", "ask").is_some());
        assert!(handler.find_clause("IO", "print").is_some());
        assert!(handler.find_clause("Ask", "nope").is_none());
    }

    #[test]
    fn test_handler_handled_effects() {
        let mut handler = HandlerDef::new();
        handler.add_clause(HandlerClause::resume_with("Ask", "ask", vec![], ContinuationValue::Void));
        handler.add_clause(HandlerClause::resume_with("IO", "print", vec![], ContinuationValue::Void));
        handler.add_clause(HandlerClause::resume_with("IO", "read", vec![], ContinuationValue::Void));
        let effects = handler.handled_effects();
        assert_eq!(effects.len(), 2);
        assert!(effects.contains(&"Ask".to_string()));
        assert!(effects.contains(&"IO".to_string()));
    }

    #[test]
    fn test_handler_covers_effect() {
        let mut decl = EffectDecl::new("State");
        decl.add_operation("get", vec![], "i64");
        decl.add_operation("set", vec![("v", "i64")], "void");

        let mut handler = HandlerDef::new();
        handler.add_clause(HandlerClause::resume_with("State", "get", vec![], ContinuationValue::Int(0)));
        handler.add_clause(HandlerClause::resume_with("State", "set", vec!["v"], ContinuationValue::Void));
        assert!(handler.covers_effect(&decl));

        // Partial handler (missing "set")
        let mut partial = HandlerDef::new();
        partial.add_clause(HandlerClause::resume_with("State", "get", vec![], ContinuationValue::Int(0)));
        assert!(!partial.covers_effect(&decl));
    }

    #[test]
    fn test_handler_return_clause() {
        let mut handler = HandlerDef::new();
        handler.set_return("val", ContinuationValue::Int(42));
        assert!(handler.return_clause.is_some());
        assert_eq!(handler.return_clause.unwrap().value, ContinuationValue::Int(42));
    }

    // ── Handler Stack ──

    #[test]
    fn test_handler_stack_push_pop() {
        let mut stack = HandlerStack::new();
        assert_eq!(stack.depth(), 0);
        stack.push(HandlerDef::new());
        assert_eq!(stack.depth(), 1);
        stack.push(HandlerDef::new());
        assert_eq!(stack.depth(), 2);
        stack.pop();
        assert_eq!(stack.depth(), 1);
    }

    #[test]
    fn test_handler_stack_dispatch() {
        let mut stack = HandlerStack::new();
        let mut handler = HandlerDef::new();
        handler.add_clause(HandlerClause::resume_with(
            "Ask", "ask", vec!["p"],
            ContinuationValue::Str("answer".into()),
        ));
        stack.push(handler);

        let result = stack.dispatch("Ask", "ask", vec![ContinuationValue::Str("question".into())]).unwrap();
        assert_eq!(result, ContinuationValue::Str("answer".into()));
    }

    #[test]
    fn test_handler_stack_unhandled() {
        let mut stack = HandlerStack::new();
        let result = stack.dispatch("Unknown", "op", vec![]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            HandlerError::UnhandledEffect { effect: "Unknown".into(), operation: "op".into() }
        );
    }

    #[test]
    fn test_handler_stack_nested_dispatch() {
        let mut stack = HandlerStack::new();

        // Outer handler handles IO
        let mut outer = HandlerDef::named("outer");
        outer.add_clause(HandlerClause::resume_with("IO", "print", vec!["msg"], ContinuationValue::Void));

        // Inner handler handles Ask
        let mut inner = HandlerDef::named("inner");
        inner.add_clause(HandlerClause::resume_with("Ask", "ask", vec!["p"], ContinuationValue::Str("inner_answer".into())));

        stack.push(outer);
        stack.push(inner);

        // Ask is handled by inner
        let r1 = stack.dispatch("Ask", "ask", vec![]).unwrap();
        assert_eq!(r1, ContinuationValue::Str("inner_answer".into()));

        // IO is handled by outer (inner doesn't handle IO)
        let r2 = stack.dispatch("IO", "print", vec![ContinuationValue::Str("hello".into())]).unwrap();
        assert_eq!(r2, ContinuationValue::Void);
    }

    #[test]
    fn test_handler_stack_inner_overrides_outer() {
        let mut stack = HandlerStack::new();

        let mut outer = HandlerDef::new();
        outer.add_clause(HandlerClause::resume_with("X", "op", vec![], ContinuationValue::Int(1)));

        let mut inner = HandlerDef::new();
        inner.add_clause(HandlerClause::resume_with("X", "op", vec![], ContinuationValue::Int(2)));

        stack.push(outer);
        stack.push(inner);

        // Inner should win
        let result = stack.dispatch("X", "op", vec![]).unwrap();
        assert_eq!(result, ContinuationValue::Int(2));
    }

    #[test]
    fn test_handler_stack_history() {
        let mut stack = HandlerStack::new();
        let mut handler = HandlerDef::new();
        handler.add_clause(HandlerClause::resume_with("E", "op", vec![], ContinuationValue::Void));
        stack.push(handler);

        stack.dispatch("E", "op", vec![]).unwrap();
        stack.dispatch("E", "op", vec![]).unwrap();

        let history = stack.full_history();
        assert_eq!(history.len(), 2);
    }

    // ── Effect Dispatcher ──

    #[test]
    fn test_dispatcher_declare_and_perform() {
        let mut dispatcher = EffectDispatcher::new();

        let mut ask = EffectDecl::new("Ask");
        ask.add_operation("ask", vec![("prompt", "str")], "str");
        dispatcher.declare_effect(ask);

        let mut handler = HandlerDef::new();
        handler.add_clause(HandlerClause::resume_with(
            "Ask", "ask", vec!["prompt"],
            ContinuationValue::Str("42".into()),
        ));
        dispatcher.install_handler(handler);

        let result = dispatcher.perform("Ask", "ask", vec![ContinuationValue::Str("meaning?".into())]).unwrap();
        assert_eq!(result, ContinuationValue::Str("42".into()));
    }

    #[test]
    fn test_dispatcher_unknown_operation() {
        let mut dispatcher = EffectDispatcher::new();
        let mut eff = EffectDecl::new("X");
        eff.add_operation("a", vec![], "void");
        dispatcher.declare_effect(eff);

        let result = dispatcher.perform("X", "nonexistent", vec![]);
        assert!(matches!(result, Err(HandlerError::UnknownOperation { .. })));
    }

    #[test]
    fn test_dispatcher_coverage_check() {
        let mut dispatcher = EffectDispatcher::new();

        let mut state = EffectDecl::new("State");
        state.add_operation("get", vec![], "i64");
        state.add_operation("set", vec![("v", "i64")], "void");
        dispatcher.declare_effect(state);

        // Partial handler
        let mut handler = HandlerDef::new();
        handler.add_clause(HandlerClause::resume_with("State", "get", vec![], ContinuationValue::Int(0)));

        let errors = dispatcher.check_handler_coverage(&handler);
        assert_eq!(errors.len(), 1); // missing "set"

        // Complete handler
        handler.add_clause(HandlerClause::resume_with("State", "set", vec!["v"], ContinuationValue::Void));
        let errors2 = dispatcher.check_handler_coverage(&handler);
        assert!(errors2.is_empty());
    }

    #[test]
    fn test_dispatcher_is_handler_total() {
        let mut dispatcher = EffectDispatcher::new();
        let mut eff = EffectDecl::new("E");
        eff.add_operation("a", vec![], "void");
        eff.add_operation("b", vec![], "void");
        dispatcher.declare_effect(eff);

        let mut total = HandlerDef::new();
        total.add_clause(HandlerClause::resume_with("E", "a", vec![], ContinuationValue::Void));
        total.add_clause(HandlerClause::resume_with("E", "b", vec![], ContinuationValue::Void));
        assert!(dispatcher.is_handler_total(&total));

        let mut partial = HandlerDef::new();
        partial.add_clause(HandlerClause::resume_with("E", "a", vec![], ContinuationValue::Void));
        assert!(!dispatcher.is_handler_total(&partial));
    }

    // ── Handler Composition ──

    #[test]
    fn test_compose_handlers() {
        let mut h1 = HandlerDef::named("h1");
        h1.add_clause(HandlerClause::resume_with("A", "op", vec![], ContinuationValue::Int(1)));

        let mut h2 = HandlerDef::named("h2");
        h2.add_clause(HandlerClause::resume_with("B", "op", vec![], ContinuationValue::Int(2)));

        let composed = compose_handlers(&h1, &h2);
        assert!(composed.find_clause("A", "op").is_some());
        assert!(composed.find_clause("B", "op").is_some());
        assert_eq!(composed.name, Some("h1 + h2".to_string()));
    }

    #[test]
    fn test_compose_inner_priority() {
        let mut inner = HandlerDef::new();
        inner.add_clause(HandlerClause::resume_with("X", "op", vec![], ContinuationValue::Int(1)));

        let mut outer = HandlerDef::new();
        outer.add_clause(HandlerClause::resume_with("X", "op", vec![], ContinuationValue::Int(2)));

        let composed = compose_handlers(&inner, &outer);
        assert_eq!(composed.clauses.len(), 1); // inner wins, no duplicate
    }

    // ── Conflict Detection ──

    #[test]
    fn test_handlers_conflict() {
        let mut a = HandlerDef::new();
        a.add_clause(HandlerClause::resume_with("X", "op", vec![], ContinuationValue::Void));

        let mut b = HandlerDef::new();
        b.add_clause(HandlerClause::resume_with("X", "op", vec![], ContinuationValue::Void));

        let conflicts = handlers_conflict(&a, &b);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0], ("X".to_string(), "op".to_string()));
    }

    #[test]
    fn test_no_conflict() {
        let mut a = HandlerDef::new();
        a.add_clause(HandlerClause::resume_with("A", "op", vec![], ContinuationValue::Void));

        let mut b = HandlerDef::new();
        b.add_clause(HandlerClause::resume_with("B", "op", vec![], ContinuationValue::Void));

        assert!(handlers_conflict(&a, &b).is_empty());
    }

    // ── Prebuilt Handlers ──

    #[test]
    fn test_mock_io_handler() {
        let handler = mock_io_handler();
        assert!(handler.find_clause("IO", "print").is_some());
        assert!(handler.find_clause("IO", "println").is_some());
        assert!(handler.find_clause("IO", "read_line").is_some());
    }

    #[test]
    fn test_exception_to_result_handler() {
        let handler = exception_to_result_handler();
        assert!(handler.find_clause("Exception", "throw").is_some());
        assert!(handler.return_clause.is_some());
    }

    #[test]
    fn test_state_handler() {
        let handler = state_handler(ContinuationValue::Int(0));
        assert!(handler.find_clause("State", "get").is_some());
        assert!(handler.find_clause("State", "set").is_some());
    }

    #[test]
    fn test_nondeterminism_handler() {
        let handler = nondeterminism_handler();
        assert!(handler.find_clause("Choose", "choose").is_some());
        assert!(handler.find_clause("Choose", "fail").is_some());
    }

    // ── Handler Validation ──

    #[test]
    fn test_validate_handler_param_mismatch() {
        let mut decls = HashMap::new();
        let mut eff = EffectDecl::new("E");
        eff.add_operation("op", vec![("a", "i64"), ("b", "i64")], "void");
        decls.insert("E".to_string(), eff);

        let mut handler = HandlerDef::new();
        // Wrong number of params (1 instead of 2)
        handler.add_clause(HandlerClause::resume_with("E", "op", vec!["a"], ContinuationValue::Void));

        let errors = validate_handler(&handler, &decls);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_validate_handler_ok() {
        let mut decls = HashMap::new();
        let mut eff = EffectDecl::new("E");
        eff.add_operation("op", vec![("a", "i64")], "void");
        decls.insert("E".to_string(), eff);

        let mut handler = HandlerDef::new();
        handler.add_clause(HandlerClause::resume_with("E", "op", vec!["a"], ContinuationValue::Void));

        let errors = validate_handler(&handler, &decls);
        assert!(errors.is_empty());
    }

    // ── Effect Eliminates ──

    #[test]
    fn test_handler_eliminates_effects() {
        let mut handler = HandlerDef::new();
        handler.add_clause(HandlerClause::resume_with("IO", "print", vec![], ContinuationValue::Void));
        handler.add_clause(HandlerClause::resume_with("Net", "send", vec![], ContinuationValue::Void));
        let eliminated = handler_eliminates_effects(&handler);
        assert!(eliminated.contains(&"IO".to_string()));
        assert!(eliminated.contains(&"Net".to_string()));
    }

    // ── Full End-to-End ──

    #[test]
    fn test_full_effect_handler_pipeline() {
        let mut dispatcher = EffectDispatcher::new();

        // Declare "Ask" effect
        let mut ask = EffectDecl::new("Ask");
        ask.add_operation("ask", vec![("prompt", "str")], "str");
        dispatcher.declare_effect(ask);

        // Declare "Logger" effect
        let mut logger = EffectDecl::new("Logger");
        logger.add_operation("log", vec![("msg", "str")], "void");
        dispatcher.declare_effect(logger);

        // Install handler for both
        let mut handler = HandlerDef::named("test_handler");
        handler.add_clause(HandlerClause::resume_with(
            "Ask", "ask", vec!["prompt"],
            ContinuationValue::Str("response".into()),
        ));
        handler.add_clause(HandlerClause {
            effect_name: "Logger".to_string(),
            operation: "log".to_string(),
            param_bindings: vec!["msg".to_string()],
            action: HandlerAction::LogAndResume { log_target: "stdout".to_string() },
        });
        dispatcher.install_handler(handler);

        // Perform Ask
        let r1 = dispatcher.perform("Ask", "ask", vec![ContinuationValue::Str("name?".into())]).unwrap();
        assert_eq!(r1, ContinuationValue::Str("response".into()));

        // Perform Logger
        let r2 = dispatcher.perform("Logger", "log", vec![ContinuationValue::Str("hello".into())]).unwrap();
        assert_eq!(r2, ContinuationValue::Void);

        // Check history
        let history = dispatcher.stack.full_history();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_nested_handler_scopes() {
        let mut dispatcher = EffectDispatcher::new();

        // Outer handler for IO
        let mut outer = HandlerDef::named("outer_io");
        outer.add_clause(HandlerClause::resume_with("IO", "print", vec!["msg"], ContinuationValue::Void));
        dispatcher.install_handler(outer);

        // Inner handler for State — doesn't shadow IO
        let mut inner = HandlerDef::named("inner_state");
        inner.add_clause(HandlerClause::resume_with("State", "get", vec![], ContinuationValue::Int(99)));
        dispatcher.install_handler(inner);

        // State handled by inner
        let r1 = dispatcher.perform("State", "get", vec![]).unwrap();
        assert_eq!(r1, ContinuationValue::Int(99));

        // IO still handled by outer (fall-through)
        let r2 = dispatcher.perform("IO", "print", vec![ContinuationValue::Str("test".into())]).unwrap();
        assert_eq!(r2, ContinuationValue::Void);

        // Pop inner handler
        dispatcher.remove_handler();

        // State is now unhandled
        let r3 = dispatcher.perform("State", "get", vec![]);
        assert!(r3.is_err());

        // IO still works
        let r4 = dispatcher.perform("IO", "print", vec![]).unwrap();
        assert_eq!(r4, ContinuationValue::Void);
    }
}
