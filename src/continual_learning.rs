//! Continual Learning Module — Vitalis v44.0
//!
//! Algorithms for learning without catastrophic forgetting:
//! - Elastic Weight Consolidation (EWC) — Fisher information regularization
//! - Progressive Neural Networks — lateral connections, no forgetting
//! - Experience Replay — episodic memory buffer with reservoir sampling
//! - Synaptic Intelligence (SI) — online importance estimation
//! - Learning without Forgetting (LwF) — knowledge distillation
//! - Task-incremental & class-incremental scenarios
//! - FFI interface for external integration

use std::collections::HashMap;
use std::sync::Mutex;

// ═══════════════════════════════════════════════════════════════════
// RNG
// ═══════════════════════════════════════════════════════════════════

struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn next_usize(&mut self, bound: usize) -> usize {
        if bound == 0 { return 0; }
        (self.next_f64() * bound as f64) as usize % bound
    }
}

// ═══════════════════════════════════════════════════════════════════
// 1. Elastic Weight Consolidation (EWC)
// ═══════════════════════════════════════════════════════════════════

/// EWC prevents catastrophic forgetting by penalizing changes to important weights.
/// Fisher information diagonal approximation.
pub struct ElasticWeightConsolidation {
    /// Stored reference weights (theta*) from previous tasks
    reference_weights: Vec<Vec<f64>>,
    /// Fisher information diagonal for each task
    fisher_diagonals: Vec<Vec<f64>>,
    /// Lambda: regularization strength
    pub lambda: f64,
    /// Number of tasks consolidated
    pub num_tasks: usize,
}

impl ElasticWeightConsolidation {
    pub fn new(lambda: f64) -> Self {
        Self {
            reference_weights: Vec::new(),
            fisher_diagonals: Vec::new(),
            lambda: lambda.max(0.0),
            num_tasks: 0,
        }
    }

    /// Register a completed task: store current weights and compute Fisher information
    pub fn consolidate(&mut self, weights: &[f64], gradients_samples: &[Vec<f64>]) {
        // Store θ* (reference weights for this task)
        self.reference_weights.push(weights.to_vec());

        // Compute Fisher Information diagonal = E[g²]
        let n = weights.len();
        let mut fisher = vec![0.0; n];
        if !gradients_samples.is_empty() {
            for grad in gradients_samples {
                for i in 0..n.min(grad.len()) {
                    fisher[i] += grad[i] * grad[i];
                }
            }
            let m = gradients_samples.len() as f64;
            for f in &mut fisher {
                *f /= m;
            }
        }
        self.fisher_diagonals.push(fisher);
        self.num_tasks += 1;
    }

    /// Compute EWC penalty: λ/2 * Σ_tasks Σ_i F_i * (θ_i - θ*_i)²
    pub fn penalty(&self, current_weights: &[f64]) -> f64 {
        let mut total = 0.0;
        for task in 0..self.num_tasks {
            let ref_w = &self.reference_weights[task];
            let fisher = &self.fisher_diagonals[task];
            let n = current_weights.len().min(ref_w.len()).min(fisher.len());
            for i in 0..n {
                let diff = current_weights[i] - ref_w[i];
                total += fisher[i] * diff * diff;
            }
        }
        0.5 * self.lambda * total
    }

