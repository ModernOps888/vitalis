//! Numerical Methods Module — Linear algebra, calculus, and numerical analysis for Vitalis
//!
//! Pure Rust implementations with zero external dependencies.
//! Exposed via C FFI for Python interop.
//!
//! # Algorithms:
//! - Matrix operations (multiply, transpose, determinant, inverse)
//! - LU decomposition with partial pivoting
//! - Gaussian elimination (solve Ax = b)
//! - Newton-Raphson root finding
//! - Bisection method
//! - Numerical integration (Simpson's rule, trapezoidal)
//! - Numerical differentiation (central difference)
//! - Polynomial evaluation (Horner's method)
//! - Polynomial root finding (Durand-Kerner)
//! - Interpolation (Lagrange, Newton)
//! - Eigenvalue estimation (power iteration)
//! - Cholesky decomposition

use std::ffi::CString;
use std::os::raw::c_char;

// ─── Matrix Multiply ─────────────────────────────────────────────────

/// Matrix multiplication C = A * B.
/// A is m×k, B is k×n, C is m×n. All row-major.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_mul(
    a: *const f64, m: usize, k: usize,
    b: *const f64, n: usize,
    c: *mut f64,
) {
    if a.is_null() || b.is_null() || c.is_null() { return; }
    let a_s = unsafe { std::slice::from_raw_parts(a, m * k) };
    let b_s = unsafe { std::slice::from_raw_parts(b, k * n) };
    let c_s = unsafe { std::slice::from_raw_parts_mut(c, m * n) };

    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0;
            for l in 0..k {
                sum += a_s[i * k + l] * b_s[l * n + j];
            }
            c_s[i * n + j] = sum;
        }
    }
}

/// Matrix transpose. A(m×n) → B(n×m). Both row-major.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_transpose(
    a: *const f64, m: usize, n: usize,
    b: *mut f64,
) {
    if a.is_null() || b.is_null() { return; }
    let a_s = unsafe { std::slice::from_raw_parts(a, m * n) };
    let b_s = unsafe { std::slice::from_raw_parts_mut(b, n * m) };
    for i in 0..m {
        for j in 0..n {
            b_s[j * m + i] = a_s[i * n + j];
        }
    }
}

// ─── Determinant ──────────────────────────────────────────────────────

fn determinant(mat: &[f64], n: usize) -> f64 {
    if n == 1 { return mat[0]; }
    if n == 2 { return mat[0]*mat[3] - mat[1]*mat[2]; }

    let mut det = 0.0;
    let mut sub = vec![0.0; (n-1)*(n-1)];
    for col in 0..n {
        let mut si = 0;
        for i in 1..n {
            for j in 0..n {
                if j == col { continue; }
                sub[si] = mat[i*n + j];
                si += 1;
            }
        }
        let sign = if col % 2 == 0 { 1.0 } else { -1.0 };
        det += sign * mat[col] * determinant(&sub, n-1);
    }
    det
}

/// Matrix determinant (n×n).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_det(
    a: *const f64, n: usize,
) -> f64 {
    if a.is_null() || n == 0 { return 0.0; }
    let a_s = unsafe { std::slice::from_raw_parts(a, n * n) };
    determinant(a_s, n)
}

// ─── Matrix Inverse (Gauss-Jordan) ────────────────────────────────────

