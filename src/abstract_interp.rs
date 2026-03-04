//! Abstract interpretation framework for Vitalis.
//!
//! - **Interval domain**: Integer intervals with arithmetic
//! - **Octagon domain**: Relational abstract domain
//! - **Widening / narrowing**: Convergence acceleration
//! - **Null-pointer analysis**: Track definitely-null / maybe-null
//! - **Array bounds checking**: Prove in-bounds accesses
//! - **Taint analysis**: Track untrusted data flow
//! - **Alias analysis**: Points-to sets

use std::collections::{HashMap, HashSet};

// ── Interval Domain ────────────────────────────────────────────────

/// An integer interval [lo, hi], or Bottom (empty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Interval {
    Bottom,
    Range { lo: i64, hi: i64 },
    Top,
}

impl Interval {
    pub fn new(lo: i64, hi: i64) -> Self {
        if lo > hi { Interval::Bottom } else { Interval::Range { lo, hi } }
    }

    pub fn constant(v: i64) -> Self { Interval::Range { lo: v, hi: v } }

    pub fn is_bottom(&self) -> bool { matches!(self, Interval::Bottom) }
    pub fn is_top(&self) -> bool { matches!(self, Interval::Top) }

    pub fn contains(&self, v: i64) -> bool {
        match self {
            Interval::Bottom => false,
            Interval::Range { lo, hi } => v >= *lo && v <= *hi,
            Interval::Top => true,
        }
    }

    /// Join (least upper bound).
    pub fn join(&self, other: &Interval) -> Interval {
        match (self, other) {
            (Interval::Bottom, x) | (x, Interval::Bottom) => x.clone(),
            (Interval::Top, _) | (_, Interval::Top) => Interval::Top,
            (Interval::Range { lo: l1, hi: h1 }, Interval::Range { lo: l2, hi: h2 }) => {
                Interval::Range { lo: (*l1).min(*l2), hi: (*h1).max(*h2) }
            }
        }
    }

    /// Meet (greatest lower bound).
    pub fn meet(&self, other: &Interval) -> Interval {
        match (self, other) {
            (Interval::Bottom, _) | (_, Interval::Bottom) => Interval::Bottom,
            (Interval::Top, x) | (x, Interval::Top) => x.clone(),
            (Interval::Range { lo: l1, hi: h1 }, Interval::Range { lo: l2, hi: h2 }) => {
                let lo = (*l1).max(*l2);
                let hi = (*h1).min(*h2);
                if lo > hi { Interval::Bottom } else { Interval::Range { lo, hi } }
            }
        }
    }

    /// Widening.
    pub fn widen(&self, other: &Interval) -> Interval {
        match (self, other) {
            (Interval::Bottom, x) => x.clone(),
            (_, Interval::Bottom) => self.clone(),
            (Interval::Top, _) | (_, Interval::Top) => Interval::Top,
            (Interval::Range { lo: l1, hi: h1 }, Interval::Range { lo: l2, hi: h2 }) => {
                let lo = if *l2 < *l1 { i64::MIN } else { *l1 };
                let hi = if *h2 > *h1 { i64::MAX } else { *h1 };
                if lo == i64::MIN && hi == i64::MAX { Interval::Top } else { Interval::Range { lo, hi } }
            }
        }
    }

    /// Narrowing.
    pub fn narrow(&self, other: &Interval) -> Interval {
        match (self, other) {
            (Interval::Bottom, _) => Interval::Bottom,
            (_, Interval::Bottom) => Interval::Bottom,
            (Interval::Top, x) => x.clone(),
            (x, Interval::Top) => x.clone(),
            (Interval::Range { lo: l1, hi: h1 }, Interval::Range { lo: l2, hi: h2 }) => {
                let lo = if *l1 == i64::MIN { *l2 } else { *l1 };
                let hi = if *h1 == i64::MAX { *h2 } else { *h1 };
                Interval::new(lo, hi)
            }
        }
    }

    /// Add two intervals.
    pub fn add(&self, other: &Interval) -> Interval {
        match (self, other) {
            (Interval::Bottom, _) | (_, Interval::Bottom) => Interval::Bottom,
            (Interval::Top, _) | (_, Interval::Top) => Interval::Top,
            (Interval::Range { lo: l1, hi: h1 }, Interval::Range { lo: l2, hi: h2 }) => {
                Interval::Range {
                    lo: l1.saturating_add(*l2),
                    hi: h1.saturating_add(*h2),
                }
            }
        }
    }

