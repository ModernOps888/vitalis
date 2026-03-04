//! Differentiable Programming — forward-mode AD, dual numbers, shape types, custom VJP.
//!
//! Provides language-level differentiable programming primitives:
//! dual numbers for forward-mode AD, shape-checked tensor types,
//! differentiable control flow, and custom vector-Jacobian product rules.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Dual Numbers (Forward-Mode AD) ──────────────────────────────────────

/// Dual number: value + ε·derivative (forward-mode automatic differentiation).
/// For f(x), dual(x, 1.0) propagates derivative through computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dual {
    pub val: f64,
    pub dot: f64, // tangent (derivative)
}

impl Dual {
    pub fn new(val: f64, dot: f64) -> Self { Dual { val, dot } }
    pub fn constant(val: f64) -> Self { Dual { val, dot: 0.0 } }
    pub fn variable(val: f64) -> Self { Dual { val, dot: 1.0 } }

    pub fn add(self, rhs: Dual) -> Dual {
        Dual { val: self.val + rhs.val, dot: self.dot + rhs.dot }
    }
    pub fn sub(self, rhs: Dual) -> Dual {
        Dual { val: self.val - rhs.val, dot: self.dot - rhs.dot }
    }
    pub fn mul(self, rhs: Dual) -> Dual {
        Dual { val: self.val * rhs.val, dot: self.val * rhs.dot + self.dot * rhs.val }
    }
    pub fn div(self, rhs: Dual) -> Dual {
        Dual {
            val: self.val / rhs.val,
            dot: (self.dot * rhs.val - self.val * rhs.dot) / (rhs.val * rhs.val),
        }
    }
    pub fn neg(self) -> Dual { Dual { val: -self.val, dot: -self.dot } }

    pub fn sin(self) -> Dual { Dual { val: self.val.sin(), dot: self.dot * self.val.cos() } }
    pub fn cos(self) -> Dual { Dual { val: self.val.cos(), dot: -self.dot * self.val.sin() } }
    pub fn exp(self) -> Dual { let e = self.val.exp(); Dual { val: e, dot: self.dot * e } }
    pub fn ln(self) -> Dual { Dual { val: self.val.ln(), dot: self.dot / self.val } }
    pub fn sqrt(self) -> Dual {
        let s = self.val.sqrt();
        Dual { val: s, dot: if s > 1e-15 { self.dot / (2.0 * s) } else { 0.0 } }
    }
    pub fn pow(self, n: f64) -> Dual {
        Dual { val: self.val.powf(n), dot: self.dot * n * self.val.powf(n - 1.0) }
    }
    pub fn abs(self) -> Dual {
        Dual { val: self.val.abs(), dot: if self.val >= 0.0 { self.dot } else { -self.dot } }
    }
    pub fn tanh(self) -> Dual {
        let t = self.val.tanh();
        Dual { val: t, dot: self.dot * (1.0 - t * t) }
    }
    pub fn sigmoid(self) -> Dual {
        let s = 1.0 / (1.0 + (-self.val).exp());
        Dual { val: s, dot: self.dot * s * (1.0 - s) }
    }
    pub fn relu(self) -> Dual {
        if self.val > 0.0 { self } else { Dual { val: 0.0, dot: 0.0 } }
    }
    pub fn max(self, rhs: Dual) -> Dual {
        if self.val >= rhs.val { self } else { rhs }
    }
    pub fn min(self, rhs: Dual) -> Dual {
        if self.val <= rhs.val { self } else { rhs }
    }
}

// ── Forward-mode Differentiation ────────────────────────────────────────

/// Compute derivative of a function at a point using dual numbers.
pub fn forward_derivative(f: impl Fn(Dual) -> Dual, x: f64) -> f64 {
    f(Dual::variable(x)).dot
}

