//! Advanced type inference engine for Vitalis.
//!
//! Implements Hindley-Milner type inference with Algorithm W, unification-based
//! constraint solving, bidirectional type checking, union/intersection types,
//! flow-sensitive type narrowing, and type scheme generalization/instantiation.
//!
//! Modeled after Haskell/OCaml inference, TypeScript's flow analysis,
//! and Kotlin's smart casts.

use std::collections::HashMap;
use std::fmt;

// ── Type Representation ──────────────────────────────────────────────

/// A unique type variable identifier.
pub type TypeVar = u32;

/// Types in the inference system.
#[derive(Debug, Clone, PartialEq)]
pub enum InferType {
    /// A type variable (to be solved via unification).
    Var(TypeVar),
    /// Concrete integer type.
    Int,
    /// Concrete 64-bit float type.
    Float,
    /// Concrete boolean type.
    Bool,
    /// Concrete string type.
    Str,
    /// Void / unit type.
    Void,
    /// Function type: params → return.
    Function(Vec<InferType>, Box<InferType>),
    /// List/array of a type.
    List(Box<InferType>),
    /// Option type (nullable).
    Option(Box<InferType>),
    /// Result type (value or error).
    Result(Box<InferType>, Box<InferType>),
    /// Named type (struct, enum, alias).
    Named(String),
    /// Tuple of types.
    Tuple(Vec<InferType>),
    /// Union of types (A | B).
    Union(Vec<InferType>),
    /// Intersection of types (A & B).
    Intersection(Vec<InferType>),
    /// A generic type applied to type arguments.
    Applied(String, Vec<InferType>),
    /// Never type (bottom) — a function that never returns.
    Never,
}

impl fmt::Display for InferType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Var(v) => write!(f, "?T{v}"),
            Self::Int => write!(f, "i64"),
            Self::Float => write!(f, "f64"),
            Self::Bool => write!(f, "bool"),
            Self::Str => write!(f, "str"),
            Self::Void => write!(f, "void"),
            Self::Never => write!(f, "never"),
            Self::Function(params, ret) => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
            }
            Self::List(t) => write!(f, "[{t}]"),
            Self::Option(t) => write!(f, "{t}?"),
            Self::Result(ok, err) => write!(f, "Result<{ok}, {err}>"),
            Self::Named(n) => write!(f, "{n}"),
            Self::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{t}")?;
                }
                write!(f, ")")
            }
            Self::Union(ts) => {
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, " | ")?; }
                    write!(f, "{t}")?;
                }
                Ok(())
            }
            Self::Intersection(ts) => {
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, " & ")?; }
                    write!(f, "{t}")?;
                }
                Ok(())
            }
            Self::Applied(name, args) => {
                write!(f, "{name}<")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{a}")?;
                }
                write!(f, ">")
            }
        }
    }
}

// ── Type Scheme (Polymorphic Types) ──────────────────────────────────

/// A type scheme (∀ a b. T) for let-polymorphism.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScheme {
    /// Universally quantified type variables.
    pub forall: Vec<TypeVar>,
    /// The body type.
    pub body: InferType,
}

impl TypeScheme {
    pub fn mono(ty: InferType) -> Self {
        Self { forall: Vec::new(), body: ty }
    }

    pub fn poly(forall: Vec<TypeVar>, body: InferType) -> Self {
        Self { forall, body }
    }

    /// Check if this is a monomorphic type (no quantified variables).
    pub fn is_mono(&self) -> bool {
        self.forall.is_empty()
    }
}

impl fmt::Display for TypeScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.forall.is_empty() {
            write!(f, "{}", self.body)
        } else {
            write!(f, "∀")?;
            for (i, v) in self.forall.iter().enumerate() {
                if i > 0 { write!(f, " ")?; }
                write!(f, "T{v}")?;
            }
            write!(f, ". {}", self.body)
        }
    }
}

// ── Substitution ─────────────────────────────────────────────────────

/// A mapping from type variables to types.
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    pub mappings: HashMap<TypeVar, InferType>,
}

impl Substitution {
    pub fn new() -> Self {
        Self { mappings: HashMap::new() }
    }

    /// Bind a type variable to a type.
    pub fn bind(&mut self, var: TypeVar, ty: InferType) {
        self.mappings.insert(var, ty);
    }

    /// Apply this substitution to a type.
    pub fn apply(&self, ty: &InferType) -> InferType {
        self.apply_inner(ty, 0)
    }

    fn apply_inner(&self, ty: &InferType, depth: usize) -> InferType {
        if depth > 200 {
            return ty.clone(); // prevent infinite recursion
        }
        match ty {
            InferType::Var(v) => {
                if let Some(bound) = self.mappings.get(v) {
                    if bound == ty {
                        return ty.clone(); // self-referential, stop
                    }
                    self.apply_inner(bound, depth + 1)
                } else {
                    ty.clone()
                }
            }
            InferType::Function(params, ret) => {
                InferType::Function(
                    params.iter().map(|p| self.apply_inner(p, depth + 1)).collect(),
                    Box::new(self.apply_inner(ret, depth + 1)),
                )
            }
            InferType::List(inner) => InferType::List(Box::new(self.apply_inner(inner, depth + 1))),
            InferType::Option(inner) => InferType::Option(Box::new(self.apply_inner(inner, depth + 1))),
            InferType::Result(ok, err) => InferType::Result(
                Box::new(self.apply_inner(ok, depth + 1)),
                Box::new(self.apply_inner(err, depth + 1)),
            ),
            InferType::Tuple(ts) => InferType::Tuple(
                ts.iter().map(|t| self.apply_inner(t, depth + 1)).collect(),
            ),
            InferType::Union(ts) => InferType::Union(
                ts.iter().map(|t| self.apply_inner(t, depth + 1)).collect(),
            ),
            InferType::Intersection(ts) => InferType::Intersection(
                ts.iter().map(|t| self.apply_inner(t, depth + 1)).collect(),
            ),
            InferType::Applied(name, args) => InferType::Applied(
                name.clone(),
                args.iter().map(|a| self.apply_inner(a, depth + 1)).collect(),
            ),
            // Concrete types pass through
            _ => ty.clone(),
        }
    }

