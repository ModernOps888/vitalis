//! Higher-Kinded Types, GADTs & Type-Level Computation
//!
//! Extends the Vitalis type system with higher-kinded types (HKT), type classes,
//! Generalized Algebraic Data Types (GADTs), type families, functional dependencies,
//! type-level natural numbers, and associated types. Uses kind-checking and
//! constraint-solving for safe, expressive type-level programming.

use std::collections::HashMap;

// ── Kinds ────────────────────────────────────────────────────────────

/// Kind = the "type of types". `*` is the kind of concrete types,
/// `* → *` is the kind of type constructors like `List`, `Option`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    /// `*` — a concrete, fully-applied type.
    Star,
    /// `k1 → k2` — a type constructor that takes a type of kind k1
    /// and produces a type of kind k2.
    Arrow(Box<Kind>, Box<Kind>),
    /// Constraint kind (for type class constraints).
    Constraint,
}

impl Kind {
    /// `* → *` — kind of List, Option, etc.
    pub fn star_to_star() -> Self {
        Kind::Arrow(Box::new(Kind::Star), Box::new(Kind::Star))
    }

    /// `(* → *) → *` — kind of a higher-kinded type parameter.
    pub fn hkt() -> Self {
        Kind::Arrow(
            Box::new(Kind::star_to_star()),
            Box::new(Kind::Star),
        )
    }

    /// Arity of this kind (number of arrows).
    pub fn arity(&self) -> usize {
        match self {
            Kind::Star | Kind::Constraint => 0,
            Kind::Arrow(_, result) => 1 + result.arity(),
        }
    }

    /// Apply one argument: `(k1 → k2)` applied to `k1` yields `k2`.
    pub fn apply(&self) -> Option<&Kind> {
        match self {
            Kind::Arrow(_, result) => Some(result),
            _ => None,
        }
    }

    /// Pretty-print the kind.
    pub fn display(&self) -> String {
        match self {
            Kind::Star => "*".into(),
            Kind::Constraint => "Constraint".into(),
            Kind::Arrow(a, b) => {
                let left = match a.as_ref() {
                    Kind::Arrow(_, _) => format!("({})", a.display()),
                    _ => a.display(),
                };
                format!("{} → {}", left, b.display())
            }
        }
    }
}

// ── Type-Level Types ─────────────────────────────────────────────────

/// A type-level expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyExpr {
    /// A concrete named type: `Int`, `Bool`, `String`.
    Con(String, Kind),
    /// A type variable: `a`, `b`, `f`.
    Var(String, Kind),
    /// Type application: `f a` (e.g., `List Int`, `Map String Int`).
    App(Box<TyExpr>, Box<TyExpr>),
    /// Forall quantifier: `∀a. T`.
    Forall(String, Kind, Box<TyExpr>),
    /// Type-level natural number.
    Nat(u64),
    /// Type-level function arrow: `a -> b`.
    Arrow(Box<TyExpr>, Box<TyExpr>),
}

impl TyExpr {
    /// Get the kind of this type expression.
    pub fn kind(&self) -> Kind {
        match self {
            TyExpr::Con(_, k) | TyExpr::Var(_, k) => k.clone(),
            TyExpr::App(f, _) => {
                if let Kind::Arrow(_, result) = f.kind() {
                    *result
                } else {
                    Kind::Star
                }
            }
            TyExpr::Forall(_, _, body) => body.kind(),
            TyExpr::Nat(_) => Kind::Star,
            TyExpr::Arrow(_, _) => Kind::Star,
        }
    }

    /// Substitute `var` with `replacement` throughout.
    pub fn substitute(&self, var: &str, replacement: &TyExpr) -> TyExpr {
        match self {
            TyExpr::Var(name, _) if name == var => replacement.clone(),
            TyExpr::Var(_, _) | TyExpr::Con(_, _) | TyExpr::Nat(_) => self.clone(),
            TyExpr::App(f, a) => TyExpr::App(
                Box::new(f.substitute(var, replacement)),
                Box::new(a.substitute(var, replacement)),
            ),
            TyExpr::Forall(v, k, body) if v != var => {
                TyExpr::Forall(v.clone(), k.clone(), Box::new(body.substitute(var, replacement)))
            }
            TyExpr::Forall(_, _, _) => self.clone(), // Shadowed
            TyExpr::Arrow(a, b) => TyExpr::Arrow(
                Box::new(a.substitute(var, replacement)),
                Box::new(b.substitute(var, replacement)),
            ),
        }
    }

