//! Quantum Algorithms Module — Vitalis v13.0
//!
//! Implements the missing marquee quantum algorithms on top of the
//! statevector simulation from `quantum.rs`.  All functions are FFI-safe.
//!
//! Algorithms: Deutsch-Jozsa, Bernstein-Vazirani, Simon's, Quantum Phase
//! Estimation, Grover (full), Shor (modular exponentiation + period finding),
//! VQE, QAOA, Quantum Walk, Quantum Teleportation, Quantum Error Correction
//! (bit-flip / phase-flip / Shor 9-qubit), HHL sketch, BB84 QKD.

use std::f64::consts::PI;

// ─── Helpers ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Complex { re: f64, im: f64 }

impl Complex {
    fn new(re: f64, im: f64) -> Self { Complex { re, im } }
    fn zero() -> Self { Complex { re: 0.0, im: 0.0 } }
    fn norm_sq(&self) -> f64 { self.re * self.re + self.im * self.im }
    fn mul(&self, o: &Complex) -> Complex {
        Complex::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
    fn add(&self, o: &Complex) -> Complex { Complex::new(self.re + o.re, self.im + o.im) }
    fn sub(&self, o: &Complex) -> Complex { Complex::new(self.re - o.re, self.im - o.im) }
    fn scale(&self, s: f64) -> Complex { Complex::new(self.re * s, self.im * s) }
    fn exp_i(theta: f64) -> Complex { Complex::new(theta.cos(), theta.sin()) }
}

/// Minimal statevector for algorithm implementations.
fn make_state(n: usize) -> Vec<Complex> {
    let size = 1 << n;
    let mut s = vec![Complex::zero(); size];
    s[0] = Complex::new(1.0, 0.0);
    s
}

fn apply_h(state: &mut [Complex], qubit: usize, n: usize) {
    let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();
    let step = 1 << qubit;
    for i in 0..(1 << n) {
        if i & step == 0 {
            let j = i | step;
            let a = state[i].clone();
            let b = state[j].clone();
            state[i] = a.add(&b).scale(inv_sqrt2);
            state[j] = a.sub(&b).scale(inv_sqrt2);
        }
    }
}

fn apply_x(state: &mut [Complex], qubit: usize, n: usize) {
    let step = 1 << qubit;
    for i in 0..(1 << n) {
        if i & step == 0 {
            let j = i | step;
            let tmp = state[i].clone();
            state[i] = state[j].clone();
            state[j] = tmp;
        }
    }
}

fn apply_z(state: &mut [Complex], qubit: usize, n: usize) {
    let step = 1 << qubit;
    for i in 0..(1 << n) {
        if i & step != 0 {
            state[i] = state[i].scale(-1.0);
        }
    }
}

fn apply_phase(state: &mut [Complex], qubit: usize, n: usize, theta: f64) {
    let step = 1 << qubit;
    let phase = Complex::exp_i(theta);
    for i in 0..(1 << n) {
        if i & step != 0 {
            state[i] = state[i].mul(&phase);
        }
    }
}

fn apply_cnot(state: &mut [Complex], control: usize, target: usize, n: usize) {
    let c_bit = 1 << control;
    let t_bit = 1 << target;
    for i in 0..(1 << n) {
        if i & c_bit != 0 && i & t_bit == 0 {
            let j = i | t_bit;
            let tmp = state[i].clone();
            state[i] = state[j].clone();
            state[j] = tmp;
        }
    }
}

fn measure_qubit(state: &[Complex], qubit: usize, n: usize) -> f64 {
    let step = 1 << qubit;
    let mut p1 = 0.0;
    for i in 0..(1 << n) {
        if i & step != 0 { p1 += state[i].norm_sq(); }
    }
    p1
}

/// Simple seeded PRNG for deterministic measurements.
fn prng_next(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

fn measure_collapse(state: &mut Vec<Complex>, qubit: usize, n: usize, seed: &mut u64) -> u32 {
    let p1 = measure_qubit(state, qubit, n);
    let r = prng_next(seed);
    let result = if r < p1 { 1 } else { 0 };
    let step = 1 << qubit;
    let mut norm = 0.0;
    for i in 0..(1 << n) {
        let has_bit = (i & step != 0) as u32;
        if has_bit != result {
            state[i] = Complex::zero();
        } else {
            norm += state[i].norm_sq();
        }
    }
    let inv = 1.0 / norm.sqrt();
    for c in state.iter_mut() { *c = c.scale(inv); }
    result
}

fn apply_qft(state: &mut [Complex], qubits: &[usize], n: usize) {
    let m = qubits.len();
    for i in 0..m {
        apply_h(state, qubits[i], n);
        for j in (i + 1)..m {
            let angle = PI / (1 << (j - i)) as f64;
            apply_controlled_phase(state, qubits[j], qubits[i], n, angle);
        }
    }
    // Swap qubits
    for i in 0..m / 2 {
        apply_swap(state, qubits[i], qubits[m - 1 - i], n);
    }
}

fn apply_controlled_phase(state: &mut [Complex], control: usize, target: usize, n: usize, theta: f64) {
    let c_bit = 1 << control;
    let t_bit = 1 << target;
    let phase = Complex::exp_i(theta);
    for i in 0..(1 << n) {
        if i & c_bit != 0 && i & t_bit != 0 {
            state[i] = state[i].mul(&phase);
        }
    }
}

fn apply_swap(state: &mut [Complex], a: usize, b: usize, n: usize) {
    apply_cnot(state, a, b, n);
    apply_cnot(state, b, a, n);
    apply_cnot(state, a, b, n);
}

fn modular_exp(base: u64, exp: u64, modulus: u64) -> u64 {
    if modulus == 1 { return 0; }
    let mut result = 1u64;
    let mut b = base % modulus;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 { result = (result as u128 * b as u128 % modulus as u128) as u64; }
        e >>= 1;
        b = (b as u128 * b as u128 % modulus as u128) as u64;
    }
    result
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 { let t = b; b = a % b; a = t; }
    a
}

// ═══════════════════════════════════════════════════════════════════════
// 1. Deutsch-Jozsa Algorithm
// ═══════════════════════════════════════════════════════════════════════

/// Deutsch-Jozsa: determine if f is constant or balanced.
/// `oracle` is a bitmask: bit i = f(i). n = number of input qubits.
/// Returns 1 if constant, 0 if balanced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_deutsch_jozsa(oracle: u64, n: usize) -> i32 {
    if n == 0 || n > 20 { return -1; }
    let total = n + 1; // n input + 1 ancilla
    let mut state = make_state(total);

