//! Vitalis Effect System & Capability Types (v22 Roadmap)
//!
//! Provides a static effect system that tracks computational effects (IO, Net,
//! FileSystem, Async, Unsafe, GPU, Evolve) and enforces capability-based security
//! at compile time.
//!
//! # Design
//!
//! Every function declares the effects it may perform via `performs` clauses.
//! The effect checker verifies that:
//! 1. Callers have sufficient capabilities to invoke effectful functions.
//! 2. Effect sets are propagated correctly through call chains.
//! 3. Pure functions (no effects) remain deterministic.
//! 4. Capability tokens can be attenuated but not forged.
//!
//! Effects are erased at the IR level — they are purely a compile-time safety mechanism.
//!
//! # Example (Vitalis syntax)
//!
//! ```text
//! fn read_file(path: str) performs IO, FileSystem -> str { ... }
//! fn serve() performs IO, Net { ... }
//! fn pure_add(a: i64, b: i64) -> i64 { a + b }  // no effects — pure
//! ```

use std::collections::{HashMap, HashSet, BTreeSet};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Effect Definitions
// ═══════════════════════════════════════════════════════════════════════

/// A computational effect that a function may perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Effect {
    /// Console / terminal input-output
    IO,
    /// Network access (TCP, UDP, HTTP)
    Net,
    /// File system read/write
    FileSystem,
    /// Async operations (spawn, await, channels)
    Async,
    /// Unsafe memory operations (raw pointers, FFI)
    Unsafe,
    /// GPU compute dispatch
    GPU,
    /// Code evolution / self-modification
    Evolve,
    /// System calls (exec, env, signals)
    System,
    /// Non-determinism (randomness, time)
    NonDet,
    /// Allocation (heap allocation beyond stack)
    Alloc,
    /// Exception / panic
    Exception,
    /// Custom user-defined effect
    Custom,
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effect::IO => write!(f, "IO"),
            Effect::Net => write!(f, "Net"),
            Effect::FileSystem => write!(f, "FileSystem"),
            Effect::Async => write!(f, "Async"),
            Effect::Unsafe => write!(f, "Unsafe"),
            Effect::GPU => write!(f, "GPU"),
            Effect::Evolve => write!(f, "Evolve"),
            Effect::System => write!(f, "System"),
            Effect::NonDet => write!(f, "NonDet"),
            Effect::Alloc => write!(f, "Alloc"),
            Effect::Exception => write!(f, "Exception"),
            Effect::Custom => write!(f, "Custom"),
        }
    }
}

impl Effect {
    /// Parse an effect name from a string.
    pub fn from_str(s: &str) -> Option<Effect> {
        match s.to_lowercase().as_str() {
            "io" => Some(Effect::IO),
            "net" | "network" => Some(Effect::Net),
            "fs" | "filesystem" => Some(Effect::FileSystem),
            "async" => Some(Effect::Async),
            "unsafe" => Some(Effect::Unsafe),
            "gpu" => Some(Effect::GPU),
            "evolve" | "evolution" => Some(Effect::Evolve),
            "system" | "sys" => Some(Effect::System),
            "nondet" | "nondeterminism" | "random" => Some(Effect::NonDet),
            "alloc" | "allocate" => Some(Effect::Alloc),
            "exception" | "panic" => Some(Effect::Exception),
            "custom" => Some(Effect::Custom),
            _ => None,
        }
    }

    /// All built-in effects.
    pub fn all() -> &'static [Effect] {
        &[
            Effect::IO, Effect::Net, Effect::FileSystem,
            Effect::Async, Effect::Unsafe, Effect::GPU,
            Effect::Evolve, Effect::System, Effect::NonDet,
            Effect::Alloc, Effect::Exception, Effect::Custom,
        ]
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Effect Set
// ═══════════════════════════════════════════════════════════════════════

/// An ordered set of effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectSet {
    effects: BTreeSet<Effect>,
    /// Custom effect names (for user-defined effects)
    custom_names: BTreeSet<String>,
}

impl EffectSet {
    /// Empty effect set (pure function).
    pub fn pure() -> Self {
        Self {
            effects: BTreeSet::new(),
            custom_names: BTreeSet::new(),
        }
    }

