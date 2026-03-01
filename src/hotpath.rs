//! Hot-path native modules — performance-critical operations compiled to native code.
//!
//! These are Rust implementations of Python hot-path functions for high-performance numeric operations, exposed via C FFI so Python can call them for maximum throughput.
//!
//! # Ported algorithms:
//! 1. **Sliding window rate limiter** — filters expired timestamps (rate limitation)
//! 2. **Token bucket** — atomic token consumption (rate limitation)
//! 3. **P95 latency computation** — percentile calculation (monitoring)
//! 4. **Quality/fitness scoring** — weighted metric scoring (code analysis)
//! 5. **Vote tallying** — consensus counting (vote tallying)
//! 6. **Cognitive complexity** — depth-weighted AST complexity (code analysis)

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── 1. Sliding Window Rate Limiter ────────────────────────────────────

/// Filter a sliding window of timestamps, removing those older than `window_seconds`.
/// Returns the count of valid (non-expired) entries.
///
/// `timestamps` is a pointer to an array of f64 (epoch seconds).
/// `count` is the number of timestamps.
/// `now` is the current time (epoch seconds).
/// `window_seconds` is the window duration.
///
/// # Safety
/// `timestamps` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_sliding_window_count(
    timestamps: *const f64,
    count: usize,
    now: f64,
    window_seconds: f64,
) -> usize {
    if timestamps.is_null() || count == 0 {
        return 0;
    }
    let ts = unsafe { std::slice::from_raw_parts(timestamps, count) };
    let cutoff = now - window_seconds;
    ts.iter().filter(|&&t| t >= cutoff).count()
}

/// Filter a sliding window in-place, returning the new count of valid entries.
/// Valid entries are compacted to the front of the array.
///
/// # Safety
/// `timestamps` must point to a valid mutable array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_sliding_window_compact(
    timestamps: *mut f64,
    count: usize,
    now: f64,
    window_seconds: f64,
) -> usize {
    if timestamps.is_null() || count == 0 {
        return 0;
    }
    let ts = unsafe { std::slice::from_raw_parts_mut(timestamps, count) };
    let cutoff = now - window_seconds;
    let mut write = 0;
    for read in 0..count {
        if ts[read] >= cutoff {
            ts[write] = ts[read];
            write += 1;
        }
    }
    write
}

// ─── 2. Token Bucket ──────────────────────────────────────────────────

/// Token bucket: check if `cost` tokens can be consumed.
/// Returns the new token count (>= 0 if allowed, < 0 if denied).
///
/// - `tokens`: current token count
/// - `max_tokens`: bucket capacity
/// - `refill_rate`: tokens per second
/// - `elapsed_seconds`: time since last refill
/// - `cost`: tokens to consume
#[unsafe(no_mangle)]
pub extern "C" fn hotpath_token_bucket(
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    elapsed_seconds: f64,
    cost: f64,
) -> f64 {
    // Refill tokens based on elapsed time
    let refilled = (tokens + refill_rate * elapsed_seconds).min(max_tokens);
    // Try to consume
    refilled - cost
}

// ─── 3. Percentile Computation ─────────────────────────────────────────

/// Compute the P95 (95th percentile) of a latency array.
/// Returns the p95 value, or -1.0 if the array is empty.
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_p95(values: *const f64, count: usize) -> f64 {
    if values.is_null() || count == 0 {
        return -1.0;
    }
    let vals = unsafe { std::slice::from_raw_parts(values, count) };
    let mut sorted: Vec<f64> = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let idx = ((count as f64 * 0.95).ceil() as usize).saturating_sub(1).min(count - 1);
    sorted[idx]
}

/// Compute arbitrary percentile of an array.
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_percentile(
    values: *const f64,
    count: usize,
    pct: f64,
) -> f64 {
    if values.is_null() || count == 0 || !(0.0..=1.0).contains(&pct) {
        return -1.0;
    }
    let vals = unsafe { std::slice::from_raw_parts(values, count) };
    let mut sorted: Vec<f64> = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let idx = ((count as f64 * pct).ceil() as usize).saturating_sub(1).min(count - 1);
    sorted[idx]
}

/// Compute mean of an array.
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_mean(values: *const f64, count: usize) -> f64 {
    if values.is_null() || count == 0 {
        return 0.0;
    }
    let vals = unsafe { std::slice::from_raw_parts(values, count) };
    vals.iter().sum::<f64>() / count as f64
}

// ─── 4. Quality / Fitness Scoring ──────────────────────────────────────

/// Compute a weighted quality score from multiple metrics.
///
/// `metrics` is an array of f64 metric values.
/// `weights` is an array of f64 weights (same length).
/// Returns the weighted sum, clamped to [0.0, 1.0].
///
/// # Safety
/// `metrics` and `weights` must point to valid arrays of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_weighted_score(
    metrics: *const f64,
    weights: *const f64,
    count: usize,
) -> f64 {
    if metrics.is_null() || weights.is_null() || count == 0 {
        return 0.0;
    }
    let m = unsafe { std::slice::from_raw_parts(metrics, count) };
    let w = unsafe { std::slice::from_raw_parts(weights, count) };

    let total_weight: f64 = w.iter().sum();
    if total_weight <= 0.0 {
        return 0.0;
    }

    let score: f64 = m.iter().zip(w.iter()).map(|(mi, wi)| mi * wi).sum::<f64>() / total_weight;
    score.clamp(0.0, 1.0)
}

/// Compute a code quality score for code analysis.
/// Takes raw metrics and returns a 0-100 score.
///
/// Inputs:
/// - `cyclomatic`: cyclomatic complexity
/// - `cognitive`: cognitive complexity
/// - `loc`: lines of code
/// - `num_functions`: number of functions
/// - `security_issues`: number of security issues
/// - `has_tests`: whether tests exist (0 or 1)
#[unsafe(no_mangle)]
pub extern "C" fn hotpath_code_quality_score(
    cyclomatic: f64,
    cognitive: f64,
    loc: f64,
    num_functions: f64,
    security_issues: f64,
    has_tests: i32,
) -> f64 {
    // Base score
    let mut score: f64 = 100.0;

    // Complexity penalties
    if num_functions > 0.0 {
        let avg_complexity = cyclomatic / num_functions;
        if avg_complexity > 10.0 {
            score -= (avg_complexity - 10.0) * 2.0;
        }
    }

    // Cognitive complexity penalty
    let cog_per_func = if num_functions > 0.0 { cognitive / num_functions } else { cognitive };
    if cog_per_func > 15.0 {
        score -= (cog_per_func - 15.0) * 1.5;
    }

    // Size penalty (very large files)
    if loc > 500.0 {
        score -= ((loc - 500.0) / 100.0) * 1.0;
    }

    // Security deduction
    score -= security_issues * 10.0;

    // Test bonus
    if has_tests != 0 {
        score += 5.0;
    }

    score.clamp(0.0, 100.0)
}

// ─── 5. Vote Tallying ──────────────────────────────────────────────────

/// Tally votes from an array of integer vote choices.
/// Returns the winning choice and writes agreement percentage to `agreement_out`.
///
/// `votes` is an array of i32 vote choices (0-indexed option numbers).
/// Returns the winning option index, or -1 if empty.
///
/// # Safety
/// `votes` must point to a valid array of `count` i32 values.
/// `agreement_out` must be a valid pointer to f64.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_tally_votes(
    votes: *const i32,
    count: usize,
    agreement_out: *mut f64,
) -> i32 {
    if votes.is_null() || count == 0 {
        if !agreement_out.is_null() {
            unsafe { *agreement_out = 0.0 };
        }
        return -1;
    }
    let v = unsafe { std::slice::from_raw_parts(votes, count) };

    // Count votes per option
    let mut counts = std::collections::HashMap::<i32, usize>::new();
    for &vote in v {
        *counts.entry(vote).or_default() += 1;
    }

    // Find winner
    let (&winner, &best_count) = counts
        .iter()
        .max_by_key(|&(_, &c)| c)
        .unwrap_or((&-1, &0));

    if !agreement_out.is_null() {
        unsafe { *agreement_out = best_count as f64 / count as f64 };
    }

    winner
}

