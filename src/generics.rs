//! Vitalis Generics — Type parameters, monomorphization, and generic resolution.
//!
//! Provides:
//! - Generic function definitions: `fn identity<T>(x: T) -> T { x }`
//! - Generic struct definitions: `struct Pair<A, B> { first: A, second: B }`
//! - Generic trait bounds: `fn add<T: Numeric>(a: T, b: T) -> T`
//! - Monomorphization: generics are expanded into concrete types at compile time
//! - Type inference for generic parameters from call-site arguments
//!
//! The monomorphizer walks the AST, finds generic usages, and generates
//! concrete specialized versions (e.g., `identity_i64`, `identity_f64`).

use std::collections::HashMap;
use std::fmt;

// ─── Type Parameters ────────────────────────────────────────────────────

/// A type parameter declaration: `T`, `T: Bound`, `T: Bound1 + Bound2`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<String>,
    pub default: Option<String>,
}

impl TypeParam {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            bounds: Vec::new(),
            default: None,
        }
    }

    pub fn with_bound(mut self, bound: &str) -> Self {
        self.bounds.push(bound.to_string());
        self
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }

    pub fn has_bound(&self, bound: &str) -> bool {
        self.bounds.iter().any(|b| b == bound)
    }
}

impl fmt::Display for TypeParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.bounds.is_empty() {
            write!(f, ": {}", self.bounds.join(" + "))?;
        }
        if let Some(ref d) = self.default {
            write!(f, " = {}", d)?;
        }
        Ok(())
    }
}

// ─── Generic Signature ──────────────────────────────────────────────────

/// A generic function or struct signature with type parameters.
#[derive(Debug, Clone)]
pub struct GenericSig {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub param_types: Vec<String>,
    pub return_type: Option<String>,
}

impl GenericSig {
    pub fn new(name: &str, type_params: Vec<TypeParam>) -> Self {
        Self {
            name: name.to_string(),
            type_params,
            param_types: Vec::new(),
            return_type: None,
        }
    }

    /// Check if a type name is one of this signature's type parameters.
    pub fn is_type_param(&self, name: &str) -> bool {
        self.type_params.iter().any(|tp| tp.name == name)
    }

    /// Get the index of a type parameter by name.
    pub fn type_param_index(&self, name: &str) -> Option<usize> {
        self.type_params.iter().position(|tp| tp.name == name)
    }

    /// Generate the mangled name for a specific instantiation.
    pub fn mangle(&self, concrete_types: &[String]) -> String {
        let suffix: Vec<&str> = concrete_types.iter().map(|s| s.as_str()).collect();
        format!("{}_{}", self.name, suffix.join("_"))
    }
}

// ─── Type Substitution ──────────────────────────────────────────────────

/// A mapping from type parameter names to concrete types.
#[derive(Debug, Clone, Default)]
pub struct TypeSubstitution {
    mappings: HashMap<String, String>,
}

impl TypeSubstitution {
    pub fn new() -> Self {
        Self { mappings: HashMap::new() }
    }

    pub fn bind(&mut self, param: &str, concrete: &str) {
        self.mappings.insert(param.to_string(), concrete.to_string());
    }

    pub fn resolve(&self, ty: &str) -> String {
        self.mappings.get(ty).cloned().unwrap_or_else(|| ty.to_string())
    }

    pub fn is_bound(&self, param: &str) -> bool {
        self.mappings.contains_key(param)
    }

    pub fn all_bound(&self, params: &[TypeParam]) -> bool {
        params.iter().all(|p| self.is_bound(&p.name))
    }
}

impl fmt::Display for TypeSubstitution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pairs: Vec<String> = self.mappings.iter()
            .map(|(k, v)| format!("{} → {}", k, v))
            .collect();
        write!(f, "{{{}}}", pairs.join(", "))
    }
}

// ─── Monomorphizer ──────────────────────────────────────────────────────

