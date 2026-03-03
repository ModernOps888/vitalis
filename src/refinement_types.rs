//! Vitalis Refinement Types (v25)
//!
//! A refinement type system that augments base types with logical predicates.
//! Refinement types allow expressing invariants like "positive integers",
//! "non-empty strings", or "percentages between 0 and 100" directly in the
//! type system.
//!
//! # Architecture
//!
//! - **RefinedType**: A base type + binder variable + predicate
//! - **Predicate**: A logical formula over the binder (comparisons, arithmetic, boolean ops)
//! - **ConstraintSolver**: Checks satisfiability and entailment of predicates
//! - **RefinementRegistry**: Named refinement types (e.g., `Positive`, `NonZero`)
//!
//! # Examples
//!
//! ```text
//! // Vitalis source (future syntax):
//! fn divide(x: i64, y: { v: i64 | v != 0 }) -> i64 { x / y }
//! fn percentage(p: { v: f64 | 0.0 <= v && v <= 100.0 }) -> f64 { p }
//! ```

use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════
//  Core Types
// ═══════════════════════════════════════════════════════════════════════

/// Base types that can be refined with predicates.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseType {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Str,
    Void,
}

impl std::fmt::Display for BaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseType::I32 => write!(f, "i32"),
            BaseType::I64 => write!(f, "i64"),
            BaseType::F32 => write!(f, "f32"),
            BaseType::F64 => write!(f, "f64"),
            BaseType::Bool => write!(f, "bool"),
            BaseType::Str => write!(f, "str"),
            BaseType::Void => write!(f, "void"),
        }
    }
}

/// A refinement type: `{ binder : base | predicate }`.
#[derive(Debug, Clone)]
pub struct RefinedType {
    /// The underlying base type.
    pub base: BaseType,
    /// The binder variable name (e.g., "v").
    pub binder: String,
    /// The refinement predicate over the binder.
    pub predicate: Predicate,
}

impl RefinedType {
    /// Create a new refinement type.
    pub fn new(base: BaseType, binder: &str, predicate: Predicate) -> Self {
        Self {
            base,
            binder: binder.to_string(),
            predicate,
        }
    }

    /// Create an unrefined type (predicate is True).
    pub fn unrefined(base: BaseType) -> Self {
        Self {
            base,
            binder: "v".to_string(),
            predicate: Predicate::True,
        }
    }

    /// Check if this type is trivially unrefined.
    pub fn is_trivial(&self) -> bool {
        matches!(self.predicate, Predicate::True)
    }
}

impl std::fmt::Display for RefinedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_trivial() {
            write!(f, "{}", self.base)
        } else {
            write!(f, "{{ {}: {} | {} }}", self.binder, self.base, self.predicate)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Predicates
// ═══════════════════════════════════════════════════════════════════════

/// Comparison operators for refinement predicates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CmpOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

impl std::fmt::Display for CmpOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmpOp::Eq => write!(f, "=="),
            CmpOp::NotEq => write!(f, "!="),
            CmpOp::Lt => write!(f, "<"),
            CmpOp::LtEq => write!(f, "<="),
            CmpOp::Gt => write!(f, ">"),
            CmpOp::GtEq => write!(f, ">="),
        }
    }
}

/// Arithmetic operators in predicate expressions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl std::fmt::Display for ArithOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArithOp::Add => write!(f, "+"),
            ArithOp::Sub => write!(f, "-"),
            ArithOp::Mul => write!(f, "*"),
            ArithOp::Div => write!(f, "/"),
            ArithOp::Mod => write!(f, "%"),
        }
    }
}

/// A logical predicate for refinement types.
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    /// Always true — trivial refinement.
    True,
    /// Always false — empty / uninhabited type.
    False,
    /// A variable reference (typically the binder).
    Var(String),
    /// An integer constant.
    IntConst(i64),
    /// A float constant.
    FloatConst(f64),
    /// A boolean constant.
    BoolConst(bool),
    /// Comparison: `lhs op rhs`.
    Compare {
        op: CmpOp,
        lhs: Box<Predicate>,
        rhs: Box<Predicate>,
    },
    /// Logical AND: `lhs && rhs`.
    And(Box<Predicate>, Box<Predicate>),
    /// Logical OR: `lhs || rhs`.
    Or(Box<Predicate>, Box<Predicate>),
    /// Logical NOT: `!p`.
    Not(Box<Predicate>),
    /// Implication: `lhs => rhs`.
    Implies(Box<Predicate>, Box<Predicate>),
    /// Arithmetic expression: `lhs op rhs`.
    Arith {
        op: ArithOp,
        lhs: Box<Predicate>,
        rhs: Box<Predicate>,
    },
    /// Uninterpreted function application: `f(args...)`.
    App(String, Vec<Predicate>),
}

