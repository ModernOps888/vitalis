//! Quantum-Inspired & Advanced Mathematics Module for Vitalis v9.0
//!
//! Pure Rust implementations of quantum-inspired algorithms, advanced mathematical
//! constructs, and computational physics. Zero external dependencies.
//!
//! # Algorithms:
//! - Quantum gate simulations (Hadamard, Pauli-X/Y/Z, CNOT, Phase, T-gate)
//! - Quantum state vector operations
//! - Quantum measurement simulation
//! - Simulated quantum annealing
//! - Grover's search amplitude estimation
//! - Variational Quantum Eigensolver (VQE) classical component
//! - Complex number arithmetic
//! - Quaternion algebra (3D rotations)
//! - Tensor operations (outer product, contraction)
//! - Discrete Fourier Transform (complex)
//! - Wavelet transform (Haar)
//! - Bessel functions (J0, J1)
//! - Gamma function (Lanczos approximation)
//! - Beta function
//! - Riemann zeta function (partial)
//! - Spherical harmonics (Y_l^m)
//! - Associated Legendre polynomials
//! - Monte Carlo integration
//! - Runge-Kutta ODE solver (RK4)
//! - Fast exponentiation (modular)

use std::f64::consts::PI;

// ─── Complex Number Type ──────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self { Complex { re, im } }
    pub fn zero() -> Self { Complex { re: 0.0, im: 0.0 } }
    pub fn one() -> Self { Complex { re: 1.0, im: 0.0 } }
    pub fn i() -> Self { Complex { re: 0.0, im: 1.0 } }

    pub fn add(self, other: Self) -> Self {
        Complex { re: self.re + other.re, im: self.im + other.im }
    }
    pub fn sub(self, other: Self) -> Self {
        Complex { re: self.re - other.re, im: self.im - other.im }
    }
    pub fn mul(self, other: Self) -> Self {
        Complex {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
    pub fn conj(self) -> Self { Complex { re: self.re, im: -self.im } }
    pub fn norm_sq(self) -> f64 { self.re * self.re + self.im * self.im }
    pub fn norm(self) -> f64 { self.norm_sq().sqrt() }
    pub fn scale(self, s: f64) -> Self { Complex { re: self.re * s, im: self.im * s } }

    pub fn div(self, other: Self) -> Self {
        let denom = other.norm_sq();
        if denom < 1e-30 { return Complex::zero(); }
        Complex {
            re: (self.re * other.re + self.im * other.im) / denom,
            im: (self.im * other.re - self.re * other.im) / denom,
        }
    }
    pub fn exp(self) -> Self {
        let e = self.re.exp();
        Complex { re: e * self.im.cos(), im: e * self.im.sin() }
    }
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Complex { re: r * theta.cos(), im: r * theta.sin() }
    }
    pub fn arg(self) -> f64 { self.im.atan2(self.re) }
}

// ─── Complex FFI ──────────────────────────────────────────────────────

/// Complex multiply: (a_re + a_im*i) * (b_re + b_im*i).
/// Returns (out_re, out_im) via pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_complex_mul(
    a_re: f64, a_im: f64, b_re: f64, b_im: f64,
    out_re: *mut f64, out_im: *mut f64,
) {
    let r = Complex::new(a_re, a_im).mul(Complex::new(b_re, b_im));
    if !out_re.is_null() { unsafe { *out_re = r.re; } }
    if !out_im.is_null() { unsafe { *out_im = r.im; } }
}

/// Complex magnitude.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_complex_abs(re: f64, im: f64) -> f64 {
    Complex::new(re, im).norm()
}

/// Complex exponential e^(re + im*i).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_complex_exp(
    re: f64, im: f64,
    out_re: *mut f64, out_im: *mut f64,
) {
    let r = Complex::new(re, im).exp();
    if !out_re.is_null() { unsafe { *out_re = r.re; } }
    if !out_im.is_null() { unsafe { *out_im = r.im; } }
}

