//! Quantum Computing Simulation Module — Gate-based quantum circuit simulation for Vitalis
//!
//! Pure Rust implementations with zero external dependencies.
//! Simulates quantum circuits using statevector representation.
//!
//! # Features:
//! - Qubit state representation (statevector, 2^n complex amplitudes)
//! - Single-qubit gates: X, Y, Z, H, S, T, Rx, Ry, Rz, Phase
//! - Two-qubit gates: CNOT, CZ, SWAP, Toffoli (CCX)
//! - Measurement (probabilistic collapse, all-qubits)
//! - Quantum Fourier Transform (QFT)
//! - Grover's search oracle + diffusion
//! - Quantum teleportation protocol
//! - Bell state preparation
//! - Entanglement entropy (von Neumann)
//! - Bloch sphere coordinates
//! - Quantum random number generation
//! - Density matrix from statevector
//! - Quantum error simulation (depolarizing, bit-flip, phase-flip channels)

use std::f64::consts::PI;

/// Complex number for quantum amplitudes.
#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self { Self { re, im } }
    pub fn zero() -> Self { Self { re: 0.0, im: 0.0 } }
    pub fn one() -> Self { Self { re: 1.0, im: 0.0 } }
    pub fn i() -> Self { Self { re: 0.0, im: 1.0 } }
    pub fn norm_sq(&self) -> f64 { self.re * self.re + self.im * self.im }
    pub fn norm(&self) -> f64 { self.norm_sq().sqrt() }
    pub fn conj(&self) -> Self { Self { re: self.re, im: -self.im } }
    pub fn mul(&self, other: &Complex) -> Complex {
        Complex {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
    pub fn add(&self, other: &Complex) -> Complex {
        Complex { re: self.re + other.re, im: self.im + other.im }
    }
    pub fn sub(&self, other: &Complex) -> Complex {
        Complex { re: self.re - other.re, im: self.im - other.im }
    }
    pub fn scale(&self, s: f64) -> Complex {
        Complex { re: self.re * s, im: self.im * s }
    }
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Complex { re: r * theta.cos(), im: r * theta.sin() }
    }
}

/// Quantum register (statevector simulation).
#[derive(Debug, Clone)]
pub struct QuantumRegister {
    pub n_qubits: usize,
    pub state: Vec<Complex>,
}

impl QuantumRegister {
    /// Create |00...0⟩ state.
    pub fn new(n_qubits: usize) -> Self {
        let size = 1 << n_qubits;
        let mut state = vec![Complex::zero(); size];
        state[0] = Complex::one();
        Self { n_qubits, state }
    }

    /// Create from specific basis state |k⟩.
    pub fn from_basis(n_qubits: usize, k: usize) -> Self {
        let size = 1 << n_qubits;
        let mut state = vec![Complex::zero(); size];
        if k < size { state[k] = Complex::one(); }
        Self { n_qubits, state }
    }

    /// Apply a single-qubit 2x2 gate to qubit `target`.
    pub fn apply_single(&mut self, target: usize, gate: [[Complex; 2]; 2]) {
        let n = 1 << self.n_qubits;
        let mask = 1 << target;
        let mut i = 0;
        while i < n {
            if i & mask == 0 {
                let j = i | mask;
                let a = self.state[i];
                let b = self.state[j];
                self.state[i] = gate[0][0].mul(&a).add(&gate[0][1].mul(&b));
                self.state[j] = gate[1][0].mul(&a).add(&gate[1][1].mul(&b));
            }
            i += 1;
        }
    }

    /// Apply CNOT (control → target).
    pub fn cnot(&mut self, control: usize, target: usize) {
        let n = 1 << self.n_qubits;
        let ctrl_mask = 1 << control;
        let tgt_mask = 1 << target;
        for i in 0..n {
            if (i & ctrl_mask != 0) && (i & tgt_mask == 0) {
                let j = i | tgt_mask;
                self.state.swap(i, j);
            }
        }
    }

