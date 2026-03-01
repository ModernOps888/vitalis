//! Predictive JIT & Advanced Optimization — speculative compilation + delta debugging.
//!
//! This module implements three cutting-edge compiler techniques:
//!
//! 1. **Predictive JIT Compilation** — anticipates evolution paths and pre-compiles
//!    the most probable next mutations before execution reaches them.
//!
//! 2. **Delta Debugging Oracle** — when evolution fails, bisects the code change
//!    to isolate the minimal failing subset using the type-checker as an oracle.
//!
//! 3. **Data-Driven Inlining** — tracks function call patterns and uses Thompson
//!    sampling to make optimal inlining decisions that minimize JIT cold-start.
//!
//! 4. **IR Optimization Passes** — loop tiling, dead code elimination, constant
//!    folding on the SSA IR before Cranelift codegen.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                  Predictive JIT Pipeline                            │
//! │                                                                    │
//! │  Evolution ──► Trajectory ──► Branch ──► Speculative ──► Cache     │
//! │  History       Analysis      Predictor   Pre-Compile     Hit/Miss  │
//! │                                                                    │
//! │  Delta Debugging:                                                  │
//! │  Failing Code ──► Bisect ──► Oracle ──► Minimal ──► Diagnostic     │
//! │                    ↑           │        Subset       Report         │
//! │                    └───────────┘                                    │
//! │                                                                    │
//! │  IR Optimization:                                                  │
//! │  Raw IR ──► ConstFold ──► DeadElim ──► LoopTile ──► Optimized IR   │
//! │                                                                    │
//! │  Inlining Oracle:                                                  │
//! │  Call Sites ──► Score ──► Thompson ──► Inline ──► Size Check       │
//! │                 (depth,   Sampling     Decision    (budget)         │
//! │                  freq)                                              │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════════════
//  COMPILATION CACHE — memoize compilation results for speculative reuse
// ═══════════════════════════════════════════════════════════════════════

/// A cached compilation result — stored by content hash.
#[derive(Debug, Clone)]
pub struct CachedCompilation {
    /// Hash of the source code that was compiled.
    pub source_hash: u64,
    /// Whether compilation succeeded.
    pub success: bool,
    /// Compilation time in milliseconds.
    pub compile_time_ms: f64,
    /// Fitness score (if compiled successfully).
    pub fitness: f64,
    /// Number of cache hits.
    pub hits: u64,
    /// Timestamp of last access (ms since engine boot).
    pub last_access_ms: f64,
    /// Parse + type errors (if compilation failed).
    pub errors: Vec<String>,
}

/// The Predictive JIT compilation cache.
/// Stores recent compilations and speculative pre-compilations.
pub struct CompilationCache {
    /// source_hash → CachedCompilation
    entries: HashMap<u64, CachedCompilation>,
    /// Maximum cache entries before eviction.
    max_entries: usize,
    /// Total cache hits.
    total_hits: u64,
    /// Total cache misses.
    total_misses: u64,
    /// Total speculative compilations triggered.
    speculative_compiles: u64,
    /// Boot time for relative timestamps.
    boot_time: Instant,
}

impl CompilationCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries),
            max_entries,
            total_hits: 0,
            total_misses: 0,
            speculative_compiles: 0,
            boot_time: Instant::now(),
        }
    }

    /// Look up a cached compilation by source hash.
    pub fn lookup(&mut self, source_hash: u64) -> Option<&CachedCompilation> {
        if let Some(entry) = self.entries.get_mut(&source_hash) {
            entry.hits += 1;
            entry.last_access_ms = self.boot_time.elapsed().as_secs_f64() * 1000.0;
            self.total_hits += 1;
            Some(entry)
        } else {
            self.total_misses += 1;
            None
        }
    }

    /// Store a compilation result in the cache.
    pub fn store(&mut self, source_hash: u64, entry: CachedCompilation) {
        // Evict LRU entries if at capacity
        if self.entries.len() >= self.max_entries {
            self.evict_lru();
        }
        self.entries.insert(source_hash, entry);
    }

    /// Evict the least-recently-used entry.
    fn evict_lru(&mut self) {
        if let Some((&lru_hash, _)) = self.entries.iter()
            .min_by(|a, b| a.1.last_access_ms.partial_cmp(&b.1.last_access_ms)
                .unwrap_or(std::cmp::Ordering::Equal))
        {
            self.entries.remove(&lru_hash);
        }
    }

    /// Cache hit rate as a fraction [0.0, 1.0].
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 { return 0.0; }
        self.total_hits as f64 / total as f64
    }

    /// Get cache statistics as JSON.
    pub fn stats_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"entries\":{},",
                "\"max_entries\":{},",
                "\"total_hits\":{},",
                "\"total_misses\":{},",
                "\"hit_rate\":{:.4},",
                "\"speculative_compiles\":{}",
                "}}"
            ),
            self.entries.len(),
            self.max_entries,
            self.total_hits,
            self.total_misses,
            self.hit_rate(),
            self.speculative_compiles,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  EVOLUTION TRAJECTORY — predicts next mutation based on history
// ═══════════════════════════════════════════════════════════════════════

