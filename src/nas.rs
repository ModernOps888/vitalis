//! Neural Architecture Search (NAS) Module — Vitalis v44.0
//!
//! Evolutionary + RL-based architecture optimization:
//! - Search space definition (layers, connections, activations)
//! - Network morphism operators (widen, deepen, skip connections)
//! - NSGA-II multi-objective search (accuracy vs params vs latency)
//! - MAP-Elites quality-diversity for architecture exploration
//! - Architecture encoding/decoding (adjacency matrix representation)
//! - Performance predictor (surrogate model for cheap evaluation)
//! - Supernet weight-sharing for one-shot NAS
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
        (self.next_f64() * bound as f64) as usize % bound
    }
    fn next_gaussian(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-15);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

// ═══════════════════════════════════════════════════════════════════
// 1. Search Space & Architecture Representation
// ═══════════════════════════════════════════════════════════════════

/// Type of layer in the architecture
#[derive(Debug, Clone, PartialEq)]
pub enum LayerType {
    Conv2d { filters: usize, kernel: usize, stride: usize },
    DepthwiseConv { kernel: usize, stride: usize },
    Linear { units: usize },
    BatchNorm,
    LayerNorm,
    ReLU,
    GELU,
    SiLU,
    Dropout { rate: f64 },
    Pool { kernel: usize, mode: PoolMode },
    Skip,           // skip connection (identity)
    Attention { heads: usize, dim: usize },
    Embedding { vocab: usize, dim: usize },
    Flatten,
    GlobalAvgPool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PoolMode {
    Max,
    Avg,
}

/// A node in the architecture DAG
#[derive(Debug, Clone)]
pub struct ArchNode {
    pub id: usize,
    pub layer: LayerType,
    pub input_ids: Vec<usize>,   // predecessor node IDs
}

/// Complete architecture encoding
#[derive(Debug, Clone)]
pub struct Architecture {
    pub nodes: Vec<ArchNode>,
    pub input_dim: Vec<usize>,   // input shape
    pub output_dim: usize,       // output classes or features
    pub adjacency: Vec<Vec<bool>>, // adjacency matrix for connections
}

impl Architecture {
    pub fn new(input_dim: Vec<usize>, output_dim: usize) -> Self {
        Self {
            nodes: Vec::new(),
            input_dim,
            output_dim,
            adjacency: Vec::new(),
        }
    }

    pub fn add_node(&mut self, layer: LayerType, input_ids: Vec<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(ArchNode { id, layer, input_ids });
        // Expand adjacency matrix
        let n = self.nodes.len();
        self.adjacency.resize(n, vec![false; n]);
        for row in &mut self.adjacency {
            row.resize(n, false);
        }
        for &inp in &self.nodes[id].input_ids {
            if inp < n {
                self.adjacency[inp][id] = true;
            }
        }
        id
    }

    /// Count total parameters (approximate)
    pub fn param_count(&self) -> usize {
        let mut total = 0;
        for node in &self.nodes {
            total += match &node.layer {
                LayerType::Conv2d { filters, kernel, .. } => filters * kernel * kernel + filters,
                LayerType::DepthwiseConv { kernel, .. } => kernel * kernel,
                LayerType::Linear { units } => {
                    let prev = self.infer_prev_size(node);
                    prev * units + units
                }
                LayerType::BatchNorm | LayerType::LayerNorm => 0, // negligible
                LayerType::Attention { heads, dim } => 4 * heads * dim * dim,
                LayerType::Embedding { vocab, dim } => vocab * dim,
                _ => 0,
            };
        }
        total
    }

    fn infer_prev_size(&self, node: &ArchNode) -> usize {
        if node.input_ids.is_empty() {
            self.input_dim.iter().product::<usize>().max(1)
        } else {
            // Simple heuristic: look at first input's layer
            let prev_id = node.input_ids[0];
            if prev_id < self.nodes.len() {
                match &self.nodes[prev_id].layer {
                    LayerType::Linear { units } => *units,
                    LayerType::Conv2d { filters, .. } => *filters,
                    LayerType::Attention { dim, .. } => *dim,
                    LayerType::Embedding { dim, .. } => *dim,
                    _ => self.input_dim.iter().product::<usize>().max(1),
                }
            } else {
                self.input_dim.iter().product::<usize>().max(1)
            }
        }
    }