    /// Create from a slice of effects.
    pub fn from_effects(effects: &[Effect]) -> Self {
        Self {
            effects: effects.iter().copied().collect(),
            custom_names: BTreeSet::new(),
        }
    }

    /// Create from capability strings (as stored in AST Function.capabilities).
    pub fn from_capabilities(caps: &[String]) -> Self {
        let mut set = Self::pure();
        for cap in caps {
            if let Some(effect) = Effect::from_str(cap) {
                set.effects.insert(effect);
            } else if !cap.starts_with('\'') {
                // Skip lifetime annotations (start with ')
                set.custom_names.insert(cap.clone());
                set.effects.insert(Effect::Custom);
            }
        }
        set
    }

    /// Check if this is a pure (no effects) set.
    pub fn is_pure(&self) -> bool {
        self.effects.is_empty()
    }

    /// Add an effect.
    pub fn add(&mut self, effect: Effect) {
        self.effects.insert(effect);
    }

    /// Add a custom named effect.
    pub fn add_custom(&mut self, name: &str) {
        self.custom_names.insert(name.to_string());
        self.effects.insert(Effect::Custom);
    }

    /// Check if an effect is present.
    pub fn has(&self, effect: Effect) -> bool {
        self.effects.contains(&effect)
    }

    /// Check if this set is a subset of another (i.e., all our effects are allowed).
    pub fn is_subset_of(&self, other: &EffectSet) -> bool {
        self.effects.is_subset(&other.effects) &&
        self.custom_names.is_subset(&other.custom_names)
    }

    /// Union of two effect sets.
    pub fn union(&self, other: &EffectSet) -> EffectSet {
        EffectSet {
            effects: self.effects.union(&other.effects).copied().collect(),
            custom_names: self.custom_names.union(&other.custom_names).cloned().collect(),
        }
    }

    /// Difference: effects in self but not in other.
    pub fn difference(&self, other: &EffectSet) -> EffectSet {
        EffectSet {
            effects: self.effects.difference(&other.effects).copied().collect(),
            custom_names: self.custom_names.difference(&other.custom_names).cloned().collect(),
        }
    }

    /// Get all effects as a slice.
    pub fn effects(&self) -> Vec<Effect> {
        self.effects.iter().copied().collect()
    }

    /// Get custom names.
    pub fn custom_names(&self) -> &BTreeSet<String> {
        &self.custom_names
    }

    /// Number of effects.
    pub fn len(&self) -> usize {
        self.effects.len()
    }
}

impl fmt::Display for EffectSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_pure() {
            return write!(f, "pure");
        }
        let names: Vec<String> = self.effects.iter().map(|e| e.to_string()).collect();
        write!(f, "{}", names.join(", "))
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Capability Token
// ═══════════════════════════════════════════════════════════════════════

/// A capability token grants permission to perform a set of effects.
/// Capabilities can be attenuated (reduced) but never amplified (forged).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityToken {
    pub name: String,
    pub granted_effects: EffectSet,
    /// Whether this capability can be further delegated.
    pub delegatable: bool,
    /// Trust tier: 0 = untrusted, 1 = sandboxed, 2 = trusted, 3 = privileged
    pub trust_tier: u8,
}

impl CapabilityToken {
    /// Create a new capability token.
    pub fn new(name: &str, effects: EffectSet, trust_tier: u8) -> Self {
        Self {
            name: name.to_string(),
            granted_effects: effects,
            delegatable: true,
            trust_tier,
        }
    }

    /// Create an attenuated (reduced) capability token.
    pub fn attenuate(&self, restricted: &EffectSet) -> Option<CapabilityToken> {
        if !self.delegatable {
            return None;
        }
        // Can only remove effects, never add
        let new_effects = EffectSet {
            effects: self.granted_effects.effects.intersection(&restricted.effects).copied().collect(),
            custom_names: self.granted_effects.custom_names.intersection(&restricted.custom_names).cloned().collect(),
        };
        Some(CapabilityToken {
            name: format!("{}/attenuated", self.name),
            granted_effects: new_effects,
            delegatable: self.delegatable,
            trust_tier: self.trust_tier.saturating_sub(1),
        })
    }

    /// Check if this token grants a specific effect.
    pub fn grants(&self, effect: Effect) -> bool {
        self.granted_effects.has(effect)
    }