impl std::fmt::Display for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::True => write!(f, "true"),
            Predicate::False => write!(f, "false"),
            Predicate::Var(name) => write!(f, "{}", name),
            Predicate::IntConst(n) => write!(f, "{}", n),
            Predicate::FloatConst(fl) => write!(f, "{}", fl),
            Predicate::BoolConst(b) => write!(f, "{}", b),
            Predicate::Compare { op, lhs, rhs } => write!(f, "({} {} {})", lhs, op, rhs),
            Predicate::And(a, b) => write!(f, "({} && {})", a, b),
            Predicate::Or(a, b) => write!(f, "({} || {})", a, b),
            Predicate::Not(p) => write!(f, "(!{})", p),
            Predicate::Implies(a, b) => write!(f, "({} => {})", a, b),
            Predicate::Arith { op, lhs, rhs } => write!(f, "({} {} {})", lhs, op, rhs),
            Predicate::App(name, args) => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Constraint Solver
// ═══════════════════════════════════════════════════════════════════════

/// Errors from refinement type checking.
#[derive(Debug, Clone)]
pub struct RefinementError {
    pub message: String,
    pub span_start: usize,
    pub span_end: usize,
}

impl std::fmt::Display for RefinementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "refinement error at {}-{}: {}", self.span_start, self.span_end, self.message)
    }
}

/// Variable bounds for the constraint solver.
#[derive(Debug, Clone)]
struct VarBounds {
    lower: Option<f64>,
    upper: Option<f64>,
    not_equal: Vec<f64>,
}

impl VarBounds {
    fn new() -> Self {
        Self {
            lower: None,
            upper: None,
            not_equal: Vec::new(),
        }
    }

    fn with_lower(mut self, lo: f64) -> Self {
        self.lower = Some(match self.lower {
            Some(old) => old.max(lo),
            None => lo,
        });
        self
    }

    fn with_upper(mut self, hi: f64) -> Self {
        self.upper = Some(match self.upper {
            Some(old) => old.min(hi),
            None => hi,
        });
        self
    }

    fn with_not_equal(mut self, val: f64) -> Self {
        self.not_equal.push(val);
        self
    }

    fn is_satisfiable(&self) -> bool {
        match (self.lower, self.upper) {
            (Some(lo), Some(hi)) => {
                if lo > hi {
                    return false;
                }
                // Check that no not-equal value blocks the only viable value
                if (lo - hi).abs() < f64::EPSILON {
                    return !self.not_equal.iter().any(|v| (*v - lo).abs() < f64::EPSILON);
                }
                true
            }
            _ => true,
        }
    }
}

/// Constraint solver for refinement type predicates.
///
/// Uses a bounds-based approach: extracts variable bounds from predicates
/// and checks satisfiability / entailment.
pub struct ConstraintSolver {
    /// Variable bounds.
    bounds: HashMap<String, VarBounds>,
    /// Known facts (assumptions).
    assumptions: Vec<Predicate>,
}

impl ConstraintSolver {
    /// Create a new empty solver.
    pub fn new() -> Self {
        Self {
            bounds: HashMap::new(),
            assumptions: Vec::new(),
        }
    }

    /// Add an assumption (known fact).
    pub fn assume(&mut self, pred: Predicate) {
        self.extract_bounds(&pred);
        self.assumptions.push(pred);
    }

    /// Check if a predicate is satisfiable under current assumptions.
    pub fn is_satisfiable(&mut self, pred: &Predicate) -> bool {
        self.extract_bounds(pred);

        match pred {
            Predicate::True => true,
            Predicate::False => false,
            Predicate::BoolConst(b) => *b,

            Predicate::Compare { op, lhs, rhs } => {
                if let (Some(l), Some(r)) = (self.eval_const(lhs), self.eval_const(rhs)) {
                    match op {
                        CmpOp::Eq => (l - r).abs() < f64::EPSILON,
                        CmpOp::NotEq => (l - r).abs() >= f64::EPSILON,
                        CmpOp::Lt => l < r,
                        CmpOp::LtEq => l <= r,
                        CmpOp::Gt => l > r,
                        CmpOp::GtEq => l >= r,
                    }
                } else {
                    // Can't fully evaluate — check bounds
                    self.bounds_satisfiable()
                }
            }

            Predicate::And(a, b) => self.is_satisfiable(a) && self.is_satisfiable(b),
            Predicate::Or(a, b) => self.is_satisfiable(a) || self.is_satisfiable(b),
            Predicate::Not(p) => !self.is_satisfiable(p),

            Predicate::Implies(a, b) => !self.is_satisfiable(a) || self.is_satisfiable(b),

            _ => true, // Conservative: assume satisfiable if we can't decide
        }
    }

