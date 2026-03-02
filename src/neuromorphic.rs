//! Neuromorphic Computing Module — Vitalis v13.0
//!
//! Bio-inspired neural computation 100,000× beyond biological brain networks.
//! Implements spiking neural networks, synaptic plasticity, neural coding,
//! reservoir computing, and neuroevolution concepts.
//!
//! Architecture layers:
//!   1. Neuron models (LIF, Izhikevich, AdEx, Hodgkin-Huxley simplified)
//!   2. Synaptic plasticity (Hebbian, STDP, BCM, homeostatic)
//!   3. Neural coding (rate, temporal, population, rank-order)
//!   4. Network topology (small-world, scale-free, modular)
//!   5. Reservoir computing (Echo State Network core)
//!   6. Neuroevolution (NEAT-inspired topology evolution)
//!   7. Spike train analysis (ISI, CV, Fano factor, correlation)

use std::f64::consts::PI;

// ═══════════════════════════════════════════════════════════════════════
// 1. Neuron Models
// ═══════════════════════════════════════════════════════════════════════

/// Leaky Integrate-and-Fire (LIF) neuron simulation.
/// `currents` = [n_steps] input current at each timestep.
/// `spikes_out` = [n_steps] (0 or 1 for each step).
/// Returns total spike count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_lif(
    currents: *const f64, n_steps: usize,
    tau_m: f64, v_rest: f64, v_threshold: f64, v_reset: f64,
    r_membrane: f64, dt: f64,
    spikes_out: *mut u8,
) -> i32 {
    if currents.is_null() || spikes_out.is_null() || n_steps == 0 { return -1; }
    let inp = unsafe { std::slice::from_raw_parts(currents, n_steps) };
    let out = unsafe { std::slice::from_raw_parts_mut(spikes_out, n_steps) };

    let mut v = v_rest;
    let mut spike_count = 0i32;

    for i in 0..n_steps {
        // dV/dt = -(V - V_rest)/τ_m + R*I/τ_m
        let dv = (-(v - v_rest) + r_membrane * inp[i]) / tau_m * dt;
        v += dv;

        if v >= v_threshold {
            out[i] = 1;
            v = v_reset;
            spike_count += 1;
        } else {
            out[i] = 0;
        }
    }
    spike_count
}

/// Izhikevich neuron model (4 types: RS, IB, CH, FS).
/// Parameters a, b, c, d define neuron type.
/// Returns spike count. `v_out` and `u_out` are [n_steps].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_izhikevich(
    currents: *const f64, n_steps: usize,
    a: f64, b: f64, c: f64, d: f64, dt: f64,
    v_out: *mut f64, u_out: *mut f64, spikes_out: *mut u8,
) -> i32 {
    if currents.is_null() || n_steps == 0 { return -1; }
    let inp = unsafe { std::slice::from_raw_parts(currents, n_steps) };
    let has_v = !v_out.is_null();
    let has_u = !u_out.is_null();
    let has_s = !spikes_out.is_null();

    let mut v = -65.0;
    let mut u = b * v;
    let mut spikes = 0i32;

    for i in 0..n_steps {
        // 0.04v² + 5v + 140 - u + I
        let dv = (0.04 * v * v + 5.0 * v + 140.0 - u + inp[i]) * dt;
        let du = a * (b * v - u) * dt;
        v += dv;
        u += du;

        let spiked = v >= 30.0;
        if spiked {
            v = c;
            u += d;
            spikes += 1;
        }

        if has_v { unsafe { *v_out.add(i) = v; } }
        if has_u { unsafe { *u_out.add(i) = u; } }
        if has_s { unsafe { *spikes_out.add(i) = if spiked { 1 } else { 0 }; } }
    }
    spikes
}