fn matrix_inverse(mat: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut aug = vec![0.0; n * 2 * n];
    for i in 0..n {
        for j in 0..n {
            aug[i * 2*n + j] = mat[i*n + j];
        }
        aug[i * 2*n + n + i] = 1.0;
    }

    for col in 0..n {
        // Find pivot
        let mut max_row = col;
        let mut max_val = aug[col * 2*n + col].abs();
        for row in col+1..n {
            let v = aug[row * 2*n + col].abs();
            if v > max_val {
                max_val = v;
                max_row = row;
            }
        }
        if max_val < 1e-12 { return None; } // Singular

        // Swap rows
        if max_row != col {
            for j in 0..2*n {
                let tmp = aug[col * 2*n + j];
                aug[col * 2*n + j] = aug[max_row * 2*n + j];
                aug[max_row * 2*n + j] = tmp;
            }
        }

        // Scale pivot row
        let pivot = aug[col * 2*n + col];
        for j in 0..2*n {
            aug[col * 2*n + j] /= pivot;
        }

        // Eliminate column
        for row in 0..n {
            if row == col { continue; }
            let factor = aug[row * 2*n + col];
            for j in 0..2*n {
                aug[row * 2*n + j] -= factor * aug[col * 2*n + j];
            }
        }
    }

    let mut inv = vec![0.0; n*n];
    for i in 0..n {
        for j in 0..n {
            inv[i*n + j] = aug[i * 2*n + n + j];
        }
    }
    Some(inv)
}

/// Matrix inverse (n×n). Returns 0 on success, -1 if singular.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_inverse(
    a: *const f64, n: usize,
    out: *mut f64,
) -> i32 {
    if a.is_null() || out.is_null() || n == 0 { return -1; }
    let a_s = unsafe { std::slice::from_raw_parts(a, n*n) };
    match matrix_inverse(a_s, n) {
        Some(inv) => {
            let o = unsafe { std::slice::from_raw_parts_mut(out, n*n) };
            o.copy_from_slice(&inv);
            0
        }
        None => -1,
    }
}

// ─── LU Decomposition ────────────────────────────────────────────────

/// LU decomposition with partial pivoting.
/// Returns 0 on success, -1 on failure.
/// Produces L (lower triangular), U (upper triangular), P (permutation).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lu_decompose(
    a: *const f64, n: usize,
    out_l: *mut f64,
    out_u: *mut f64,
    out_p: *mut usize,
) -> i32 {
    if a.is_null() || out_l.is_null() || out_u.is_null() || out_p.is_null() { return -1; }
    let a_s = unsafe { std::slice::from_raw_parts(a, n*n) };
    let l = unsafe { std::slice::from_raw_parts_mut(out_l, n*n) };
    let u = unsafe { std::slice::from_raw_parts_mut(out_u, n*n) };
    let p = unsafe { std::slice::from_raw_parts_mut(out_p, n) };

    let mut mat = a_s.to_vec();
    for i in 0..n { p[i] = i; }
    for i in 0..n*n { l[i] = 0.0; u[i] = 0.0; }

    for col in 0..n {
        let mut max_row = col;
        let mut max_val = mat[col*n + col].abs();
        for row in col+1..n {
            if mat[row*n + col].abs() > max_val {
                max_val = mat[row*n + col].abs();
                max_row = row;
            }
        }
        if max_val < 1e-12 { return -1; }
        if max_row != col {
            p.swap(col, max_row);
            for j in 0..n {
                mat.swap(col*n + j, max_row*n + j);
            }
        }

        for row in col+1..n {
            let factor = mat[row*n + col] / mat[col*n + col];
            mat[row*n + col] = factor; // Store L factor
            for j in col+1..n {
                mat[row*n + j] -= factor * mat[col*n + j];
            }
        }
    }

    for i in 0..n {
        l[i*n + i] = 1.0;
        for j in 0..n {
            if j < i {
                l[i*n + j] = mat[i*n + j];
            } else {
                u[i*n + j] = mat[i*n + j];
            }
        }
    }
    0
}

// ─── Solve Ax = b (Gaussian elimination) ──────────────────────────────