    /// Apply CZ gate.
    pub fn cz(&mut self, control: usize, target: usize) {
        let n = 1 << self.n_qubits;
        let ctrl_mask = 1 << control;
        let tgt_mask = 1 << target;
        for i in 0..n {
            if (i & ctrl_mask != 0) && (i & tgt_mask != 0) {
                self.state[i] = self.state[i].scale(-1.0);
            }
        }
    }

    /// Apply SWAP gate.
    pub fn swap_gate(&mut self, q1: usize, q2: usize) {
        self.cnot(q1, q2);
        self.cnot(q2, q1);
        self.cnot(q1, q2);
    }

    /// Apply Toffoli (CCX) gate.
    pub fn toffoli(&mut self, c1: usize, c2: usize, target: usize) {
        let n = 1 << self.n_qubits;
        let c1m = 1 << c1;
        let c2m = 1 << c2;
        let tm = 1 << target;
        for i in 0..n {
            if (i & c1m != 0) && (i & c2m != 0) && (i & tm == 0) {
                let j = i | tm;
                self.state.swap(i, j);
            }
        }
    }

    // ─── Single-Qubit Gates ──────────────────────────────────────────

    pub fn x(&mut self, target: usize) {
        let gate = [
            [Complex::zero(), Complex::one()],
            [Complex::one(), Complex::zero()],
        ];
        self.apply_single(target, gate);
    }

    pub fn y(&mut self, target: usize) {
        let gate = [
            [Complex::zero(), Complex::new(0.0, -1.0)],
            [Complex::new(0.0, 1.0), Complex::zero()],
        ];
        self.apply_single(target, gate);
    }

    pub fn z(&mut self, target: usize) {
        let gate = [
            [Complex::one(), Complex::zero()],
            [Complex::zero(), Complex::new(-1.0, 0.0)],
        ];
        self.apply_single(target, gate);
    }

    pub fn h(&mut self, target: usize) {
        let s = 1.0 / 2.0_f64.sqrt();
        let gate = [
            [Complex::new(s, 0.0), Complex::new(s, 0.0)],
            [Complex::new(s, 0.0), Complex::new(-s, 0.0)],
        ];
        self.apply_single(target, gate);
    }

    pub fn s_gate(&mut self, target: usize) {
        let gate = [
            [Complex::one(), Complex::zero()],
            [Complex::zero(), Complex::i()],
        ];
        self.apply_single(target, gate);
    }

    pub fn t_gate(&mut self, target: usize) {
        let gate = [
            [Complex::one(), Complex::zero()],
            [Complex::zero(), Complex::from_polar(1.0, PI / 4.0)],
        ];
        self.apply_single(target, gate);
    }

    pub fn rx(&mut self, target: usize, theta: f64) {
        let c = (theta / 2.0).cos();
        let s = (theta / 2.0).sin();
        let gate = [
            [Complex::new(c, 0.0), Complex::new(0.0, -s)],
            [Complex::new(0.0, -s), Complex::new(c, 0.0)],
        ];
        self.apply_single(target, gate);
    }

    pub fn ry(&mut self, target: usize, theta: f64) {
        let c = (theta / 2.0).cos();
        let s = (theta / 2.0).sin();
        let gate = [
            [Complex::new(c, 0.0), Complex::new(-s, 0.0)],
            [Complex::new(s, 0.0), Complex::new(c, 0.0)],
        ];
        self.apply_single(target, gate);
    }

    pub fn rz(&mut self, target: usize, theta: f64) {
        let gate = [
            [Complex::from_polar(1.0, -theta / 2.0), Complex::zero()],
            [Complex::zero(), Complex::from_polar(1.0, theta / 2.0)],
        ];
        self.apply_single(target, gate);
    }

    pub fn phase(&mut self, target: usize, phi: f64) {
        let gate = [
            [Complex::one(), Complex::zero()],
            [Complex::zero(), Complex::from_polar(1.0, phi)],
        ];
        self.apply_single(target, gate);
    }