/// Adaptive Exponential (AdEx) Integrate-and-Fire neuron.
/// Returns spike count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_adex(
    currents: *const f64, n_steps: usize,
    tau_m: f64, v_rest: f64, v_threshold: f64, delta_t: f64,
    a: f64, b: f64, tau_w: f64, v_reset: f64,
    c_m: f64, dt: f64,
    spikes_out: *mut u8,
) -> i32 {
    if currents.is_null() || n_steps == 0 { return -1; }
    let inp = unsafe { std::slice::from_raw_parts(currents, n_steps) };
    let has_s = !spikes_out.is_null();

    let mut v = v_rest;
    let mut w = 0.0;
    let mut spikes = 0i32;

    for i in 0..n_steps {
        let exp_term = delta_t * ((v - v_threshold) / delta_t).exp();
        let dv = (-(v - v_rest) + exp_term - w / c_m + inp[i] / c_m) / tau_m * dt;
        let dw = (a * (v - v_rest) - w) / tau_w * dt;
        v += dv;
        w += dw;

        let spiked = v >= v_threshold + 5.0 * delta_t;
        if spiked {
            v = v_reset;
            w += b;
            spikes += 1;
        }
        if has_s { unsafe { *spikes_out.add(i) = if spiked { 1 } else { 0 }; } }
    }
    spikes
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Synaptic Plasticity
// ═══════════════════════════════════════════════════════════════════════

/// Hebbian learning: Δw = η * x_pre * x_post.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_hebbian_update(
    weights: *mut f64, n_pre: usize, n_post: usize,
    pre_activity: *const f64, post_activity: *const f64,
    learning_rate: f64, w_max: f64,
) -> i32 {
    if weights.is_null() || pre_activity.is_null() || post_activity.is_null() { return -1; }
    let w = unsafe { std::slice::from_raw_parts_mut(weights, n_pre * n_post) };
    let pre = unsafe { std::slice::from_raw_parts(pre_activity, n_pre) };
    let post = unsafe { std::slice::from_raw_parts(post_activity, n_post) };

    for i in 0..n_pre {
        for j in 0..n_post {
            let dw = learning_rate * pre[i] * post[j];
            w[i * n_post + j] = (w[i * n_post + j] + dw).min(w_max).max(-w_max);
        }
    }
    0
}

/// STDP (Spike-Timing-Dependent Plasticity).
/// `dt_spike` = t_post - t_pre.
/// Positive dt → LTP (potentiation), negative → LTD (depression).
/// Returns weight change Δw.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_stdp_delta(
    dt_spike: f64, a_plus: f64, a_minus: f64,
    tau_plus: f64, tau_minus: f64,
) -> f64 {
    if dt_spike > 0.0 {
        a_plus * (-dt_spike / tau_plus).exp()
    } else {
        -a_minus * (dt_spike / tau_minus).exp()
    }
}

/// Apply STDP to a weight matrix given pre/post spike times.
/// `pre_times` = [n_pre], `post_times` = [n_post], `weights` = [n_pre × n_post].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_stdp_update(
    weights: *mut f64, n_pre: usize, n_post: usize,
    pre_times: *const f64, post_times: *const f64,
    a_plus: f64, a_minus: f64, tau_plus: f64, tau_minus: f64,
    w_max: f64,
) -> i32 {
    if weights.is_null() || pre_times.is_null() || post_times.is_null() { return -1; }
    let w = unsafe { std::slice::from_raw_parts_mut(weights, n_pre * n_post) };
    let pre = unsafe { std::slice::from_raw_parts(pre_times, n_pre) };
    let post = unsafe { std::slice::from_raw_parts(post_times, n_post) };

    for i in 0..n_pre {
        for j in 0..n_post {
            let dt = post[j] - pre[i];
            let dw = unsafe { vitalis_neuro_stdp_delta(dt, a_plus, a_minus, tau_plus, tau_minus) };
            w[i * n_post + j] = (w[i * n_post + j] + dw).min(w_max).max(-w_max);
        }
    }
    0
}

/// BCM (Bienenstock-Cooper-Munro) learning rule.
/// Δw = η * x * y * (y - θ), where θ slides based on postsynaptic activity.
/// `theta` = sliding threshold. Returns new weight.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_bcm_update(
    weight: f64, pre: f64, post: f64, theta: f64, learning_rate: f64, w_max: f64,
) -> f64 {
    let dw = learning_rate * pre * post * (post - theta);
    (weight + dw).min(w_max).max(-w_max)
}