    /// Collect all free type variables.
    pub fn free_vars(&self) -> Vec<String> {
        match self {
            TyExpr::Var(name, _) => vec![name.clone()],
            TyExpr::Con(_, _) | TyExpr::Nat(_) => vec![],
            TyExpr::App(f, a) => {
                let mut vars = f.free_vars();
                vars.extend(a.free_vars());
                vars.sort();
                vars.dedup();
                vars
            }
            TyExpr::Forall(v, _, body) => {
                body.free_vars().into_iter().filter(|n| n != v).collect()
            }
            TyExpr::Arrow(a, b) => {
                let mut vars = a.free_vars();
                vars.extend(b.free_vars());
                vars.sort();
                vars.dedup();
                vars
            }
        }
    }

    /// Pretty-print the type expression.
    pub fn display(&self) -> String {
        match self {
            TyExpr::Con(name, _) => name.clone(),
            TyExpr::Var(name, _) => name.clone(),
            TyExpr::App(f, a) => format!("({} {})", f.display(), a.display()),
            TyExpr::Forall(v, _, body) => format!("∀{}. {}", v, body.display()),
            TyExpr::Nat(n) => n.to_string(),
            TyExpr::Arrow(a, b) => format!("{} -> {}", a.display(), b.display()),
        }
    }
}

// ── Type Class ───────────────────────────────────────────────────────

/// A type class definition (like Haskell's `class`).
#[derive(Debug, Clone)]
pub struct TypeClass {
    pub name: String,
    /// Type parameters with their kinds.
    pub params: Vec<(String, Kind)>,
    /// Superclass constraints (must be satisfied for any instance).
    pub superclasses: Vec<ClassConstraint>,
    /// Method signatures.
    pub methods: Vec<ClassMethod>,
    /// Functional dependencies.
    pub fundeps: Vec<FunDep>,
}

/// A constraint: `ClassName Type1 Type2 ...`
#[derive(Debug, Clone, PartialEq)]
pub struct ClassConstraint {
    pub class_name: String,
    pub args: Vec<TyExpr>,
}

/// A method in a type class.
#[derive(Debug, Clone)]
pub struct ClassMethod {
    pub name: String,
    pub ty: TyExpr,
    /// Optional default implementation as source text.
    pub default_impl: Option<String>,
}

/// Functional dependency: determines which params are functionally determined.
/// E.g., `a -> b` means `a` uniquely determines `b`.
#[derive(Debug, Clone)]
pub struct FunDep {
    pub determiners: Vec<String>,
    pub determined: Vec<String>,
}

impl TypeClass {
    pub fn new(name: &str, params: Vec<(String, Kind)>) -> Self {
        Self {
            name: name.to_string(),
            params,
            superclasses: Vec::new(),
            methods: Vec::new(),
            fundeps: Vec::new(),
        }
    }

    pub fn add_superclass(&mut self, constraint: ClassConstraint) {
        self.superclasses.push(constraint);
    }

    pub fn add_method(&mut self, name: &str, ty: TyExpr, default_impl: Option<String>) {
        self.methods.push(ClassMethod {
            name: name.to_string(),
            ty,
            default_impl,
        });
    }

    pub fn add_fundep(&mut self, determiners: Vec<String>, determined: Vec<String>) {
        self.fundeps.push(FunDep {
            determiners,
            determined,
        });
    }

    /// Check if a given constraint would be a valid superclass.
    pub fn has_superclass(&self, class_name: &str) -> bool {
        self.superclasses.iter().any(|sc| sc.class_name == class_name)
    }
}

// ── Type Class Instance ──────────────────────────────────────────────

/// An instance of a type class for specific types.
#[derive(Debug, Clone)]
pub struct ClassInstance {
    pub class_name: String,
    /// Concrete type arguments.
    pub type_args: Vec<TyExpr>,
    /// Constraints required by this instance.
    pub constraints: Vec<ClassConstraint>,
    /// Method implementations as source text.
    pub method_impls: HashMap<String, String>,
}

impl ClassInstance {
    pub fn new(class_name: &str, type_args: Vec<TyExpr>) -> Self {
        Self {
            class_name: class_name.to_string(),
            type_args,
            constraints: Vec::new(),
            method_impls: HashMap::new(),
        }
    }

    pub fn add_constraint(&mut self, constraint: ClassConstraint) {
        self.constraints.push(constraint);
    }

    pub fn add_method_impl(&mut self, method: &str, impl_text: &str) {
        self.method_impls
            .insert(method.to_string(), impl_text.to_string());
    }

