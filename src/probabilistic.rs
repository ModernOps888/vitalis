//! Probabilistic Programming — distributions, inference engines, Bayesian NNs.
//!
//! Provides first-class probabilistic programming primitives:
//! distribution types, sample/observe/condition, MCMC (MH + HMC),
//! variational inference (ELBO + ADVI), Bayesian neural networks,
//! and Gaussian Process regression.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Distribution Types ──────────────────────────────────────────────────

/// Probability distribution variants with type-safe parameters.
#[derive(Debug, Clone)]
pub enum Distribution {
    Normal { mean: f64, std: f64 },
    Bernoulli { p: f64 },
    Categorical { probs: Vec<f64> },
    Uniform { low: f64, high: f64 },
    Beta { alpha: f64, beta: f64 },
    Gamma { shape: f64, rate: f64 },
    Poisson { lambda: f64 },
    Dirichlet { alpha: Vec<f64> },
    Exponential { rate: f64 },
    StudentT { df: f64, loc: f64, scale: f64 },
}

impl Distribution {
    /// Compute log probability density/mass at x.
    pub fn log_prob(&self, x: f64) -> f64 {
        match self {
            Distribution::Normal { mean, std } => {
                let z = (x - mean) / std;
                -0.5 * z * z - std.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln()
            }
            Distribution::Bernoulli { p } => {
                if x == 1.0 { p.ln() }
                else if x == 0.0 { (1.0 - p).ln() }
                else { f64::NEG_INFINITY }
            }
            Distribution::Categorical { probs } => {
                let idx = x as usize;
                if idx < probs.len() { probs[idx].ln() } else { f64::NEG_INFINITY }
            }
            Distribution::Uniform { low, high } => {
                if x >= *low && x <= *high { -(high - low).ln() } else { f64::NEG_INFINITY }
            }
            Distribution::Beta { alpha, beta } => {
                if x <= 0.0 || x >= 1.0 { return f64::NEG_INFINITY; }
                (alpha - 1.0) * x.ln() + (beta - 1.0) * (1.0 - x).ln()
                    - ln_beta(*alpha, *beta)
            }
            Distribution::Gamma { shape, rate } => {
                if x <= 0.0 { return f64::NEG_INFINITY; }
                shape * rate.ln() + (shape - 1.0) * x.ln() - rate * x - ln_gamma(*shape)
            }
            Distribution::Poisson { lambda } => {
                let k = x as u64;
                k as f64 * lambda.ln() - *lambda - ln_factorial(k)
            }
            Distribution::Exponential { rate } => {
                if x < 0.0 { f64::NEG_INFINITY } else { rate.ln() - rate * x }
            }
            Distribution::StudentT { df, loc, scale } => {
                let z = (x - loc) / scale;
                ln_gamma(0.5 * (df + 1.0)) - ln_gamma(0.5 * df)
                    - 0.5 * (df * std::f64::consts::PI).ln() - scale.ln()
                    - 0.5 * (df + 1.0) * (1.0 + z * z / df).ln()
            }
            Distribution::Dirichlet { alpha } => {
                // For Dirichlet, x should be encoded as first component
                // Full vector version uses sample_dirichlet
                let _ = alpha;
                0.0 // Placeholder for scalar projection
            }
        }
    }

    /// Mean of the distribution.
    pub fn mean(&self) -> f64 {
        match self {
            Distribution::Normal { mean, .. } => *mean,
            Distribution::Bernoulli { p } => *p,
            Distribution::Categorical { probs } => {
                probs.iter().enumerate().map(|(i, &p)| i as f64 * p).sum()
            }
            Distribution::Uniform { low, high } => (low + high) / 2.0,
            Distribution::Beta { alpha, beta } => alpha / (alpha + beta),
            Distribution::Gamma { shape, rate } => shape / rate,
            Distribution::Poisson { lambda } => *lambda,
            Distribution::Exponential { rate } => 1.0 / rate,
            Distribution::StudentT { loc, .. } => *loc,
            Distribution::Dirichlet { alpha } => {
                let sum: f64 = alpha.iter().sum();
                if !alpha.is_empty() { alpha[0] / sum } else { 0.0 }
            }
        }
    }