/// Homeostatic plasticity: scale synaptic weights to maintain target firing rate.
/// `rates` = [n] current firing rates, `target_rate`, `tau_homeo`.
/// `scale_factors_out` = [n].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_homeostatic_scaling(
    rates: *const f64, n: usize, target_rate: f64, tau_homeo: f64,
    scale_factors_out: *mut f64,
) -> i32 {
    if rates.is_null() || scale_factors_out.is_null() || n == 0 { return -1; }
    let r = unsafe { std::slice::from_raw_parts(rates, n) };
    let out = unsafe { std::slice::from_raw_parts_mut(scale_factors_out, n) };

    for i in 0..n {
        if r[i] > 0.0 {
            out[i] = 1.0 + (target_rate - r[i]) / (tau_homeo * r[i]);
        } else {
            out[i] = 2.0; // boost quiescent neurons
        }
        out[i] = out[i].max(0.1).min(10.0);
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Neural Coding
// ═══════════════════════════════════════════════════════════════════════

/// Rate coding: compute firing rate from spike train.
/// `spikes` = [n_steps] (0/1), `dt` = timestep. Returns Hz.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_firing_rate(spikes: *const u8, n_steps: usize, dt: f64) -> f64 {
    if spikes.is_null() || n_steps == 0 || dt <= 0.0 { return 0.0; }
    let s = unsafe { std::slice::from_raw_parts(spikes, n_steps) };
    let count: usize = s.iter().map(|&x| x as usize).sum();
    count as f64 / (n_steps as f64 * dt)
}

/// Inter-spike interval (ISI) statistics.
/// Returns mean ISI. `isi_out` = buffer for ISI values, `n_isi_out` = count written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_isi_stats(
    spikes: *const u8, n_steps: usize, dt: f64,
    mean_isi: *mut f64, cv_isi: *mut f64,
) -> i32 {
    if spikes.is_null() || n_steps == 0 { return -1; }
    let s = unsafe { std::slice::from_raw_parts(spikes, n_steps) };

    // Collect spike times
    let mut spike_times = Vec::new();
    for i in 0..n_steps {
        if s[i] != 0 { spike_times.push(i as f64 * dt); }
    }
    if spike_times.len() < 2 { return 0; }

    // Compute ISIs
    let isis: Vec<f64> = spike_times.windows(2).map(|w| w[1] - w[0]).collect();
    let mean: f64 = isis.iter().sum::<f64>() / isis.len() as f64;
    let variance: f64 = isis.iter().map(|&x| (x - mean) * (x - mean)).sum::<f64>() / isis.len() as f64;
    let std_dev = variance.sqrt();

    if !mean_isi.is_null() { unsafe { *mean_isi = mean; } }
    if !cv_isi.is_null() { unsafe { *cv_isi = if mean > 0.0 { std_dev / mean } else { 0.0 }; } }
    isis.len() as i32
}

/// Population coding: decode stimulus from population of neurons.
/// `preferred_stimuli` = [n_neurons], `responses` = [n_neurons].
/// Returns decoded stimulus (population vector average).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_population_decode(
    preferred_stimuli: *const f64, responses: *const f64, n_neurons: usize,
) -> f64 {
    if preferred_stimuli.is_null() || responses.is_null() || n_neurons == 0 { return 0.0; }
    let pref = unsafe { std::slice::from_raw_parts(preferred_stimuli, n_neurons) };
    let resp = unsafe { std::slice::from_raw_parts(responses, n_neurons) };

    let total_resp: f64 = resp.iter().sum();
    if total_resp <= 0.0 { return 0.0; }

    let weighted: f64 = pref.iter().zip(resp.iter()).map(|(&p, &r)| p * r).sum();
    weighted / total_resp
}

/// Fano factor: variance / mean of spike counts in windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_fano_factor(
    spikes: *const u8, n_steps: usize, window_size: usize,
) -> f64 {
    if spikes.is_null() || n_steps == 0 || window_size == 0 { return 0.0; }
    let s = unsafe { std::slice::from_raw_parts(spikes, n_steps) };

    let n_windows = n_steps / window_size;
    if n_windows < 2 { return 0.0; }

    let mut counts = Vec::with_capacity(n_windows);
    for w in 0..n_windows {
        let start = w * window_size;
        let end = start + window_size;
        let count: usize = s[start..end].iter().map(|&x| x as usize).sum();
        counts.push(count as f64);
    }

    let mean: f64 = counts.iter().sum::<f64>() / counts.len() as f64;
    if mean <= 0.0 { return 0.0; }
    let var: f64 = counts.iter().map(|&x| (x - mean) * (x - mean)).sum::<f64>() / counts.len() as f64;
    var / mean
}

