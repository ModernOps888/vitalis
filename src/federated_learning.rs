//! Federated Learning Module — Vitalis v44.0
//!
//! Privacy-preserving distributed training:
//! - FedAvg (Federated Averaging) algorithm
//! - FedProx (proximal term for heterogeneous data)
//! - Differential Privacy (Gaussian mechanism, gradient clipping)
//! - Secure Aggregation (simulated secret sharing)
//! - Client selection strategies (random, power-of-choice)
//! - Communication compression (top-k sparsification, quantization)
//! - Non-IID data handling and convergence tracking
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
    fn next_gaussian(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-15);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

// ═══════════════════════════════════════════════════════════════════
// 1. Differential Privacy
// ═══════════════════════════════════════════════════════════════════

/// Differential privacy mechanism for gradient protection
pub struct DifferentialPrivacy {
    /// Privacy budget epsilon
    pub epsilon: f64,
    /// Privacy loss parameter delta
    pub delta: f64,
    /// Gradient clipping norm (L2)
    pub clip_norm: f64,
    /// Noise multiplier (sigma)
    pub noise_multiplier: f64,
    /// Accumulated privacy spent (epsilon tracking)
    pub total_epsilon_spent: f64,
    /// Number of queries made
    pub num_queries: usize,
    rng: SimpleRng,
}

impl DifferentialPrivacy {
    pub fn new(epsilon: f64, delta: f64, clip_norm: f64, seed: u64) -> Self {
        // Compute noise multiplier from epsilon and delta
        // Using Gaussian mechanism: sigma >= sqrt(2 * ln(1.25/delta)) * sensitivity / epsilon
        let sensitivity = clip_norm; // L2 sensitivity = clip_norm
        let noise_mult = (2.0 * (1.25 / delta).ln()).sqrt() * sensitivity / epsilon;
        Self {
            epsilon: epsilon.max(0.01),
            delta: delta.max(1e-10),
            clip_norm: clip_norm.max(0.01),
            noise_multiplier: noise_mult,
            total_epsilon_spent: 0.0,
            num_queries: 0,
            rng: SimpleRng::new(seed),
        }
    }

    /// Clip gradient to L2 norm bound
    pub fn clip_gradient(&self, gradient: &mut [f64]) {
        let norm: f64 = gradient.iter().map(|g| g * g).sum::<f64>().sqrt();
        if norm > self.clip_norm {
            let scale = self.clip_norm / norm;
            for g in gradient.iter_mut() {
                *g *= scale;
            }
        }
    }

    /// Add calibrated Gaussian noise to gradient
    pub fn add_noise(&mut self, gradient: &mut [f64]) {
        for g in gradient.iter_mut() {
            *g += self.noise_multiplier * self.rng.next_gaussian();
        }
        self.num_queries += 1;
        // Simple composition: accumulate epsilon
        self.total_epsilon_spent += self.epsilon / (self.num_queries as f64).sqrt();
    }

    /// Full DP mechanism: clip + noise
    pub fn privatize(&mut self, gradient: &mut [f64]) {
        self.clip_gradient(gradient);
        self.add_noise(gradient);
    }

    /// Check if privacy budget is exhausted
    pub fn budget_exhausted(&self) -> bool {
        self.total_epsilon_spent >= self.epsilon * 10.0 // generous budget
    }