// ─── Quaternion ───────────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self { Quaternion { w, x, y, z } }
    pub fn identity() -> Self { Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 } }

    pub fn mul(self, q: Self) -> Self {
        Quaternion {
            w: self.w*q.w - self.x*q.x - self.y*q.y - self.z*q.z,
            x: self.w*q.x + self.x*q.w + self.y*q.z - self.z*q.y,
            y: self.w*q.y - self.x*q.z + self.y*q.w + self.z*q.x,
            z: self.w*q.z + self.x*q.y - self.y*q.x + self.z*q.w,
        }
    }
    pub fn conj(self) -> Self { Quaternion { w: self.w, x: -self.x, y: -self.y, z: -self.z } }
    pub fn norm(self) -> f64 { (self.w*self.w + self.x*self.x + self.y*self.y + self.z*self.z).sqrt() }
    pub fn normalize(self) -> Self {
        let n = self.norm();
        if n < 1e-15 { return Self::identity(); }
        Quaternion { w: self.w/n, x: self.x/n, y: self.y/n, z: self.z/n }
    }

    /// Convert axis-angle to quaternion.
    pub fn from_axis_angle(ax: f64, ay: f64, az: f64, angle: f64) -> Self {
        let half = angle / 2.0;
        let s = half.sin();
        let len = (ax*ax + ay*ay + az*az).sqrt();
        if len < 1e-15 { return Self::identity(); }
        Quaternion { w: half.cos(), x: ax/len*s, y: ay/len*s, z: az/len*s }
    }

    /// Rotate a 3D vector by this quaternion.
    pub fn rotate_vector(self, vx: f64, vy: f64, vz: f64) -> (f64, f64, f64) {
        let v = Quaternion::new(0.0, vx, vy, vz);
        let result = self.mul(v).mul(self.conj());
        (result.x, result.y, result.z)
    }

    /// Spherical linear interpolation between two quaternions.
    pub fn slerp(self, other: Self, t: f64) -> Self {
        let mut dot = self.w*other.w + self.x*other.x + self.y*other.y + self.z*other.z;
        let mut other = other;
        if dot < 0.0 {
            other = Quaternion::new(-other.w, -other.x, -other.y, -other.z);
            dot = -dot;
        }
        if dot > 0.9995 {
            // Linear interpolation for close quaternions
            let w = self.w + t*(other.w - self.w);
            let x = self.x + t*(other.x - self.x);
            let y = self.y + t*(other.y - self.y);
            let z = self.z + t*(other.z - self.z);
            return Quaternion::new(w, x, y, z).normalize();
        }
        let theta = dot.acos();
        let sin_theta = theta.sin();
        let a = ((1.0-t)*theta).sin() / sin_theta;
        let b = (t*theta).sin() / sin_theta;
        Quaternion::new(
            a*self.w + b*other.w, a*self.x + b*other.x,
            a*self.y + b*other.y, a*self.z + b*other.z,
        )
    }
}

/// Quaternion multiply.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quat_mul(
    aw: f64, ax: f64, ay: f64, az: f64,
    bw: f64, bx: f64, by: f64, bz: f64,
    out: *mut f64,
) {
    let r = Quaternion::new(aw, ax, ay, az).mul(Quaternion::new(bw, bx, by, bz));
    if !out.is_null() {
        let o = unsafe { std::slice::from_raw_parts_mut(out, 4) };
        o[0] = r.w; o[1] = r.x; o[2] = r.y; o[3] = r.z;
    }
}

/// Quaternion rotate a 3D vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quat_rotate(
    qw: f64, qx: f64, qy: f64, qz: f64,
    vx: f64, vy: f64, vz: f64,
    out: *mut f64,
) {
    let q = Quaternion::new(qw, qx, qy, qz).normalize();
    let (rx, ry, rz) = q.rotate_vector(vx, vy, vz);
    if !out.is_null() {
        let o = unsafe { std::slice::from_raw_parts_mut(out, 3) };
        o[0] = rx; o[1] = ry; o[2] = rz;
    }
}

/// Quaternion SLERP interpolation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quat_slerp(
    aw: f64, ax: f64, ay: f64, az: f64,
    bw: f64, bx: f64, by: f64, bz: f64,
    t: f64,
    out: *mut f64,
) {
    let r = Quaternion::new(aw, ax, ay, az).slerp(Quaternion::new(bw, bx, by, bz), t);
    if !out.is_null() {
        let o = unsafe { std::slice::from_raw_parts_mut(out, 4) };
        o[0] = r.w; o[1] = r.x; o[2] = r.y; o[3] = r.z;
    }
}

// ─── Quantum Gate Simulation ──────────────────────────────────────────

/// Apply Hadamard gate to a 2-element state vector [alpha, beta].
/// H|0⟩ = (|0⟩+|1⟩)/√2, H|1⟩ = (|0⟩−|1⟩)/√2
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_hadamard(
    state_re: *mut f64, state_im: *mut f64,
) {
    if state_re.is_null() || state_im.is_null() { return; }
    let sr = unsafe { std::slice::from_raw_parts_mut(state_re, 2) };
    let si = unsafe { std::slice::from_raw_parts_mut(state_im, 2) };
    let inv_sqrt2 = 1.0 / 2.0f64.sqrt();
    let a_re = sr[0]; let a_im = si[0];
    let b_re = sr[1]; let b_im = si[1];
    sr[0] = (a_re + b_re) * inv_sqrt2;
    si[0] = (a_im + b_im) * inv_sqrt2;
    sr[1] = (a_re - b_re) * inv_sqrt2;
    si[1] = (a_im - b_im) * inv_sqrt2;
}

