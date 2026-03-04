//! Self Optimizer — RL-based pass ordering, cost models, and auto-tuning.
//!
//! Provides reinforcement learning for compiler optimization pass ordering,
//! cost models for performance prediction, auto-tuning via Bayesian optimization,
//! and adaptive compilation strategies.

use std::collections::HashMap;

// ── Optimization Pass ───────────────────────────────────────────────────

/// Compiler optimization pass identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptPass {
    ConstantFolding,
    DeadCodeElimination,
    CommonSubexprElimination,
    LoopInvariantCodeMotion,
    Inlining,
    StrengthReduction,
    TailCallOptimization,
    RegisterAllocation,
    InstructionScheduling,
    LoopUnrolling,
    Vectorization,
    MemoryToRegister,
    AlgebraicSimplification,
    BranchElimination,
    GlobalValueNumbering,
}

impl OptPass {
    pub fn all() -> &'static [OptPass] {
        &[
            OptPass::ConstantFolding,
            OptPass::DeadCodeElimination,
            OptPass::CommonSubexprElimination,
            OptPass::LoopInvariantCodeMotion,
            OptPass::Inlining,
            OptPass::StrengthReduction,
            OptPass::TailCallOptimization,
            OptPass::RegisterAllocation,
            OptPass::InstructionScheduling,
            OptPass::LoopUnrolling,
            OptPass::Vectorization,
            OptPass::MemoryToRegister,
            OptPass::AlgebraicSimplification,
            OptPass::BranchElimination,
            OptPass::GlobalValueNumbering,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            OptPass::ConstantFolding => "const_fold",
            OptPass::DeadCodeElimination => "dce",
            OptPass::CommonSubexprElimination => "cse",
            OptPass::LoopInvariantCodeMotion => "licm",
            OptPass::Inlining => "inline",
            OptPass::StrengthReduction => "strength_reduce",
            OptPass::TailCallOptimization => "tco",
            OptPass::RegisterAllocation => "regalloc",
            OptPass::InstructionScheduling => "isched",
            OptPass::LoopUnrolling => "unroll",
            OptPass::Vectorization => "vectorize",
            OptPass::MemoryToRegister => "mem2reg",
            OptPass::AlgebraicSimplification => "alg_simp",
            OptPass::BranchElimination => "branch_elim",
            OptPass::GlobalValueNumbering => "gvn",
        }
    }
}

// ── Cost Model ──────────────────────────────────────────────────────────

/// Program features for cost prediction.
#[derive(Debug, Clone, Default)]
pub struct ProgramFeatures {
    pub num_instructions: usize,
    pub num_basic_blocks: usize,
    pub num_branches: usize,
    pub num_loops: usize,
    pub num_calls: usize,
    pub max_loop_depth: usize,
    pub num_memory_ops: usize,
    pub num_arithmetic_ops: usize,
    pub num_phi_nodes: usize,
    pub estimated_register_pressure: usize,
}

/// Cost model for predicting optimization benefit.
#[derive(Debug, Clone)]
pub struct CostModel {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub feature_count: usize,
}

impl CostModel {
    pub fn new(feature_count: usize) -> Self {
        CostModel {
            weights: vec![0.0; feature_count],
            bias: 0.0,
            feature_count,
        }
    }

    /// Predict the cost of a program given its features.
    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut cost = self.bias;
        for (w, f) in self.weights.iter().zip(features.iter()) {
            cost += w * f;
        }
        cost
    }

    /// Train cost model on (features, actual_cost) pairs.
    pub fn train(&mut self, data: &[(Vec<f64>, f64)], learning_rate: f64, epochs: usize) {
        for _ in 0..epochs {
            for (features, target) in data {
                let predicted = self.predict(features);
                let error = predicted - target;

                // SGD update
                self.bias -= learning_rate * error;
                for (i, &f) in features.iter().enumerate() {
                    if i < self.weights.len() {
                        self.weights[i] -= learning_rate * error * f;
                    }
                }
            }
        }
    }

    /// Convert ProgramFeatures to feature vector.
    pub fn features_to_vec(features: &ProgramFeatures) -> Vec<f64> {
        vec![
            features.num_instructions as f64,
            features.num_basic_blocks as f64,
            features.num_branches as f64,
            features.num_loops as f64,
            features.num_calls as f64,
            features.max_loop_depth as f64,
            features.num_memory_ops as f64,
            features.num_arithmetic_ops as f64,
            features.num_phi_nodes as f64,
            features.estimated_register_pressure as f64,
        ]
    }
}