    /// Variance of the distribution.
    pub fn variance(&self) -> f64 {
        match self {
            Distribution::Normal { std, .. } => std * std,
            Distribution::Bernoulli { p } => p * (1.0 - p),
            Distribution::Uniform { low, high } => (high - low).powi(2) / 12.0,
            Distribution::Beta { alpha, beta } => {
                let s = alpha + beta;
                alpha * beta / (s * s * (s + 1.0))
            }
            Distribution::Gamma { shape, rate } => shape / (rate * rate),
            Distribution::Poisson { lambda } => *lambda,
            Distribution::Exponential { rate } => 1.0 / (rate * rate),
            Distribution::StudentT { df, scale, .. } => {
                if *df > 2.0 { scale * scale * df / (df - 2.0) } else { f64::INFINITY }
            }
            _ => 0.0,
        }
    }

    /// Sample from distribution using a simple LCG PRNG.
    pub fn sample(&self, rng: &mut SimpleRng) -> f64 {
        match self {
            Distribution::Normal { mean, std } => {
                // Box-Muller transform
                let u1 = rng.next_f64().max(1e-15);
                let u2 = rng.next_f64();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                mean + std * z
            }
            Distribution::Bernoulli { p } => {
                if rng.next_f64() < *p { 1.0 } else { 0.0 }
            }
            Distribution::Categorical { probs } => {
                let u = rng.next_f64();
                let mut cum = 0.0;
                for (i, &p) in probs.iter().enumerate() {
                    cum += p;
                    if u < cum { return i as f64; }
                }
                (probs.len() - 1) as f64
            }
            Distribution::Uniform { low, high } => low + (high - low) * rng.next_f64(),
            Distribution::Beta { alpha, beta } => {
                // Use gamma samples: X ~ Gamma(α,1), Y ~ Gamma(β,1) → X/(X+Y) ~ Beta(α,β)
                let x = Distribution::Gamma { shape: *alpha, rate: 1.0 }.sample(rng);
                let y = Distribution::Gamma { shape: *beta, rate: 1.0 }.sample(rng);
                if x + y > 0.0 { x / (x + y) } else { 0.5 }
            }
            Distribution::Gamma { shape, rate } => {
                // Marsaglia-Tsang method
                if *shape < 1.0 {
                    let g = Distribution::Gamma { shape: shape + 1.0, rate: *rate }.sample(rng);
                    return g * rng.next_f64().powf(1.0 / shape);
                }
                let d = shape - 1.0 / 3.0;
                let c = 1.0 / (9.0 * d).sqrt();
                loop {
                    let x = {
                        let u1 = rng.next_f64().max(1e-15);
                        let u2 = rng.next_f64();
                        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
                    };
                    let v = (1.0 + c * x).powi(3);
                    if v > 0.0 {
                        let u = rng.next_f64();
                        if u < 1.0 - 0.0331 * x.powi(4)
                            || u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln())
                        {
                            return d * v / rate;
                        }
                    }
                }
            }
            Distribution::Poisson { lambda } => {
                // Knuth algorithm
                let l = (-lambda).exp();
                let mut k = 0u64;
                let mut p = 1.0;
                loop {
                    k += 1;
                    p *= rng.next_f64();
                    if p <= l { break; }
                }
                (k - 1) as f64
            }
            Distribution::Exponential { rate } => {
                -rng.next_f64().max(1e-15).ln() / rate
            }
            Distribution::StudentT { df, loc, scale } => {
                // Ratio of normal / sqrt(chi²/df)
                let z = Distribution::Normal { mean: 0.0, std: 1.0 }.sample(rng);
                let chi2 = Distribution::Gamma { shape: df / 2.0, rate: 0.5 }.sample(rng);
                loc + scale * z / (chi2 / df).sqrt()
            }
            Distribution::Dirichlet { alpha } => {
                // Return first component of Dirichlet sample
                let gammas: Vec<f64> = alpha.iter().map(|&a| {
                    Distribution::Gamma { shape: a, rate: 1.0 }.sample(rng)
                }).collect();
                let sum: f64 = gammas.iter().sum();
                if sum > 0.0 { gammas[0] / sum } else { 1.0 / alpha.len() as f64 }
            }
        }
    }
}