/// Tally string-based votes (e.g., agent responses).
/// Returns JSON: `{"winner": "...", "agreement": 0.XX, "counts": {...}}`.
/// Caller must free with `slang_free_string`.
///
/// # Safety
/// `votes_json` must be a valid null-terminated JSON array string: `["a","b","a"]`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_tally_string_votes(votes_json: *const c_char) -> *mut c_char {
    let input = match unsafe { CStr::from_ptr(votes_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring("{}"),
    };

    // Simple JSON array parser (no dependency needed)
    let votes: Vec<String> = parse_json_string_array(input);
    if votes.is_empty() {
        return alloc_cstring("{\"winner\":\"\",\"agreement\":0.0,\"counts\":{}}");
    }

    let mut counts = std::collections::HashMap::<String, usize>::new();
    for v in &votes {
        *counts.entry(v.clone()).or_default() += 1;
    }

    let total = votes.len();
    let (winner, best_count) = counts
        .iter()
        .max_by_key(|&(_, &c)| c)
        .map(|(k, &c)| (k.clone(), c))
        .unwrap_or_default();

    let agreement = best_count as f64 / total as f64;
    let counts_json: String = counts
        .iter()
        .map(|(k, v)| format!("\"{}\":{}", k, v))
        .collect::<Vec<_>>()
        .join(",");

    alloc_cstring(&format!(
        "{{\"winner\":\"{}\",\"agreement\":{:.4},\"counts\":{{{}}}}}",
        winner, agreement, counts_json
    ))
}

// ─── 6. Cognitive Complexity ───────────────────────────────────────────

/// Compute cognitive complexity from nesting depths.
/// `depths` is an array where each entry represents a control-flow node's nesting depth.
/// Complexity = sum of (1 + depth) for each node.
///
/// # Safety
/// `depths` must point to a valid array of `count` u32 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_cognitive_complexity(
    depths: *const u32,
    count: usize,
) -> u64 {
    if depths.is_null() || count == 0 {
        return 0;
    }
    let d = unsafe { std::slice::from_raw_parts(depths, count) };
    d.iter().map(|&depth| (1 + depth) as u64).sum()
}

// ─── 7. Quantum-Inspired & Mathematical Optimization ───────────────────
//
// These operations power autonomous self-evolution with cutting-edge algorithms:
//   - Quantum annealing acceptance (Metropolis + tunneling)
//   - Bayesian UCB (Upper Confidence Bound) acquisition
//   - Boltzmann/softmax selection probabilities
//   - Shannon entropy diversity measurement
//   - Pareto dominance for multi-objective optimization
//   - CMA-ES mean vector update (covariance matrix adaptation)
//   - Exponential moving average for fitness trend tracking
//   - Lévy flight step generation for mutation magnitude

/// Quantum-inspired annealing acceptance criterion.
///
/// Decides whether to accept a mutation with `new_fitness` vs `old_fitness`.
/// Combines the classical Metropolis criterion with a quantum tunneling term:
///
/// $$P_{accept} = \min\bigl(1,\; e^{(\Delta f) / T}\bigr) + \gamma \cdot e^{-\text{barrier\_width}^2 / T}$$
///
/// - `temperature`: controls exploration breadth (higher = more exploration)
/// - `tunnel_strength`: γ, probability boost from quantum tunneling [0, 1]
/// - `barrier_width`: estimated distance between solutions (0 = adjacent)
///
/// Returns 1 if accepted, 0 if rejected (deterministic for better-fitness;
/// probabilistic for worse-fitness using the seeded value `rand_uniform`).
///
/// # Arguments
/// * `old_fitness` — fitness of current solution
/// * `new_fitness` — fitness of candidate solution
/// * `temperature` — annealing temperature (> 0)
/// * `tunnel_strength` — quantum tunneling coefficient γ ∈ [0, 1]
/// * `barrier_width` — mutation distance between old and new
/// * `rand_uniform` — pre-generated uniform random value ∈ [0, 1)
#[unsafe(no_mangle)]
pub extern "C" fn hotpath_quantum_anneal_accept(
    old_fitness: f64,
    new_fitness: f64,
    temperature: f64,
    tunnel_strength: f64,
    barrier_width: f64,
    rand_uniform: f64,
) -> i32 {
    // Always accept improvements
    if new_fitness >= old_fitness {
        return 1;
    }

    let temp = temperature.max(1e-12); // prevent division by zero
    let delta = new_fitness - old_fitness; // negative for worse solutions

    // Classical Metropolis: P = exp(Δf / T)
    let metropolis = (delta / temp).exp();

    // Quantum tunneling: P_tunnel = γ · exp(-barrier² / T)
    // Models probability of "tunneling through" the fitness barrier
    let tunnel = tunnel_strength.clamp(0.0, 1.0) * (-barrier_width * barrier_width / temp).exp();

    // Combined probability (clamped to [0, 1])
    let accept_prob = (metropolis + tunnel).min(1.0);

    if rand_uniform < accept_prob { 1 } else { 0 }
}

/// Bayesian Upper Confidence Bound (UCB1) acquisition score.
///
/// Computes: $\text{UCB} = \bar{x}_i + \kappa \sqrt{\frac{\ln N}{n_i}}$
///
/// Used to decide WHICH evolvable function to mutate next, balancing:
/// - **Exploitation**: prefer functions with high mean fitness
/// - **Exploration**: prefer functions tried fewer times
///
/// # Arguments
/// * `mean_fitness` — average fitness of this function's variants
/// * `num_trials` — number of times this function has been evolved (n_i)
/// * `total_trials` — total evolution steps across all functions (N)
/// * `kappa` — exploration coefficient (√2 ≈ 1.414 is theoretically optimal)
///
/// Returns the UCB score (higher = should evolve next).
#[unsafe(no_mangle)]
pub extern "C" fn hotpath_bayesian_ucb(
    mean_fitness: f64,
    num_trials: u64,
    total_trials: u64,
    kappa: f64,
) -> f64 {
    if num_trials == 0 {
        return f64::MAX; // Never tried → infinite priority
    }
    let n_i = num_trials as f64;
    let big_n = (total_trials.max(1)) as f64;
    mean_fitness + kappa * (big_n.ln() / n_i).sqrt()
}

/// Boltzmann (softmax) selection probabilities.
///
/// Computes: $p_i = \frac{e^{f_i / T}}{\sum_j e^{f_j / T}}$
///
/// Temperature controls selection pressure:
/// - T → 0: greedy selection (always pick best)
/// - T → ∞: uniform random selection
/// - T ≈ 1: moderate selection pressure
///
/// # Safety
/// `fitnesses` must point to a valid array of `count` f64 values.
/// `probs_out` must point to a writable array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_boltzmann_select(
    fitnesses: *const f64,
    count: usize,
    temperature: f64,
    probs_out: *mut f64,
) {
    if fitnesses.is_null() || probs_out.is_null() || count == 0 {
        return;
    }
    let f = unsafe { std::slice::from_raw_parts(fitnesses, count) };
    let out = unsafe { std::slice::from_raw_parts_mut(probs_out, count) };
    let temp = temperature.max(1e-12);

    // Numerically stable softmax: subtract max to prevent overflow
    let max_f = f.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exp_sum: f64 = f.iter().map(|&fi| ((fi - max_f) / temp).exp()).sum();

    for i in 0..count {
        out[i] = ((f[i] - max_f) / temp).exp() / exp_sum;
    }
}

/// Shannon entropy of a probability distribution (diversity metric).
///
/// $H = -\sum_i p_i \ln(p_i)$ where $p_i > 0$
///
/// Returns 0.0 for empty/degenerate input. Max entropy = ln(n) for uniform.
/// Normalized to [0, 1] by dividing by ln(count).
///
/// # Safety
/// `probs` must point to a valid array of `count` f64 values that sum to ~1.0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_shannon_diversity(
    probs: *const f64,
    count: usize,
) -> f64 {
    if probs.is_null() || count <= 1 {
        return 0.0;
    }
    let p = unsafe { std::slice::from_raw_parts(probs, count) };
    let max_entropy = (count as f64).ln();
    if max_entropy <= 0.0 {
        return 0.0;
    }

    let entropy: f64 = p.iter()
        .filter(|&&pi| pi > 1e-15) // skip zero-probability entries
        .map(|&pi| -pi * pi.ln())
        .sum();

    (entropy / max_entropy).clamp(0.0, 1.0)
}

/// Check if solution A Pareto-dominates solution B.
///
/// A dominates B iff ∀i: A[i] ≥ B[i] AND ∃j: A[j] > B[j]
/// (all objectives at least as good, at least one strictly better)
///
/// # Safety
/// `a` and `b` must point to valid arrays of `n_objectives` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_pareto_dominates(
    a: *const f64,
    b: *const f64,
    n_objectives: usize,
) -> i32 {
    if a.is_null() || b.is_null() || n_objectives == 0 {
        return 0;
    }
    let va = unsafe { std::slice::from_raw_parts(a, n_objectives) };
    let vb = unsafe { std::slice::from_raw_parts(b, n_objectives) };

    let mut all_geq = true;
    let mut any_greater = false;

    for i in 0..n_objectives {
        if va[i] < vb[i] {
            all_geq = false;
            break;
        }
        if va[i] > vb[i] {
            any_greater = true;
        }
    }

    if all_geq && any_greater { 1 } else { 0 }
}