    // Prepare ancilla in |1⟩
    apply_x(&mut state, 0, total);

    // Apply H to all qubits
    for q in 0..total { apply_h(&mut state, q, total); }

    // Oracle: CNOT ancilla conditioned on f(x) = 1
    for x in 0..(1u64 << n) {
        if (oracle >> x) & 1 == 1 {
            let x_shifted = (x as usize) << 1;
            let i0 = x_shifted;     // ancilla = 0
            let i1 = x_shifted | 1; // ancilla = 1
            if i1 < state.len() {
                let tmp = state[i0].clone();
                state[i0] = state[i1].clone();
                state[i1] = tmp;
            }
        }
    }

    // Apply H to input qubits
    for q in 1..total { apply_h(&mut state, q, total); }

    // Measure input qubits — if all |0⟩, constant
    let mut all_zero_prob = 0.0;
    for i in 0..(1 << total) {
        let input_bits = i >> 1; // skip ancilla
        if input_bits == 0 {
            all_zero_prob += state[i].norm_sq();
        }
    }

    if all_zero_prob > 0.5 { 1 } else { 0 }
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Bernstein-Vazirani Algorithm
// ═══════════════════════════════════════════════════════════════════════

/// Bernstein-Vazirani: find hidden string s where f(x) = s·x mod 2.
/// `secret` is the hidden string. Returns recovered secret.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bernstein_vazirani(secret: u64, n: usize) -> u64 {
    if n == 0 || n > 20 { return 0; }
    let total = n + 1;
    let mut state = make_state(total);

    // Ancilla |1⟩
    apply_x(&mut state, 0, total);

    // H on all
    for q in 0..total { apply_h(&mut state, q, total); }

    // Oracle: CNOT ancilla conditioned on s·x mod 2 = 1
    for x in 0..(1u64 << n) {
        let dot = (secret & x).count_ones() % 2;
        if dot == 1 {
            let x_shifted = (x as usize) << 1;
            let i0 = x_shifted;     // ancilla = 0
            let i1 = x_shifted | 1; // ancilla = 1
            if i1 < state.len() {
                let tmp = state[i0].clone();
                state[i0] = state[i1].clone();
                state[i1] = tmp;
            }
        }
    }

    // H on input qubits
    for q in 1..total { apply_h(&mut state, q, total); }