    /// Check if this instance provides an implementation for a method.
    pub fn has_method(&self, method: &str) -> bool {
        self.method_impls.contains_key(method)
    }
}

// ── Instance Resolution ──────────────────────────────────────────────

/// Resolves type class instances for a given constraint.
#[derive(Debug, Clone, Default)]
pub struct InstanceResolver {
    pub classes: HashMap<String, TypeClass>,
    pub instances: Vec<ClassInstance>,
}

impl InstanceResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_class(&mut self, class: TypeClass) {
        self.classes.insert(class.name.clone(), class);
    }

    pub fn register_instance(&mut self, instance: ClassInstance) {
        self.instances.push(instance);
    }

    /// Find all instances that match a constraint.
    pub fn resolve(&self, constraint: &ClassConstraint) -> Vec<&ClassInstance> {
        self.instances
            .iter()
            .filter(|inst| {
                inst.class_name == constraint.class_name
                    && inst.type_args.len() == constraint.args.len()
                    && inst
                        .type_args
                        .iter()
                        .zip(&constraint.args)
                        .all(|(inst_ty, arg_ty)| self.types_match(inst_ty, arg_ty))
            })
            .collect()
    }

    /// Check if two types match (simple structural equality, vars match anything).
    fn types_match(&self, pattern: &TyExpr, target: &TyExpr) -> bool {
        match (pattern, target) {
            (TyExpr::Var(_, _), _) => true, // Variable matches anything
            (_, TyExpr::Var(_, _)) => true,
            (TyExpr::Con(a, _), TyExpr::Con(b, _)) => a == b,
            (TyExpr::App(f1, a1), TyExpr::App(f2, a2)) => {
                self.types_match(f1, f2) && self.types_match(a1, a2)
            }
            (TyExpr::Nat(a), TyExpr::Nat(b)) => a == b,
            (TyExpr::Arrow(a1, b1), TyExpr::Arrow(a2, b2)) => {
                self.types_match(a1, a2) && self.types_match(b1, b2)
            }
            _ => false,
        }
    }

    /// Check for overlapping instances (ambiguity detection).
    pub fn check_overlap(&self, class_name: &str) -> Vec<(usize, usize)> {
        let class_instances: Vec<(usize, &ClassInstance)> = self
            .instances
            .iter()
            .enumerate()
            .filter(|(_, inst)| inst.class_name == class_name)
            .collect();

        let mut overlaps = Vec::new();
        for i in 0..class_instances.len() {
            for j in (i + 1)..class_instances.len() {
                let (idx_i, inst_i) = &class_instances[i];
                let (idx_j, inst_j) = &class_instances[j];
                if inst_i.type_args.len() == inst_j.type_args.len()
                    && inst_i
                        .type_args
                        .iter()
                        .zip(&inst_j.type_args)
                        .all(|(a, b)| self.types_match(a, b))
                {
                    overlaps.push((*idx_i, *idx_j));
                }
            }
        }
        overlaps
    }

    /// Verify that all superclass constraints are satisfied for an instance.
    pub fn check_superclasses(&self, instance: &ClassInstance) -> Vec<String> {
        let mut errors = Vec::new();
        if let Some(class) = self.classes.get(&instance.class_name) {
            for sc in &class.superclasses {
                // Substitute type args into superclass constraint
                let mut resolved_args = sc.args.clone();
                for (i, (param_name, _)) in class.params.iter().enumerate() {
                    if i < instance.type_args.len() {
                        resolved_args = resolved_args
                            .iter()
                            .map(|a| a.substitute(param_name, &instance.type_args[i]))
                            .collect();
                    }
                }
                let resolved = ClassConstraint {
                    class_name: sc.class_name.clone(),
                    args: resolved_args,
                };
                if self.resolve(&resolved).is_empty() {
                    errors.push(format!(
                        "Missing superclass instance: {} for {}",
                        sc.class_name, instance.class_name
                    ));
                }
            }
        }
        errors
    }
}

// ── GADTs ────────────────────────────────────────────────────────────

/// A Generalized Algebraic Data Type definition.
#[derive(Debug, Clone)]
pub struct Gadt {
    pub name: String,
    pub params: Vec<(String, Kind)>,
    pub constructors: Vec<GadtConstructor>,
}

/// A GADT constructor with its specific return type (type witness).
#[derive(Debug, Clone)]
pub struct GadtConstructor {
    pub name: String,
    /// Parameter types.
    pub params: Vec<TyExpr>,
    /// The specific return type — may refine the GADT's type parameters.
    pub return_type: TyExpr,
}