// ── Simple RNG (deterministic, seedable) ────────────────────────────────

/// Linear congruential generator for reproducible sampling.
#[derive(Debug, Clone)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self { SimpleRng { state: seed.wrapping_add(1) } }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ── Probabilistic Model ────────────────────────────────────────────────

/// A trace records sampled/observed values in a probabilistic program.
#[derive(Debug, Clone, Default)]
pub struct Trace {
    pub values: HashMap<String, f64>,
    pub log_probs: HashMap<String, f64>,
}

impl Trace {
    pub fn new() -> Self { Trace::default() }

    /// Record a sample from a named distribution.
    pub fn sample(&mut self, name: &str, dist: &Distribution, rng: &mut SimpleRng) -> f64 {
        let val = dist.sample(rng);
        let lp = dist.log_prob(val);
        self.values.insert(name.to_string(), val);
        self.log_probs.insert(name.to_string(), lp);
        val
    }

    /// Record an observation (fixed value).
    pub fn observe(&mut self, name: &str, dist: &Distribution, value: f64) {
        let lp = dist.log_prob(value);
        self.values.insert(name.to_string(), value);
        self.log_probs.insert(name.to_string(), lp);
    }

    /// Total log probability of the trace.
    pub fn log_joint(&self) -> f64 {
        self.log_probs.values().sum()
    }
}

// ── MCMC: Metropolis-Hastings ───────────────────────────────────────────

/// Metropolis-Hastings sampler configuration.
#[derive(Debug, Clone)]
pub struct MHConfig {
    pub n_samples: usize,
    pub burn_in: usize,
    pub proposal_std: f64,
}

impl Default for MHConfig {
    fn default() -> Self {
        MHConfig { n_samples: 1000, burn_in: 200, proposal_std: 0.5 }
    }
}

/// Run Metropolis-Hastings on a log-density function.
pub fn metropolis_hastings(
    log_density: impl Fn(f64) -> f64,
    initial: f64,
    config: &MHConfig,
    rng: &mut SimpleRng,
) -> Vec<f64> {
    let mut samples = Vec::with_capacity(config.n_samples);
    let mut current = initial;
    let mut current_lp = log_density(current);

    let total = config.n_samples + config.burn_in;
    for i in 0..total {
        // Propose
        let proposal = current + config.proposal_std * {
            let u1 = rng.next_f64().max(1e-15);
            let u2 = rng.next_f64();
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        };
        let proposal_lp = log_density(proposal);

        // Accept/reject
        let log_alpha = proposal_lp - current_lp;
        if log_alpha >= 0.0 || rng.next_f64().ln() < log_alpha {
            current = proposal;
            current_lp = proposal_lp;
        }

        if i >= config.burn_in {
            samples.push(current);
        }
    }
    samples
}

// ── MCMC: Hamiltonian Monte Carlo ───────────────────────────────────────

/// HMC sampler configuration.
#[derive(Debug, Clone)]
pub struct HMCConfig {
    pub n_samples: usize,
    pub burn_in: usize,
    pub step_size: f64,
    pub n_leapfrog: usize,
}

impl Default for HMCConfig {
    fn default() -> Self {
        HMCConfig { n_samples: 500, burn_in: 100, step_size: 0.1, n_leapfrog: 10 }
    }
}