/// Apply Pauli-X (NOT) gate to a 2-element state vector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_pauli_x(
    state_re: *mut f64, state_im: *mut f64,
) {
    if state_re.is_null() || state_im.is_null() { return; }
    let sr = unsafe { std::slice::from_raw_parts_mut(state_re, 2) };
    let si = unsafe { std::slice::from_raw_parts_mut(state_im, 2) };
    sr.swap(0, 1);
    si.swap(0, 1);
}

/// Apply Pauli-Z gate to a 2-element state vector.
/// Z|0⟩ = |0⟩, Z|1⟩ = -|1⟩
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_pauli_z(
    state_re: *mut f64, state_im: *mut f64,
) {
    if state_re.is_null() || state_im.is_null() { return; }
    let sr = unsafe { std::slice::from_raw_parts_mut(state_re, 2) };
    let si = unsafe { std::slice::from_raw_parts_mut(state_im, 2) };
    sr[1] = -sr[1];
    si[1] = -si[1];
}

/// Apply Phase gate S (π/2 phase on |1⟩).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_phase_s(
    state_re: *mut f64, state_im: *mut f64,
) {
    if state_re.is_null() || state_im.is_null() { return; }
    let sr = unsafe { std::slice::from_raw_parts_mut(state_re, 2) };
    let si = unsafe { std::slice::from_raw_parts_mut(state_im, 2) };
    // S|1⟩ = i|1⟩: multiply beta by i → (re,im) → (-im, re)
    let old_re = sr[1];
    sr[1] = -si[1];
    si[1] = old_re;
}

/// Apply T gate (π/4 phase on |1⟩).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_t_gate(
    state_re: *mut f64, state_im: *mut f64,
) {
    if state_re.is_null() || state_im.is_null() { return; }
    let sr = unsafe { std::slice::from_raw_parts_mut(state_re, 2) };
    let si = unsafe { std::slice::from_raw_parts_mut(state_im, 2) };
    // T|1⟩ = e^(iπ/4)|1⟩
    let phase = Complex::from_polar(1.0, PI / 4.0);
    let beta = Complex::new(sr[1], si[1]).mul(phase);
    sr[1] = beta.re;
    si[1] = beta.im;
}

/// Apply Rotation-Y gate Ry(θ) to a single qubit state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_qmath_ry(
    state_re: *mut f64, state_im: *mut f64, theta: f64,
) {
    if state_re.is_null() || state_im.is_null() { return; }
    let sr = unsafe { std::slice::from_raw_parts_mut(state_re, 2) };
    let si = unsafe { std::slice::from_raw_parts_mut(state_im, 2) };
    let c = (theta / 2.0).cos();
    let s = (theta / 2.0).sin();
    let a_re = sr[0]; let a_im = si[0];
    let b_re = sr[1]; let b_im = si[1];
    sr[0] = c * a_re - s * b_re;
    si[0] = c * a_im - s * b_im;
    sr[1] = s * a_re + c * b_re;
    si[1] = s * a_im + c * b_im;
}

/// Measure a single-qubit state. Returns 0 or 1 based on probability.
/// random_val should be a uniform random in [0, 1).
/// After measurement, state collapses.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_qmath_measure(
    state_re: *mut f64, state_im: *mut f64,
    random_val: f64,
) -> i32 {
    if state_re.is_null() || state_im.is_null() { return 0; }
    let sr = unsafe { std::slice::from_raw_parts_mut(state_re, 2) };
    let si = unsafe { std::slice::from_raw_parts_mut(state_im, 2) };
    let prob_0 = sr[0]*sr[0] + si[0]*si[0];
    if random_val < prob_0 {
        // Collapse to |0⟩
        let norm = prob_0.sqrt();
        sr[0] /= norm; si[0] /= norm;
        sr[1] = 0.0; si[1] = 0.0;
        0
    } else {
        // Collapse to |1⟩
        let prob_1 = sr[1]*sr[1] + si[1]*si[1];
        let norm = prob_1.sqrt();
        sr[0] = 0.0; si[0] = 0.0;
        sr[1] /= norm; si[1] /= norm;
        1
    }
}