    /// Full capability (kernel-level, all effects).
    pub fn full() -> Self {
        Self {
            name: "FULL".to_string(),
            granted_effects: EffectSet::from_effects(Effect::all()),
            delegatable: true,
            trust_tier: 3,
        }
    }

    /// Sandboxed capability (only IO and Alloc).
    pub fn sandboxed() -> Self {
        Self {
            name: "SANDBOX".to_string(),
            granted_effects: EffectSet::from_effects(&[Effect::IO, Effect::Alloc]),
            delegatable: false,
            trust_tier: 1,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Effect Errors
// ═══════════════════════════════════════════════════════════════════════

/// An error detected during effect checking.
#[derive(Debug, Clone)]
pub struct EffectError {
    pub kind: EffectErrorKind,
    pub message: String,
    pub function: String,
    pub callee: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectErrorKind {
    /// Calling a function that requires effects the caller doesn't have.
    InsufficientCapability,
    /// A pure function is performing an effect.
    PurityViolation,
    /// An effect is declared but never used.
    UnusedEffect,
    /// Capability token cannot be delegated.
    NonDelegatable,
    /// Trust tier too low for the operation.
    InsufficientTrust,
}

impl fmt::Display for EffectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "effect error in '{}': {}", self.function, self.message)?;
        if let Some(callee) = &self.callee {
            write!(f, " (calling '{}')", callee)?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Effect Checker
// ═══════════════════════════════════════════════════════════════════════

use crate::ast::{Expr, Function, Program, Stmt, TopLevel, Block};

/// Static effect checker for Vitalis programs.
///
/// Ensures that effectful operations are only performed by functions that
/// declare the appropriate effects, and that callers propagate effects
/// correctly.
pub struct EffectChecker {
    /// Function name → declared effects
    function_effects: HashMap<String, EffectSet>,
    /// Built-in function effects (stdlib)
    builtin_effects: HashMap<String, EffectSet>,
    /// Collected errors
    errors: Vec<EffectError>,
    /// Current function being checked
    current_function: String,
}

impl EffectChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            function_effects: HashMap::new(),
            builtin_effects: HashMap::new(),
            errors: Vec::new(),
            current_function: String::new(),
        };
        checker.register_builtins();
        checker
    }

    fn register_builtins(&mut self) {
        // Print functions perform IO
        let io = EffectSet::from_effects(&[Effect::IO]);
        for name in &["println", "print", "eprintln", "eprint", "dbg"] {
            self.builtin_effects.insert(name.to_string(), io.clone());
        }

        // File operations
        let fs = EffectSet::from_effects(&[Effect::IO, Effect::FileSystem]);
        for name in &["read_file", "write_file", "file_exists", "delete_file", "list_dir"] {
            self.builtin_effects.insert(name.to_string(), fs.clone());
        }

        // Network
        let net = EffectSet::from_effects(&[Effect::IO, Effect::Net]);
        for name in &["http_get", "http_post", "tcp_connect", "tcp_listen"] {
            self.builtin_effects.insert(name.to_string(), net.clone());
        }

        // Random / time (non-deterministic)
        let nondet = EffectSet::from_effects(&[Effect::NonDet]);
        for name in &["random", "random_range", "time_now", "sleep"] {
            self.builtin_effects.insert(name.to_string(), nondet.clone());
        }

        // Pure math functions
        let pure = EffectSet::pure();
        for name in &["abs", "sqrt", "pow", "sin", "cos", "tan", "log", "exp",
                       "min", "max", "floor", "ceil", "round", "len", "push", "pop"] {
            self.builtin_effects.insert(name.to_string(), pure.clone());
        }
    }

    /// Check a full program for effect errors.
    pub fn check(&mut self, program: &Program) -> Vec<EffectError> {
        // Phase 1: Collect all function effect declarations
        for item in &program.items {
            self.collect_effects(item);
        }

        // Phase 2: Verify effect constraints
        for item in &program.items {
            self.check_item(item);
        }

        self.errors.clone()
    }

    fn collect_effects(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Function(func) => {
                let effects = EffectSet::from_capabilities(&func.capabilities);
                self.function_effects.insert(func.name.clone(), effects);
            }
            TopLevel::Impl(impl_block) => {
                for method in &impl_block.methods {
                    let qualified = format!("{}::{}", impl_block.type_name, method.name);
                    let effects = EffectSet::from_capabilities(&method.capabilities);
                    self.function_effects.insert(qualified, effects.clone());
                    self.function_effects.insert(method.name.clone(), effects);
                }
            }
            TopLevel::Annotated { item, .. } => {
                self.collect_effects(item);
            }
            _ => {}
        }
    }