/// Hamiltonian Monte Carlo for 1D target.
pub fn hmc_sample(
    log_density: impl Fn(f64) -> f64,
    grad_log_density: impl Fn(f64) -> f64,
    initial: f64,
    config: &HMCConfig,
    rng: &mut SimpleRng,
) -> Vec<f64> {
    let mut samples = Vec::with_capacity(config.n_samples);
    let mut q = initial;

    let total = config.n_samples + config.burn_in;
    for i in 0..total {
        // Sample momentum
        let p0 = {
            let u1 = rng.next_f64().max(1e-15);
            let u2 = rng.next_f64();
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        };

        let mut q_new = q;
        let mut p_new = p0;

        // Leapfrog integration
        p_new += 0.5 * config.step_size * grad_log_density(q_new);
        for _ in 0..config.n_leapfrog - 1 {
            q_new += config.step_size * p_new;
            p_new += config.step_size * grad_log_density(q_new);
        }
        q_new += config.step_size * p_new;
        p_new += 0.5 * config.step_size * grad_log_density(q_new);

        // Metropolis correction
        let h_old = -log_density(q) + 0.5 * p0 * p0;
        let h_new = -log_density(q_new) + 0.5 * p_new * p_new;
        let log_alpha = h_old - h_new;

        if log_alpha >= 0.0 || rng.next_f64().ln() < log_alpha {
            q = q_new;
        }

        if i >= config.burn_in {
            samples.push(q);
        }
    }
    samples
}

// ── Variational Inference ───────────────────────────────────────────────

/// Simple mean-field variational inference result.
#[derive(Debug, Clone)]
pub struct VariationalResult {
    pub mean: f64,
    pub std: f64,
    pub elbo_history: Vec<f64>,
}

/// Mean-field ADVI: fit a normal approximation q(z) ≈ p(z|x).
pub fn variational_inference(
    log_density: impl Fn(f64) -> f64,
    n_iter: usize,
    n_samples: usize,
    learning_rate: f64,
    rng: &mut SimpleRng,
) -> VariationalResult {
    let mut mu: f64 = 0.0;
    let mut log_sigma: f64 = 0.0;
    let mut elbo_history = Vec::with_capacity(n_iter);

    for _ in 0..n_iter {
        let sigma = log_sigma.exp();
        let mut grad_mu = 0.0;
        let mut grad_log_sigma = 0.0;
        let mut elbo = 0.0;

        for _ in 0..n_samples {
            // Reparameterization trick: z = μ + σ·ε
            let eps = {
                let u1 = rng.next_f64().max(1e-15);
                let u2 = rng.next_f64();
                (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
            };
            let z = mu + sigma * eps;
            let lp = log_density(z);

            // Entropy of q(z) = N(μ, σ²): H = 0.5 * ln(2πeσ²)
            let log_q = -0.5 * eps * eps - log_sigma - 0.5 * (2.0 * std::f64::consts::PI).ln();
            elbo += lp - log_q;

            // Gradient estimates (score function + reparam)
            grad_mu += (lp - log_q) * eps / sigma;
            grad_log_sigma += (lp - log_q) * (eps * eps - 1.0);
        }

        let ns = n_samples as f64;
        elbo /= ns;
        grad_mu /= ns;
        grad_log_sigma /= ns;

        mu += learning_rate * grad_mu;
        log_sigma += learning_rate * grad_log_sigma;

        elbo_history.push(elbo);
    }

    VariationalResult {
        mean: mu,
        std: log_sigma.exp(),
        elbo_history,
    }
}

// ── Gaussian Process Regression ─────────────────────────────────────────

/// Gaussian Process with squared-exponential kernel.
#[derive(Debug, Clone)]
pub struct GaussianProcess {
    pub length_scale: f64,
    pub signal_variance: f64,
    pub noise_variance: f64,
    pub x_train: Vec<f64>,
    pub y_train: Vec<f64>,
    pub alpha: Vec<f64>, // K⁻¹ y (precomputed)
}

impl GaussianProcess {
    pub fn new(length_scale: f64, signal_variance: f64, noise_variance: f64) -> Self {
        GaussianProcess {
            length_scale, signal_variance, noise_variance,
            x_train: vec![], y_train: vec![], alpha: vec![],
        }
    }

    /// Squared-exponential (RBF) kernel.
    pub fn kernel(&self, x1: f64, x2: f64) -> f64 {
        let diff = x1 - x2;
        self.signal_variance * (-0.5 * diff * diff / (self.length_scale * self.length_scale)).exp()
    }

    /// Fit GP to training data.
    pub fn fit(&mut self, x: &[f64], y: &[f64]) {
        let n = x.len();
        self.x_train = x.to_vec();
        self.y_train = y.to_vec();

        // Build K + σ²I
        let mut k_matrix = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                k_matrix[i * n + j] = self.kernel(x[i], x[j]);
                if i == j { k_matrix[i * n + j] += self.noise_variance; }
            }
        }

        // Solve K α = y via Cholesky (simplified: just use direct inversion for small n)
        self.alpha = solve_symmetric(&k_matrix, y, n);
    }

    /// Predict mean and variance at test point.
    pub fn predict(&self, x_test: f64) -> (f64, f64) {
        let n = self.x_train.len();
        let mut k_star = vec![0.0; n];
        for i in 0..n {
            k_star[i] = self.kernel(x_test, self.x_train[i]);
        }

        // Mean: k* · α
        let mean: f64 = k_star.iter().zip(self.alpha.iter()).map(|(k, a)| k * a).sum();

        // Variance: k** - k*ᵀ K⁻¹ k* (simplified)
        let k_ss = self.kernel(x_test, x_test) + self.noise_variance;
        // Approximate variance reduction
        let var_reduction: f64 = k_star.iter().map(|k| k * k / self.noise_variance).sum();
        let variance = (k_ss - var_reduction).max(1e-10);

        (mean, variance)
    }
}