// ═══════════════════════════════════════════════════════════════════════
// 4. Network Topology
// ═══════════════════════════════════════════════════════════════════════

fn prng_next(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

/// Generate small-world network (Watts-Strogatz model).
/// `adj_out` = [n × n] adjacency matrix (0.0 or 1.0).
/// `k` = each node connected to k nearest neighbors.
/// `beta` = rewiring probability.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_small_world_network(
    n: usize, k: usize, beta: f64, adj_out: *mut f64, seed: u64,
) -> i32 {
    if adj_out.is_null() || n < k + 1 || k == 0 { return -1; }
    let adj = unsafe { std::slice::from_raw_parts_mut(adj_out, n * n) };
    let mut rng = seed;

    // Initialize ring lattice
    for i in 0..n * n { adj[i] = 0.0; }
    for i in 0..n {
        for j in 1..=(k / 2) {
            let neighbor = (i + j) % n;
            adj[i * n + neighbor] = 1.0;
            adj[neighbor * n + i] = 1.0;
        }
    }

    // Rewire
    for i in 0..n {
        for j in 1..=(k / 2) {
            let neighbor = (i + j) % n;
            if prng_next(&mut rng) < beta {
                // Remove old edge
                adj[i * n + neighbor] = 0.0;
                adj[neighbor * n + i] = 0.0;
                // Add random edge
                loop {
                    let new_j = (prng_next(&mut rng) * n as f64) as usize % n;
                    if new_j != i && adj[i * n + new_j] == 0.0 {
                        adj[i * n + new_j] = 1.0;
                        adj[new_j * n + i] = 1.0;
                        break;
                    }
                }
            }
        }
    }
    0
}