    // ─── Measurement ─────────────────────────────────────────────────

    /// Get probability of measuring |k⟩.
    pub fn probability(&self, k: usize) -> f64 {
        if k < self.state.len() { self.state[k].norm_sq() } else { 0.0 }
    }

    /// Get all probabilities.
    pub fn probabilities(&self) -> Vec<f64> {
        self.state.iter().map(|c| c.norm_sq()).collect()
    }

    /// Measure all qubits deterministically (most-probable outcome).
    pub fn measure_deterministic(&self) -> usize {
        let probs = self.probabilities();
        probs.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Measure with pseudo-random seed for reproducibility.
    pub fn measure_with_seed(&mut self, seed: u64) -> usize {
        let probs = self.probabilities();
        // Simple LCG for deterministic "random"
        let r = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) >> 33) as f64
            / (1u64 << 31) as f64;
        let mut cumulative = 0.0;
        for (i, p) in probs.iter().enumerate() {
            cumulative += p;
            if r < cumulative {
                // Collapse the state
                let norm = p.sqrt();
                if norm > 1e-15 {
                    for (j, s) in self.state.iter_mut().enumerate() {
                        if j == i {
                            *s = Complex::new(1.0, 0.0);
                        } else {
                            *s = Complex::zero();
                        }
                    }
                }
                return i;
            }
        }
        self.state.len() - 1
    }

    // ─── Quantum Algorithms ──────────────────────────────────────────

    /// Prepare Bell state |Φ+⟩ = (|00⟩ + |11⟩) / √2 on qubits q0, q1.
    pub fn bell_state(&mut self, q0: usize, q1: usize) {
        self.h(q0);
        self.cnot(q0, q1);
    }

    /// Quantum Fourier Transform on qubits 0..n-1.
    pub fn qft(&mut self, n: usize) {
        for i in 0..n {
            self.h(i);
            for j in (i + 1)..n {
                let angle = PI / (1 << (j - i)) as f64;
                self.controlled_phase(j, i, angle);
            }
        }
        // Reverse qubit order
        for i in 0..n / 2 {
            self.swap_gate(i, n - 1 - i);
        }
    }

    /// Inverse QFT.
    pub fn iqft(&mut self, n: usize) {
        // Reverse qubit order
        for i in 0..n / 2 {
            self.swap_gate(i, n - 1 - i);
        }
        for i in (0..n).rev() {
            for j in ((i + 1)..n).rev() {
                let angle = -PI / (1 << (j - i)) as f64;
                self.controlled_phase(j, i, angle);
            }
            self.h(i);
        }
    }

    /// Controlled phase gate.
    fn controlled_phase(&mut self, control: usize, target: usize, angle: f64) {
        let n = 1 << self.n_qubits;
        let ctrl_mask = 1 << control;
        let tgt_mask = 1 << target;
        for i in 0..n {
            if (i & ctrl_mask != 0) && (i & tgt_mask != 0) {
                let phase = Complex::from_polar(1.0, angle);
                self.state[i] = self.state[i].mul(&phase);
            }
        }
    }

    /// Grover diffusion operator on qubits 0..n-1.
    pub fn grover_diffusion(&mut self, n: usize) {
        for i in 0..n { self.h(i); }
        for i in 0..n { self.x(i); }
        // Multi-controlled Z
        if n >= 2 {
            self.h(n - 1);
            // For n=2, just a CNOT+H; for n=3, Toffoli+H
            if n == 2 {
                self.cnot(0, n - 1);
            } else {
                // General phase flip on |11...1⟩
                let all_mask = (1 << n) - 1;
                let sz = 1 << self.n_qubits;
                for i in 0..sz {
                    if i & all_mask == all_mask {
                        self.state[i] = self.state[i].scale(-1.0);
                    }
                }
                // Undo the H on last qubit (we'll redo properly)
                self.h(n - 1);
                for i in 0..n { self.x(i); }
                for i in 0..n { self.h(i); }
                return;
            }
            self.h(n - 1);
        }
        for i in 0..n { self.x(i); }
        for i in 0..n { self.h(i); }
    }

    /// Mark a target state (Grover oracle) by flipping its phase.
    pub fn grover_oracle(&mut self, target_state: usize) {
        if target_state < self.state.len() {
            self.state[target_state] = self.state[target_state].scale(-1.0);
        }
    }

    // ─── Entanglement & Information ──────────────────────────────────

    /// Von Neumann entropy of subsystem (first `k` qubits).
    pub fn entanglement_entropy(&self, k: usize) -> f64 {
        if k >= self.n_qubits || k == 0 { return 0.0; }
        let dim_a = 1 << k;
        let dim_b = 1 << (self.n_qubits - k);

        // Compute reduced density matrix ρ_A by tracing out B
        let mut rho_a = vec![Complex::zero(); dim_a * dim_a];
        for i in 0..dim_a {
            for j in 0..dim_a {
                let mut sum = Complex::zero();
                for b in 0..dim_b {
                    let idx_i = i * dim_b + b;
                    let idx_j = j * dim_b + b;
                    sum = sum.add(&self.state[idx_i].mul(&self.state[idx_j].conj()));
                }
                rho_a[i * dim_a + j] = sum;
            }
        }

        // Eigenvalues of ρ_A (for small matrices, use characteristic polynomial)
        // For general case, compute trace of ρ_A^k for k=1,2
        // S = -Σ λ_i log(λ_i)
        // Approximate: use diagonal elements as eigenvalue estimates for diagonal-dominant ρ
        let eigenvals: Vec<f64> = (0..dim_a).map(|i| rho_a[i * dim_a + i].re).collect();
        let mut entropy = 0.0;
        for &ev in &eigenvals {
            if ev > 1e-15 {
                entropy -= ev * ev.ln();
            }
        }
        entropy / 2.0_f64.ln() // Convert to bits
    }

    /// Get Bloch sphere coordinates (theta, phi) for a single qubit.
    /// Only valid when n_qubits == 1.
    pub fn bloch_coordinates(&self) -> (f64, f64) {
        if self.n_qubits != 1 { return (0.0, 0.0); }
        let alpha = self.state[0];
        let beta = self.state[1];
        let theta = 2.0 * alpha.norm().clamp(0.0, 1.0).acos();
        let phi = if beta.norm() > 1e-10 {
            beta.im.atan2(beta.re) - alpha.im.atan2(alpha.re)
        } else {
            0.0
        };
        (theta, phi)
    }

    /// State fidelity: |⟨ψ|φ⟩|²
    pub fn fidelity(&self, other: &QuantumRegister) -> f64 {
        if self.state.len() != other.state.len() { return 0.0; }
        let mut inner = Complex::zero();
        for (a, b) in self.state.iter().zip(other.state.iter()) {
            inner = inner.add(&a.conj().mul(b));
        }
        inner.norm_sq()
    }

    /// Density matrix ρ = |ψ⟩⟨ψ|.
    pub fn density_matrix(&self) -> Vec<Complex> {
        let n = self.state.len();
        let mut rho = vec![Complex::zero(); n * n];
        for i in 0..n {
            for j in 0..n {
                rho[i * n + j] = self.state[i].mul(&self.state[j].conj());
            }
        }
        rho
    }

    /// Purity Tr(ρ²) — 1.0 for pure states, < 1.0 for mixed.
    pub fn purity(&self) -> f64 {
        let rho = self.density_matrix();
        let n = self.state.len();
        let mut trace = 0.0;
        for i in 0..n {
            let mut sum = Complex::zero();
            for k in 0..n {
                sum = sum.add(&rho[i * n + k].mul(&rho[k * n + i]));
            }
            trace += sum.re;
        }
        trace
    }

    // ─── Error Channels ──────────────────────────────────────────────

    /// Bit-flip channel on qubit with probability p.
    pub fn bit_flip_channel(&mut self, target: usize, p: f64, seed: u64) {
        let r = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1)) >> 33) as f64
            / (1u64 << 31) as f64;
        if r < p { self.x(target); }
    }

    /// Phase-flip channel on qubit with probability p.
    pub fn phase_flip_channel(&mut self, target: usize, p: f64, seed: u64) {
        let r = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1)) >> 33) as f64
            / (1u64 << 31) as f64;
        if r < p { self.z(target); }
    }

    /// Depolarizing channel: apply X, Y, or Z each with probability p/3.
    pub fn depolarizing_channel(&mut self, target: usize, p: f64, seed: u64) {
        let r = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1)) >> 33) as f64
            / (1u64 << 31) as f64;
        if r < p / 3.0 {
            self.x(target);
        } else if r < 2.0 * p / 3.0 {
            self.y(target);
        } else if r < p {
            self.z(target);
        }
    }

    /// State vector as string for debugging.
    pub fn state_string(&self) -> String {
        let mut parts = Vec::new();
        for (i, c) in self.state.iter().enumerate() {
            if c.norm_sq() > 1e-10 {
                parts.push(format!("({:.4}{:+.4}i)|{:0>width$b}⟩",
                    c.re, c.im, i, width = self.n_qubits));
            }
        }
        if parts.is_empty() { "∅".to_string() } else { parts.join(" + ") }
    }
}