/// Simple symmetric positive-definite solver (conjugate gradient).
fn solve_symmetric(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut x = vec![0.0; n];
    let mut r: Vec<f64> = b.to_vec();
    let mut p = r.clone();

    for _ in 0..n * 3 {
        // A·p
        let mut ap = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                ap[i] += a[i * n + j] * p[j];
            }
        }

        let r_dot_r: f64 = r.iter().map(|v| v * v).sum();
        if r_dot_r < 1e-15 { break; }

        let p_dot_ap: f64 = p.iter().zip(ap.iter()).map(|(a, b)| a * b).sum();
        if p_dot_ap.abs() < 1e-15 { break; }
        let alpha = r_dot_r / p_dot_ap;

        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }

        let r_new_dot: f64 = r.iter().map(|v| v * v).sum();
        if r_new_dot < 1e-15 { break; }

        let beta = r_new_dot / r_dot_r;
        for i in 0..n {
            p[i] = r[i] + beta * p[i];
        }
    }
    x
}

// ── Bayesian Neural Network Layer ───────────────────────────────────────

/// Bayesian linear layer with weight uncertainty.
#[derive(Debug, Clone)]
pub struct BayesianLinear {
    pub in_features: usize,
    pub out_features: usize,
    pub weight_mean: Vec<f64>,
    pub weight_log_std: Vec<f64>,
    pub bias_mean: Vec<f64>,
    pub bias_log_std: Vec<f64>,
}

impl BayesianLinear {
    pub fn new(in_features: usize, out_features: usize, rng: &mut SimpleRng) -> Self {
        let n_weights = in_features * out_features;
        let scale = (2.0 / (in_features + out_features) as f64).sqrt();

        let weight_mean: Vec<f64> = (0..n_weights).map(|_| {
            let u1 = rng.next_f64().max(1e-15);
            let u2 = rng.next_f64();
            scale * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }).collect();
        let weight_log_std = vec![-3.0; n_weights]; // Start with small variance
        let bias_mean = vec![0.0; out_features];
        let bias_log_std = vec![-3.0; out_features];

        BayesianLinear {
            in_features, out_features,
            weight_mean, weight_log_std,
            bias_mean, bias_log_std,
        }
    }