/// Compute Jacobian-vector product (JVP) for multi-input functions.
pub fn jvp(f: impl Fn(&[Dual]) -> Vec<Dual>, inputs: &[f64], tangents: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let duals: Vec<Dual> = inputs.iter().zip(tangents.iter())
        .map(|(&v, &t)| Dual::new(v, t))
        .collect();
    let results = f(&duals);
    let primals = results.iter().map(|d| d.val).collect();
    let tangent_out = results.iter().map(|d| d.dot).collect();
    (primals, tangent_out)
}

/// Compute full Jacobian matrix via forward-mode (one pass per input dimension).
pub fn forward_jacobian(f: impl Fn(&[Dual]) -> Vec<Dual>, inputs: &[f64]) -> Vec<Vec<f64>> {
    let n_in = inputs.len();
    let mut jacobian = Vec::new();

    for i in 0..n_in {
        let mut tangents = vec![0.0; n_in];
        tangents[i] = 1.0;
        let (_, col) = jvp(&f, inputs, &tangents);
        jacobian.push(col);
    }
    // Transpose: jacobian[i][j] = ∂f_j/∂x_i → we want jacobian[j][i]
    if jacobian.is_empty() { return vec![]; }
    let n_out = jacobian[0].len();
    let mut result = vec![vec![0.0; n_in]; n_out];
    for i in 0..n_in {
        for j in 0..n_out {
            result[j][i] = jacobian[i][j];
        }
    }
    result
}

/// Compute Hessian via forward-over-forward (nested dual numbers approximation).
/// Uses finite differences on the dual tangent for simplicity.
pub fn hessian_diagonal(f: impl Fn(Dual) -> Dual, x: f64) -> f64 {
    let h = 1e-5;
    let df_plus = forward_derivative(&f, x + h);
    let df_minus = forward_derivative(&f, x - h);
    (df_plus - df_minus) / (2.0 * h)
}

// ── Shape Types ─────────────────────────────────────────────────────────

/// Compile-time shape descriptor for tensor type checking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapeType {
    pub dims: Vec<ShapeDim>,
}

/// A dimension can be concrete, symbolic (named), or dynamic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShapeDim {
    /// Fixed dimension size.
    Fixed(usize),
    /// Named symbolic dimension (e.g., "B" for batch).
    Symbolic(String),
    /// Dynamic (unknown at compile time).
    Dynamic,
}

impl ShapeType {
    pub fn fixed(dims: &[usize]) -> Self {
        ShapeType { dims: dims.iter().map(|&d| ShapeDim::Fixed(d)).collect() }
    }

    pub fn ndim(&self) -> usize { self.dims.len() }

    /// Check if two shapes are compatible for broadcasting.
    pub fn broadcast_compatible(&self, other: &ShapeType) -> bool {
        let n = self.ndim().max(other.ndim());
        for i in 0..n {
            let a = if i < self.ndim() { &self.dims[self.ndim() - 1 - i] } else { &ShapeDim::Fixed(1) };
            let b = if i < other.ndim() { &other.dims[other.ndim() - 1 - i] } else { &ShapeDim::Fixed(1) };
            match (a, b) {
                (ShapeDim::Fixed(x), ShapeDim::Fixed(y)) => {
                    if *x != *y && *x != 1 && *y != 1 { return false; }
                }
                (ShapeDim::Dynamic, _) | (_, ShapeDim::Dynamic) => {}
                (ShapeDim::Symbolic(s1), ShapeDim::Symbolic(s2)) => {
                    if s1 != s2 { return false; } // Different symbolic dims are incompatible
                }
                _ => {} // Symbolic + Fixed: assume compatible at compile time
            }
        }
        true
    }

    /// Compute matmul result shape: [..., M, K] × [..., K, N] → [..., M, N].
    pub fn matmul_result(&self, other: &ShapeType) -> Option<ShapeType> {
        if self.ndim() < 2 || other.ndim() < 2 { return None; }
        let k1 = &self.dims[self.ndim() - 1];
        let k2 = &other.dims[other.ndim() - 2];
        // Inner dims must match
        match (k1, k2) {
            (ShapeDim::Fixed(a), ShapeDim::Fixed(b)) if a != b => return None,
            _ => {}
        }
        let mut result_dims = Vec::new();
        // Batch dims from self
        for i in 0..self.ndim() - 2 {
            result_dims.push(self.dims[i].clone());
        }
        result_dims.push(self.dims[self.ndim() - 2].clone()); // M
        result_dims.push(other.dims[other.ndim() - 1].clone()); // N
        Some(ShapeType { dims: result_dims })
    }
}