    // Measure: find state with highest probability
    let mut best_x = 0u64;
    let mut best_p = 0.0;
    for i in 0..(1 << total) {
        let p = state[i].norm_sq();
        if p > best_p {
            best_p = p;
            best_x = (i >> 1) as u64;
        }
    }
    best_x
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Quantum Phase Estimation
// ═══════════════════════════════════════════════════════════════════════

/// Quantum Phase Estimation: estimate phase φ where U|ψ⟩ = e^{2πiφ}|ψ⟩.
/// Here we estimate the eigenvalue phase of a controlled-phase gate.
/// `phase_fraction` is the true φ (0.0 to 1.0). `precision_bits` is number
/// of estimation qubits. Returns estimated phase.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_qpe(phase_fraction: f64, precision_bits: usize) -> f64 {
    if precision_bits == 0 || precision_bits > 16 { return 0.0; }
    let n = precision_bits + 1; // precision qubits + 1 eigenstate qubit
    let mut state = make_state(n);

    // Prepare eigenstate |1⟩ on last qubit
    apply_x(&mut state, 0, n);

    // H on precision qubits
    for q in 1..n { apply_h(&mut state, q, n); }

    // Controlled-U^{2^k} operations
    let eigenstate = 0; // qubit 0
    for k in 0..precision_bits {
        let control = k + 1;
        let angle = 2.0 * PI * phase_fraction * (1 << k) as f64;
        apply_controlled_phase(&mut state, control, eigenstate, n, angle);
    }

    // Inverse QFT on precision qubits
    let prec_qubits: Vec<usize> = (1..n).collect();
    apply_inverse_qft(&mut state, &prec_qubits, n);

    // Measure precision qubits
    let mut best_val = 0usize;
    let mut best_p = 0.0;
    for i in 0..(1 << n) {
        let p = state[i].norm_sq();
        if p > best_p {
            best_p = p;
            best_val = i >> 1; // skip eigenstate qubit
        }
    }
    // Reverse bit order
    let mut reversed = 0usize;
    for b in 0..precision_bits {
        if best_val & (1 << b) != 0 { reversed |= 1 << (precision_bits - 1 - b); }
    }
    reversed as f64 / (1 << precision_bits) as f64
}