    /// Forward pass with sampled weights (reparameterization trick).
    pub fn forward(&self, input: &[f64], rng: &mut SimpleRng) -> Vec<f64> {
        let mut output = self.bias_mean.clone();

        for j in 0..self.out_features {
            for i in 0..self.in_features {
                let idx = j * self.in_features + i;
                let eps = {
                    let u1 = rng.next_f64().max(1e-15);
                    let u2 = rng.next_f64();
                    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
                };
                let w = self.weight_mean[idx] + self.weight_log_std[idx].exp() * eps;
                output[j] += w * input[i];
            }
            // Add bias noise
            let eps = {
                let u1 = rng.next_f64().max(1e-15);
                let u2 = rng.next_f64();
                (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
            };
            output[j] += self.bias_log_std[j].exp() * eps;
        }
        output
    }

    /// KL divergence of weight posterior from prior N(0, 1).
    pub fn kl_divergence(&self) -> f64 {
        let mut kl = 0.0;
        for i in 0..self.weight_mean.len() {
            let mu = self.weight_mean[i];
            let sigma = self.weight_log_std[i].exp();
            kl += 0.5 * (sigma * sigma + mu * mu - 1.0 - 2.0 * self.weight_log_std[i]);
        }
        for i in 0..self.bias_mean.len() {
            let mu = self.bias_mean[i];
            let sigma = self.bias_log_std[i].exp();
            kl += 0.5 * (sigma * sigma + mu * mu - 1.0 - 2.0 * self.bias_log_std[i]);
        }
        kl
    }
}

// ── Helper Math Functions ───────────────────────────────────────────────

/// Stirling's approximation for ln(Γ(x)).
fn ln_gamma(x: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    // Lanczos approximation (7-term)
    let g = 7.0;
    let c = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];
    if x < 0.5 {
        let reflected = std::f64::consts::PI / (std::f64::consts::PI * x).sin();
        reflected.abs().ln() - ln_gamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let mut sum = c[0];
        for i in 1..9 {
            sum += c[i] / (x + i as f64);
        }
        let t = x + g + 0.5;
        0.5 * (2.0 * std::f64::consts::PI).ln() + (t.ln() * (x + 0.5)) - t + sum.ln()
    }
}

/// ln(B(α, β)) = ln(Γ(α)) + ln(Γ(β)) - ln(Γ(α+β))
fn ln_beta(a: f64, b: f64) -> f64 {
    ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
}

/// ln(n!) via ln_gamma(n+1).
fn ln_factorial(n: u64) -> f64 {
    ln_gamma(n as f64 + 1.0)
}

// ── FFI Interface ───────────────────────────────────────────────────────

static PROB_STORES: Mutex<Option<HashMap<i64, Distribution>>> = Mutex::new(None);

fn prob_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, Distribution>>> {
    PROB_STORES.lock().unwrap()
}