    /// Compute EWC gradient contribution: λ * Σ_tasks F_i * (θ_i - θ*_i)
    pub fn penalty_gradient(&self, current_weights: &[f64]) -> Vec<f64> {
        let n = current_weights.len();
        let mut grad = vec![0.0; n];
        for task in 0..self.num_tasks {
            let ref_w = &self.reference_weights[task];
            let fisher = &self.fisher_diagonals[task];
            let m = n.min(ref_w.len()).min(fisher.len());
            for i in 0..m {
                grad[i] += self.lambda * fisher[i] * (current_weights[i] - ref_w[i]);
            }
        }
        grad
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. Progressive Neural Networks
// ═══════════════════════════════════════════════════════════════════

/// A column in a progressive network (one per task)
#[derive(Debug, Clone)]
pub struct ProgressiveColumn {
    pub task_id: usize,
    /// Weight matrices per layer: [layer_idx][row * col]
    pub layers: Vec<Vec<f64>>,
    /// Layer dimensions: (input_dim, output_dim)
    pub layer_dims: Vec<(usize, usize)>,
    /// Lateral connection weights from previous columns
    /// lateral[layer][prev_col] = weight matrix
    pub lateral_weights: Vec<Vec<Vec<f64>>>,
    pub frozen: bool,
}

impl ProgressiveColumn {
    pub fn new(task_id: usize, layer_dims: &[(usize, usize)], seed: u64) -> Self {
        let mut rng = SimpleRng::new(seed);
        let mut layers = Vec::new();
        for &(inp, out) in layer_dims {
            let scale = (2.0 / inp as f64).sqrt();
            let weights: Vec<f64> = (0..inp * out).map(|_| {
                let u1 = rng.next_f64().max(1e-15);
                let u2 = rng.next_f64();
                (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() * scale
            }).collect();
            layers.push(weights);
        }
        Self {
            task_id,
            layers,
            layer_dims: layer_dims.to_vec(),
            lateral_weights: vec![Vec::new(); layer_dims.len()],
            frozen: false,
        }
    }

    /// Forward pass through this column (simple MLP)
    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        let mut activation = input.to_vec();
        for (layer_idx, weights) in self.layers.iter().enumerate() {
            let (inp, out) = self.layer_dims[layer_idx];
            let mut output = vec![0.0; out];
            for o in 0..out {
                let mut sum = 0.0;
                for i in 0..inp.min(activation.len()) {
                    sum += activation[i] * weights[i * out + o];
                }
                // ReLU activation
                output[o] = sum.max(0.0);
            }
            activation = output;
        }
        activation
    }

    /// Number of parameters
    pub fn param_count(&self) -> usize {
        self.layers.iter().map(|l| l.len()).sum::<usize>()
            + self.lateral_weights.iter().flat_map(|l| l.iter()).map(|w| w.len()).sum::<usize>()
    }
}

/// Progressive Neural Network: one column per task, with lateral connections
pub struct ProgressiveNet {
    pub columns: Vec<ProgressiveColumn>,
    pub layer_dims: Vec<(usize, usize)>,
}

impl ProgressiveNet {
    pub fn new(layer_dims: Vec<(usize, usize)>) -> Self {
        Self {
            columns: Vec::new(),
            layer_dims,
        }
    }

    /// Add a new column for a new task
    pub fn add_task(&mut self, seed: u64) -> usize {
        let task_id = self.columns.len();
        let mut col = ProgressiveColumn::new(task_id, &self.layer_dims, seed);

        // Initialize lateral connections from all previous columns
        for layer_idx in 0..self.layer_dims.len() {
            let (inp, out) = self.layer_dims[layer_idx];
            for _prev_col in 0..self.columns.len() {
                let lateral = vec![0.01; inp * out]; // small lateral init
                col.lateral_weights[layer_idx].push(lateral);
            }
        }

        // Freeze all previous columns
        for prev_col in &mut self.columns {
            prev_col.frozen = true;
        }

        self.columns.push(col);
        task_id
    }

    /// Number of tasks
    pub fn num_tasks(&self) -> usize {
        self.columns.len()
    }

    /// Total parameters across all columns
    pub fn total_params(&self) -> usize {
        self.columns.iter().map(|c| c.param_count()).sum()
    }

    /// Forward pass through a specific task's column
    pub fn forward(&self, task_id: usize, input: &[f64]) -> Vec<f64> {
        if task_id < self.columns.len() {
            self.columns[task_id].forward(input)
        } else {
            Vec::new()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3. Experience Replay
// ═══════════════════════════════════════════════════════════════════

/// A stored experience sample
#[derive(Debug, Clone)]
pub struct Experience {
    pub input: Vec<f64>,
    pub target: Vec<f64>,
    pub task_id: usize,
    pub priority: f64,
}

/// Experience replay buffer with reservoir sampling
pub struct ReplayBuffer {
    pub buffer: Vec<Experience>,
    pub capacity: usize,
    total_seen: usize,
    rng: SimpleRng,
}

impl ReplayBuffer {
    pub fn new(capacity: usize, seed: u64) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            total_seen: 0,
            rng: SimpleRng::new(seed),
        }
    }

    /// Add experience with reservoir sampling
    pub fn add(&mut self, exp: Experience) {
        self.total_seen += 1;
        if self.buffer.len() < self.capacity {
            self.buffer.push(exp);
        } else {
            // Reservoir sampling: replace with probability capacity/total_seen
            let j = self.rng.next_usize(self.total_seen);
            if j < self.capacity {
                self.buffer[j] = exp;
            }
        }
    }

    /// Sample a batch of experiences
    pub fn sample(&mut self, batch_size: usize) -> Vec<&Experience> {
        if self.buffer.is_empty() { return Vec::new(); }
        let mut batch = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let idx = self.rng.next_usize(self.buffer.len());
            batch.push(&self.buffer[idx]);
        }
        batch
    }

    /// Sample balanced across tasks
    pub fn sample_balanced(&mut self, per_task: usize) -> Vec<&Experience> {
        // Group indices by task
        let mut task_indices: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, exp) in self.buffer.iter().enumerate() {
            task_indices.entry(exp.task_id).or_default().push(i);
        }

        let mut batch = Vec::new();
        for (_task, indices) in &task_indices {
            for _ in 0..per_task.min(indices.len()) {
                let j = self.rng.next_usize(indices.len());
                batch.push(&self.buffer[indices[j]]);
            }
        }
        batch
    }

    /// Number of unique tasks in buffer
    pub fn num_tasks(&self) -> usize {
        let mut tasks = std::collections::HashSet::new();
        for exp in &self.buffer {
            tasks.insert(exp.task_id);
        }
        tasks.len()
    }

    /// Buffer utilization
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.buffer.len() as f64 / self.capacity as f64
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. Synaptic Intelligence (SI)
// ═══════════════════════════════════════════════════════════════════

/// Synaptic Intelligence: online importance estimation
pub struct SynapticIntelligence {
    /// Running sum of gradient * delta_weight per parameter
    omega_accumulator: Vec<f64>,
    /// Consolidated importance per parameter per task
    consolidated_importance: Vec<Vec<f64>>,
    /// Previous weights (at task start)
    prev_weights: Vec<f64>,
    /// Damping constant
    pub xi: f64,
    /// Regularization strength
    pub c: f64,
    pub num_tasks: usize,
}

impl SynapticIntelligence {
    pub fn new(n_params: usize, c: f64, xi: f64) -> Self {
        Self {
            omega_accumulator: vec![0.0; n_params],
            consolidated_importance: Vec::new(),
            prev_weights: vec![0.0; n_params],
            xi,
            c: c.max(0.0),
            num_tasks: 0,
        }
    }

    /// Call at the start of each task to record initial weights
    pub fn begin_task(&mut self, weights: &[f64]) {
        let n = weights.len().min(self.prev_weights.len());
        self.prev_weights[..n].copy_from_slice(&weights[..n]);
        // Reset accumulator
        for v in &mut self.omega_accumulator {
            *v = 0.0;
        }
    }

    /// Update accumulator during training: called after each step
    pub fn update_accumulator(&mut self, gradients: &[f64], weight_delta: &[f64]) {
        let n = gradients.len().min(weight_delta.len()).min(self.omega_accumulator.len());
        for i in 0..n {
            // omega_hat += -grad * delta_w
            self.omega_accumulator[i] += -gradients[i] * weight_delta[i];
        }
    }

    /// Consolidate at end of task
    pub fn end_task(&mut self, final_weights: &[f64]) {
        let n = final_weights.len().min(self.prev_weights.len()).min(self.omega_accumulator.len());
        let mut importance = vec![0.0; n];
        for i in 0..n {
            let delta = (final_weights[i] - self.prev_weights[i]).abs() + self.xi;
            importance[i] = self.omega_accumulator[i].max(0.0) / delta;
        }
        self.consolidated_importance.push(importance);
        self.num_tasks += 1;
    }

    /// Compute SI regularization penalty
    pub fn penalty(&self, current_weights: &[f64]) -> f64 {
        let mut total = 0.0;
        for task in 0..self.num_tasks {
            let ref_w = if task < self.consolidated_importance.len() {
                // Use prev_weights as approximation (simplified)
                &self.prev_weights
            } else {
                continue;
            };
            let omega = &self.consolidated_importance[task];
            let n = current_weights.len().min(ref_w.len()).min(omega.len());
            for i in 0..n {
                let diff = current_weights[i] - ref_w[i];
                total += omega[i] * diff * diff;
            }
        }
        self.c * total
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. Learning without Forgetting (LwF)
// ═══════════════════════════════════════════════════════════════════

/// Knowledge distillation loss for continual learning
pub struct LearningWithoutForgetting {
    /// Stored soft targets from previous task model
    stored_logits: Vec<Vec<f64>>,
    /// Temperature for softmax
    pub temperature: f64,
    /// Distillation loss weight
    pub alpha: f64,
}

impl LearningWithoutForgetting {
    pub fn new(temperature: f64, alpha: f64) -> Self {
        Self {
            stored_logits: Vec::new(),
            temperature: temperature.max(0.1),
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Record soft targets before starting new task training
    pub fn record_logits(&mut self, logits: Vec<Vec<f64>>) {
        self.stored_logits = logits;
    }

    /// Soft cross-entropy loss between current and stored logits
    pub fn distillation_loss(&self, current_logits: &[f64], sample_idx: usize) -> f64 {
        if sample_idx >= self.stored_logits.len() { return 0.0; }
        let old = &self.stored_logits[sample_idx];
        if old.is_empty() || current_logits.is_empty() { return 0.0; }

        let n = old.len().min(current_logits.len());

        // Softmax with temperature on old logits
        let old_softmax = Self::softmax_temp(&old[..n], self.temperature);
        // Log-softmax with temperature on current logits
        let current_softmax = Self::softmax_temp(&current_logits[..n], self.temperature);

        // KL divergence: Σ p_old * log(p_old / p_current)
        let mut loss = 0.0;
        for i in 0..n {
            if old_softmax[i] > 1e-10 && current_softmax[i] > 1e-10 {
                loss += old_softmax[i] * (old_softmax[i] / current_softmax[i]).ln();
            }
        }
        self.alpha * self.temperature * self.temperature * loss
    }

    fn softmax_temp(logits: &[f64], temp: f64) -> Vec<f64> {
        let max_val = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exps: Vec<f64> = logits.iter().map(|x| ((x - max_val) / temp).exp()).collect();
        let sum: f64 = exps.iter().sum();
        if sum > 0.0 {
            exps.iter().map(|e| e / sum).collect()
        } else {
            vec![1.0 / logits.len() as f64; logits.len()]
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 6. Task Manager
// ═══════════════════════════════════════════════════════════════════

/// Scenario type for continual learning
#[derive(Debug, Clone, PartialEq)]
pub enum Scenario {
    TaskIncremental,   // task ID known at inference
    ClassIncremental,  // must distinguish all classes seen so far
    DomainIncremental, // same classes, different domains
}

/// Metrics for tracking continual learning performance
#[derive(Debug, Clone)]
pub struct ContinualMetrics {
    /// Accuracy per task after each task is learned
    pub accuracy_matrix: Vec<Vec<f64>>,  // [task_learned][task_evaluated]
    pub current_task: usize,
}

impl ContinualMetrics {
    pub fn new() -> Self {
        Self {
            accuracy_matrix: Vec::new(),
            current_task: 0,
        }
    }

    /// Record accuracy on all tasks after learning task t
    pub fn record(&mut self, accuracies: Vec<f64>) {
        self.accuracy_matrix.push(accuracies);
        self.current_task = self.accuracy_matrix.len();
    }

    /// Average accuracy across all seen tasks (after learning all)
    pub fn average_accuracy(&self) -> f64 {
        if self.accuracy_matrix.is_empty() { return 0.0; }
        let last = self.accuracy_matrix.last().unwrap();
        if last.is_empty() { return 0.0; }
        last.iter().sum::<f64>() / last.len() as f64
    }

    /// Backward Transfer (BWT): how much learning new tasks hurts old ones
    /// BWT = 1/(T-1) * Σ_{i<T} (R_{T,i} - R_{i,i})
    pub fn backward_transfer(&self) -> f64 {
        let t = self.accuracy_matrix.len();
        if t < 2 { return 0.0; }
        let mut sum = 0.0;
        let count = (t - 1) as f64;
        let last = &self.accuracy_matrix[t - 1];
        for i in 0..(t - 1) {
            let r_ti = if i < last.len() { last[i] } else { 0.0 };
            let r_ii = if i < self.accuracy_matrix[i].len() {
                self.accuracy_matrix[i][i]
            } else {
                0.0
            };
            sum += r_ti - r_ii;
        }
        sum / count
    }

    /// Forward Transfer (FWT): how much prior learning helps new tasks
    pub fn forward_transfer(&self, baseline_accuracies: &[f64]) -> f64 {
        let t = self.accuracy_matrix.len();
        if t < 2 { return 0.0; }
        let mut sum = 0.0;
        let mut count = 0.0;
        for i in 1..t {
            if i < self.accuracy_matrix.len() && i - 1 < self.accuracy_matrix[i - 1].len() {
                let r_prev_i = if i < self.accuracy_matrix[i - 1].len() {
                    self.accuracy_matrix[i - 1][i]
                } else {
                    0.0
                };
                let baseline = if i < baseline_accuracies.len() { baseline_accuracies[i] } else { 0.0 };
                sum += r_prev_i - baseline;
                count += 1.0;
            }
        }
        if count > 0.0 { sum / count } else { 0.0 }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 7. Continual Learning System
// ═══════════════════════════════════════════════════════════════════

/// Enum of available continual learning strategies
#[derive(Debug, Clone, PartialEq)]
pub enum Strategy {
    Naive,                // fine-tuning without protection
    EWC { lambda: f64 },
    SI { c: f64 },
    Replay { buffer_size: usize },
    Progressive,
    LwF { temperature: f64, alpha: f64 },
}

/// Top-level continual learning system
pub struct ContinualLearner {
    pub strategy: Strategy,
    pub scenario: Scenario,
    pub metrics: ContinualMetrics,
    pub num_tasks_seen: usize,
    ewc: Option<ElasticWeightConsolidation>,
    si: Option<SynapticIntelligence>,
    replay: Option<ReplayBuffer>,
    progressive: Option<ProgressiveNet>,
    lwf: Option<LearningWithoutForgetting>,
}

impl ContinualLearner {
    pub fn new(strategy: Strategy, scenario: Scenario) -> Self {
        let ewc = match &strategy {
            Strategy::EWC { lambda } => Some(ElasticWeightConsolidation::new(*lambda)),
            _ => None,
        };
        let si = match &strategy {
            Strategy::SI { c } => Some(SynapticIntelligence::new(0, *c, 0.1)),
            _ => None,
        };
        let replay = match &strategy {
            Strategy::Replay { buffer_size } => Some(ReplayBuffer::new(*buffer_size, 42)),
            _ => None,
        };
        let progressive = match &strategy {
            Strategy::Progressive => Some(ProgressiveNet::new(vec![(784, 256), (256, 128), (128, 10)])),
            _ => None,
        };
        let lwf = match &strategy {
            Strategy::LwF { temperature, alpha } => Some(LearningWithoutForgetting::new(*temperature, *alpha)),
            _ => None,
        };

        Self {
            strategy,
            scenario,
            metrics: ContinualMetrics::new(),
            num_tasks_seen: 0,
            ewc,
            si,
            replay,
            progressive,
            lwf,
        }
    }

    /// Begin a new task
    pub fn begin_task(&mut self, weights: &[f64]) {
        if let Some(ref mut si) = self.si {
            if si.omega_accumulator.len() != weights.len() {
                *si = SynapticIntelligence::new(weights.len(), si.c, si.xi);
            }
            si.begin_task(weights);
        }
        if let Some(ref mut net) = self.progressive {
            net.add_task(self.num_tasks_seen as u64 + 42);
        }
    }

    /// End current task
    pub fn end_task(&mut self, weights: &[f64], gradient_samples: &[Vec<f64>]) {
        if let Some(ref mut ewc) = self.ewc {
            ewc.consolidate(weights, gradient_samples);
        }
        if let Some(ref mut si) = self.si {
            si.end_task(weights);
        }
        self.num_tasks_seen += 1;
    }

    /// Get regularization penalty for current weights
    pub fn regularization_penalty(&self, weights: &[f64]) -> f64 {
        match &self.strategy {
            Strategy::EWC { .. } => self.ewc.as_ref().map(|e| e.penalty(weights)).unwrap_or(0.0),
            Strategy::SI { .. } => self.si.as_ref().map(|s| s.penalty(weights)).unwrap_or(0.0),
            _ => 0.0,
        }
    }

    /// Store experience for replay
    pub fn store_experience(&mut self, input: Vec<f64>, target: Vec<f64>, task_id: usize) {
        if let Some(ref mut buf) = self.replay {
            buf.add(Experience { input, target, task_id, priority: 1.0 });
        }
    }

    /// Get replay batch
    pub fn replay_batch(&mut self, batch_size: usize) -> Vec<(Vec<f64>, Vec<f64>)> {
        if let Some(ref mut buf) = self.replay {
            buf.sample(batch_size)
                .iter()
                .map(|e| (e.input.clone(), e.target.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Summary of the learner state
    pub fn summary(&self) -> String {
        format!(
            "ContinualLearner(strategy={:?}, scenario={:?}, tasks_seen={}, avg_accuracy={:.4}, bwt={:.4})",
            self.strategy, self.scenario, self.num_tasks_seen,
            self.metrics.average_accuracy(), self.metrics.backward_transfer()
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// 8. FFI Interface
// ═══════════════════════════════════════════════════════════════════

static CL_STORE: Mutex<Option<HashMap<i64, ContinualLearner>>> = Mutex::new(None);

fn cl_store_init() -> std::sync::MutexGuard<'static, Option<HashMap<i64, ContinualLearner>>> {
    let mut guard = CL_STORE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

/// Create a continual learner with EWC strategy
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_cl_create_ewc(lambda: f64) -> i64 {
    let mut store = cl_store_init();
    let map = store.as_mut().unwrap();
    let id = map.len() as i64 + 1;
    let learner = ContinualLearner::new(
        Strategy::EWC { lambda },
        Scenario::TaskIncremental,
    );
    map.insert(id, learner);
    id
}

/// Create a continual learner with replay strategy
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_cl_create_replay(buffer_size: i64) -> i64 {
    let mut store = cl_store_init();
    let map = store.as_mut().unwrap();
    let id = map.len() as i64 + 1;
    let learner = ContinualLearner::new(
        Strategy::Replay { buffer_size: buffer_size.max(10) as usize },
        Scenario::TaskIncremental,
    );
    map.insert(id, learner);
    id
}

/// Get number of tasks seen
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_cl_tasks_seen(id: i64) -> i64 {
    let store = cl_store_init();
    let map = store.as_ref().unwrap();
    map.get(&id).map(|l| l.num_tasks_seen as i64).unwrap_or(-1)
}

/// Get average accuracy
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_cl_avg_accuracy(id: i64) -> f64 {
    let store = cl_store_init();
    let map = store.as_ref().unwrap();
    map.get(&id).map(|l| l.metrics.average_accuracy()).unwrap_or(-1.0)
}

/// Free a continual learner
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_cl_free(id: i64) -> i64 {
    let mut store = cl_store_init();
    let map = store.as_mut().unwrap();
    if map.remove(&id).is_some() { 1 } else { 0 }
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ewc_consolidate() {
        let mut ewc = ElasticWeightConsolidation::new(1000.0);
        let weights = vec![0.5, -0.3, 0.8];
        let grads = vec![
            vec![0.1, 0.2, 0.3],
            vec![0.15, 0.25, 0.05],
        ];
        ewc.consolidate(&weights, &grads);
        assert_eq!(ewc.num_tasks, 1);
        // No penalty at same weights
        let penalty = ewc.penalty(&weights);
        assert!(penalty.abs() < 1e-10);
    }

    #[test]
    fn test_ewc_penalty() {
        let mut ewc = ElasticWeightConsolidation::new(100.0);
        let old_weights = vec![1.0, 2.0, 3.0];
        let grads = vec![vec![1.0, 1.0, 1.0]]; // uniform importance
        ewc.consolidate(&old_weights, &grads);

        let new_weights = vec![1.1, 2.0, 3.0]; // small change in first weight
        let penalty = ewc.penalty(&new_weights);
        assert!(penalty > 0.0);
    }

    #[test]
    fn test_ewc_gradient() {
        let mut ewc = ElasticWeightConsolidation::new(10.0);
        let weights = vec![1.0, 2.0];
        let grads = vec![vec![1.0, 1.0]];
        ewc.consolidate(&weights, &grads);

        let grad = ewc.penalty_gradient(&vec![1.5, 2.0]);
        assert!(grad[0].abs() > 0.0); // gradient should push back
        assert!(grad[1].abs() < 1e-10); // no change in weight 2
    }

    #[test]
    fn test_progressive_net() {
        let mut net = ProgressiveNet::new(vec![(4, 8), (8, 4)]);
        let t0 = net.add_task(42);
        assert_eq!(t0, 0);
        assert_eq!(net.num_tasks(), 1);

        let t1 = net.add_task(43);
        assert_eq!(t1, 1);
        assert!(net.columns[0].frozen);
        assert!(!net.columns[1].frozen);
    }

    #[test]
    fn test_progressive_forward() {
        let mut net = ProgressiveNet::new(vec![(4, 8), (8, 2)]);
        net.add_task(42);
        let output = net.forward(0, &[1.0, 0.5, -0.3, 0.8]);
        assert_eq!(output.len(), 2); // output dim = 2
    }

    #[test]
    fn test_replay_buffer() {
        let mut buf = ReplayBuffer::new(5, 42);
        for i in 0..10 {
            buf.add(Experience {
                input: vec![i as f64],
                target: vec![0.0],
                task_id: i % 3,
                priority: 1.0,
            });
        }
        assert_eq!(buf.buffer.len(), 5); // capped at capacity
        assert_eq!(buf.total_seen, 10);
        assert!(buf.utilization() > 0.99);
    }

    #[test]
    fn test_replay_sample() {
        let mut buf = ReplayBuffer::new(100, 42);
        for i in 0..50 {
            buf.add(Experience {
                input: vec![i as f64],
                target: vec![(i % 5) as f64],
                task_id: i % 3,
                priority: 1.0,
            });
        }
        let batch = buf.sample(10);
        assert_eq!(batch.len(), 10);
    }

    #[test]
    fn test_synaptic_intelligence() {
        let mut si = SynapticIntelligence::new(3, 1.0, 0.1);
        let init_weights = vec![0.0, 0.0, 0.0];
        si.begin_task(&init_weights);

        // Simulate training updates
        si.update_accumulator(&[0.5, 0.3, 0.1], &[-0.01, -0.02, -0.005]);
        si.update_accumulator(&[0.4, 0.2, 0.08], &[-0.008, -0.015, -0.003]);

        let final_weights = vec![0.1, 0.2, 0.05];
        si.end_task(&final_weights);
        assert_eq!(si.num_tasks, 1);
    }

    #[test]
    fn test_lwf_distillation() {
        let mut lwf = LearningWithoutForgetting::new(2.0, 0.5);
        lwf.record_logits(vec![
            vec![1.0, 2.0, 0.5],
            vec![0.3, 1.5, 2.0],
        ]);
        let loss = lwf.distillation_loss(&[1.0, 2.0, 0.5], 0);
        assert!(loss < 0.01); // same logits → near-zero loss
    }

    #[test]
    fn test_lwf_divergent() {
        let mut lwf = LearningWithoutForgetting::new(2.0, 0.5);
        lwf.record_logits(vec![vec![5.0, 1.0, 0.1]]);
        let loss = lwf.distillation_loss(&[0.1, 1.0, 5.0], 0);
        assert!(loss > 0.0); // divergent logits → positive loss
    }

    #[test]
    fn test_continual_metrics() {
        let mut metrics = ContinualMetrics::new();
        metrics.record(vec![0.95]);                    // after task 0
        metrics.record(vec![0.90, 0.92]);             // after task 1
        metrics.record(vec![0.85, 0.88, 0.93]);      // after task 2

        let avg = metrics.average_accuracy();
        assert!(avg > 0.0 && avg < 1.0);

        let bwt = metrics.backward_transfer();
        // bwt should be negative (forgetting)
        assert!(bwt < 0.0);
    }

    #[test]
    fn test_continual_learner_ewc() {
        let mut learner = ContinualLearner::new(
            Strategy::EWC { lambda: 100.0 },
            Scenario::TaskIncremental,
        );
        let weights = vec![0.5, -0.3, 0.8, 0.1];
        learner.begin_task(&weights);
        let grads = vec![vec![0.1, 0.2, 0.3, 0.05]];
        learner.end_task(&weights, &grads);
        assert_eq!(learner.num_tasks_seen, 1);
        let penalty = learner.regularization_penalty(&weights);
        assert!(penalty.abs() < 1e-10);
    }

    #[test]
    fn test_continual_learner_replay() {
        let mut learner = ContinualLearner::new(
            Strategy::Replay { buffer_size: 100 },
            Scenario::ClassIncremental,
        );
        for i in 0..20 {
            learner.store_experience(vec![i as f64], vec![0.0], 0);
        }
        let batch = learner.replay_batch(5);
        assert_eq!(batch.len(), 5);
    }

    #[test]
    fn test_ffi_create_and_free() {
        let id = vitalis_cl_create_ewc(100.0);
        assert!(id > 0);
        assert_eq!(vitalis_cl_tasks_seen(id), 0);
        assert_eq!(vitalis_cl_free(id), 1);
        assert_eq!(vitalis_cl_free(id), 0); // already freed
    }

    #[test]
    fn test_ffi_replay() {
        let id = vitalis_cl_create_replay(50);
        assert!(id > 0);
        assert_eq!(vitalis_cl_tasks_seen(id), 0);
        assert_eq!(vitalis_cl_free(id), 1);
    }

    #[test]
    fn test_scenario_enum() {
        assert_ne!(Scenario::TaskIncremental, Scenario::ClassIncremental);
        assert_ne!(Scenario::ClassIncremental, Scenario::DomainIncremental);
    }

    #[test]
    fn test_strategy_enum() {
        let s1 = Strategy::EWC { lambda: 100.0 };
        let s2 = Strategy::SI { c: 1.0 };
        assert_ne!(s1, s2);
        assert_eq!(s1, Strategy::EWC { lambda: 100.0 });
    }
}