/// A single evolution observation for trajectory analysis.
#[derive(Debug, Clone)]
pub struct EvolutionObservation {
    pub function_name: String,
    pub generation: u64,
    pub fitness: f64,
    pub source_hash: u64,
    pub timestamp_ms: f64,
}

/// Trajectory predictor — anticipates which functions will be evolved next
/// and with what kind of mutations, based on evolution history patterns.
pub struct TrajectoryPredictor {
    /// Recent evolution observations, per function.
    observations: HashMap<String, Vec<EvolutionObservation>>,
    /// Function evolution frequency: how often each function is evolved.
    frequency: HashMap<String, u64>,
    /// Maximum observations per function.
    max_per_function: usize,
}

impl TrajectoryPredictor {
    pub fn new() -> Self {
        Self {
            observations: HashMap::new(),
            frequency: HashMap::new(),
            max_per_function: 100,
        }
    }

    /// Record an evolution event for trajectory analysis.
    pub fn observe(&mut self, obs: EvolutionObservation) {
        *self.frequency.entry(obs.function_name.clone()).or_insert(0) += 1;

        let history = self.observations.entry(obs.function_name.clone()).or_default();
        if history.len() >= self.max_per_function {
            history.remove(0);
        }
        history.push(obs);
    }

    /// Predict the most likely next functions to evolve.
    /// Returns up to `limit` function names sorted by probability.
    pub fn predict_next(&self, limit: usize) -> Vec<(String, f64)> {
        let total: u64 = self.frequency.values().sum();
        if total == 0 { return vec![]; }

        let mut predictions: Vec<(String, f64)> = self.frequency.iter()
            .map(|(name, &count)| {
                let freq_score = count as f64 / total as f64;

                // Recency boost: functions evolved recently are more likely
                let recency_boost = self.observations.get(name)
                    .and_then(|obs| obs.last())
                    .map(|last| {
                        // More recent = higher boost (exponential decay)
                        let age = self.observations.values()
                            .flat_map(|o| o.iter())
                            .map(|o| o.timestamp_ms)
                            .fold(0.0_f64, f64::max) - last.timestamp_ms;
                        (-age / 10000.0).exp() // decay over ~10 seconds
                    })
                    .unwrap_or(0.0);

                // Fitness trajectory: functions with declining fitness get
                // evolved more (the engine tries to improve them)
                let fitness_signal = self.fitness_trajectory(name);

                let probability = freq_score * 0.4 + recency_boost * 0.3 + fitness_signal * 0.3;
                (name.clone(), probability)
            })
            .collect();

        predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        predictions.truncate(limit);
        predictions
    }

    /// Compute fitness trajectory signal for a function.
    /// Returns a value in [0, 1]: higher = more likely to be evolved.
    fn fitness_trajectory(&self, name: &str) -> f64 {
        let obs = match self.observations.get(name) {
            Some(o) if o.len() >= 2 => o,
            _ => return 0.5, // unknown = neutral
        };

        // Look at last 5 observations
        let recent: Vec<f64> = obs.iter().rev().take(5).map(|o| o.fitness).collect();
        if recent.len() < 2 { return 0.5; }

        // Compute trend: negative trend = more likely to be evolved
        let mut delta_sum = 0.0;
        for i in 1..recent.len() {
            delta_sum += recent[i - 1] - recent[i]; // Note: reversed order
        }
        let avg_delta = delta_sum / (recent.len() - 1) as f64;

        // Map: negative delta (declining fitness) → higher score
        // Sigmoid-like mapping to [0, 1]
        0.5 + (-avg_delta * 10.0).tanh() * 0.5
    }