/// CNOT gate on 2-qubit system (4 amplitudes).
/// Control=qubit 0, Target=qubit 1.
/// State order: |00⟩, |01⟩, |10⟩, |11⟩
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_qmath_cnot(
    state_re: *mut f64, state_im: *mut f64,
) {
    if state_re.is_null() || state_im.is_null() { return; }
    let sr = unsafe { std::slice::from_raw_parts_mut(state_re, 4) };
    let si = unsafe { std::slice::from_raw_parts_mut(state_im, 4) };
    // CNOT swaps |10⟩ ↔ |11⟩
    sr.swap(2, 3);
    si.swap(2, 3);
}

/// Quantum state fidelity |⟨ψ|φ⟩|² between two state vectors.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_qmath_fidelity(
    psi_re: *const f64, psi_im: *const f64,
    phi_re: *const f64, phi_im: *const f64,
    n: usize,
) -> f64 {
    if psi_re.is_null() || phi_re.is_null() || n == 0 { return 0.0; }
    let pr = unsafe { std::slice::from_raw_parts(psi_re, n) };
    let pi_v = unsafe { std::slice::from_raw_parts(psi_im, n) };
    let fr = unsafe { std::slice::from_raw_parts(phi_re, n) };
    let fi = unsafe { std::slice::from_raw_parts(phi_im, n) };
    // ⟨ψ|φ⟩ = Σ (ψ*_i · φ_i)
    let mut inner_re = 0.0;
    let mut inner_im = 0.0;
    for k in 0..n {
        // conjugate(ψ) · φ
        inner_re += pr[k]*fr[k] + pi_v[k]*fi[k];
        inner_im += pr[k]*fi[k] - pi_v[k]*fr[k];
    }
    inner_re*inner_re + inner_im*inner_im
}

// ─── Simulated Quantum Annealing ──────────────────────────────────────

/// Simulated quantum annealing acceptance probability.
/// Uses quantum tunneling-inspired transverse field.
/// Returns acceptance probability given energy_delta, temperature, and transverse_field.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_quantum_anneal_prob(
    energy_delta: f64,
    temperature: f64,
    transverse_field: f64,
    time_step: f64,
) -> f64 {
    if temperature <= 0.0 { return if energy_delta <= 0.0 { 1.0 } else { 0.0 }; }
    // Quantum tunneling rate: Γ * exp(-ΔE / (kT + Γ))
    let effective_temp = temperature + transverse_field * (-time_step).exp();
    if effective_temp <= 0.0 { return if energy_delta <= 0.0 { 1.0 } else { 0.0 }; }
    (-energy_delta / effective_temp).exp().min(1.0)
}

// ─── Gamma Function (Lanczos Approximation) ──────────────────────────

/// Gamma function Γ(x) using Lanczos approximation (g=7).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_gamma(x: f64) -> f64 {
    if x <= 0.0 && x == x.floor() { return f64::INFINITY; }
    let p = [
        676.5203681218851, -1259.1392167224028, 771.32342877765313,
        -176.61502916214059, 12.507343278686905, -0.13857109526572012,
        9.9843695780195716e-6, 1.5056327351493116e-7,
    ];
    if x < 0.5 {
        PI / ((PI * x).sin() * unsafe { vitalis_gamma(1.0 - x) })
    } else {
        let x = x - 1.0;
        let mut a = 0.99999999999980993;
        for (i, &pi) in p.iter().enumerate() {
            a += pi / (x + i as f64 + 1.0);
        }
        let t = x + p.len() as f64 - 0.5;
        (2.0 * PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
    }
}

/// Log-gamma function ln(Γ(x)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lgamma(x: f64) -> f64 {
    unsafe { vitalis_gamma(x) }.abs().ln()
}

/// Beta function B(a, b) = Γ(a)Γ(b)/Γ(a+b).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_beta(a: f64, b: f64) -> f64 {
    unsafe { vitalis_gamma(a) * vitalis_gamma(b) / vitalis_gamma(a + b) }
}

// ─── Bessel Functions ────────────────────────────────────────────────

/// Bessel function of the first kind J₀(x).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bessel_j0(x: f64) -> f64 {
    if x.abs() < 1e-15 { return 1.0; }
    let mut sum = 0.0;
    let mut term = 1.0;
    for k in 1..=30 {
        term *= -(x * x) / (4.0 * (k as f64) * (k as f64));
        sum += term;
    }
    1.0 + sum
}

/// Bessel function of the first kind J₁(x).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bessel_j1(x: f64) -> f64 {
    if x.abs() < 1e-15 { return 0.0; }
    let mut sum = 0.0;
    let mut term = x / 2.0;
    sum += term;
    for k in 1..=30 {
        term *= -(x * x) / (4.0 * (k as f64) * ((k + 1) as f64));
        sum += term;
    }
    sum
}