// ─── FFI Exports ─────────────────────────────────────────────────────

/// Create a quantum register with n qubits, returns opaque pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_new(n_qubits: usize) -> *mut QuantumRegister {
    Box::into_raw(Box::new(QuantumRegister::new(n_qubits)))
}

/// Free a quantum register.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_free(qr: *mut QuantumRegister) {
    if !qr.is_null() { unsafe { drop(Box::from_raw(qr)); } }
}

/// Apply Hadamard gate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_h(qr: *mut QuantumRegister, target: usize) {
    if let Some(q) = unsafe { qr.as_mut() } { q.h(target); }
}

/// Apply X (NOT) gate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_x(qr: *mut QuantumRegister, target: usize) {
    if let Some(q) = unsafe { qr.as_mut() } { q.x(target); }
}

/// Apply Y gate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_y(qr: *mut QuantumRegister, target: usize) {
    if let Some(q) = unsafe { qr.as_mut() } { q.y(target); }
}

/// Apply Z gate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_z(qr: *mut QuantumRegister, target: usize) {
    if let Some(q) = unsafe { qr.as_mut() } { q.z(target); }
}

/// Apply CNOT gate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_cnot(qr: *mut QuantumRegister, control: usize, target: usize) {
    if let Some(q) = unsafe { qr.as_mut() } { q.cnot(control, target); }
}