// ── Custom VJP (Vector-Jacobian Products) ───────────────────────────────

/// A custom VJP rule registered for a named function.
#[derive(Clone)]
pub struct VJPRule {
    pub name: String,
    pub n_inputs: usize,
    pub n_outputs: usize,
}

/// VJP rule registry.
#[derive(Clone, Default)]
pub struct VJPRegistry {
    pub rules: HashMap<String, VJPRule>,
}

impl VJPRegistry {
    pub fn new() -> Self { VJPRegistry { rules: HashMap::new() } }

    pub fn register(&mut self, name: &str, n_inputs: usize, n_outputs: usize) {
        self.rules.insert(name.to_string(), VJPRule {
            name: name.to_string(), n_inputs, n_outputs,
        });
    }

    pub fn get(&self, name: &str) -> Option<&VJPRule> {
        self.rules.get(name)
    }

    pub fn has_rule(&self, name: &str) -> bool {
        self.rules.contains_key(name)
    }
}

// ── Differentiable Control Flow ─────────────────────────────────────────

/// Straight-through estimator: gradient passes through non-differentiable branch.
pub fn straight_through_if(condition: bool, true_val: Dual, false_val: Dual) -> Dual {
    if condition { true_val } else { false_val }
}

/// Unroll differentiable while loop (scan).
pub fn differentiable_scan(
    init: Vec<Dual>,
    n_steps: usize,
    step_fn: impl Fn(usize, &[Dual]) -> Vec<Dual>,
) -> Vec<Dual> {
    let mut state = init;
    for i in 0..n_steps {
        state = step_fn(i, &state);
    }
    state
}

/// Implicit differentiation: find x such that g(x, θ) = 0.
/// Uses Newton's method on dual numbers to propagate gradient through fixed-point.
pub fn implicit_diff_newton(
    g: impl Fn(Dual, Dual) -> Dual,
    theta: f64,
    x0: f64,
    max_iter: usize,
    tol: f64,
) -> Dual {
    let theta_dual = Dual::variable(theta);
    let mut x = x0;

    // Newton iterations to find root x*(θ): g(x, θ) = 0
    for _ in 0..max_iter {
        let x_dual = Dual::new(x, 0.0);
        let gval = g(x_dual, Dual::constant(theta));
        if gval.val.abs() < tol { break; }

        // dg/dx via forward-mode
        let x_tangent = Dual::new(x, 1.0);
        let dg_dx = g(x_tangent, Dual::constant(theta)).dot;

        if dg_dx.abs() > 1e-15 {
            x -= gval.val / dg_dx;
        } else {
            break;
        }
    }

    // Now compute dx*/dθ = -(∂g/∂θ) / (∂g/∂x) at the solution
    let x_const = Dual::constant(x);
    let dg_dtheta = g(x_const, theta_dual).dot;

    let x_tangent = Dual::new(x, 1.0);
    let dg_dx = g(x_tangent, Dual::constant(theta)).dot;

    let dx_dtheta = if dg_dx.abs() > 1e-15 { -dg_dtheta / dg_dx } else { 0.0 };
    Dual::new(x, dx_dtheta)
}