impl Gadt {
    pub fn new(name: &str, params: Vec<(String, Kind)>) -> Self {
        Self {
            name: name.to_string(),
            params,
            constructors: Vec::new(),
        }
    }

    pub fn add_constructor(&mut self, name: &str, params: Vec<TyExpr>, return_type: TyExpr) {
        self.constructors.push(GadtConstructor {
            name: name.to_string(),
            params,
            return_type,
        });
    }

    /// Verify that constructor return types are valid applications of this GADT.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for ctor in &self.constructors {
            // The return type must be an application of this GADT
            match &ctor.return_type {
                TyExpr::Con(name, _) if name == &self.name => {}
                TyExpr::App(f, _) => {
                    if let TyExpr::Con(name, _) = f.as_ref() {
                        if name != &self.name {
                            errors.push(format!(
                                "Constructor {} return type must be an application of {}",
                                ctor.name, self.name
                            ));
                        }
                    }
                }
                _ => {
                    if self.params.is_empty() {
                        // OK if no params
                    } else {
                        errors.push(format!(
                            "Constructor {} has invalid return type for GADT {}",
                            ctor.name, self.name
                        ));
                    }
                }
            }
        }
        errors
    }
}

// ── Type Families ────────────────────────────────────────────────────

/// A type family (type-level function).
#[derive(Debug, Clone)]
pub struct TypeFamily {
    pub name: String,
    pub kind: Kind,
    /// Whether the family is closed (all equations known) or open (extensible).
    pub closed: bool,
    /// Equations: pattern → result.
    pub equations: Vec<TypeFamilyEquation>,
}

/// One equation in a type family.
#[derive(Debug, Clone)]
pub struct TypeFamilyEquation {
    pub patterns: Vec<TyExpr>,
    pub result: TyExpr,
}

impl TypeFamily {
    pub fn new(name: &str, kind: Kind, closed: bool) -> Self {
        Self {
            name: name.to_string(),
            kind,
            closed,
            equations: Vec::new(),
        }
    }

    pub fn add_equation(&mut self, patterns: Vec<TyExpr>, result: TyExpr) {
        self.equations.push(TypeFamilyEquation { patterns, result });
    }

    /// Evaluate the type family applied to arguments.
    pub fn evaluate(&self, args: &[TyExpr]) -> Option<TyExpr> {
        for eq in &self.equations {
            if eq.patterns.len() != args.len() {
                continue;
            }
            let mut subst: HashMap<String, TyExpr> = HashMap::new();
            if Self::match_patterns(&eq.patterns, args, &mut subst) {
                let mut result = eq.result.clone();
                for (var, ty) in &subst {
                    result = result.substitute(var, ty);
                }
                return Some(result);
            }
        }
        None
    }

    fn match_patterns(
        patterns: &[TyExpr],
        args: &[TyExpr],
        subst: &mut HashMap<String, TyExpr>,
    ) -> bool {
        for (pat, arg) in patterns.iter().zip(args) {
            if !Self::match_pattern(pat, arg, subst) {
                return false;
            }
        }
        true
    }

    fn match_pattern(pat: &TyExpr, arg: &TyExpr, subst: &mut HashMap<String, TyExpr>) -> bool {
        match pat {
            TyExpr::Var(name, _) => {
                if let Some(existing) = subst.get(name) {
                    *existing == *arg
                } else {
                    subst.insert(name.clone(), arg.clone());
                    true
                }
            }
            TyExpr::Con(a, _) => matches!(arg, TyExpr::Con(b, _) if a == b),
            TyExpr::Nat(a) => matches!(arg, TyExpr::Nat(b) if a == b),
            TyExpr::App(f1, a1) => {
                if let TyExpr::App(f2, a2) = arg {
                    Self::match_pattern(f1, f2, subst) && Self::match_pattern(a1, a2, subst)
                } else {
                    false
                }
            }
            _ => pat == arg,
        }
    }

    /// Check for overlapping equations (closed families only).
    pub fn check_overlap(&self) -> Vec<(usize, usize)> {
        let mut overlaps = Vec::new();
        for i in 0..self.equations.len() {
            for j in (i + 1)..self.equations.len() {
                let mut subst = HashMap::new();
                if Self::match_patterns(&self.equations[i].patterns, &self.equations[j].patterns, &mut subst) {
                    overlaps.push((i, j));
                }
            }
        }
        overlaps
    }
}

// ── Type-Level Naturals ──────────────────────────────────────────────