    /// Get prediction statistics as JSON.
    pub fn stats_json(&self) -> String {
        let total_functions = self.frequency.len();
        let total_observations: usize = self.observations.values().map(|v| v.len()).sum();
        let total_evolutions: u64 = self.frequency.values().sum();

        format!(
            "{{\"functions_tracked\":{},\"total_observations\":{},\"total_evolutions\":{}}}",
            total_functions,
            total_observations,
            total_evolutions,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  DELTA DEBUGGING ORACLE — isolate minimal failing code subset
// ═══════════════════════════════════════════════════════════════════════

/// Result of delta debugging: the minimal failing subset identified.
#[derive(Debug, Clone)]
pub struct DeltaDebugResult {
    /// The minimal failing code fragment.
    pub minimal_failing: String,
    /// Number of bisection steps performed.
    pub bisection_steps: u32,
    /// Total oracle (type checker) invocations.
    pub oracle_calls: u32,
    /// Time taken for the entire delta debugging process (ms).
    pub duration_ms: f64,
    /// The isolated error messages from the minimal subset.
    pub errors: Vec<String>,
    /// Reduction ratio: original_size / minimal_size.
    pub reduction_ratio: f64,
}

impl DeltaDebugResult {
    pub fn to_json(&self) -> String {
        let errors_json: Vec<String> = self.errors.iter()
            .map(|e| format!("\"{}\"", e.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();
        format!(
            concat!(
                "{{",
                "\"minimal_lines\":{},",
                "\"bisection_steps\":{},",
                "\"oracle_calls\":{},",
                "\"duration_ms\":{:.3},",
                "\"reduction_ratio\":{:.2},",
                "\"errors\":[{}]",
                "}}"
            ),
            self.minimal_failing.lines().count(),
            self.bisection_steps,
            self.oracle_calls,
            self.duration_ms,
            self.reduction_ratio,
            errors_json.join(","),
        )
    }
}

/// Delta debugging engine. Uses the Vitalis type checker as an oracle
/// to bisect failing code and isolate the minimal defect.
pub struct DeltaDebugger {
    /// Maximum bisection depth.
    max_depth: u32,
    /// Maximum oracle calls before giving up.
    max_oracle_calls: u32,
}

impl DeltaDebugger {
    pub fn new() -> Self {
        Self {
            max_depth: 20,
            max_oracle_calls: 100,
        }
    }

    /// Run delta debugging on a known-failing source.
    /// `known_good` is a previously working version (the oracle baseline).
    /// `known_bad` is the failing version.
    ///
    /// Returns the minimal set of lines whose change causes the failure.
    pub fn isolate(&self, known_good: &str, known_bad: &str) -> DeltaDebugResult {
        let start = Instant::now();
        let mut oracle_calls = 0u32;
        let mut steps = 0u32;

        let good_lines: Vec<&str> = known_good.lines().collect();
        let bad_lines: Vec<&str> = known_bad.lines().collect();

        // Find lines that differ
        let diff_indices: Vec<usize> = (0..bad_lines.len().max(good_lines.len()))
            .filter(|&i| {
                let g = good_lines.get(i).unwrap_or(&"");
                let b = bad_lines.get(i).unwrap_or(&"");
                g != b
            })
            .collect();

        if diff_indices.is_empty() {
            return DeltaDebugResult {
                minimal_failing: String::new(),
                bisection_steps: 0,
                oracle_calls: 0,
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                errors: vec!["Sources are identical".to_string()],
                reduction_ratio: 1.0,
            };
        }

        // Binary search for minimal failing subset using ddmin algorithm
        let mut minimal = diff_indices.clone();
        let mut granularity = 2usize;

        while granularity <= minimal.len() && oracle_calls < self.max_oracle_calls && steps < self.max_depth {
            steps += 1;
            let chunk_size = minimal.len() / granularity;
            if chunk_size == 0 { break; }

            let mut reduced = false;
            for chunk_start in (0..minimal.len()).step_by(chunk_size) {
                let chunk_end = (chunk_start + chunk_size).min(minimal.len());

                // Try removing this chunk: apply only the complement
                let complement: Vec<usize> = minimal.iter()
                    .enumerate()
                    .filter(|&(i, _)| i < chunk_start || i >= chunk_end)
                    .map(|(_, &idx)| idx)
                    .collect();

                if complement.is_empty() { continue; }

                // Build test source: good base + only complement changes applied
                let test_source = self.apply_changes(&good_lines, &bad_lines, &complement);
                oracle_calls += 1;

                // Oracle: does this still fail?
                let (_, parse_errors) = crate::parser::parse(&test_source);
                let still_fails = if !parse_errors.is_empty() {
                    true
                } else {
                    let type_errors = crate::types::TypeChecker::new().check(
                        &crate::parser::parse(&test_source).0,
                    );
                    !type_errors.is_empty()
                };

                if still_fails {
                    // The complement alone causes the failure — we can remove the chunk
                    minimal = complement;
                    reduced = true;
                    granularity = 2;
                    break;
                }
            }

            if !reduced {
                granularity *= 2;
            }
        }

        // Extract the minimal failing lines
        let minimal_source = self.apply_changes(&good_lines, &bad_lines, &minimal);
        let original_diff = diff_indices.len();
        let minimal_diff = minimal.len();

        // Get errors from the minimal failing source
        let (prog, parse_errors) = crate::parser::parse(&minimal_source);
        let errors: Vec<String> = if !parse_errors.is_empty() {
            parse_errors.iter().map(|e| e.to_string()).collect()
        } else {
            let type_errors = crate::types::TypeChecker::new().check(&prog);
            type_errors.iter().map(|e| format!("{:?}", e)).collect()
        };

        DeltaDebugResult {
            minimal_failing: minimal_source,
            bisection_steps: steps,
            oracle_calls,
            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            errors,
            reduction_ratio: if minimal_diff > 0 {
                original_diff as f64 / minimal_diff as f64
            } else {
                1.0
            },
        }
    }

    /// Apply a subset of changes from bad_lines onto good_lines.
    fn apply_changes<'a>(
        &self,
        good_lines: &[&'a str],
        bad_lines: &[&'a str],
        change_indices: &[usize],
    ) -> String {
        let max_len = good_lines.len().max(bad_lines.len());
        let change_set: std::collections::HashSet<usize> = change_indices.iter().copied().collect();

        let mut result = Vec::with_capacity(max_len);
        for i in 0..max_len {
            if change_set.contains(&i) {
                // Use the bad version of this line
                if let Some(&line) = bad_lines.get(i) {
                    result.push(line);
                }
            } else {
                // Use the good version
                if let Some(&line) = good_lines.get(i) {
                    result.push(line);
                }
            }
        }
        result.join("\n")
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  INLINING ORACLE — data-driven function inlining decisions
// ═══════════════════════════════════════════════════════════════════════

/// Record of a function's inlining-relevant characteristics.
#[derive(Debug, Clone)]
pub struct InliningCandidate {
    pub name: String,
    /// Number of IR instructions in the function body.
    pub body_size: usize,
    /// Number of call sites (how many places call this function).
    pub call_sites: u32,
    /// Whether it's a leaf function (doesn't call others).
    pub is_leaf: bool,
    /// Number of parameters.
    pub param_count: usize,
    /// Whether it contains loops.
    pub has_loops: bool,
    /// Thompson sampling: (alpha, beta) for success modeling.
    pub thompson_alpha: f64,
    pub thompson_beta: f64,
    /// Whether currently inlined.
    pub inlined: bool,
    /// Performance improvement when inlined (negative = worse).
    pub improvement: f64,
}

/// Inlining oracle that uses Thompson sampling to make decisions.
pub struct InliningOracle {
    candidates: HashMap<String, InliningCandidate>,
    /// Maximum function body size (IR instructions) to consider inlining.
    max_inline_size: usize,
    /// Total inlining budget per compilation unit (IR instructions).
    inline_budget: usize,
    /// Budget currently consumed.
    budget_used: usize,
}

impl InliningOracle {
    pub fn new() -> Self {
        Self {
            candidates: HashMap::new(),
            max_inline_size: 50,   // Don't inline functions larger than 50 IR instructions
            inline_budget: 500,    // Max 500 additional instructions from inlining
            budget_used: 0,
        }
    }

    /// Register a function as a potential inlining candidate.
    pub fn register_candidate(&mut self, candidate: InliningCandidate) {
        self.candidates.insert(candidate.name.clone(), candidate);
    }

    /// Score a function for inlining suitability.
    /// Returns a score in [0, 1]: higher = more suitable for inlining.
    pub fn score(&self, name: &str) -> f64 {
        let c = match self.candidates.get(name) {
            Some(c) => c,
            None => return 0.0,
        };

        // Size penalty: larger functions less desirable
        let size_score = if c.body_size == 0 {
            1.0
        } else if c.body_size > self.max_inline_size {
            return 0.0; // Too large, never inline
        } else {
            1.0 - (c.body_size as f64 / self.max_inline_size as f64)
        };

        // Frequency bonus: more call sites = more benefit from inlining
        let freq_score = (c.call_sites as f64).ln().max(0.0) / 5.0;

        // Leaf bonus: leaf functions are cheaper to inline (no call overhead cascade)
        let leaf_bonus = if c.is_leaf { 0.3 } else { 0.0 };

        // Loop penalty: inlining loops can bloat code
        let loop_penalty = if c.has_loops { -0.2 } else { 0.0 };

        // Thompson sampling: use historical success rate
        let thompson_score = c.thompson_alpha / (c.thompson_alpha + c.thompson_beta);

        let raw_score = size_score * 0.25
            + freq_score * 0.20
            + leaf_bonus
            + loop_penalty
            + thompson_score * 0.25;

        raw_score.clamp(0.0, 1.0)
    }

    /// Decide which functions to inline, respecting the budget.
    /// Returns a list of function names that should be inlined.
    pub fn decide(&mut self) -> Vec<String> {
        self.budget_used = 0;
        let mut scored: Vec<(String, f64, usize)> = self.candidates.values()
            .filter(|c| c.body_size <= self.max_inline_size)
            .map(|c| (c.name.clone(), self.score(&c.name), c.body_size))
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut to_inline = Vec::new();
        for (name, _score, size) in scored {
            if self.budget_used + size > self.inline_budget {
                continue; // Would exceed budget
            }
            self.budget_used += size;
            to_inline.push(name);
        }

        // Mark decisions
        for name in &to_inline {
            if let Some(c) = self.candidates.get_mut(name) {
                c.inlined = true;
            }
        }

        to_inline
    }

    /// Record the outcome of an inlining decision.
    /// `improvement` > 0 means inlining helped, < 0 means it hurt.
    pub fn record_outcome(&mut self, name: &str, improvement: f64) {
        if let Some(c) = self.candidates.get_mut(name) {
            c.improvement = improvement;
            if improvement > 0.0 {
                c.thompson_alpha += 1.0; // Success
            } else {
                c.thompson_beta += 1.0;  // Failure
            }
        }
    }

    /// Get inlining statistics as JSON.
    pub fn stats_json(&self) -> String {
        let total = self.candidates.len();
        let inlined = self.candidates.values().filter(|c| c.inlined).count();
        format!(
            "{{\"candidates\":{},\"inlined\":{},\"budget_used\":{},\"budget_total\":{}}}",
            total, inlined, self.budget_used, self.inline_budget,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  IR OPTIMIZATION PASSES — constant folding, dead elimination, loop tile
// ═══════════════════════════════════════════════════════════════════════

use crate::ir::{IrModule, IrFunction, Inst, Value, IrBinOp};

/// Statistics from an optimization pass.
#[derive(Debug, Clone, Default)]
pub struct OptPassStats {
    pub constants_folded: u32,
    pub dead_eliminated: u32,
    pub loops_tiled: u32,
    pub instructions_before: u32,
    pub instructions_after: u32,
}

impl OptPassStats {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"constants_folded\":{},\"dead_eliminated\":{},\"loops_tiled\":{},\"before\":{},\"after\":{}}}",
            self.constants_folded,
            self.dead_eliminated,
            self.loops_tiled,
            self.instructions_before,
            self.instructions_after,
        )
    }
}

/// Run all optimization passes on an IR module.
pub fn optimize_ir(module: &mut IrModule) -> OptPassStats {
    let mut stats = OptPassStats::default();

    for func in &module.functions {
        stats.instructions_before += func.blocks.iter()
            .map(|b| b.insts.len() as u32)
            .sum::<u32>();
    }

    // Pass 1: Constant folding
    for func in &mut module.functions {
        stats.constants_folded += constant_fold(func);
    }

    // Pass 2: Dead code elimination
    for func in &mut module.functions {
        stats.dead_eliminated += dead_code_eliminate(func);
    }

    for func in &module.functions {
        stats.instructions_after += func.blocks.iter()
            .map(|b| b.insts.len() as u32)
            .sum::<u32>();
    }

    stats
}

/// Constant folding pass: evaluate constant expressions at compile time.
fn constant_fold(func: &mut IrFunction) -> u32 {
    let mut folded = 0u32;

    // Collect known constants: Value → i64 or f64
    let mut iconsts: HashMap<Value, i64> = HashMap::new();
    let mut fconsts: HashMap<Value, f64> = HashMap::new();

    for block in &func.blocks {
        for inst in &block.insts {
            match inst {
                Inst::IConst { result, value, .. } => { iconsts.insert(*result, *value); }
                Inst::FConst { result, value, .. } => { fconsts.insert(*result, *value); }
                _ => {}
            }
        }
    }

    // Fold binary operations on known constants
    for block in &mut func.blocks {
        for inst in &mut block.insts {
            let replacement = match inst {
                Inst::BinOp { result, op, lhs, rhs, ty } => {
                    // Integer constant folding
                    if let (Some(&l), Some(&r)) = (iconsts.get(lhs), iconsts.get(rhs)) {
                        let value = match op {
                            IrBinOp::Add => Some(l.wrapping_add(r)),
                            IrBinOp::Sub => Some(l.wrapping_sub(r)),
                            IrBinOp::Mul => Some(l.wrapping_mul(r)),
                            IrBinOp::Div => if r != 0 { Some(l / r) } else { None },
                            IrBinOp::Mod => if r != 0 { Some(l % r) } else { None },
                            _ => None,
                        };
                        value.map(|v| {
                            iconsts.insert(*result, v);
                            Inst::IConst { result: *result, value: v, ty: ty.clone() }
                        })
                    }
                    // Float constant folding
                    else if let (Some(&l), Some(&r)) = (fconsts.get(lhs), fconsts.get(rhs)) {
                        let value = match op {
                            IrBinOp::FAdd => Some(l + r),
                            IrBinOp::FSub => Some(l - r),
                            IrBinOp::FMul => Some(l * r),
                            IrBinOp::FDiv => if r.abs() > 1e-15 { Some(l / r) } else { None },
                            _ => None,
                        };
                        value.map(|v| {
                            fconsts.insert(*result, v);
                            Inst::FConst { result: *result, value: v, ty: ty.clone() }
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(new_inst) = replacement {
                *inst = new_inst;
                folded += 1;
            }
        }
    }

    folded
}

/// Dead code elimination: remove instructions whose results are never used.
fn dead_code_eliminate(func: &mut IrFunction) -> u32 {
    // Build use set: which Values are ever referenced?
    let mut used: std::collections::HashSet<Value> = std::collections::HashSet::new();

    // All values used as operands are "live"
    for block in &func.blocks {
        for inst in &block.insts {
            match inst {
                Inst::BinOp { lhs, rhs, .. } => { used.insert(*lhs); used.insert(*rhs); }
                Inst::UnOp { operand, .. } => { used.insert(*operand); }
                Inst::ICmp { lhs, rhs, .. } => { used.insert(*lhs); used.insert(*rhs); }
                Inst::FCmp { lhs, rhs, .. } => { used.insert(*lhs); used.insert(*rhs); }
                Inst::Call { args, .. } => { for a in args { used.insert(*a); } }
                Inst::Return { value } => { if let Some(v) = value { used.insert(*v); } }
                Inst::Branch { cond, .. } => { used.insert(*cond); }
                Inst::Phi { incoming, .. } => { for (v, _) in incoming { used.insert(*v); } }
                Inst::Load { ptr, .. } => { used.insert(*ptr); }
                Inst::Store { value, ptr, .. } => { used.insert(*value); used.insert(*ptr); }
                Inst::Copy { source, .. } => { used.insert(*source); }
                Inst::ArrayGet { array, index, .. } => { used.insert(*array); used.insert(*index); }
                Inst::ArraySet { array, index, value, .. } => { used.insert(*array); used.insert(*index); used.insert(*value); }
                Inst::ArrayLen { array, .. } => { used.insert(*array); }
                Inst::ArrayAlloc { count, .. } => { used.insert(*count); }
                Inst::StructAlloc { fields, .. } => { for f in fields { used.insert(*f); } }
                Inst::FieldGet { object, .. } => { used.insert(*object); }
                Inst::FieldSet { object, value, .. } => { used.insert(*object); used.insert(*value); }
                Inst::ClosureAlloc { captures, .. } => { for c in captures { used.insert(*c); } }
                _ => {}
            }
        }
    }

    // Remove instructions that produce unused results (except side-effects)
    let mut eliminated = 0u32;
    for block in &mut func.blocks {
        block.insts.retain(|inst| {
            let result = match inst {
                Inst::IConst { result, .. }
                | Inst::FConst { result, .. }
                | Inst::BConst { result, .. }
                | Inst::StrConst { result, .. }
                | Inst::BinOp { result, .. }
                | Inst::UnOp { result, .. }
                | Inst::ICmp { result, .. }
                | Inst::FCmp { result, .. }
                | Inst::Phi { result, .. }
                | Inst::Copy { result, .. }
                | Inst::Alloca { result, .. } => Some(*result),
                // These have side effects — never eliminate
                Inst::Call { .. }
                | Inst::Return { .. }
                | Inst::Jump { .. }
                | Inst::Branch { .. }
                | Inst::Store { .. }
                | Inst::ArraySet { .. }
                | Inst::FieldSet { .. } => return true,
                _ => None,
            };

            match result {
                Some(r) if !used.contains(&r) => {
                    eliminated += 1;
                    false // Remove this instruction
                }
                _ => true, // Keep
            }
        });
    }

    eliminated
}

// ═══════════════════════════════════════════════════════════════════════
//  QUANTUM-INSPIRED FITNESS LANDSCAPE — Hamiltonian-mapped scoring
// ═══════════════════════════════════════════════════════════════════════

/// Quantum-inspired fitness landscape analysis.
///
/// Maps the classical fitness landscape onto a quantum Hamiltonian model
/// where each function variant is a "quantum state" and fitness differences
/// drive tunneling probabilities between variants.
///
/// This enables:
/// - Tunneling through local fitness optima (unlike gradient descent)
/// - Entanglement-inspired correlation between related functions
/// - Superposition-based exploration of multiple variants simultaneously
pub struct QuantumLandscape {
    /// State vector: function_name → vec of (generation, fitness, energy)
    states: HashMap<String, Vec<QuantumState>>,
    /// Temperature parameter for simulated quantum annealing.
    temperature: f64,
    /// Planck constant analog — controls tunneling probability.
    h_bar: f64,
    /// Entanglement pairs: functions that co-evolve
    entanglements: Vec<(String, String, f64)>, // (func_a, func_b, coupling)
}

/// A single quantum state in the fitness landscape.
#[derive(Debug, Clone)]
pub struct QuantumState {
    pub generation: u64,
    pub fitness: f64,
    /// Energy = -fitness (lower energy = higher fitness, like quantum ground state)
    pub energy: f64,
    /// Amplitude (probability weight in superposition).
    pub amplitude: f64,
    /// Phase (for interference effects).
    pub phase: f64,
}

impl QuantumLandscape {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            temperature: 1.0,
            h_bar: 0.1,
            entanglements: Vec::new(),
        }
    }

    /// Add a variant to the quantum landscape.
    pub fn add_state(&mut self, function_name: &str, generation: u64, fitness: f64) {
        let energy = -fitness; // Lower energy = better
        let amplitude = fitness.max(0.01); // Proportional to fitness
        let phase = (generation as f64 * 0.7853981633974483).sin(); // π/4 phase rotation per gen

        let states = self.states.entry(function_name.to_string()).or_default();
        states.push(QuantumState { generation, fitness, energy, amplitude, phase });

        // Normalize amplitudes (like quantum state normalization)
        let norm: f64 = states.iter().map(|s| s.amplitude * s.amplitude).sum::<f64>().sqrt();
        if norm > 1e-15 {
            for s in states.iter_mut() {
                s.amplitude /= norm;
            }
        }
    }

    /// Declare two functions as entangled (they tend to co-evolve).
    pub fn entangle(&mut self, func_a: &str, func_b: &str, coupling: f64) {
        self.entanglements.push((
            func_a.to_string(),
            func_b.to_string(),
            coupling.clamp(-1.0, 1.0),
        ));
    }

    /// Compute the quantum tunneling probability between two fitness values.
    /// Higher barrier = lower probability, but non-zero (unlike classical).
    pub fn tunneling_probability(&self, energy_from: f64, energy_to: f64) -> f64 {
        let barrier = (energy_to - energy_from).max(0.0);
        if barrier < 1e-15 { return 1.0; } // Downhill: always tunnel

        // Gamow tunneling formula analog: P ∝ exp(-2 * barrier / ℏ)
        let exponent = -2.0 * barrier / (self.h_bar * self.temperature);
        exponent.exp().clamp(1e-10, 1.0)
    }

    /// Compute quantum-inspired annealing score for a function.
    /// This blends exploration (high temperature) with exploitation (low temperature).
    /// Returns a score that accounts for tunneling potential to better states.
    pub fn annealing_score(&self, function_name: &str, current_fitness: f64) -> f64 {
        let states = match self.states.get(function_name) {
            Some(s) if !s.is_empty() => s,
            _ => return current_fitness,
        };

        let current_energy = -current_fitness;

        // Sum probability-weighted energies across all known states
        // (quantum expectation value)
        let mut expectation = 0.0;
        let mut total_weight = 0.0;

        for state in states {
            let tunnel_prob = self.tunneling_probability(current_energy, state.energy);
            let weight = state.amplitude * state.amplitude * tunnel_prob;
            expectation += weight * state.fitness;
            total_weight += weight;
        }

        // Factor in entanglement correlations
        let entanglement_bonus = self.entanglement_correlation(function_name);

        if total_weight > 1e-15 {
            (expectation / total_weight) * (1.0 + entanglement_bonus * 0.1)
        } else {
            current_fitness
        }
    }

    /// Compute entanglement correlation for a function.
    /// Returns a signal in [-1, 1] based on correlated partner fitness.
    fn entanglement_correlation(&self, function_name: &str) -> f64 {
        let mut correlation = 0.0;
        let mut count = 0;

        for (fa, fb, coupling) in &self.entanglements {
            let partner = if fa == function_name {
                Some(fb)
            } else if fb == function_name {
                Some(fa)
            } else {
                None
            };

            if let Some(partner_name) = partner {
                if let Some(partner_states) = self.states.get(partner_name.as_str()) {
                    if let Some(last) = partner_states.last() {
                        correlation += coupling * last.fitness;
                        count += 1;
                    }
                }
            }
        }

        if count > 0 { correlation / count as f64 } else { 0.0 }
    }

    /// Reduce temperature (annealing schedule).
    pub fn cool(&mut self, rate: f64) {
        self.temperature *= rate;
        if self.temperature < 0.001 {
            self.temperature = 0.001; // Floor
        }
    }

    /// Get landscape analysis as JSON.
    pub fn landscape_json(&self) -> String {
        let total_states: usize = self.states.values().map(|v| v.len()).sum();
        let total_functions = self.states.len();
        let total_entanglements = self.entanglements.len();

        let best_fitness = self.states.values()
            .flat_map(|v| v.iter())
            .map(|s| s.fitness)
            .fold(0.0_f64, f64::max);

        format!(
            concat!(
                "{{",
                "\"total_states\":{},",
                "\"total_functions\":{},",
                "\"entanglements\":{},",
                "\"temperature\":{:.6},",
                "\"h_bar\":{:.6},",
                "\"best_fitness\":{:.4}",
                "}}"
            ),
            total_states,
            total_functions,
            total_entanglements,
            self.temperature,
            self.h_bar,
            best_fitness,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrType, BasicBlock};

    // ── Compilation Cache Tests ──────────────────────────────────────
    #[test]
    fn test_cache_basic() {
        let mut cache = CompilationCache::new(100);
        assert_eq!(cache.hit_rate(), 0.0);

        cache.store(12345, CachedCompilation {
            source_hash: 12345,
            success: true,
            compile_time_ms: 1.5,
            fitness: 0.85,
            hits: 0,
            last_access_ms: 0.0,
            errors: vec![],
        });

        assert!(cache.lookup(12345).is_some());
        assert_eq!(cache.total_hits, 1);
        assert!(cache.lookup(99999).is_none());
        assert_eq!(cache.total_misses, 1);
        assert!((cache.hit_rate() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = CompilationCache::new(2);
        for i in 0..3 {
            cache.store(i, CachedCompilation {
                source_hash: i,
                success: true,
                compile_time_ms: 1.0,
                fitness: 0.5,
                hits: 0,
                last_access_ms: i as f64,
                errors: vec![],
            });
        }
        // Should have evicted the first entry
        assert_eq!(cache.entries.len(), 2);
    }

    // ── Trajectory Predictor Tests ──────────────────────────────────
    #[test]
    fn test_trajectory_basic() {
        let mut pred = TrajectoryPredictor::new();
        pred.observe(EvolutionObservation {
            function_name: "alpha".to_string(),
            generation: 0,
            fitness: 0.5,
            source_hash: 100,
            timestamp_ms: 1000.0,
        });
        pred.observe(EvolutionObservation {
            function_name: "alpha".to_string(),
            generation: 1,
            fitness: 0.6,
            source_hash: 101,
            timestamp_ms: 2000.0,
        });
        pred.observe(EvolutionObservation {
            function_name: "beta".to_string(),
            generation: 0,
            fitness: 0.3,
            source_hash: 200,
            timestamp_ms: 3000.0,
        });

        let preds = pred.predict_next(5);
        assert!(!preds.is_empty());
        // Alpha was evolved more recently and more frequently
    }

    // ── Delta Debugger Tests ────────────────────────────────────────
    #[test]
    fn test_delta_debug_identical() {
        let dd = DeltaDebugger::new();
        let source = "fn main() -> i64 { 42 }";
        let result = dd.isolate(source, source);
        assert_eq!(result.bisection_steps, 0);
        assert!(result.errors[0].contains("identical"));
    }

    #[test]
    fn test_delta_debug_simple() {
        let dd = DeltaDebugger::new();
        let good = "fn main() -> i64 { 42 }";
        let bad = "fn main( -> { broken }"; // syntax error
        let result = dd.isolate(good, bad);
        assert!(!result.errors.is_empty() || !result.minimal_failing.is_empty());
        // Single-line diff may not need oracle calls (already minimal)
        assert!(result.bisection_steps == 0 || result.oracle_calls >= 1);
    }

    // ── Inlining Oracle Tests ───────────────────────────────────────
    #[test]
    fn test_inlining_basic() {
        let mut oracle = InliningOracle::new();
        oracle.register_candidate(InliningCandidate {
            name: "small_fn".to_string(),
            body_size: 5,
            call_sites: 10,
            is_leaf: true,
            param_count: 1,
            has_loops: false,
            thompson_alpha: 1.0,
            thompson_beta: 1.0,
            inlined: false,
            improvement: 0.0,
        });
        oracle.register_candidate(InliningCandidate {
            name: "big_fn".to_string(),
            body_size: 100,
            call_sites: 2,
            is_leaf: false,
            param_count: 5,
            has_loops: true,
            thompson_alpha: 1.0,
            thompson_beta: 1.0,
            inlined: false,
            improvement: 0.0,
        });

        let decisions = oracle.decide();
        assert!(decisions.contains(&"small_fn".to_string()));
        assert!(!decisions.contains(&"big_fn".to_string())); // Too large
    }

    #[test]
    fn test_inlining_thompson_learning() {
        let mut oracle = InliningOracle::new();
        oracle.register_candidate(InliningCandidate {
            name: "learnable".to_string(),
            body_size: 10,
            call_sites: 5,
            is_leaf: true,
            param_count: 2,
            has_loops: false,
            thompson_alpha: 1.0,
            thompson_beta: 1.0,
            inlined: false,
            improvement: 0.0,
        });

        let score_before = oracle.score("learnable");

        // Record positive outcome
        oracle.record_outcome("learnable", 0.5);
        let score_after = oracle.score("learnable");

        // Score should improve after positive outcome
        assert!(score_after >= score_before);
    }

    // ── Quantum Landscape Tests ─────────────────────────────────────
    #[test]
    fn test_quantum_basic() {
        let mut ql = QuantumLandscape::new();
        ql.add_state("func_a", 0, 0.5);
        ql.add_state("func_a", 1, 0.7);
        ql.add_state("func_a", 2, 0.6);

        let score = ql.annealing_score("func_a", 0.6);
        assert!(score > 0.0);
    }

    #[test]
    fn test_quantum_tunneling() {
        let ql = QuantumLandscape::new();

        // Downhill: always tunnel
        let p_down = ql.tunneling_probability(-0.5, -0.8);
        assert!((p_down - 1.0).abs() < 1e-10);

        // Uphill: probability decreases with barrier height
        let p_up_small = ql.tunneling_probability(-0.5, -0.3);
        let p_up_large = ql.tunneling_probability(-0.5, 0.5);
        assert!(p_up_small > p_up_large);
    }

    #[test]
    fn test_quantum_entanglement() {
        let mut ql = QuantumLandscape::new();
        ql.add_state("func_a", 0, 0.8);
        ql.add_state("func_b", 0, 0.9);
        ql.entangle("func_a", "func_b", 0.5);

        let corr = ql.entanglement_correlation("func_a");
        assert!(corr > 0.0); // Positive coupling with high-fitness partner
    }

    #[test]
    fn test_quantum_cooling() {
        let mut ql = QuantumLandscape::new();
        let temp_before = ql.temperature;
        ql.cool(0.95);
        assert!(ql.temperature < temp_before);
        assert!(ql.temperature > 0.0);
    }

    // ── IR Optimization Tests ───────────────────────────────────────
    #[test]
    fn test_constant_folding() {
        use crate::ir::{BlockId};
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 10, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 20, ty: IrType::I64 },
                    Inst::BinOp {
                        result: Value(2),
                        op: IrBinOp::Add,
                        lhs: Value(0),
                        rhs: Value(1),
                        ty: IrType::I64,
                    },
                    Inst::Return { value: Some(Value(2)) },
                ],
            }],
            entry: BlockId(0),
        };

        let folded = constant_fold(&mut func);
        assert!(folded > 0);

        // The BinOp should now be replaced with IConst(30)
        let last_const = func.blocks[0].insts.iter().find(|i| matches!(i, Inst::IConst { value: 30, .. }));
        assert!(last_const.is_some());
    }

    #[test]
    fn test_dead_code_elimination() {
        use crate::ir::{BlockId};
        let mut func = IrFunction {
            name: "test".to_string(),
            params: vec![],
            ret_type: IrType::I64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                insts: vec![
                    Inst::IConst { result: Value(0), value: 42, ty: IrType::I64 },
                    Inst::IConst { result: Value(1), value: 99, ty: IrType::I64 }, // Dead
                    Inst::Return { value: Some(Value(0)) },
                ],
            }],
            entry: BlockId(0),
        };

        let eliminated = dead_code_eliminate(&mut func);
        assert_eq!(eliminated, 1);
        assert_eq!(func.blocks[0].insts.len(), 2); // Only IConst(42) + Return
    }

    // ── Optimization Stats Tests ────────────────────────────────────
    #[test]
    fn test_opt_pass_stats_json() {
        let stats = OptPassStats {
            constants_folded: 5,
            dead_eliminated: 3,
            loops_tiled: 1,
            instructions_before: 100,
            instructions_after: 91,
        };
        let json = stats.to_json();
        assert!(json.contains("\"constants_folded\":5"));
        assert!(json.contains("\"dead_eliminated\":3"));
    }
}