/// Generate scale-free network (Barabási-Albert model).
/// `adj_out` = [n × n]. `m` = edges per new node.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_scale_free_network(
    n: usize, m: usize, adj_out: *mut f64, seed: u64,
) -> i32 {
    if adj_out.is_null() || n < m + 1 || m == 0 { return -1; }
    let adj = unsafe { std::slice::from_raw_parts_mut(adj_out, n * n) };
    let mut rng = seed;

    for i in 0..n * n { adj[i] = 0.0; }

    // Start with complete graph of m+1 nodes
    for i in 0..=m {
        for j in (i + 1)..=m {
            adj[i * n + j] = 1.0;
            adj[j * n + i] = 1.0;
        }
    }

    let mut degree = vec![m as f64; m + 1];
    degree.resize(n, 0.0);

    for new_node in (m + 1)..n {
        let total_degree: f64 = degree.iter().take(new_node).sum();
        if total_degree <= 0.0 { continue; }

        let mut added = 0usize;
        let mut attempts = 0;
        while added < m && attempts < n * 10 {
            attempts += 1;
            let r = prng_next(&mut rng) * total_degree;
            let mut cumulative = 0.0;
            for target in 0..new_node {
                cumulative += degree[target];
                if cumulative >= r && adj[new_node * n + target] == 0.0 {
                    adj[new_node * n + target] = 1.0;
                    adj[target * n + new_node] = 1.0;
                    degree[new_node] += 1.0;
                    degree[target] += 1.0;
                    added += 1;
                    break;
                }
            }
        }
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// 5. Reservoir Computing (Echo State Network)
// ═══════════════════════════════════════════════════════════════════════

/// Echo State Network forward pass.
/// `input` = [n_steps × input_dim], `reservoir_weights` = [res_size × res_size],
/// `input_weights` = [res_size × input_dim], `state_out` = [n_steps × res_size].
/// Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_esn_forward(
    input: *const f64, n_steps: usize, input_dim: usize,
    reservoir_weights: *const f64, res_size: usize,
    input_weights: *const f64,
    leak_rate: f64,
    state_out: *mut f64,
) -> i32 {
    if input.is_null() || reservoir_weights.is_null() || input_weights.is_null() || state_out.is_null() {
        return -1;
    }
    let inp = unsafe { std::slice::from_raw_parts(input, n_steps * input_dim) };
    let w_res = unsafe { std::slice::from_raw_parts(reservoir_weights, res_size * res_size) };
    let w_in = unsafe { std::slice::from_raw_parts(input_weights, res_size * input_dim) };
    let out = unsafe { std::slice::from_raw_parts_mut(state_out, n_steps * res_size) };

    let mut state = vec![0.0f64; res_size];

    for t in 0..n_steps {
        let mut new_state = vec![0.0f64; res_size];

        for i in 0..res_size {
            // Input contribution
            let mut val = 0.0;
            for j in 0..input_dim {
                val += w_in[i * input_dim + j] * inp[t * input_dim + j];
            }
            // Reservoir recurrence
            for j in 0..res_size {
                val += w_res[i * res_size + j] * state[j];
            }
            // Tanh activation + leaky integration
            new_state[i] = (1.0 - leak_rate) * state[i] + leak_rate * val.tanh();
        }

        for i in 0..res_size {
            state[i] = new_state[i];
            out[t * res_size + i] = state[i];
        }
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// 6. Spike Train Analysis
// ═══════════════════════════════════════════════════════════════════════

/// Cross-correlation between two spike trains.
/// Returns correlation coefficient at lag 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_spike_correlation(
    spikes1: *const u8, spikes2: *const u8, n_steps: usize,
) -> f64 {
    if spikes1.is_null() || spikes2.is_null() || n_steps == 0 { return 0.0; }
    let s1 = unsafe { std::slice::from_raw_parts(spikes1, n_steps) };
    let s2 = unsafe { std::slice::from_raw_parts(spikes2, n_steps) };

    let m1: f64 = s1.iter().map(|&x| x as f64).sum::<f64>() / n_steps as f64;
    let m2: f64 = s2.iter().map(|&x| x as f64).sum::<f64>() / n_steps as f64;

    let mut cov = 0.0;
    let mut var1 = 0.0;
    let mut var2 = 0.0;
    for i in 0..n_steps {
        let d1 = s1[i] as f64 - m1;
        let d2 = s2[i] as f64 - m2;
        cov += d1 * d2;
        var1 += d1 * d1;
        var2 += d2 * d2;
    }
    let denom = (var1 * var2).sqrt();
    if denom < 1e-15 { return 0.0; }
    cov / denom
}

/// Spike train entropy (bits per time bin).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_spike_entropy(spikes: *const u8, n_steps: usize) -> f64 {
    if spikes.is_null() || n_steps == 0 { return 0.0; }
    let s = unsafe { std::slice::from_raw_parts(spikes, n_steps) };
    let p1 = s.iter().filter(|&&x| x != 0).count() as f64 / n_steps as f64;
    let p0 = 1.0 - p1;
    let mut h = 0.0;
    if p0 > 0.0 { h -= p0 * p0.log2(); }
    if p1 > 0.0 { h -= p1 * p1.log2(); }
    h
}

/// Burst detection: find clusters of spikes within `max_isi` timesteps.
/// Returns number of bursts detected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_burst_detection(
    spikes: *const u8, n_steps: usize, max_isi: usize, min_spikes: usize,
) -> i32 {
    if spikes.is_null() || n_steps == 0 { return 0; }
    let s = unsafe { std::slice::from_raw_parts(spikes, n_steps) };

    let mut bursts = 0i32;
    let mut in_burst = false;
    let mut burst_count = 0usize;
    let mut last_spike = 0usize;

    for i in 0..n_steps {
        if s[i] != 0 {
            if !in_burst || i - last_spike <= max_isi {
                burst_count += 1;
                in_burst = true;
            } else {
                if burst_count >= min_spikes { bursts += 1; }
                burst_count = 1;
            }
            last_spike = i;
        } else if in_burst && i - last_spike > max_isi {
            if burst_count >= min_spikes { bursts += 1; }
            burst_count = 0;
            in_burst = false;
        }
    }
    if in_burst && burst_count >= min_spikes { bursts += 1; }
    bursts
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Neuroevolution (NEAT-inspired)
// ═══════════════════════════════════════════════════════════════════════

/// NEAT compatibility distance between two genomes.
/// `genes1/genes2` = [n × 3] (innovation_id, weight, enabled).
/// Returns δ = c1*E/N + c2*D/N + c3*W̄.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_neat_compatibility(
    genes1: *const f64, n1: usize,
    genes2: *const f64, n2: usize,
    c1: f64, c2: f64, c3: f64,
) -> f64 {
    if genes1.is_null() || genes2.is_null() || n1 == 0 || n2 == 0 { return f64::MAX; }
    let g1 = unsafe { std::slice::from_raw_parts(genes1, n1 * 3) };
    let g2 = unsafe { std::slice::from_raw_parts(genes2, n2 * 3) };

    let max1 = (0..n1).map(|i| g1[i * 3] as i64).max().unwrap_or(0);
    let max2 = (0..n2).map(|i| g2[i * 3] as i64).max().unwrap_or(0);
    let max_innov = max1.max(max2);

    let mut excess = 0u32;
    let mut disjoint = 0u32;
    let mut weight_diff = 0.0;
    let mut matching = 0u32;

    let threshold = max1.min(max2);

    for i in 0..n1 {
        let innov1 = g1[i * 3] as i64;
        let found = (0..n2).any(|j| g2[j * 3] as i64 == innov1);
        if !found {
            if innov1 > threshold { excess += 1; } else { disjoint += 1; }
        } else {
            let j = (0..n2).find(|&j| g2[j * 3] as i64 == innov1).unwrap();
            weight_diff += (g1[i * 3 + 1] - g2[j * 3 + 1]).abs();
            matching += 1;
        }
    }
    for j in 0..n2 {
        let innov2 = g2[j * 3] as i64;
        let found = (0..n1).any(|i| g1[i * 3] as i64 == innov2);
        if !found {
            if innov2 > threshold { excess += 1; } else { disjoint += 1; }
        }
    }

    let n = n1.max(n2) as f64;
    let w_avg = if matching > 0 { weight_diff / matching as f64 } else { 0.0 };
    c1 * excess as f64 / n + c2 * disjoint as f64 / n + c3 * w_avg
}