/// Tracks which generic instantiations have been requested and generates
/// concrete specialized versions.
#[derive(Debug, Default)]
pub struct Monomorphizer {
    /// All known generic function signatures.
    generic_fns: HashMap<String, GenericSig>,
    /// All known generic struct signatures.
    generic_structs: HashMap<String, GenericSig>,
    /// Generated concrete function names → (original_name, substitution).
    instantiated_fns: HashMap<String, (String, TypeSubstitution)>,
    /// Generated concrete struct names → (original_name, substitution).
    instantiated_structs: HashMap<String, (String, TypeSubstitution)>,
}

impl Monomorphizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a generic function signature.
    pub fn register_generic_fn(&mut self, sig: GenericSig) {
        self.generic_fns.insert(sig.name.clone(), sig);
    }

    /// Register a generic struct signature.
    pub fn register_generic_struct(&mut self, sig: GenericSig) {
        self.generic_structs.insert(sig.name.clone(), sig);
    }

    /// Check if a function is generic.
    pub fn is_generic_fn(&self, name: &str) -> bool {
        self.generic_fns.contains_key(name)
    }

    /// Check if a struct is generic.
    pub fn is_generic_struct(&self, name: &str) -> bool {
        self.generic_structs.contains_key(name)
    }

    /// Request a concrete instantiation of a generic function.
    /// Returns the mangled name of the concrete version.
    pub fn instantiate_fn(&mut self, name: &str, concrete_types: &[String]) -> Option<String> {
        let sig = self.generic_fns.get(name)?.clone();
        if concrete_types.len() != sig.type_params.len() {
            return None;
        }

        let mangled = sig.mangle(concrete_types);
        if !self.instantiated_fns.contains_key(&mangled) {
            let mut sub = TypeSubstitution::new();
            for (param, concrete) in sig.type_params.iter().zip(concrete_types) {
                sub.bind(&param.name, concrete);
            }
            self.instantiated_fns.insert(mangled.clone(), (name.to_string(), sub));
        }
        Some(mangled)
    }

    /// Request a concrete instantiation of a generic struct.
    pub fn instantiate_struct(&mut self, name: &str, concrete_types: &[String]) -> Option<String> {
        let sig = self.generic_structs.get(name)?.clone();
        if concrete_types.len() != sig.type_params.len() {
            return None;
        }

        let mangled = sig.mangle(concrete_types);
        if !self.instantiated_structs.contains_key(&mangled) {
            let mut sub = TypeSubstitution::new();
            for (param, concrete) in sig.type_params.iter().zip(concrete_types) {
                sub.bind(&param.name, concrete);
            }
            self.instantiated_structs.insert(mangled.clone(), (name.to_string(), sub));
        }
        Some(mangled)
    }

    /// Get all instantiated function names with their substitutions.
    pub fn instantiated_functions(&self) -> &HashMap<String, (String, TypeSubstitution)> {
        &self.instantiated_fns
    }

    /// Get all instantiated struct names with their substitutions.
    pub fn instantiated_structs(&self) -> &HashMap<String, (String, TypeSubstitution)> {
        &self.instantiated_structs
    }

    /// Number of registered generic functions.
    pub fn generic_fn_count(&self) -> usize {
        self.generic_fns.len()
    }

    /// Number of generated concrete instantiations.
    pub fn instantiation_count(&self) -> usize {
        self.instantiated_fns.len() + self.instantiated_structs.len()
    }
}

// ─── Type Inference ─────────────────────────────────────────────────────

/// Infer type parameters from argument types at a call site.
pub fn infer_type_params(
    sig: &GenericSig,
    arg_types: &[String],
) -> Option<TypeSubstitution> {
    if arg_types.len() != sig.param_types.len() {
        return None;
    }

    let mut sub = TypeSubstitution::new();
    for (param_ty, arg_ty) in sig.param_types.iter().zip(arg_types) {
        if sig.is_type_param(param_ty) {
            if sub.is_bound(param_ty) {
                // Already bound — check consistency
                if sub.resolve(param_ty) != *arg_ty {
                    return None; // Conflicting types
                }
            } else {
                sub.bind(param_ty, arg_ty);
            }
        }
    }

    // Check all params are bound (or have defaults)
    for tp in &sig.type_params {
        if !sub.is_bound(&tp.name) {
            if let Some(ref default) = tp.default {
                sub.bind(&tp.name, default);
            } else {
                return None; // Cannot infer
            }
        }
    }

    Some(sub)
}

