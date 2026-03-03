//! Vitalis Compile-Time Evaluation (v26)
//!
//! A compile-time expression evaluator that supports constant folding,
//! `const fn` evaluation, compile-time assertions, and const generics.
//! This module runs before codegen, evaluating expressions that can be
//! fully resolved at compile time, enabling zero-cost abstractions.
//!
//! # Architecture
//!
//! - **ConstValue**: The value domain at compile time (Int, Float, Bool, Str, Array, Struct)
//! - **ConstEvaluator**: Walks AST expressions, evaluating them to `ConstValue`
//! - **ConstFnRegistry**: Tracks functions marked `const fn` for compile-time dispatch
//! - **StaticAssert**: Compile-time assertions (`static_assert!(cond, "msg")`)
//!
//! # Examples
//!
//! ```text
//! // Vitalis source (future syntax):
//! const PI: f64 = 3.14159265358979;
//! const TAU: f64 = PI * 2.0;
//! const fn factorial(n: i64) -> i64 { if n <= 1 { 1 } else { n * factorial(n - 1) } }
//! const PRECOMPUTED: i64 = factorial(10);
//! static_assert!(PRECOMPUTED == 3628800, "factorial(10) mismatch");
//! ```

use std::collections::HashMap;
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Compile-Time Values
// ═══════════════════════════════════════════════════════════════════════

/// A value that is fully known at compile time.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    /// 64-bit integer.
    Int(i64),
    /// 64-bit float.
    Float(f64),
    /// Boolean.
    Bool(bool),
    /// String.
    Str(String),
    /// Fixed-size array of const values.
    Array(Vec<ConstValue>),
    /// Struct literal: name + ordered fields.
    Struct {
        name: String,
        fields: Vec<(String, ConstValue)>,
    },
    /// The unit / void value.
    Void,
}

impl fmt::Display for ConstValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstValue::Int(n) => write!(f, "{}", n),
            ConstValue::Float(v) => write!(f, "{}", v),
            ConstValue::Bool(b) => write!(f, "{}", b),
            ConstValue::Str(s) => write!(f, "\"{}\"", s),
            ConstValue::Array(elems) => {
                write!(f, "[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }
            ConstValue::Struct { name, fields } => {
                write!(f, "{} {{ ", name)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            ConstValue::Void => write!(f, "void"),
        }
    }
}

impl ConstValue {
    /// Returns the type name of this compile-time value.
    pub fn type_name(&self) -> &'static str {
        match self {
            ConstValue::Int(_) => "i64",
            ConstValue::Float(_) => "f64",
            ConstValue::Bool(_) => "bool",
            ConstValue::Str(_) => "str",
            ConstValue::Array(_) => "array",
            ConstValue::Struct { .. } => "struct",
            ConstValue::Void => "void",
        }
    }

    /// Try to extract as i64.
    pub fn as_int(&self) -> Option<i64> {
        if let ConstValue::Int(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Try to extract as f64.
    pub fn as_float(&self) -> Option<f64> {
        if let ConstValue::Float(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Try to extract as bool.
    pub fn as_bool(&self) -> Option<bool> {
        if let ConstValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Try to extract as string.
    pub fn as_str(&self) -> Option<&str> {
        if let ConstValue::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Check if this value is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            ConstValue::Bool(b) => *b,
            ConstValue::Int(n) => *n != 0,
            ConstValue::Float(v) => *v != 0.0,
            ConstValue::Str(s) => !s.is_empty(),
            ConstValue::Array(a) => !a.is_empty(),
            ConstValue::Void => false,
            ConstValue::Struct { .. } => true,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Const Types for Const Generics
// ═══════════════════════════════════════════════════════════════════════

/// The type of a const generic parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstType {
    I64,
    Bool,
    Str,
}

impl fmt::Display for ConstType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstType::I64 => write!(f, "i64"),
            ConstType::Bool => write!(f, "bool"),
            ConstType::Str => write!(f, "str"),
        }
    }
}

/// A const generic parameter declaration.
#[derive(Debug, Clone)]
pub struct ConstGenericParam {
    pub name: String,
    pub const_type: ConstType,
    pub default: Option<ConstValue>,
}

impl fmt::Display for ConstGenericParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "const {}: {}", self.name, self.const_type)?;
        if let Some(default) = &self.default {
            write!(f, " = {}", default)?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Errors
// ═══════════════════════════════════════════════════════════════════════

/// Errors that can occur during compile-time evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstEvalError {
    /// A variable/name was not found in the const environment.
    UndefinedVariable(String),
    /// Type mismatch in a binary operation.
    TypeMismatch {
        op: String,
        left: String,
        right: String,
    },
    /// Division by zero.
    DivisionByZero,
    /// The expression is not const-evaluable (e.g., function call to non-const fn).
    NotConstEvaluable(String),
    /// A static assertion failed.
    StaticAssertFailed(String),
    /// Overflow in arithmetic.
    Overflow(String),
    /// Recursion limit exceeded for const fn.
    RecursionLimit { depth: usize },
    /// Index out of bounds for const array access.
    IndexOutOfBounds { index: i64, length: usize },
    /// Unknown const function.
    UnknownConstFn(String),
    /// Wrong number of arguments to const fn.
    ArgCountMismatch {
        func: String,
        expected: usize,
        got: usize,
    },
}

impl fmt::Display for ConstEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstEvalError::UndefinedVariable(name) => {
                write!(f, "undefined const variable `{}`", name)
            }
            ConstEvalError::TypeMismatch { op, left, right } => {
                write!(
                    f,
                    "type mismatch in `{}`: cannot apply to {} and {}",
                    op, left, right
                )
            }
            ConstEvalError::DivisionByZero => write!(f, "division by zero in const eval"),
            ConstEvalError::NotConstEvaluable(expr) => {
                write!(f, "expression is not const-evaluable: {}", expr)
            }
            ConstEvalError::StaticAssertFailed(msg) => {
                write!(f, "static assertion failed: {}", msg)
            }
            ConstEvalError::Overflow(msg) => {
                write!(f, "const eval overflow: {}", msg)
            }
            ConstEvalError::RecursionLimit { depth } => {
                write!(f, "const fn recursion limit exceeded (depth {})", depth)
            }
            ConstEvalError::IndexOutOfBounds { index, length } => {
                write!(
                    f,
                    "const array index {} out of bounds (length {})",
                    index, length
                )
            }
            ConstEvalError::UnknownConstFn(name) => {
                write!(f, "unknown const fn `{}`", name)
            }
            ConstEvalError::ArgCountMismatch {
                func,
                expected,
                got,
            } => {
                write!(
                    f,
                    "const fn `{}` expects {} args, got {}",
                    func, expected, got
                )
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Const Function Registry
// ═══════════════════════════════════════════════════════════════════════

/// A const function definition for compile-time dispatch.
#[derive(Debug, Clone)]
pub struct ConstFnDef {
    pub name: String,
    pub params: Vec<String>,
    /// The body is stored as a simple expression tree for const eval.
    pub body: ConstExpr,
}

/// A simplified expression tree for const fn bodies.
/// These are a subset of the full AST, limited to what can run at compile time.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstExpr {
    /// A literal value.
    Literal(ConstValue),
    /// A variable reference (parameter or local const).
    Var(String),
    /// Binary operation.
    BinOp {
        op: ConstBinOp,
        left: Box<ConstExpr>,
        right: Box<ConstExpr>,
    },
    /// Unary operation.
    UnaryOp {
        op: ConstUnaryOp,
        operand: Box<ConstExpr>,
    },
    /// If-then-else expression.
    If {
        condition: Box<ConstExpr>,
        then_val: Box<ConstExpr>,
        else_val: Box<ConstExpr>,
    },
    /// Call a const fn.
    Call {
        func: String,
        args: Vec<ConstExpr>,
    },
    /// Array constructor.
    Array(Vec<ConstExpr>),
    /// Array index: expr[index].
    Index {
        array: Box<ConstExpr>,
        index: Box<ConstExpr>,
    },
    /// Let binding in a const context: `let x = val; body`.
    Let {
        name: String,
        value: Box<ConstExpr>,
        body: Box<ConstExpr>,
    },
}

/// Binary operators available in const expressions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl fmt::Display for ConstBinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstBinOp::Add => write!(f, "+"),
            ConstBinOp::Sub => write!(f, "-"),
            ConstBinOp::Mul => write!(f, "*"),
            ConstBinOp::Div => write!(f, "/"),
            ConstBinOp::Mod => write!(f, "%"),
            ConstBinOp::Eq => write!(f, "=="),
            ConstBinOp::NotEq => write!(f, "!="),
            ConstBinOp::Lt => write!(f, "<"),
            ConstBinOp::Gt => write!(f, ">"),
            ConstBinOp::LtEq => write!(f, "<="),
            ConstBinOp::GtEq => write!(f, ">="),
            ConstBinOp::And => write!(f, "&&"),
            ConstBinOp::Or => write!(f, "||"),
            ConstBinOp::BitAnd => write!(f, "&"),
            ConstBinOp::BitOr => write!(f, "|"),
            ConstBinOp::BitXor => write!(f, "^"),
            ConstBinOp::Shl => write!(f, "<<"),
            ConstBinOp::Shr => write!(f, ">>"),
        }
    }
}