    fn check_item(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Function(func) => {
                self.current_function = func.name.clone();
                self.check_block(&func.body);
            }
            TopLevel::Impl(impl_block) => {
                for method in &impl_block.methods {
                    self.current_function = format!("{}::{}", impl_block.type_name, method.name);
                    self.check_block(&method.body);
                }
            }
            TopLevel::Annotated { item, .. } => {
                self.check_item(item);
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        if let Some(expr) = &block.tail_expr {
            self.check_expr(expr);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { value: Some(val), .. } => self.check_expr(val),
            Stmt::Expr(expr) => self.check_expr(expr),
            _ => {}
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { func, args, .. } => {
                // Determine the callee name
                if let Expr::Ident(name, _) = func.as_ref() {
                    self.check_call(name);
                }
                // Recurse into arguments
                self.check_expr(func);
                for arg in args {
                    self.check_expr(arg);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.check_expr(operand);
            }
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.check_expr(condition);
                self.check_block(then_branch);
                if let Some(eb) = else_branch {
                    self.check_block(eb);
                }
            }
            Expr::Block(block) => {
                self.check_block(block);
            }
            Expr::Assign { target, value, .. } => {
                self.check_expr(target);
                self.check_expr(value);
            }
            Expr::Return { value, .. } => {
                if let Some(val) = value {
                    self.check_expr(val);
                }
            }
            _ => {}
        }
    }

    fn check_call(&mut self, callee: &str) {
        let caller_effects = self.function_effects
            .get(&self.current_function)
            .cloned()
            .unwrap_or_else(EffectSet::pure);

        // Check builtin effects
        if let Some(required) = self.builtin_effects.get(callee) {
            if !required.is_subset_of(&caller_effects) {
                let _missing = required.difference(&caller_effects);
                self.errors.push(EffectError {
                    kind: EffectErrorKind::InsufficientCapability,
                    message: format!(
                        "calling '{}' requires effects [{}] but '{}' only has [{}]",
                        callee, required, self.current_function, caller_effects
                    ),
                    function: self.current_function.clone(),
                    callee: Some(callee.to_string()),
                });
            }
            return;
        }

        // Check user-defined function effects
        if let Some(required) = self.function_effects.get(callee).cloned() {
            if !required.is_subset_of(&caller_effects) {
                self.errors.push(EffectError {
                    kind: EffectErrorKind::InsufficientCapability,
                    message: format!(
                        "calling '{}' requires effects [{}] but '{}' only has [{}]",
                        callee, required, self.current_function, caller_effects
                    ),
                    function: self.current_function.clone(),
                    callee: Some(callee.to_string()),
                });
            }
        }
    }

    /// Query the effects declared by a function.
    pub fn effects_of(&self, function: &str) -> EffectSet {
        // Check user-defined functions first, then builtins
        if let Some(effects) = self.function_effects.get(function) {
            return effects.clone();
        }
        if let Some(effects) = self.builtin_effects.get(function) {
            return effects.clone();
        }
        EffectSet::pure()
    }

    /// Check if a function is pure (no effects).
    pub fn is_pure(&self, function: &str) -> bool {
        self.effects_of(function).is_pure()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Effect Handler (for algebraic effects, future extension)
// ═══════════════════════════════════════════════════════════════════════

/// Represents an effect handler that intercepts effectful operations.
/// This is a foundation for algebraic effects support.
#[derive(Debug, Clone)]
pub struct EffectHandler {
    pub name: String,
    /// Effects this handler intercepts.
    pub handles: EffectSet,
    /// Whether the handler resumes the computation.
    pub resumable: bool,
}

impl EffectHandler {
    pub fn new(name: &str, handles: EffectSet) -> Self {
        Self {
            name: name.to_string(),
            handles,
            resumable: true,
        }
    }

    /// Create a handler that handles IO by logging.
    pub fn io_logger() -> Self {
        Self::new("io_logger", EffectSet::from_effects(&[Effect::IO]))
    }

    /// Create a handler that handles exceptions by converting to Results.
    pub fn exception_to_result() -> Self {
        Self::new("exception_to_result", EffectSet::from_effects(&[Effect::Exception]))
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_from_str() {
        assert_eq!(Effect::from_str("IO"), Some(Effect::IO));
        assert_eq!(Effect::from_str("net"), Some(Effect::Net));
        assert_eq!(Effect::from_str("filesystem"), Some(Effect::FileSystem));
        assert_eq!(Effect::from_str("async"), Some(Effect::Async));
        assert_eq!(Effect::from_str("unknown"), None);
    }

    #[test]
    fn test_effect_set_pure() {
        let set = EffectSet::pure();
        assert!(set.is_pure());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_effect_set_operations() {
        let a = EffectSet::from_effects(&[Effect::IO, Effect::Net]);
        let b = EffectSet::from_effects(&[Effect::IO, Effect::FileSystem]);

        // Union
        let union = a.union(&b);
        assert!(union.has(Effect::IO));
        assert!(union.has(Effect::Net));
        assert!(union.has(Effect::FileSystem));
        assert_eq!(union.len(), 3);

        // Subset
        assert!(!a.is_subset_of(&b)); // a has Net, b doesn't
        let io_only = EffectSet::from_effects(&[Effect::IO]);
        assert!(io_only.is_subset_of(&a));
        assert!(io_only.is_subset_of(&b));

        // Difference
        let diff = a.difference(&b);
        assert!(diff.has(Effect::Net));
        assert!(!diff.has(Effect::IO));
    }

    #[test]
    fn test_effect_set_from_capabilities() {
        let caps = vec![
            "IO".to_string(),
            "Net".to_string(),
            "'a".to_string(), // lifetime, not an effect
        ];
        let set = EffectSet::from_capabilities(&caps);
        assert!(set.has(Effect::IO));
        assert!(set.has(Effect::Net));
        assert_eq!(set.len(), 2); // lifetime should be excluded
    }

    #[test]
    fn test_capability_token() {
        let full = CapabilityToken::full();
        assert!(full.grants(Effect::IO));
        assert!(full.grants(Effect::GPU));
        assert_eq!(full.trust_tier, 3);

        let sandboxed = CapabilityToken::sandboxed();
        assert!(sandboxed.grants(Effect::IO));
        assert!(!sandboxed.grants(Effect::Net));
        assert!(!sandboxed.delegatable);
    }

    #[test]
    fn test_capability_attenuation() {
        let full = CapabilityToken::full();
        let restricted = EffectSet::from_effects(&[Effect::IO, Effect::Alloc]);
        let attenuated = full.attenuate(&restricted).unwrap();

        assert!(attenuated.grants(Effect::IO));
        assert!(attenuated.grants(Effect::Alloc));
        assert!(!attenuated.grants(Effect::Net));
        assert_eq!(attenuated.trust_tier, 2); // reduced by 1
    }

    #[test]
    fn test_non_delegatable_attenuation() {
        let sandboxed = CapabilityToken::sandboxed();
        let restricted = EffectSet::from_effects(&[Effect::IO]);
        assert!(sandboxed.attenuate(&restricted).is_none());
    }

    #[test]
    fn test_effect_handler() {
        let handler = EffectHandler::io_logger();
        assert!(handler.handles.has(Effect::IO));
        assert!(handler.resumable);

        let exc = EffectHandler::exception_to_result();
        assert!(exc.handles.has(Effect::Exception));
    }

    #[test]
    fn test_effect_display() {
        let set = EffectSet::from_effects(&[Effect::IO, Effect::Net]);
        let display = format!("{}", set);
        assert!(display.contains("IO"));
        assert!(display.contains("Net"));

        let pure = EffectSet::pure();
        assert_eq!(format!("{}", pure), "pure");
    }

    #[test]
    fn test_builtin_effects() {
        let checker = EffectChecker::new();
        // println is IO
        assert!(checker.effects_of("println").has(Effect::IO));
        // sqrt is pure
        assert!(checker.is_pure("sqrt"));
        // http_get is IO + Net
        let http_effects = checker.effects_of("http_get");
        assert!(http_effects.has(Effect::IO));
        assert!(http_effects.has(Effect::Net));
    }
}