    /// Check if `assumption` entails `goal` (i.e., assumption => goal).
    pub fn entails(&mut self, assumption: &Predicate, goal: &Predicate) -> bool {
        // Simple structural check first
        if assumption == goal {
            return true;
        }
        if matches!(goal, Predicate::True) {
            return true;
        }
        if matches!(assumption, Predicate::False) {
            return true; // False entails anything
        }

        // Try bounds-based reasoning
        self.bounds.clear();
        self.extract_bounds(assumption);

        match goal {
            Predicate::Compare { op, lhs, rhs } => {
                if let (Some(l), Some(r)) = (self.eval_const_with_bounds(lhs), self.eval_const_with_bounds(rhs)) {
                    match op {
                        CmpOp::Eq => (l - r).abs() < f64::EPSILON,
                        CmpOp::NotEq => (l - r).abs() >= f64::EPSILON,
                        CmpOp::Lt => l < r,
                        CmpOp::LtEq => l <= r,
                        CmpOp::Gt => l > r,
                        CmpOp::GtEq => l >= r,
                    }
                } else {
                    // Try entailment via bounds
                    self.check_entailment_via_bounds(goal)
                }
            }
            Predicate::And(a, b) => self.entails(assumption, a) && self.entails(assumption, b),
            _ => false, // Conservative
        }
    }

    /// Check subtype relationship: refined_sub <: refined_super.
    pub fn is_subtype(&mut self, sub: &RefinedType, sup: &RefinedType) -> Result<(), RefinementError> {
        if sub.base != sup.base {
            return Err(RefinementError {
                message: format!(
                    "Base type mismatch: {} vs {}",
                    sub.base, sup.base
                ),
                span_start: 0,
                span_end: 0,
            });
        }

        if sup.is_trivial() {
            return Ok(());
        }

        // Substitute binder: sub's predicate[sub.binder/sup.binder]
        let sub_pred = if sub.binder != sup.binder {
            substitute(&sub.predicate, &sub.binder, &Predicate::Var(sup.binder.clone()))
        } else {
            sub.predicate.clone()
        };

        if self.entails(&sub_pred, &sup.predicate) {
            Ok(())
        } else {
            Err(RefinementError {
                message: format!(
                    "Cannot prove refinement: {} does not entail {}",
                    sub_pred, sup.predicate
                ),
                span_start: 0,
                span_end: 0,
            })
        }
    }

    // ─── Internal ────────────────────────────────────────────────────

    fn extract_bounds(&mut self, pred: &Predicate) {
        match pred {
            Predicate::Compare { op, lhs, rhs } => {
                // Pattern: var op const or const op var
                if let Predicate::Var(name) = lhs.as_ref() {
                    if let Some(val) = self.eval_const(rhs) {
                        let bounds = self
                            .bounds
                            .entry(name.clone())
                            .or_insert_with(VarBounds::new)
                            .clone();
                        let new_bounds = match op {
                            CmpOp::Gt => bounds.with_lower(val + 1.0),
                            CmpOp::GtEq => bounds.with_lower(val),
                            CmpOp::Lt => bounds.with_upper(val - 1.0),
                            CmpOp::LtEq => bounds.with_upper(val),
                            CmpOp::NotEq => bounds.with_not_equal(val),
                            CmpOp::Eq => bounds.with_lower(val).with_upper(val),
                        };
                        self.bounds.insert(name.clone(), new_bounds);
                    }
                }
                if let Predicate::Var(name) = rhs.as_ref() {
                    if let Some(val) = self.eval_const(lhs) {
                        let bounds = self
                            .bounds
                            .entry(name.clone())
                            .or_insert_with(VarBounds::new)
                            .clone();
                        let new_bounds = match op {
                            CmpOp::Lt => bounds.with_lower(val + 1.0),
                            CmpOp::LtEq => bounds.with_lower(val),
                            CmpOp::Gt => bounds.with_upper(val - 1.0),
                            CmpOp::GtEq => bounds.with_upper(val),
                            CmpOp::NotEq => bounds.with_not_equal(val),
                            CmpOp::Eq => bounds.with_lower(val).with_upper(val),
                        };
                        self.bounds.insert(name.clone(), new_bounds);
                    }
                }
            }
            Predicate::And(a, b) => {
                self.extract_bounds(a);
                self.extract_bounds(b);
            }
            _ => {}
        }
    }

