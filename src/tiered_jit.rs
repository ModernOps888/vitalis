//! Tiered JIT compilation for Vitalis.
//!
//! Implements a 3-tier compilation system:
//! - **Tier 0 (Interpreter)**: Immediate execution, no compile cost
//! - **Tier 1 (Baseline JIT)**: Fast compile, moderate code quality
//! - **Tier 2 (Optimizing JIT)**: Expensive compile, best code quality
//! - **Profile counters**: Call/loop counters driving tier promotion
//! - **On-Stack Replacement (OSR)**: Mid-execution tier transition
//! - **Deoptimization**: Speculative → safe fallback on guard failure
//! - **Speculative optimization**: Type specialization, inline caching

use std::collections::HashMap;

// ── Compilation Tiers ───────────────────────────────────────────────

/// Compilation tier levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Interpreter = 0,
    BaselineJit = 1,
    OptimizingJit = 2,
}

impl Tier {
    pub fn name(&self) -> &'static str {
        match self {
            Tier::Interpreter => "Interpreter",
            Tier::BaselineJit => "BaselineJIT",
            Tier::OptimizingJit => "OptimizingJIT",
        }
    }

    pub fn next(&self) -> Option<Tier> {
        match self {
            Tier::Interpreter => Some(Tier::BaselineJit),
            Tier::BaselineJit => Some(Tier::OptimizingJit),
            Tier::OptimizingJit => None,
        }
    }
}

// ── Profile Counters ────────────────────────────────────────────────

/// Per-function execution profile.
#[derive(Debug, Clone)]
pub struct Profile {
    pub call_count: u64,
    pub loop_iterations: u64,
    pub type_observations: HashMap<String, TypeObservation>,
    pub branch_history: Vec<BranchRecord>,
    pub deopt_count: u32,
    pub last_deopt_reason: Option<String>,
}

impl Profile {
    pub fn new() -> Self {
        Self {
            call_count: 0,
            loop_iterations: 0,
            type_observations: HashMap::new(),
            branch_history: Vec::new(),
            deopt_count: 0,
            last_deopt_reason: None,
        }
    }

    pub fn record_call(&mut self) {
        self.call_count += 1;
    }

    pub fn record_loop_iteration(&mut self, count: u64) {
        self.loop_iterations += count;
    }

    pub fn record_type(&mut self, site: &str, observed: TypeObservation) {
        self.type_observations.insert(site.to_string(), observed);
    }

    pub fn record_branch(&mut self, taken: bool) {
        self.branch_history.push(BranchRecord { taken });
        if self.branch_history.len() > 64 {
            self.branch_history.remove(0);
        }
    }

    pub fn branch_bias(&self) -> f64 {
        if self.branch_history.is_empty() {
            return 0.5;
        }
        let taken = self.branch_history.iter().filter(|b| b.taken).count();
        taken as f64 / self.branch_history.len() as f64
    }

    /// Check if this function is "hot" enough for tier promotion.
    pub fn is_hot(&self, thresholds: &TierThresholds) -> bool {
        self.call_count >= thresholds.call_threshold
            || self.loop_iterations >= thresholds.loop_threshold
    }
}

/// A type observed at a call site.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeObservation {
    Monomorphic(String),           // Always one type.
    Polymorphic(Vec<String>),      // 2-4 types.
    Megamorphic,                   // Too many types.
}

/// A branch execution record.
#[derive(Debug, Clone)]
pub struct BranchRecord {
    pub taken: bool,
}

// ── Tier Thresholds ─────────────────────────────────────────────────

/// Thresholds for tier promotion.
#[derive(Debug, Clone)]
pub struct TierThresholds {
    pub call_threshold: u64,
    pub loop_threshold: u64,
}

impl Default for TierThresholds {
    fn default() -> Self {
        Self {
            call_threshold: 1000,
            loop_threshold: 10_000,
        }
    }
}

// ── Compiled Function ───────────────────────────────────────────────

/// Represents a function at a particular tier.
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub name: String,
    pub tier: Tier,
    pub code_size: usize,
    pub compile_time_us: u64,
    pub guards: Vec<Guard>,
    pub osr_points: Vec<OsrPoint>,
    pub speculations: Vec<Speculation>,
}

impl CompiledFunction {
    pub fn new(name: &str, tier: Tier) -> Self {
        Self {
            name: name.to_string(),
            tier,
            code_size: 0,
            compile_time_us: 0,
            guards: Vec::new(),
            osr_points: Vec::new(),
            speculations: Vec::new(),
        }
    }
}

