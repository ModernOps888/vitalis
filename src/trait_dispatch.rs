//! Trait Dispatch — Full trait impl resolution with vtable-like dispatch
//!
//! Provides:
//! - Trait registry mapping trait names → method signatures
//! - Impl registry mapping (type, trait) → concrete implementations
//! - Method resolution: given a type and method name, find the impl
//! - Vtable construction for dynamic dispatch

use std::collections::HashMap;

/// A method signature in a trait
#[derive(Debug, Clone)]
pub struct TraitMethodSig {
    pub name: String,
    pub param_count: usize,
    pub has_self: bool,
    pub has_default: bool,
}

/// Registered trait definition
#[derive(Debug, Clone)]
pub struct TraitInfo {
    pub name: String,
    pub methods: Vec<TraitMethodSig>,
}

/// Concrete implementation of a trait for a type
#[derive(Debug, Clone)]
pub struct ImplInfo {
    pub type_name: String,
    pub trait_name: String,
    pub methods: Vec<String>,  // implemented method names
}

/// Virtual method table for dynamic dispatch
#[derive(Debug, Clone)]
pub struct VTable {
    pub trait_name: String,
    pub type_name: String,
    pub entries: Vec<VTableEntry>,
}

/// A single vtable entry
#[derive(Debug, Clone)]
pub struct VTableEntry {
    pub method_name: String,
    pub is_default: bool,
    pub slot: usize,
}

/// The trait dispatch registry
pub struct TraitDispatcher {
    traits: HashMap<String, TraitInfo>,
    impls: HashMap<(String, String), ImplInfo>,  // (type, trait) -> impl
    vtables: HashMap<(String, String), VTable>,
}

impl TraitDispatcher {
    pub fn new() -> Self {
        Self {
            traits: HashMap::new(),
            impls: HashMap::new(),
            vtables: HashMap::new(),
        }
    }

    /// Register a trait definition
    pub fn register_trait(&mut self, name: &str, methods: Vec<TraitMethodSig>) {
        self.traits.insert(name.to_string(), TraitInfo {
            name: name.to_string(),
            methods,
        });
    }

    /// Register a trait impl for a type
    pub fn register_impl(&mut self, type_name: &str, trait_name: &str, methods: Vec<String>) -> Result<(), String> {
        // Verify the trait exists
        let trait_info = self.traits.get(trait_name)
            .ok_or_else(|| format!("Unknown trait '{}'", trait_name))?;

        // Verify all required methods are implemented
        for method in &trait_info.methods {
            if !method.has_default && !methods.contains(&method.name) {
                return Err(format!(
                    "Type '{}' doesn't implement required method '{}' from trait '{}'",
                    type_name, method.name, trait_name
                ));
            }
        }

        let key = (type_name.to_string(), trait_name.to_string());
        self.impls.insert(key.clone(), ImplInfo {
            type_name: type_name.to_string(),
            trait_name: trait_name.to_string(),
            methods,
        });

        // Build vtable
        self.build_vtable(type_name, trait_name);

        Ok(())
    }

    fn build_vtable(&mut self, type_name: &str, trait_name: &str) {
        let trait_info = match self.traits.get(trait_name) {
            Some(t) => t.clone(),
            None => return,
        };
        let impl_info = match self.impls.get(&(type_name.to_string(), trait_name.to_string())) {
            Some(i) => i.clone(),
            None => return,
        };

        let entries: Vec<VTableEntry> = trait_info.methods.iter().enumerate()
            .map(|(slot, method)| {
                let is_default = !impl_info.methods.contains(&method.name);
                VTableEntry {
                    method_name: method.name.clone(),
                    is_default,
                    slot,
                }
            })
            .collect();

        self.vtables.insert(
            (type_name.to_string(), trait_name.to_string()),
            VTable {
                trait_name: trait_name.to_string(),
                type_name: type_name.to_string(),
                entries,
            },
        );
    }

    /// Resolve a method call: given a type and method name, find which trait provides it
    pub fn resolve_method(&self, type_name: &str, method_name: &str) -> Option<(&str, usize)> {
        for ((tn, trait_n), vtable) in &self.vtables {
            if tn == type_name {
                for entry in &vtable.entries {
                    if entry.method_name == method_name {
                        return Some((trait_n.as_str(), entry.slot));
                    }
                }
            }
        }
        None
    }

    /// Get vtable for a (type, trait) pair
    pub fn vtable_for(&self, type_name: &str, trait_name: &str) -> Option<&VTable> {
        self.vtables.get(&(type_name.to_string(), trait_name.to_string()))
    }

    /// Check if a type implements a trait
    pub fn implements(&self, type_name: &str, trait_name: &str) -> bool {
        self.impls.contains_key(&(type_name.to_string(), trait_name.to_string()))
    }

    /// Get all traits implemented by a type
    pub fn traits_of(&self, type_name: &str) -> Vec<&str> {
        self.impls.keys()
            .filter(|(tn, _)| tn == type_name)
            .map(|(_, tr)| tr.as_str())
            .collect()
    }

    /// Get all types that implement a trait
    pub fn implementors_of(&self, trait_name: &str) -> Vec<&str> {
        self.impls.keys()
            .filter(|(_, tr)| tr == trait_name)
            .map(|(tn, _)| tn.as_str())
            .collect()
    }