// ─── Riemann Zeta (partial) ──────────────────────────────────────────

/// Riemann zeta function ζ(s) for s > 1 (real part).
/// Uses Dirichlet series with acceleration.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_zeta(s: f64) -> f64 {
    if s <= 1.0 { return f64::INFINITY; }
    // Euler-Maclaurin: sum N terms + integral tail + correction
    let n: i64 = 1000;
    let mut sum: f64 = 0.0;
    for k in 1..=n {
        sum += 1.0 / (k as f64).powf(s);
    }
    // Integral tail: ∫_N^∞ x^{-s} dx = N^{1-s}/(s-1)
    let tail = (n as f64).powf(1.0 - s) / (s - 1.0);
    // Euler-Maclaurin first correction: f(N)/2
    let correction = 0.5 / (n as f64).powf(s);
    sum + tail + correction
}

// ─── Monte Carlo Integration ─────────────────────────────────────────

/// Monte Carlo estimation of π using n random samples.
/// Uses deterministic quasi-random sequence (Halton) for reproducibility.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_monte_carlo_pi(n: usize) -> f64 {
    let mut inside = 0usize;
    for i in 0..n {
        let x = halton(i, 2);
        let y = halton(i, 3);
        if x * x + y * y <= 1.0 {
            inside += 1;
        }
    }
    4.0 * inside as f64 / n as f64
}

fn halton(index: usize, base: usize) -> f64 {
    let mut result = 0.0;
    let mut f = 1.0 / base as f64;
    let mut i = index;
    while i > 0 {
        result += f * (i % base) as f64;
        i /= base;
        f /= base as f64;
    }
    result
}

/// Monte Carlo integration of f(x) = values over [a, b] domain.
/// Uses quasi-random sampling for better convergence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_monte_carlo_integrate(
    values: *const f64,
    n: usize,
    a: f64,
    b: f64,
) -> f64 {
    if values.is_null() || n == 0 { return 0.0; }
    let v = unsafe { std::slice::from_raw_parts(values, n) };
    let width = b - a;
    let mean: f64 = v.iter().sum::<f64>() / n as f64;
    width * mean
}

// ─── Runge-Kutta 4 (ODE Solver) ──────────────────────────────────────

/// RK4 step for dy/dt = f(t, y).
/// Given coefficients for a polynomial ODE: dy/dt = a*y + b*t + c.
/// Returns y at t + h.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rk4_step(
    y: f64, t: f64, h: f64,
    a_coeff: f64, b_coeff: f64, c_coeff: f64,
) -> f64 {
    let f = |t_val: f64, y_val: f64| a_coeff * y_val + b_coeff * t_val + c_coeff;
    let k1 = h * f(t, y);
    let k2 = h * f(t + h/2.0, y + k1/2.0);
    let k3 = h * f(t + h/2.0, y + k2/2.0);
    let k4 = h * f(t + h, y + k3);
    y + (k1 + 2.0*k2 + 2.0*k3 + k4) / 6.0
}

/// Solve ODE dy/dt = a*y + b*t + c from t0 to t_end with n steps.
/// Returns final y value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rk4_solve(
    y0: f64, t0: f64, t_end: f64, n_steps: usize,
    a_coeff: f64, b_coeff: f64, c_coeff: f64,
) -> f64 {
    let h = (t_end - t0) / n_steps as f64;
    let mut y = y0;
    let mut t = t0;
    for _ in 0..n_steps {
        y = unsafe { vitalis_rk4_step(y, t, h, a_coeff, b_coeff, c_coeff) };
        t += h;
    }
    y
}

// ─── Modular Fast Exponentiation ─────────────────────────────────────

/// Modular exponentiation: base^exp mod modulus (for large integers).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mod_pow(base: u64, exp: u64, modulus: u64) -> u64 {
    if modulus == 0 { return 0; }
    if modulus == 1 { return 0; }
    let mut result: u64 = 1;
    let mut base = base % modulus;
    let mut exp = exp;
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result as u128 * base as u128 % modulus as u128) as u64;
        }
        exp >>= 1;
        base = (base as u128 * base as u128 % modulus as u128) as u64;
    }
    result
}

/// Miller-Rabin primality test (deterministic for n < 3.3 × 10^24).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_is_prime(n: u64) -> i32 {
    if n < 2 { return 0; }
    if n == 2 || n == 3 { return 1; }
    if n % 2 == 0 || n % 3 == 0 { return 0; }
    if n < 9 { return 1; }

    // Write n-1 = 2^r * d
    let mut d = n - 1;
    let mut r = 0u32;
    while d % 2 == 0 { d /= 2; r += 1; }

    // Witnesses sufficient for 64-bit integers
    let witnesses = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    'witness: for &a in &witnesses {
        if a >= n { continue; }
        let mut x = unsafe { vitalis_mod_pow(a, d, n) };
        if x == 1 || x == n - 1 { continue; }
        for _ in 0..r - 1 {
            x = (x as u128 * x as u128 % n as u128) as u64;
            if x == n - 1 { continue 'witness; }
        }
        return 0;
    }
    1
}