/// Type-level natural number operations (Peano + arithmetic).
pub struct TypeNat;

impl TypeNat {
    /// Type-level zero.
    pub fn zero() -> TyExpr {
        TyExpr::Nat(0)
    }

    /// Type-level successor.
    pub fn succ(n: &TyExpr) -> TyExpr {
        if let TyExpr::Nat(v) = n {
            TyExpr::Nat(v + 1)
        } else {
            TyExpr::App(
                Box::new(TyExpr::Con("Succ".into(), Kind::star_to_star())),
                Box::new(n.clone()),
            )
        }
    }

    /// Type-level addition.
    pub fn add(a: &TyExpr, b: &TyExpr) -> TyExpr {
        match (a, b) {
            (TyExpr::Nat(x), TyExpr::Nat(y)) => TyExpr::Nat(x + y),
            _ => TyExpr::App(
                Box::new(TyExpr::App(
                    Box::new(TyExpr::Con("Add".into(), Kind::Arrow(
                        Box::new(Kind::Star),
                        Box::new(Kind::star_to_star()),
                    ))),
                    Box::new(a.clone()),
                )),
                Box::new(b.clone()),
            ),
        }
    }

    /// Type-level multiplication.
    pub fn mul(a: &TyExpr, b: &TyExpr) -> TyExpr {
        match (a, b) {
            (TyExpr::Nat(x), TyExpr::Nat(y)) => TyExpr::Nat(x * y),
            _ => TyExpr::App(
                Box::new(TyExpr::App(
                    Box::new(TyExpr::Con("Mul".into(), Kind::Arrow(
                        Box::new(Kind::Star),
                        Box::new(Kind::star_to_star()),
                    ))),
                    Box::new(a.clone()),
                )),
                Box::new(b.clone()),
            ),
        }
    }

    /// Type-level less-than comparison.
    pub fn lt(a: &TyExpr, b: &TyExpr) -> Option<bool> {
        match (a, b) {
            (TyExpr::Nat(x), TyExpr::Nat(y)) => Some(x < y),
            _ => None,
        }
    }

    /// Type-level equality.
    pub fn eq(a: &TyExpr, b: &TyExpr) -> Option<bool> {
        match (a, b) {
            (TyExpr::Nat(x), TyExpr::Nat(y)) => Some(x == y),
            _ => None,
        }
    }
}

// ── Kind Checker ─────────────────────────────────────────────────────

/// Checks that type expressions are well-kinded.
#[derive(Debug, Clone, Default)]
pub struct KindChecker {
    env: HashMap<String, Kind>,
}

impl KindChecker {
    pub fn new() -> Self {
        let mut env = HashMap::new();
        // Built-in type constructors
        env.insert("Int".into(), Kind::Star);
        env.insert("Bool".into(), Kind::Star);
        env.insert("String".into(), Kind::Star);
        env.insert("Float".into(), Kind::Star);
        env.insert("List".into(), Kind::star_to_star());
        env.insert("Option".into(), Kind::star_to_star());
        env.insert("Result".into(), Kind::Arrow(
            Box::new(Kind::Star),
            Box::new(Kind::star_to_star()),
        ));
        env.insert("Map".into(), Kind::Arrow(
            Box::new(Kind::Star),
            Box::new(Kind::star_to_star()),
        ));
        Self { env }
    }

    /// Register a type constructor with its kind.
    pub fn register(&mut self, name: &str, kind: Kind) {
        self.env.insert(name.to_string(), kind);
    }

    /// Infer the kind of a type expression.
    pub fn infer_kind(&self, ty: &TyExpr) -> Result<Kind, String> {
        match ty {
            TyExpr::Con(name, k) => {
                if let Some(env_kind) = self.env.get(name) {
                    if env_kind == k {
                        Ok(k.clone())
                    } else {
                        Err(format!(
                            "Kind mismatch for {}: declared {:?} but environment has {:?}",
                            name, k, env_kind
                        ))
                    }
                } else {
                    Ok(k.clone())
                }
            }
            TyExpr::Var(_, k) => Ok(k.clone()),
            TyExpr::App(f, a) => {
                let f_kind = self.infer_kind(f)?;
                let a_kind = self.infer_kind(a)?;
                match f_kind {
                    Kind::Arrow(expected_arg, result) => {
                        if *expected_arg == a_kind {
                            Ok(*result)
                        } else {
                            Err(format!(
                                "Kind mismatch in application: expected {:?}, got {:?}",
                                expected_arg, a_kind
                            ))
                        }
                    }
                    _ => Err(format!(
                        "Cannot apply type of kind {:?} to an argument",
                        f_kind
                    )),
                }
            }
            TyExpr::Forall(_, _, body) => self.infer_kind(body),
            TyExpr::Nat(_) => Ok(Kind::Star),
            TyExpr::Arrow(a, b) => {
                let ak = self.infer_kind(a)?;
                let bk = self.infer_kind(b)?;
                if ak == Kind::Star && bk == Kind::Star {
                    Ok(Kind::Star)
                } else {
                    Err(format!(
                        "Arrow types must have kind *, got {:?} -> {:?}",
                        ak, bk
                    ))
                }
            }
        }
    }