// ── RL Pass Ordering ────────────────────────────────────────────────────

/// Q-learning agent for optimization pass ordering.
#[derive(Debug, Clone)]
pub struct PassOrderingAgent {
    /// Q-table: state → action → value
    pub q_table: HashMap<Vec<usize>, Vec<f64>>,
    pub num_passes: usize,
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub epsilon: f64,
    pub episode: usize,
}

impl PassOrderingAgent {
    pub fn new(num_passes: usize, learning_rate: f64, discount_factor: f64, epsilon: f64) -> Self {
        PassOrderingAgent {
            q_table: HashMap::new(),
            num_passes,
            learning_rate,
            discount_factor,
            epsilon,
            episode: 0,
        }
    }

    /// Get Q-values for a state, initializing if needed.
    fn get_q_values(&self, state: &[usize]) -> Vec<f64> {
        self.q_table.get(state).cloned().unwrap_or_else(|| vec![0.0; self.num_passes])
    }

    /// Select action using epsilon-greedy policy.
    pub fn select_action(&self, state: &[usize], available: &[usize], seed: u64) -> usize {
        // Epsilon-greedy
        let mut rng_state = seed.wrapping_add(self.episode as u64);
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let r = rng_state as f64 / u64::MAX as f64;

        if r < self.epsilon {
            // Random action
            let idx = (rng_state as usize) % available.len();
            available[idx]
        } else {
            // Greedy
            let q_values = self.get_q_values(state);
            let mut best_action = available[0];
            let mut best_value = f64::NEG_INFINITY;
            for &a in available {
                if a < q_values.len() && q_values[a] > best_value {
                    best_value = q_values[a];
                    best_action = a;
                }
            }
            best_action
        }
    }

    /// Update Q-value after observing reward.
    pub fn update(&mut self, state: &[usize], action: usize, reward: f64, next_state: &[usize]) {
        let next_q = self.get_q_values(next_state);
        let max_next_q = next_q.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let q_values = self.q_table.entry(state.to_vec()).or_insert_with(|| vec![0.0; self.num_passes]);
        if action < q_values.len() {
            let td_target = reward + self.discount_factor * max_next_q;
            q_values[action] += self.learning_rate * (td_target - q_values[action]);
        }
    }

    /// Decay epsilon for exploration reduction.
    pub fn decay_epsilon(&mut self, min_epsilon: f64, decay_rate: f64) {
        self.epsilon = (self.epsilon * decay_rate).max(min_epsilon);
        self.episode += 1;
    }

    /// Get best known pass ordering for a given state.
    pub fn best_ordering(&self, initial_state: &[usize], max_passes: usize) -> Vec<usize> {
        let mut state = initial_state.to_vec();
        let mut ordering = Vec::new();
        let mut used = std::collections::HashSet::new();

        for _ in 0..max_passes {
            let q_values = self.get_q_values(&state);
            let available: Vec<usize> = (0..self.num_passes)
                .filter(|a| !used.contains(a))
                .collect();
            if available.is_empty() { break; }

            let mut best_action = available[0];
            let mut best_value = f64::NEG_INFINITY;
            for &a in &available {
                if a < q_values.len() && q_values[a] > best_value {
                    best_value = q_values[a];
                    best_action = a;
                }
            }

            ordering.push(best_action);
            used.insert(best_action);
            state.push(best_action);
        }
        ordering
    }
}

// ── Bayesian Optimization for Auto-Tuning ───────────────────────────────

/// Simple Bayesian optimization using upper confidence bound (UCB).
#[derive(Debug, Clone)]
pub struct BayesianOptimizer {
    pub observations: Vec<(Vec<f64>, f64)>, // (params, objective)
    pub param_bounds: Vec<(f64, f64)>,
    pub exploration_weight: f64,
    pub best_params: Vec<f64>,
    pub best_objective: f64,
}