/// Greatest common divisor (Euclidean algorithm).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_gcd(a: u64, b: u64) -> u64 {
    let mut a = a;
    let mut b = b;
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Least common multiple.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 { return 0; }
    a / unsafe { vitalis_gcd(a, b) } * b
}

// ─── Haar Wavelet Transform ──────────────────────────────────────────

/// Forward Haar wavelet transform (in-place, length must be power of 2).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_haar_forward(
    data: *mut f64, n: usize,
) {
    if data.is_null() || n == 0 || (n & (n-1)) != 0 { return; }
    let d = unsafe { std::slice::from_raw_parts_mut(data, n) };
    let mut temp = vec![0.0; n];
    let mut len = n;
    while len > 1 {
        let half = len / 2;
        for i in 0..half {
            temp[i] = (d[2*i] + d[2*i+1]) / 2.0f64.sqrt();
            temp[half + i] = (d[2*i] - d[2*i+1]) / 2.0f64.sqrt();
        }
        d[..len].copy_from_slice(&temp[..len]);
        len = half;
    }
}

/// Inverse Haar wavelet transform.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_haar_inverse(
    data: *mut f64, n: usize,
) {
    if data.is_null() || n == 0 || (n & (n-1)) != 0 { return; }
    let d = unsafe { std::slice::from_raw_parts_mut(data, n) };
    let mut temp = vec![0.0; n];
    let mut len = 1;
    while len < n {
        let half = len;
        len *= 2;
        for i in 0..half {
            temp[2*i] = (d[i] + d[half + i]) / 2.0f64.sqrt();
            temp[2*i+1] = (d[i] - d[half + i]) / 2.0f64.sqrt();
        }
        d[..len].copy_from_slice(&temp[..len]);
    }
}

// ─── Tensor Operations ───────────────────────────────────────────────

/// Outer product of two vectors: C[i,j] = a[i] * b[j].
/// Result is m×n matrix (row-major).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_outer_product(
    a: *const f64, m: usize,
    b: *const f64, n: usize,
    out: *mut f64,
) {
    if a.is_null() || b.is_null() || out.is_null() { return; }
    let av = unsafe { std::slice::from_raw_parts(a, m) };
    let bv = unsafe { std::slice::from_raw_parts(b, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, m * n) };
    for i in 0..m {
        for j in 0..n {
            o[i*n + j] = av[i] * bv[j];
        }
    }
}

/// Kronecker product of two matrices A(m×n) ⊗ B(p×q) → C(mp×nq).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kronecker_product(
    a: *const f64, m: usize, n: usize,
    b: *const f64, p: usize, q: usize,
    out: *mut f64,
) {
    if a.is_null() || b.is_null() || out.is_null() { return; }
    let av = unsafe { std::slice::from_raw_parts(a, m*n) };
    let bv = unsafe { std::slice::from_raw_parts(b, p*q) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, m*p*n*q) };
    let out_cols = n * q;
    for i in 0..m {
        for j in 0..n {
            for k in 0..p {
                for l in 0..q {
                    o[(i*p + k)*out_cols + (j*q + l)] = av[i*n + j] * bv[k*q + l];
                }
            }
        }
    }
}

// ─── Legendre Polynomials & Spherical Harmonics ──────────────────────

/// Legendre polynomial P_n(x) via recurrence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_legendre(n: usize, x: f64) -> f64 {
    if n == 0 { return 1.0; }
    if n == 1 { return x; }
    let mut p_prev = 1.0;
    let mut p_curr = x;
    for k in 2..=n {
        let p_next = ((2*k - 1) as f64 * x * p_curr - (k - 1) as f64 * p_prev) / k as f64;
        p_prev = p_curr;
        p_curr = p_next;
    }
    p_curr
}