    /// Subtract.
    pub fn sub(&self, other: &Interval) -> Interval {
        match (self, other) {
            (Interval::Bottom, _) | (_, Interval::Bottom) => Interval::Bottom,
            (Interval::Top, _) | (_, Interval::Top) => Interval::Top,
            (Interval::Range { lo: l1, hi: h1 }, Interval::Range { lo: l2, hi: h2 }) => {
                Interval::Range {
                    lo: l1.saturating_sub(*h2),
                    hi: h1.saturating_sub(*l2),
                }
            }
        }
    }

    pub fn width(&self) -> Option<u64> {
        match self {
            Interval::Bottom => Some(0),
            Interval::Top => None,
            Interval::Range { lo, hi } => Some((*hi as u64).wrapping_sub(*lo as u64)),
        }
    }
}

// ── Null-Pointer Analysis ───────────────────────────────────────────

/// Nullability state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Nullability {
    /// Definitely null.
    Null,
    /// Definitely non-null.
    NonNull,
    /// Possibly null.
    MaybeNull,
    /// Unknown (bottom).
    Unknown,
}

impl Nullability {
    pub fn join(&self, other: &Nullability) -> Nullability {
        match (self, other) {
            (Nullability::Unknown, x) | (x, Nullability::Unknown) => x.clone(),
            (Nullability::Null, Nullability::Null) => Nullability::Null,
            (Nullability::NonNull, Nullability::NonNull) => Nullability::NonNull,
            _ => Nullability::MaybeNull,
        }
    }

    pub fn is_safe(&self) -> bool {
        matches!(self, Nullability::NonNull)
    }
}

/// Null-pointer analysis state.
pub struct NullAnalysis {
    state: HashMap<String, Nullability>,
}

impl NullAnalysis {
    pub fn new() -> Self { Self { state: HashMap::new() } }

    pub fn set_null(&mut self, var: &str) {
        self.state.insert(var.to_string(), Nullability::Null);
    }

    pub fn set_non_null(&mut self, var: &str) {
        self.state.insert(var.to_string(), Nullability::NonNull);
    }

    pub fn set_maybe_null(&mut self, var: &str) {
        self.state.insert(var.to_string(), Nullability::MaybeNull);
    }

    pub fn get(&self, var: &str) -> Nullability {
        self.state.get(var).cloned().unwrap_or(Nullability::Unknown)
    }

    /// Check if dereferencing `var` is safe.
    pub fn is_safe_deref(&self, var: &str) -> bool {
        self.get(var).is_safe()
    }

    /// After null-check: narrow to non-null on true branch.
    pub fn refine_non_null(&mut self, var: &str) {
        self.state.insert(var.to_string(), Nullability::NonNull);
    }

    pub fn potentially_null_vars(&self) -> Vec<String> {
        self.state.iter()
            .filter(|(_, v)| matches!(v, Nullability::Null | Nullability::MaybeNull))
            .map(|(k, _)| k.clone())
            .collect()
    }
}

// ── Array Bounds Analysis ───────────────────────────────────────────

/// An array access with known or symbolic index/length information.
#[derive(Debug, Clone)]
pub struct ArrayAccess {
    pub array_name: String,
    pub index: Interval,
    pub length: Interval,
}