    /// Remaining privacy budget fraction
    pub fn budget_remaining(&self) -> f64 {
        let budget = self.epsilon * 10.0;
        (1.0 - self.total_epsilon_spent / budget).max(0.0)
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. Communication Compression
// ═══════════════════════════════════════════════════════════════════

/// Compression methods for gradient communication
pub struct GradientCompressor;

impl GradientCompressor {
    /// Top-K sparsification: keep only the K largest magnitude entries
    pub fn top_k(gradient: &[f64], k: usize) -> Vec<(usize, f64)> {
        if gradient.is_empty() || k == 0 { return Vec::new(); }
        let k = k.min(gradient.len());

        let mut indexed: Vec<(usize, f64)> = gradient.iter().enumerate()
            .map(|(i, &v)| (i, v))
            .collect();
        indexed.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
        indexed.truncate(k);
        indexed
    }

    /// Reconstruct full gradient from sparse representation
    pub fn from_sparse(sparse: &[(usize, f64)], dim: usize) -> Vec<f64> {
        let mut full = vec![0.0; dim];
        for &(i, v) in sparse {
            if i < dim {
                full[i] = v;
            }
        }
        full
    }

    /// Stochastic quantization to k bits
    pub fn quantize(gradient: &[f64], bits: u32) -> Vec<f64> {
        if gradient.is_empty() { return Vec::new(); }
        let levels = (1u64 << bits) as f64;
        let min_val = gradient.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = gradient.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = (max_val - min_val).max(1e-10);

        gradient.iter().map(|&v| {
            let normalized = (v - min_val) / range;
            let quantized = (normalized * levels).round() / levels;
            min_val + quantized * range
        }).collect()
    }

    /// Compression ratio for top-k
    pub fn compression_ratio(original_size: usize, k: usize) -> f64 {
        if original_size == 0 { return 0.0; }
        // Each sparse entry = (index, value) = 2 values vs 1 value per dense entry
        let sparse_cost = k * 2;
        if sparse_cost >= original_size {
            1.0
        } else {
            sparse_cost as f64 / original_size as f64
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3. Federated Client
// ═══════════════════════════════════════════════════════════════════

/// A participating client in federated learning
#[derive(Debug, Clone)]
pub struct FedClient {
    pub id: usize,
    pub data_size: usize,
    /// Local model weights
    pub weights: Vec<f64>,
    /// Training loss history
    pub loss_history: Vec<f64>,
    /// Communication rounds participated
    pub rounds_participated: usize,
    /// Is this client available?
    pub available: bool,
    /// Computation speed factor (1.0 = normal)
    pub speed_factor: f64,
}

impl FedClient {
    pub fn new(id: usize, data_size: usize, n_params: usize, seed: u64) -> Self {
        let mut rng = SimpleRng::new(seed + id as u64);
        let weights: Vec<f64> = (0..n_params).map(|_| rng.next_gaussian() * 0.01).collect();
        Self {
            id,
            data_size,
            weights,
            loss_history: Vec::new(),
            rounds_participated: 0,
            available: true,
            speed_factor: 1.0,
        }
    }

    /// Simulate local training: gradient descent steps
    pub fn local_train(&mut self, global_weights: &[f64], local_epochs: usize, lr: f64, seed: u64) -> Vec<f64> {
        let n = global_weights.len().min(self.weights.len());
        self.weights[..n].copy_from_slice(&global_weights[..n]);

        let mut rng = SimpleRng::new(seed);
        let mut loss = 0.0;

        for _ in 0..local_epochs {
            // Simulate gradient computation (synthetic)
            let mut gradient = vec![0.0; n];
            for i in 0..n {
                // Simple synthetic loss gradient: weight decay + noise
                gradient[i] = self.weights[i] * 0.01 + rng.next_gaussian() * 0.1;
            }
            // SGD step
            for i in 0..n {
                self.weights[i] -= lr * gradient[i];
            }
            loss = gradient.iter().map(|g| g * g).sum::<f64>() / n as f64;
        }

        self.loss_history.push(loss);
        self.rounds_participated += 1;

        // Return weight update (delta)
        let mut delta = vec![0.0; n];
        for i in 0..n {
            delta[i] = self.weights[i] - global_weights[i];
        }
        delta
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. Client Selection
// ═══════════════════════════════════════════════════════════════════

/// Strategy for selecting clients each round
#[derive(Debug, Clone, PartialEq)]
pub enum SelectionStrategy {
    Random,
    PowerOfChoice { d: usize },  // select best of d random candidates
    All,
}

pub struct ClientSelector;

impl ClientSelector {
    /// Select clients for a round
    pub fn select(
        clients: &[FedClient],
        num_select: usize,
        strategy: &SelectionStrategy,
        seed: u64,
    ) -> Vec<usize> {
        let mut rng = SimpleRng::new(seed);
        let available: Vec<usize> = clients.iter()
            .filter(|c| c.available)
            .map(|c| c.id)
            .collect();

        if available.is_empty() { return Vec::new(); }
        let num = num_select.min(available.len());

        match strategy {
            SelectionStrategy::All => available,
            SelectionStrategy::Random => {
                let mut selected = Vec::with_capacity(num);
                let mut remaining = available;
                for _ in 0..num {
                    if remaining.is_empty() { break; }
                    let idx = rng.next_usize(remaining.len());
                    selected.push(remaining.remove(idx));
                }
                selected
            }
            SelectionStrategy::PowerOfChoice { d } => {
                // Sample d candidates, pick those with most data
                let mut selected = Vec::with_capacity(num);
                let mut remaining = available;
                for _ in 0..num {
                    if remaining.is_empty() { break; }
                    let candidates: Vec<usize> = (0..(*d).min(remaining.len()))
                        .map(|_| rng.next_usize(remaining.len()))
                        .collect();
                    // Pick the candidate with largest data size
                    let best_idx = candidates.iter()
                        .max_by_key(|&&idx| clients.get(remaining[idx]).map(|c| c.data_size).unwrap_or(0))
                        .copied()
                        .unwrap_or(0);
                    if best_idx < remaining.len() {
                        selected.push(remaining.remove(best_idx));
                    }
                }
                selected
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. Aggregation Strategies
// ═══════════════════════════════════════════════════════════════════

/// Aggregation method for combining client updates
#[derive(Debug, Clone, PartialEq)]
pub enum AggregationMethod {
    FedAvg,             // weighted average by data size
    FedProx { mu: f64 }, // proximal FedAvg
    SimpleAvg,          // unweighted average
    Median,             // coordinate-wise median (Byzantine-resilient)
}

pub struct Aggregator;

impl Aggregator {
    /// FedAvg: weighted average of updates by data size
    pub fn fedavg(updates: &[Vec<f64>], data_sizes: &[usize], n_params: usize) -> Vec<f64> {
        let total_data: f64 = data_sizes.iter().sum::<usize>() as f64;
        if total_data < 1.0 || updates.is_empty() { return vec![0.0; n_params]; }

        let mut aggregated = vec![0.0; n_params];
        for (update, &size) in updates.iter().zip(data_sizes.iter()) {
            let weight = size as f64 / total_data;
            for i in 0..n_params.min(update.len()) {
                aggregated[i] += weight * update[i];
            }
        }
        aggregated
    }

    /// Simple unweighted average
    pub fn simple_avg(updates: &[Vec<f64>], n_params: usize) -> Vec<f64> {
        if updates.is_empty() { return vec![0.0; n_params]; }
        let n = updates.len() as f64;
        let mut aggregated = vec![0.0; n_params];
        for update in updates {
            for i in 0..n_params.min(update.len()) {
                aggregated[i] += update[i] / n;
            }
        }
        aggregated
    }

    /// Coordinate-wise median (Byzantine-resilient)
    pub fn median(updates: &[Vec<f64>], n_params: usize) -> Vec<f64> {
        if updates.is_empty() { return vec![0.0; n_params]; }
        let mut aggregated = vec![0.0; n_params];
        for i in 0..n_params {
            let mut values: Vec<f64> = updates.iter()
                .filter_map(|u| u.get(i).copied())
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            if values.is_empty() { continue; }
            aggregated[i] = if values.len() % 2 == 0 {
                (values[values.len() / 2 - 1] + values[values.len() / 2]) / 2.0
            } else {
                values[values.len() / 2]
            };
        }
        aggregated
    }
}

// ═══════════════════════════════════════════════════════════════════
// 6. Secure Aggregation (Simulated)
// ═══════════════════════════════════════════════════════════════════

/// Simulated secure aggregation using additive secret sharing
pub struct SecureAggregation {
    /// Pairwise masks for secret sharing
    masks: HashMap<(usize, usize), Vec<f64>>,
    rng: SimpleRng,
}

impl SecureAggregation {
    pub fn new(seed: u64) -> Self {
        Self {
            masks: HashMap::new(),
            rng: SimpleRng::new(seed),
        }
    }

    /// Generate pairwise masks for a set of clients
    pub fn generate_masks(&mut self, client_ids: &[usize], n_params: usize) {
        self.masks.clear();
        for i in 0..client_ids.len() {
            for j in (i + 1)..client_ids.len() {
                let mask: Vec<f64> = (0..n_params).map(|_| self.rng.next_gaussian()).collect();
                self.masks.insert((client_ids[i], client_ids[j]), mask);
            }
        }
    }

    /// Apply mask to a client's update (for sending)
    pub fn mask_update(&self, client_id: usize, update: &[f64], all_ids: &[usize]) -> Vec<f64> {
        let mut masked = update.to_vec();
        for &other_id in all_ids {
            if other_id == client_id { continue; }
            let key = if client_id < other_id {
                (client_id, other_id)
            } else {
                (other_id, client_id)
            };
            if let Some(mask) = self.masks.get(&key) {
                let sign = if client_id < other_id { 1.0 } else { -1.0 };
                for i in 0..masked.len().min(mask.len()) {
                    masked[i] += sign * mask[i];
                }
            }
        }
        masked
    }

    /// Number of mask pairs generated
    pub fn num_masks(&self) -> usize {
        self.masks.len()
    }
}

// ═══════════════════════════════════════════════════════════════════
// 7. Federated Learning Server
// ═══════════════════════════════════════════════════════════════════

/// Central server coordinating federated learning
pub struct FedServer {
    /// Global model weights
    pub global_weights: Vec<f64>,
    /// Participating clients
    pub clients: Vec<FedClient>,
    /// Current communication round
    pub round: usize,
    /// Aggregation method
    pub aggregation: AggregationMethod,
    /// Client selection strategy
    pub selection: SelectionStrategy,
    /// Clients per round
    pub clients_per_round: usize,
    /// Local epochs per client
    pub local_epochs: usize,
    /// Learning rate
    pub lr: f64,
    /// Global loss history
    pub loss_history: Vec<f64>,
    /// Differential privacy (optional)
    pub dp: Option<DifferentialPrivacy>,
    /// Secure aggregation (optional)
    pub secure_agg: Option<SecureAggregation>,
    rng: SimpleRng,
}

impl FedServer {
    pub fn new(n_params: usize, aggregation: AggregationMethod, seed: u64) -> Self {
        let mut rng = SimpleRng::new(seed);
        let global_weights: Vec<f64> = (0..n_params).map(|_| rng.next_gaussian() * 0.01).collect();
        Self {
            global_weights,
            clients: Vec::new(),
            round: 0,
            aggregation,
            selection: SelectionStrategy::Random,
            clients_per_round: 10,
            local_epochs: 5,
            lr: 0.01,
            loss_history: Vec::new(),
            dp: None,
            secure_agg: None,
            rng,
        }
    }

    /// Add a client to the federation
    pub fn add_client(&mut self, data_size: usize) -> usize {
        let id = self.clients.len();
        let n = self.global_weights.len();
        let client = FedClient::new(id, data_size, n, self.rng.next_u64());
        self.clients.push(client);
        id
    }

    /// Enable differential privacy
    pub fn enable_dp(&mut self, epsilon: f64, delta: f64, clip_norm: f64) {
        self.dp = Some(DifferentialPrivacy::new(epsilon, delta, clip_norm, self.rng.next_u64()));
    }

    /// Enable secure aggregation
    pub fn enable_secure_agg(&mut self) {
        self.secure_agg = Some(SecureAggregation::new(self.rng.next_u64()));
    }

    /// Run one communication round
    pub fn run_round(&mut self) {
        let seed = self.rng.next_u64();
        let n_params = self.global_weights.len();

        // 1. Select clients
        let selected = ClientSelector::select(
            &self.clients,
            self.clients_per_round,
            &self.selection,
            seed,
        );

        if selected.is_empty() { return; }

        // 2. Generate secure aggregation masks if enabled
        if let Some(ref mut sa) = self.secure_agg {
            sa.generate_masks(&selected, n_params);
        }

        // 3. Local training on each selected client
        let mut updates = Vec::new();
        let mut data_sizes = Vec::new();
        let global_clone = self.global_weights.clone();

        for &cid in &selected {
            if cid >= self.clients.len() { continue; }
            let local_seed = seed.wrapping_add(cid as u64);
            let mut delta = self.clients[cid].local_train(
                &global_clone,
                self.local_epochs,
                self.lr,
                local_seed,
            );

            // 4. Apply DP if enabled
            if let Some(ref mut dp) = self.dp {
                dp.privatize(&mut delta);
            }

            data_sizes.push(self.clients[cid].data_size);
            updates.push(delta);
        }

        // 5. Aggregate updates
        let aggregated = match &self.aggregation {
            AggregationMethod::FedAvg | AggregationMethod::FedProx { .. } => {
                Aggregator::fedavg(&updates, &data_sizes, n_params)
            }
            AggregationMethod::SimpleAvg => {
                Aggregator::simple_avg(&updates, n_params)
            }
            AggregationMethod::Median => {
                Aggregator::median(&updates, n_params)
            }
        };

        // 6. Apply proximal term if FedProx
        if let AggregationMethod::FedProx { mu } = &self.aggregation {
            let mu = *mu;
            for i in 0..n_params {
                self.global_weights[i] += aggregated[i] - mu * aggregated[i];
            }
        } else {
            // Standard update
            for i in 0..n_params {
                self.global_weights[i] += aggregated[i];
            }
        }

        // Track loss
        let avg_loss: f64 = if !updates.is_empty() {
            updates.iter().map(|u| {
                u.iter().map(|v| v * v).sum::<f64>() / u.len() as f64
            }).sum::<f64>() / updates.len() as f64
        } else {
            0.0
        };
        self.loss_history.push(avg_loss);
        self.round += 1;
    }

    /// Run multiple rounds
    pub fn train(&mut self, rounds: usize) {
        for _ in 0..rounds {
            self.run_round();
        }
    }

    /// Get current global loss (last recorded)
    pub fn current_loss(&self) -> f64 {
        self.loss_history.last().copied().unwrap_or(f64::MAX)
    }

    /// Convergence check: loss hasn't improved for `patience` rounds
    pub fn has_converged(&self, patience: usize, min_delta: f64) -> bool {
        if self.loss_history.len() < patience + 1 { return false; }
        let recent = &self.loss_history[self.loss_history.len() - patience..];
        let oldest = recent[0];
        recent.iter().all(|&l| (oldest - l).abs() < min_delta)
    }

    /// Number of clients
    pub fn num_clients(&self) -> usize {
        self.clients.len()
    }

    /// Summary
    pub fn summary(&self) -> String {
        format!(
            "FedServer(round={}, clients={}, params={}, loss={:.6}, method={:?})",
            self.round, self.clients.len(), self.global_weights.len(),
            self.current_loss(), self.aggregation
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// 8. FFI Interface
// ═══════════════════════════════════════════════════════════════════

static FED_STORE: Mutex<Option<HashMap<i64, FedServer>>> = Mutex::new(None);

fn fed_store_init() -> std::sync::MutexGuard<'static, Option<HashMap<i64, FedServer>>> {
    let mut guard = FED_STORE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

/// Create a federated learning server
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_fed_create(n_params: i64, seed: i64) -> i64 {
    let mut store = fed_store_init();
    let map = store.as_mut().unwrap();
    let id = map.len() as i64 + 1;
    let server = FedServer::new(n_params.max(1) as usize, AggregationMethod::FedAvg, seed as u64);
    map.insert(id, server);
    id
}

/// Add a client to the federation
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_fed_add_client(id: i64, data_size: i64) -> i64 {
    let mut store = fed_store_init();
    let map = store.as_mut().unwrap();
    if let Some(server) = map.get_mut(&id) {
        server.add_client(data_size.max(1) as usize) as i64
    } else {
        -1
    }
}

/// Run a training round
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_fed_train_round(id: i64) -> i64 {
    let mut store = fed_store_init();
    let map = store.as_mut().unwrap();
    if let Some(server) = map.get_mut(&id) {
        server.run_round();
        server.round as i64
    } else {
        -1
    }
}

/// Get current round
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_fed_round(id: i64) -> i64 {
    let store = fed_store_init();
    let map = store.as_ref().unwrap();
    map.get(&id).map(|s| s.round as i64).unwrap_or(-1)
}

/// Get number of clients
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_fed_num_clients(id: i64) -> i64 {
    let store = fed_store_init();
    let map = store.as_ref().unwrap();
    map.get(&id).map(|s| s.num_clients() as i64).unwrap_or(-1)
}

/// Free a federated server
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_fed_free(id: i64) -> i64 {
    let mut store = fed_store_init();
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
    fn test_dp_clip_gradient() {
        let dp = DifferentialPrivacy::new(1.0, 1e-5, 1.0, 42);
        let mut grad = vec![3.0, 4.0]; // norm = 5.0
        dp.clip_gradient(&mut grad);
        let norm: f64 = grad.iter().map(|g| g * g).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6); // clipped to norm 1.0
    }

    #[test]
    fn test_dp_add_noise() {
        let mut dp = DifferentialPrivacy::new(1.0, 1e-5, 1.0, 42);
        let mut grad = vec![1.0, 2.0, 3.0];
        let original = grad.clone();
        dp.add_noise(&mut grad);
        // At least one value should differ after noise
        let any_different = grad.iter().zip(&original).any(|(a, b)| (a - b).abs() > 1e-10);
        assert!(any_different);
        assert_eq!(dp.num_queries, 1);
    }

    #[test]
    fn test_dp_privatize() {
        let mut dp = DifferentialPrivacy::new(1.0, 1e-5, 0.5, 42);
        let mut grad = vec![10.0, 20.0]; // will be clipped then noised
        dp.privatize(&mut grad);
        let norm: f64 = grad.iter().map(|g| g * g).sum::<f64>().sqrt();
        // After clipping to 0.5 and adding noise, should be around 0.5
        assert!(norm < 10.0); // definitely not the original 22.36
    }

    #[test]
    fn test_dp_budget() {
        let dp = DifferentialPrivacy::new(1.0, 1e-5, 1.0, 42);
        assert!(!dp.budget_exhausted());
        assert!((dp.budget_remaining() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_top_k_sparsification() {
        let grad = vec![0.1, -5.0, 0.2, 3.0, -0.01, 4.0];
        let sparse = GradientCompressor::top_k(&grad, 3);
        assert_eq!(sparse.len(), 3);
        // Should contain the 3 largest magnitude values
        let indices: Vec<usize> = sparse.iter().map(|&(i, _)| i).collect();
        assert!(indices.contains(&1)); // -5.0
        assert!(indices.contains(&5)); // 4.0
        assert!(indices.contains(&3)); // 3.0
    }

    #[test]
    fn test_sparse_reconstruct() {
        let sparse = vec![(1, 5.0), (3, -2.0)];
        let full = GradientCompressor::from_sparse(&sparse, 5);
        assert_eq!(full[0], 0.0);
        assert_eq!(full[1], 5.0);
        assert_eq!(full[3], -2.0);
    }

    #[test]
    fn test_quantization() {
        let grad = vec![0.0, 0.5, 1.0, 0.25, 0.75];
        let q = GradientCompressor::quantize(&grad, 4); // 16 levels
        assert_eq!(q.len(), 5);
        // Quantized values should be close to originals with 16 levels
        for (orig, quant) in grad.iter().zip(&q) {
            assert!((orig - quant).abs() < 0.1);
        }
    }

    #[test]
    fn test_compression_ratio() {
        let ratio = GradientCompressor::compression_ratio(1000, 100);
        assert!((ratio - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_fed_client() {
        let mut client = FedClient::new(0, 100, 10, 42);
        assert_eq!(client.id, 0);
        assert_eq!(client.data_size, 100);
        assert_eq!(client.weights.len(), 10);

        let global = vec![0.0; 10];
        let delta = client.local_train(&global, 3, 0.01, 42);
        assert_eq!(delta.len(), 10);
        assert_eq!(client.rounds_participated, 1);
    }

    #[test]
    fn test_client_selection_random() {
        let clients: Vec<FedClient> = (0..10)
            .map(|i| FedClient::new(i, 100, 5, 42 + i as u64))
            .collect();
        let selected = ClientSelector::select(&clients, 3, &SelectionStrategy::Random, 42);
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn test_client_selection_all() {
        let clients: Vec<FedClient> = (0..5)
            .map(|i| FedClient::new(i, 100, 5, 42 + i as u64))
            .collect();
        let selected = ClientSelector::select(&clients, 10, &SelectionStrategy::All, 42);
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_aggregation_fedavg() {
        let updates = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0],
        ];
        let data_sizes = vec![100, 300]; // 1:3 ratio
        let agg = Aggregator::fedavg(&updates, &data_sizes, 2);
        assert!((agg[0] - 2.5).abs() < 1e-6); // 0.25*1 + 0.75*3
        assert!((agg[1] - 3.5).abs() < 1e-6); // 0.25*2 + 0.75*4
    }

    #[test]
    fn test_aggregation_median() {
        let updates = vec![
            vec![1.0, 100.0],  // Byzantine
            vec![2.0, 3.0],
            vec![3.0, 4.0],
        ];
        let agg = Aggregator::median(&updates, 2);
        assert_eq!(agg[0], 2.0); // median of [1, 2, 3]
        assert_eq!(agg[1], 4.0); // median of [100, 3, 4] = 4
    }

    #[test]
    fn test_secure_aggregation() {
        let mut sa = SecureAggregation::new(42);
        sa.generate_masks(&[0, 1, 2], 5);
        assert_eq!(sa.num_masks(), 3); // C(3,2) = 3 pairs
    }

    #[test]
    fn test_secure_agg_cancellation() {
        let mut sa = SecureAggregation::new(42);
        let ids = vec![0, 1];
        sa.generate_masks(&ids, 3);

        let update0 = vec![1.0, 2.0, 3.0];
        let update1 = vec![4.0, 5.0, 6.0];

        let masked0 = sa.mask_update(0, &update0, &ids);
        let masked1 = sa.mask_update(1, &update1, &ids);

        // Sum of masked updates should equal sum of original updates
        let masked_sum: Vec<f64> = (0..3).map(|i| masked0[i] + masked1[i]).collect();
        let original_sum: Vec<f64> = (0..3).map(|i| update0[i] + update1[i]).collect();
        for i in 0..3 {
            assert!((masked_sum[i] - original_sum[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_fed_server_basic() {
        let mut server = FedServer::new(10, AggregationMethod::FedAvg, 42);
        assert_eq!(server.round, 0);
        assert_eq!(server.global_weights.len(), 10);
    }

    #[test]
    fn test_fed_server_training() {
        let mut server = FedServer::new(10, AggregationMethod::FedAvg, 42);
        server.clients_per_round = 3;
        server.local_epochs = 2;
        for i in 0..5 {
            server.add_client(100 + i * 50);
        }
        server.train(3);
        assert_eq!(server.round, 3);
        assert_eq!(server.loss_history.len(), 3);
    }

    #[test]
    fn test_fed_server_with_dp() {
        let mut server = FedServer::new(10, AggregationMethod::FedAvg, 42);
        server.enable_dp(1.0, 1e-5, 1.0);
        assert!(server.dp.is_some());
        for i in 0..3 {
            server.add_client(100 * (i + 1));
        }
        server.clients_per_round = 2;
        server.run_round();
        assert_eq!(server.round, 1);
    }

    #[test]
    fn test_fed_server_fedprox() {
        let mut server = FedServer::new(10, AggregationMethod::FedProx { mu: 0.01 }, 42);
        for _ in 0..4 {
            server.add_client(200);
        }
        server.clients_per_round = 2;
        server.run_round();
        assert_eq!(server.round, 1);
    }

    #[test]
    fn test_convergence_check() {
        let mut server = FedServer::new(5, AggregationMethod::SimpleAvg, 42);
        server.loss_history = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        assert!(server.has_converged(3, 0.01));
        server.loss_history = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        assert!(!server.has_converged(3, 0.01));
    }

    #[test]
    fn test_ffi_create_and_free() {
        let id = vitalis_fed_create(10, 42);
        assert!(id > 0);
        assert_eq!(vitalis_fed_round(id), 0);
        assert_eq!(vitalis_fed_num_clients(id), 0);
        vitalis_fed_add_client(id, 100);
        assert_eq!(vitalis_fed_num_clients(id), 1);
        assert_eq!(vitalis_fed_free(id), 1);
        assert_eq!(vitalis_fed_free(id), 0);
    }

    #[test]
    fn test_ffi_train_round() {
        let id = vitalis_fed_create(5, 42);
        for _ in 0..3 {
            vitalis_fed_add_client(id, 50);
        }
        let round = vitalis_fed_train_round(id);
        assert_eq!(round, 1);
        vitalis_fed_free(id);
    }

    #[test]
    fn test_selection_strategy_eq() {
        assert_eq!(SelectionStrategy::Random, SelectionStrategy::Random);
        assert_ne!(SelectionStrategy::Random, SelectionStrategy::All);
    }

    #[test]
    fn test_aggregation_method_eq() {
        assert_eq!(AggregationMethod::FedAvg, AggregationMethod::FedAvg);
        assert_ne!(AggregationMethod::FedAvg, AggregationMethod::SimpleAvg);
    }
}