// ── FFI Interface ───────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_dual_new(val: f64, dot: f64, out_val: *mut f64, out_dot: *mut f64) {
    let d = Dual::new(val, dot);
    unsafe { *out_val = d.val; *out_dot = d.dot; }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_dual_mul(a_val: f64, a_dot: f64, b_val: f64, b_dot: f64, out_val: *mut f64, out_dot: *mut f64) {
    let r = Dual::new(a_val, a_dot).mul(Dual::new(b_val, b_dot));
    unsafe { *out_val = r.val; *out_dot = r.dot; }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_forward_deriv(x: f64, func_id: i64) -> f64 {
    // Built-in test functions for FFI
    match func_id {
        0 => forward_derivative(|d| d.mul(d), x),            // f(x) = x², f'(x) = 2x
        1 => forward_derivative(|d| d.sin(), x),             // f(x) = sin(x), f'(x) = cos(x)
        2 => forward_derivative(|d| d.exp(), x),             // f(x) = e^x, f'(x) = e^x
        3 => forward_derivative(|d| d.mul(d).mul(d), x),     // f(x) = x³, f'(x) = 3x²
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_shape_broadcast_ok(ndim_a: i64, dims_a: *const i64, ndim_b: i64, dims_b: *const i64) -> i64 {
    let a = unsafe { std::slice::from_raw_parts(dims_a, ndim_a as usize) };
    let b = unsafe { std::slice::from_raw_parts(dims_b, ndim_b as usize) };
    let sa = ShapeType { dims: a.iter().map(|&d| ShapeDim::Fixed(d as usize)).collect() };
    let sb = ShapeType { dims: b.iter().map(|&d| ShapeDim::Fixed(d as usize)).collect() };
    if sa.broadcast_compatible(&sb) { 1 } else { 0 }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_arithmetic() {
        let a = Dual::new(3.0, 1.0);
        let b = Dual::constant(2.0);
        let r = a.add(b);
        assert!((r.val - 5.0).abs() < 1e-10);
        assert!((r.dot - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_dual_mul() {
        let a = Dual::variable(3.0);
        let b = Dual::constant(2.0);
        let r = a.mul(b);
        assert!((r.val - 6.0).abs() < 1e-10);
        assert!((r.dot - 2.0).abs() < 1e-10); // d/dx (2x) = 2
    }

    #[test]
    fn test_dual_div() {
        let a = Dual::variable(6.0);
        let b = Dual::constant(3.0);
        let r = a.div(b);
        assert!((r.val - 2.0).abs() < 1e-10);
        assert!((r.dot - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_dual_chain_rule() {
        // f(x) = sin(x²), f'(x) = 2x·cos(x²)
        let x = 2.0;
        let deriv = forward_derivative(|d| d.mul(d).sin(), x);
        let expected = 2.0 * x * (x * x).cos();
        assert!((deriv - expected).abs() < 1e-8);
    }

    #[test]
    fn test_dual_exp() {
        let d = forward_derivative(|x| x.exp(), 1.0);
        assert!((d - 1.0_f64.exp()).abs() < 1e-10);
    }

    #[test]
    fn test_dual_ln() {
        let d = forward_derivative(|x| x.ln(), 2.0);
        assert!((d - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_dual_pow() {
        // f(x) = x^3, f'(x) = 3x²
        let d = forward_derivative(|x| x.pow(3.0), 2.0);
        assert!((d - 12.0).abs() < 1e-8);
    }

    #[test]
    fn test_dual_sigmoid() {
        let x = 0.0;
        let d = forward_derivative(|dd| dd.sigmoid(), x);
        // sigmoid'(0) = sigmoid(0) * (1 - sigmoid(0)) = 0.5 * 0.5 = 0.25
        assert!((d - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_dual_relu() {
        assert!((forward_derivative(|x| x.relu(), 2.0) - 1.0).abs() < 1e-10);
        assert!((forward_derivative(|x| x.relu(), -2.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_dual_tanh() {
        let x = 0.0;
        let d = forward_derivative(|dd| dd.tanh(), x);
        // tanh'(0) = 1 - tanh²(0) = 1
        assert!((d - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_forward_jacobian() {
        // f(x, y) = (x+y, x*y) → J = [[1, 1], [y, x]]
        let jac = forward_jacobian(|inp| {
            vec![inp[0].add(inp[1]), inp[0].mul(inp[1])]
        }, &[3.0, 4.0]);

        assert_eq!(jac.len(), 2);
        assert!((jac[0][0] - 1.0).abs() < 1e-10); // ∂(x+y)/∂x = 1
        assert!((jac[0][1] - 1.0).abs() < 1e-10); // ∂(x+y)/∂y = 1
        assert!((jac[1][0] - 4.0).abs() < 1e-10); // ∂(xy)/∂x = y = 4
        assert!((jac[1][1] - 3.0).abs() < 1e-10); // ∂(xy)/∂y = x = 3
    }

    #[test]
    fn test_hessian_diagonal() {
        // f(x) = x^3 → f'(x) = 3x², f''(x) = 6x
        let h = hessian_diagonal(|x| x.pow(3.0), 2.0);
        assert!((h - 12.0).abs() < 0.01); // 6*2 = 12
    }

    #[test]
    fn test_shape_broadcast() {
        let a = ShapeType::fixed(&[3, 4]);
        let b = ShapeType::fixed(&[4]);
        assert!(a.broadcast_compatible(&b));

        let c = ShapeType::fixed(&[3, 4]);
        let d = ShapeType::fixed(&[5, 4]);
        assert!(!c.broadcast_compatible(&d));

        let e = ShapeType::fixed(&[1, 4]);
        assert!(c.broadcast_compatible(&e));
    }

    #[test]
    fn test_shape_matmul() {
        let a = ShapeType::fixed(&[3, 4]);
        let b = ShapeType::fixed(&[4, 5]);
        let r = a.matmul_result(&b).unwrap();
        assert_eq!(r.dims, vec![ShapeDim::Fixed(3), ShapeDim::Fixed(5)]);
    }

    #[test]
    fn test_shape_matmul_mismatch() {
        let a = ShapeType::fixed(&[3, 4]);
        let b = ShapeType::fixed(&[5, 6]);
        assert!(a.matmul_result(&b).is_none());
    }

    #[test]
    fn test_vjp_registry() {
        let mut reg = VJPRegistry::new();
        reg.register("custom_fn", 2, 1);
        assert!(reg.has_rule("custom_fn"));
        assert!(!reg.has_rule("unknown"));
        assert_eq!(reg.get("custom_fn").unwrap().n_inputs, 2);
    }

    #[test]
    fn test_straight_through() {
        let a = Dual::variable(3.0);
        let b = Dual::constant(0.0);
        let r = straight_through_if(true, a, b);
        assert!((r.val - 3.0).abs() < 1e-10);
        assert!((r.dot - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_differentiable_scan() {
        // Accumulate: state[0] += 1 each step
        let result = differentiable_scan(
            vec![Dual::variable(0.0)],
            5,
            |_, state| vec![state[0].add(Dual::constant(1.0))],
        );
        assert!((result[0].val - 5.0).abs() < 1e-10);
        assert!((result[0].dot - 1.0).abs() < 1e-10); // derivative passes through additions
    }

    #[test]
    fn test_implicit_diff() {
        // g(x, θ) = x - θ² = 0 → x* = θ², dx*/dθ = 2θ
        let result = implicit_diff_newton(
            |x, theta| x.sub(theta.mul(theta)),
            3.0, 1.0, 100, 1e-12,
        );
        assert!((result.val - 9.0).abs() < 1e-6);
        assert!((result.dot - 6.0).abs() < 1e-4); // dx*/dθ = 2θ = 6
    }

    #[test]
    fn test_ffi_forward_deriv() {
        // x² at x=3 → derivative = 6
        assert!((vitalis_forward_deriv(3.0, 0) - 6.0).abs() < 1e-10);
        // sin at x=0 → derivative = cos(0) = 1
        assert!((vitalis_forward_deriv(0.0, 1) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_broadcast() {
        let a = [3i64, 4];
        let b = [4i64];
        assert_eq!(vitalis_shape_broadcast_ok(2, a.as_ptr(), 1, b.as_ptr()), 1);
    }
}