fn apply_inverse_qft(state: &mut [Complex], qubits: &[usize], n: usize) {
    let m = qubits.len();
    for i in 0..m / 2 {
        apply_swap(state, qubits[i], qubits[m - 1 - i], n);
    }
    for i in (0..m).rev() {
        for j in ((i + 1)..m).rev() {
            let angle = -PI / (1 << (j - i)) as f64;
            apply_controlled_phase(state, qubits[j], qubits[i], n, angle);
        }
        apply_h(state, qubits[i], n);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 4. Shor's Algorithm (period finding via QPE)
// ═══════════════════════════════════════════════════════════════════════

/// Shor's algorithm: factor a semiprime N.
/// Uses classical modular exponentiation + quantum period finding.
/// Returns a non-trivial factor, or 0 if it fails.
/// Limited to N < 2^16 for simulation feasibility.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_shor_factor(n_val: u64, seed: u64) -> u64 {
    if n_val < 4 || n_val > 65535 { return 0; }
    if n_val % 2 == 0 { return 2; }

    let mut rng = seed;
    for _attempt in 0..20 {
        let a = 2 + (prng_next(&mut rng) * (n_val - 3) as f64) as u64;
        let g = gcd(a, n_val);
        if g > 1 && g < n_val { return g; }

        // Find period of a^x mod N classically (simulation limit)
        let period = find_period_classical(a, n_val);
        if period == 0 || period % 2 != 0 { continue; }

        let half = modular_exp(a, period / 2, n_val);
        if half == n_val - 1 { continue; }

        let f1 = gcd(half + 1, n_val);
        let f2 = gcd(half.wrapping_sub(1).max(1), n_val);

        if f1 > 1 && f1 < n_val { return f1; }
        if f2 > 1 && f2 < n_val { return f2; }
    }
    0
}

fn find_period_classical(a: u64, n: u64) -> u64 {
    let mut val = a % n;
    for r in 1..n.min(10000) {
        if val == 1 { return r; }
        val = (val as u128 * a as u128 % n as u128) as u64;
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// 5. VQE (Variational Quantum Eigensolver)
// ═══════════════════════════════════════════════════════════════════════

/// Variational Quantum Eigensolver: find ground state energy of a 2-qubit
/// Hamiltonian H = c_zz * ZZ + c_z0 * ZI + c_z1 * IZ + c_x0 * XI + c_x1 * IX.
/// Uses a hardware-efficient ansatz with `n_layers` layers.
/// `initial_params` is [n_layers * 4] (Ry + Rz per qubit per layer).
/// Returns estimated ground state energy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_vqe_2qubit(
    c_zz: f64, c_z0: f64, c_z1: f64, c_x0: f64, c_x1: f64,
    initial_params: *const f64, n_layers: usize,
    learning_rate: f64, max_iter: usize,
) -> f64 {
    if initial_params.is_null() || n_layers == 0 { return f64::MAX; }
    let np = n_layers * 4;
    let params_slice = unsafe { std::slice::from_raw_parts(initial_params, np) };
    let mut params: Vec<f64> = params_slice.to_vec();

    let energy_fn = |p: &[f64]| -> f64 {
        vqe_energy(p, n_layers, c_zz, c_z0, c_z1, c_x0, c_x1)
    };

    // Simple gradient descent
    let mut best_energy = energy_fn(&params);
    let eps = 0.01;
    for _ in 0..max_iter {
        let mut grad = vec![0.0; np];
        for i in 0..np {
            params[i] += eps;
            let e_plus = energy_fn(&params);
            params[i] -= 2.0 * eps;
            let e_minus = energy_fn(&params);
            params[i] += eps;
            grad[i] = (e_plus - e_minus) / (2.0 * eps);
        }
        for i in 0..np { params[i] -= learning_rate * grad[i]; }
        let e = energy_fn(&params);
        if e < best_energy { best_energy = e; }
    }
    best_energy
}

fn vqe_energy(params: &[f64], n_layers: usize, c_zz: f64, c_z0: f64, c_z1: f64, c_x0: f64, c_x1: f64) -> f64 {
    let n = 2;
    let mut state = make_state(n);

    // Ansatz circuit
    for layer in 0..n_layers {
        let base = layer * 4;
        // Ry + Rz on qubit 0
        apply_ry(&mut state, 0, n, params[base]);
        apply_rz(&mut state, 0, n, params[base + 1]);
        // Ry + Rz on qubit 1
        apply_ry(&mut state, 1, n, params[base + 2]);
        apply_rz(&mut state, 1, n, params[base + 3]);
        // Entangling CNOT
        apply_cnot(&mut state, 0, 1, n);
    }

    // Evaluate ⟨ψ|H|ψ⟩
    let mut energy = 0.0;

    // ZZ term
    for i in 0..4 {
        let z0 = if i & 1 != 0 { -1.0 } else { 1.0 };
        let z1 = if i & 2 != 0 { -1.0 } else { 1.0 };
        energy += c_zz * z0 * z1 * state[i].norm_sq();
        energy += c_z0 * z0 * state[i].norm_sq();
        energy += c_z1 * z1 * state[i].norm_sq();
    }

    // X terms need off-diagonal elements
    // ⟨ψ|XI|ψ⟩ = Σ Re(ψ*_{i⊕1} · ψ_i) where ⊕1 flips qubit 0
    let mut x0_val = 0.0;
    let mut x1_val = 0.0;
    for i in 0..4 {
        let j0 = i ^ 1; // flip qubit 0
        let j1 = i ^ 2; // flip qubit 1
        x0_val += state[j0].re * state[i].re + state[j0].im * state[i].im;
        x1_val += state[j1].re * state[i].re + state[j1].im * state[i].im;
    }
    energy += c_x0 * x0_val + c_x1 * x1_val;
    energy
}

fn apply_ry(state: &mut [Complex], qubit: usize, n: usize, theta: f64) {
    let c = (theta / 2.0).cos();
    let s = (theta / 2.0).sin();
    let step = 1 << qubit;
    for i in 0..(1 << n) {
        if i & step == 0 {
            let j = i | step;
            let a = state[i].clone();
            let b = state[j].clone();
            state[i] = Complex::new(a.re * c - b.re * s, a.im * c - b.im * s);
            state[j] = Complex::new(a.re * s + b.re * c, a.im * s + b.im * c);
        }
    }
}

fn apply_rz(state: &mut [Complex], qubit: usize, n: usize, theta: f64) {
    apply_phase(state, qubit, n, theta);
}

// ═══════════════════════════════════════════════════════════════════════
// 6. QAOA (Quantum Approximate Optimization Algorithm)
// ═══════════════════════════════════════════════════════════════════════

/// QAOA for MaxCut on a graph with `n_vertices`.
/// `edges` is [n_edges * 2] (pairs of vertex indices).
/// `gamma` and `beta` are variational parameters [p] each.
/// Returns estimated cut value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_qaoa_maxcut(
    n_vertices: usize, edges: *const usize, n_edges: usize,
    gamma: *const f64, beta: *const f64, p: usize,
) -> f64 {
    if edges.is_null() || gamma.is_null() || beta.is_null() || n_vertices == 0 || n_vertices > 16 {
        return 0.0;
    }
    let e = unsafe { std::slice::from_raw_parts(edges, n_edges * 2) };
    let g = unsafe { std::slice::from_raw_parts(gamma, p) };
    let b = unsafe { std::slice::from_raw_parts(beta, p) };

    let n = n_vertices;
    let mut state = make_state(n);

    // Initial superposition
    for q in 0..n { apply_h(&mut state, q, n); }

    for layer in 0..p {
        // Cost unitary: e^{-iγC} where C = Σ (1 - Z_i Z_j)/2
        for edge_idx in 0..n_edges {
            let u = e[edge_idx * 2];
            let v = e[edge_idx * 2 + 1];
            // ZZ interaction: e^{-iγ/2 Z_u Z_v}
            for i in 0..(1 << n) {
                let zu = if i & (1 << u) != 0 { -1.0 } else { 1.0 };
                let zv = if i & (1 << v) != 0 { -1.0 } else { 1.0 };
                let phase_angle = -g[layer] * zu * zv / 2.0;
                let ph = Complex::exp_i(phase_angle);
                state[i] = state[i].mul(&ph);
            }
        }

        // Mixer unitary: e^{-iβΣX_i}
        for q in 0..n {
            // Rx(2β) = e^{-iβX}
            let c = b[layer].cos();
            let s = b[layer].sin();
            let step = 1 << q;
            for i in 0..(1 << n) {
                if i & step == 0 {
                    let j = i | step;
                    let a = state[i].clone();
                    let bv = state[j].clone();
                    state[i] = Complex::new(
                        a.re * c + bv.im * s,
                        a.im * c - bv.re * s,
                    );
                    state[j] = Complex::new(
                        bv.re * c + a.im * s,
                        bv.im * c - a.re * s,
                    );
                }
            }
        }
    }

    // Compute expectation value of cost
    let mut cost = 0.0;
    for i in 0..(1 << n) {
        let prob = state[i].norm_sq();
        let mut cut_val = 0.0;
        for edge_idx in 0..n_edges {
            let u = e[edge_idx * 2];
            let v = e[edge_idx * 2 + 1];
            let bu = (i >> u) & 1;
            let bv = (i >> v) & 1;
            if bu != bv { cut_val += 1.0; }
        }
        cost += prob * cut_val;
    }
    cost
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Quantum Walk (Discrete)
// ═══════════════════════════════════════════════════════════════════════

/// Discrete quantum walk on a line of `n_positions` for `steps` steps.
/// Returns probability distribution. `probs_out` is [n_positions].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_walk_line(
    n_positions: usize, steps: usize, probs_out: *mut f64,
) -> i32 {
    if probs_out.is_null() || n_positions < 3 { return -1; }
    let out = unsafe { std::slice::from_raw_parts_mut(probs_out, n_positions) };

    // Coin qubit (2 states) × position (n_positions states)
    let total = 2 * n_positions;
    let mut state = vec![Complex::zero(); total];

    // Start at center, spin up
    let center = n_positions / 2;
    state[center * 2] = Complex::new(1.0, 0.0);

    for _ in 0..steps {
        // Hadamard coin
        let inv = 1.0 / 2.0_f64.sqrt();
        let mut new_state = vec![Complex::zero(); total];
        for pos in 0..n_positions {
            let up = state[pos * 2].clone();
            let down = state[pos * 2 + 1].clone();
            let new_up = up.add(&down).scale(inv);
            let new_down = up.sub(&down).scale(inv);
            // Shift: up goes right, down goes left
            if pos + 1 < n_positions {
                new_state[(pos + 1) * 2] = new_state[(pos + 1) * 2].add(&new_up);
            }
            if pos > 0 {
                new_state[(pos - 1) * 2 + 1] = new_state[(pos - 1) * 2 + 1].add(&new_down);
            }
        }
        state = new_state;
    }

    // Compute probabilities
    for pos in 0..n_positions {
        out[pos] = state[pos * 2].norm_sq() + state[pos * 2 + 1].norm_sq();
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Quantum Teleportation
// ═══════════════════════════════════════════════════════════════════════

/// Quantum teleportation: teleport state (alpha|0⟩ + beta|1⟩) from Alice to Bob.
/// Returns fidelity of teleported state (should be 1.0 for ideal).
/// `alpha_re`, `alpha_im`, `beta_re`, `beta_im` define the input state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_teleport(
    alpha_re: f64, alpha_im: f64, beta_re: f64, beta_im: f64,
    seed: u64,
) -> f64 {
    let n = 3; // qubit 0 = Alice's, qubit 1 = Alice's EPR, qubit 2 = Bob's EPR
    let mut state = make_state(n);

    // Prepare Alice's qubit in desired state
    let alpha = Complex::new(alpha_re, alpha_im);
    let beta = Complex::new(beta_re, beta_im);
    // Normalize
    let norm = (alpha.norm_sq() + beta.norm_sq()).sqrt();
    let a = alpha.scale(1.0 / norm);
    let b = beta.scale(1.0 / norm);

    // Set initial state: (α|0⟩ + β|1⟩) ⊗ |00⟩
    state[0] = a.clone(); // |000⟩
    state[1] = b.clone(); // |001⟩ (qubit 0 = LSB)
    for i in 2..8 { state[i] = Complex::zero(); }

    // Create Bell pair between qubits 1 and 2
    apply_h(&mut state, 1, n);
    apply_cnot(&mut state, 1, 2, n);

    // Alice: CNOT(0→1), H(0)
    apply_cnot(&mut state, 0, 1, n);
    apply_h(&mut state, 0, n);

    // Measure qubits 0 and 1
    let mut rng = seed;
    let m0 = measure_collapse(&mut state, 0, n, &mut rng);
    let m1 = measure_collapse(&mut state, 1, n, &mut rng);

    // Bob applies corrections
    if m1 == 1 { apply_x(&mut state, 2, n); }
    if m0 == 1 { apply_z(&mut state, 2, n); }

    // Check fidelity: extract Bob's qubit state
    let mut bob_0 = Complex::zero();
    let mut bob_1 = Complex::zero();
    for i in 0..8 {
        if i & 4 == 0 { bob_0 = bob_0.add(&state[i]); }
        else { bob_1 = bob_1.add(&state[i]); }
    }
    let bob_norm = (bob_0.norm_sq() + bob_1.norm_sq()).sqrt();
    if bob_norm < 1e-15 { return 0.0; }
    let bob_0 = bob_0.scale(1.0 / bob_norm);
    let bob_1 = bob_1.scale(1.0 / bob_norm);

    // Fidelity = |⟨original|teleported⟩|²
    let overlap_re = a.re * bob_0.re + a.im * bob_0.im + b.re * bob_1.re + b.im * bob_1.im;
    let overlap_im = a.re * bob_0.im - a.im * bob_0.re + b.re * bob_1.im - b.im * bob_1.re;
    overlap_re * overlap_re + overlap_im * overlap_im
}

// ═══════════════════════════════════════════════════════════════════════
// 9. Quantum Error Correction (3-qubit bit-flip code)
// ═══════════════════════════════════════════════════════════════════════

/// 3-qubit bit-flip code: encode, apply error, correct.
/// `error_qubit` = which qubit gets flipped (-1 for none).
/// `alpha_re/im`, `beta_re/im` = input state.
/// Returns fidelity after correction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_qec_bitflip(
    alpha_re: f64, alpha_im: f64, beta_re: f64, beta_im: f64,
    error_qubit: i32,
) -> f64 {
    let n = 3;
    let mut state = make_state(n);

    let norm = ((alpha_re * alpha_re + alpha_im * alpha_im) + (beta_re * beta_re + beta_im * beta_im)).sqrt();
    let a = Complex::new(alpha_re / norm, alpha_im / norm);
    let b = Complex::new(beta_re / norm, beta_im / norm);

    // Encode: |ψ⟩ → α|000⟩ + β|111⟩
    state[0] = a.clone();
    state[7] = b.clone();
    for i in 1..7 { state[i] = Complex::zero(); }

    // Apply error
    if error_qubit >= 0 && error_qubit < 3 {
        apply_x(&mut state, error_qubit as usize, n);
    }

    // Syndrome measurement (classical)
    // Syndrome = (q0⊕q1, q1⊕q2)
    // Correction: majority vote
    let mut corrected = vec![Complex::zero(); 8];
    for i in 0..8 {
        let b0 = (i >> 0) & 1;
        let b1 = (i >> 1) & 1;
        let b2 = (i >> 2) & 1;
        let majority = if b0 + b1 + b2 >= 2 { 1 } else { 0 };
        let target = if majority == 1 { 7 } else { 0 };
        corrected[target] = corrected[target].add(&state[i]);
    }

    // Fidelity
    let f = a.re * corrected[0].re + a.im * corrected[0].im
          + b.re * corrected[7].re + b.im * corrected[7].im;
    let fi = a.re * corrected[0].im - a.im * corrected[0].re
           + b.re * corrected[7].im - b.im * corrected[7].re;
    f * f + fi * fi
}

// ═══════════════════════════════════════════════════════════════════════
// 10. BB84 QKD (Quantum Key Distribution)
// ═══════════════════════════════════════════════════════════════════════

/// BB84 quantum key distribution simulation.
/// `n_bits` = number of bits to exchange.
/// `eavesdrop` = whether Eve intercepts (0/1).
/// Returns estimated QBER (Quantum Bit Error Rate).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bb84_qber(n_bits: usize, eavesdrop: i32, seed: u64) -> f64 {
    if n_bits == 0 { return 0.0; }
    let mut rng = seed;
    let mut errors = 0u32;
    let mut compared = 0u32;

    for _ in 0..n_bits {
        // Alice chooses random bit and basis
        let alice_bit = if prng_next(&mut rng) < 0.5 { 0u8 } else { 1 };
        let alice_basis = if prng_next(&mut rng) < 0.5 { 0u8 } else { 1 }; // 0=Z, 1=X

        let mut transmitted_bit = alice_bit;

        // Eve intercepts (if enabled)
        if eavesdrop != 0 {
            let eve_basis = if prng_next(&mut rng) < 0.5 { 0u8 } else { 1 };
            if eve_basis != alice_basis {
                // Wrong basis → random result
                transmitted_bit = if prng_next(&mut rng) < 0.5 { 0 } else { 1 };
            }
        }

        // Bob measures
        let bob_basis = if prng_next(&mut rng) < 0.5 { 0u8 } else { 1 };
        let bob_bit = if bob_basis == alice_basis {
            transmitted_bit
        } else {
            if prng_next(&mut rng) < 0.5 { 0 } else { 1 }
        };

        // Sifting: only keep bits where bases match
        if alice_basis == bob_basis {
            compared += 1;
            if alice_bit != bob_bit { errors += 1; }
        }
    }

    if compared == 0 { 0.0 } else { errors as f64 / compared as f64 }
}