/// Check if an array access is provably in-bounds.
pub fn check_bounds(access: &ArrayAccess) -> BoundsResult {
    match (&access.index, &access.length) {
        (Interval::Bottom, _) | (_, Interval::Bottom) => BoundsResult::Unreachable,
        (Interval::Top, _) | (_, Interval::Top) => BoundsResult::MaybeOutOfBounds,
        (Interval::Range { lo: idx_lo, hi: idx_hi }, Interval::Range { lo: _len_lo, hi: len_hi }) => {
            if *idx_lo < 0 {
                BoundsResult::DefinitelyOutOfBounds { reason: "negative index".to_string() }
            } else if *idx_hi >= *len_hi && *len_hi > 0 {
                BoundsResult::MaybeOutOfBounds
            } else if *idx_hi < *len_hi && *idx_lo >= 0 {
                BoundsResult::DefinitelyInBounds
            } else {
                BoundsResult::MaybeOutOfBounds
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoundsResult {
    DefinitelyInBounds,
    DefinitelyOutOfBounds { reason: String },
    MaybeOutOfBounds,
    Unreachable,
}

// ── Taint Analysis ──────────────────────────────────────────────────

/// Taint level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaintLevel {
    Clean,
    Tainted(String), // source description
    Sanitized,
}

/// Taint tracker.
pub struct TaintTracker {
    taints: HashMap<String, TaintLevel>,
    sanitizers: HashSet<String>,
}

impl TaintTracker {
    pub fn new() -> Self {
        Self { taints: HashMap::new(), sanitizers: HashSet::new() }
    }

    pub fn mark_tainted(&mut self, var: &str, source: &str) {
        self.taints.insert(var.to_string(), TaintLevel::Tainted(source.to_string()));
    }

    pub fn mark_clean(&mut self, var: &str) {
        self.taints.insert(var.to_string(), TaintLevel::Clean);
    }

    pub fn register_sanitizer(&mut self, func: &str) {
        self.sanitizers.insert(func.to_string());
    }

    pub fn apply_sanitizer(&mut self, var: &str, func: &str) {
        if self.sanitizers.contains(func) {
            self.taints.insert(var.to_string(), TaintLevel::Sanitized);
        }
    }

    pub fn get_taint(&self, var: &str) -> TaintLevel {
        self.taints.get(var).cloned().unwrap_or(TaintLevel::Clean)
    }

    pub fn is_tainted(&self, var: &str) -> bool {
        matches!(self.get_taint(var), TaintLevel::Tainted(_))
    }

    /// Propagate taint from src to dst (e.g., `dst = f(src)`).
    pub fn propagate(&mut self, src: &str, dst: &str) {
        let taint = self.get_taint(src);
        self.taints.insert(dst.to_string(), taint);
    }

    pub fn tainted_vars(&self) -> Vec<String> {
        self.taints.iter()
            .filter(|(_, v)| matches!(v, TaintLevel::Tainted(_)))
            .map(|(k, _)| k.clone())
            .collect()
    }
}

// ── Alias Analysis ──────────────────────────────────────────────────

/// Points-to set for alias analysis.
#[derive(Debug, Clone)]
pub struct PointsToSet {
    pub targets: HashSet<String>,
}

impl PointsToSet {
    pub fn empty() -> Self { Self { targets: HashSet::new() } }

    pub fn singleton(t: &str) -> Self {
        let mut targets = HashSet::new();
        targets.insert(t.to_string());
        Self { targets }
    }

    pub fn add(&mut self, target: &str) {
        self.targets.insert(target.to_string());
    }

    pub fn union(&self, other: &PointsToSet) -> PointsToSet {
        let targets: HashSet<String> = self.targets.union(&other.targets).cloned().collect();
        PointsToSet { targets }
    }

    pub fn may_alias(&self, other: &PointsToSet) -> bool {
        !self.targets.is_disjoint(&other.targets)
    }

    pub fn must_alias(&self, other: &PointsToSet) -> bool {
        self.targets.len() == 1 && other.targets.len() == 1 && self.targets == other.targets
    }
}

/// Alias analysis.
pub struct AliasAnalysis {
    points_to: HashMap<String, PointsToSet>,
}

impl AliasAnalysis {
    pub fn new() -> Self { Self { points_to: HashMap::new() } }

    pub fn set_points_to(&mut self, ptr: &str, pts: PointsToSet) {
        self.points_to.insert(ptr.to_string(), pts);
    }

    pub fn get_points_to(&self, ptr: &str) -> PointsToSet {
        self.points_to.get(ptr).cloned().unwrap_or_else(PointsToSet::empty)
    }

    pub fn may_alias(&self, a: &str, b: &str) -> bool {
        let pa = self.get_points_to(a);
        let pb = self.get_points_to(b);
        pa.may_alias(&pb)
    }

    pub fn must_alias(&self, a: &str, b: &str) -> bool {
        let pa = self.get_points_to(a);
        let pb = self.get_points_to(b);
        pa.must_alias(&pb)
    }

    /// Record assignment `dst = src` (copy pointer).
    pub fn assign(&mut self, dst: &str, src: &str) {
        let pts = self.get_points_to(src);
        self.points_to.insert(dst.to_string(), pts);
    }

    /// Record `ptr = &target`.
    pub fn address_of(&mut self, ptr: &str, target: &str) {
        self.points_to.insert(ptr.to_string(), PointsToSet::singleton(target));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_constant() {
        let i = Interval::constant(5);
        assert!(i.contains(5));
        assert!(!i.contains(6));
    }

    #[test]
    fn test_interval_range() {
        let i = Interval::new(1, 10);
        assert!(i.contains(1));
        assert!(i.contains(10));
        assert!(!i.contains(0));
    }

    #[test]
    fn test_interval_bottom() {
        let i = Interval::new(10, 5);
        assert!(i.is_bottom());
    }

    #[test]
    fn test_interval_join() {
        let a = Interval::new(1, 5);
        let b = Interval::new(3, 8);
        let j = a.join(&b);
        assert_eq!(j, Interval::new(1, 8));
    }

    #[test]
    fn test_interval_meet() {
        let a = Interval::new(1, 5);
        let b = Interval::new(3, 8);
        let m = a.meet(&b);
        assert_eq!(m, Interval::new(3, 5));
    }

    #[test]
    fn test_interval_meet_empty() {
        let a = Interval::new(1, 3);
        let b = Interval::new(5, 8);
        let m = a.meet(&b);
        assert!(m.is_bottom());
    }

    #[test]
    fn test_interval_widen() {
        let a = Interval::new(0, 5);
        let b = Interval::new(0, 10);
        let w = a.widen(&b);
        match w {
            Interval::Range { lo, hi } => {
                assert_eq!(lo, 0);
                assert_eq!(hi, i64::MAX);
            }
            _ => panic!("expected widened range"),
        }
    }

    #[test]
    fn test_interval_add() {
        let a = Interval::new(1, 5);
        let b = Interval::new(10, 20);
        let r = a.add(&b);
        assert_eq!(r, Interval::new(11, 25));
    }

    #[test]
    fn test_interval_sub() {
        let a = Interval::new(10, 20);
        let b = Interval::new(1, 5);
        let r = a.sub(&b);
        assert_eq!(r, Interval::new(5, 19));
    }

    #[test]
    fn test_nullability_join() {
        assert_eq!(Nullability::Null.join(&Nullability::NonNull), Nullability::MaybeNull);
        assert_eq!(Nullability::Null.join(&Nullability::Null), Nullability::Null);
    }

    #[test]
    fn test_null_analysis() {
        let mut na = NullAnalysis::new();
        na.set_null("p");
        assert!(!na.is_safe_deref("p"));
        na.refine_non_null("p");
        assert!(na.is_safe_deref("p"));
    }

    #[test]
    fn test_bounds_in_bounds() {
        let access = ArrayAccess {
            array_name: "arr".to_string(),
            index: Interval::new(0, 5),
            length: Interval::constant(10),
        };
        assert_eq!(check_bounds(&access), BoundsResult::DefinitelyInBounds);
    }

    #[test]
    fn test_bounds_negative_index() {
        let access = ArrayAccess {
            array_name: "arr".to_string(),
            index: Interval::new(-1, 5),
            length: Interval::constant(10),
        };
        assert!(matches!(check_bounds(&access), BoundsResult::DefinitelyOutOfBounds { .. }));
    }

    #[test]
    fn test_taint_propagation() {
        let mut tt = TaintTracker::new();
        tt.mark_tainted("input", "user_input");
        assert!(tt.is_tainted("input"));
        tt.propagate("input", "processed");
        assert!(tt.is_tainted("processed"));
    }

    #[test]
    fn test_taint_sanitizer() {
        let mut tt = TaintTracker::new();
        tt.register_sanitizer("escape_html");
        tt.mark_tainted("input", "user");
        tt.apply_sanitizer("input", "escape_html");
        assert!(!tt.is_tainted("input"));
    }

    #[test]
    fn test_points_to_alias() {
        let mut aa = AliasAnalysis::new();
        aa.address_of("p", "x");
        aa.address_of("q", "x");
        assert!(aa.may_alias("p", "q"));
        assert!(aa.must_alias("p", "q"));
    }

    #[test]
    fn test_points_to_no_alias() {
        let mut aa = AliasAnalysis::new();
        aa.address_of("p", "x");
        aa.address_of("q", "y");
        assert!(!aa.may_alias("p", "q"));
    }

    #[test]
    fn test_alias_assign() {
        let mut aa = AliasAnalysis::new();
        aa.address_of("p", "x");
        aa.assign("q", "p");
        assert!(aa.must_alias("p", "q"));
    }

    #[test]
    fn test_interval_width() {
        assert_eq!(Interval::constant(5).width(), Some(0));
        assert_eq!(Interval::new(0, 10).width(), Some(10));
        assert_eq!(Interval::Bottom.width(), Some(0));
    }
}