    fn eval_const(&self, pred: &Predicate) -> Option<f64> {
        match pred {
            Predicate::IntConst(n) => Some(*n as f64),
            Predicate::FloatConst(f) => Some(*f),
            Predicate::Arith { op, lhs, rhs } => {
                let l = self.eval_const(lhs)?;
                let r = self.eval_const(rhs)?;
                Some(match op {
                    ArithOp::Add => l + r,
                    ArithOp::Sub => l - r,
                    ArithOp::Mul => l * r,
                    ArithOp::Div => {
                        if r.abs() < f64::EPSILON {
                            return None;
                        }
                        l / r
                    }
                    ArithOp::Mod => {
                        if r.abs() < f64::EPSILON {
                            return None;
                        }
                        l % r
                    }
                })
            }
            _ => None,
        }
    }

    fn eval_const_with_bounds(&self, pred: &Predicate) -> Option<f64> {
        if let Some(v) = self.eval_const(pred) {
            return Some(v);
        }
        if let Predicate::Var(name) = pred {
            if let Some(bounds) = self.bounds.get(name) {
                if let (Some(lo), Some(hi)) = (bounds.lower, bounds.upper) {
                    if (lo - hi).abs() < f64::EPSILON {
                        return Some(lo);
                    }
                }
            }
        }
        None
    }

    fn bounds_satisfiable(&self) -> bool {
        self.bounds.values().all(|b| b.is_satisfiable())
    }