fn next_prob_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_prob_normal(mean: f64, std: f64) -> i64 {
    let id = next_prob_id();
    let mut store = prob_store();
    store.get_or_insert_with(HashMap::new).insert(id, Distribution::Normal { mean, std });
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_prob_log_prob(dist_id: i64, x: f64) -> f64 {
    let store = prob_store();
    store.as_ref().and_then(|s| s.get(&dist_id)).map(|d| d.log_prob(x)).unwrap_or(f64::NEG_INFINITY)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_prob_sample(dist_id: i64, seed: i64) -> f64 {
    let store = prob_store();
    let dist = store.as_ref().and_then(|s| s.get(&dist_id)).cloned();
    drop(store);
    match dist {
        Some(d) => {
            let mut rng = SimpleRng::new(seed as u64);
            d.sample(&mut rng)
        }
        None => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_prob_free(dist_id: i64) {
    let mut store = prob_store();
    if let Some(s) = store.as_mut() { s.remove(&dist_id); }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_mcmc_normal_mean(data_ptr: *const f64, n: i64, n_samples: i64, seed: i64) -> f64 {
    let data = unsafe { std::slice::from_raw_parts(data_ptr, n as usize) };
    let mut rng = SimpleRng::new(seed as u64);
    let config = MHConfig { n_samples: n_samples as usize, burn_in: 100, proposal_std: 0.3 };

    let samples = metropolis_hastings(
        |mu| {
            // Normal prior: N(0, 10)
            let prior = Distribution::Normal { mean: 0.0, std: 10.0 }.log_prob(mu);
            // Likelihood: product of N(mu, 1)
            let lik: f64 = data.iter().map(|&x| Distribution::Normal { mean: mu, std: 1.0 }.log_prob(x)).sum();
            prior + lik
        },
        0.0,
        &config,
        &mut rng,
    );

    samples.iter().sum::<f64>() / samples.len() as f64
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_log_prob() {
        let d = Distribution::Normal { mean: 0.0, std: 1.0 };
        let lp = d.log_prob(0.0);
        let expected = -0.5 * (2.0 * std::f64::consts::PI).ln();
        assert!((lp - expected).abs() < 1e-10);
    }

    #[test]
    fn test_bernoulli() {
        let d = Distribution::Bernoulli { p: 0.7 };
        assert!((d.log_prob(1.0) - 0.7_f64.ln()).abs() < 1e-10);
        assert!((d.log_prob(0.0) - 0.3_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_uniform() {
        let d = Distribution::Uniform { low: 0.0, high: 1.0 };
        assert!((d.log_prob(0.5) - 0.0).abs() < 1e-10);
        assert!(d.log_prob(1.5) == f64::NEG_INFINITY);
    }

    #[test]
    fn test_normal_sample() {
        let d = Distribution::Normal { mean: 5.0, std: 1.0 };
        let mut rng = SimpleRng::new(42);
        let samples: Vec<f64> = (0..1000).map(|_| d.sample(&mut rng)).collect();
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 5.0).abs() < 0.3); // Should be close to 5
    }

    #[test]
    fn test_exponential() {
        let d = Distribution::Exponential { rate: 2.0 };
        assert!((d.mean() - 0.5).abs() < 1e-10);
        let lp = d.log_prob(1.0);
        assert!((lp - (2.0_f64.ln() - 2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_distribution_mean_variance() {
        let d = Distribution::Normal { mean: 3.0, std: 2.0 };
        assert!((d.mean() - 3.0).abs() < 1e-10);
        assert!((d.variance() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_categorical() {
        let d = Distribution::Categorical { probs: vec![0.2, 0.3, 0.5] };
        let mut rng = SimpleRng::new(42);
        let mut counts = [0u32; 3];
        for _ in 0..1000 {
            let s = d.sample(&mut rng) as usize;
            if s < 3 { counts[s] += 1; }
        }
        // All categories should be sampled
        assert!(counts[0] > 50);
        assert!(counts[1] > 50);
        assert!(counts[2] > 50);
    }

    #[test]
    fn test_beta_sample() {
        let d = Distribution::Beta { alpha: 2.0, beta: 5.0 };
        let mut rng = SimpleRng::new(42);
        let samples: Vec<f64> = (0..1000).map(|_| d.sample(&mut rng)).collect();
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        let expected_mean = 2.0 / 7.0;
        assert!((mean - expected_mean).abs() < 0.1);
    }

    #[test]
    fn test_gamma_sample() {
        let d = Distribution::Gamma { shape: 3.0, rate: 2.0 };
        let mut rng = SimpleRng::new(42);
        let samples: Vec<f64> = (0..1000).map(|_| d.sample(&mut rng)).collect();
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 1.5).abs() < 0.3); // E[X] = shape/rate = 1.5
    }

    #[test]
    fn test_trace() {
        let mut trace = Trace::new();
        let mut rng = SimpleRng::new(42);
        let d = Distribution::Normal { mean: 0.0, std: 1.0 };
        let _ = trace.sample("z", &d, &mut rng);
        trace.observe("x", &d, 1.0);
        assert!(trace.values.contains_key("z"));
        assert!(trace.values.contains_key("x"));
        assert!(trace.log_joint().is_finite());
    }

    #[test]
    fn test_metropolis_hastings() {
        // Sample from N(3, 1): log p(x) = -0.5*(x-3)²
        let mut rng = SimpleRng::new(42);
        let config = MHConfig { n_samples: 2000, burn_in: 500, proposal_std: 1.0 };
        let samples = metropolis_hastings(
            |x| -0.5 * (x - 3.0).powi(2),
            0.0,
            &config,
            &mut rng,
        );
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 3.0).abs() < 0.5);
    }

    #[test]
    fn test_hmc() {
        let mut rng = SimpleRng::new(42);
        let config = HMCConfig { n_samples: 500, burn_in: 100, step_size: 0.05, n_leapfrog: 20 };
        let samples = hmc_sample(
            |x| -0.5 * (x - 2.0).powi(2),
            |x| -(x - 2.0),
            0.0,
            &config,
            &mut rng,
        );
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 2.0).abs() < 0.5);
    }

    #[test]
    fn test_variational_inference() {
        // Fit q(z) to N(5, 1)
        let mut rng = SimpleRng::new(42);
        let result = variational_inference(
            |x| -0.5 * (x - 5.0).powi(2),
            200, 10, 0.01,
            &mut rng,
        );
        assert!((result.mean - 5.0).abs() < 2.0); // Should converge near 5
        assert!(result.elbo_history.len() == 200);
    }

    #[test]
    fn test_gaussian_process() {
        let mut gp = GaussianProcess::new(1.0, 1.0, 0.01);
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![0.0, 1.0, 4.0, 9.0]; // y ≈ x²
        gp.fit(&x, &y);

        let (mean, var) = gp.predict(1.5);
        assert!((mean - 2.25).abs() < 1.5); // Should interpolate near 1.5² = 2.25
        assert!(var > 0.0);
    }

    #[test]
    fn test_bayesian_linear() {
        let mut rng = SimpleRng::new(42);
        let layer = BayesianLinear::new(3, 2, &mut rng);
        let input = vec![1.0, 2.0, 3.0];

        let out = layer.forward(&input, &mut rng);
        assert_eq!(out.len(), 2);

        let kl = layer.kl_divergence();
        assert!(kl > 0.0); // KL should be positive
    }

    #[test]
    fn test_ffi_normal() {
        let id = vitalis_prob_normal(0.0, 1.0);
        assert!(id > 0);
        let lp = vitalis_prob_log_prob(id, 0.0);
        assert!(lp < 0.0);
        assert!(lp > -2.0);
        vitalis_prob_free(id);
    }

    #[test]
    fn test_ffi_sample() {
        let id = vitalis_prob_normal(5.0, 0.1);
        let s = vitalis_prob_sample(id, 42);
        assert!((s - 5.0).abs() < 2.0);
        vitalis_prob_free(id);
    }

    #[test]
    fn test_poisson() {
        let d = Distribution::Poisson { lambda: 3.0 };
        assert!((d.mean() - 3.0).abs() < 1e-10);
        let mut rng = SimpleRng::new(42);
        let samples: Vec<f64> = (0..1000).map(|_| d.sample(&mut rng)).collect();
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 3.0).abs() < 0.5);
    }

    #[test]
    fn test_student_t() {
        let d = Distribution::StudentT { df: 10.0, loc: 0.0, scale: 1.0 };
        assert!((d.mean() - 0.0).abs() < 1e-10);
        assert!(d.variance() > 1.0); // Var = df/(df-2) = 10/8 = 1.25
    }
}