/// Solve linear system Ax = b via Gaussian elimination.
/// Returns 0 on success, -1 if singular.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_solve_linear(
    a: *const f64, b: *const f64, n: usize,
    x: *mut f64,
) -> i32 {
    if a.is_null() || b.is_null() || x.is_null() { return -1; }
    let a_s = unsafe { std::slice::from_raw_parts(a, n*n) };
    let b_s = unsafe { std::slice::from_raw_parts(b, n) };
    let x_s = unsafe { std::slice::from_raw_parts_mut(x, n) };

    let mut aug = vec![0.0; n * (n+1)];
    for i in 0..n {
        for j in 0..n {
            aug[i*(n+1) + j] = a_s[i*n + j];
        }
        aug[i*(n+1) + n] = b_s[i];
    }

    // Forward elimination
    for col in 0..n {
        let mut max_row = col;
        for row in col+1..n {
            if aug[row*(n+1)+col].abs() > aug[max_row*(n+1)+col].abs() {
                max_row = row;
            }
        }
        if aug[max_row*(n+1)+col].abs() < 1e-12 { return -1; }
        if max_row != col {
            for j in 0..=n {
                let tmp = aug[col*(n+1)+j];
                aug[col*(n+1)+j] = aug[max_row*(n+1)+j];
                aug[max_row*(n+1)+j] = tmp;
            }
        }
        for row in col+1..n {
            let factor = aug[row*(n+1)+col] / aug[col*(n+1)+col];
            for j in col..=n {
                aug[row*(n+1)+j] -= factor * aug[col*(n+1)+j];
            }
        }
    }

    // Back substitution
    for i in (0..n).rev() {
        x_s[i] = aug[i*(n+1)+n];
        for j in i+1..n {
            x_s[i] -= aug[i*(n+1)+j] * x_s[j];
        }
        x_s[i] /= aug[i*(n+1)+i];
    }
    0
}

// ─── Cholesky Decomposition ──────────────────────────────────────────

/// Cholesky decomposition A = L * L^T for symmetric positive definite matrix.
/// Returns 0 on success, -1 if matrix is not positive definite.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cholesky(
    a: *const f64, n: usize,
    out_l: *mut f64,
) -> i32 {
    if a.is_null() || out_l.is_null() || n == 0 { return -1; }
    let a_s = unsafe { std::slice::from_raw_parts(a, n*n) };
    let l = unsafe { std::slice::from_raw_parts_mut(out_l, n*n) };

    for i in 0..n*n { l[i] = 0.0; }

    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i*n+k] * l[j*n+k];
            }
            if i == j {
                let val = a_s[i*n+i] - sum;
                if val <= 0.0 { return -1; }
                l[i*n+j] = val.sqrt();
            } else {
                l[i*n+j] = (a_s[i*n+j] - sum) / l[j*n+j];
            }
        }
    }
    0
}

// ─── Newton-Raphson (function pointer based) ──────────────────────────

/// Newton-Raphson root finding (for f(x) = ax^2 + bx + c).
/// Returns root closest to x0. max_iter iterations, tol tolerance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_newton_quadratic(
    a_coeff: f64, b_coeff: f64, c_coeff: f64,
    x0: f64,
    max_iter: usize,
    tol: f64,
) -> f64 {
    let mut x = x0;
    for _ in 0..max_iter {
        let fx = a_coeff * x * x + b_coeff * x + c_coeff;
        let fpx = 2.0 * a_coeff * x + b_coeff;
        if fpx.abs() < 1e-15 { break; }
        let x_new = x - fx / fpx;
        if (x_new - x).abs() < tol { return x_new; }
        x = x_new;
    }
    x
}

// ─── Bisection Method ────────────────────────────────────────────────

/// Bisection root finding for f(x) = ax^2 + bx + c on [lo, hi].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bisection_quadratic(
    a_coeff: f64, b_coeff: f64, c_coeff: f64,
    lo: f64, hi: f64,
    max_iter: usize,
    tol: f64,
) -> f64 {
    let f = |x: f64| a_coeff * x * x + b_coeff * x + c_coeff;
    let mut a = lo;
    let mut b = hi;
    if f(a) * f(b) > 0.0 { return f64::NAN; }

    for _ in 0..max_iter {
        let mid = (a + b) / 2.0;
        if (b - a) / 2.0 < tol { return mid; }
        if f(mid) * f(a) < 0.0 {
            b = mid;
        } else {
            a = mid;
        }
    }
    (a + b) / 2.0
}