// ── Guards & Deoptimization ─────────────────────────────────────────

/// A guard that protects a speculative optimization.
#[derive(Debug, Clone)]
pub struct Guard {
    pub id: u32,
    pub kind: GuardKind,
    pub deopt_target: DeoptTarget,
    pub fail_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuardKind {
    TypeCheck(String),        // Expected type name.
    BoundsCheck(usize),       // Array index.
    NullCheck,
    OverflowCheck,
    ClassHierarchy(String),   // Expected class/struct.
}

/// Where to resume after deoptimization.
#[derive(Debug, Clone)]
pub struct DeoptTarget {
    pub tier: Tier,
    pub bytecode_offset: usize,
    pub frame_state: Vec<(String, i64)>, // variable → value
}

/// A speculative optimization.
#[derive(Debug, Clone)]
pub struct Speculation {
    pub id: u32,
    pub kind: SpeculationKind,
    pub guard_id: u32,
    pub success_count: u64,
    pub fail_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpeculationKind {
    TypeSpecialization(String),
    InlineCache(String),     // Cached target function.
    BranchPrediction(bool),  // Predicted direction.
    ConstantFold(i64),       // Speculated constant value.
    LoopBoundSpeculation(i64),
}

impl Speculation {
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.fail_count;
        if total == 0 { 0.0 } else { self.success_count as f64 / total as f64 }
    }

    pub fn should_invalidate(&self, threshold: f64) -> bool {
        self.success_rate() < threshold
    }
}

// ── On-Stack Replacement (OSR) ──────────────────────────────────────

/// An OSR entry point in compiled code.
#[derive(Debug, Clone)]
pub struct OsrPoint {
    pub id: u32,
    pub bytecode_offset: usize,
    pub loop_header: bool,
    pub entry_values: Vec<String>, // Variables live at this point.
}

/// OSR transition request.
#[derive(Debug, Clone)]
pub struct OsrTransition {
    pub from_tier: Tier,
    pub to_tier: Tier,
    pub function_name: String,
    pub osr_point_id: u32,
    pub live_values: Vec<(String, i64)>,
}

// ── Tiered JIT Compiler ─────────────────────────────────────────────

/// The tiered JIT compilation manager.
pub struct TieredJit {
    /// Function profiles.
    pub profiles: HashMap<String, Profile>,
    /// Compiled functions at each tier.
    pub compiled: HashMap<String, CompiledFunction>,
    /// Tier promotion thresholds.
    pub thresholds: TierThresholds,
    /// Deoptimization events.
    pub deopt_log: Vec<DeoptEvent>,
    /// OSR transition log.
    pub osr_log: Vec<OsrTransition>,
    /// Stats.
    pub stats: JitStats,
    /// Next guard ID.
    next_guard_id: u32,
    /// Next speculation ID.
    next_spec_id: u32,
}

/// JIT compilation statistics.
#[derive(Debug, Clone, Default)]
pub struct JitStats {
    pub interpretations: u64,
    pub baseline_compilations: u64,
    pub optimizing_compilations: u64,
    pub deopts: u64,
    pub osr_transitions: u64,
    pub guards_checked: u64,
    pub guards_failed: u64,
}

/// A deoptimization event.
#[derive(Debug, Clone)]
pub struct DeoptEvent {
    pub function: String,
    pub from_tier: Tier,
    pub to_tier: Tier,
    pub reason: String,
    pub guard_id: u32,
}

impl TieredJit {
    pub fn new(thresholds: TierThresholds) -> Self {
        Self {
            profiles: HashMap::new(),
            compiled: HashMap::new(),
            thresholds,
            deopt_log: Vec::new(),
            osr_log: Vec::new(),
            stats: JitStats::default(),
            next_guard_id: 1,
            next_spec_id: 1,
        }
    }

    /// Get or create a profile for a function.
    pub fn profile(&mut self, name: &str) -> &mut Profile {
        self.profiles.entry(name.to_string()).or_insert_with(Profile::new)
    }

    /// Record a function call and check for tier promotion.
    pub fn record_call(&mut self, name: &str) -> Option<Tier> {
        let profile = self.profiles.entry(name.to_string()).or_insert_with(Profile::new);
        profile.record_call();

        let current_tier = self.compiled.get(name).map(|c| c.tier).unwrap_or(Tier::Interpreter);
        
        if profile.is_hot(&self.thresholds) {
            current_tier.next()
        } else {
            None
        }
    }