/// Unary operators available in const expressions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstUnaryOp {
    Neg,
    Not,
    BitNot,
}

impl fmt::Display for ConstUnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstUnaryOp::Neg => write!(f, "-"),
            ConstUnaryOp::Not => write!(f, "!"),
            ConstUnaryOp::BitNot => write!(f, "~"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Static Assertions
// ═══════════════════════════════════════════════════════════════════════

/// A compile-time assertion.
#[derive(Debug, Clone)]
pub struct StaticAssert {
    pub condition: ConstExpr,
    pub message: String,
}

// ═══════════════════════════════════════════════════════════════════════
//  Const Fn Registry
// ═══════════════════════════════════════════════════════════════════════

/// Registry of const functions available for compile-time evaluation.
pub struct ConstFnRegistry {
    functions: HashMap<String, ConstFnDef>,
}

impl ConstFnRegistry {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    /// Register a const function.
    pub fn register(&mut self, def: ConstFnDef) {
        self.functions.insert(def.name.clone(), def);
    }

    /// Look up a const function by name.
    pub fn get(&self, name: &str) -> Option<&ConstFnDef> {
        self.functions.get(name)
    }

    /// Check if a function is registered as const.
    pub fn is_const_fn(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Number of registered const functions.
    pub fn count(&self) -> usize {
        self.functions.len()
    }

    /// List all const function names.
    pub fn list(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ConstFnRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Const Evaluator
// ═══════════════════════════════════════════════════════════════════════

/// The compile-time expression evaluator.
pub struct ConstEvaluator {
    /// Named constants in scope.
    constants: HashMap<String, ConstValue>,
    /// Registered const functions.
    fn_registry: ConstFnRegistry,
    /// Maximum recursion depth for const fn calls.
    max_recursion: usize,
    /// Static assertions to verify.
    assertions: Vec<StaticAssert>,
    /// Count of evaluations performed.
    eval_count: usize,
}

impl ConstEvaluator {
    pub fn new() -> Self {
        let mut eval = Self {
            constants: HashMap::new(),
            fn_registry: ConstFnRegistry::new(),
            max_recursion: 256,
            assertions: Vec::new(),
            eval_count: 0,
        };
        eval.register_builtins();
        eval
    }

    fn register_builtins(&mut self) {
        // Register common math const fns
        self.fn_registry.register(ConstFnDef {
            name: "abs".to_string(),
            params: vec!["x".to_string()],
            body: ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Lt,
                    left: Box::new(ConstExpr::Var("x".to_string())),
                    right: Box::new(ConstExpr::Literal(ConstValue::Int(0))),
                }),
                then_val: Box::new(ConstExpr::UnaryOp {
                    op: ConstUnaryOp::Neg,
                    operand: Box::new(ConstExpr::Var("x".to_string())),
                }),
                else_val: Box::new(ConstExpr::Var("x".to_string())),
            },
        });

        self.fn_registry.register(ConstFnDef {
            name: "max".to_string(),
            params: vec!["a".to_string(), "b".to_string()],
            body: ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Gt,
                    left: Box::new(ConstExpr::Var("a".to_string())),
                    right: Box::new(ConstExpr::Var("b".to_string())),
                }),
                then_val: Box::new(ConstExpr::Var("a".to_string())),
                else_val: Box::new(ConstExpr::Var("b".to_string())),
            },
        });

        self.fn_registry.register(ConstFnDef {
            name: "min".to_string(),
            params: vec!["a".to_string(), "b".to_string()],
            body: ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Lt,
                    left: Box::new(ConstExpr::Var("a".to_string())),
                    right: Box::new(ConstExpr::Var("b".to_string())),
                }),
                then_val: Box::new(ConstExpr::Var("a".to_string())),
                else_val: Box::new(ConstExpr::Var("b".to_string())),
            },
        });
    }

    /// Set maximum recursion depth.
    pub fn set_max_recursion(&mut self, depth: usize) {
        self.max_recursion = depth;
    }

    /// Define a named constant.
    pub fn define_const(&mut self, name: &str, value: ConstValue) {
        self.constants.insert(name.to_string(), value);
    }

    /// Look up a constant by name.
    pub fn get_const(&self, name: &str) -> Option<&ConstValue> {
        self.constants.get(name)
    }

    /// Register a const function.
    pub fn register_const_fn(&mut self, def: ConstFnDef) {
        self.fn_registry.register(def);
    }

    /// Check if a function is registered as const.
    pub fn is_const_fn(&self, name: &str) -> bool {
        self.fn_registry.is_const_fn(name)
    }

    /// Add a static assertion.
    pub fn add_assertion(&mut self, assertion: StaticAssert) {
        self.assertions.push(assertion);
    }

    /// Number of evaluations performed.
    pub fn eval_count(&self) -> usize {
        self.eval_count
    }

    /// Number of defined constants.
    pub fn const_count(&self) -> usize {
        self.constants.len()
    }

    /// Number of registered const fns.
    pub fn fn_count(&self) -> usize {
        self.fn_registry.count()
    }

    /// Evaluate a const expression.
    pub fn eval(&mut self, expr: &ConstExpr) -> Result<ConstValue, ConstEvalError> {
        self.eval_depth(expr, &HashMap::new(), 0)
    }

    /// Evaluate with local variable bindings and depth tracking.
    fn eval_depth(
        &mut self,
        expr: &ConstExpr,
        locals: &HashMap<String, ConstValue>,
        depth: usize,
    ) -> Result<ConstValue, ConstEvalError> {
        if depth > self.max_recursion {
            return Err(ConstEvalError::RecursionLimit { depth });
        }
        self.eval_count += 1;

        match expr {
            ConstExpr::Literal(v) => Ok(v.clone()),

            ConstExpr::Var(name) => {
                if let Some(v) = locals.get(name) {
                    Ok(v.clone())
                } else if let Some(v) = self.constants.get(name) {
                    Ok(v.clone())
                } else {
                    Err(ConstEvalError::UndefinedVariable(name.clone()))
                }
            }

            ConstExpr::BinOp { op, left, right } => {
                let l = self.eval_depth(left, locals, depth + 1)?;
                let r = self.eval_depth(right, locals, depth + 1)?;
                self.eval_binop(*op, &l, &r)
            }

            ConstExpr::UnaryOp { op, operand } => {
                let v = self.eval_depth(operand, locals, depth + 1)?;
                self.eval_unaryop(*op, &v)
            }

            ConstExpr::If {
                condition,
                then_val,
                else_val,
            } => {
                let cond = self.eval_depth(condition, locals, depth + 1)?;
                if cond.is_truthy() {
                    self.eval_depth(then_val, locals, depth + 1)
                } else {
                    self.eval_depth(else_val, locals, depth + 1)
                }
            }

            ConstExpr::Call { func, args } => {
                let arg_vals: Vec<ConstValue> = args
                    .iter()
                    .map(|a| self.eval_depth(a, locals, depth + 1))
                    .collect::<Result<_, _>>()?;

                let fn_def = self
                    .fn_registry
                    .get(func)
                    .ok_or_else(|| ConstEvalError::UnknownConstFn(func.clone()))?
                    .clone();

                if arg_vals.len() != fn_def.params.len() {
                    return Err(ConstEvalError::ArgCountMismatch {
                        func: func.clone(),
                        expected: fn_def.params.len(),
                        got: arg_vals.len(),
                    });
                }

                let mut fn_locals = locals.clone();
                for (param, val) in fn_def.params.iter().zip(arg_vals) {
                    fn_locals.insert(param.clone(), val);
                }
                self.eval_depth(&fn_def.body, &fn_locals, depth + 1)
            }

            ConstExpr::Array(elems) => {
                let vals: Vec<ConstValue> = elems
                    .iter()
                    .map(|e| self.eval_depth(e, locals, depth + 1))
                    .collect::<Result<_, _>>()?;
                Ok(ConstValue::Array(vals))
            }

            ConstExpr::Index { array, index } => {
                let arr = self.eval_depth(array, locals, depth + 1)?;
                let idx = self.eval_depth(index, locals, depth + 1)?;
                match (&arr, &idx) {
                    (ConstValue::Array(elems), ConstValue::Int(i)) => {
                        let i = *i;
                        if i < 0 || (i as usize) >= elems.len() {
                            Err(ConstEvalError::IndexOutOfBounds {
                                index: i,
                                length: elems.len(),
                            })
                        } else {
                            Ok(elems[i as usize].clone())
                        }
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        op: "index".to_string(),
                        left: arr.type_name().to_string(),
                        right: idx.type_name().to_string(),
                    }),
                }
            }

            ConstExpr::Let { name, value, body } => {
                let val = self.eval_depth(value, locals, depth + 1)?;
                let mut new_locals = locals.clone();
                new_locals.insert(name.clone(), val);
                self.eval_depth(body, &new_locals, depth + 1)
            }
        }
    }

    fn eval_binop(
        &self,
        op: ConstBinOp,
        left: &ConstValue,
        right: &ConstValue,
    ) -> Result<ConstValue, ConstEvalError> {
        match (left, right) {
            // ── Integer arithmetic ──────────────────────────────
            (ConstValue::Int(a), ConstValue::Int(b)) => match op {
                ConstBinOp::Add => a
                    .checked_add(*b)
                    .map(ConstValue::Int)
                    .ok_or_else(|| ConstEvalError::Overflow(format!("{} + {}", a, b))),
                ConstBinOp::Sub => a
                    .checked_sub(*b)
                    .map(ConstValue::Int)
                    .ok_or_else(|| ConstEvalError::Overflow(format!("{} - {}", a, b))),
                ConstBinOp::Mul => a
                    .checked_mul(*b)
                    .map(ConstValue::Int)
                    .ok_or_else(|| ConstEvalError::Overflow(format!("{} * {}", a, b))),
                ConstBinOp::Div => {
                    if *b == 0 {
                        Err(ConstEvalError::DivisionByZero)
                    } else {
                        Ok(ConstValue::Int(a / b))
                    }
                }
                ConstBinOp::Mod => {
                    if *b == 0 {
                        Err(ConstEvalError::DivisionByZero)
                    } else {
                        Ok(ConstValue::Int(a % b))
                    }
                }
                ConstBinOp::Eq => Ok(ConstValue::Bool(a == b)),
                ConstBinOp::NotEq => Ok(ConstValue::Bool(a != b)),
                ConstBinOp::Lt => Ok(ConstValue::Bool(a < b)),
                ConstBinOp::Gt => Ok(ConstValue::Bool(a > b)),
                ConstBinOp::LtEq => Ok(ConstValue::Bool(a <= b)),
                ConstBinOp::GtEq => Ok(ConstValue::Bool(a >= b)),
                ConstBinOp::BitAnd => Ok(ConstValue::Int(a & b)),
                ConstBinOp::BitOr => Ok(ConstValue::Int(a | b)),
                ConstBinOp::BitXor => Ok(ConstValue::Int(a ^ b)),
                ConstBinOp::Shl => Ok(ConstValue::Int(a << (b & 63))),
                ConstBinOp::Shr => Ok(ConstValue::Int(a >> (b & 63))),
                ConstBinOp::And => Ok(ConstValue::Bool(*a != 0 && *b != 0)),
                ConstBinOp::Or => Ok(ConstValue::Bool(*a != 0 || *b != 0)),
            },

            // ── Float arithmetic ────────────────────────────────
            (ConstValue::Float(a), ConstValue::Float(b)) => match op {
                ConstBinOp::Add => Ok(ConstValue::Float(a + b)),
                ConstBinOp::Sub => Ok(ConstValue::Float(a - b)),
                ConstBinOp::Mul => Ok(ConstValue::Float(a * b)),
                ConstBinOp::Div => {
                    if *b == 0.0 {
                        Err(ConstEvalError::DivisionByZero)
                    } else {
                        Ok(ConstValue::Float(a / b))
                    }
                }
                ConstBinOp::Mod => {
                    if *b == 0.0 {
                        Err(ConstEvalError::DivisionByZero)
                    } else {
                        Ok(ConstValue::Float(a % b))
                    }
                }
                ConstBinOp::Eq => Ok(ConstValue::Bool((a - b).abs() < f64::EPSILON)),
                ConstBinOp::NotEq => Ok(ConstValue::Bool((a - b).abs() >= f64::EPSILON)),
                ConstBinOp::Lt => Ok(ConstValue::Bool(a < b)),
                ConstBinOp::Gt => Ok(ConstValue::Bool(a > b)),
                ConstBinOp::LtEq => Ok(ConstValue::Bool(a <= b)),
                ConstBinOp::GtEq => Ok(ConstValue::Bool(a >= b)),
                _ => Err(ConstEvalError::TypeMismatch {
                    op: format!("{}", op),
                    left: "f64".to_string(),
                    right: "f64".to_string(),
                }),
            },

            // ── Boolean logic ───────────────────────────────────
            (ConstValue::Bool(a), ConstValue::Bool(b)) => match op {
                ConstBinOp::And => Ok(ConstValue::Bool(*a && *b)),
                ConstBinOp::Or => Ok(ConstValue::Bool(*a || *b)),
                ConstBinOp::Eq => Ok(ConstValue::Bool(a == b)),
                ConstBinOp::NotEq => Ok(ConstValue::Bool(a != b)),
                _ => Err(ConstEvalError::TypeMismatch {
                    op: format!("{}", op),
                    left: "bool".to_string(),
                    right: "bool".to_string(),
                }),
            },

            // ── String concatenation & comparison ───────────────
            (ConstValue::Str(a), ConstValue::Str(b)) => match op {
                ConstBinOp::Add => Ok(ConstValue::Str(format!("{}{}", a, b))),
                ConstBinOp::Eq => Ok(ConstValue::Bool(a == b)),
                ConstBinOp::NotEq => Ok(ConstValue::Bool(a != b)),
                _ => Err(ConstEvalError::TypeMismatch {
                    op: format!("{}", op),
                    left: "str".to_string(),
                    right: "str".to_string(),
                }),
            },

            // ── Mixed int/float promotion ───────────────────────
            (ConstValue::Int(a), ConstValue::Float(b)) => {
                self.eval_binop(op, &ConstValue::Float(*a as f64), &ConstValue::Float(*b))
            }
            (ConstValue::Float(a), ConstValue::Int(b)) => {
                self.eval_binop(op, &ConstValue::Float(*a), &ConstValue::Float(*b as f64))
            }

            _ => Err(ConstEvalError::TypeMismatch {
                op: format!("{}", op),
                left: left.type_name().to_string(),
                right: right.type_name().to_string(),
            }),
        }
    }

    fn eval_unaryop(
        &self,
        op: ConstUnaryOp,
        val: &ConstValue,
    ) -> Result<ConstValue, ConstEvalError> {
        match (op, val) {
            (ConstUnaryOp::Neg, ConstValue::Int(n)) => Ok(ConstValue::Int(-n)),
            (ConstUnaryOp::Neg, ConstValue::Float(v)) => Ok(ConstValue::Float(-v)),
            (ConstUnaryOp::Not, ConstValue::Bool(b)) => Ok(ConstValue::Bool(!b)),
            (ConstUnaryOp::BitNot, ConstValue::Int(n)) => Ok(ConstValue::Int(!n)),
            _ => Err(ConstEvalError::TypeMismatch {
                op: format!("{}", op),
                left: val.type_name().to_string(),
                right: "n/a".to_string(),
            }),
        }
    }

    /// Verify all registered static assertions.
    pub fn verify_assertions(&mut self) -> Vec<ConstEvalError> {
        let assertions: Vec<StaticAssert> = self.assertions.clone();
        let mut errors = Vec::new();
        for assertion in &assertions {
            match self.eval(&assertion.condition) {
                Ok(ConstValue::Bool(true)) => { /* pass */ }
                Ok(ConstValue::Bool(false)) => {
                    errors.push(ConstEvalError::StaticAssertFailed(
                        assertion.message.clone(),
                    ));
                }
                Ok(other) => {
                    errors.push(ConstEvalError::TypeMismatch {
                        op: "static_assert".to_string(),
                        left: other.type_name().to_string(),
                        right: "bool".to_string(),
                    });
                }
                Err(e) => errors.push(e),
            }
        }
        errors
    }

    /// Define a constant from a const expression, evaluating immediately.
    pub fn define_const_expr(
        &mut self,
        name: &str,
        expr: &ConstExpr,
    ) -> Result<ConstValue, ConstEvalError> {
        let val = self.eval(expr)?;
        self.constants.insert(name.to_string(), val.clone());
        Ok(val)
    }

    /// List all defined constant names.
    pub fn list_constants(&self) -> Vec<&str> {
        self.constants.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ConstEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Constant Folding Pass
// ═══════════════════════════════════════════════════════════════════════

/// Attempt to fold a const expression to a value, returning None if not fully const.
pub fn try_fold(expr: &ConstExpr) -> Option<ConstValue> {
    let mut eval = ConstEvaluator::new();
    eval.eval(expr).ok()
}

/// Check whether an expression is fully const-evaluable.
pub fn is_const(expr: &ConstExpr, registry: &ConstFnRegistry) -> bool {
    match expr {
        ConstExpr::Literal(_) => true,
        ConstExpr::Var(_) => true, // Assumes defined in scope
        ConstExpr::BinOp { left, right, .. } => {
            is_const(left, registry) && is_const(right, registry)
        }
        ConstExpr::UnaryOp { operand, .. } => is_const(operand, registry),
        ConstExpr::If {
            condition,
            then_val,
            else_val,
        } => {
            is_const(condition, registry)
                && is_const(then_val, registry)
                && is_const(else_val, registry)
        }
        ConstExpr::Call { func, args } => {
            registry.is_const_fn(func) && args.iter().all(|a| is_const(a, registry))
        }
        ConstExpr::Array(elems) => elems.iter().all(|e| is_const(e, registry)),
        ConstExpr::Index { array, index } => {
            is_const(array, registry) && is_const(index, registry)
        }
        ConstExpr::Let { value, body, .. } => {
            is_const(value, registry) && is_const(body, registry)
        }
    }
}

/// Count the number of nodes in a const expression tree.
pub fn expr_depth(expr: &ConstExpr) -> usize {
    match expr {
        ConstExpr::Literal(_) | ConstExpr::Var(_) => 1,
        ConstExpr::BinOp { left, right, .. } => 1 + expr_depth(left) + expr_depth(right),
        ConstExpr::UnaryOp { operand, .. } => 1 + expr_depth(operand),
        ConstExpr::If {
            condition,
            then_val,
            else_val,
        } => 1 + expr_depth(condition) + expr_depth(then_val) + expr_depth(else_val),
        ConstExpr::Call { args, .. } => {
            1 + args.iter().map(expr_depth).sum::<usize>()
        }
        ConstExpr::Array(elems) => 1 + elems.iter().map(expr_depth).sum::<usize>(),
        ConstExpr::Index { array, index } => 1 + expr_depth(array) + expr_depth(index),
        ConstExpr::Let { value, body, .. } => 1 + expr_depth(value) + expr_depth(body),
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── ConstValue basics ──────────────────────────────────────────

    #[test]
    fn test_const_value_display_int() {
        assert_eq!(ConstValue::Int(42).to_string(), "42");
    }

    #[test]
    fn test_const_value_display_float() {
        assert_eq!(ConstValue::Float(3.14).to_string(), "3.14");
    }

    #[test]
    fn test_const_value_display_bool() {
        assert_eq!(ConstValue::Bool(true).to_string(), "true");
    }

    #[test]
    fn test_const_value_display_str() {
        assert_eq!(ConstValue::Str("hello".to_string()).to_string(), "\"hello\"");
    }

    #[test]
    fn test_const_value_display_array() {
        let arr = ConstValue::Array(vec![ConstValue::Int(1), ConstValue::Int(2)]);
        assert_eq!(arr.to_string(), "[1, 2]");
    }

    #[test]
    fn test_const_value_display_struct() {
        let s = ConstValue::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), ConstValue::Int(1)),
                ("y".to_string(), ConstValue::Int(2)),
            ],
        };
        assert_eq!(s.to_string(), "Point { x: 1, y: 2 }");
    }

    #[test]
    fn test_const_value_display_void() {
        assert_eq!(ConstValue::Void.to_string(), "void");
    }

    #[test]
    fn test_const_value_type_name() {
        assert_eq!(ConstValue::Int(0).type_name(), "i64");
        assert_eq!(ConstValue::Float(0.0).type_name(), "f64");
        assert_eq!(ConstValue::Bool(false).type_name(), "bool");
        assert_eq!(ConstValue::Str(String::new()).type_name(), "str");
        assert_eq!(ConstValue::Array(vec![]).type_name(), "array");
        assert_eq!(ConstValue::Void.type_name(), "void");
    }

    #[test]
    fn test_const_value_as_int() {
        assert_eq!(ConstValue::Int(42).as_int(), Some(42));
        assert_eq!(ConstValue::Bool(true).as_int(), None);
    }

    #[test]
    fn test_const_value_as_float() {
        assert_eq!(ConstValue::Float(1.5).as_float(), Some(1.5));
        assert_eq!(ConstValue::Int(1).as_float(), None);
    }

    #[test]
    fn test_const_value_as_bool() {
        assert_eq!(ConstValue::Bool(true).as_bool(), Some(true));
        assert_eq!(ConstValue::Int(1).as_bool(), None);
    }

    #[test]
    fn test_const_value_as_str() {
        assert_eq!(
            ConstValue::Str("hello".to_string()).as_str(),
            Some("hello")
        );
        assert_eq!(ConstValue::Int(1).as_str(), None);
    }

    #[test]
    fn test_const_value_is_truthy() {
        assert!(ConstValue::Bool(true).is_truthy());
        assert!(!ConstValue::Bool(false).is_truthy());
        assert!(ConstValue::Int(1).is_truthy());
        assert!(!ConstValue::Int(0).is_truthy());
        assert!(ConstValue::Str("x".to_string()).is_truthy());
        assert!(!ConstValue::Str(String::new()).is_truthy());
        assert!(!ConstValue::Void.is_truthy());
    }

    // ── ConstType ──────────────────────────────────────────────────

    #[test]
    fn test_const_type_display() {
        assert_eq!(ConstType::I64.to_string(), "i64");
        assert_eq!(ConstType::Bool.to_string(), "bool");
        assert_eq!(ConstType::Str.to_string(), "str");
    }

    // ── ConstGenericParam ──────────────────────────────────────────

    #[test]
    fn test_const_generic_param_display() {
        let p = ConstGenericParam {
            name: "N".to_string(),
            const_type: ConstType::I64,
            default: None,
        };
        assert_eq!(p.to_string(), "const N: i64");

        let p2 = ConstGenericParam {
            name: "N".to_string(),
            const_type: ConstType::I64,
            default: Some(ConstValue::Int(10)),
        };
        assert_eq!(p2.to_string(), "const N: i64 = 10");
    }

    // ── ConstBinOp display ─────────────────────────────────────────

    #[test]
    fn test_const_binop_display() {
        assert_eq!(ConstBinOp::Add.to_string(), "+");
        assert_eq!(ConstBinOp::Sub.to_string(), "-");
        assert_eq!(ConstBinOp::Mul.to_string(), "*");
        assert_eq!(ConstBinOp::Div.to_string(), "/");
        assert_eq!(ConstBinOp::Mod.to_string(), "%");
        assert_eq!(ConstBinOp::Eq.to_string(), "==");
        assert_eq!(ConstBinOp::Lt.to_string(), "<");
        assert_eq!(ConstBinOp::BitAnd.to_string(), "&");
        assert_eq!(ConstBinOp::Shl.to_string(), "<<");
    }

    #[test]
    fn test_const_unaryop_display() {
        assert_eq!(ConstUnaryOp::Neg.to_string(), "-");
        assert_eq!(ConstUnaryOp::Not.to_string(), "!");
        assert_eq!(ConstUnaryOp::BitNot.to_string(), "~");
    }

    // ── ConstEvalError display ─────────────────────────────────────

    #[test]
    fn test_error_display() {
        let e = ConstEvalError::UndefinedVariable("x".to_string());
        assert!(e.to_string().contains("x"));

        let e = ConstEvalError::DivisionByZero;
        assert!(e.to_string().contains("division"));

        let e = ConstEvalError::StaticAssertFailed("invariant".to_string());
        assert!(e.to_string().contains("invariant"));

        let e = ConstEvalError::RecursionLimit { depth: 100 };
        assert!(e.to_string().contains("100"));

        let e = ConstEvalError::IndexOutOfBounds {
            index: 5,
            length: 3,
        };
        assert!(e.to_string().contains("5"));

        let e = ConstEvalError::UnknownConstFn("f".to_string());
        assert!(e.to_string().contains("f"));

        let e = ConstEvalError::ArgCountMismatch {
            func: "g".to_string(),
            expected: 2,
            got: 1,
        };
        assert!(e.to_string().contains("g"));
    }

    // ── Literal evaluation ─────────────────────────────────────────

    #[test]
    fn test_eval_literal_int() {
        let mut eval = ConstEvaluator::new();
        let result = eval.eval(&ConstExpr::Literal(ConstValue::Int(42))).unwrap();
        assert_eq!(result, ConstValue::Int(42));
    }

    #[test]
    fn test_eval_literal_float() {
        let mut eval = ConstEvaluator::new();
        let result = eval
            .eval(&ConstExpr::Literal(ConstValue::Float(3.14)))
            .unwrap();
        assert_eq!(result, ConstValue::Float(3.14));
    }

    #[test]
    fn test_eval_literal_bool() {
        let mut eval = ConstEvaluator::new();
        let result = eval
            .eval(&ConstExpr::Literal(ConstValue::Bool(true)))
            .unwrap();
        assert_eq!(result, ConstValue::Bool(true));
    }

    #[test]
    fn test_eval_literal_str() {
        let mut eval = ConstEvaluator::new();
        let result = eval
            .eval(&ConstExpr::Literal(ConstValue::Str("hi".to_string())))
            .unwrap();
        assert_eq!(result, ConstValue::Str("hi".to_string()));
    }

    // ── Variable evaluation ────────────────────────────────────────

    #[test]
    fn test_eval_defined_const() {
        let mut eval = ConstEvaluator::new();
        eval.define_const("PI", ConstValue::Float(3.14));
        let result = eval.eval(&ConstExpr::Var("PI".to_string())).unwrap();
        assert_eq!(result, ConstValue::Float(3.14));
    }

    #[test]
    fn test_eval_undefined_variable() {
        let mut eval = ConstEvaluator::new();
        let result = eval.eval(&ConstExpr::Var("nope".to_string()));
        assert!(matches!(result, Err(ConstEvalError::UndefinedVariable(_))));
    }

    // ── Integer arithmetic ─────────────────────────────────────────

    #[test]
    fn test_eval_int_add() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Add,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(3))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(4))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(7));
    }

    #[test]
    fn test_eval_int_sub() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Sub,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(10))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(3))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(7));
    }

    #[test]
    fn test_eval_int_mul() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Mul,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(6))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(7))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(42));
    }

    #[test]
    fn test_eval_int_div() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Div,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(10))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(3))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(3));
    }

    #[test]
    fn test_eval_int_mod() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Mod,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(10))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(3))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(1));
    }

    #[test]
    fn test_eval_division_by_zero() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Div,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(10))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(0))),
        };
        assert!(matches!(eval.eval(&expr), Err(ConstEvalError::DivisionByZero)));
    }

    // ── Float arithmetic ───────────────────────────────────────────

    #[test]
    fn test_eval_float_add() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Add,
            left: Box::new(ConstExpr::Literal(ConstValue::Float(1.5))),
            right: Box::new(ConstExpr::Literal(ConstValue::Float(2.5))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Float(4.0));
    }

    #[test]
    fn test_eval_float_mul() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Mul,
            left: Box::new(ConstExpr::Literal(ConstValue::Float(3.0))),
            right: Box::new(ConstExpr::Literal(ConstValue::Float(2.0))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Float(6.0));
    }

    // ── Comparison ─────────────────────────────────────────────────

    #[test]
    fn test_eval_int_eq() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Eq,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(5))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(5))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Bool(true));
    }

    #[test]
    fn test_eval_int_lt() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Lt,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(3))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(5))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Bool(true));
    }

    // ── Bool logic ─────────────────────────────────────────────────

    #[test]
    fn test_eval_bool_and() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::And,
            left: Box::new(ConstExpr::Literal(ConstValue::Bool(true))),
            right: Box::new(ConstExpr::Literal(ConstValue::Bool(false))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Bool(false));
    }

    #[test]
    fn test_eval_bool_or() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Or,
            left: Box::new(ConstExpr::Literal(ConstValue::Bool(false))),
            right: Box::new(ConstExpr::Literal(ConstValue::Bool(true))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Bool(true));
    }

    // ── String ops ─────────────────────────────────────────────────

    #[test]
    fn test_eval_string_concat() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Add,
            left: Box::new(ConstExpr::Literal(ConstValue::Str("hello".to_string()))),
            right: Box::new(ConstExpr::Literal(ConstValue::Str(" world".to_string()))),
        };
        assert_eq!(
            eval.eval(&expr).unwrap(),
            ConstValue::Str("hello world".to_string())
        );
    }

    #[test]
    fn test_eval_string_eq() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Eq,
            left: Box::new(ConstExpr::Literal(ConstValue::Str("a".to_string()))),
            right: Box::new(ConstExpr::Literal(ConstValue::Str("a".to_string()))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Bool(true));
    }

    // ── Unary ops ──────────────────────────────────────────────────

    #[test]
    fn test_eval_unary_neg() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::UnaryOp {
            op: ConstUnaryOp::Neg,
            operand: Box::new(ConstExpr::Literal(ConstValue::Int(5))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(-5));
    }

    #[test]
    fn test_eval_unary_not() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::UnaryOp {
            op: ConstUnaryOp::Not,
            operand: Box::new(ConstExpr::Literal(ConstValue::Bool(true))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Bool(false));
    }

    #[test]
    fn test_eval_unary_bitnot() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::UnaryOp {
            op: ConstUnaryOp::BitNot,
            operand: Box::new(ConstExpr::Literal(ConstValue::Int(0))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(-1));
    }

    // ── If expression ──────────────────────────────────────────────

    #[test]
    fn test_eval_if_true() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::If {
            condition: Box::new(ConstExpr::Literal(ConstValue::Bool(true))),
            then_val: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
            else_val: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(1));
    }

    #[test]
    fn test_eval_if_false() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::If {
            condition: Box::new(ConstExpr::Literal(ConstValue::Bool(false))),
            then_val: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
            else_val: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(2));
    }

    // ── Array ops ──────────────────────────────────────────────────

    #[test]
    fn test_eval_array_literal() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::Array(vec![
            ConstExpr::Literal(ConstValue::Int(10)),
            ConstExpr::Literal(ConstValue::Int(20)),
        ]);
        assert_eq!(
            eval.eval(&expr).unwrap(),
            ConstValue::Array(vec![ConstValue::Int(10), ConstValue::Int(20)])
        );
    }

    #[test]
    fn test_eval_array_index() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::Index {
            array: Box::new(ConstExpr::Array(vec![
                ConstExpr::Literal(ConstValue::Int(10)),
                ConstExpr::Literal(ConstValue::Int(20)),
                ConstExpr::Literal(ConstValue::Int(30)),
            ])),
            index: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(20));
    }

    #[test]
    fn test_eval_array_index_out_of_bounds() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::Index {
            array: Box::new(ConstExpr::Array(vec![
                ConstExpr::Literal(ConstValue::Int(1)),
            ])),
            index: Box::new(ConstExpr::Literal(ConstValue::Int(5))),
        };
        assert!(matches!(
            eval.eval(&expr),
            Err(ConstEvalError::IndexOutOfBounds { .. })
        ));
    }

    // ── Let binding ────────────────────────────────────────────────

    #[test]
    fn test_eval_let_binding() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::Let {
            name: "x".to_string(),
            value: Box::new(ConstExpr::Literal(ConstValue::Int(10))),
            body: Box::new(ConstExpr::BinOp {
                op: ConstBinOp::Mul,
                left: Box::new(ConstExpr::Var("x".to_string())),
                right: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
            }),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(20));
    }

    // ── Const fn calls ─────────────────────────────────────────────

    #[test]
    fn test_eval_builtin_abs() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::Call {
            func: "abs".to_string(),
            args: vec![ConstExpr::Literal(ConstValue::Int(-7))],
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(7));
    }

    #[test]
    fn test_eval_builtin_max() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::Call {
            func: "max".to_string(),
            args: vec![
                ConstExpr::Literal(ConstValue::Int(3)),
                ConstExpr::Literal(ConstValue::Int(7)),
            ],
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(7));
    }

    #[test]
    fn test_eval_builtin_min() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::Call {
            func: "min".to_string(),
            args: vec![
                ConstExpr::Literal(ConstValue::Int(3)),
                ConstExpr::Literal(ConstValue::Int(7)),
            ],
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(3));
    }

    #[test]
    fn test_eval_custom_const_fn() {
        let mut eval = ConstEvaluator::new();
        // Register: const fn double(x) = x * 2
        eval.register_const_fn(ConstFnDef {
            name: "double".to_string(),
            params: vec!["x".to_string()],
            body: ConstExpr::BinOp {
                op: ConstBinOp::Mul,
                left: Box::new(ConstExpr::Var("x".to_string())),
                right: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
            },
        });
        let expr = ConstExpr::Call {
            func: "double".to_string(),
            args: vec![ConstExpr::Literal(ConstValue::Int(21))],
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(42));
    }

    #[test]
    fn test_eval_recursive_const_fn() {
        let mut eval = ConstEvaluator::new();
        // Register: const fn factorial(n) = if n <= 1 { 1 } else { n * factorial(n - 1) }
        eval.register_const_fn(ConstFnDef {
            name: "factorial".to_string(),
            params: vec!["n".to_string()],
            body: ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::LtEq,
                    left: Box::new(ConstExpr::Var("n".to_string())),
                    right: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
                }),
                then_val: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
                else_val: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Mul,
                    left: Box::new(ConstExpr::Var("n".to_string())),
                    right: Box::new(ConstExpr::Call {
                        func: "factorial".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: ConstBinOp::Sub,
                            left: Box::new(ConstExpr::Var("n".to_string())),
                            right: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
                        }],
                    }),
                }),
            },
        });
        let expr = ConstExpr::Call {
            func: "factorial".to_string(),
            args: vec![ConstExpr::Literal(ConstValue::Int(5))],
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(120));
    }

    #[test]
    fn test_eval_unknown_const_fn() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::Call {
            func: "nonexistent".to_string(),
            args: vec![],
        };
        assert!(matches!(
            eval.eval(&expr),
            Err(ConstEvalError::UnknownConstFn(_))
        ));
    }

    #[test]
    fn test_eval_arg_count_mismatch() {
        let mut eval = ConstEvaluator::new();
        // abs expects 1 arg
        let expr = ConstExpr::Call {
            func: "abs".to_string(),
            args: vec![
                ConstExpr::Literal(ConstValue::Int(1)),
                ConstExpr::Literal(ConstValue::Int(2)),
            ],
        };
        assert!(matches!(
            eval.eval(&expr),
            Err(ConstEvalError::ArgCountMismatch { .. })
        ));
    }

    // ── Static assertions ──────────────────────────────────────────

    #[test]
    fn test_static_assert_pass() {
        let mut eval = ConstEvaluator::new();
        eval.add_assertion(StaticAssert {
            condition: ConstExpr::BinOp {
                op: ConstBinOp::Eq,
                left: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
                right: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
            },
            message: "two equals two".to_string(),
        });
        let errors = eval.verify_assertions();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_static_assert_fail() {
        let mut eval = ConstEvaluator::new();
        eval.add_assertion(StaticAssert {
            condition: ConstExpr::BinOp {
                op: ConstBinOp::Eq,
                left: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
                right: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
            },
            message: "one != two".to_string(),
        });
        let errors = eval.verify_assertions();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ConstEvalError::StaticAssertFailed(_)));
    }

    // ── Bitwise ops ────────────────────────────────────────────────

    #[test]
    fn test_eval_bitwise_and() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::BitAnd,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(0b1100))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(0b1010))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(0b1000));
    }

    #[test]
    fn test_eval_bitwise_or() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::BitOr,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(0b1100))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(0b1010))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(0b1110));
    }

    #[test]
    fn test_eval_shift_left() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Shl,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(4))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Int(16));
    }

    // ── Mixed int/float promotion ──────────────────────────────────

    #[test]
    fn test_eval_mixed_int_float() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Add,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
            right: Box::new(ConstExpr::Literal(ConstValue::Float(0.5))),
        };
        assert_eq!(eval.eval(&expr).unwrap(), ConstValue::Float(1.5));
    }

    // ── ConstFnRegistry ────────────────────────────────────────────

    #[test]
    fn test_fn_registry_basics() {
        let mut reg = ConstFnRegistry::new();
        assert_eq!(reg.count(), 0);
        reg.register(ConstFnDef {
            name: "f".to_string(),
            params: vec![],
            body: ConstExpr::Literal(ConstValue::Int(0)),
        });
        assert!(reg.is_const_fn("f"));
        assert!(!reg.is_const_fn("g"));
        assert_eq!(reg.count(), 1);
    }

    // ── define_const_expr ──────────────────────────────────────────

    #[test]
    fn test_define_const_expr() {
        let mut eval = ConstEvaluator::new();
        let val = eval
            .define_const_expr(
                "TAU",
                &ConstExpr::BinOp {
                    op: ConstBinOp::Mul,
                    left: Box::new(ConstExpr::Literal(ConstValue::Float(3.14159))),
                    right: Box::new(ConstExpr::Literal(ConstValue::Float(2.0))),
                },
            )
            .unwrap();
        assert!(matches!(val, ConstValue::Float(_)));
        assert!(eval.get_const("TAU").is_some());
    }

    // ── try_fold ───────────────────────────────────────────────────

    #[test]
    fn test_try_fold_simple() {
        let result = try_fold(&ConstExpr::BinOp {
            op: ConstBinOp::Add,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(3))),
        });
        assert_eq!(result, Some(ConstValue::Int(5)));
    }

    #[test]
    fn test_try_fold_with_undefined() {
        let result = try_fold(&ConstExpr::Var("undefined".to_string()));
        assert!(result.is_none());
    }

    // ── is_const ───────────────────────────────────────────────────

    #[test]
    fn test_is_const_literal() {
        let reg = ConstFnRegistry::new();
        assert!(is_const(&ConstExpr::Literal(ConstValue::Int(1)), &reg));
    }

    #[test]
    fn test_is_const_call_registered() {
        let mut reg = ConstFnRegistry::new();
        reg.register(ConstFnDef {
            name: "f".to_string(),
            params: vec![],
            body: ConstExpr::Literal(ConstValue::Int(0)),
        });
        let expr = ConstExpr::Call {
            func: "f".to_string(),
            args: vec![],
        };
        assert!(is_const(&expr, &reg));
    }

    #[test]
    fn test_is_const_call_unregistered() {
        let reg = ConstFnRegistry::new();
        let expr = ConstExpr::Call {
            func: "g".to_string(),
            args: vec![],
        };
        assert!(!is_const(&expr, &reg));
    }

    // ── expr_depth ─────────────────────────────────────────────────

    #[test]
    fn test_expr_depth_literal() {
        assert_eq!(expr_depth(&ConstExpr::Literal(ConstValue::Int(1))), 1);
    }

    #[test]
    fn test_expr_depth_nested() {
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Add,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
            right: Box::new(ConstExpr::BinOp {
                op: ConstBinOp::Mul,
                left: Box::new(ConstExpr::Literal(ConstValue::Int(2))),
                right: Box::new(ConstExpr::Literal(ConstValue::Int(3))),
            }),
        };
        assert_eq!(expr_depth(&expr), 5); // 1+1+(1+1+1)
    }

    // ── eval_count tracking ────────────────────────────────────────

    #[test]
    fn test_eval_count_increments() {
        let mut eval = ConstEvaluator::new();
        assert_eq!(eval.eval_count(), 0);
        eval.eval(&ConstExpr::Literal(ConstValue::Int(1))).unwrap();
        assert!(eval.eval_count() > 0);
    }

    // ── Overflow detection ─────────────────────────────────────────

    #[test]
    fn test_eval_overflow() {
        let mut eval = ConstEvaluator::new();
        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Add,
            left: Box::new(ConstExpr::Literal(ConstValue::Int(i64::MAX))),
            right: Box::new(ConstExpr::Literal(ConstValue::Int(1))),
        };
        assert!(matches!(eval.eval(&expr), Err(ConstEvalError::Overflow(_))));
    }
}