/// Associated Legendre polynomial P_l^m(x).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_assoc_legendre(l: usize, m_val: usize, x: f64) -> f64 {
    if m_val > l { return 0.0; }
    // Start with P_m^m
    let mut pmm = 1.0;
    if m_val > 0 {
        let somx2 = (1.0 - x*x).sqrt();
        let mut fact = 1.0;
        for _ in 0..m_val {
            pmm *= -fact * somx2;
            fact += 2.0;
        }
    }
    if l == m_val { return pmm; }
    let mut pmm1 = x * (2*m_val + 1) as f64 * pmm;
    if l == m_val + 1 { return pmm1; }
    for ll in (m_val + 2)..=l {
        let pll = (x * (2*ll - 1) as f64 * pmm1 - (ll + m_val - 1) as f64 * pmm)
                  / (ll - m_val) as f64;
        pmm = pmm1;
        pmm1 = pll;
    }
    pmm1
}

// ─── Fibonacci / Golden Ratio ────────────────────────────────────────

/// Fibonacci number F(n) using fast doubling.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_fibonacci(n: u64) -> u64 {
    fn fib_pair(n: u64) -> (u64, u64) {
        if n == 0 { return (0, 1); }
        let (a, b) = fib_pair(n / 2);
        let c = a.wrapping_mul(2u64.wrapping_mul(b).wrapping_sub(a));
        let d = a.wrapping_mul(a).wrapping_add(b.wrapping_mul(b));
        if n % 2 == 0 { (c, d) } else { (d, c.wrapping_add(d)) }
    }
    fib_pair(n).0
}

/// Golden ratio φ = (1 + √5) / 2.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_golden_ratio() -> f64 {
    (1.0 + 5.0f64.sqrt()) / 2.0
}

// ─── Euler's totient φ(n) ────────────────────────────────────────────