impl BayesianOptimizer {
    pub fn new(param_bounds: Vec<(f64, f64)>, exploration_weight: f64) -> Self {
        BayesianOptimizer {
            observations: Vec::new(),
            param_bounds: param_bounds.clone(),
            exploration_weight,
            best_params: param_bounds.iter().map(|(lo, hi)| (lo + hi) / 2.0).collect(),
            best_objective: f64::NEG_INFINITY,
        }
    }

    /// Record an observation.
    pub fn observe(&mut self, params: Vec<f64>, objective: f64) {
        if objective > self.best_objective {
            self.best_objective = objective;
            self.best_params = params.clone();
        }
        self.observations.push((params, objective));
    }

    /// Suggest next parameter point to evaluate (using simplified UCB).
    pub fn suggest(&self, seed: u64) -> Vec<f64> {
        let n_candidates = 100;
        let mut best_candidate = self.best_params.clone();
        let mut best_ucb = f64::NEG_INFINITY;

        let mut rng_state = seed;
        for _ in 0..n_candidates {
            // Random candidate within bounds
            let candidate: Vec<f64> = self.param_bounds.iter().map(|&(lo, hi)| {
                rng_state ^= rng_state << 13;
                rng_state ^= rng_state >> 7;
                rng_state ^= rng_state << 17;
                let r = rng_state as f64 / u64::MAX as f64;
                lo + r * (hi - lo)
            }).collect();

            // Compute UCB: mean + exploration * uncertainty
            let (mean, variance) = self.predict(&candidate);
            let ucb = mean + self.exploration_weight * variance.sqrt();

            if ucb > best_ucb {
                best_ucb = ucb;
                best_candidate = candidate;
            }
        }
        best_candidate
    }

    /// Simple kernel-based prediction (nearest neighbor weighted).
    fn predict(&self, params: &[f64]) -> (f64, f64) {
        if self.observations.is_empty() {
            return (0.0, 1.0);
        }

        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;
        let mut sq_sum = 0.0;

        for (obs_params, obs_val) in &self.observations {
            let dist: f64 = params.iter().zip(obs_params.iter())
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            let weight = (-dist * 2.0).exp(); // RBF kernel
            weighted_sum += weight * obs_val;
            sq_sum += weight * obs_val * obs_val;
            weight_total += weight;
        }

        if weight_total > 0.0 {
            let mean = weighted_sum / weight_total;
            let variance = (sq_sum / weight_total - mean * mean).max(0.001);
            (mean, variance)
        } else {
            (0.0, 1.0)
        }
    }
}

// ── Adaptive Compilation ────────────────────────────────────────────────

/// Compilation tier for tiered JIT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompilationTier {
    Interpreted,
    BaselineJIT,
    OptimizedJIT,
    FullyOptimized,
}

/// Compilation strategy decision.
#[derive(Debug, Clone)]
pub struct CompilationDecision {
    pub tier: CompilationTier,
    pub passes: Vec<OptPass>,
    pub estimated_speedup: f64,
    pub estimated_compile_time_ms: f64,
}

/// Adaptive compilation manager.
#[derive(Debug, Clone)]
pub struct AdaptiveCompiler {
    pub hot_functions: HashMap<String, usize>, // function → call count
    pub tier_thresholds: [usize; 3],
    pub history: Vec<(String, CompilationTier, f64)>, // (func, tier, runtime)
}

impl AdaptiveCompiler {
    pub fn new() -> Self {
        AdaptiveCompiler {
            hot_functions: HashMap::new(),
            tier_thresholds: [10, 100, 1000], // baseline, optimized, fully-optimized
            history: Vec::new(),
        }
    }

    /// Record a function call and return recommended compilation tier.
    pub fn record_call(&mut self, function: &str) -> CompilationTier {
        let count = self.hot_functions.entry(function.to_string()).or_insert(0);
        *count += 1;

        if *count >= self.tier_thresholds[2] {
            CompilationTier::FullyOptimized
        } else if *count >= self.tier_thresholds[1] {
            CompilationTier::OptimizedJIT
        } else if *count >= self.tier_thresholds[0] {
            CompilationTier::BaselineJIT
        } else {
            CompilationTier::Interpreted
        }
    }