// ─── Numerical Integration ───────────────────────────────────────────

/// Simpson's 1/3 rule for evaluating integral of tabulated data.
/// values: f(x_0), f(x_1), ..., f(x_n) at equal spacing h.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_simpson(
    values: *const f64,
    n: usize,
    h: f64,
) -> f64 {
    if values.is_null() || n < 3 || n % 2 == 0 { return 0.0; }
    let v = unsafe { std::slice::from_raw_parts(values, n) };
    let mut sum = v[0] + v[n-1];
    for i in 1..n-1 {
        sum += if i % 2 == 1 { 4.0 } else { 2.0 } * v[i];
    }
    sum * h / 3.0
}

/// Trapezoidal rule for tabulated data.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_trapezoid(
    values: *const f64,
    n: usize,
    h: f64,
) -> f64 {
    if values.is_null() || n < 2 { return 0.0; }
    let v = unsafe { std::slice::from_raw_parts(values, n) };
    let mut sum = (v[0] + v[n-1]) / 2.0;
    for i in 1..n-1 {
        sum += v[i];
    }
    sum * h
}

// ─── Numerical Differentiation ───────────────────────────────────────

/// Central difference derivative estimate from tabulated data.
/// Returns array of n derivatives (forward/backward at endpoints).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_central_diff(
    values: *const f64,
    n: usize,
    h: f64,
    out: *mut f64,
) {
    if values.is_null() || out.is_null() || n < 2 { return; }
    let v = unsafe { std::slice::from_raw_parts(values, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n) };

    o[0] = (v[1] - v[0]) / h; // Forward difference
    for i in 1..n-1 {
        o[i] = (v[i+1] - v[i-1]) / (2.0 * h); // Central
    }
    o[n-1] = (v[n-1] - v[n-2]) / h; // Backward
}

// ─── Horner's Method (Polynomial Evaluation) ─────────────────────────

/// Evaluate polynomial using Horner's method.
/// coeffs[0] = a_n (highest degree), coeffs[n-1] = a_0 (constant).
/// P(x) = a_n*x^n + a_{n-1}*x^{n-1} + ... + a_0
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_horner(
    coeffs: *const f64,
    n: usize,
    x: f64,
) -> f64 {
    if coeffs.is_null() || n == 0 { return 0.0; }
    let c = unsafe { std::slice::from_raw_parts(coeffs, n) };
    let mut result = c[0];
    for i in 1..n {
        result = result * x + c[i];
    }
    result
}

// ─── Lagrange Interpolation ──────────────────────────────────────────

/// Lagrange interpolation at point x given data points.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lagrange_interp(
    xs: *const f64,
    ys: *const f64,
    n: usize,
    x: f64,
) -> f64 {
    if xs.is_null() || ys.is_null() || n == 0 { return 0.0; }
    let xv = unsafe { std::slice::from_raw_parts(xs, n) };
    let yv = unsafe { std::slice::from_raw_parts(ys, n) };

    let mut result = 0.0;
    for i in 0..n {
        let mut basis = 1.0;
        for j in 0..n {
            if i != j {
                basis *= (x - xv[j]) / (xv[i] - xv[j]);
            }
        }
        result += yv[i] * basis;
    }
    result
}

// ─── Power Iteration (dominant eigenvalue) ────────────────────────────