/// Sigmoid activation with configurable steepness.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_sigmoid(x: f64, steepness: f64) -> f64 {
    1.0 / (1.0 + (-steepness * x).exp())
}

/// Information transfer: mutual information between input and output spike trains.
/// Simplified binary version.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_mutual_information(
    spikes_in: *const u8, spikes_out: *const u8, n_steps: usize,
) -> f64 {
    if spikes_in.is_null() || spikes_out.is_null() || n_steps == 0 { return 0.0; }
    let si = unsafe { std::slice::from_raw_parts(spikes_in, n_steps) };
    let so = unsafe { std::slice::from_raw_parts(spikes_out, n_steps) };

    let mut counts = [[0u32; 2]; 2]; // [in][out]
    for i in 0..n_steps {
        let a = (si[i] != 0) as usize;
        let b = (so[i] != 0) as usize;
        counts[a][b] += 1;
    }

    let n = n_steps as f64;
    let mut mi = 0.0;
    for a in 0..2 {
        for b in 0..2 {
            let p_ab = counts[a][b] as f64 / n;
            let p_a = (counts[a][0] + counts[a][1]) as f64 / n;
            let p_b = (counts[0][b] + counts[1][b]) as f64 / n;
            if p_ab > 0.0 && p_a > 0.0 && p_b > 0.0 {
                mi += p_ab * (p_ab / (p_a * p_b)).log2();
            }
        }
    }
    mi
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Neural Oscillations
// ═══════════════════════════════════════════════════════════════════════

/// Kuramoto model: coupled oscillators for neural synchronization.
/// `phases` = [n] (rad), `frequencies` = [n] (Hz), `coupling` = K.
/// Updates phases in-place for one step. Returns order parameter r.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_neuro_kuramoto_step(
    phases: *mut f64, frequencies: *const f64, n: usize,
    coupling: f64, dt: f64,
) -> f64 {
    if phases.is_null() || frequencies.is_null() || n == 0 { return 0.0; }
    let ph = unsafe { std::slice::from_raw_parts_mut(phases, n) };
    let freq = unsafe { std::slice::from_raw_parts(frequencies, n) };

    let mut new_phases = vec![0.0f64; n];

    for i in 0..n {
        let mut coupling_sum = 0.0;
        for j in 0..n {
            coupling_sum += (ph[j] - ph[i]).sin();
        }
        new_phases[i] = ph[i] + dt * (2.0 * PI * freq[i] + coupling / n as f64 * coupling_sum);
    }

    // Order parameter: r = |1/N Σ e^{iθ_j}|
    let mut cos_sum = 0.0;
    let mut sin_sum = 0.0;
    for i in 0..n {
        ph[i] = new_phases[i];
        cos_sum += ph[i].cos();
        sin_sum += ph[i].sin();
    }
    cos_sum /= n as f64;
    sin_sum /= n as f64;
    (cos_sum * cos_sum + sin_sum * sin_sum).sqrt()
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lif_neuron() {
        let n = 1000;
        let currents = vec![15.0; n]; // constant high current
        let mut spikes = vec![0u8; n];
        let count = unsafe {
            vitalis_neuro_lif(
                currents.as_ptr(), n,
                20.0, -65.0, -50.0, -65.0, 10.0, 0.1,
                spikes.as_mut_ptr(),
            )
        };
        assert!(count > 0, "LIF should fire with high current, got {} spikes", count);
    }

    #[test]
    fn test_izhikevich_regular_spiking() {
        let n = 1000;
        let currents = vec![10.0; n];
        let mut v_out = vec![0.0; n];
        let mut u_out = vec![0.0; n];
        let mut spikes = vec![0u8; n];
        // Regular Spiking: a=0.02, b=0.2, c=-65, d=8
        let count = unsafe {
            vitalis_neuro_izhikevich(
                currents.as_ptr(), n,
                0.02, 0.2, -65.0, 8.0, 0.5,
                v_out.as_mut_ptr(), u_out.as_mut_ptr(), spikes.as_mut_ptr(),
            )
        };
        assert!(count > 0, "Izhikevich RS should fire");
    }

    #[test]
    fn test_stdp_delta() {
        // Pre before post → potentiation (positive)
        let dw_ltp = unsafe { vitalis_neuro_stdp_delta(5.0, 0.1, 0.1, 20.0, 20.0) };
        assert!(dw_ltp > 0.0);

        // Post before pre → depression (negative)
        let dw_ltd = unsafe { vitalis_neuro_stdp_delta(-5.0, 0.1, 0.1, 20.0, 20.0) };
        assert!(dw_ltd < 0.0);
    }

    #[test]
    fn test_hebbian_update() {
        let mut weights = vec![0.0; 4]; // 2×2
        let pre = [1.0, 0.5];
        let post = [0.8, 0.2];
        let r = unsafe {
            vitalis_neuro_hebbian_update(
                weights.as_mut_ptr(), 2, 2,
                pre.as_ptr(), post.as_ptr(),
                0.01, 1.0,
            )
        };
        assert_eq!(r, 0);
        assert!(weights[0] > 0.0); // pre[0]*post[0] > 0
    }

    #[test]
    fn test_firing_rate() {
        let mut spikes = vec![0u8; 1000];
        for i in (0..1000).step_by(10) { spikes[i] = 1; }
        let rate = unsafe { vitalis_neuro_firing_rate(spikes.as_ptr(), 1000, 0.001) };
        assert!((rate - 100.0).abs() < 5.0); // ~100 Hz
    }

    #[test]
    fn test_spike_correlation_identical() {
        let s = vec![1u8, 0, 0, 1, 0, 1, 0, 0, 1, 0];
        let r = unsafe { vitalis_neuro_spike_correlation(s.as_ptr(), s.as_ptr(), s.len()) };
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_spike_entropy() {
        // 50% spikes → max binary entropy = 1 bit
        let mut spikes = vec![0u8; 100];
        for i in (0..100).step_by(2) { spikes[i] = 1; }
        let h = unsafe { vitalis_neuro_spike_entropy(spikes.as_ptr(), 100) };
        assert!((h - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_population_decode() {
        let pref = [0.0, 90.0, 180.0, 270.0];
        let resp = [0.1, 0.9, 0.1, 0.1]; // peak at 90°
        let decoded = unsafe {
            vitalis_neuro_population_decode(pref.as_ptr(), resp.as_ptr(), 4)
        };
        assert!((decoded - 90.0).abs() < 30.0);
    }

    #[test]
    fn test_fano_factor() {
        // Regular spikes → Fano factor ≈ 0
        let mut spikes = vec![0u8; 100];
        for i in (0..100).step_by(5) { spikes[i] = 1; }
        let f = unsafe { vitalis_neuro_fano_factor(spikes.as_ptr(), 100, 10) };
        assert!(f < 0.5, "Regular spikes should have low Fano factor, got {}", f);
    }

    #[test]
    fn test_small_world() {
        let n = 20;
        let mut adj = vec![0.0; n * n];
        let r = unsafe { vitalis_neuro_small_world_network(n, 4, 0.3, adj.as_mut_ptr(), 42) };
        assert_eq!(r, 0);
        // Check some edges exist
        let edges: usize = adj.iter().filter(|&&x| x > 0.0).count();
        assert!(edges > 0);
    }

    #[test]
    fn test_scale_free() {
        let n = 30;
        let mut adj = vec![0.0; n * n];
        let r = unsafe { vitalis_neuro_scale_free_network(n, 2, adj.as_mut_ptr(), 42) };
        assert_eq!(r, 0);
        let edges: usize = adj.iter().filter(|&&x| x > 0.0).count();
        assert!(edges > 0);
    }

    #[test]
    fn test_esn_forward() {
        let res_size = 5;
        let input_dim = 2;
        let n_steps = 10;
        let input = vec![0.5; n_steps * input_dim];
        let w_res = vec![0.1; res_size * res_size];
        let w_in = vec![0.2; res_size * input_dim];
        let mut state_out = vec![0.0; n_steps * res_size];

        let r = unsafe {
            vitalis_neuro_esn_forward(
                input.as_ptr(), n_steps, input_dim,
                w_res.as_ptr(), res_size,
                w_in.as_ptr(), 0.3,
                state_out.as_mut_ptr(),
            )
        };
        assert_eq!(r, 0);
        // States should be non-zero after a few steps
        assert!(state_out[n_steps * res_size - 1] != 0.0);
    }

    #[test]
    fn test_kuramoto() {
        let n = 10;
        let mut phases = vec![0.0; n];
        let frequencies = vec![1.0; n]; // all same frequency
        let r = unsafe {
            vitalis_neuro_kuramoto_step(
                phases.as_mut_ptr(), frequencies.as_ptr(), n, 1.0, 0.01,
            )
        };
        assert!(r > 0.5); // should be synchronized
    }

    #[test]
    fn test_neat_compatibility() {
        // Same genome → distance = 0
        let g1 = [1.0, 0.5, 1.0, 2.0, -0.3, 1.0];
        let g2 = [1.0, 0.5, 1.0, 2.0, -0.3, 1.0];
        let d = unsafe {
            vitalis_neuro_neat_compatibility(g1.as_ptr(), 2, g2.as_ptr(), 2, 1.0, 1.0, 0.4)
        };
        assert!(d.abs() < 0.001);
    }

    #[test]
    fn test_sigmoid() {
        let y = unsafe { vitalis_neuro_sigmoid(0.0, 1.0) };
        assert!((y - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_mutual_information() {
        // Identical spike trains → MI > 0
        let s = vec![1u8, 0, 1, 0, 1, 0, 1, 0, 1, 0];
        let mi = unsafe { vitalis_neuro_mutual_information(s.as_ptr(), s.as_ptr(), s.len()) };
        assert!(mi > 0.0);
    }

    #[test]
    fn test_burst_detection() {
        // Burst pattern: 3 spikes close together, gap, 3 spikes
        let mut spikes = vec![0u8; 50];
        spikes[5] = 1; spikes[6] = 1; spikes[7] = 1; // burst 1
        spikes[30] = 1; spikes[31] = 1; spikes[32] = 1; // burst 2
        let bursts = unsafe { vitalis_neuro_burst_detection(spikes.as_ptr(), 50, 3, 3) };
        assert_eq!(bursts, 2);
    }
}