    /// Get recommended pass sequence for a tier.
    pub fn passes_for_tier(&self, tier: CompilationTier) -> Vec<OptPass> {
        match tier {
            CompilationTier::Interpreted => vec![],
            CompilationTier::BaselineJIT => vec![
                OptPass::ConstantFolding,
                OptPass::DeadCodeElimination,
            ],
            CompilationTier::OptimizedJIT => vec![
                OptPass::ConstantFolding,
                OptPass::DeadCodeElimination,
                OptPass::CommonSubexprElimination,
                OptPass::Inlining,
                OptPass::MemoryToRegister,
            ],
            CompilationTier::FullyOptimized => vec![
                OptPass::ConstantFolding,
                OptPass::DeadCodeElimination,
                OptPass::CommonSubexprElimination,
                OptPass::Inlining,
                OptPass::MemoryToRegister,
                OptPass::LoopInvariantCodeMotion,
                OptPass::LoopUnrolling,
                OptPass::GlobalValueNumbering,
                OptPass::Vectorization,
                OptPass::InstructionScheduling,
                OptPass::RegisterAllocation,
            ],
        }
    }

    /// Decide compilation strategy for a function.
    pub fn decide(&mut self, function: &str) -> CompilationDecision {
        let tier = self.record_call(function);
        let passes = self.passes_for_tier(tier);
        let estimated_speedup = match tier {
            CompilationTier::Interpreted => 1.0,
            CompilationTier::BaselineJIT => 5.0,
            CompilationTier::OptimizedJIT => 20.0,
            CompilationTier::FullyOptimized => 50.0,
        };
        let estimated_compile_time_ms = match tier {
            CompilationTier::Interpreted => 0.0,
            CompilationTier::BaselineJIT => 1.0,
            CompilationTier::OptimizedJIT => 10.0,
            CompilationTier::FullyOptimized => 100.0,
        };

        CompilationDecision { tier, passes, estimated_speedup, estimated_compile_time_ms }
    }
}

impl Default for AdaptiveCompiler {
    fn default() -> Self { Self::new() }
}