/// Compute Pareto front indices from a population of multi-objective solutions.
///
/// Returns the number of non-dominated solutions. Their indices are written to
/// `front_indices_out` (must be pre-allocated with capacity `pop_size`).
///
/// Each solution has `n_objectives` objectives. Solutions are stored as a flat
/// row-major array: solution[i][j] = objectives[i * n_objectives + j].
///
/// # Safety
/// `objectives` must point to `pop_size * n_objectives` f64 values.
/// `front_indices_out` must point to a writable array of `pop_size` u32 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_pareto_front(
    objectives: *const f64,
    pop_size: usize,
    n_objectives: usize,
    front_indices_out: *mut u32,
) -> usize {
    if objectives.is_null() || front_indices_out.is_null() || pop_size == 0 || n_objectives == 0 {
        return 0;
    }
    let objs = unsafe { std::slice::from_raw_parts(objectives, pop_size * n_objectives) };
    let out = unsafe { std::slice::from_raw_parts_mut(front_indices_out, pop_size) };

    let mut front_count = 0usize;

    for i in 0..pop_size {
        let mut dominated = false;
        for j in 0..pop_size {
            if i == j {
                continue;
            }
            // Check if j dominates i
            let mut j_all_geq = true;
            let mut j_any_greater = false;
            for k in 0..n_objectives {
                let oj = objs[j * n_objectives + k];
                let oi = objs[i * n_objectives + k];
                if oj < oi {
                    j_all_geq = false;
                    break;
                }
                if oj > oi {
                    j_any_greater = true;
                }
            }
            if j_all_geq && j_any_greater {
                dominated = true;
                break;
            }
        }
        if !dominated {
            out[front_count] = i as u32;
            front_count += 1;
        }
    }

    front_count
}

/// CMA-ES mean vector update step.
///
/// Updates the distribution mean based on the weighted recombination of the
/// best `mu` solutions from a population of `lambda` candidates.
///
/// $m_{new} = \sum_{i=1}^{\mu} w_i \cdot x_{i:\lambda}$
///
/// where $x_{i:\lambda}$ are the `mu` best solutions ranked by fitness.
///
/// # Arguments
/// * `solutions` — flat array of `lambda * dim` f64 values (each solution is `dim` floats)
/// * `fitnesses` — array of `lambda` fitness values (higher = better)
/// * `lambda` — total population size (number of candidate solutions)
/// * `mu` — number of best solutions to use (typically lambda/2)
/// * `dim` — dimensionality of each solution vector
/// * `mean_out` — output array of `dim` f64 values for the updated mean
///
/// # Safety
/// All pointer args must be valid and correctly sized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_cma_es_mean_update(
    solutions: *const f64,
    fitnesses: *const f64,
    lambda: usize,
    mu: usize,
    dim: usize,
    mean_out: *mut f64,
) {
    if solutions.is_null() || fitnesses.is_null() || mean_out.is_null()
        || lambda == 0 || mu == 0 || dim == 0 || mu > lambda
    {
        return;
    }
    let sols = unsafe { std::slice::from_raw_parts(solutions, lambda * dim) };
    let fits = unsafe { std::slice::from_raw_parts(fitnesses, lambda) };
    let out = unsafe { std::slice::from_raw_parts_mut(mean_out, dim) };

    // Rank solutions by fitness (descending)
    let mut indices: Vec<usize> = (0..lambda).collect();
    indices.sort_by(|&a, &b| fits[b].partial_cmp(&fits[a]).unwrap_or(std::cmp::Ordering::Equal));

    // CMA-ES log-linear weights: w_i = ln(μ+0.5) - ln(i+1), normalized
    let mu_f = mu as f64;
    let mut weights: Vec<f64> = (0..mu)
        .map(|i| (mu_f + 0.5).ln() - ((i + 1) as f64).ln())
        .collect();
    let w_sum: f64 = weights.iter().sum();
    for w in &mut weights {
        *w /= w_sum;
    }

    // Weighted recombination: m_new = Σ w_i * x_{i:λ}
    for d in 0..dim {
        out[d] = 0.0;
    }
    for (rank, &idx) in indices.iter().take(mu).enumerate() {
        let sol_start = idx * dim;
        for d in 0..dim {
            out[d] += weights[rank] * sols[sol_start + d];
        }
    }
}

/// Exponential Moving Average update.
///
/// $\text{EMA}_{new} = \alpha \cdot x + (1 - \alpha) \cdot \text{EMA}_{old}$
///
/// Used for tracking fitness trends over evolution generations.
///
/// # Arguments
/// * `ema_old` — previous EMA value
/// * `new_value` — latest observation
/// * `alpha` — smoothing factor ∈ (0, 1]. Higher = more responsive.
#[unsafe(no_mangle)]
pub extern "C" fn hotpath_ema_update(
    ema_old: f64,
    new_value: f64,
    alpha: f64,
) -> f64 {
    let a = alpha.clamp(0.0, 1.0);
    a * new_value + (1.0 - a) * ema_old
}

/// Lévy flight step magnitude for mutation.
///
/// Generates a Lévy-distributed step size that enables both small local
/// mutations (exploitation) and occasional large jumps (exploration).
///
/// Uses the Mantegna algorithm approximation:
/// $s = \frac{u}{|v|^{1/\beta}}$ where $u \sim N(0, \sigma_u^2)$, $v \sim N(0, 1)$
///
/// Since we don't have a PRNG here, the caller provides pre-generated
/// standard normal samples u and v.
///
/// # Arguments
/// * `u_normal` — standard normal sample for numerator
/// * `v_normal` — standard normal sample for denominator
/// * `beta` — Lévy exponent ∈ (0, 2]. β=1.5 is typical.
/// * `scale` — overall step scale multiplier
#[unsafe(no_mangle)]
pub extern "C" fn hotpath_levy_step(
    u_normal: f64,
    v_normal: f64,
    beta: f64,
    scale: f64,
) -> f64 {
    let b = beta.clamp(0.1, 2.0);

    // Mantegna's approximation of sigma_u
    // σ_u = { Γ(1+β) sin(πβ/2) / [Γ((1+β)/2) · β · 2^((β-1)/2)] }^{1/β}
    // For β=1.5: σ_u ≈ 0.6966
    // We use a direct approximation for common β values
    let sigma_u = mantegna_sigma(b);

    let step = (sigma_u * u_normal) / v_normal.abs().powf(1.0 / b);
    scale * step
}

/// Approximate Mantegna sigma for Lévy flight.
fn mantegna_sigma(beta: f64) -> f64 {
    // Γ(1+β) sin(πβ/2) / [Γ((1+β)/2) · β · 2^((β-1)/2)]
    // Use Stirling's approximation for the gamma function ratio
    let pi = std::f64::consts::PI;
    let numerator = gamma_approx(1.0 + beta) * (pi * beta / 2.0).sin();
    let denominator = gamma_approx((1.0 + beta) / 2.0) * beta * 2.0_f64.powf((beta - 1.0) / 2.0);
    if denominator.abs() < 1e-15 {
        return 1.0;
    }
    (numerator / denominator).powf(1.0 / beta)
}