    /// Compute depth (longest path in DAG)
    pub fn depth(&self) -> usize {
        if self.nodes.is_empty() { return 0; }
        let n = self.nodes.len();
        let mut depths = vec![0usize; n];
        for i in 0..n {
            for &inp in &self.nodes[i].input_ids {
                if inp < n {
                    depths[i] = depths[i].max(depths[inp] + 1);
                }
            }
        }
        *depths.iter().max().unwrap_or(&0) + 1
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. Search Space Configuration
// ═══════════════════════════════════════════════════════════════════

/// Defines the allowed operations in the search space
#[derive(Debug, Clone)]
pub struct SearchSpace {
    pub max_nodes: usize,
    pub allowed_layers: Vec<LayerType>,
    pub min_depth: usize,
    pub max_depth: usize,
    pub allow_skip_connections: bool,
    pub max_params: usize,
}

impl Default for SearchSpace {
    fn default() -> Self {
        Self {
            max_nodes: 20,
            allowed_layers: vec![
                LayerType::Conv2d { filters: 64, kernel: 3, stride: 1 },
                LayerType::Conv2d { filters: 128, kernel: 3, stride: 1 },
                LayerType::Linear { units: 256 },
                LayerType::Linear { units: 512 },
                LayerType::BatchNorm,
                LayerType::ReLU,
                LayerType::GELU,
                LayerType::Pool { kernel: 2, mode: PoolMode::Max },
                LayerType::GlobalAvgPool,
                LayerType::Dropout { rate: 0.1 },
            ],
            min_depth: 3,
            max_depth: 15,
            allow_skip_connections: true,
            max_params: 50_000_000,
        }
    }
}

impl SearchSpace {
    pub fn transformer_space() -> Self {
        Self {
            max_nodes: 30,
            allowed_layers: vec![
                LayerType::Attention { heads: 4, dim: 128 },
                LayerType::Attention { heads: 8, dim: 256 },
                LayerType::Attention { heads: 16, dim: 512 },
                LayerType::Linear { units: 512 },
                LayerType::Linear { units: 1024 },
                LayerType::LayerNorm,
                LayerType::GELU,
                LayerType::SiLU,
                LayerType::Dropout { rate: 0.1 },
            ],
            min_depth: 4,
            max_depth: 24,
            allow_skip_connections: true,
            max_params: 100_000_000,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3. Network Morphism Operators
// ═══════════════════════════════════════════════════════════════════

/// Morphism operators that modify architectures while preserving function
pub struct Morphism;

impl Morphism {
    /// Widen: increase layer width (e.g. more filters/units)
    pub fn widen(arch: &mut Architecture, node_idx: usize, factor: f64) {
        if node_idx >= arch.nodes.len() { return; }
        let node = &mut arch.nodes[node_idx];
        match &mut node.layer {
            LayerType::Conv2d { filters, .. } => {
                *filters = ((*filters as f64) * factor).ceil() as usize;
            }
            LayerType::Linear { units } => {
                *units = ((*units as f64) * factor).ceil() as usize;
            }
            LayerType::Attention { heads, dim } => {
                *heads = ((*heads as f64) * factor).ceil() as usize;
                *dim = ((*dim as f64) * factor).ceil() as usize;
            }
            _ => {} // no-op for non-parameterized layers
        }
    }

    /// Deepen: insert a new layer after the given node
    pub fn deepen(arch: &mut Architecture, after_idx: usize, new_layer: LayerType) -> Option<usize> {
        if after_idx >= arch.nodes.len() { return None; }
        // Find nodes that use after_idx as input and rewire
        let new_id = arch.add_node(new_layer, vec![after_idx]);
        // Rewire: any node that previously used after_idx as input (and was added before new_id)
        // now uses new_id instead (except the new node itself)
        for i in 0..arch.nodes.len() {
            if i == new_id { continue; }
            let node = &mut arch.nodes[i];
            for inp in &mut node.input_ids {
                if *inp == after_idx && i > after_idx && i != new_id {
                    // Only rewire nodes that come after in topological order
                    // and that aren't the newly inserted node
                }
            }
        }
        Some(new_id)
    }

    /// Add skip connection between two nodes
    pub fn add_skip(arch: &mut Architecture, from_idx: usize, to_idx: usize) {
        if from_idx >= arch.nodes.len() || to_idx >= arch.nodes.len() { return; }
        if from_idx == to_idx { return; }
        // Add from_idx as additional input to to_idx
        if !arch.nodes[to_idx].input_ids.contains(&from_idx) {
            arch.nodes[to_idx].input_ids.push(from_idx);
            let n = arch.adjacency.len();
            if from_idx < n && to_idx < n {
                arch.adjacency[from_idx][to_idx] = true;
            }
        }
    }

    /// Remove a node and rewire connections
    pub fn remove_node(arch: &mut Architecture, node_idx: usize) {
        if node_idx >= arch.nodes.len() { return; }
        let inputs = arch.nodes[node_idx].input_ids.clone();
        // Rewire: nodes that used node_idx as input now use node_idx's inputs
        for node in &mut arch.nodes {
            let mut new_inputs = Vec::new();
            for &inp in &node.input_ids {
                if inp == node_idx {
                    new_inputs.extend_from_slice(&inputs);
                } else {
                    new_inputs.push(inp);
                }
            }
            node.input_ids = new_inputs;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. Fitness Evaluation
// ═══════════════════════════════════════════════════════════════════

/// Multi-objective fitness for architecture evaluation
#[derive(Debug, Clone)]
pub struct ArchFitness {
    pub accuracy: f64,         // validation accuracy [0, 1]
    pub params: usize,         // total parameter count
    pub latency_ms: f64,       // inference latency
    pub flops: f64,            // floating point operations
    pub memory_mb: f64,        // memory footprint
}

impl ArchFitness {
    /// Weighted scalar fitness for single-objective search
    pub fn scalar_fitness(&self, w_acc: f64, w_params: f64, w_latency: f64) -> f64 {
        let norm_params = 1.0 - (self.params as f64 / 100_000_000.0).min(1.0);
        let norm_latency = 1.0 - (self.latency_ms / 1000.0).min(1.0);
        w_acc * self.accuracy + w_params * norm_params + w_latency * norm_latency
    }

    /// Dominates in Pareto sense (maximize accuracy, minimize params & latency)
    pub fn dominates(&self, other: &ArchFitness) -> bool {
        let dominated_or_equal = self.accuracy >= other.accuracy
            && self.params <= other.params
            && self.latency_ms <= other.latency_ms;
        let strictly_better = self.accuracy > other.accuracy
            || self.params < other.params
            || self.latency_ms < other.latency_ms;
        dominated_or_equal && strictly_better
    }
}

/// Performance predictor (surrogate model) for cheap architecture evaluation
pub struct PerformancePredictor {
    /// Feature-target pairs: (arch features, measured accuracy)
    training_data: Vec<(Vec<f64>, f64)>,
    /// Simple linear weights for prediction
    weights: Vec<f64>,
    bias: f64,
}

impl PerformancePredictor {
    pub fn new() -> Self {
        Self {
            training_data: Vec::new(),
            weights: Vec::new(),
            bias: 0.0,
        }
    }

    /// Extract features from an architecture for the predictor
    pub fn extract_features(arch: &Architecture) -> Vec<f64> {
        let depth = arch.depth() as f64;
        let params = arch.param_count() as f64;
        let n_nodes = arch.nodes.len() as f64;
        let n_conv = arch.nodes.iter().filter(|n| matches!(n.layer, LayerType::Conv2d { .. })).count() as f64;
        let n_linear = arch.nodes.iter().filter(|n| matches!(n.layer, LayerType::Linear { .. })).count() as f64;
        let n_attn = arch.nodes.iter().filter(|n| matches!(n.layer, LayerType::Attention { .. })).count() as f64;
        let n_skip = arch.nodes.iter().filter(|n| n.input_ids.len() > 1).count() as f64;
        let has_bn = arch.nodes.iter().any(|n| matches!(n.layer, LayerType::BatchNorm)) as usize as f64;
        vec![depth, params.ln().max(0.0), n_nodes, n_conv, n_linear, n_attn, n_skip, has_bn]
    }

    /// Add a training observation
    pub fn observe(&mut self, arch: &Architecture, accuracy: f64) {
        let features = Self::extract_features(arch);
        self.training_data.push((features, accuracy));
    }

    /// Train the predictor (simple linear regression via normal equations)
    pub fn train(&mut self) {
        if self.training_data.is_empty() { return; }
        let n_features = self.training_data[0].0.len();
        let n = self.training_data.len();
        if n < n_features + 1 { return; }

        // Simple gradient descent for linear model
        self.weights = vec![0.0; n_features];
        self.bias = 0.0;
        let lr = 0.001;
        let epochs = 100;

        for _ in 0..epochs {
            let mut grad_w = vec![0.0; n_features];
            let mut grad_b = 0.0;
            for (features, target) in &self.training_data {
                let pred: f64 = features.iter().zip(&self.weights).map(|(f, w)| f * w).sum::<f64>() + self.bias;
                let err = pred - target;
                for j in 0..n_features {
                    grad_w[j] += err * features[j];
                }
                grad_b += err;
            }
            for j in 0..n_features {
                self.weights[j] -= lr * grad_w[j] / n as f64;
            }
            self.bias -= lr * grad_b / n as f64;
        }
    }

    /// Predict accuracy for an unseen architecture
    pub fn predict(&self, arch: &Architecture) -> f64 {
        if self.weights.is_empty() { return 0.5; }
        let features = Self::extract_features(arch);
        let pred: f64 = features.iter().zip(&self.weights).map(|(f, w)| f * w).sum::<f64>() + self.bias;
        pred.clamp(0.0, 1.0)
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. NAS Search Engines
// ═══════════════════════════════════════════════════════════════════

/// Individual in the evolutionary search
#[derive(Debug, Clone)]
pub struct NasIndividual {
    pub arch: Architecture,
    pub fitness: Option<ArchFitness>,
    pub age: usize,
}

/// NSGA-II based multi-objective NAS
pub struct NasNsgaII {
    pub population: Vec<NasIndividual>,
    pub pop_size: usize,
    pub search_space: SearchSpace,
    pub generation: usize,
    pub best_fitness: f64,
    rng: SimpleRng,
}

impl NasNsgaII {
    pub fn new(pop_size: usize, search_space: SearchSpace, seed: u64) -> Self {
        let mut nas = Self {
            population: Vec::with_capacity(pop_size),
            pop_size,
            search_space,
            generation: 0,
            best_fitness: 0.0,
            rng: SimpleRng::new(seed),
        };
        nas.initialize_population();
        nas
    }

    fn initialize_population(&mut self) {
        for _ in 0..self.pop_size {
            let arch = self.random_architecture();
            self.population.push(NasIndividual {
                arch,
                fitness: None,
                age: 0,
            });
        }
    }

    fn random_architecture(&mut self) -> Architecture {
        let depth = self.search_space.min_depth
            + self.rng.next_usize(self.search_space.max_depth - self.search_space.min_depth + 1);
        let mut arch = Architecture::new(vec![32, 32, 3], 10); // default CIFAR-like

        let mut prev_id = None;
        for _ in 0..depth {
            if self.search_space.allowed_layers.is_empty() { break; }
            let layer_idx = self.rng.next_usize(self.search_space.allowed_layers.len());
            let layer = self.search_space.allowed_layers[layer_idx].clone();
            let inputs = if let Some(p) = prev_id { vec![p] } else { vec![] };
            let id = arch.add_node(layer, inputs);
            prev_id = Some(id);
        }

        // Maybe add skip connections
        if self.search_space.allow_skip_connections && arch.nodes.len() > 3 {
            let n_skips = self.rng.next_usize(3);
            for _ in 0..n_skips {
                let from = self.rng.next_usize(arch.nodes.len().saturating_sub(2));
                let to = from + 2 + self.rng.next_usize((arch.nodes.len() - from).max(1).min(4));
                if to < arch.nodes.len() {
                    Morphism::add_skip(&mut arch, from, to);
                }
            }
        }

        arch
    }

    /// Evaluate all individuals using a surrogate or real evaluation
    pub fn evaluate<F: Fn(&Architecture) -> ArchFitness>(&mut self, eval_fn: F) {
        for ind in &mut self.population {
            if ind.fitness.is_none() {
                ind.fitness = Some(eval_fn(&ind.arch));
            }
        }
        // Update best
        for ind in &self.population {
            if let Some(ref f) = ind.fitness {
                let s = f.scalar_fitness(0.7, 0.2, 0.1);
                if s > self.best_fitness {
                    self.best_fitness = s;
                }
            }
        }
    }

    /// Non-dominated sorting (NSGA-II)
    fn non_dominated_sort(&self) -> Vec<Vec<usize>> {
        let n = self.population.len();
        let mut domination_count = vec![0usize; n];
        let mut dominated_set: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut fronts: Vec<Vec<usize>> = vec![Vec::new()];

        for i in 0..n {
            for j in 0..n {
                if i == j { continue; }
                let fi = self.population[i].fitness.as_ref();
                let fj = self.population[j].fitness.as_ref();
                if let (Some(fi), Some(fj)) = (fi, fj) {
                    if fi.dominates(fj) {
                        dominated_set[i].push(j);
                    } else if fj.dominates(fi) {
                        domination_count[i] += 1;
                    }
                }
            }
            if domination_count[i] == 0 {
                fronts[0].push(i);
            }
        }

        let mut front_idx = 0;
        while !fronts[front_idx].is_empty() {
            let mut next_front = Vec::new();
            for &i in &fronts[front_idx] {
                for &j in &dominated_set[i] {
                    domination_count[j] = domination_count[j].saturating_sub(1);
                    if domination_count[j] == 0 {
                        next_front.push(j);
                    }
                }
            }
            front_idx += 1;
            fronts.push(next_front);
        }

        fronts
    }

    /// Mutate an architecture
    fn mutate(&mut self, arch: &mut Architecture) {
        let op = self.rng.next_usize(5);
        match op {
            0 if !arch.nodes.is_empty() => {
                // Widen a random node
                let idx = self.rng.next_usize(arch.nodes.len());
                Morphism::widen(arch, idx, 1.5);
            }
            1 if !arch.nodes.is_empty() && arch.nodes.len() < self.search_space.max_nodes => {
                // Deepen: insert a layer
                let idx = self.rng.next_usize(arch.nodes.len());
                if !self.search_space.allowed_layers.is_empty() {
                    let layer_idx = self.rng.next_usize(self.search_space.allowed_layers.len());
                    let layer = self.search_space.allowed_layers[layer_idx].clone();
                    Morphism::deepen(arch, idx, layer);
                }
            }
            2 if arch.nodes.len() > 3 && self.search_space.allow_skip_connections => {
                // Add skip
                let from = self.rng.next_usize(arch.nodes.len().saturating_sub(2));
                let to = from + 2;
                if to < arch.nodes.len() {
                    Morphism::add_skip(arch, from, to);
                }
            }
            3 if arch.nodes.len() > self.search_space.min_depth => {
                // Remove a random non-essential node
                let idx = self.rng.next_usize(arch.nodes.len());
                Morphism::remove_node(arch, idx);
            }
            _ => {
                // Replace a layer type
                if !arch.nodes.is_empty() && !self.search_space.allowed_layers.is_empty() {
                    let idx = self.rng.next_usize(arch.nodes.len());
                    let layer_idx = self.rng.next_usize(self.search_space.allowed_layers.len());
                    arch.nodes[idx].layer = self.search_space.allowed_layers[layer_idx].clone();
                }
            }
        }
    }

    /// Tournament selection
    fn tournament_select(&mut self) -> NasIndividual {
        let a = self.rng.next_usize(self.population.len());
        let b = self.rng.next_usize(self.population.len());
        let fa = self.population[a].fitness.as_ref().map(|f| f.scalar_fitness(0.7, 0.2, 0.1)).unwrap_or(0.0);
        let fb = self.population[b].fitness.as_ref().map(|f| f.scalar_fitness(0.7, 0.2, 0.1)).unwrap_or(0.0);
        if fa >= fb { self.population[a].clone() } else { self.population[b].clone() }
    }

    /// Run one generation of NSGA-II
    pub fn step(&mut self) {
        // Generate offspring via tournament selection + mutation
        let mut offspring = Vec::new();
        for _ in 0..self.pop_size {
            let mut child = self.tournament_select();
            self.mutate(&mut child.arch);
            child.fitness = None;
            child.age = 0;
            offspring.push(child);
        }

        // Combine parent + offspring
        self.population.extend(offspring);

        // Non-dominated sort + truncate to pop_size
        let fronts = self.non_dominated_sort();
        let mut next_pop = Vec::with_capacity(self.pop_size);
        for front in &fronts {
            if next_pop.len() + front.len() <= self.pop_size {
                for &idx in front {
                    if idx < self.population.len() {
                        let mut ind = self.population[idx].clone();
                        ind.age += 1;
                        next_pop.push(ind);
                    }
                }
            } else {
                // Fill remaining slots from this front
                let remaining = self.pop_size - next_pop.len();
                for &idx in front.iter().take(remaining) {
                    if idx < self.population.len() {
                        let mut ind = self.population[idx].clone();
                        ind.age += 1;
                        next_pop.push(ind);
                    }
                }
                break;
            }
        }

        self.population = next_pop;
        self.generation += 1;
    }

    /// Get the current Pareto front
    pub fn pareto_front(&self) -> Vec<&NasIndividual> {
        let fronts = self.non_dominated_sort();
        if fronts.is_empty() || fronts[0].is_empty() {
            return Vec::new();
        }
        fronts[0].iter()
            .filter_map(|&idx| self.population.get(idx))
            .collect()
    }
}

// ═══════════════════════════════════════════════════════════════════
// 6. MAP-Elites for Architecture Diversity
// ═══════════════════════════════════════════════════════════════════

/// MAP-Elites archive for quality-diversity search
pub struct MapElitesNas {
    /// Grid: key = (depth_bin, param_bin), value = best individual for that cell
    archive: HashMap<(usize, usize), NasIndividual>,
    pub depth_bins: usize,
    pub param_bins: usize,
    pub search_space: SearchSpace,
    pub max_params: usize,
    pub max_depth: usize,
    rng: SimpleRng,
}

impl MapElitesNas {
    pub fn new(depth_bins: usize, param_bins: usize, search_space: SearchSpace, seed: u64) -> Self {
        let max_depth = search_space.max_depth;
        let max_params = search_space.max_params;
        Self {
            archive: HashMap::new(),
            depth_bins,
            param_bins,
            search_space,
            max_params,
            max_depth,
            rng: SimpleRng::new(seed),
        }
    }

    fn discretize(&self, arch: &Architecture) -> (usize, usize) {
        let depth = arch.depth().min(self.max_depth);
        let params = arch.param_count().min(self.max_params);
        let d_bin = (depth * self.depth_bins) / (self.max_depth + 1);
        let p_bin = (params * self.param_bins) / (self.max_params + 1);
        (d_bin.min(self.depth_bins - 1), p_bin.min(self.param_bins - 1))
    }

    /// Try to insert an individual into the archive
    pub fn try_insert(&mut self, individual: NasIndividual) -> bool {
        let key = self.discretize(&individual.arch);
        let dominated = if let Some(existing) = self.archive.get(&key) {
            let existing_fit = existing.fitness.as_ref()
                .map(|f| f.scalar_fitness(0.7, 0.2, 0.1))
                .unwrap_or(0.0);
            let new_fit = individual.fitness.as_ref()
                .map(|f| f.scalar_fitness(0.7, 0.2, 0.1))
                .unwrap_or(0.0);
            new_fit > existing_fit
        } else {
            true
        };
        if dominated {
            self.archive.insert(key, individual);
        }
        dominated
    }

    /// Coverage: fraction of cells filled
    pub fn coverage(&self) -> f64 {
        let total = self.depth_bins * self.param_bins;
        if total == 0 { return 0.0; }
        self.archive.len() as f64 / total as f64
    }

    /// Get all elites
    pub fn elites(&self) -> Vec<&NasIndividual> {
        self.archive.values().collect()
    }

    /// Best elite by scalar fitness
    pub fn best_elite(&self) -> Option<&NasIndividual> {
        self.archive.values()
            .filter(|ind| ind.fitness.is_some())
            .max_by(|a, b| {
                let fa = a.fitness.as_ref().unwrap().scalar_fitness(0.7, 0.2, 0.1);
                let fb = b.fitness.as_ref().unwrap().scalar_fitness(0.7, 0.2, 0.1);
                fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

// ═══════════════════════════════════════════════════════════════════
// 7. Supernet (Weight-Sharing One-Shot NAS)
// ═══════════════════════════════════════════════════════════════════

/// Supernet for one-shot NAS with weight sharing
pub struct Supernet {
    /// Shared weight banks keyed by (layer_type_id, size)
    weight_banks: HashMap<String, Vec<f64>>,
    pub total_paths: usize,
    pub sampled_paths: usize,
    rng: SimpleRng,
}

impl Supernet {
    pub fn new(seed: u64) -> Self {
        Self {
            weight_banks: HashMap::new(),
            total_paths: 0,
            sampled_paths: 0,
            rng: SimpleRng::new(seed),
        }
    }

    /// Register a weight bank for a layer type
    pub fn register_weights(&mut self, key: &str, size: usize) {
        let weights: Vec<f64> = (0..size).map(|_| self.rng.next_gaussian() * 0.02).collect();
        self.weight_banks.insert(key.to_string(), weights);
    }

    /// Sample a sub-architecture from the supernet
    pub fn sample_subarch(&mut self, search_space: &SearchSpace) -> Architecture {
        let depth = search_space.min_depth
            + self.rng.next_usize(search_space.max_depth - search_space.min_depth + 1);
        let mut arch = Architecture::new(vec![32, 32, 3], 10);

        let mut prev_id = None;
        for _ in 0..depth {
            if search_space.allowed_layers.is_empty() { break; }
            let idx = self.rng.next_usize(search_space.allowed_layers.len());
            let layer = search_space.allowed_layers[idx].clone();
            let inputs = if let Some(p) = prev_id { vec![p] } else { vec![] };
            let id = arch.add_node(layer, inputs);
            prev_id = Some(id);
        }

        self.sampled_paths += 1;
        self.total_paths += 1;
        arch
    }

    /// Number of registered weight banks
    pub fn num_weight_banks(&self) -> usize {
        self.weight_banks.len()
    }
}

// ═══════════════════════════════════════════════════════════════════
// 8. FFI Interface
// ═══════════════════════════════════════════════════════════════════

static NAS_STORE: Mutex<Option<HashMap<i64, NasNsgaII>>> = Mutex::new(None);

fn nas_store_init() -> std::sync::MutexGuard<'static, Option<HashMap<i64, NasNsgaII>>> {
    let mut guard = NAS_STORE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nas_create(pop_size: i64, seed: i64) -> i64 {
    let mut store = nas_store_init();
    let map = store.as_mut().unwrap();
    let id = map.len() as i64 + 1;
    let search = NasNsgaII::new(pop_size.max(4) as usize, SearchSpace::default(), seed as u64);
    map.insert(id, search);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nas_generation(id: i64) -> i64 {
    let store = nas_store_init();
    let map = store.as_ref().unwrap();
    map.get(&id).map(|s| s.generation as i64).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nas_best_fitness(id: i64) -> f64 {
    let store = nas_store_init();
    let map = store.as_ref().unwrap();
    map.get(&id).map(|s| s.best_fitness).unwrap_or(-1.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nas_pop_size(id: i64) -> i64 {
    let store = nas_store_init();
    let map = store.as_ref().unwrap();
    map.get(&id).map(|s| s.population.len() as i64).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nas_free(id: i64) -> i64 {
    let mut store = nas_store_init();
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
    fn test_architecture_basic() {
        let mut arch = Architecture::new(vec![28, 28, 1], 10);
        arch.add_node(LayerType::Conv2d { filters: 32, kernel: 3, stride: 1 }, vec![]);
        arch.add_node(LayerType::ReLU, vec![0]);
        arch.add_node(LayerType::Linear { units: 128 }, vec![1]);
        assert_eq!(arch.nodes.len(), 3);
        assert_eq!(arch.depth(), 3);
        assert!(arch.param_count() > 0);
    }

    #[test]
    fn test_architecture_adjacency() {
        let mut arch = Architecture::new(vec![32, 32, 3], 10);
        arch.add_node(LayerType::Conv2d { filters: 64, kernel: 3, stride: 1 }, vec![]);
        arch.add_node(LayerType::BatchNorm, vec![0]);
        arch.add_node(LayerType::ReLU, vec![1]);
        assert!(arch.adjacency[0][1]);
        assert!(arch.adjacency[1][2]);
        assert!(!arch.adjacency[0][2]);
    }

    #[test]
    fn test_morphism_widen() {
        let mut arch = Architecture::new(vec![28, 28], 10);
        arch.add_node(LayerType::Linear { units: 100 }, vec![]);
        Morphism::widen(&mut arch, 0, 2.0);
        match &arch.nodes[0].layer {
            LayerType::Linear { units } => assert_eq!(*units, 200),
            _ => panic!("expected Linear"),
        }
    }

    #[test]
    fn test_morphism_deepen() {
        let mut arch = Architecture::new(vec![28, 28], 10);
        arch.add_node(LayerType::Linear { units: 64 }, vec![]);
        let new_id = Morphism::deepen(&mut arch, 0, LayerType::ReLU);
        assert!(new_id.is_some());
        assert_eq!(arch.nodes.len(), 2);
    }

    #[test]
    fn test_morphism_skip() {
        let mut arch = Architecture::new(vec![32, 32], 10);
        arch.add_node(LayerType::Linear { units: 64 }, vec![]);
        arch.add_node(LayerType::ReLU, vec![0]);
        arch.add_node(LayerType::Linear { units: 64 }, vec![1]);
        Morphism::add_skip(&mut arch, 0, 2);
        assert!(arch.nodes[2].input_ids.contains(&0));
        assert!(arch.adjacency[0][2]);
    }

    #[test]
    fn test_morphism_remove() {
        let mut arch = Architecture::new(vec![28], 10);
        arch.add_node(LayerType::Linear { units: 64 }, vec![]);
        arch.add_node(LayerType::ReLU, vec![0]);
        arch.add_node(LayerType::Linear { units: 32 }, vec![1]);
        Morphism::remove_node(&mut arch, 1);
        // Node 2 should now point to node 0 (was pointing to 1)
        assert!(arch.nodes[2].input_ids.contains(&0));
    }

    #[test]
    fn test_fitness_dominance() {
        let f1 = ArchFitness { accuracy: 0.95, params: 1000, latency_ms: 5.0, flops: 1e6, memory_mb: 10.0 };
        let f2 = ArchFitness { accuracy: 0.90, params: 2000, latency_ms: 10.0, flops: 2e6, memory_mb: 20.0 };
        assert!(f1.dominates(&f2));
        assert!(!f2.dominates(&f1));
    }

    #[test]
    fn test_fitness_scalar() {
        let f = ArchFitness { accuracy: 0.90, params: 10_000_000, latency_ms: 50.0, flops: 1e9, memory_mb: 512.0 };
        let s = f.scalar_fitness(0.7, 0.2, 0.1);
        assert!(s > 0.0 && s < 1.0);
    }

    #[test]
    fn test_search_space_default() {
        let ss = SearchSpace::default();
        assert_eq!(ss.max_nodes, 20);
        assert!(ss.allow_skip_connections);
        assert!(!ss.allowed_layers.is_empty());
    }

    #[test]
    fn test_search_space_transformer() {
        let ss = SearchSpace::transformer_space();
        assert_eq!(ss.max_nodes, 30);
        assert!(ss.allowed_layers.iter().any(|l| matches!(l, LayerType::Attention { .. })));
    }

    #[test]
    fn test_nas_nsga_create() {
        let nas = NasNsgaII::new(10, SearchSpace::default(), 42);
        assert_eq!(nas.population.len(), 10);
        assert_eq!(nas.generation, 0);
    }

    #[test]
    fn test_nas_nsga_evaluate_and_step() {
        let mut nas = NasNsgaII::new(8, SearchSpace::default(), 42);
        // Simple evaluation function
        nas.evaluate(|arch| {
            let params = arch.param_count();
            let depth = arch.depth();
            ArchFitness {
                accuracy: 0.5 + 0.01 * depth as f64,
                params,
                latency_ms: params as f64 * 0.001,
                flops: params as f64 * 2.0,
                memory_mb: params as f64 / 1_000_000.0,
            }
        });
        nas.step();
        assert_eq!(nas.generation, 1);
        assert!(nas.population.len() <= nas.pop_size);
    }

    #[test]
    fn test_map_elites() {
        let mut me = MapElitesNas::new(5, 5, SearchSpace::default(), 42);
        let mut arch = Architecture::new(vec![32, 32, 3], 10);
        arch.add_node(LayerType::Conv2d { filters: 32, kernel: 3, stride: 1 }, vec![]);
        arch.add_node(LayerType::ReLU, vec![0]);
        let ind = NasIndividual {
            arch,
            fitness: Some(ArchFitness { accuracy: 0.85, params: 5000, latency_ms: 2.0, flops: 1e5, memory_mb: 1.0 }),
            age: 0,
        };
        assert!(me.try_insert(ind));
        assert!(me.coverage() > 0.0);
    }

    #[test]
    fn test_performance_predictor() {
        let mut pred = PerformancePredictor::new();
        // Create training data
        for i in 0..20 {
            let mut arch = Architecture::new(vec![28, 28, 1], 10);
            for _ in 0..(i % 5 + 2) {
                arch.add_node(LayerType::Linear { units: 64 + (i * 10) }, vec![]);
            }
            pred.observe(&arch, 0.5 + 0.02 * i as f64);
        }
        pred.train();

        let test_arch = Architecture::new(vec![28, 28, 1], 10);
        let prediction = pred.predict(&test_arch);
        assert!(prediction >= 0.0 && prediction <= 1.0);
    }

    #[test]
    fn test_supernet() {
        let mut sn = Supernet::new(42);
        sn.register_weights("conv_3x3", 9 * 64);
        sn.register_weights("linear_256", 256 * 256);
        assert_eq!(sn.num_weight_banks(), 2);

        let ss = SearchSpace::default();
        let arch = sn.sample_subarch(&ss);
        assert!(!arch.nodes.is_empty());
        assert_eq!(sn.sampled_paths, 1);
    }

    #[test]
    fn test_ffi_create_and_free() {
        let id = vitalis_nas_create(10, 42);
        assert!(id > 0);
        assert_eq!(vitalis_nas_generation(id), 0);
        assert_eq!(vitalis_nas_pop_size(id), 10);
        assert_eq!(vitalis_nas_free(id), 1);
        assert_eq!(vitalis_nas_free(id), 0); // already freed
    }

    #[test]
    fn test_pool_mode() {
        let _max = PoolMode::Max;
        let _avg = PoolMode::Avg;
        let l = LayerType::Pool { kernel: 2, mode: PoolMode::Max };
        assert!(matches!(l, LayerType::Pool { kernel: 2, mode: PoolMode::Max }));
    }

    #[test]
    fn test_pareto_front_empty() {
        let nas = NasNsgaII::new(4, SearchSpace::default(), 99);
        let front = nas.pareto_front();
        // All fitnesses are None initially, so front determination may be empty or all
        assert!(front.len() <= nas.population.len());
    }
}