    /// Compile a function at a given tier.
    pub fn compile(&mut self, name: &str, tier: Tier) -> CompiledFunction {
        let mut func = CompiledFunction::new(name, tier);

        match tier {
            Tier::Interpreter => {
                self.stats.interpretations += 1;
                func.code_size = 0;
                func.compile_time_us = 0;
            }
            Tier::BaselineJit => {
                self.stats.baseline_compilations += 1;
                func.code_size = 256; // simulated
                func.compile_time_us = 50;
                // Add OSR points at loop headers.
                func.osr_points.push(OsrPoint {
                    id: 1,
                    bytecode_offset: 0,
                    loop_header: true,
                    entry_values: vec!["i".to_string(), "sum".to_string()],
                });
            }
            Tier::OptimizingJit => {
                self.stats.optimizing_compilations += 1;
                func.code_size = 512;
                func.compile_time_us = 500;

                // Add speculative optimizations based on profile.
                if let Some(profile) = self.profiles.get(name) {
                    for (site, obs) in &profile.type_observations {
                        if let TypeObservation::Monomorphic(ty) = obs {
                            let guard_id = self.next_guard_id;
                            self.next_guard_id += 1;
                            func.guards.push(Guard {
                                id: guard_id,
                                kind: GuardKind::TypeCheck(ty.clone()),
                                deopt_target: DeoptTarget {
                                    tier: Tier::BaselineJit,
                                    bytecode_offset: 0,
                                    frame_state: Vec::new(),
                                },
                                fail_count: 0,
                            });
                            func.speculations.push(Speculation {
                                id: self.next_spec_id,
                                kind: SpeculationKind::TypeSpecialization(ty.clone()),
                                guard_id,
                                success_count: 0,
                                fail_count: 0,
                            });
                            self.next_spec_id += 1;
                        }
                    }

                    // Branch prediction speculation.
                    let bias = profile.branch_bias();
                    if bias > 0.8 || bias < 0.2 {
                        let predicted = bias > 0.5;
                        let guard_id = self.next_guard_id;
                        self.next_guard_id += 1;
                        func.guards.push(Guard {
                            id: guard_id,
                            kind: GuardKind::TypeCheck("branch".to_string()),
                            deopt_target: DeoptTarget {
                                tier: Tier::BaselineJit,
                                bytecode_offset: 0,
                                frame_state: Vec::new(),
                            },
                            fail_count: 0,
                        });
                        func.speculations.push(Speculation {
                            id: self.next_spec_id,
                            kind: SpeculationKind::BranchPrediction(predicted),
                            guard_id,
                            success_count: 0,
                            fail_count: 0,
                        });
                        self.next_spec_id += 1;
                    }
                }
            }
        }

        self.compiled.insert(name.to_string(), func.clone());
        func
    }

    /// Trigger deoptimization for a function.
    pub fn deoptimize(&mut self, name: &str, guard_id: u32, reason: &str) {
        let from_tier = self.compiled.get(name).map(|c| c.tier).unwrap_or(Tier::Interpreter);
        let to_tier = match from_tier {
            Tier::OptimizingJit => Tier::BaselineJit,
            Tier::BaselineJit => Tier::Interpreter,
            Tier::Interpreter => Tier::Interpreter,
        };

        self.deopt_log.push(DeoptEvent {
            function: name.to_string(),
            from_tier,
            to_tier,
            reason: reason.to_string(),
            guard_id,
        });

        self.stats.deopts += 1;
        self.stats.guards_failed += 1;

        if let Some(profile) = self.profiles.get_mut(name) {
            profile.deopt_count += 1;
            profile.last_deopt_reason = Some(reason.to_string());
        }

        // Recompile at lower tier.
        self.compile(name, to_tier);
    }

    /// Perform OSR transition.
    pub fn osr_transition(&mut self, transition: OsrTransition) {
        self.stats.osr_transitions += 1;
        self.compile(&transition.function_name, transition.to_tier);
        self.osr_log.push(transition);
    }

    /// Get the current tier for a function.
    pub fn current_tier(&self, name: &str) -> Tier {
        self.compiled.get(name).map(|c| c.tier).unwrap_or(Tier::Interpreter)
    }