// ── FFI Interface ───────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_selfopt_cost_predict(features: *const f64, feature_count: i64, weights: *const f64, bias: f64) -> f64 {
    let f = unsafe { std::slice::from_raw_parts(features, feature_count as usize) };
    let w = unsafe { std::slice::from_raw_parts(weights, feature_count as usize) };
    let model = CostModel { weights: w.to_vec(), bias, feature_count: feature_count as usize };
    model.predict(f)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_selfopt_num_passes() -> i64 {
    OptPass::all().len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_selfopt_tier(call_count: i64) -> i64 {
    if call_count >= 1000 { 3 }
    else if call_count >= 100 { 2 }
    else if call_count >= 10 { 1 }
    else { 0 }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opt_pass_count() {
        assert_eq!(OptPass::all().len(), 15);
    }

    #[test]
    fn test_cost_model_predict() {
        let mut model = CostModel::new(3);
        model.weights = vec![1.0, 2.0, 3.0];
        model.bias = 0.5;
        assert!((model.predict(&[1.0, 1.0, 1.0]) - 6.5).abs() < 1e-10);
    }

    #[test]
    fn test_cost_model_train() {
        let mut model = CostModel::new(2);
        let data = vec![
            (vec![1.0, 0.0], 2.0),
            (vec![0.0, 1.0], 3.0),
            (vec![1.0, 1.0], 5.0),
        ];
        model.train(&data, 0.01, 100);
        let pred = model.predict(&[1.0, 1.0]);
        assert!((pred - 5.0).abs() < 1.0); // Should be approximately 5.0
    }

    #[test]
    fn test_features_to_vec() {
        let features = ProgramFeatures {
            num_instructions: 100,
            num_basic_blocks: 10,
            ..Default::default()
        };
        let vec = CostModel::features_to_vec(&features);
        assert_eq!(vec.len(), 10);
        assert_eq!(vec[0], 100.0);
        assert_eq!(vec[1], 10.0);
    }

    #[test]
    fn test_pass_ordering_agent() {
        let mut agent = PassOrderingAgent::new(5, 0.1, 0.9, 0.1);
        let state = vec![0, 1];
        let action = agent.select_action(&state, &[2, 3, 4], 42);
        assert!([2, 3, 4].contains(&action));

        // Update Q-value
        agent.update(&state, action, 1.0, &[0, 1, action]);
    }

    #[test]
    fn test_pass_ordering_best() {
        let mut agent = PassOrderingAgent::new(3, 0.5, 0.9, 0.0); // No exploration
        // Set Q-values manually
        agent.q_table.insert(vec![], vec![3.0, 1.0, 2.0]);
        agent.q_table.insert(vec![0], vec![0.0, 1.0, 5.0]);

        let ordering = agent.best_ordering(&[], 2);
        assert_eq!(ordering[0], 0); // Highest Q in empty state
        assert_eq!(ordering[1], 2); // Highest Q after choosing 0
    }

    #[test]
    fn test_epsilon_decay() {
        let mut agent = PassOrderingAgent::new(5, 0.1, 0.9, 1.0);
        agent.decay_epsilon(0.01, 0.95);
        assert!((agent.epsilon - 0.95).abs() < 1e-10);
        for _ in 0..100 {
            agent.decay_epsilon(0.01, 0.95);
        }
        assert!(agent.epsilon >= 0.01);
    }

    #[test]
    fn test_bayesian_optimizer() {
        let bounds = vec![(0.0, 1.0), (0.0, 10.0)];
        let mut opt = BayesianOptimizer::new(bounds, 2.0);

        opt.observe(vec![0.5, 5.0], 10.0);
        opt.observe(vec![0.3, 7.0], 15.0);

        let suggestion = opt.suggest(42);
        assert_eq!(suggestion.len(), 2);
        assert!(suggestion[0] >= 0.0 && suggestion[0] <= 1.0);
        assert!(suggestion[1] >= 0.0 && suggestion[1] <= 10.0);
    }

    #[test]
    fn test_bayesian_best_tracking() {
        let bounds = vec![(0.0, 1.0)];
        let mut opt = BayesianOptimizer::new(bounds, 1.0);
        opt.observe(vec![0.1], 5.0);
        opt.observe(vec![0.9], 20.0);
        opt.observe(vec![0.5], 10.0);
        assert!((opt.best_objective - 20.0).abs() < 1e-10);
        assert!((opt.best_params[0] - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_compiler_tiers() {
        let mut compiler = AdaptiveCompiler::new();
        for _ in 0..9 {
            assert_eq!(compiler.decide("f").tier, CompilationTier::Interpreted);
        }
        assert_eq!(compiler.decide("f").tier, CompilationTier::BaselineJIT);
        for _ in 0..90 {
            compiler.decide("f");
        }
        assert_eq!(compiler.decide("f").tier, CompilationTier::OptimizedJIT);
    }

    #[test]
    fn test_adaptive_pass_sequences() {
        let compiler = AdaptiveCompiler::new();
        assert_eq!(compiler.passes_for_tier(CompilationTier::Interpreted).len(), 0);
        assert_eq!(compiler.passes_for_tier(CompilationTier::BaselineJIT).len(), 2);
        assert!(compiler.passes_for_tier(CompilationTier::FullyOptimized).len() > 5);
    }

    #[test]
    fn test_compilation_decision() {
        let mut compiler = AdaptiveCompiler::new();
        for _ in 0..15 {
            compiler.decide("hot_func");
        }
        let decision = compiler.decide("hot_func");
        assert!(decision.estimated_speedup > 1.0);
        assert!(decision.passes.len() > 0);
    }

    #[test]
    fn test_ffi_cost_predict() {
        let features = [10.0f64, 20.0];
        let weights = [1.0f64, 0.5];
        let result = vitalis_selfopt_cost_predict(features.as_ptr(), 2, weights.as_ptr(), 1.0);
        assert!((result - 21.0).abs() < 1e-10); // 10*1 + 20*0.5 + 1.0
    }

    #[test]
    fn test_ffi_num_passes() {
        assert_eq!(vitalis_selfopt_num_passes(), 15);
    }

    #[test]
    fn test_ffi_tier() {
        assert_eq!(vitalis_selfopt_tier(5), 0);
        assert_eq!(vitalis_selfopt_tier(50), 1);
        assert_eq!(vitalis_selfopt_tier(500), 2);
        assert_eq!(vitalis_selfopt_tier(5000), 3);
    }

    #[test]
    fn test_pass_names() {
        for pass in OptPass::all() {
            assert!(!pass.name().is_empty());
        }
    }
}