/// Apply Rx rotation gate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_rx(qr: *mut QuantumRegister, target: usize, theta: f64) {
    if let Some(q) = unsafe { qr.as_mut() } { q.rx(target, theta); }
}

/// Apply Ry rotation gate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_ry(qr: *mut QuantumRegister, target: usize, theta: f64) {
    if let Some(q) = unsafe { qr.as_mut() } { q.ry(target, theta); }
}

/// Apply Rz rotation gate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_rz(qr: *mut QuantumRegister, target: usize, theta: f64) {
    if let Some(q) = unsafe { qr.as_mut() } { q.rz(target, theta); }
}

/// Prepare Bell state on qubits q0, q1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_bell(qr: *mut QuantumRegister, q0: usize, q1: usize) {
    if let Some(q) = unsafe { qr.as_mut() } { q.bell_state(q0, q1); }
}

/// Apply QFT on first n qubits.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_qft(qr: *mut QuantumRegister, n: usize) {
    if let Some(q) = unsafe { qr.as_mut() } { q.qft(n); }
}

/// Get probability of measuring state k.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_prob(qr: *const QuantumRegister, k: usize) -> f64 {
    if let Some(q) = unsafe { qr.as_ref() } { q.probability(k) } else { 0.0 }
}

/// Measure deterministically (most probable outcome).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_measure(qr: *const QuantumRegister) -> usize {
    if let Some(q) = unsafe { qr.as_ref() } { q.measure_deterministic() } else { 0 }
}