    fn check_entailment_via_bounds(&self, goal: &Predicate) -> bool {
        match goal {
            Predicate::Compare { op, lhs, rhs } => {
                if let Predicate::Var(name) = lhs.as_ref() {
                    if let Some(bounds) = self.bounds.get(name) {
                        if let Some(val) = self.eval_const(rhs) {
                            return match op {
                                CmpOp::Gt => bounds.lower.is_some_and(|lo| lo > val),
                                CmpOp::GtEq => bounds.lower.is_some_and(|lo| lo >= val),
                                CmpOp::Lt => bounds.upper.is_some_and(|hi| hi < val),
                                CmpOp::LtEq => bounds.upper.is_some_and(|hi| hi <= val),
                                CmpOp::NotEq => {
                                    bounds.lower.is_some_and(|lo| lo > val)
                                        || bounds.upper.is_some_and(|hi| hi < val)
                                }
                                CmpOp::Eq => {
                                    bounds.lower.is_some_and(|lo| (lo - val).abs() < f64::EPSILON)
                                        && bounds.upper.is_some_and(|hi| (hi - val).abs() < f64::EPSILON)
                                }
                            };
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Predicate helpers
// ═══════════════════════════════════════════════════════════════════════

/// Substitute all occurrences of `var` with `replacement` in `pred`.
pub fn substitute(pred: &Predicate, var: &str, replacement: &Predicate) -> Predicate {
    match pred {
        Predicate::True | Predicate::False => pred.clone(),
        Predicate::Var(name) => {
            if name == var {
                replacement.clone()
            } else {
                pred.clone()
            }
        }
        Predicate::IntConst(_) | Predicate::FloatConst(_) | Predicate::BoolConst(_) => pred.clone(),
        Predicate::Compare { op, lhs, rhs } => Predicate::Compare {
            op: *op,
            lhs: Box::new(substitute(lhs, var, replacement)),
            rhs: Box::new(substitute(rhs, var, replacement)),
        },
        Predicate::And(a, b) => Predicate::And(
            Box::new(substitute(a, var, replacement)),
            Box::new(substitute(b, var, replacement)),
        ),
        Predicate::Or(a, b) => Predicate::Or(
            Box::new(substitute(a, var, replacement)),
            Box::new(substitute(b, var, replacement)),
        ),
        Predicate::Not(p) => Predicate::Not(Box::new(substitute(p, var, replacement))),
        Predicate::Implies(a, b) => Predicate::Implies(
            Box::new(substitute(a, var, replacement)),
            Box::new(substitute(b, var, replacement)),
        ),
        Predicate::Arith { op, lhs, rhs } => Predicate::Arith {
            op: *op,
            lhs: Box::new(substitute(lhs, var, replacement)),
            rhs: Box::new(substitute(rhs, var, replacement)),
        },
        Predicate::App(name, args) => Predicate::App(
            name.clone(),
            args.iter().map(|a| substitute(a, var, replacement)).collect(),
        ),
    }
}

/// Negate a predicate.
pub fn negate(pred: &Predicate) -> Predicate {
    match pred {
        Predicate::True => Predicate::False,
        Predicate::False => Predicate::True,
        Predicate::Not(p) => *p.clone(),
        Predicate::Compare { op, lhs, rhs } => {
            let neg_op = match op {
                CmpOp::Eq => CmpOp::NotEq,
                CmpOp::NotEq => CmpOp::Eq,
                CmpOp::Lt => CmpOp::GtEq,
                CmpOp::LtEq => CmpOp::Gt,
                CmpOp::Gt => CmpOp::LtEq,
                CmpOp::GtEq => CmpOp::Lt,
            };
            Predicate::Compare {
                op: neg_op,
                lhs: lhs.clone(),
                rhs: rhs.clone(),
            }
        }
        other => Predicate::Not(Box::new(other.clone())),
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Refinement Registry (built-in refined types)
// ═══════════════════════════════════════════════════════════════════════

/// Registry of named refinement types — provides common refinements
/// that can be referenced by name.
pub struct RefinementRegistry {
    entries: HashMap<String, RefinedType>,
}

impl RefinementRegistry {
    /// Create a registry with built-in refinement types.
    pub fn with_builtins() -> Self {
        let mut entries = HashMap::new();

        // Positive: { v: i64 | v > 0 }
        entries.insert(
            "Positive".to_string(),
            RefinedType::new(
                BaseType::I64,
                "v",
                Predicate::Compare {
                    op: CmpOp::Gt,
                    lhs: Box::new(Predicate::Var("v".into())),
                    rhs: Box::new(Predicate::IntConst(0)),
                },
            ),
        );

        // Natural: { v: i64 | v >= 0 }
        entries.insert(
            "Natural".to_string(),
            RefinedType::new(
                BaseType::I64,
                "v",
                Predicate::Compare {
                    op: CmpOp::GtEq,
                    lhs: Box::new(Predicate::Var("v".into())),
                    rhs: Box::new(Predicate::IntConst(0)),
                },
            ),
        );

        // NonZero: { v: i64 | v != 0 }
        entries.insert(
            "NonZero".to_string(),
            RefinedType::new(
                BaseType::I64,
                "v",
                Predicate::Compare {
                    op: CmpOp::NotEq,
                    lhs: Box::new(Predicate::Var("v".into())),
                    rhs: Box::new(Predicate::IntConst(0)),
                },
            ),
        );

        // Percentage: { v: f64 | 0.0 <= v && v <= 100.0 }
        entries.insert(
            "Percentage".to_string(),
            RefinedType::new(
                BaseType::F64,
                "v",
                Predicate::And(
                    Box::new(Predicate::Compare {
                        op: CmpOp::GtEq,
                        lhs: Box::new(Predicate::Var("v".into())),
                        rhs: Box::new(Predicate::FloatConst(0.0)),
                    }),
                    Box::new(Predicate::Compare {
                        op: CmpOp::LtEq,
                        lhs: Box::new(Predicate::Var("v".into())),
                        rhs: Box::new(Predicate::FloatConst(100.0)),
                    }),
                ),
            ),
        );

        // UnitInterval: { v: f64 | 0.0 <= v && v <= 1.0 }
        entries.insert(
            "UnitInterval".to_string(),
            RefinedType::new(
                BaseType::F64,
                "v",
                Predicate::And(
                    Box::new(Predicate::Compare {
                        op: CmpOp::GtEq,
                        lhs: Box::new(Predicate::Var("v".into())),
                        rhs: Box::new(Predicate::FloatConst(0.0)),
                    }),
                    Box::new(Predicate::Compare {
                        op: CmpOp::LtEq,
                        lhs: Box::new(Predicate::Var("v".into())),
                        rhs: Box::new(Predicate::FloatConst(1.0)),
                    }),
                ),
            ),
        );

        // Byte: { v: i64 | 0 <= v && v <= 255 }
        entries.insert(
            "Byte".to_string(),
            RefinedType::new(
                BaseType::I64,
                "v",
                Predicate::And(
                    Box::new(Predicate::Compare {
                        op: CmpOp::GtEq,
                        lhs: Box::new(Predicate::Var("v".into())),
                        rhs: Box::new(Predicate::IntConst(0)),
                    }),
                    Box::new(Predicate::Compare {
                        op: CmpOp::LtEq,
                        lhs: Box::new(Predicate::Var("v".into())),
                        rhs: Box::new(Predicate::IntConst(255)),
                    }),
                ),
            ),
        );

        Self { entries }
    }

    /// Look up a refinement type by name.
    pub fn lookup(&self, name: &str) -> Option<&RefinedType> {
        self.entries.get(name)
    }

    /// Register a new named refinement type.
    pub fn register(&mut self, name: String, ty: RefinedType) {
        self.entries.insert(name, ty);
    }

    /// List all registered names.
    pub fn names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for RefinementRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ─── BaseType ────────────────────────────────────────────────────

    #[test]
    fn test_base_type_display() {
        assert_eq!(format!("{}", BaseType::I64), "i64");
        assert_eq!(format!("{}", BaseType::F64), "f64");
        assert_eq!(format!("{}", BaseType::Bool), "bool");
        assert_eq!(format!("{}", BaseType::Str), "str");
        assert_eq!(format!("{}", BaseType::Void), "void");
        assert_eq!(format!("{}", BaseType::I32), "i32");
        assert_eq!(format!("{}", BaseType::F32), "f32");
    }

    // ─── RefinedType ─────────────────────────────────────────────────

    #[test]
    fn test_unrefined_is_trivial() {
        let t = RefinedType::unrefined(BaseType::I64);
        assert!(t.is_trivial());
        assert_eq!(format!("{}", t), "i64");
    }

    #[test]
    fn test_refined_display() {
        let t = RefinedType::new(
            BaseType::I64,
            "v",
            Predicate::Compare {
                op: CmpOp::Gt,
                lhs: Box::new(Predicate::Var("v".into())),
                rhs: Box::new(Predicate::IntConst(0)),
            },
        );
        assert!(!t.is_trivial());
        let s = format!("{}", t);
        assert!(s.contains("v: i64"), "got: {}", s);
        assert!(s.contains("> 0"), "got: {}", s);
    }

    // ─── Predicates ──────────────────────────────────────────────────

    #[test]
    fn test_predicate_display_true_false() {
        assert_eq!(format!("{}", Predicate::True), "true");
        assert_eq!(format!("{}", Predicate::False), "false");
    }

    #[test]
    fn test_predicate_display_compare() {
        let p = Predicate::Compare {
            op: CmpOp::Lt,
            lhs: Box::new(Predicate::Var("x".into())),
            rhs: Box::new(Predicate::IntConst(10)),
        };
        assert_eq!(format!("{}", p), "(x < 10)");
    }

    #[test]
    fn test_predicate_display_and_or() {
        let p = Predicate::And(
            Box::new(Predicate::Var("a".into())),
            Box::new(Predicate::Var("b".into())),
        );
        assert_eq!(format!("{}", p), "(a && b)");

        let p2 = Predicate::Or(
            Box::new(Predicate::True),
            Box::new(Predicate::False),
        );
        assert_eq!(format!("{}", p2), "(true || false)");
    }

    #[test]
    fn test_predicate_display_not() {
        let p = Predicate::Not(Box::new(Predicate::Var("x".into())));
        assert_eq!(format!("{}", p), "(!x)");
    }

    #[test]
    fn test_predicate_display_implies() {
        let p = Predicate::Implies(
            Box::new(Predicate::Var("a".into())),
            Box::new(Predicate::Var("b".into())),
        );
        assert_eq!(format!("{}", p), "(a => b)");
    }

    #[test]
    fn test_predicate_display_arith() {
        let p = Predicate::Arith {
            op: ArithOp::Add,
            lhs: Box::new(Predicate::Var("x".into())),
            rhs: Box::new(Predicate::IntConst(1)),
        };
        assert_eq!(format!("{}", p), "(x + 1)");
    }

    #[test]
    fn test_predicate_display_app() {
        let p = Predicate::App("len".into(), vec![Predicate::Var("s".into())]);
        assert_eq!(format!("{}", p), "len(s)");
    }

    #[test]
    fn test_cmp_op_display() {
        assert_eq!(format!("{}", CmpOp::Eq), "==");
        assert_eq!(format!("{}", CmpOp::NotEq), "!=");
        assert_eq!(format!("{}", CmpOp::Lt), "<");
        assert_eq!(format!("{}", CmpOp::LtEq), "<=");
        assert_eq!(format!("{}", CmpOp::Gt), ">");
        assert_eq!(format!("{}", CmpOp::GtEq), ">=");
    }

    #[test]
    fn test_arith_op_display() {
        assert_eq!(format!("{}", ArithOp::Add), "+");
        assert_eq!(format!("{}", ArithOp::Sub), "-");
        assert_eq!(format!("{}", ArithOp::Mul), "*");
        assert_eq!(format!("{}", ArithOp::Div), "/");
        assert_eq!(format!("{}", ArithOp::Mod), "%");
    }

    // ─── Solver: satisfiability ──────────────────────────────────────

    #[test]
    fn test_solver_true_sat() {
        let mut solver = ConstraintSolver::new();
        assert!(solver.is_satisfiable(&Predicate::True));
    }

    #[test]
    fn test_solver_false_unsat() {
        let mut solver = ConstraintSolver::new();
        assert!(!solver.is_satisfiable(&Predicate::False));
    }

    #[test]
    fn test_solver_compare_consts() {
        let mut solver = ConstraintSolver::new();
        let p = Predicate::Compare {
            op: CmpOp::Lt,
            lhs: Box::new(Predicate::IntConst(3)),
            rhs: Box::new(Predicate::IntConst(5)),
        };
        assert!(solver.is_satisfiable(&p));
    }

    #[test]
    fn test_solver_compare_consts_false() {
        let mut solver = ConstraintSolver::new();
        let p = Predicate::Compare {
            op: CmpOp::Gt,
            lhs: Box::new(Predicate::IntConst(3)),
            rhs: Box::new(Predicate::IntConst(5)),
        };
        assert!(!solver.is_satisfiable(&p));
    }

    #[test]
    fn test_solver_and_sat() {
        let mut solver = ConstraintSolver::new();
        let p = Predicate::And(
            Box::new(Predicate::BoolConst(true)),
            Box::new(Predicate::BoolConst(true)),
        );
        assert!(solver.is_satisfiable(&p));
    }

    #[test]
    fn test_solver_and_unsat() {
        let mut solver = ConstraintSolver::new();
        let p = Predicate::And(
            Box::new(Predicate::True),
            Box::new(Predicate::False),
        );
        assert!(!solver.is_satisfiable(&p));
    }

    #[test]
    fn test_solver_or_sat() {
        let mut solver = ConstraintSolver::new();
        let p = Predicate::Or(
            Box::new(Predicate::False),
            Box::new(Predicate::True),
        );
        assert!(solver.is_satisfiable(&p));
    }

    #[test]
    fn test_solver_implies() {
        let mut solver = ConstraintSolver::new();
        // false => anything is true
        let p = Predicate::Implies(
            Box::new(Predicate::False),
            Box::new(Predicate::False),
        );
        assert!(solver.is_satisfiable(&p));
    }

    // ─── Solver: entailment ──────────────────────────────────────────

    #[test]
    fn test_entails_trivial() {
        let mut solver = ConstraintSolver::new();
        assert!(solver.entails(&Predicate::True, &Predicate::True));
    }

    #[test]
    fn test_entails_false_anything() {
        let mut solver = ConstraintSolver::new();
        assert!(solver.entails(
            &Predicate::False,
            &Predicate::Compare {
                op: CmpOp::Gt,
                lhs: Box::new(Predicate::Var("x".into())),
                rhs: Box::new(Predicate::IntConst(0)),
            }
        ));
    }

    #[test]
    fn test_entails_positive_to_natural() {
        let mut solver = ConstraintSolver::new();
        // v > 0 entails v >= 0
        let positive = Predicate::Compare {
            op: CmpOp::Gt,
            lhs: Box::new(Predicate::Var("v".into())),
            rhs: Box::new(Predicate::IntConst(0)),
        };
        let natural = Predicate::Compare {
            op: CmpOp::GtEq,
            lhs: Box::new(Predicate::Var("v".into())),
            rhs: Box::new(Predicate::IntConst(0)),
        };
        assert!(solver.entails(&positive, &natural));
    }

    // ─── Solver: subtyping ───────────────────────────────────────────

    #[test]
    fn test_subtype_same_base() {
        let mut solver = ConstraintSolver::new();
        let sub = RefinedType::unrefined(BaseType::I64);
        let sup = RefinedType::unrefined(BaseType::I64);
        assert!(solver.is_subtype(&sub, &sup).is_ok());
    }

    #[test]
    fn test_subtype_different_base() {
        let mut solver = ConstraintSolver::new();
        let sub = RefinedType::unrefined(BaseType::I64);
        let sup = RefinedType::unrefined(BaseType::F64);
        assert!(solver.is_subtype(&sub, &sup).is_err());
    }

    #[test]
    fn test_subtype_refined_to_unrefined() {
        let mut solver = ConstraintSolver::new();
        let sub = RefinedType::new(
            BaseType::I64,
            "v",
            Predicate::Compare {
                op: CmpOp::Gt,
                lhs: Box::new(Predicate::Var("v".into())),
                rhs: Box::new(Predicate::IntConst(0)),
            },
        );
        let sup = RefinedType::unrefined(BaseType::I64);
        assert!(solver.is_subtype(&sub, &sup).is_ok());
    }

    // ─── Substitute ──────────────────────────────────────────────────

    #[test]
    fn test_substitute_var() {
        let p = Predicate::Var("x".into());
        let result = substitute(&p, "x", &Predicate::IntConst(42));
        assert_eq!(result, Predicate::IntConst(42));
    }

    #[test]
    fn test_substitute_no_match() {
        let p = Predicate::Var("y".into());
        let result = substitute(&p, "x", &Predicate::IntConst(42));
        assert_eq!(result, Predicate::Var("y".into()));
    }

    #[test]
    fn test_substitute_in_compare() {
        let p = Predicate::Compare {
            op: CmpOp::Gt,
            lhs: Box::new(Predicate::Var("v".into())),
            rhs: Box::new(Predicate::IntConst(0)),
        };
        let result = substitute(&p, "v", &Predicate::IntConst(5));
        match result {
            Predicate::Compare { lhs, .. } => {
                assert_eq!(*lhs, Predicate::IntConst(5));
            }
            _ => panic!("Expected Compare"),
        }
    }

    // ─── Negate ──────────────────────────────────────────────────────

    #[test]
    fn test_negate_true() {
        assert_eq!(negate(&Predicate::True), Predicate::False);
    }

    #[test]
    fn test_negate_false() {
        assert_eq!(negate(&Predicate::False), Predicate::True);
    }

    #[test]
    fn test_negate_compare() {
        let p = Predicate::Compare {
            op: CmpOp::Lt,
            lhs: Box::new(Predicate::Var("x".into())),
            rhs: Box::new(Predicate::IntConst(0)),
        };
        let neg = negate(&p);
        match neg {
            Predicate::Compare { op, .. } => assert_eq!(op, CmpOp::GtEq),
            _ => panic!("Expected Compare"),
        }
    }

    #[test]
    fn test_negate_double() {
        let p = Predicate::Var("x".into());
        let neg = negate(&Predicate::Not(Box::new(p.clone())));
        assert_eq!(neg, p);
    }

    // ─── Registry ────────────────────────────────────────────────────

    #[test]
    fn test_registry_builtins_exist() {
        let reg = RefinementRegistry::with_builtins();
        assert!(reg.lookup("Positive").is_some());
        assert!(reg.lookup("Natural").is_some());
        assert!(reg.lookup("NonZero").is_some());
        assert!(reg.lookup("Percentage").is_some());
        assert!(reg.lookup("UnitInterval").is_some());
        assert!(reg.lookup("Byte").is_some());
    }

    #[test]
    fn test_registry_lookup_missing() {
        let reg = RefinementRegistry::with_builtins();
        assert!(reg.lookup("DoesNotExist").is_none());
    }

    #[test]
    fn test_registry_register_custom() {
        let mut reg = RefinementRegistry::with_builtins();
        reg.register(
            "Even".into(),
            RefinedType::new(
                BaseType::I64,
                "v",
                Predicate::Compare {
                    op: CmpOp::Eq,
                    lhs: Box::new(Predicate::Arith {
                        op: ArithOp::Mod,
                        lhs: Box::new(Predicate::Var("v".into())),
                        rhs: Box::new(Predicate::IntConst(2)),
                    }),
                    rhs: Box::new(Predicate::IntConst(0)),
                },
            ),
        );
        assert!(reg.lookup("Even").is_some());
    }

    #[test]
    fn test_registry_names() {
        let reg = RefinementRegistry::with_builtins();
        let names = reg.names();
        assert!(names.contains(&"Positive"));
        assert!(names.contains(&"Natural"));
    }

    #[test]
    fn test_registry_default() {
        let reg = RefinementRegistry::default();
        assert!(reg.lookup("Positive").is_some());
    }

    // ─── VarBounds ───────────────────────────────────────────────────

    #[test]
    fn test_bounds_satisfiable() {
        let b = VarBounds::new().with_lower(0.0).with_upper(10.0);
        assert!(b.is_satisfiable());
    }

    #[test]
    fn test_bounds_unsatisfiable() {
        let b = VarBounds::new().with_lower(10.0).with_upper(5.0);
        assert!(!b.is_satisfiable());
    }

    #[test]
    fn test_bounds_not_equal_blocks() {
        let b = VarBounds::new()
            .with_lower(5.0)
            .with_upper(5.0)
            .with_not_equal(5.0);
        assert!(!b.is_satisfiable());
    }

    // ─── RefinementError ─────────────────────────────────────────────

    #[test]
    fn test_refinement_error_display() {
        let e = RefinementError {
            message: "type mismatch".into(),
            span_start: 0,
            span_end: 5,
        };
        let s = format!("{}", e);
        assert!(s.contains("type mismatch"));
        assert!(s.contains("0-5"));
    }

    #[test]
    fn test_solver_default() {
        let solver = ConstraintSolver::default();
        assert!(solver.assumptions.is_empty());
    }

    #[test]
    fn test_solver_assume() {
        let mut solver = ConstraintSolver::new();
        solver.assume(Predicate::Compare {
            op: CmpOp::Gt,
            lhs: Box::new(Predicate::Var("x".into())),
            rhs: Box::new(Predicate::IntConst(0)),
        });
        assert_eq!(solver.assumptions.len(), 1);
    }

    #[test]
    fn test_bool_const_predicate() {
        let mut solver = ConstraintSolver::new();
        assert!(solver.is_satisfiable(&Predicate::BoolConst(true)));
        assert!(!solver.is_satisfiable(&Predicate::BoolConst(false)));
    }

    #[test]
    fn test_float_const_display() {
        let p = Predicate::FloatConst(3.14);
        assert_eq!(format!("{}", p), "3.14");
    }
}