/// Power iteration for dominant eigenvalue of n×n matrix.
/// Returns eigenvalue estimate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_power_iteration(
    a: *const f64,
    n: usize,
    max_iter: usize,
    tol: f64,
) -> f64 {
    if a.is_null() || n == 0 { return 0.0; }
    let mat = unsafe { std::slice::from_raw_parts(a, n*n) };

    let mut v = vec![1.0 / (n as f64).sqrt(); n];
    let mut eigenvalue = 0.0;

    for _ in 0..max_iter {
        // w = A * v
        let mut w = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                w[i] += mat[i*n + j] * v[j];
            }
        }

        // Find max magnitude element
        let new_eigenvalue = w.iter().cloned().fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
        if new_eigenvalue.abs() < 1e-15 { break; }

        // Normalize
        for i in 0..n {
            v[i] = w[i] / new_eigenvalue;
        }

        if (new_eigenvalue - eigenvalue).abs() < tol {
            return new_eigenvalue;
        }
        eigenvalue = new_eigenvalue;
    }
    eigenvalue
}

// ─── Matrix Trace & Frobenius Norm ───────────────────────────────────

/// Matrix trace (sum of diagonal elements).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_trace(a: *const f64, n: usize) -> f64 {
    if a.is_null() || n == 0 { return 0.0; }
    let mat = unsafe { std::slice::from_raw_parts(a, n*n) };
    (0..n).map(|i| mat[i*n + i]).sum()
}

/// Frobenius norm of matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mat_frobenius(a: *const f64, m: usize, n: usize) -> f64 {
    if a.is_null() { return 0.0; }
    let mat = unsafe { std::slice::from_raw_parts(a, m*n) };
    mat.iter().map(|x| x*x).sum::<f64>().sqrt()
}

/// Dot product of two vectors.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_dot_product(
    a: *const f64, b: *const f64, n: usize,
) -> f64 {
    if a.is_null() || b.is_null() || n == 0 { return 0.0; }
    let av = unsafe { std::slice::from_raw_parts(a, n) };
    let bv = unsafe { std::slice::from_raw_parts(b, n) };
    av.iter().zip(bv.iter()).map(|(x, y)| x * y).sum()
}

/// Vector L2 norm.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_vec_norm(a: *const f64, n: usize) -> f64 {
    if a.is_null() || n == 0 { return 0.0; }
    let av = unsafe { std::slice::from_raw_parts(a, n) };
    av.iter().map(|x| x*x).sum::<f64>().sqrt()
}