/// Lanczos approximation of the Gamma function.
fn gamma_approx(z: f64) -> f64 {
    if z < 0.5 {
        // Reflection formula: Γ(z) = π / [sin(πz) · Γ(1-z)]
        let pi = std::f64::consts::PI;
        return pi / ((pi * z).sin() * gamma_approx(1.0 - z));
    }
    // Lanczos coefficients (g=7)
    let coeffs = [
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
    let z = z - 1.0;
    let mut x = coeffs[0];
    for (i, &c) in coeffs.iter().enumerate().skip(1) {
        x += c / (z + i as f64);
    }
    let t = z + 7.5; // g + 0.5
    let pi2 = (2.0 * std::f64::consts::PI).sqrt();
    pi2 * t.powf(z + 0.5) * (-t).exp() * x
}

/// Multi-objective fitness score combining speed, correctness, complexity, and security.
///
/// Produces a single scalar from multiple objectives using adaptive weighting:
/// - `speed_score`: 0-1 (higher = faster)
/// - `correctness_score`: 0-1 (higher = more correct)
/// - `complexity_score`: 0-1 (higher = simpler / less complex)
/// - `security_score`: 0-1 (higher = more secure)
/// - `generation`: which evolution generation (used for adaptive weighting)
///
/// Early generations weight exploration (correctness + simplicity);
/// later generations weight exploitation (speed + security).
#[unsafe(no_mangle)]
pub extern "C" fn hotpath_adaptive_fitness(
    speed_score: f64,
    correctness_score: f64,
    complexity_score: f64,
    security_score: f64,
    generation: u64,
) -> f64 {
    let generation_f = generation as f64;

    // Adaptive weights shift from exploration → exploitation over generations
    // Early: correctness 0.4, complexity 0.3, speed 0.2, security 0.1
    // Late:  correctness 0.25, complexity 0.15, speed 0.35, security 0.25
    let maturity = (generation_f / 50.0).min(1.0); // 0→1 over 50 generations

    let w_correct = 0.40 - 0.15 * maturity;  // 0.40 → 0.25
    let w_complex = 0.30 - 0.15 * maturity;  // 0.30 → 0.15
    let w_speed   = 0.20 + 0.15 * maturity;  // 0.20 → 0.35
    let w_secure  = 0.10 + 0.15 * maturity;  // 0.10 → 0.25

    let score = w_correct * correctness_score.clamp(0.0, 1.0)
              + w_complex * complexity_score.clamp(0.0, 1.0)
              + w_speed   * speed_score.clamp(0.0, 1.0)
              + w_secure  * security_score.clamp(0.0, 1.0);

    score.clamp(0.0, 1.0)
}

// ─── Helpers ───────────────────────────────────────────────────────────

fn alloc_cstring(s: &str) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("ERROR").unwrap())
        .into_raw()
}

/// Minimal JSON string array parser: ["a","b","c"] → vec!["a","b","c"]
fn parse_json_string_array(input: &str) -> Vec<String> {
    let trimmed = input.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Vec::new();
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut result = Vec::new();
    let mut in_string = false;
    let mut current = String::new();
    let mut escape = false;

    for ch in inner.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape = true,
            '"' => {
                if in_string {
                    result.push(current.clone());
                    current.clear();
                }
                in_string = !in_string;
            }
            _ if in_string => current.push(ch),
            _ => {} // skip commas, whitespace outside strings
        }
    }
    result
}

// ─── 8. Vector Operations (Phase 21) ──────────────────────────────────