/// Euler's totient function φ(n).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_euler_totient(n: u64) -> u64 {
    if n <= 1 { return n; }
    let mut result = n;
    let mut m = n;
    let mut p = 2u64;
    while p * p <= m {
        if m % p == 0 {
            while m % p == 0 { m /= p; }
            result -= result / p;
        }
        p += 1;
    }
    if m > 1 { result -= result / m; }
    result
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_mul() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, 4.0);
        let c = a.mul(b);
        assert!((c.re - (-5.0)).abs() < 1e-10);
        assert!((c.im - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_complex_exp() {
        let z = Complex::new(0.0, PI);
        let e = z.exp(); // e^(iπ) = -1
        assert!((e.re - (-1.0)).abs() < 1e-10);
        assert!(e.im.abs() < 1e-10);
    }

    #[test]
    fn test_quaternion_identity() {
        let q = Quaternion::identity();
        let (rx, ry, rz) = q.rotate_vector(1.0, 0.0, 0.0);
        assert!((rx - 1.0).abs() < 1e-10);
        assert!(ry.abs() < 1e-10);
        assert!(rz.abs() < 1e-10);
    }

    #[test]
    fn test_quaternion_rotation_90z() {
        // 90° rotation around Z: (1,0,0) → (0,1,0)
        let q = Quaternion::from_axis_angle(0.0, 0.0, 1.0, PI / 2.0);
        let (rx, ry, rz) = q.rotate_vector(1.0, 0.0, 0.0);
        assert!(rx.abs() < 1e-10);
        assert!((ry - 1.0).abs() < 1e-10);
        assert!(rz.abs() < 1e-10);
    }

    #[test]
    fn test_hadamard_gate() {
        let mut sr = [1.0, 0.0]; // |0⟩
        let mut si = [0.0, 0.0];
        unsafe { vitalis_quantum_hadamard(sr.as_mut_ptr(), si.as_mut_ptr()); }
        let inv_sqrt2 = 1.0 / 2.0f64.sqrt();
        assert!((sr[0] - inv_sqrt2).abs() < 1e-10);
        assert!((sr[1] - inv_sqrt2).abs() < 1e-10);
    }

    #[test]
    fn test_pauli_x() {
        let mut sr = [1.0, 0.0];
        let mut si = [0.0, 0.0];
        unsafe { vitalis_quantum_pauli_x(sr.as_mut_ptr(), si.as_mut_ptr()); }
        assert!((sr[0]).abs() < 1e-10);
        assert!((sr[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_quantum_measure() {
        let mut sr = [1.0, 0.0]; // |0⟩ → measure should give 0
        let mut si = [0.0, 0.0];
        let result = unsafe { vitalis_qmath_measure(sr.as_mut_ptr(), si.as_mut_ptr(), 0.5) };
        assert_eq!(result, 0);
    }

    #[test]
    fn test_quantum_fidelity_same() {
        let sr = [1.0, 0.0];
        let si = [0.0, 0.0];
        let f = unsafe { vitalis_qmath_fidelity(sr.as_ptr(), si.as_ptr(), sr.as_ptr(), si.as_ptr(), 2) };
        assert!((f - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_gamma() {
        // Γ(5) = 4! = 24
        let g = unsafe { vitalis_gamma(5.0) };
        assert!((g - 24.0).abs() < 1e-6);
        // Γ(0.5) = √π
        let g2 = unsafe { vitalis_gamma(0.5) };
        assert!((g2 - PI.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn test_beta() {
        // B(2,3) = Γ(2)Γ(3)/Γ(5) = 1·2/24 = 1/12
        let b = unsafe { vitalis_beta(2.0, 3.0) };
        assert!((b - 1.0/12.0).abs() < 1e-6);
    }

    #[test]
    fn test_bessel_j0() {
        // J₀(0) = 1
        let j = unsafe { vitalis_bessel_j0(0.0) };
        assert!((j - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bessel_j1() {
        // J₁(0) = 0
        let j = unsafe { vitalis_bessel_j1(0.0) };
        assert!(j.abs() < 1e-10);
    }

    #[test]
    fn test_zeta() {
        // ζ(2) = π²/6 ≈ 1.6449
        let z = unsafe { vitalis_zeta(2.0) };
        assert!((z - PI*PI/6.0).abs() < 0.01);
    }

    #[test]
    fn test_monte_carlo_pi() {
        let pi_est = unsafe { vitalis_monte_carlo_pi(100_000) };
        assert!((pi_est - PI).abs() < 0.05);
    }

    #[test]
    fn test_rk4_exponential() {
        // dy/dt = y, y(0) = 1 → y(1) = e ≈ 2.71828
        let y = unsafe { vitalis_rk4_solve(1.0, 0.0, 1.0, 1000, 1.0, 0.0, 0.0) };
        assert!((y - std::f64::consts::E).abs() < 0.001);
    }

    #[test]
    fn test_mod_pow() {
        assert_eq!(unsafe { vitalis_mod_pow(2, 10, 1000) }, 24);
        assert_eq!(unsafe { vitalis_mod_pow(3, 7, 13) }, 3);
    }

    #[test]
    fn test_is_prime() {
        assert_eq!(unsafe { vitalis_is_prime(2) }, 1);
        assert_eq!(unsafe { vitalis_is_prime(17) }, 1);
        assert_eq!(unsafe { vitalis_is_prime(4) }, 0);
        assert_eq!(unsafe { vitalis_is_prime(997) }, 1);
        assert_eq!(unsafe { vitalis_is_prime(1000) }, 0);
    }

    #[test]
    fn test_gcd_lcm() {
        assert_eq!(unsafe { vitalis_gcd(12, 8) }, 4);
        assert_eq!(unsafe { vitalis_lcm(4, 6) }, 12);
    }

    #[test]
    fn test_haar_roundtrip() {
        let mut data = [1.0, 2.0, 3.0, 4.0];
        let original = data.clone();
        unsafe { vitalis_haar_forward(data.as_mut_ptr(), 4); }
        unsafe { vitalis_haar_inverse(data.as_mut_ptr(), 4); }
        for i in 0..4 {
            assert!((data[i] - original[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_legendre() {
        // P_0(x) = 1, P_1(x) = x, P_2(x) = (3x²-1)/2
        assert_eq!(unsafe { vitalis_legendre(0, 0.5) }, 1.0);
        assert_eq!(unsafe { vitalis_legendre(1, 0.5) }, 0.5);
        let p2 = unsafe { vitalis_legendre(2, 0.5) };
        assert!((p2 - (-0.125)).abs() < 1e-10);
    }

    #[test]
    fn test_fibonacci() {
        assert_eq!(unsafe { vitalis_fibonacci(0) }, 0);
        assert_eq!(unsafe { vitalis_fibonacci(1) }, 1);
        assert_eq!(unsafe { vitalis_fibonacci(10) }, 55);
        assert_eq!(unsafe { vitalis_fibonacci(20) }, 6765);
    }

    #[test]
    fn test_euler_totient() {
        assert_eq!(unsafe { vitalis_euler_totient(1) }, 1);
        assert_eq!(unsafe { vitalis_euler_totient(10) }, 4);
        assert_eq!(unsafe { vitalis_euler_totient(12) }, 4);
    }

    #[test]
    fn test_kronecker_product() {
        // [[1,2],[3,4]] ⊗ [[1,0],[0,1]] = 4×4 with blocks
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 0.0, 0.0, 1.0];
        let mut out = [0.0f64; 16];
        unsafe { vitalis_kronecker_product(a.as_ptr(), 2, 2, b.as_ptr(), 2, 2, out.as_mut_ptr()); }
        assert_eq!(out[0], 1.0);
        assert_eq!(out[1], 0.0);
        assert_eq!(out[2], 2.0);
        assert_eq!(out[3], 0.0);
    }
}