    /// Apply substitution to a type scheme's body.
    pub fn apply_scheme(&self, scheme: &TypeScheme) -> TypeScheme {
        // Don't substitute the quantified variables
        let filtered: Substitution = Substitution {
            mappings: self.mappings.iter()
                .filter(|(k, _)| !scheme.forall.contains(k))
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
        };
        TypeScheme {
            forall: scheme.forall.clone(),
            body: filtered.apply(&scheme.body),
        }
    }

    /// Compose two substitutions: apply s2 after s1.
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = Substitution::new();
        for (k, v) in &other.mappings {
            result.bind(*k, self.apply(v));
        }
        for (k, v) in &self.mappings {
            result.mappings.entry(*k).or_insert_with(|| v.clone());
        }
        result
    }

    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    pub fn len(&self) -> usize {
        self.mappings.len()
    }
}

// ── Inference Errors ─────────────────────────────────────────────────

/// Errors during type inference.
#[derive(Debug, Clone, PartialEq)]
pub enum InferError {
    /// Two types could not be unified.
    UnificationFailure(InferType, InferType),
    /// Occurs check failed (infinite type).
    OccursCheck(TypeVar, InferType),
    /// Variable not found in environment.
    UnboundVariable(String),
    /// Wrong number of arguments.
    ArityMismatch { expected: usize, got: usize },
    /// Cannot narrow type in this context.
    NarrowingFailed(InferType, String),
    /// Type is not callable.
    NotCallable(InferType),
    /// Ambiguous type — cannot determine.
    Ambiguous(InferType),
    /// Generic error message.
    Other(String),
}