    /// Get number of functions at each tier.
    pub fn tier_distribution(&self) -> HashMap<Tier, usize> {
        let mut dist = HashMap::new();
        for func in self.compiled.values() {
            *dist.entry(func.tier).or_insert(0) += 1;
        }
        dist
    }
}

// ── Inline Cache ────────────────────────────────────────────────────

/// An inline cache for polymorphic dispatch.
#[derive(Debug, Clone)]
pub struct InlineCache {
    pub site_id: u32,
    pub state: InlineCacheState,
    pub entries: Vec<InlineCacheEntry>,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InlineCacheState {
    Uninitialized,
    Monomorphic,
    Polymorphic,
    Megamorphic,
}

#[derive(Debug, Clone)]
pub struct InlineCacheEntry {
    pub type_name: String,
    pub target: String,
    pub hit_count: u64,
}

impl InlineCache {
    pub fn new(site_id: u32, max_entries: usize) -> Self {
        Self {
            site_id,
            state: InlineCacheState::Uninitialized,
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Look up a cached target for a type.
    pub fn lookup(&mut self, type_name: &str) -> Option<&str> {
        for entry in &mut self.entries {
            if entry.type_name == type_name {
                entry.hit_count += 1;
                return Some(&entry.target);
            }
        }
        None
    }

    /// Insert a new cache entry.
    pub fn insert(&mut self, type_name: &str, target: &str) {
        if self.entries.len() >= self.max_entries {
            self.state = InlineCacheState::Megamorphic;
            return;
        }

        self.entries.push(InlineCacheEntry {
            type_name: type_name.to_string(),
            target: target.to_string(),
            hit_count: 1,
        });

        self.state = match self.entries.len() {
            1 => InlineCacheState::Monomorphic,
            _ => InlineCacheState::Polymorphic,
        };
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_ordering() {
        assert!(Tier::Interpreter < Tier::BaselineJit);
        assert!(Tier::BaselineJit < Tier::OptimizingJit);
    }

    #[test]
    fn test_tier_next() {
        assert_eq!(Tier::Interpreter.next(), Some(Tier::BaselineJit));
        assert_eq!(Tier::BaselineJit.next(), Some(Tier::OptimizingJit));
        assert_eq!(Tier::OptimizingJit.next(), None);
    }

    #[test]
    fn test_profile_call_count() {
        let mut profile = Profile::new();
        profile.record_call();
        profile.record_call();
        assert_eq!(profile.call_count, 2);
    }

    #[test]
    fn test_profile_hotness() {
        let mut profile = Profile::new();
        let thresholds = TierThresholds { call_threshold: 5, loop_threshold: 100 };
        assert!(!profile.is_hot(&thresholds));
        for _ in 0..5 {
            profile.record_call();
        }
        assert!(profile.is_hot(&thresholds));
    }

    #[test]
    fn test_profile_branch_bias() {
        let mut profile = Profile::new();
        for _ in 0..8 { profile.record_branch(true); }
        for _ in 0..2 { profile.record_branch(false); }
        assert!(profile.branch_bias() > 0.7);
    }

    #[test]
    fn test_compile_interpreter() {
        let mut jit = TieredJit::new(TierThresholds::default());
        let func = jit.compile("test", Tier::Interpreter);
        assert_eq!(func.tier, Tier::Interpreter);
        assert_eq!(func.code_size, 0);
    }

    #[test]
    fn test_compile_baseline() {
        let mut jit = TieredJit::new(TierThresholds::default());
        let func = jit.compile("test", Tier::BaselineJit);
        assert_eq!(func.tier, Tier::BaselineJit);
        assert!(func.code_size > 0);
        assert!(!func.osr_points.is_empty());
    }

    #[test]
    fn test_compile_optimizing() {
        let mut jit = TieredJit::new(TierThresholds::default());
        jit.profile("hot_fn").record_type("arg0", TypeObservation::Monomorphic("i64".into()));
        let func = jit.compile("hot_fn", Tier::OptimizingJit);
        assert_eq!(func.tier, Tier::OptimizingJit);
        assert!(!func.guards.is_empty());
        assert!(!func.speculations.is_empty());
    }

    #[test]
    fn test_tier_promotion() {
        let mut jit = TieredJit::new(TierThresholds { call_threshold: 3, loop_threshold: 100 });
        assert!(jit.record_call("fn1").is_none());
        assert!(jit.record_call("fn1").is_none());
        let promotion = jit.record_call("fn1");
        assert_eq!(promotion, Some(Tier::BaselineJit));
    }

    #[test]
    fn test_deoptimization() {
        let mut jit = TieredJit::new(TierThresholds::default());
        jit.compile("fn1", Tier::OptimizingJit);
        jit.deoptimize("fn1", 1, "type guard failed");
        assert_eq!(jit.current_tier("fn1"), Tier::BaselineJit);
        assert_eq!(jit.stats.deopts, 1);
    }

    #[test]
    fn test_osr_transition() {
        let mut jit = TieredJit::new(TierThresholds::default());
        jit.compile("loop_fn", Tier::BaselineJit);
        jit.osr_transition(OsrTransition {
            from_tier: Tier::BaselineJit,
            to_tier: Tier::OptimizingJit,
            function_name: "loop_fn".to_string(),
            osr_point_id: 1,
            live_values: vec![("i".to_string(), 42)],
        });
        assert_eq!(jit.current_tier("loop_fn"), Tier::OptimizingJit);
        assert_eq!(jit.stats.osr_transitions, 1);
    }

    #[test]
    fn test_speculation_success_rate() {
        let spec = Speculation {
            id: 1,
            kind: SpeculationKind::TypeSpecialization("i64".into()),
            guard_id: 1,
            success_count: 90,
            fail_count: 10,
        };
        assert!((spec.success_rate() - 0.9).abs() < 0.01);
        assert!(!spec.should_invalidate(0.8));
    }

    #[test]
    fn test_inline_cache_monomorphic() {
        let mut ic = InlineCache::new(1, 4);
        ic.insert("Point", "Point::distance");
        assert_eq!(ic.state, InlineCacheState::Monomorphic);
        assert_eq!(ic.lookup("Point"), Some("Point::distance"));
    }

    #[test]
    fn test_inline_cache_polymorphic() {
        let mut ic = InlineCache::new(1, 4);
        ic.insert("Circle", "Circle::area");
        ic.insert("Square", "Square::area");
        assert_eq!(ic.state, InlineCacheState::Polymorphic);
    }

    #[test]
    fn test_inline_cache_megamorphic() {
        let mut ic = InlineCache::new(1, 2);
        ic.insert("A", "A::f");
        ic.insert("B", "B::f");
        ic.insert("C", "C::f"); // exceeds max
        assert_eq!(ic.state, InlineCacheState::Megamorphic);
    }

    #[test]
    fn test_tier_name() {
        assert_eq!(Tier::Interpreter.name(), "Interpreter");
        assert_eq!(Tier::BaselineJit.name(), "BaselineJIT");
        assert_eq!(Tier::OptimizingJit.name(), "OptimizingJIT");
    }

    #[test]
    fn test_tier_distribution() {
        let mut jit = TieredJit::new(TierThresholds::default());
        jit.compile("a", Tier::Interpreter);
        jit.compile("b", Tier::BaselineJit);
        jit.compile("c", Tier::OptimizingJit);
        let dist = jit.tier_distribution();
        assert_eq!(dist[&Tier::Interpreter], 1);
        assert_eq!(dist[&Tier::BaselineJit], 1);
        assert_eq!(dist[&Tier::OptimizingJit], 1);
    }

    #[test]
    fn test_guard_kinds() {
        let g1 = GuardKind::TypeCheck("i64".into());
        let g2 = GuardKind::BoundsCheck(42);
        let g3 = GuardKind::NullCheck;
        assert_ne!(g1, g2);
        assert_ne!(g2, GuardKind::NullCheck);
        assert_eq!(g3, GuardKind::NullCheck);
    }

    #[test]
    fn test_type_observation() {
        let mono = TypeObservation::Monomorphic("i64".into());
        let poly = TypeObservation::Polymorphic(vec!["i64".into(), "f64".into()]);
        let mega = TypeObservation::Megamorphic;
        assert_ne!(mono, poly);
        assert_ne!(poly, mega);
    }

    #[test]
    fn test_deopt_log() {
        let mut jit = TieredJit::new(TierThresholds::default());
        jit.compile("fn1", Tier::OptimizingJit);
        jit.deoptimize("fn1", 1, "overflow");
        assert_eq!(jit.deopt_log.len(), 1);
        assert_eq!(jit.deopt_log[0].reason, "overflow");
    }

    #[test]
    fn test_profile_loop_hot() {
        let mut profile = Profile::new();
        let thresholds = TierThresholds { call_threshold: 10000, loop_threshold: 50 };
        profile.record_loop_iteration(60);
        assert!(profile.is_hot(&thresholds));
    }
}