    /// Check that a type expression has the expected kind.
    pub fn check_kind(&self, ty: &TyExpr, expected: &Kind) -> Result<(), String> {
        let actual = self.infer_kind(ty)?;
        if actual == *expected {
            Ok(())
        } else {
            Err(format!(
                "Expected kind {:?}, got {:?}",
                expected, actual
            ))
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Kind ─────────────────────────────────────────────────────────

    #[test]
    fn test_kind_star() {
        let k = Kind::Star;
        assert_eq!(k.arity(), 0);
        assert_eq!(k.display(), "*");
    }

    #[test]
    fn test_kind_arrow() {
        let k = Kind::star_to_star();
        assert_eq!(k.arity(), 1);
        assert_eq!(k.display(), "* → *");
    }

    #[test]
    fn test_kind_hkt() {
        let k = Kind::hkt();
        assert_eq!(k.arity(), 1);
        assert!(k.display().contains("→"));
    }

    #[test]
    fn test_kind_apply() {
        let k = Kind::star_to_star();
        assert_eq!(k.apply(), Some(&Kind::Star));
        assert_eq!(Kind::Star.apply(), None);
    }

    // ── TyExpr ───────────────────────────────────────────────────────

    #[test]
    fn test_tyexpr_con() {
        let ty = TyExpr::Con("Int".into(), Kind::Star);
        assert_eq!(ty.kind(), Kind::Star);
        assert_eq!(ty.display(), "Int");
    }

    #[test]
    fn test_tyexpr_app() {
        let list = TyExpr::Con("List".into(), Kind::star_to_star());
        let int = TyExpr::Con("Int".into(), Kind::Star);
        let list_int = TyExpr::App(Box::new(list), Box::new(int));
        assert_eq!(list_int.kind(), Kind::Star);
        assert_eq!(list_int.display(), "(List Int)");
    }

    #[test]
    fn test_tyexpr_substitute() {
        let var_a = TyExpr::Var("a".into(), Kind::Star);
        let int = TyExpr::Con("Int".into(), Kind::Star);
        let result = var_a.substitute("a", &int);
        assert_eq!(result, int);
    }

    #[test]
    fn test_tyexpr_free_vars() {
        let ty = TyExpr::Arrow(
            Box::new(TyExpr::Var("a".into(), Kind::Star)),
            Box::new(TyExpr::Var("b".into(), Kind::Star)),
        );
        let fvs = ty.free_vars();
        assert!(fvs.contains(&"a".to_string()));
        assert!(fvs.contains(&"b".to_string()));
    }

    #[test]
    fn test_tyexpr_forall_binds() {
        let ty = TyExpr::Forall(
            "a".into(),
            Kind::Star,
            Box::new(TyExpr::Var("a".into(), Kind::Star)),
        );
        let fvs = ty.free_vars();
        assert!(fvs.is_empty(), "Bound variable should not be free");
    }

    #[test]
    fn test_tyexpr_nat() {
        let n = TyExpr::Nat(42);
        assert_eq!(n.display(), "42");
        assert_eq!(n.kind(), Kind::Star);
    }

    // ── Type Class ───────────────────────────────────────────────────

    #[test]
    fn test_type_class_creation() {
        let mut tc = TypeClass::new("Eq", vec![("a".into(), Kind::Star)]);
        tc.add_method(
            "eq",
            TyExpr::Arrow(
                Box::new(TyExpr::Var("a".into(), Kind::Star)),
                Box::new(TyExpr::Arrow(
                    Box::new(TyExpr::Var("a".into(), Kind::Star)),
                    Box::new(TyExpr::Con("Bool".into(), Kind::Star)),
                )),
            ),
            None,
        );
        assert_eq!(tc.methods.len(), 1);
        assert_eq!(tc.methods[0].name, "eq");
    }

    #[test]
    fn test_type_class_superclass() {
        let mut tc = TypeClass::new("Ord", vec![("a".into(), Kind::Star)]);
        tc.add_superclass(ClassConstraint {
            class_name: "Eq".into(),
            args: vec![TyExpr::Var("a".into(), Kind::Star)],
        });
        assert!(tc.has_superclass("Eq"));
        assert!(!tc.has_superclass("Show"));
    }

    #[test]
    fn test_type_class_fundep() {
        let mut tc = TypeClass::new("Convert", vec![
            ("a".into(), Kind::Star),
            ("b".into(), Kind::Star),
        ]);
        tc.add_fundep(vec!["a".into()], vec!["b".into()]);
        assert_eq!(tc.fundeps.len(), 1);
        assert_eq!(tc.fundeps[0].determiners, vec!["a"]);
    }

    // ── Instance Resolution ──────────────────────────────────────────

    #[test]
    fn test_instance_resolution() {
        let mut resolver = InstanceResolver::new();

        let eq_class = TypeClass::new("Eq", vec![("a".into(), Kind::Star)]);
        resolver.register_class(eq_class);

        let mut int_eq = ClassInstance::new("Eq", vec![TyExpr::Con("Int".into(), Kind::Star)]);
        int_eq.add_method_impl("eq", "a == b");
        resolver.register_instance(int_eq);

        let constraint = ClassConstraint {
            class_name: "Eq".into(),
            args: vec![TyExpr::Con("Int".into(), Kind::Star)],
        };
        let matches = resolver.resolve(&constraint);
        assert_eq!(matches.len(), 1);
        assert!(matches[0].has_method("eq"));
    }

    #[test]
    fn test_instance_no_match() {
        let resolver = InstanceResolver::new();
        let constraint = ClassConstraint {
            class_name: "Show".into(),
            args: vec![TyExpr::Con("Int".into(), Kind::Star)],
        };
        let matches = resolver.resolve(&constraint);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_instance_overlap_detection() {
        let mut resolver = InstanceResolver::new();
        // Two instances for Eq with overlapping types
        resolver.register_instance(ClassInstance::new("Eq", vec![
            TyExpr::Var("a".into(), Kind::Star), // Eq a — matches everything
        ]));
        resolver.register_instance(ClassInstance::new("Eq", vec![
            TyExpr::Con("Int".into(), Kind::Star), // Eq Int — specific
        ]));
        let overlaps = resolver.check_overlap("Eq");
        assert!(!overlaps.is_empty(), "Should detect overlap");
    }

    // ── GADT ─────────────────────────────────────────────────────────

    #[test]
    fn test_gadt_creation() {
        let mut gadt = Gadt::new("Expr", vec![("a".into(), Kind::Star)]);
        gadt.add_constructor(
            "IntLit",
            vec![TyExpr::Con("Int".into(), Kind::Star)],
            TyExpr::App(
                Box::new(TyExpr::Con("Expr".into(), Kind::star_to_star())),
                Box::new(TyExpr::Con("Int".into(), Kind::Star)),
            ),
        );
        gadt.add_constructor(
            "BoolLit",
            vec![TyExpr::Con("Bool".into(), Kind::Star)],
            TyExpr::App(
                Box::new(TyExpr::Con("Expr".into(), Kind::star_to_star())),
                Box::new(TyExpr::Con("Bool".into(), Kind::Star)),
            ),
        );
        assert_eq!(gadt.constructors.len(), 2);
    }

    #[test]
    fn test_gadt_validate() {
        let mut gadt = Gadt::new("Expr", vec![("a".into(), Kind::Star)]);
        gadt.add_constructor(
            "IntLit",
            vec![TyExpr::Con("Int".into(), Kind::Star)],
            TyExpr::App(
                Box::new(TyExpr::Con("Expr".into(), Kind::star_to_star())),
                Box::new(TyExpr::Con("Int".into(), Kind::Star)),
            ),
        );
        let errors = gadt.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_gadt_invalid_return() {
        let mut gadt = Gadt::new("Expr", vec![("a".into(), Kind::Star)]);
        gadt.add_constructor(
            "Bad",
            vec![],
            TyExpr::Con("NotExpr".into(), Kind::Star), // Wrong!
        );
        let errors = gadt.validate();
        assert!(!errors.is_empty());
    }

    // ── Type Family ──────────────────────────────────────────────────

    #[test]
    fn test_type_family_evaluate() {
        let mut fam = TypeFamily::new("Add", Kind::Star, true);

        // Add Zero b = b
        fam.add_equation(
            vec![TyExpr::Nat(0), TyExpr::Var("b".into(), Kind::Star)],
            TyExpr::Var("b".into(), Kind::Star),
        );

        // Evaluate Add 0 5 → 5
        let result = fam.evaluate(&[TyExpr::Nat(0), TyExpr::Nat(5)]);
        assert_eq!(result, Some(TyExpr::Nat(5)));
    }

    #[test]
    fn test_type_family_no_match() {
        let fam = TypeFamily::new("Empty", Kind::Star, true);
        let result = fam.evaluate(&[TyExpr::Nat(1)]);
        assert!(result.is_none());
    }

    #[test]
    fn test_type_family_overlap() {
        let mut fam = TypeFamily::new("F", Kind::Star, true);
        fam.add_equation(
            vec![TyExpr::Var("a".into(), Kind::Star)],
            TyExpr::Con("Int".into(), Kind::Star),
        );
        fam.add_equation(
            vec![TyExpr::Con("Bool".into(), Kind::Star)],
            TyExpr::Con("String".into(), Kind::Star),
        );
        let overlaps = fam.check_overlap();
        assert!(!overlaps.is_empty());
    }

    // ── Type-Level Naturals ──────────────────────────────────────────

    #[test]
    fn test_type_nat_zero() {
        assert_eq!(TypeNat::zero(), TyExpr::Nat(0));
    }

    #[test]
    fn test_type_nat_succ() {
        let one = TypeNat::succ(&TypeNat::zero());
        assert_eq!(one, TyExpr::Nat(1));
    }

    #[test]
    fn test_type_nat_add() {
        let result = TypeNat::add(&TyExpr::Nat(3), &TyExpr::Nat(4));
        assert_eq!(result, TyExpr::Nat(7));
    }

    #[test]
    fn test_type_nat_mul() {
        let result = TypeNat::mul(&TyExpr::Nat(6), &TyExpr::Nat(7));
        assert_eq!(result, TyExpr::Nat(42));
    }

    #[test]
    fn test_type_nat_lt() {
        assert_eq!(TypeNat::lt(&TyExpr::Nat(3), &TyExpr::Nat(5)), Some(true));
        assert_eq!(TypeNat::lt(&TyExpr::Nat(5), &TyExpr::Nat(3)), Some(false));
    }

    #[test]
    fn test_type_nat_eq() {
        assert_eq!(TypeNat::eq(&TyExpr::Nat(42), &TyExpr::Nat(42)), Some(true));
        assert_eq!(TypeNat::eq(&TyExpr::Nat(1), &TyExpr::Nat(2)), Some(false));
    }

    // ── Kind Checker ─────────────────────────────────────────────────

    #[test]
    fn test_kind_check_star() {
        let kc = KindChecker::new();
        let ty = TyExpr::Con("Int".into(), Kind::Star);
        assert!(kc.check_kind(&ty, &Kind::Star).is_ok());
    }

    #[test]
    fn test_kind_check_app() {
        let kc = KindChecker::new();
        let list_int = TyExpr::App(
            Box::new(TyExpr::Con("List".into(), Kind::star_to_star())),
            Box::new(TyExpr::Con("Int".into(), Kind::Star)),
        );
        assert!(kc.check_kind(&list_int, &Kind::Star).is_ok());
    }

    #[test]
    fn test_kind_check_mismatch() {
        let kc = KindChecker::new();
        // Applying Int (kind *) to Int — should fail
        let bad = TyExpr::App(
            Box::new(TyExpr::Con("Int".into(), Kind::Star)),
            Box::new(TyExpr::Con("Int".into(), Kind::Star)),
        );
        assert!(kc.infer_kind(&bad).is_err());
    }

    #[test]
    fn test_kind_check_arrow() {
        let kc = KindChecker::new();
        let arrow = TyExpr::Arrow(
            Box::new(TyExpr::Con("Int".into(), Kind::Star)),
            Box::new(TyExpr::Con("Bool".into(), Kind::Star)),
        );
        assert!(kc.check_kind(&arrow, &Kind::Star).is_ok());
    }

    #[test]
    fn test_kind_checker_register() {
        let mut kc = KindChecker::new();
        kc.register("Tree", Kind::star_to_star());
        let tree_int = TyExpr::App(
            Box::new(TyExpr::Con("Tree".into(), Kind::star_to_star())),
            Box::new(TyExpr::Con("Int".into(), Kind::Star)),
        );
        assert!(kc.check_kind(&tree_int, &Kind::Star).is_ok());
    }
}