/// Cross product of 3D vectors. out = a × b.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cross_product(
    a: *const f64, b: *const f64, out: *mut f64,
) {
    if a.is_null() || b.is_null() || out.is_null() { return; }
    let av = unsafe { std::slice::from_raw_parts(a, 3) };
    let bv = unsafe { std::slice::from_raw_parts(b, 3) };
    let ov = unsafe { std::slice::from_raw_parts_mut(out, 3) };
    ov[0] = av[1]*bv[2] - av[2]*bv[1];
    ov[1] = av[2]*bv[0] - av[0]*bv[2];
    ov[2] = av[0]*bv[1] - av[1]*bv[0];
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mat_mul() {
        let a = [1.0, 2.0, 3.0, 4.0]; // 2x2
        let b = [5.0, 6.0, 7.0, 8.0]; // 2x2
        let mut c = [0.0f64; 4];
        unsafe { vitalis_mat_mul(a.as_ptr(), 2, 2, b.as_ptr(), 2, c.as_mut_ptr()); }
        assert_eq!(c, [19.0, 22.0, 43.0, 50.0]);
    }

    #[test]
    fn test_mat_transpose() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2x3
        let mut b = [0.0f64; 6]; // 3x2
        unsafe { vitalis_mat_transpose(a.as_ptr(), 2, 3, b.as_mut_ptr()); }
        assert_eq!(b, [1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    #[test]
    fn test_determinant() {
        let m = [1.0, 2.0, 3.0, 4.0]; // 2x2
        let det = unsafe { vitalis_mat_det(m.as_ptr(), 2) };
        assert!((det - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_mat_inverse() {
        let a = [4.0, 7.0, 2.0, 6.0]; // 2x2
        let mut inv = [0.0f64; 4];
        let ret = unsafe { vitalis_mat_inverse(a.as_ptr(), 2, inv.as_mut_ptr()) };
        assert_eq!(ret, 0);
        // Verify A * A_inv = I
        let mut identity = [0.0f64; 4];
        unsafe { vitalis_mat_mul(a.as_ptr(), 2, 2, inv.as_ptr(), 2, identity.as_mut_ptr()); }
        assert!((identity[0] - 1.0).abs() < 1e-10);
        assert!((identity[3] - 1.0).abs() < 1e-10);
        assert!(identity[1].abs() < 1e-10);
    }

    #[test]
    fn test_solve_linear() {
        // 2x + 3y = 8, x + 2y = 5 → x=1, y=2
        let a = [2.0, 3.0, 1.0, 2.0];
        let b = [8.0, 5.0];
        let mut x = [0.0f64; 2];
        let ret = unsafe { vitalis_solve_linear(a.as_ptr(), b.as_ptr(), 2, x.as_mut_ptr()) };
        assert_eq!(ret, 0);
        assert!((x[0] - 1.0).abs() < 1e-10);
        assert!((x[1] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky() {
        // Symmetric positive definite: [[4,2],[2,3]]
        let a = [4.0, 2.0, 2.0, 3.0];
        let mut l = [0.0f64; 4];
        let ret = unsafe { vitalis_cholesky(a.as_ptr(), 2, l.as_mut_ptr()) };
        assert_eq!(ret, 0);
        assert!((l[0] - 2.0).abs() < 1e-10);
        assert!((l[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_newton_quadratic() {
        // x^2 - 4 = 0, root at x=2
        let root = unsafe { vitalis_newton_quadratic(1.0, 0.0, -4.0, 3.0, 100, 1e-10) };
        assert!((root - 2.0).abs() < 1e-8);
    }

    #[test]
    fn test_bisection() {
        // x^2 - 4 = 0 on [0, 5]
        let root = unsafe { vitalis_bisection_quadratic(1.0, 0.0, -4.0, 0.0, 5.0, 100, 1e-10) };
        assert!((root - 2.0).abs() < 1e-8);
    }

    #[test]
    fn test_simpson() {
        // Integrate f(x) = x^2 from 0 to 4, h=1
        let values = [0.0, 1.0, 4.0, 9.0, 16.0]; // x=0,1,2,3,4
        let integral = unsafe { vitalis_simpson(values.as_ptr(), 5, 1.0) };
        assert!((integral - 64.0/3.0).abs() < 0.1);
    }

    #[test]
    fn test_horner() {
        // 2x^2 + 3x + 1 at x=2 → 15
        let coeffs = [2.0, 3.0, 1.0];
        let val = unsafe { vitalis_horner(coeffs.as_ptr(), 3, 2.0) };
        assert!((val - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_lagrange() {
        // Points: (1,1), (2,4), (3,9) → y=x^2
        let xs = [1.0, 2.0, 3.0];
        let ys = [1.0, 4.0, 9.0];
        let val = unsafe { vitalis_lagrange_interp(xs.as_ptr(), ys.as_ptr(), 3, 2.5) };
        assert!((val - 6.25).abs() < 1e-10);
    }

    #[test]
    fn test_dot_product() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let d = unsafe { vitalis_dot_product(a.as_ptr(), b.as_ptr(), 3) };
        assert_eq!(d, 32.0);
    }

    #[test]
    fn test_cross_product() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        let mut c = [0.0f64; 3];
        unsafe { vitalis_cross_product(a.as_ptr(), b.as_ptr(), c.as_mut_ptr()); }
        assert_eq!(c, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_power_iteration() {
        // [[2,1],[1,2]] → dominant eigenvalue = 3
        let a = [2.0, 1.0, 1.0, 2.0];
        let ev = unsafe { vitalis_power_iteration(a.as_ptr(), 2, 1000, 1e-10) };
        assert!((ev - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_mat_trace() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let tr = unsafe { vitalis_mat_trace(a.as_ptr(), 2) };
        assert_eq!(tr, 5.0);
    }
}