impl fmt::Display for InferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnificationFailure(a, b) => write!(f, "cannot unify {a} with {b}"),
            Self::OccursCheck(v, t) => write!(f, "infinite type: ?T{v} occurs in {t}"),
            Self::UnboundVariable(name) => write!(f, "unbound variable: {name}"),
            Self::ArityMismatch { expected, got } => {
                write!(f, "expected {expected} arguments, got {got}")
            }
            Self::NarrowingFailed(ty, ctx) => write!(f, "cannot narrow {ty} in {ctx}"),
            Self::NotCallable(ty) => write!(f, "{ty} is not callable"),
            Self::Ambiguous(ty) => write!(f, "ambiguous type: {ty}"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

// ── Type Environment ─────────────────────────────────────────────────

/// A typing environment mapping names to type schemes.
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    bindings: HashMap<String, TypeScheme>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self { bindings: HashMap::new() }
    }

    pub fn bind(&mut self, name: &str, scheme: TypeScheme) {
        self.bindings.insert(name.to_string(), scheme);
    }

    pub fn lookup(&self, name: &str) -> Option<&TypeScheme> {
        self.bindings.get(name)
    }

    pub fn remove(&mut self, name: &str) {
        self.bindings.remove(name);
    }

    pub fn contains(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Apply a substitution to all bindings.
    pub fn apply_subst(&mut self, subst: &Substitution) {
        for scheme in self.bindings.values_mut() {
            *scheme = subst.apply_scheme(scheme);
        }
    }

    /// Free type variables in the environment.
    pub fn free_vars(&self) -> Vec<TypeVar> {
        let mut result = Vec::new();
        for scheme in self.bindings.values() {
            let fvs = free_type_vars(&scheme.body);
            for fv in fvs {
                if !scheme.forall.contains(&fv) && !result.contains(&fv) {
                    result.push(fv);
                }
            }
        }
        result
    }

    pub fn names(&self) -> Vec<&str> {
        self.bindings.keys().map(|s| s.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

// ── Free Variable Extraction ─────────────────────────────────────────

/// Collect all free type variables in a type.
pub fn free_type_vars(ty: &InferType) -> Vec<TypeVar> {
    match ty {
        InferType::Var(v) => vec![*v],
        InferType::Function(params, ret) => {
            let mut vars: Vec<TypeVar> = params.iter()
                .flat_map(free_type_vars)
                .collect();
            vars.extend(free_type_vars(ret));
            vars.sort();
            vars.dedup();
            vars
        }
        InferType::List(inner) | InferType::Option(inner) => free_type_vars(inner),
        InferType::Result(ok, err) => {
            let mut vars = free_type_vars(ok);
            vars.extend(free_type_vars(err));
            vars.sort();
            vars.dedup();
            vars
        }
        InferType::Tuple(ts) | InferType::Union(ts) | InferType::Intersection(ts) => {
            let mut vars: Vec<TypeVar> = ts.iter().flat_map(free_type_vars).collect();
            vars.sort();
            vars.dedup();
            vars
        }
        InferType::Applied(_, args) => {
            let mut vars: Vec<TypeVar> = args.iter().flat_map(free_type_vars).collect();
            vars.sort();
            vars.dedup();
            vars
        }
        _ => Vec::new(),
    }
}

/// Check if a type variable occurs in a type (occurs check).
pub fn occurs_in(var: TypeVar, ty: &InferType) -> bool {
    match ty {
        InferType::Var(v) => *v == var,
        InferType::Function(params, ret) => {
            params.iter().any(|p| occurs_in(var, p)) || occurs_in(var, ret)
        }
        InferType::List(inner) | InferType::Option(inner) => occurs_in(var, inner),
        InferType::Result(ok, err) => occurs_in(var, ok) || occurs_in(var, err),
        InferType::Tuple(ts) | InferType::Union(ts) | InferType::Intersection(ts) => {
            ts.iter().any(|t| occurs_in(var, t))
        }
        InferType::Applied(_, args) => args.iter().any(|a| occurs_in(var, a)),
        _ => false,
    }
}

// ── Unification ──────────────────────────────────────────────────────

/// Unify two types, producing a substitution that makes them equal.
pub fn unify(a: &InferType, b: &InferType) -> Result<Substitution, InferError> {
    match (a, b) {
        // Identical concrete types
        (InferType::Int, InferType::Int)
        | (InferType::Float, InferType::Float)
        | (InferType::Bool, InferType::Bool)
        | (InferType::Str, InferType::Str)
        | (InferType::Void, InferType::Void)
        | (InferType::Never, InferType::Never) => Ok(Substitution::new()),

        // Named types match by name
        (InferType::Named(n1), InferType::Named(n2)) if n1 == n2 => Ok(Substitution::new()),

        // Variable binding
        (InferType::Var(v), ty) | (ty, InferType::Var(v)) => {
            if let InferType::Var(v2) = ty {
                if v == v2 {
                    return Ok(Substitution::new());
                }
            }
            // Occurs check
            if occurs_in(*v, ty) {
                return Err(InferError::OccursCheck(*v, ty.clone()));
            }
            let mut subst = Substitution::new();
            subst.bind(*v, ty.clone());
            Ok(subst)
        }

        // Function types
        (InferType::Function(p1, r1), InferType::Function(p2, r2)) => {
            if p1.len() != p2.len() {
                return Err(InferError::ArityMismatch {
                    expected: p1.len(),
                    got: p2.len(),
                });
            }
            let mut subst = Substitution::new();
            for (a_param, b_param) in p1.iter().zip(p2.iter()) {
                let s = unify(&subst.apply(a_param), &subst.apply(b_param))?;
                subst = s.compose(&subst);
            }
            let s = unify(&subst.apply(r1), &subst.apply(r2))?;
            Ok(s.compose(&subst))
        }

        // List types
        (InferType::List(a_inner), InferType::List(b_inner)) => unify(a_inner, b_inner),

        // Option types
        (InferType::Option(a_inner), InferType::Option(b_inner)) => unify(a_inner, b_inner),

        // Result types
        (InferType::Result(a_ok, a_err), InferType::Result(b_ok, b_err)) => {
            let s1 = unify(a_ok, b_ok)?;
            let s2 = unify(&s1.apply(a_err), &s1.apply(b_err))?;
            Ok(s2.compose(&s1))
        }

        // Tuple types
        (InferType::Tuple(ts1), InferType::Tuple(ts2)) => {
            if ts1.len() != ts2.len() {
                return Err(InferError::UnificationFailure(a.clone(), b.clone()));
            }
            let mut subst = Substitution::new();
            for (t1, t2) in ts1.iter().zip(ts2.iter()) {
                let s = unify(&subst.apply(t1), &subst.apply(t2))?;
                subst = s.compose(&subst);
            }
            Ok(subst)
        }

        // Applied types (generics)
        (InferType::Applied(n1, a1), InferType::Applied(n2, a2)) if n1 == n2 => {
            if a1.len() != a2.len() {
                return Err(InferError::UnificationFailure(a.clone(), b.clone()));
            }
            let mut subst = Substitution::new();
            for (t1, t2) in a1.iter().zip(a2.iter()) {
                let s = unify(&subst.apply(t1), &subst.apply(t2))?;
                subst = s.compose(&subst);
            }
            Ok(subst)
        }

        // Never unifies with anything (it's the bottom type)
        (InferType::Never, _) | (_, InferType::Never) => Ok(Substitution::new()),

        // Failure
        _ => Err(InferError::UnificationFailure(a.clone(), b.clone())),
    }
}

// ── Type Inference Engine ─────────────────────────────────────────────

/// Expressions for type inference (simplified AST for inference).
#[derive(Debug, Clone)]
pub enum InferExpr {
    /// Integer literal.
    IntLit(i64),
    /// Float literal.
    FloatLit(f64),
    /// Boolean literal.
    BoolLit(bool),
    /// String literal.
    StrLit(String),
    /// Variable reference.
    Var(String),
    /// Function application.
    App(Box<InferExpr>, Vec<InferExpr>),
    /// Lambda: params (name, optional type annotation) → body.
    Lambda(Vec<(String, Option<InferType>)>, Box<InferExpr>),
    /// Let binding: let name = expr in body.
    Let(String, Box<InferExpr>, Box<InferExpr>),
    /// If expression: condition, then branch, else branch.
    If(Box<InferExpr>, Box<InferExpr>, Box<InferExpr>),
    /// Type annotation: expr : type.
    Ascription(Box<InferExpr>, InferType),
    /// Tuple construction.
    MakeTuple(Vec<InferExpr>),
    /// List construction.
    MakeList(Vec<InferExpr>),
}

/// The type inference engine implementing Algorithm W with extensions.
#[derive(Debug)]
pub struct InferEngine {
    next_var: TypeVar,
    pub env: TypeEnv,
    /// Narrowing context for flow-sensitive typing.
    pub narrowings: HashMap<String, Vec<InferType>>,
}

impl InferEngine {
    pub fn new() -> Self {
        Self {
            next_var: 100,
            env: TypeEnv::new(),
            narrowings: HashMap::new(),
        }
    }

    /// Create a fresh type variable.
    pub fn fresh_var(&mut self) -> InferType {
        let v = self.next_var;
        self.next_var += 1;
        InferType::Var(v)
    }

    /// Generalize a type over variables not free in the environment.
    pub fn generalize(&self, ty: &InferType) -> TypeScheme {
        let env_vars = self.env.free_vars();
        let ty_vars = free_type_vars(ty);
        let forall: Vec<TypeVar> = ty_vars
            .into_iter()
            .filter(|v| !env_vars.contains(v))
            .collect();
        TypeScheme { forall, body: ty.clone() }
    }

    /// Instantiate a type scheme with fresh variables.
    pub fn instantiate(&mut self, scheme: &TypeScheme) -> InferType {
        let mut subst = Substitution::new();
        for &var in &scheme.forall {
            subst.bind(var, self.fresh_var());
        }
        subst.apply(&scheme.body)
    }

    /// Infer the type of an expression (Algorithm W).
    pub fn infer(&mut self, expr: &InferExpr) -> Result<(Substitution, InferType), InferError> {
        match expr {
            InferExpr::IntLit(_) => Ok((Substitution::new(), InferType::Int)),
            InferExpr::FloatLit(_) => Ok((Substitution::new(), InferType::Float)),
            InferExpr::BoolLit(_) => Ok((Substitution::new(), InferType::Bool)),
            InferExpr::StrLit(_) => Ok((Substitution::new(), InferType::Str)),

            InferExpr::Var(name) => {
                // Check narrowings first
                if let Some(narrowed) = self.narrowings.get(name) {
                    if let Some(last) = narrowed.last() {
                        return Ok((Substitution::new(), last.clone()));
                    }
                }
                if let Some(scheme) = self.env.lookup(name).cloned() {
                    let ty = self.instantiate(&scheme);
                    Ok((Substitution::new(), ty))
                } else {
                    Err(InferError::UnboundVariable(name.clone()))
                }
            }

            InferExpr::App(func, args) => {
                let ret = self.fresh_var();
                let (s1, func_ty) = self.infer(func)?;
                self.env.apply_subst(&s1);

                let mut param_types = Vec::new();
                let mut subst = s1;
                for arg in args {
                    let (s, arg_ty) = self.infer(arg)?;
                    self.env.apply_subst(&s);
                    subst = s.compose(&subst);
                    param_types.push(subst.apply(&arg_ty));
                }

                let expected_fn = InferType::Function(param_types, Box::new(ret.clone()));
                let s_final = unify(&subst.apply(&func_ty), &expected_fn)?;
                let result_subst = s_final.compose(&subst);
                Ok((result_subst.clone(), result_subst.apply(&ret)))
            }

            InferExpr::Lambda(params, body) => {
                let mut param_types = Vec::new();
                let old_env = self.env.clone();

                for (name, annotation) in params {
                    let param_ty = annotation.clone().unwrap_or_else(|| self.fresh_var());
                    self.env.bind(name, TypeScheme::mono(param_ty.clone()));
                    param_types.push(param_ty);
                }

                let (s, body_ty) = self.infer(body)?;
                self.env = old_env;

                let fn_ty = InferType::Function(
                    param_types.iter().map(|t| s.apply(t)).collect(),
                    Box::new(body_ty),
                );
                Ok((s, fn_ty))
            }

            InferExpr::Let(name, value, body) => {
                let (s1, val_ty) = self.infer(value)?;
                self.env.apply_subst(&s1);
                let scheme = self.generalize(&val_ty);
                let old = self.env.lookup(name).cloned();
                self.env.bind(name, scheme);
                let (s2, body_ty) = self.infer(body)?;
                // Restore
                if let Some(prev) = old {
                    self.env.bind(name, prev);
                } else {
                    self.env.remove(name);
                }
                Ok((s2.compose(&s1), body_ty))
            }

            InferExpr::If(cond, then_br, else_br) => {
                let (s1, cond_ty) = self.infer(cond)?;
                let s_bool = unify(&s1.apply(&cond_ty), &InferType::Bool)?;
                let s2 = s_bool.compose(&s1);
                self.env.apply_subst(&s2);

                let (s3, then_ty) = self.infer(then_br)?;
                let s4 = s3.compose(&s2);
                self.env.apply_subst(&s3);

                let (s5, else_ty) = self.infer(else_br)?;
                let s6 = s5.compose(&s4);

                let s_unify = unify(&s6.apply(&then_ty), &s6.apply(&else_ty))?;
                let final_subst = s_unify.compose(&s6);
                Ok((final_subst.clone(), final_subst.apply(&then_ty)))
            }

            InferExpr::Ascription(expr, expected) => {
                let (s1, inferred) = self.infer(expr)?;
                let s2 = unify(&s1.apply(&inferred), expected)?;
                Ok((s2.compose(&s1), expected.clone()))
            }

            InferExpr::MakeTuple(elems) => {
                let mut subst = Substitution::new();
                let mut types = Vec::new();
                for elem in elems {
                    let (s, ty) = self.infer(elem)?;
                    self.env.apply_subst(&s);
                    subst = s.compose(&subst);
                    types.push(subst.apply(&ty));
                }
                Ok((subst, InferType::Tuple(types)))
            }

            InferExpr::MakeList(elems) => {
                if elems.is_empty() {
                    let elem_ty = self.fresh_var();
                    return Ok((Substitution::new(), InferType::List(Box::new(elem_ty))));
                }
                let (mut subst, first_ty) = self.infer(&elems[0])?;
                for elem in &elems[1..] {
                    let (s, elem_ty) = self.infer(elem)?;
                    let s2 = unify(&subst.apply(&first_ty), &s.apply(&elem_ty))?;
                    subst = s2.compose(&s).compose(&subst);
                }
                Ok((subst.clone(), InferType::List(Box::new(subst.apply(&first_ty)))))
            }
        }
    }

    /// Add a type narrowing for a variable (flow-sensitive).
    pub fn narrow(&mut self, name: &str, ty: InferType) {
        self.narrowings.entry(name.to_string()).or_default().push(ty);
    }

    /// Remove the last narrowing for a variable.
    pub fn un_narrow(&mut self, name: &str) {
        if let Some(stack) = self.narrowings.get_mut(name) {
            stack.pop();
            if stack.is_empty() {
                self.narrowings.remove(name);
            }
        }
    }

    /// Bind a name with a monomorphic type.
    pub fn bind_mono(&mut self, name: &str, ty: InferType) {
        self.env.bind(name, TypeScheme::mono(ty));
    }

    /// Bind a name with a polymorphic type scheme.
    pub fn bind_poly(&mut self, name: &str, scheme: TypeScheme) {
        self.env.bind(name, scheme);
    }

    pub fn var_count(&self) -> TypeVar {
        self.next_var
    }
}

impl Default for InferEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Bidirectional Type Checking ──────────────────────────────────────

/// Mode for bidirectional type checking.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckMode {
    /// Synthesize a type from the expression.
    Synthesize,
    /// Check that the expression has a given type.
    Check(InferType),
}

/// Bidirectional type checker wrapping the inference engine.
#[derive(Debug)]
pub struct BidirectionalChecker {
    pub engine: InferEngine,
}

impl BidirectionalChecker {
    pub fn new() -> Self {
        Self { engine: InferEngine::new() }
    }

    /// Check an expression in a given mode.
    pub fn check(&mut self, expr: &InferExpr, mode: &CheckMode) -> Result<InferType, InferError> {
        match mode {
            CheckMode::Synthesize => {
                let (subst, ty) = self.engine.infer(expr)?;
                Ok(subst.apply(&ty))
            }
            CheckMode::Check(expected) => {
                let (subst, inferred) = self.engine.infer(expr)?;
                let resolved = subst.apply(&inferred);
                let _ = unify(&resolved, expected)?;
                Ok(expected.clone())
            }
        }
    }

    /// Synthesize (infer) the type of an expression.
    pub fn synthesize(&mut self, expr: &InferExpr) -> Result<InferType, InferError> {
        self.check(expr, &CheckMode::Synthesize)
    }

    /// Check that an expression has a specific type.
    pub fn check_against(&mut self, expr: &InferExpr, expected: &InferType) -> Result<InferType, InferError> {
        self.check(expr, &CheckMode::Check(expected.clone()))
    }
}

impl Default for BidirectionalChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ── Union / Intersection Helpers ─────────────────────────────────────

/// Construct a union type, flattening nested unions.
pub fn make_union(types: Vec<InferType>) -> InferType {
    let mut flat = Vec::new();
    for ty in types {
        if let InferType::Union(inner) = ty {
            flat.extend(inner);
        } else {
            flat.push(ty);
        }
    }
    // Deduplicate
    flat.dedup();
    if flat.len() == 1 {
        flat.into_iter().next().unwrap()
    } else {
        InferType::Union(flat)
    }
}

/// Construct an intersection type, flattening nested intersections.
pub fn make_intersection(types: Vec<InferType>) -> InferType {
    let mut flat = Vec::new();
    for ty in types {
        if let InferType::Intersection(inner) = ty {
            flat.extend(inner);
        } else {
            flat.push(ty);
        }
    }
    flat.dedup();
    if flat.len() == 1 {
        flat.into_iter().next().unwrap()
    } else {
        InferType::Intersection(flat)
    }
}

/// Check if type `sub` is a subtype of `sup` (subset relation for unions).
pub fn is_subtype(sub: &InferType, sup: &InferType) -> bool {
    if sub == sup {
        return true;
    }
    // Never is subtype of everything
    if matches!(sub, InferType::Never) {
        return true;
    }
    // A union is subtype if all members are subtypes of sup
    if let InferType::Union(members) = sub {
        return members.iter().all(|m| is_subtype(m, sup));
    }
    // Everything is subtype of a union containing it
    if let InferType::Union(members) = sup {
        return members.iter().any(|m| is_subtype(sub, m));
    }
    // An intersection is subtype if any member is subtype
    if let InferType::Intersection(members) = sub {
        return members.iter().any(|m| is_subtype(m, sup));
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Type Display ─────────────────────────────────────────────────

    #[test]
    fn test_type_display() {
        assert_eq!(format!("{}", InferType::Int), "i64");
        assert_eq!(format!("{}", InferType::Var(0)), "?T0");
        assert_eq!(format!("{}", InferType::Never), "never");
        assert_eq!(
            format!("{}", InferType::Function(vec![InferType::Int], Box::new(InferType::Bool))),
            "(i64) -> bool"
        );
        assert_eq!(format!("{}", InferType::List(Box::new(InferType::Str))), "[str]");
        assert_eq!(format!("{}", InferType::Option(Box::new(InferType::Int))), "i64?");
        assert_eq!(
            format!("{}", InferType::Union(vec![InferType::Int, InferType::Str])),
            "i64 | str"
        );
        assert_eq!(
            format!("{}", InferType::Intersection(vec![InferType::Named("A".into()), InferType::Named("B".into())])),
            "A & B"
        );
    }

    #[test]
    fn test_type_display_complex() {
        let ty = InferType::Applied("Map".into(), vec![InferType::Str, InferType::Int]);
        assert_eq!(format!("{ty}"), "Map<str, i64>");
        let tuple = InferType::Tuple(vec![InferType::Int, InferType::Bool]);
        assert_eq!(format!("{tuple}"), "(i64, bool)");
        let result = InferType::Result(Box::new(InferType::Int), Box::new(InferType::Str));
        assert_eq!(format!("{result}"), "Result<i64, str>");
    }

    // ── Type Scheme ──────────────────────────────────────────────────

    #[test]
    fn test_type_scheme_mono() {
        let s = TypeScheme::mono(InferType::Int);
        assert!(s.is_mono());
        assert_eq!(format!("{s}"), "i64");
    }

    #[test]
    fn test_type_scheme_poly() {
        let s = TypeScheme::poly(vec![0, 1], InferType::Function(
            vec![InferType::Var(0)],
            Box::new(InferType::Var(1)),
        ));
        assert!(!s.is_mono());
        assert_eq!(format!("{s}"), "∀T0 T1. (?T0) -> ?T1");
    }

    // ── Substitution ─────────────────────────────────────────────────

    #[test]
    fn test_substitution_apply() {
        let mut subst = Substitution::new();
        subst.bind(0, InferType::Int);
        assert_eq!(subst.apply(&InferType::Var(0)), InferType::Int);
        assert_eq!(subst.apply(&InferType::Var(1)), InferType::Var(1));
        assert_eq!(subst.apply(&InferType::Bool), InferType::Bool);
    }

    #[test]
    fn test_substitution_apply_nested() {
        let mut subst = Substitution::new();
        subst.bind(0, InferType::Int);
        let fn_ty = InferType::Function(vec![InferType::Var(0)], Box::new(InferType::Var(0)));
        let result = subst.apply(&fn_ty);
        assert_eq!(result, InferType::Function(vec![InferType::Int], Box::new(InferType::Int)));
    }

    #[test]
    fn test_substitution_compose() {
        let mut s1 = Substitution::new();
        s1.bind(0, InferType::Var(1));
        let mut s2 = Substitution::new();
        s2.bind(1, InferType::Int);
        let composed = s2.compose(&s1);
        assert_eq!(composed.apply(&InferType::Var(0)), InferType::Int);
    }

    #[test]
    fn test_substitution_apply_scheme() {
        let mut subst = Substitution::new();
        subst.bind(0, InferType::Int);
        subst.bind(1, InferType::Bool);
        // Scheme quantifies over 0, so it shouldn't be substituted
        let scheme = TypeScheme::poly(vec![0], InferType::Function(
            vec![InferType::Var(0)],
            Box::new(InferType::Var(1)),
        ));
        let applied = subst.apply_scheme(&scheme);
        assert_eq!(applied.body, InferType::Function(
            vec![InferType::Var(0)], // unchanged (quantified)
            Box::new(InferType::Bool), // substituted
        ));
    }

    // ── Free Variables ───────────────────────────────────────────────

    #[test]
    fn test_free_type_vars() {
        assert_eq!(free_type_vars(&InferType::Int), Vec::<TypeVar>::new());
        assert_eq!(free_type_vars(&InferType::Var(0)), vec![0]);
        let fn_ty = InferType::Function(vec![InferType::Var(0)], Box::new(InferType::Var(1)));
        let mut fvs = free_type_vars(&fn_ty);
        fvs.sort();
        assert_eq!(fvs, vec![0, 1]);
    }

    // ── Occurs Check ─────────────────────────────────────────────────

    #[test]
    fn test_occurs_in() {
        assert!(occurs_in(0, &InferType::Var(0)));
        assert!(!occurs_in(0, &InferType::Var(1)));
        assert!(!occurs_in(0, &InferType::Int));
        assert!(occurs_in(0, &InferType::List(Box::new(InferType::Var(0)))));
        assert!(occurs_in(0, &InferType::Function(vec![InferType::Var(0)], Box::new(InferType::Int))));
    }

    // ── Unification ──────────────────────────────────────────────────

    #[test]
    fn test_unify_same_concrete() {
        assert!(unify(&InferType::Int, &InferType::Int).is_ok());
        assert!(unify(&InferType::Bool, &InferType::Bool).is_ok());
        assert!(unify(&InferType::Str, &InferType::Str).is_ok());
    }

    #[test]
    fn test_unify_different_concrete_fails() {
        assert!(unify(&InferType::Int, &InferType::Bool).is_err());
        assert!(unify(&InferType::Str, &InferType::Float).is_err());
    }

    #[test]
    fn test_unify_var_concrete() {
        let result = unify(&InferType::Var(0), &InferType::Int).unwrap();
        assert_eq!(result.apply(&InferType::Var(0)), InferType::Int);
    }

    #[test]
    fn test_unify_var_var() {
        let result = unify(&InferType::Var(0), &InferType::Var(1)).unwrap();
        // One should map to the other
        let a = result.apply(&InferType::Var(0));
        let b = result.apply(&InferType::Var(1));
        assert_eq!(a, b);
    }

    #[test]
    fn test_unify_functions() {
        let f1 = InferType::Function(vec![InferType::Var(0)], Box::new(InferType::Int));
        let f2 = InferType::Function(vec![InferType::Bool], Box::new(InferType::Var(1)));
        let result = unify(&f1, &f2).unwrap();
        assert_eq!(result.apply(&InferType::Var(0)), InferType::Bool);
        assert_eq!(result.apply(&InferType::Var(1)), InferType::Int);
    }

    #[test]
    fn test_unify_arity_mismatch() {
        let f1 = InferType::Function(vec![InferType::Int], Box::new(InferType::Int));
        let f2 = InferType::Function(vec![InferType::Int, InferType::Int], Box::new(InferType::Int));
        let err = unify(&f1, &f2).unwrap_err();
        assert!(matches!(err, InferError::ArityMismatch { expected: 1, got: 2 }));
    }

    #[test]
    fn test_unify_occurs_check() {
        // ?T0 = [?T0] would create infinite type
        let err = unify(&InferType::Var(0), &InferType::List(Box::new(InferType::Var(0)))).unwrap_err();
        assert!(matches!(err, InferError::OccursCheck(0, _)));
    }

    #[test]
    fn test_unify_lists() {
        let l1 = InferType::List(Box::new(InferType::Var(0)));
        let l2 = InferType::List(Box::new(InferType::Int));
        let result = unify(&l1, &l2).unwrap();
        assert_eq!(result.apply(&InferType::Var(0)), InferType::Int);
    }

    #[test]
    fn test_unify_tuples() {
        let t1 = InferType::Tuple(vec![InferType::Var(0), InferType::Var(1)]);
        let t2 = InferType::Tuple(vec![InferType::Int, InferType::Bool]);
        let result = unify(&t1, &t2).unwrap();
        assert_eq!(result.apply(&InferType::Var(0)), InferType::Int);
        assert_eq!(result.apply(&InferType::Var(1)), InferType::Bool);
    }

    #[test]
    fn test_unify_result_types() {
        let r1 = InferType::Result(Box::new(InferType::Var(0)), Box::new(InferType::Str));
        let r2 = InferType::Result(Box::new(InferType::Int), Box::new(InferType::Var(1)));
        let result = unify(&r1, &r2).unwrap();
        assert_eq!(result.apply(&InferType::Var(0)), InferType::Int);
        assert_eq!(result.apply(&InferType::Var(1)), InferType::Str);
    }

    #[test]
    fn test_unify_never() {
        assert!(unify(&InferType::Never, &InferType::Int).is_ok());
        assert!(unify(&InferType::Bool, &InferType::Never).is_ok());
    }

    // ── Inference Engine ─────────────────────────────────────────────

    #[test]
    fn test_infer_literals() {
        let mut engine = InferEngine::new();
        let (_, ty) = engine.infer(&InferExpr::IntLit(42)).unwrap();
        assert_eq!(ty, InferType::Int);
        let (_, ty) = engine.infer(&InferExpr::FloatLit(3.14)).unwrap();
        assert_eq!(ty, InferType::Float);
        let (_, ty) = engine.infer(&InferExpr::BoolLit(true)).unwrap();
        assert_eq!(ty, InferType::Bool);
        let (_, ty) = engine.infer(&InferExpr::StrLit("hi".into())).unwrap();
        assert_eq!(ty, InferType::Str);
    }

    #[test]
    fn test_infer_variable() {
        let mut engine = InferEngine::new();
        engine.bind_mono("x", InferType::Int);
        let (_, ty) = engine.infer(&InferExpr::Var("x".into())).unwrap();
        assert_eq!(ty, InferType::Int);
    }

    #[test]
    fn test_infer_unbound_variable() {
        let mut engine = InferEngine::new();
        let err = engine.infer(&InferExpr::Var("x".into())).unwrap_err();
        assert!(matches!(err, InferError::UnboundVariable(_)));
    }

    #[test]
    fn test_infer_lambda() {
        let mut engine = InferEngine::new();
        let lam = InferExpr::Lambda(
            vec![("x".into(), Some(InferType::Int))],
            Box::new(InferExpr::Var("x".into())),
        );
        let (_, ty) = engine.infer(&lam).unwrap();
        assert_eq!(ty, InferType::Function(vec![InferType::Int], Box::new(InferType::Int)));
    }

    #[test]
    fn test_infer_application() {
        let mut engine = InferEngine::new();
        engine.bind_mono("add1", InferType::Function(
            vec![InferType::Int],
            Box::new(InferType::Int),
        ));
        let app = InferExpr::App(
            Box::new(InferExpr::Var("add1".into())),
            vec![InferExpr::IntLit(5)],
        );
        let (_, ty) = engine.infer(&app).unwrap();
        assert_eq!(ty, InferType::Int);
    }

    #[test]
    fn test_infer_let_polymorphism() {
        let mut engine = InferEngine::new();
        // let id = λx.x in (id 42)
        let expr = InferExpr::Let(
            "id".into(),
            Box::new(InferExpr::Lambda(
                vec![("x".into(), None)],
                Box::new(InferExpr::Var("x".into())),
            )),
            Box::new(InferExpr::App(
                Box::new(InferExpr::Var("id".into())),
                vec![InferExpr::IntLit(42)],
            )),
        );
        let (_, ty) = engine.infer(&expr).unwrap();
        assert_eq!(ty, InferType::Int);
    }

    #[test]
    fn test_infer_if_expression() {
        let mut engine = InferEngine::new();
        let expr = InferExpr::If(
            Box::new(InferExpr::BoolLit(true)),
            Box::new(InferExpr::IntLit(1)),
            Box::new(InferExpr::IntLit(2)),
        );
        let (_, ty) = engine.infer(&expr).unwrap();
        assert_eq!(ty, InferType::Int);
    }

    #[test]
    fn test_infer_if_branch_mismatch() {
        let mut engine = InferEngine::new();
        let expr = InferExpr::If(
            Box::new(InferExpr::BoolLit(true)),
            Box::new(InferExpr::IntLit(1)),
            Box::new(InferExpr::StrLit("oops".into())),
        );
        assert!(engine.infer(&expr).is_err());
    }

    #[test]
    fn test_infer_ascription() {
        let mut engine = InferEngine::new();
        let expr = InferExpr::Ascription(
            Box::new(InferExpr::IntLit(42)),
            InferType::Int,
        );
        let (_, ty) = engine.infer(&expr).unwrap();
        assert_eq!(ty, InferType::Int);
    }

    #[test]
    fn test_infer_tuple() {
        let mut engine = InferEngine::new();
        let expr = InferExpr::MakeTuple(vec![
            InferExpr::IntLit(1),
            InferExpr::BoolLit(true),
            InferExpr::StrLit("hi".into()),
        ]);
        let (_, ty) = engine.infer(&expr).unwrap();
        assert_eq!(ty, InferType::Tuple(vec![InferType::Int, InferType::Bool, InferType::Str]));
    }

    #[test]
    fn test_infer_list() {
        let mut engine = InferEngine::new();
        let expr = InferExpr::MakeList(vec![
            InferExpr::IntLit(1),
            InferExpr::IntLit(2),
            InferExpr::IntLit(3),
        ]);
        let (_, ty) = engine.infer(&expr).unwrap();
        assert_eq!(ty, InferType::List(Box::new(InferType::Int)));
    }

    #[test]
    fn test_infer_empty_list() {
        let mut engine = InferEngine::new();
        let expr = InferExpr::MakeList(vec![]);
        let (_, ty) = engine.infer(&expr).unwrap();
        // Should be [?T] for some fresh variable
        assert!(matches!(ty, InferType::List(_)));
    }

    #[test]
    fn test_infer_list_heterogeneous_fails() {
        let mut engine = InferEngine::new();
        let expr = InferExpr::MakeList(vec![
            InferExpr::IntLit(1),
            InferExpr::StrLit("two".into()),
        ]);
        assert!(engine.infer(&expr).is_err());
    }

    // ── Generalization / Instantiation ───────────────────────────────

    #[test]
    fn test_generalize_no_env() {
        let engine = InferEngine::new();
        let ty = InferType::Function(vec![InferType::Var(0)], Box::new(InferType::Var(0)));
        let scheme = engine.generalize(&ty);
        assert_eq!(scheme.forall, vec![0]);
    }

    #[test]
    fn test_instantiate_fresh() {
        let mut engine = InferEngine::new();
        let scheme = TypeScheme::poly(vec![0], InferType::Function(
            vec![InferType::Var(0)],
            Box::new(InferType::Var(0)),
        ));
        let inst1 = engine.instantiate(&scheme);
        let inst2 = engine.instantiate(&scheme);
        // Both should have fresh vars, but different from each other
        assert_ne!(inst1, inst2);
    }

    // ── Narrowing ────────────────────────────────────────────────────

    #[test]
    fn test_narrowing() {
        let mut engine = InferEngine::new();
        engine.bind_mono("x", InferType::Union(vec![InferType::Int, InferType::Str]));
        engine.narrow("x", InferType::Int);

        let (_, ty) = engine.infer(&InferExpr::Var("x".into())).unwrap();
        assert_eq!(ty, InferType::Int);

        engine.un_narrow("x");
        let (_, ty) = engine.infer(&InferExpr::Var("x".into())).unwrap();
        // Should be back to union
        assert!(matches!(ty, InferType::Union(_)));
    }

    // ── Bidirectional Checker ────────────────────────────────────────

    #[test]
    fn test_bidir_synthesize() {
        let mut checker = BidirectionalChecker::new();
        let ty = checker.synthesize(&InferExpr::IntLit(42)).unwrap();
        assert_eq!(ty, InferType::Int);
    }

    #[test]
    fn test_bidir_check_success() {
        let mut checker = BidirectionalChecker::new();
        let ty = checker.check_against(&InferExpr::IntLit(42), &InferType::Int).unwrap();
        assert_eq!(ty, InferType::Int);
    }

    #[test]
    fn test_bidir_check_failure() {
        let mut checker = BidirectionalChecker::new();
        assert!(checker.check_against(&InferExpr::IntLit(42), &InferType::Bool).is_err());
    }

    // ── Union / Intersection helpers ─────────────────────────────────

    #[test]
    fn test_make_union_flatten() {
        let u = make_union(vec![
            InferType::Int,
            InferType::Union(vec![InferType::Str, InferType::Bool]),
        ]);
        assert_eq!(u, InferType::Union(vec![InferType::Int, InferType::Str, InferType::Bool]));
    }

    #[test]
    fn test_make_union_single() {
        let u = make_union(vec![InferType::Int]);
        assert_eq!(u, InferType::Int);
    }

    #[test]
    fn test_make_intersection_flatten() {
        let i = make_intersection(vec![
            InferType::Named("A".into()),
            InferType::Intersection(vec![InferType::Named("B".into()), InferType::Named("C".into())]),
        ]);
        assert_eq!(i, InferType::Intersection(vec![
            InferType::Named("A".into()),
            InferType::Named("B".into()),
            InferType::Named("C".into()),
        ]));
    }

    // ── Subtyping ────────────────────────────────────────────────────

    #[test]
    fn test_subtype_same() {
        assert!(is_subtype(&InferType::Int, &InferType::Int));
    }

    #[test]
    fn test_subtype_never() {
        assert!(is_subtype(&InferType::Never, &InferType::Int));
        assert!(is_subtype(&InferType::Never, &InferType::Str));
    }

    #[test]
    fn test_subtype_member_of_union() {
        let union = InferType::Union(vec![InferType::Int, InferType::Str, InferType::Bool]);
        assert!(is_subtype(&InferType::Int, &union));
        assert!(is_subtype(&InferType::Str, &union));
        assert!(!is_subtype(&InferType::Float, &union));
    }

    #[test]
    fn test_subtype_union_of_union() {
        let sub = InferType::Union(vec![InferType::Int, InferType::Str]);
        let sup = InferType::Union(vec![InferType::Int, InferType::Str, InferType::Bool]);
        assert!(is_subtype(&sub, &sup));
    }

    // ── Error Display ────────────────────────────────────────────────

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", InferError::UnificationFailure(InferType::Int, InferType::Bool)),
            "cannot unify i64 with bool"
        );
        assert_eq!(
            format!("{}", InferError::OccursCheck(0, InferType::List(Box::new(InferType::Var(0))))),
            "infinite type: ?T0 occurs in [?T0]"
        );
        assert_eq!(
            format!("{}", InferError::UnboundVariable("x".into())),
            "unbound variable: x"
        );
        assert_eq!(
            format!("{}", InferError::ArityMismatch { expected: 2, got: 3 }),
            "expected 2 arguments, got 3"
        );
    }

    // ── Type Environment ─────────────────────────────────────────────

    #[test]
    fn test_type_env() {
        let mut env = TypeEnv::new();
        assert!(env.is_empty());
        env.bind("x", TypeScheme::mono(InferType::Int));
        env.bind("y", TypeScheme::mono(InferType::Bool));
        assert_eq!(env.len(), 2);
        assert!(env.contains("x"));
        assert!(!env.contains("z"));

        let names = env.names();
        assert!(names.contains(&"x"));
        assert!(names.contains(&"y"));

        env.remove("x");
        assert!(!env.contains("x"));
    }
}