    /// Get all registered trait names
    pub fn all_traits(&self) -> Vec<&str> {
        self.traits.keys().map(|s| s.as_str()).collect()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_dispatcher() -> TraitDispatcher {
        let mut d = TraitDispatcher::new();
        d.register_trait("Display", vec![
            TraitMethodSig { name: "to_string".into(), param_count: 0, has_self: true, has_default: false },
        ]);
        d.register_trait("Debug", vec![
            TraitMethodSig { name: "debug_fmt".into(), param_count: 0, has_self: true, has_default: false },
        ]);
        d.register_trait("WithDefault", vec![
            TraitMethodSig { name: "required".into(), param_count: 0, has_self: true, has_default: false },
            TraitMethodSig { name: "optional".into(), param_count: 0, has_self: true, has_default: true },
        ]);
        d
    }

    #[test]
    fn test_dispatch_register_trait() {
        let d = make_dispatcher();
        assert_eq!(d.all_traits().len(), 3);
    }

    #[test]
    fn test_dispatch_register_impl() {
        let mut d = make_dispatcher();
        let result = d.register_impl("Point", "Display", vec!["to_string".into()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dispatch_missing_method() {
        let mut d = make_dispatcher();
        let result = d.register_impl("Point", "Display", vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("to_string"));
    }

    #[test]
    fn test_dispatch_unknown_trait() {
        let mut d = make_dispatcher();
        let result = d.register_impl("Point", "Nonexistent", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_dispatch_resolve_method() {
        let mut d = make_dispatcher();
        d.register_impl("Point", "Display", vec!["to_string".into()]).unwrap();
        let (trait_name, slot) = d.resolve_method("Point", "to_string").unwrap();
        assert_eq!(trait_name, "Display");
        assert_eq!(slot, 0);
    }

    #[test]
    fn test_dispatch_resolve_missing() {
        let d = make_dispatcher();
        assert!(d.resolve_method("Point", "whatever").is_none());
    }

    #[test]
    fn test_dispatch_implements() {
        let mut d = make_dispatcher();
        d.register_impl("Point", "Display", vec!["to_string".into()]).unwrap();
        assert!(d.implements("Point", "Display"));
        assert!(!d.implements("Point", "Debug"));
    }

    #[test]
    fn test_dispatch_traits_of() {
        let mut d = make_dispatcher();
        d.register_impl("Point", "Display", vec!["to_string".into()]).unwrap();
        d.register_impl("Point", "Debug", vec!["debug_fmt".into()]).unwrap();
        let traits = d.traits_of("Point");
        assert_eq!(traits.len(), 2);
    }

    #[test]
    fn test_dispatch_implementors_of() {
        let mut d = make_dispatcher();
        d.register_impl("Point", "Display", vec!["to_string".into()]).unwrap();
        d.register_impl("Circle", "Display", vec!["to_string".into()]).unwrap();
        let types = d.implementors_of("Display");
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn test_dispatch_vtable() {
        let mut d = make_dispatcher();
        d.register_impl("Point", "Display", vec!["to_string".into()]).unwrap();
        let vtable = d.vtable_for("Point", "Display").unwrap();
        assert_eq!(vtable.entries.len(), 1);
        assert_eq!(vtable.entries[0].method_name, "to_string");
        assert!(!vtable.entries[0].is_default);
    }

    #[test]
    fn test_dispatch_default_method() {
        let mut d = make_dispatcher();
        // Only implement the required method, not the optional one
        d.register_impl("Widget", "WithDefault", vec!["required".into()]).unwrap();
        let vtable = d.vtable_for("Widget", "WithDefault").unwrap();
        assert_eq!(vtable.entries.len(), 2);
        // required is not default
        assert!(!vtable.entries[0].is_default);
        // optional IS default (not in methods list)
        assert!(vtable.entries[1].is_default);
    }

    #[test]
    fn test_dispatch_vtable_slots() {
        let mut d = make_dispatcher();
        d.register_impl("Widget", "WithDefault", vec!["required".into()]).unwrap();
        let vtable = d.vtable_for("Widget", "WithDefault").unwrap();
        assert_eq!(vtable.entries[0].slot, 0);
        assert_eq!(vtable.entries[1].slot, 1);
    }

    #[test]
    fn test_dispatch_multiple_traits() {
        let mut d = make_dispatcher();
        d.register_impl("Rect", "Display", vec!["to_string".into()]).unwrap();
        d.register_impl("Rect", "Debug", vec!["debug_fmt".into()]).unwrap();

        let (t1, _) = d.resolve_method("Rect", "to_string").unwrap();
        assert_eq!(t1, "Display");
        let (t2, _) = d.resolve_method("Rect", "debug_fmt").unwrap();
        assert_eq!(t2, "Debug");
    }

    #[test]
    fn test_dispatch_no_vtable() {
        let d = make_dispatcher();
        assert!(d.vtable_for("Nonexistent", "Display").is_none());
    }

    #[test]
    fn test_dispatch_empty() {
        let d = TraitDispatcher::new();
        assert!(d.all_traits().is_empty());
        assert!(!d.implements("x", "y"));
    }

    #[test]
    fn test_dispatch_method_sig_fields() {
        let sig = TraitMethodSig {
            name: "foo".into(),
            param_count: 3,
            has_self: true,
            has_default: false,
        };
        assert_eq!(sig.name, "foo");
        assert_eq!(sig.param_count, 3);
        assert!(sig.has_self);
        assert!(!sig.has_default);
    }

    #[test]
    fn test_vtable_entry_fields() {
        let entry = VTableEntry {
            method_name: "bar".into(),
            is_default: true,
            slot: 7,
        };
        assert_eq!(entry.method_name, "bar");
        assert!(entry.is_default);
        assert_eq!(entry.slot, 7);
    }

    #[test]
    fn test_impl_info_fields() {
        let info = ImplInfo {
            type_name: "Vec".into(),
            trait_name: "Iterator".into(),
            methods: vec!["next".into(), "size_hint".into()],
        };
        assert_eq!(info.type_name, "Vec");
        assert_eq!(info.trait_name, "Iterator");
        assert_eq!(info.methods.len(), 2);
    }
}