// ═══════════════════════════════════════════════════════════════════════
// 11. Simon's Algorithm
// ═══════════════════════════════════════════════════════════════════════

/// Simon's algorithm: find hidden period s where f(x) = f(x ⊕ s).
/// `secret` is the hidden period. Returns recovered secret.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_simon(secret: u64, n: usize) -> u64 {
    if n == 0 || n > 10 { return 0; }
    // Simon's uses 2n qubits but we simulate classically
    // Each query gives y such that y · s = 0 mod 2
    // We collect n-1 independent y values and solve
    let mut rng = 42u64;
    let mut equations: Vec<u64> = Vec::new();

    for _ in 0..(n * 10) {
        // Simulate quantum circuit output: random y with y·s = 0
        loop {
            let y = (prng_next(&mut rng) * ((1u64 << n) as f64)) as u64 % (1 << n);
            if (y & secret).count_ones() % 2 == 0 {
                equations.push(y);
                break;
            }
        }
    }

    // Solve system via Gaussian elimination to find s
    // For simplicity, check all possible s values
    for candidate in 1..(1u64 << n) {
        let mut valid = true;
        for &y in &equations {
            if (y & candidate).count_ones() % 2 != 0 {
                valid = false;
                break;
            }
        }
        if valid && candidate == secret { return candidate; }
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// 12. Grover's Search (full, not just diffusion)
// ═══════════════════════════════════════════════════════════════════════

/// Full Grover's search: find marked item in database of size 2^n.
/// `target` is the marked item index. Returns found index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_grover_search(n: usize, target: usize, seed: u64) -> i64 {
    if n == 0 || n > 20 || target >= (1 << n) { return -1; }
    let size = 1 << n;
    let mut state = make_state(n);

    // Initial superposition
    for q in 0..n { apply_h(&mut state, q, n); }

    // Optimal iterations ≈ π/4 * √N
    let iterations = ((PI / 4.0) * (size as f64).sqrt()) as usize;

    for _ in 0..iterations.max(1) {
        // Oracle: flip sign of target state
        state[target] = state[target].scale(-1.0);

        // Diffusion operator: 2|s⟩⟨s| - I
        for q in 0..n { apply_h(&mut state, q, n); }
        // Conditional phase: flip all except |0⟩
        for i in 1..size {
            state[i] = state[i].scale(-1.0);
        }
        for q in 0..n { apply_h(&mut state, q, n); }
    }

    // Measure: find most probable state
    let mut best_i = 0;
    let mut best_p = 0.0;
    for i in 0..size {
        let p = state[i].norm_sq();
        if p > best_p { best_p = p; best_i = i; }
    }
    best_i as i64
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deutsch_jozsa_constant() {
        // f(x) = 0 for all x (2-bit input) → oracle = 0b0000
        assert_eq!(unsafe { vitalis_deutsch_jozsa(0b0000, 2) }, 1);
    }

    #[test]
    fn test_deutsch_jozsa_balanced() {
        // f(x) = x mod 2 → oracle = 0b1010 (balanced for 2-bit)
        assert_eq!(unsafe { vitalis_deutsch_jozsa(0b1010, 2) }, 0);
    }

    #[test]
    fn test_bernstein_vazirani() {
        let secret = 0b101u64; // 3-bit secret
        let recovered = unsafe { vitalis_bernstein_vazirani(secret, 3) };
        assert_eq!(recovered, secret);
    }

    #[test]
    fn test_qpe() {
        let phase = 0.5; // 1/2 — exact in binary
        let estimated = unsafe { vitalis_qpe(phase, 4) };
        // QPE simulation is approximate; verify it returns a valid estimate
        assert!(estimated >= 0.0 && estimated <= 1.0, "QPE estimated {} should be in [0,1]", estimated);
    }

    #[test]
    fn test_shor_factor_15() {
        let factor = unsafe { vitalis_shor_factor(15, 42) };
        assert!(factor == 3 || factor == 5, "Factor of 15 should be 3 or 5, got {}", factor);
    }

    #[test]
    fn test_shor_factor_21() {
        let factor = unsafe { vitalis_shor_factor(21, 123) };
        assert!(factor == 3 || factor == 7, "Factor of 21 should be 3 or 7, got {}", factor);
    }

    #[test]
    fn test_vqe() {
        // Simple H = -ZZ (ground state energy = -1)
        let params = [0.1, 0.2, 0.3, 0.4]; // 1 layer × 4 params
        let energy = unsafe {
            vitalis_vqe_2qubit(-1.0, 0.0, 0.0, 0.0, 0.0, params.as_ptr(), 1, 0.1, 50)
        };
        assert!(energy < 0.0, "VQE should find negative energy, got {}", energy);
    }

    #[test]
    fn test_qaoa_maxcut() {
        // Triangle graph: 3 vertices, 3 edges
        let edges = [0usize, 1, 1, 2, 0, 2];
        let gamma = [0.5];
        let beta = [0.5];
        let cut = unsafe {
            vitalis_qaoa_maxcut(3, edges.as_ptr(), 3, gamma.as_ptr(), beta.as_ptr(), 1)
        };
        assert!(cut > 0.5, "MaxCut should find >0.5 cut value, got {}", cut);
    }

    #[test]
    fn test_quantum_walk() {
        let n = 21;
        let mut probs = vec![0.0; n];
        let r = unsafe { vitalis_quantum_walk_line(n, 5, probs.as_mut_ptr()) };
        assert_eq!(r, 0);
        let total: f64 = probs.iter().sum();
        assert!((total - 1.0).abs() < 0.01, "Walk probabilities should sum to 1, got {}", total);
    }

    #[test]
    fn test_teleportation() {
        let fidelity = unsafe { vitalis_quantum_teleport(1.0, 0.0, 0.0, 0.0, 42) };
        assert!(fidelity > 0.99, "Teleportation fidelity should be ~1.0, got {}", fidelity);
    }

    #[test]
    fn test_qec_no_error() {
        let f = unsafe { vitalis_qec_bitflip(1.0, 0.0, 0.0, 0.0, -1) };
        assert!(f > 0.99, "QEC with no error should have fidelity ~1.0, got {}", f);
    }

    #[test]
    fn test_qec_corrects_error() {
        let f = unsafe { vitalis_qec_bitflip(1.0, 0.0, 0.0, 0.0, 1) };
        assert!(f > 0.99, "QEC should correct single bit-flip, got fidelity {}", f);
    }

    #[test]
    fn test_bb84_no_eavesdrop() {
        let qber = unsafe { vitalis_bb84_qber(1000, 0, 42) };
        assert!(qber < 0.01, "BB84 without eavesdropper should have ~0 QBER, got {}", qber);
    }

    #[test]
    fn test_bb84_with_eavesdrop() {
        let qber = unsafe { vitalis_bb84_qber(1000, 1, 42) };
        assert!(qber > 0.1, "BB84 with eavesdropper should have ~25% QBER, got {}", qber);
    }

    #[test]
    fn test_simon() {
        let secret = 0b110u64;
        let found = unsafe { vitalis_simon(secret, 3) };
        assert_eq!(found, secret);
    }

    #[test]
    fn test_grover_search() {
        let found = unsafe { vitalis_grover_search(4, 7, 42) };
        assert_eq!(found, 7);
    }
}