/// Cosine similarity between two vectors.
/// Returns a value in [-1, 1]: 1 = identical direction, 0 = orthogonal, -1 = opposite.
///
/// # Safety
/// `a` and `b` must point to valid arrays of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_cosine_similarity(
    a: *const f64,
    b: *const f64,
    count: usize,
) -> f64 {
    if a.is_null() || b.is_null() || count == 0 {
        return 0.0;
    }
    let va = unsafe { std::slice::from_raw_parts(a, count) };
    let vb = unsafe { std::slice::from_raw_parts(b, count) };

    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;

    for i in 0..count {
        dot += va[i] * vb[i];
        norm_a += va[i] * va[i];
        norm_b += vb[i] * vb[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-15 { return 0.0; }
    (dot / denom).clamp(-1.0, 1.0)
}

/// L2-normalize a vector in place. Returns the original magnitude.
///
/// # Safety
/// `values` must point to a valid mutable array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_l2_normalize(
    values: *mut f64,
    count: usize,
) -> f64 {
    if values.is_null() || count == 0 {
        return 0.0;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    let magnitude: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if magnitude < 1e-15 {
        return 0.0;
    }
    for x in v.iter_mut() {
        *x /= magnitude;
    }
    magnitude
}

/// Standard deviation of an array of f64 values.
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_stddev(
    values: *const f64,
    count: usize,
) -> f64 {
    if values.is_null() || count < 2 {
        return 0.0;
    }
    let v = unsafe { std::slice::from_raw_parts(values, count) };
    let mean = v.iter().sum::<f64>() / count as f64;
    let variance = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (count - 1) as f64;
    variance.sqrt()
}

/// Median of an array (sorts a copy).
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_median(
    values: *const f64,
    count: usize,
) -> f64 {
    if values.is_null() || count == 0 {
        return 0.0;
    }
    let v = unsafe { std::slice::from_raw_parts(values, count) };
    let mut sorted: Vec<f64> = v.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if count % 2 == 1 {
        sorted[count / 2]
    } else {
        (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
    }
}

// ─── 9. Phase 22: Advanced Analytics ────────────────────────────────────

/// Exponential Moving Average over a full series.
/// Returns the final EMA value after processing all data points.
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_exponential_moving_average(
    values: *const f64,
    count: usize,
    alpha: f64,
) -> f64 {
    if values.is_null() || count == 0 {
        return 0.0;
    }
    let v = unsafe { std::slice::from_raw_parts(values, count) };
    let mut ema = v[0];
    for i in 1..count {
        ema = alpha * v[i] + (1.0 - alpha) * ema;
    }
    ema
}

/// Shannon entropy of a probability distribution (in bits).
/// Input values should be non-negative and sum to ~1.0.
///
/// # Safety
/// `probs` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_entropy(
    probs: *const f64,
    count: usize,
) -> f64 {
    if probs.is_null() || count == 0 {
        return 0.0;
    }
    let p = unsafe { std::slice::from_raw_parts(probs, count) };
    let mut h = 0.0;
    for &pi in p {
        if pi > 0.0 {
            h -= pi * pi.log2();
        }
    }
    h
}

/// Min-max normalization of a vector in-place.
/// Returns the range (max - min). If range is 0, all values become 0.
///
/// # Safety
/// `values` must point to a valid mutable array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_min_max_normalize(
    values: *mut f64,
    count: usize,
) -> f64 {
    if values.is_null() || count == 0 {
        return 0.0;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;
    for &x in v.iter() {
        if x < min_val { min_val = x; }
        if x > max_val { max_val = x; }
    }
    let range = max_val - min_val;
    if range.abs() < 1e-15 {
        for x in v.iter_mut() { *x = 0.0; }
        return 0.0;
    }
    for x in v.iter_mut() {
        *x = (*x - min_val) / range;
    }
    range
}

/// Hamming distance between two i64 values (count of differing bits).
#[unsafe(no_mangle)]
pub extern "C" fn hotpath_hamming_distance(a: i64, b: i64) -> i64 {
    (a ^ b).count_ones() as i64
}

// ─── 10. Phase 23: ML Operations ────────────────────────────────────────

/// Softmax over a vector (in-place). Numerically stable (subtracts max first).
///
/// # Safety
/// `values` must point to a valid mutable array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_softmax(
    values: *mut f64,
    count: usize,
) {
    if values.is_null() || count == 0 {
        return;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    let max_val = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut sum = 0.0;
    for x in v.iter_mut() {
        *x = (*x - max_val).exp();
        sum += *x;
    }
    if sum > 0.0 {
        for x in v.iter_mut() {
            *x /= sum;
        }
    }
}

/// Cross-entropy loss: -sum(target * ln(predicted)).
/// Both arrays must have the same length.
///
/// # Safety
/// `target` and `predicted` must point to valid arrays of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_cross_entropy(
    target: *const f64,
    predicted: *const f64,
    count: usize,
) -> f64 {
    if target.is_null() || predicted.is_null() || count == 0 {
        return 0.0;
    }
    let t = unsafe { std::slice::from_raw_parts(target, count) };
    let p = unsafe { std::slice::from_raw_parts(predicted, count) };
    let mut loss = 0.0;
    for i in 0..count {
        let pi = p[i].max(1e-15); // clamp to avoid ln(0)
        loss -= t[i] * pi.ln();
    }
    loss
}

/// Batch sigmoid: apply sigmoid(x) = 1/(1+e^-x) to each element in-place.
///
/// # Safety
/// `values` must point to a valid mutable array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_batch_sigmoid(
    values: *mut f64,
    count: usize,
) {
    if values.is_null() || count == 0 {
        return;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    for x in v.iter_mut() {
        *x = 1.0 / (1.0 + (-*x).exp());
    }
}

/// Argmax: returns the index of the maximum value in the array.
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_argmax(
    values: *const f64,
    count: usize,
) -> usize {
    if values.is_null() || count == 0 {
        return 0;
    }
    let v = unsafe { std::slice::from_raw_parts(values, count) };
    let mut max_idx = 0;
    let mut max_val = v[0];
    for i in 1..count {
        if v[i] > max_val {
            max_val = v[i];
            max_idx = i;
        }
    }
    max_idx
}

/// Batch ReLU: apply max(0, x) to each element in-place.
///
/// # Safety
/// `values` must point to a valid mutable array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_batch_relu(
    values: *mut f64,
    count: usize,
) {
    if values.is_null() || count == 0 {
        return;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    for x in v.iter_mut() {
        if *x < 0.0 { *x = 0.0; }
    }
}

// ─── Phase 24: Advanced ML & Optimization Ops ──────────────────────────

/// Batch Leaky ReLU: max(alpha*x, x) for each element
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_batch_leaky_relu(
    values: *mut f64,
    count: usize,
    alpha: f64,
) {
    if values.is_null() || count == 0 {
        return;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    for x in v.iter_mut() {
        if *x < 0.0 { *x = alpha * *x; }
    }
}

/// Batch normalization: normalize values to zero mean and unit variance,
/// then scale by gamma and shift by beta.
/// Returns the mean of the original values.
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_batch_norm(
    values: *mut f64,
    count: usize,
    gamma: f64,
    beta: f64,
    epsilon: f64,
) -> f64 {
    if values.is_null() || count == 0 {
        return 0.0;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    let n = count as f64;
    let mean = v.iter().sum::<f64>() / n;
    let variance = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let std_dev = (variance + epsilon).sqrt();
    for x in v.iter_mut() {
        *x = gamma * (*x - mean) / std_dev + beta;
    }
    mean
}

/// KL divergence: D_KL(P || Q) = sum(p * ln(p/q))
/// Computes KL divergence between distributions P and Q.
///
/// # Safety
/// `p` and `q` must point to valid arrays of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_kl_divergence(
    p: *const f64,
    q: *const f64,
    count: usize,
) -> f64 {
    if p.is_null() || q.is_null() || count == 0 {
        return 0.0;
    }
    let p_slice = unsafe { std::slice::from_raw_parts(p, count) };
    let q_slice = unsafe { std::slice::from_raw_parts(q, count) };
    let mut kl = 0.0;
    for i in 0..count {
        let pi = p_slice[i];
        let qi = q_slice[i].max(1e-12); // avoid log(0)
        if pi > 1e-12 {
            kl += pi * (pi / qi).ln();
        }
    }
    kl
}

/// Batch GELU activation (approximate): x * 0.5 * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_gelu_batch(
    values: *mut f64,
    count: usize,
) {
    if values.is_null() || count == 0 {
        return;
    }
    let c = (2.0_f64 / std::f64::consts::PI).sqrt();
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    for x in v.iter_mut() {
        let inner = c * (*x + 0.044715 * x.powi(3));
        *x = 0.5 * *x * (1.0 + inner.tanh());
    }
}

/// Clip/clamp all values to [min_val, max_val] range
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_clip(
    values: *mut f64,
    count: usize,
    min_val: f64,
    max_val: f64,
) {
    if values.is_null() || count == 0 {
        return;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    for x in v.iter_mut() {
        *x = x.clamp(min_val, max_val);
    }
}

// ── Phase 25: Numerical Linear Algebra & Loss Operations ─────────────

/// Layer normalization: normalize across the feature dimension
/// Returns the mean. Modifies values in-place: (x - mean) / sqrt(var + eps) * gamma + beta
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_layer_norm(
    values: *mut f64,
    count: usize,
    gamma: f64,
    beta: f64,
    epsilon: f64,
) -> f64 {
    if values.is_null() || count == 0 {
        return 0.0;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    let mean = v.iter().sum::<f64>() / count as f64;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / count as f64;
    let inv_std = 1.0 / (var + epsilon).sqrt();
    for x in v.iter_mut() {
        *x = (*x - mean) * inv_std * gamma + beta;
    }
    mean
}

/// Deterministic dropout mask: zero out elements at regular intervals
/// `keep_prob` determines the fraction of elements to keep (e.g., 0.8 = keep 80%)
/// `seed` provides deterministic reproducibility
///
/// # Safety
/// `values` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_dropout_mask(
    values: *mut f64,
    count: usize,
    keep_prob: f64,
    seed: u64,
) {
    if values.is_null() || count == 0 || keep_prob >= 1.0 {
        return;
    }
    let v = unsafe { std::slice::from_raw_parts_mut(values, count) };
    let mut state = seed;
    let threshold = (keep_prob * u64::MAX as f64) as u64;
    let scale = 1.0 / keep_prob;
    for x in v.iter_mut() {
        // Xorshift64
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        if state < threshold {
            *x *= scale; // Scale up kept values
        } else {
            *x = 0.0; // Drop
        }
    }
}

/// Cosine distance: 1 - cosine_similarity
///
/// # Safety
/// `a` and `b` must point to valid arrays of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_cosine_distance(
    a: *const f64,
    b: *const f64,
    count: usize,
) -> f64 {
    if a.is_null() || b.is_null() || count == 0 {
        return 1.0;
    }
    let a = unsafe { std::slice::from_raw_parts(a, count) };
    let b = unsafe { std::slice::from_raw_parts(b, count) };
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..count {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom < 1e-15 {
        1.0
    } else {
        1.0 - dot / denom
    }
}

/// Huber loss (smooth L1 loss): mean of element-wise Huber loss
/// For |y - p| <= delta: 0.5 * (y-p)^2
/// For |y - p| > delta: delta * (|y-p| - 0.5 * delta)
///
/// # Safety
/// `targets` and `predicted` must point to valid arrays of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_huber_loss(
    targets: *const f64,
    predicted: *const f64,
    count: usize,
    delta: f64,
) -> f64 {
    if targets.is_null() || predicted.is_null() || count == 0 {
        return 0.0;
    }
    let t = unsafe { std::slice::from_raw_parts(targets, count) };
    let p = unsafe { std::slice::from_raw_parts(predicted, count) };
    let mut total = 0.0;
    for i in 0..count {
        let diff = (t[i] - p[i]).abs();
        if diff <= delta {
            total += 0.5 * diff * diff;
        } else {
            total += delta * (diff - 0.5 * delta);
        }
    }
    total / count as f64
}

/// Mean squared error loss: mean((targets - predicted)^2)
///
/// # Safety
/// `targets` and `predicted` must point to valid arrays of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_mse_loss(
    targets: *const f64,
    predicted: *const f64,
    count: usize,
) -> f64 {
    if targets.is_null() || predicted.is_null() || count == 0 {
        return 0.0;
    }
    let t = unsafe { std::slice::from_raw_parts(targets, count) };
    let p = unsafe { std::slice::from_raw_parts(predicted, count) };
    let mut total = 0.0;
    for i in 0..count {
        let d = t[i] - p[i];
        total += d * d;
    }
    total / count as f64
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window() {
        let now = 1000.0;
        let timestamps = vec![990.0, 995.0, 999.0, 980.0, 970.0];
        let count = unsafe {
            hotpath_sliding_window_count(timestamps.as_ptr(), timestamps.len(), now, 15.0)
        };
        // 990, 995, 999 are within 15s; 980 is borderline (1000-15=985 → excluded), 970 excluded
        assert_eq!(count, 3);
    }

    #[test]
    fn test_sliding_window_compact() {
        let mut timestamps = vec![990.0, 995.0, 999.0, 980.0, 970.0];
        let count = unsafe {
            hotpath_sliding_window_compact(timestamps.as_mut_ptr(), timestamps.len(), 1000.0, 15.0)
        };
        assert_eq!(count, 3);
        assert_eq!(&timestamps[..count], &[990.0, 995.0, 999.0]);
    }

    #[test]
    fn test_token_bucket() {
        // 5 tokens, max 10, refill 2/s, 3s elapsed, cost 4
        let remaining = hotpath_token_bucket(5.0, 10.0, 2.0, 3.0, 4.0);
        // 5 + 2*3 = 11, capped to 10, minus 4 = 6
        assert_eq!(remaining, 6.0);

        // Over-consume
        let remaining = hotpath_token_bucket(1.0, 10.0, 1.0, 0.5, 5.0);
        // 1 + 1*0.5 = 1.5, minus 5 = -3.5 (denied)
        assert!(remaining < 0.0);
    }

    #[test]
    fn test_p95() {
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let p95 = unsafe { hotpath_p95(values.as_ptr(), values.len()) };
        assert_eq!(p95, 95.0);
    }

    #[test]
    fn test_percentile_median() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let median = unsafe { hotpath_percentile(values.as_ptr(), values.len(), 0.5) };
        assert_eq!(median, 3.0);
    }

    #[test]
    fn test_mean() {
        let values = vec![10.0, 20.0, 30.0];
        let m = unsafe { hotpath_mean(values.as_ptr(), values.len()) };
        assert!((m - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_weighted_score() {
        let metrics = vec![0.8, 0.6, 0.9];
        let weights = vec![1.0, 1.0, 1.0];
        let score = unsafe {
            hotpath_weighted_score(metrics.as_ptr(), weights.as_ptr(), metrics.len())
        };
        // (0.8 + 0.6 + 0.9) / 3 = 0.7667
        assert!((score - 0.7667).abs() < 0.01);
    }

    #[test]
    fn test_code_quality_score() {
        let score = hotpath_code_quality_score(20.0, 30.0, 200.0, 5.0, 0.0, 1);
        // cyclomatic/func = 4 (no penalty), cognitive/func = 6 (no penalty), loc ok, tests +5
        assert!(score > 90.0); // Should be ~105, clamped to 100
        assert!(score <= 100.0);
    }

    #[test]
    fn test_tally_votes() {
        let votes = vec![1, 2, 1, 1, 2, 3];
        let mut agreement = 0.0;
        let winner = unsafe {
            hotpath_tally_votes(votes.as_ptr(), votes.len(), &mut agreement)
        };
        assert_eq!(winner, 1);
        assert!((agreement - 0.5).abs() < 0.01); // 3/6 = 50%
    }

    #[test]
    fn test_tally_string_votes() {
        let input = CString::new(r#"["yes","no","yes","yes","no"]"#).unwrap();
        let result = unsafe { hotpath_tally_string_votes(input.as_ptr()) };
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { crate::bridge::slang_free_string(result) };
        assert!(s.contains("\"winner\":\"yes\""));
        assert!(s.contains("\"agreement\":0.6"));
    }

    #[test]
    fn test_cognitive_complexity() {
        let depths = vec![0u32, 1, 2, 1, 0];
        let complexity = unsafe {
            hotpath_cognitive_complexity(depths.as_ptr(), depths.len())
        };
        // (1+0) + (1+1) + (1+2) + (1+1) + (1+0) = 1+2+3+2+1 = 9
        assert_eq!(complexity, 9);
    }

    #[test]
    fn test_parse_json_string_array() {
        let result = parse_json_string_array(r#"["hello","world"]"#);
        assert_eq!(result, vec!["hello", "world"]);
    }

    // ─── Quantum-Inspired & Mathematical Optimization Tests ────────────

    #[test]
    fn test_quantum_anneal_accept_improvement() {
        // Better fitness → always accept
        let result = hotpath_quantum_anneal_accept(0.5, 0.8, 1.0, 0.1, 0.5, 0.99);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_quantum_anneal_accept_worse_high_temp() {
        // Worse fitness but high temperature → likely accept
        let result = hotpath_quantum_anneal_accept(0.8, 0.7, 100.0, 0.0, 0.0, 0.5);
        assert_eq!(result, 1); // exp(-0.1/100) ≈ 0.999 > 0.5
    }

    #[test]
    fn test_quantum_anneal_accept_worse_low_temp() {
        // Worse fitness, low temperature, no tunneling → reject
        let result = hotpath_quantum_anneal_accept(0.8, 0.3, 0.001, 0.0, 5.0, 0.5);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_quantum_anneal_tunneling() {
        // Worse fitness, low temp, but strong tunneling + small barrier → accept
        let result = hotpath_quantum_anneal_accept(0.8, 0.3, 0.5, 1.0, 0.1, 0.5);
        // tunneling = 1.0 * exp(-0.01/0.5) = exp(-0.02) ≈ 0.98
        // metropolis = exp(-0.5/0.5) = exp(-1) ≈ 0.368
        // total ≈ 1.0 (clamped), 0.5 < 1.0 → accept
        assert_eq!(result, 1);
    }

    #[test]
    fn test_bayesian_ucb_untried() {
        let score = hotpath_bayesian_ucb(0.0, 0, 100, 1.414);
        assert_eq!(score, f64::MAX); // Never tried → infinite priority
    }

    #[test]
    fn test_bayesian_ucb_exploited() {
        // High mean, many trials → high score from exploitation
        let score = hotpath_bayesian_ucb(0.9, 50, 200, 1.414);
        // 0.9 + 1.414 * sqrt(ln(200)/50) ≈ 0.9 + 1.414 * sqrt(0.106) ≈ 0.9 + 0.46 ≈ 1.36
        assert!(score > 1.0);
        assert!(score < 2.0);
    }

    #[test]
    fn test_bayesian_ucb_exploration() {
        // Low mean, few trials, many total → UCB explores
        let score_few = hotpath_bayesian_ucb(0.3, 2, 200, 1.414);
        let score_many = hotpath_bayesian_ucb(0.3, 50, 200, 1.414);
        assert!(score_few > score_many); // Fewer trials → higher exploration bonus
    }

    #[test]
    fn test_boltzmann_select() {
        let fitnesses = vec![1.0, 2.0, 3.0];
        let mut probs = vec![0.0; 3];
        unsafe {
            hotpath_boltzmann_select(fitnesses.as_ptr(), 3, 1.0, probs.as_mut_ptr());
        }
        // Probabilities should sum to 1.0
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
        // Higher fitness → higher probability
        assert!(probs[2] > probs[1]);
        assert!(probs[1] > probs[0]);
    }

    #[test]
    fn test_boltzmann_select_high_temp() {
        let fitnesses = vec![0.1, 0.5, 0.9];
        let mut probs = vec![0.0; 3];
        unsafe {
            hotpath_boltzmann_select(fitnesses.as_ptr(), 3, 1000.0, probs.as_mut_ptr());
        }
        // Very high temperature → nearly uniform
        for p in &probs {
            assert!((*p - 1.0 / 3.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_shannon_diversity_uniform() {
        // Uniform distribution → maximum diversity → 1.0
        let probs = vec![0.25, 0.25, 0.25, 0.25];
        let diversity = unsafe { hotpath_shannon_diversity(probs.as_ptr(), probs.len()) };
        assert!((diversity - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_shannon_diversity_concentrated() {
        // All weight on one → minimum diversity → 0.0
        let probs = vec![1.0, 0.0, 0.0, 0.0];
        let diversity = unsafe { hotpath_shannon_diversity(probs.as_ptr(), probs.len()) };
        assert!(diversity < 0.01);
    }

    #[test]
    fn test_shannon_diversity_partial() {
        let probs = vec![0.7, 0.2, 0.1];
        let diversity = unsafe { hotpath_shannon_diversity(probs.as_ptr(), probs.len()) };
        assert!(diversity > 0.3 && diversity < 0.9);
    }

    #[test]
    fn test_pareto_dominates_yes() {
        let a = vec![0.8, 0.9, 0.7];
        let b = vec![0.7, 0.8, 0.7]; // a ≥ b in all, a > b in first two
        let result = unsafe { hotpath_pareto_dominates(a.as_ptr(), b.as_ptr(), 3) };
        assert_eq!(result, 1);
    }

    #[test]
    fn test_pareto_dominates_no() {
        let a = vec![0.8, 0.7, 0.9]; // a better in 0,2
        let b = vec![0.7, 0.9, 0.8]; // b better in 1 → no domination
        let result = unsafe { hotpath_pareto_dominates(a.as_ptr(), b.as_ptr(), 3) };
        assert_eq!(result, 0);
    }

    #[test]
    fn test_pareto_dominates_equal() {
        let a = vec![0.5, 0.5];
        let result = unsafe { hotpath_pareto_dominates(a.as_ptr(), a.as_ptr(), 2) };
        assert_eq!(result, 0); // Equal → no strict domination
    }

    #[test]
    fn test_pareto_front() {
        // 4 solutions, 2 objectives
        // Sol 0: (0.9, 0.1) — best at obj0
        // Sol 1: (0.1, 0.9) — best at obj1
        // Sol 2: (0.5, 0.5) — balanced, dominated by neither 0 nor 1
        // Sol 3: (0.3, 0.3) — dominated by sol 2
        let objectives = vec![
            0.9, 0.1,
            0.1, 0.9,
            0.5, 0.5,
            0.3, 0.3,
        ];
        let mut front = vec![0u32; 4];
        let count = unsafe {
            hotpath_pareto_front(objectives.as_ptr(), 4, 2, front.as_mut_ptr())
        };
        assert_eq!(count, 3); // Sol 0, 1, 2 are non-dominated
        let front_set: Vec<u32> = front[..count].to_vec();
        assert!(front_set.contains(&0));
        assert!(front_set.contains(&1));
        assert!(front_set.contains(&2));
        assert!(!front_set.contains(&3)); // Dominated
    }

    #[test]
    fn test_cma_es_mean_update() {
        // 4 solutions, dim=2, use best 2 (mu=2)
        let solutions = vec![
            1.0, 2.0,   // sol 0: fitness 0.5
            3.0, 4.0,   // sol 1: fitness 0.9 ← best
            5.0, 6.0,   // sol 2: fitness 0.7 ← second best
            7.0, 8.0,   // sol 3: fitness 0.3
        ];
        let fitnesses = vec![0.5, 0.9, 0.7, 0.3];
        let mut mean_out = vec![0.0; 2];
        unsafe {
            hotpath_cma_es_mean_update(
                solutions.as_ptr(), fitnesses.as_ptr(),
                4, 2, 2, mean_out.as_mut_ptr()
            );
        }
        // Best two: sol 1 (3,4) and sol 2 (5,6)
        // Weights: w1 = ln(2.5)-ln(1) = 0.916, w2 = ln(2.5)-ln(2) = 0.223
        // Normalized: w1 ≈ 0.804, w2 ≈ 0.196
        // mean ≈ 0.804*(3,4) + 0.196*(5,6) = (3.39, 4.39)
        assert!(mean_out[0] > 3.0 && mean_out[0] < 5.0);
        assert!(mean_out[1] > 4.0 && mean_out[1] < 6.0);
        // Should be weighted toward the best solution
        assert!(mean_out[0] < 4.0); // closer to sol 1's x=3
    }

    #[test]
    fn test_ema_update() {
        let ema = hotpath_ema_update(0.5, 1.0, 0.3);
        // 0.3 * 1.0 + 0.7 * 0.5 = 0.3 + 0.35 = 0.65
        assert!((ema - 0.65).abs() < 1e-10);
    }

    #[test]
    fn test_ema_update_full_weight() {
        let ema = hotpath_ema_update(0.5, 1.0, 1.0);
        assert!((ema - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ema_update_zero_weight() {
        let ema = hotpath_ema_update(0.5, 1.0, 0.0);
        assert!((ema - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_levy_step() {
        // With u=1.0, v=1.0, beta=1.5, scale=1.0
        let step = hotpath_levy_step(1.0, 1.0, 1.5, 1.0);
        // Should be finite and non-zero
        assert!(step.is_finite());
        assert!(step.abs() > 0.0);
    }

    #[test]
    fn test_levy_step_scaling() {
        let step1 = hotpath_levy_step(1.0, 1.0, 1.5, 1.0);
        let step2 = hotpath_levy_step(1.0, 1.0, 1.5, 2.0);
        assert!((step2 / step1 - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_fitness_early_gen() {
        // Early generation: correctness and complexity matter more
        let score = hotpath_adaptive_fitness(0.5, 1.0, 1.0, 0.5, 0);
        // w: correct=0.4, complex=0.3, speed=0.2, secure=0.1
        // 0.4*1.0 + 0.3*1.0 + 0.2*0.5 + 0.1*0.5 = 0.4+0.3+0.1+0.05 = 0.85
        assert!((score - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_fitness_late_gen() {
        // Late generation: speed and security matter more
        let score = hotpath_adaptive_fitness(1.0, 0.5, 0.5, 1.0, 100);
        // w: correct=0.25, complex=0.15, speed=0.35, secure=0.25
        // 0.25*0.5 + 0.15*0.5 + 0.35*1.0 + 0.25*1.0 = 0.125+0.075+0.35+0.25 = 0.80
        assert!((score - 0.80).abs() < 0.01);
    }

    #[test]
    fn test_gamma_approx() {
        // Γ(1) = 1, Γ(2) = 1, Γ(3) = 2, Γ(4) = 6
        assert!((gamma_approx(1.0) - 1.0).abs() < 1e-8);
        assert!((gamma_approx(2.0) - 1.0).abs() < 1e-8);
        assert!((gamma_approx(3.0) - 2.0).abs() < 1e-8);
        assert!((gamma_approx(4.0) - 6.0).abs() < 1e-6);
        // Γ(0.5) = √π ≈ 1.7725
        assert!((gamma_approx(0.5) - std::f64::consts::PI.sqrt()).abs() < 1e-6);
    }

    // ─── Phase 21: Vector Operations Tests ────────────────────────────

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let result = unsafe { hotpath_cosine_similarity(a.as_ptr(), a.as_ptr(), 3) };
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let result = unsafe { hotpath_cosine_similarity(a.as_ptr(), b.as_ptr(), 2) };
        assert!(result.abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let result = unsafe { hotpath_cosine_similarity(a.as_ptr(), b.as_ptr(), 3) };
        assert!((result + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_l2_normalize() {
        let mut v = vec![3.0, 4.0];
        let mag = unsafe { hotpath_l2_normalize(v.as_mut_ptr(), 2) };
        assert!((mag - 5.0).abs() < 1e-10);
        assert!((v[0] - 0.6).abs() < 1e-10);
        assert!((v[1] - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_stddev() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = unsafe { hotpath_stddev(values.as_ptr(), values.len()) };
        // Sample std dev ≈ 2.138
        assert!(sd > 1.5 && sd < 2.5);
    }

    #[test]
    fn test_median_odd() {
        let values = vec![3.0, 1.0, 2.0, 5.0, 4.0];
        let med = unsafe { hotpath_median(values.as_ptr(), values.len()) };
        assert!((med - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_median_even() {
        let values = vec![1.0, 2.0, 3.0, 4.0];
        let med = unsafe { hotpath_median(values.as_ptr(), values.len()) };
        assert!((med - 2.5).abs() < 1e-10);
    }

    // ─── Phase 22: Advanced Analytics Tests ─────────────────────────

    #[test]
    fn test_exponential_moving_average() {
        // Simple series: 1, 2, 3, 4, 5 with alpha=0.5
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ema = unsafe { hotpath_exponential_moving_average(values.as_ptr(), values.len(), 0.5) };
        // EMA: 1.0 → 1.5 → 2.25 → 3.125 → 4.0625
        assert!((ema - 4.0625).abs() < 1e-10);
    }

    #[test]
    fn test_exponential_moving_average_alpha1() {
        let values = vec![1.0, 2.0, 3.0];
        let ema = unsafe { hotpath_exponential_moving_average(values.as_ptr(), values.len(), 1.0) };
        // Alpha=1 means just take the last value
        assert!((ema - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_entropy_uniform() {
        // Uniform distribution of 4 outcomes: entropy = log2(4) = 2.0 bits
        let probs = vec![0.25, 0.25, 0.25, 0.25];
        let h = unsafe { hotpath_entropy(probs.as_ptr(), probs.len()) };
        assert!((h - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_entropy_certain() {
        // Certain outcome: entropy = 0
        let probs = vec![1.0, 0.0, 0.0];
        let h = unsafe { hotpath_entropy(probs.as_ptr(), probs.len()) };
        assert!(h.abs() < 1e-10);
    }

    #[test]
    fn test_min_max_normalize() {
        let mut values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let range = unsafe { hotpath_min_max_normalize(values.as_mut_ptr(), values.len()) };
        assert!((range - 40.0).abs() < 1e-10);
        assert!((values[0] - 0.0).abs() < 1e-10);
        assert!((values[2] - 0.5).abs() < 1e-10);
        assert!((values[4] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_hamming_distance() {
        // 0b0000 vs 0b1111 = 4 differing bits
        assert_eq!(hotpath_hamming_distance(0, 15), 4);
        // Same values = 0
        assert_eq!(hotpath_hamming_distance(42, 42), 0);
        // 1 vs 0 = 1 bit
        assert_eq!(hotpath_hamming_distance(0, 1), 1);
    }

    // ─── Phase 23: ML Operations Tests ──────────────────────────────

    #[test]
    fn test_softmax() {
        let mut values = vec![1.0, 2.0, 3.0];
        unsafe { hotpath_softmax(values.as_mut_ptr(), values.len()) };
        // Sum should be 1.0
        let sum: f64 = values.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
        // Should be ascending
        assert!(values[0] < values[1]);
        assert!(values[1] < values[2]);
    }

    #[test]
    fn test_softmax_numerical_stability() {
        // Large values shouldn't overflow
        let mut values = vec![1000.0, 1001.0, 1002.0];
        unsafe { hotpath_softmax(values.as_mut_ptr(), values.len()) };
        let sum: f64 = values.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cross_entropy() {
        let target = vec![1.0, 0.0, 0.0];
        let predicted = vec![0.9, 0.05, 0.05];
        let loss = unsafe { hotpath_cross_entropy(target.as_ptr(), predicted.as_ptr(), 3) };
        // -1.0 * ln(0.9) ≈ 0.1054
        assert!(loss > 0.0 && loss < 0.2);
    }

    #[test]
    fn test_batch_sigmoid() {
        let mut values = vec![0.0, 100.0, -100.0];
        unsafe { hotpath_batch_sigmoid(values.as_mut_ptr(), values.len()) };
        assert!((values[0] - 0.5).abs() < 1e-10);
        assert!((values[1] - 1.0).abs() < 1e-5);
        assert!(values[2].abs() < 1e-5);
    }

    #[test]
    fn test_argmax() {
        let values = vec![1.0, 5.0, 3.0, 2.0];
        let idx = unsafe { hotpath_argmax(values.as_ptr(), values.len()) };
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_argmax_first() {
        let values = vec![10.0, 5.0, 3.0];
        let idx = unsafe { hotpath_argmax(values.as_ptr(), values.len()) };
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_batch_relu() {
        let mut values = vec![-3.0, -1.0, 0.0, 1.0, 5.0];
        unsafe { hotpath_batch_relu(values.as_mut_ptr(), values.len()) };
        assert!((values[0]).abs() < 1e-10);
        assert!((values[1]).abs() < 1e-10);
        assert!((values[2]).abs() < 1e-10);
        assert!((values[3] - 1.0).abs() < 1e-10);
        assert!((values[4] - 5.0).abs() < 1e-10);
    }

    // ─── Phase 24: Advanced ML Operations Tests ─────────────────────────

    #[test]
    fn test_batch_leaky_relu() {
        let mut values = vec![-2.0, -1.0, 0.0, 1.0, 3.0];
        unsafe { hotpath_batch_leaky_relu(values.as_mut_ptr(), values.len(), 0.1) };
        assert!((values[0] - (-0.2)).abs() < 1e-10);
        assert!((values[1] - (-0.1)).abs() < 1e-10);
        assert!((values[2]).abs() < 1e-10);
        assert!((values[3] - 1.0).abs() < 1e-10);
        assert!((values[4] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_batch_norm() {
        let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mean = unsafe { hotpath_batch_norm(values.as_mut_ptr(), values.len(), 1.0, 0.0, 1e-5) };
        assert!((mean - 3.0).abs() < 1e-10);
        // After normalization with gamma=1, beta=0: sum should be ~0
        let sum: f64 = values.iter().sum();
        assert!(sum.abs() < 1e-8);
    }

    #[test]
    fn test_batch_norm_scale_shift() {
        let mut values = vec![1.0, 3.0];
        let mean = unsafe { hotpath_batch_norm(values.as_mut_ptr(), values.len(), 2.0, 5.0, 1e-5) };
        assert!((mean - 2.0).abs() < 1e-10);
        // gamma=2, beta=5: centered, scaled by 2, shifted by 5
        assert!((values[0] - 3.0).abs() < 0.1); // ~ 2*(-1) + 5 = 3
        assert!((values[1] - 7.0).abs() < 0.1); // ~ 2*(1) + 5 = 7
    }

    #[test]
    fn test_kl_divergence_identical() {
        let p = vec![0.25, 0.25, 0.25, 0.25];
        let q = vec![0.25, 0.25, 0.25, 0.25];
        let kl = unsafe { hotpath_kl_divergence(p.as_ptr(), q.as_ptr(), 4) };
        assert!(kl.abs() < 1e-10);
    }

    #[test]
    fn test_kl_divergence_different() {
        let p = vec![0.5, 0.5];
        let q = vec![0.9, 0.1];
        let kl = unsafe { hotpath_kl_divergence(p.as_ptr(), q.as_ptr(), 2) };
        assert!(kl > 0.0); // KL divergence is always non-negative
    }

    #[test]
    fn test_gelu_batch() {
        let mut values = vec![0.0, 1.0, -1.0];
        unsafe { hotpath_gelu_batch(values.as_mut_ptr(), values.len()) };
        assert!(values[0].abs() < 1e-10); // GELU(0) ≈ 0
        assert!(values[1] > 0.8); // GELU(1) ≈ 0.841
        assert!(values[2] < 0.0 && values[2] > -0.2); // GELU(-1) ≈ -0.159
    }

    #[test]
    fn test_clip() {
        let mut values = vec![-5.0, -1.0, 0.5, 1.5, 10.0];
        unsafe { hotpath_clip(values.as_mut_ptr(), values.len(), -1.0, 2.0) };
        assert!((values[0] - (-1.0)).abs() < 1e-10);
        assert!((values[1] - (-1.0)).abs() < 1e-10);
        assert!((values[2] - 0.5).abs() < 1e-10);
        assert!((values[3] - 1.5).abs() < 1e-10);
        assert!((values[4] - 2.0).abs() < 1e-10);
    }

    // ── Phase 25 Tests ──────────────────────────────────────────────

    #[test]
    fn test_layer_norm_basic() {
        let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mean = unsafe { hotpath_layer_norm(values.as_mut_ptr(), values.len(), 1.0, 0.0, 1e-5) };
        assert!((mean - 3.0).abs() < 1e-10);
        // After norm: mean should be ~0
        let result_mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        assert!(result_mean.abs() < 1e-10);
    }

    #[test]
    fn test_layer_norm_scale_shift() {
        let mut values = vec![1.0, 3.0];
        let _mean = unsafe { hotpath_layer_norm(values.as_mut_ptr(), values.len(), 2.0, 5.0, 1e-5) };
        // gamma=2, beta=5: centered, scaled by 2, shifted by 5
        assert!((values[0] - 3.0).abs() < 0.1);
        assert!((values[1] - 7.0).abs() < 0.1);
    }

    #[test]
    fn test_dropout_mask() {
        let mut values = vec![1.0; 1000];
        unsafe { hotpath_dropout_mask(values.as_mut_ptr(), values.len(), 0.5, 42) };
        let zeros = values.iter().filter(|&&x| x == 0.0).count();
        // Roughly 50% should be zeroed (with some variance)
        assert!(zeros > 200 && zeros < 800);
    }

    #[test]
    fn test_dropout_mask_keep_all() {
        let mut values = vec![2.0, 3.0, 4.0];
        unsafe { hotpath_dropout_mask(values.as_mut_ptr(), values.len(), 1.0, 42) };
        // keep_prob >= 1.0 means no dropout
        assert!((values[0] - 2.0).abs() < 1e-10);
        assert!((values[1] - 3.0).abs() < 1e-10);
        assert!((values[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_distance_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let d = unsafe { hotpath_cosine_distance(a.as_ptr(), b.as_ptr(), 3) };
        assert!(d.abs() < 1e-10); // identical vectors → distance 0
    }

    #[test]
    fn test_cosine_distance_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let d = unsafe { hotpath_cosine_distance(a.as_ptr(), b.as_ptr(), 2) };
        assert!((d - 1.0).abs() < 1e-10); // orthogonal → distance 1
    }

    #[test]
    fn test_cosine_distance_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let d = unsafe { hotpath_cosine_distance(a.as_ptr(), b.as_ptr(), 2) };
        assert!((d - 2.0).abs() < 1e-10); // opposite → distance 2
    }

    #[test]
    fn test_huber_loss_small_diff() {
        // When |diff| < delta, huber = 0.5 * diff^2
        let t = vec![1.0, 2.0, 3.0];
        let p = vec![1.1, 2.1, 3.1];
        let loss = unsafe { hotpath_huber_loss(t.as_ptr(), p.as_ptr(), 3, 1.0) };
        // Each element: 0.5 * 0.01 = 0.005, mean = 0.005
        assert!((loss - 0.005).abs() < 1e-10);
    }

    #[test]
    fn test_huber_loss_large_diff() {
        // When |diff| > delta, huber = delta * (|diff| - 0.5*delta)
        let t = vec![0.0];
        let p = vec![5.0];
        let loss = unsafe { hotpath_huber_loss(t.as_ptr(), p.as_ptr(), 1, 1.0) };
        // delta * (5 - 0.5) = 4.5
        assert!((loss - 4.5).abs() < 1e-10);
    }

    #[test]
    fn test_huber_loss_zero() {
        let t = vec![1.0, 2.0];
        let p = vec![1.0, 2.0];
        let loss = unsafe { hotpath_huber_loss(t.as_ptr(), p.as_ptr(), 2, 1.0) };
        assert!(loss.abs() < 1e-10);
    }

    #[test]
    fn test_mse_loss_basic() {
        let t = vec![1.0, 2.0, 3.0];
        let p = vec![1.5, 2.5, 3.5];
        let loss = unsafe { hotpath_mse_loss(t.as_ptr(), p.as_ptr(), 3) };
        // Each element: 0.25, mean = 0.25
        assert!((loss - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_mse_loss_zero() {
        let t = vec![1.0, 2.0, 3.0];
        let p = vec![1.0, 2.0, 3.0];
        let loss = unsafe { hotpath_mse_loss(t.as_ptr(), p.as_ptr(), 3) };
        assert!(loss.abs() < 1e-10);
    }
}