// ─── Built-in Trait Bounds ──────────────────────────────────────────────

/// Well-known trait bounds that the compiler understands.
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinBound {
    /// Type supports numeric operations (+, -, *, /)
    Numeric,
    /// Type supports equality comparison (==, !=)
    Eq,
    /// Type supports ordering comparison (<, >, <=, >=)
    Ord,
    /// Type can be displayed as a string
    Display,
    /// Type can be copied (all primitives)
    Copy,
    /// Type can be default-constructed
    Default,
}

impl BuiltinBound {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Numeric" => Some(BuiltinBound::Numeric),
            "Eq" => Some(BuiltinBound::Eq),
            "Ord" => Some(BuiltinBound::Ord),
            "Display" => Some(BuiltinBound::Display),
            "Copy" => Some(BuiltinBound::Copy),
            "Default" => Some(BuiltinBound::Default),
            _ => None,
        }
    }

    /// Check if a concrete type satisfies this bound.
    pub fn satisfied_by(&self, ty: &str) -> bool {
        match self {
            BuiltinBound::Numeric => matches!(ty, "i32" | "i64" | "f32" | "f64"),
            BuiltinBound::Eq => matches!(ty, "i32" | "i64" | "f32" | "f64" | "bool" | "str"),
            BuiltinBound::Ord => matches!(ty, "i32" | "i64" | "f32" | "f64"),
            BuiltinBound::Display => true, // All types can be displayed
            BuiltinBound::Copy => matches!(ty, "i32" | "i64" | "f32" | "f64" | "bool"),
            BuiltinBound::Default => matches!(ty, "i32" | "i64" | "f32" | "f64" | "bool"),
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_param_creation() {
        let tp = TypeParam::new("T").with_bound("Numeric");
        assert_eq!(tp.name, "T");
        assert!(tp.has_bound("Numeric"));
        assert!(!tp.has_bound("Eq"));
    }

    #[test]
    fn test_type_param_display() {
        let tp = TypeParam::new("T").with_bound("Numeric").with_bound("Eq");
        assert_eq!(format!("{}", tp), "T: Numeric + Eq");
    }

    #[test]
    fn test_type_param_default() {
        let tp = TypeParam::new("T").with_default("i64");
        assert_eq!(tp.default, Some("i64".to_string()));
        assert_eq!(format!("{}", tp), "T = i64");
    }

    #[test]
    fn test_generic_sig_mangle() {
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        assert_eq!(sig.mangle(&["i64".into()]), "identity_i64");
        assert_eq!(sig.mangle(&["f64".into()]), "identity_f64");
    }

    #[test]
    fn test_generic_sig_is_type_param() {
        let sig = GenericSig::new("swap", vec![
            TypeParam::new("A"),
            TypeParam::new("B"),
        ]);
        assert!(sig.is_type_param("A"));
        assert!(sig.is_type_param("B"));
        assert!(!sig.is_type_param("C"));
    }

    #[test]
    fn test_type_substitution() {
        let mut sub = TypeSubstitution::new();
        sub.bind("T", "i64");
        assert_eq!(sub.resolve("T"), "i64");
        assert_eq!(sub.resolve("U"), "U"); // Not bound, returns as-is
        assert!(sub.is_bound("T"));
        assert!(!sub.is_bound("U"));
    }

    #[test]
    fn test_monomorphizer_register() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);
        assert!(mono.is_generic_fn("identity"));
        assert!(!mono.is_generic_fn("unknown"));
        assert_eq!(mono.generic_fn_count(), 1);
    }

    #[test]
    fn test_monomorphizer_instantiate() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);

        let name = mono.instantiate_fn("identity", &["i64".into()]);
        assert_eq!(name, Some("identity_i64".to_string()));

        let name2 = mono.instantiate_fn("identity", &["f64".into()]);
        assert_eq!(name2, Some("identity_f64".to_string()));

        assert_eq!(mono.instantiation_count(), 2);
    }

    #[test]
    fn test_monomorphizer_dedup() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("id", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);

        mono.instantiate_fn("id", &["i64".into()]);
        mono.instantiate_fn("id", &["i64".into()]); // Duplicate
        assert_eq!(mono.instantiation_count(), 1); // Should not duplicate
    }

    #[test]
    fn test_monomorphizer_struct() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("Pair", vec![
            TypeParam::new("A"),
            TypeParam::new("B"),
        ]);
        mono.register_generic_struct(sig);

        let name = mono.instantiate_struct("Pair", &["i64".into(), "str".into()]);
        assert_eq!(name, Some("Pair_i64_str".to_string()));
        assert!(mono.is_generic_struct("Pair"));
    }

    #[test]
    fn test_monomorphizer_wrong_arity() {
        let mut mono = Monomorphizer::new();
        let sig = GenericSig::new("identity", vec![TypeParam::new("T")]);
        mono.register_generic_fn(sig);

        // Too many type args
        assert_eq!(mono.instantiate_fn("identity", &["i64".into(), "f64".into()]), None);
        // Unknown function
        assert_eq!(mono.instantiate_fn("unknown", &["i64".into()]), None);
    }

    #[test]
    fn test_type_inference() {
        let mut sig = GenericSig::new("add", vec![TypeParam::new("T")]);
        sig.param_types = vec!["T".into(), "T".into()];

        let result = infer_type_params(&sig, &["i64".into(), "i64".into()]);
        assert!(result.is_some());
        let sub = result.unwrap();
        assert_eq!(sub.resolve("T"), "i64");
    }

    #[test]
    fn test_type_inference_conflict() {
        let mut sig = GenericSig::new("add", vec![TypeParam::new("T")]);
        sig.param_types = vec!["T".into(), "T".into()];

        // i64 vs f64 → conflict
        let result = infer_type_params(&sig, &["i64".into(), "f64".into()]);
        assert!(result.is_none());
    }

    #[test]
    fn test_type_inference_default() {
        let mut sig = GenericSig::new("zero", vec![
            TypeParam::new("T").with_default("i64"),
        ]);
        sig.param_types = vec![];

        let result = infer_type_params(&sig, &[]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().resolve("T"), "i64");
    }

    #[test]
    fn test_builtin_bound_numeric() {
        assert!(BuiltinBound::Numeric.satisfied_by("i64"));
        assert!(BuiltinBound::Numeric.satisfied_by("f64"));
        assert!(!BuiltinBound::Numeric.satisfied_by("bool"));
        assert!(!BuiltinBound::Numeric.satisfied_by("str"));
    }

    #[test]
    fn test_builtin_bound_eq() {
        assert!(BuiltinBound::Eq.satisfied_by("i64"));
        assert!(BuiltinBound::Eq.satisfied_by("str"));
        assert!(BuiltinBound::Eq.satisfied_by("bool"));
    }

    #[test]
    fn test_builtin_bound_from_name() {
        assert_eq!(BuiltinBound::from_name("Numeric"), Some(BuiltinBound::Numeric));
        assert_eq!(BuiltinBound::from_name("Copy"), Some(BuiltinBound::Copy));
        assert_eq!(BuiltinBound::from_name("Unknown"), None);
    }

    #[test]
    fn test_substitution_display() {
        let mut sub = TypeSubstitution::new();
        sub.bind("T", "i64");
        let s = format!("{}", sub);
        assert!(s.contains("T → i64"));
    }

    #[test]
    fn test_sig_param_index() {
        let sig = GenericSig::new("f", vec![
            TypeParam::new("A"),
            TypeParam::new("B"),
        ]);
        assert_eq!(sig.type_param_index("A"), Some(0));
        assert_eq!(sig.type_param_index("B"), Some(1));
        assert_eq!(sig.type_param_index("C"), None);
    }
}