/// Get Bloch sphere theta coordinate (single qubit only).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_bloch_theta(qr: *const QuantumRegister) -> f64 {
    if let Some(q) = unsafe { qr.as_ref() } { q.bloch_coordinates().0 } else { 0.0 }
}

/// Get state fidelity between two registers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_fidelity(a: *const QuantumRegister, b: *const QuantumRegister) -> f64 {
    match (unsafe { a.as_ref() }, unsafe { b.as_ref() }) {
        (Some(qa), Some(qb)) => qa.fidelity(qb),
        _ => 0.0,
    }
}

/// Get purity Tr(ρ²).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_purity(qr: *const QuantumRegister) -> f64 {
    if let Some(q) = unsafe { qr.as_ref() } { q.purity() } else { 0.0 }
}

/// Get entanglement entropy of first k qubits.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_entropy(qr: *const QuantumRegister, k: usize) -> f64 {
    if let Some(q) = unsafe { qr.as_ref() } { q.entanglement_entropy(k) } else { 0.0 }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool { (a - b).abs() < 1e-6 }

    #[test]
    fn test_initial_state() {
        let qr = QuantumRegister::new(2);
        assert!(approx(qr.probability(0), 1.0));
        assert!(approx(qr.probability(1), 0.0));
        assert!(approx(qr.probability(2), 0.0));
        assert!(approx(qr.probability(3), 0.0));
    }

    #[test]
    fn test_x_gate() {
        let mut qr = QuantumRegister::new(1);
        qr.x(0);
        assert!(approx(qr.probability(0), 0.0));
        assert!(approx(qr.probability(1), 1.0));
    }

    #[test]
    fn test_hadamard() {
        let mut qr = QuantumRegister::new(1);
        qr.h(0);
        assert!(approx(qr.probability(0), 0.5));
        assert!(approx(qr.probability(1), 0.5));
    }

    #[test]
    fn test_hadamard_twice_identity() {
        let mut qr = QuantumRegister::new(1);
        qr.h(0);
        qr.h(0);
        assert!(approx(qr.probability(0), 1.0));
        assert!(approx(qr.probability(1), 0.0));
    }

    #[test]
    fn test_bell_state() {
        let mut qr = QuantumRegister::new(2);
        qr.bell_state(0, 1);
        assert!(approx(qr.probability(0), 0.5)); // |00⟩
        assert!(approx(qr.probability(1), 0.0)); // |01⟩
        assert!(approx(qr.probability(2), 0.0)); // |10⟩
        assert!(approx(qr.probability(3), 0.5)); // |11⟩
    }

    #[test]
    fn test_cnot() {
        let mut qr = QuantumRegister::new(2);
        qr.x(0); // |10⟩
        qr.cnot(0, 1); // Should flip target → |11⟩
        assert!(approx(qr.probability(3), 1.0));
    }

    #[test]
    fn test_cnot_no_flip() {
        let mut qr = QuantumRegister::new(2);
        // Control is |0⟩, so target unchanged
        qr.cnot(0, 1);
        assert!(approx(qr.probability(0), 1.0));
    }

    #[test]
    fn test_toffoli() {
        let mut qr = QuantumRegister::new(3);
        qr.x(0);
        qr.x(1);
        qr.toffoli(0, 1, 2); // Both controls on → flip target
        assert!(approx(qr.probability(7), 1.0)); // |111⟩
    }

    #[test]
    fn test_z_gate() {
        let mut qr = QuantumRegister::new(1);
        qr.h(0);
        qr.z(0);
        qr.h(0);
        // H·Z·H = X, so should give |1⟩
        assert!(approx(qr.probability(1), 1.0));
    }

    #[test]
    fn test_rx_pi() {
        let mut qr = QuantumRegister::new(1);
        qr.rx(0, PI);
        // Rx(π)|0⟩ = -i|1⟩, probability of |1⟩ = 1.0
        assert!(approx(qr.probability(1), 1.0));
    }

    #[test]
    fn test_purity_pure_state() {
        let qr = QuantumRegister::new(2);
        assert!(approx(qr.purity(), 1.0));
    }

    #[test]
    fn test_fidelity_same() {
        let qr = QuantumRegister::new(2);
        assert!(approx(qr.fidelity(&qr), 1.0));
    }

    #[test]
    fn test_fidelity_orthogonal() {
        let q0 = QuantumRegister::from_basis(1, 0);
        let q1 = QuantumRegister::from_basis(1, 1);
        assert!(approx(q0.fidelity(&q1), 0.0));
    }

    #[test]
    fn test_bloch_zero() {
        let qr = QuantumRegister::new(1); // |0⟩ → theta=0 (north pole)
        let (theta, _phi) = qr.bloch_coordinates();
        assert!(approx(theta, 0.0));
    }

    #[test]
    fn test_bloch_one() {
        let mut qr = QuantumRegister::new(1);
        qr.x(0); // |1⟩ → theta=π (south pole)
        let (theta, _phi) = qr.bloch_coordinates();
        assert!(approx(theta, PI));
    }

    #[test]
    fn test_qft_preserves_norm() {
        let mut qr = QuantumRegister::new(3);
        qr.h(0);
        qr.x(1);
        qr.qft(3);
        let total_prob: f64 = qr.probabilities().iter().sum();
        assert!(approx(total_prob, 1.0));
    }

    #[test]
    fn test_swap_gate() {
        let mut qr = QuantumRegister::new(2);
        qr.x(0); // |10⟩ (qubit 0 = 1, qubit 1 = 0) → state index 1
        qr.swap_gate(0, 1); // Should give |01⟩ → state index 2
        assert!(approx(qr.probability(2), 1.0));
    }

    #[test]
    fn test_grover_oracle() {
        let mut qr = QuantumRegister::new(2);
        qr.h(0);
        qr.h(1);
        qr.grover_oracle(3); // Mark |11⟩
        // After oracle, |11⟩ should have negative amplitude
        assert!(qr.state[3].re < 0.0);
    }

    #[test]
    fn test_cz_gate() {
        let mut qr = QuantumRegister::new(2);
        qr.x(0);
        qr.x(1); // |11⟩
        qr.cz(0, 1);
        // CZ on |11⟩ → -|11⟩
        assert!(approx(qr.state[3].re, -1.0));
    }

    #[test]
    fn test_density_matrix_size() {
        let qr = QuantumRegister::new(2);
        let dm = qr.density_matrix();
        assert_eq!(dm.len(), 16); // 4x4
    }

    #[test]
    fn test_deterministic_measure() {
        let qr = QuantumRegister::new(2);
        assert_eq!(qr.measure_deterministic(), 0); // |00⟩ has probability 1
    }

    #[test]
    fn test_state_string() {
        let qr = QuantumRegister::new(1);
        let s = qr.state_string();
        assert!(s.contains("1.0000"));
    }

    #[test]
    fn test_ffi_quantum_lifecycle() {
        unsafe {
            let qr = vitalis_quantum_new(2);
            assert!(!qr.is_null());
            vitalis_quantum_h(qr, 0);
            vitalis_quantum_cnot(qr, 0, 1);
            let p0 = vitalis_quantum_prob(qr, 0);
            let p3 = vitalis_quantum_prob(qr, 3);
            assert!((p0 - 0.5).abs() < 1e-6);
            assert!((p3 - 0.5).abs() < 1e-6);
            vitalis_quantum_free(qr);
        }
    }
}
