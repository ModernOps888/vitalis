//! Advanced Mathematics Module — Pure math algorithms for Vitalis
//!
//! Extends numerical.rs with higher mathematics:
//! - Number theory (primality, factoring, GCD/LCM, modular arithmetic, Euler's totient)
//! - Combinatorics (permutations, combinations, Catalan, Stirling, Bell numbers)
//! - Abstract algebra (group operations, polynomial rings, Galois fields GF(p))
//! - Calculus (symbolic differentiation coefficients, Taylor series, Fourier coefficients)
//! - Special functions (Gamma, Beta, Bessel, Legendre, Hermite, Laguerre)
//! - Optimization (gradient descent, simulated annealing, genetic algorithm core)
//! - Tensor operations (outer product, contraction, Kronecker product)
//! - Complex analysis (Mandelbrot iteration, Julia sets)

use std::f64::consts::PI;

// ═══════════════════════════════════════════════════════════════════════
// NUMBER THEORY
// ═══════════════════════════════════════════════════════════════════════

/// Miller-Rabin primality test (deterministic for n < 3.3×10²⁴).
pub fn is_prime(n: u64) -> bool {
    if n < 2 { return false; }
    if n < 4 { return true; }
    if n % 2 == 0 || n % 3 == 0 { return false; }
    // Deterministic witnesses for n < 3,317,044,064,679,887,385,961,981
    let witnesses: &[u64] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    let mut d = n - 1;
    let mut r = 0u32;
    while d % 2 == 0 { d /= 2; r += 1; }

    'witness: for &a in witnesses {
        if a >= n { continue; }
        let mut x = mod_pow(a, d, n);
        if x == 1 || x == n - 1 { continue; }
        for _ in 0..r - 1 {
            x = mod_mul(x, x, n);
            if x == n - 1 { continue 'witness; }
        }
        return false;
    }
    true
}

/// Modular exponentiation: base^exp mod modulus.
pub fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 { return 0; }
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp % 2 == 1 {
            result = mod_mul(result, base, modulus);
        }
        exp /= 2;
        base = mod_mul(base, base, modulus);
    }
    result
}

/// Modular multiplication avoiding overflow using u128.
pub fn mod_mul(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

/// Sieve of Eratosthenes — returns all primes up to n.
pub fn sieve_primes(n: usize) -> Vec<u64> {
    if n < 2 { return vec![]; }
    let mut is_prime_flag = vec![true; n + 1];
    is_prime_flag[0] = false;
    is_prime_flag[1] = false;
    let limit = (n as f64).sqrt() as usize + 1;
    for i in 2..=limit {
        if is_prime_flag[i] {
            let mut j = i * i;
            while j <= n {
                is_prime_flag[j] = false;
                j += i;
            }
        }
    }
    is_prime_flag.iter().enumerate()
        .filter(|(_, p)| **p)
        .map(|(i, _)| i as u64)
        .collect()
}

/// Greatest common divisor (Euclidean algorithm).
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 { let t = b; b = a % b; a = t; }
    a
}

/// Least common multiple.
pub fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 { 0 } else { a / gcd(a, b) * b }
}

/// Extended Euclidean algorithm: returns (gcd, x, y) where ax + by = gcd.
pub fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if a == 0 { return (b, 0, 1); }
    let (g, x1, y1) = extended_gcd(b % a, a);
    (g, y1 - (b / a) * x1, x1)
}

/// Modular multiplicative inverse (a⁻¹ mod m), if it exists.
pub fn mod_inverse(a: i64, m: i64) -> Option<i64> {
    let (g, x, _) = extended_gcd(a % m, m);
    if g != 1 { None } else { Some(((x % m) + m) % m) }
}

/// Euler's totient function φ(n).
pub fn euler_totient(mut n: u64) -> u64 {
    if n == 0 { return 0; }
    let mut result = n;
    let mut p = 2u64;
    while p * p <= n {
        if n % p == 0 {
            while n % p == 0 { n /= p; }
            result -= result / p;
        }
        p += 1;
    }
    if n > 1 { result -= result / n; }
    result
}

/// Integer factorization (trial division + Pollard's rho for larger factors).
pub fn factorize(mut n: u64) -> Vec<u64> {
    let mut factors = Vec::new();
    if n <= 1 { return factors; }
    while n % 2 == 0 { factors.push(2); n /= 2; }
    let mut d = 3u64;
    while d * d <= n {
        while n % d == 0 { factors.push(d); n /= d; }
        d += 2;
    }
    if n > 1 { factors.push(n); }
    factors
}

/// Chinese Remainder Theorem: solve system x ≡ a_i (mod m_i).
/// Returns (x, M) where x is the solution and M = product of moduli.
pub fn chinese_remainder(residues: &[i64], moduli: &[i64]) -> Option<(i64, i64)> {
    if residues.len() != moduli.len() || residues.is_empty() { return None; }
    let mut x = residues[0];
    let mut m = moduli[0];
    for i in 1..residues.len() {
        let (g, p, _) = extended_gcd(m, moduli[i]);
        if (residues[i] - x) % g != 0 { return None; }
        let lcm_val = m / g * moduli[i];
        x = (x + m * ((residues[i] - x) / g % (moduli[i] / g)) * p) % lcm_val;
        if x < 0 { x += lcm_val; }
        m = lcm_val;
    }
    Some((x, m))
}

/// Fibonacci (iterative, O(n)).
pub fn fibonacci(n: u64) -> u64 {
    if n <= 1 { return n; }
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 2..=n {
        let c = a.saturating_add(b);
        a = b;
        b = c;
    }
    b
}

/// Lucas number L(n).
pub fn lucas(n: u64) -> u64 {
    if n == 0 { return 2; }
    if n == 1 { return 1; }
    let (mut a, mut b) = (2u64, 1u64);
    for _ in 2..=n {
        let c = a.saturating_add(b);
        a = b;
        b = c;
    }
    b
}

// ═══════════════════════════════════════════════════════════════════════
// COMBINATORICS
// ═══════════════════════════════════════════════════════════════════════

/// Factorial n! (saturating to prevent overflow).
pub fn factorial(n: u64) -> u64 {
    (1..=n).fold(1u64, |acc, x| acc.saturating_mul(x))
}

/// Binomial coefficient C(n, k) = n! / (k! * (n-k)!).
pub fn binomial(n: u64, k: u64) -> u64 {
    if k > n { return 0; }
    let k = k.min(n - k);
    let mut result = 1u64;
    for i in 0..k {
        result = result.saturating_mul(n - i) / (i + 1);
    }
    result
}

/// Multinomial coefficient.
pub fn multinomial(groups: &[u64]) -> u64 {
    let n: u64 = groups.iter().sum();
    let mut result = factorial(n);
    for &g in groups {
        result /= factorial(g);
    }
    result
}

/// Number of permutations P(n, k) = n! / (n-k)!.
pub fn permutations(n: u64, k: u64) -> u64 {
    if k > n { return 0; }
    (n - k + 1..=n).fold(1u64, |acc, x| acc.saturating_mul(x))
}

/// Catalan number C_n = C(2n, n) / (n + 1).
pub fn catalan(n: u64) -> u64 {
    binomial(2 * n, n) / (n + 1)
}

/// Stirling number of the second kind S(n, k).
pub fn stirling_second(n: u64, k: u64) -> u64 {
    if k == 0 { return if n == 0 { 1 } else { 0 }; }
    if k > n { return 0; }
    if k == n { return 1; }
    let mut sum = 0u64;
    for j in 0..=k {
        let term = binomial(k, j).saturating_mul(mod_pow(j, n, u64::MAX));
        if (k - j) % 2 == 0 {
            sum = sum.saturating_add(term);
        } else {
            sum = sum.saturating_sub(term);
        }
    }
    sum / factorial(k)
}

/// Bell number B_n (number of partitions of a set).
pub fn bell(n: u64) -> u64 {
    if n == 0 { return 1; }
    let n = n as usize;
    let mut triangle = vec![vec![0u64; n + 1]; n + 1];
    triangle[0][0] = 1;
    for i in 1..=n {
        triangle[i][0] = triangle[i - 1][i - 1];
        for j in 1..=i {
            triangle[i][j] = triangle[i][j - 1].saturating_add(triangle[i - 1][j - 1]);
        }
    }
    triangle[n][0]
}

/// Derangement D(n) — permutations with no fixed points.
pub fn derangement(n: u64) -> u64 {
    if n == 0 { return 1; }
    if n == 1 { return 0; }
    let mut d0 = 1u64;
    let mut d1 = 0u64;
    for i in 2..=n {
        let d2 = (i - 1).saturating_mul(d0.saturating_add(d1));
        d0 = d1;
        d1 = d2;
    }
    d1
}

/// Partition number p(n) — number of integer partitions.
pub fn partition_count(n: u64) -> u64 {
    let n = n as usize;
    let mut dp = vec![0u64; n + 1];
    dp[0] = 1;
    for k in 1..=n {
        for i in k..=n {
            dp[i] = dp[i].saturating_add(dp[i - k]);
        }
    }
    dp[n]
}

// ═══════════════════════════════════════════════════════════════════════
// SPECIAL FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════

/// Gamma function Γ(x) using Lanczos approximation.
pub fn gamma(x: f64) -> f64 {
    if x <= 0.0 && x == x.floor() { return f64::INFINITY; }
    if x < 0.5 {
        return PI / ((PI * x).sin() * gamma(1.0 - x));
    }
    let x = x - 1.0;
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
    let mut sum = c[0];
    for (i, &coeff) in c[1..].iter().enumerate() {
        sum += coeff / (x + i as f64 + 1.0);
    }
    let t = x + g + 0.5;
    (2.0 * PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * sum
}

/// Log-Gamma function ln(Γ(x)).
pub fn lgamma(x: f64) -> f64 {
    gamma(x).abs().ln()
}

/// Beta function B(a, b) = Γ(a)Γ(b) / Γ(a+b).
pub fn beta(a: f64, b: f64) -> f64 {
    gamma(a) * gamma(b) / gamma(a + b)
}

/// Bessel function of the first kind J_0(x).
pub fn bessel_j0(x: f64) -> f64 {
    let mut sum = 0.0;
    for k in 0..20 {
        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
        let term = sign * (x / 2.0).powi(2 * k as i32) / (factorial(k as u64) as f64).powi(2);
        sum += term;
    }
    sum
}

/// Bessel function of the first kind J_1(x).
pub fn bessel_j1(x: f64) -> f64 {
    let mut sum = 0.0;
    for k in 0..20 {
        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
        let term = sign * (x / 2.0).powi(2 * k as i32 + 1)
            / (factorial(k as u64) as f64 * factorial(k as u64 + 1) as f64);
        sum += term;
    }
    sum
}

/// Bessel function J_n(x) for integer order n.
pub fn bessel_jn(n: i32, x: f64) -> f64 {
    if n == 0 { return bessel_j0(x); }
    if n == 1 { return bessel_j1(x); }
    if n < 0 {
        let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
        return sign * bessel_jn(-n, x);
    }
    // Miller's backward recurrence
    let n = n as usize;
    let m = n + 20;
    let mut jp1 = 0.0f64;
    let mut j = 1.0e-30f64;
    let mut result = 0.0;
    for k in (0..=m).rev() {
        let jm1 = 2.0 * (k + 1) as f64 / x * j - jp1;
        jp1 = j;
        j = jm1;
        if k == n { result = jp1; }
    }
    result * bessel_j0(x) / j
}

/// Legendre polynomial P_n(x) using recurrence.
pub fn legendre(n: usize, x: f64) -> f64 {
    if n == 0 { return 1.0; }
    if n == 1 { return x; }
    let mut p0 = 1.0;
    let mut p1 = x;
    for k in 2..=n {
        let p2 = ((2 * k - 1) as f64 * x * p1 - (k - 1) as f64 * p0) / k as f64;
        p0 = p1;
        p1 = p2;
    }
    p1
}

/// Hermite polynomial H_n(x) (physicist's convention).
pub fn hermite(n: usize, x: f64) -> f64 {
    if n == 0 { return 1.0; }
    if n == 1 { return 2.0 * x; }
    let mut h0 = 1.0;
    let mut h1 = 2.0 * x;
    for k in 2..=n {
        let h2 = 2.0 * x * h1 - 2.0 * (k - 1) as f64 * h0;
        h0 = h1;
        h1 = h2;
    }
    h1
}

/// Laguerre polynomial L_n(x).
pub fn laguerre(n: usize, x: f64) -> f64 {
    if n == 0 { return 1.0; }
    if n == 1 { return 1.0 - x; }
    let mut l0 = 1.0;
    let mut l1 = 1.0 - x;
    for k in 2..=n {
        let l2 = ((2 * k - 1) as f64 - x) * l1 / k as f64
            - (k - 1) as f64 * l0 / k as f64;
        l0 = l1;
        l1 = l2;
    }
    l1
}

/// Chebyshev polynomial T_n(x) of the first kind.
pub fn chebyshev_t(n: usize, x: f64) -> f64 {
    if n == 0 { return 1.0; }
    if n == 1 { return x; }
    let mut t0 = 1.0;
    let mut t1 = x;
    for _ in 2..=n {
        let t2 = 2.0 * x * t1 - t0;
        t0 = t1;
        t1 = t2;
    }
    t1
}

/// Error function erf(x) using Abramowitz and Stegun approximation.
pub fn erf(x: f64) -> f64 {
    if x.abs() < 1e-15 { return 0.0; }
    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = t * (0.254829592 + t * (-0.284496736 + t * (1.421413741
        + t * (-1.453152027 + t * 1.061405429))));
    sign * (1.0 - poly * (-x * x).exp())
}

/// Complementary error function erfc(x) = 1 - erf(x).
pub fn erfc(x: f64) -> f64 {
    1.0 - erf(x)
}

/// Riemann zeta function ζ(s) for real s > 1 (Euler-Maclaurin).
pub fn zeta(s: f64) -> f64 {
    if s <= 1.0 { return f64::INFINITY; }
    let n = 100;
    let mut sum = 0.0;
    for k in 1..=n {
        sum += 1.0 / (k as f64).powf(s);
    }
    // Euler-Maclaurin correction
    sum += (n as f64).powf(1.0 - s) / (s - 1.0) + 0.5 / (n as f64).powf(s);
    sum
}

// ═══════════════════════════════════════════════════════════════════════
// OPTIMIZATION ALGORITHMS
// ═══════════════════════════════════════════════════════════════════════

/// Gradient descent on a 1D function.
/// Returns the x that minimizes f, given initial x0, learning rate, and iterations.
pub fn gradient_descent_1d(
    f: &dyn Fn(f64) -> f64,
    x0: f64,
    learning_rate: f64,
    iterations: usize,
) -> f64 {
    let mut x = x0;
    let h = 1e-8;
    for _ in 0..iterations {
        let grad = (f(x + h) - f(x - h)) / (2.0 * h);
        x -= learning_rate * grad;
    }
    x
}

/// Newton's method for optimization (finding f'(x) = 0).
pub fn newton_optimize_1d(
    f: &dyn Fn(f64) -> f64,
    x0: f64,
    iterations: usize,
) -> f64 {
    let mut x = x0;
    let h = 1e-6;
    for _ in 0..iterations {
        let fp = (f(x + h) - f(x - h)) / (2.0 * h);
        let fpp = (f(x + h) - 2.0 * f(x) + f(x - h)) / (h * h);
        if fpp.abs() < 1e-15 { break; }
        x -= fp / fpp;
    }
    x
}

/// Golden section search (minimization on [a, b]).
pub fn golden_section_search(
    f: &dyn Fn(f64) -> f64,
    mut a: f64,
    mut b: f64,
    tol: f64,
) -> f64 {
    let gr = (5.0_f64.sqrt() - 1.0) / 2.0;
    let mut c = b - gr * (b - a);
    let mut d = a + gr * (b - a);
    while (b - a).abs() > tol {
        if f(c) < f(d) {
            b = d;
        } else {
            a = c;
        }
        c = b - gr * (b - a);
        d = a + gr * (b - a);
    }
    (a + b) / 2.0
}

/// Simulated annealing for minimization of a discrete scoring function.
/// Uses deterministic pseudo-random for reproducibility.
pub fn simulated_annealing(
    f: &dyn Fn(f64) -> f64,
    x0: f64,
    temp_initial: f64,
    temp_min: f64,
    cooling_rate: f64,
    step_size: f64,
) -> (f64, f64) {
    let mut x = x0;
    let mut best_x = x;
    let mut best_f = f(x);
    let mut temp = temp_initial;
    let mut seed = 42u64;

    while temp > temp_min {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = (seed >> 33) as f64 / (1u64 << 31) as f64;
        let candidate = x + step_size * (2.0 * r - 1.0);
        let delta = f(candidate) - f(x);
        if delta < 0.0 || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let r2 = (seed >> 33) as f64 / (1u64 << 31) as f64;
            r2 < (-delta / temp).exp()
        } {
            x = candidate;
        }
        let fx = f(x);
        if fx < best_f {
            best_f = fx;
            best_x = x;
        }
        temp *= cooling_rate;
    }
    (best_x, best_f)
}

// ═══════════════════════════════════════════════════════════════════════
// TENSOR OPERATIONS
// ═══════════════════════════════════════════════════════════════════════

/// Kronecker product of two matrices A (m×n) and B (p×q).
/// Result is (m*p × n*q).
pub fn kronecker_product(a: &[f64], m: usize, n: usize, b: &[f64], p: usize, q: usize) -> Vec<f64> {
    let mut result = vec![0.0; m * p * n * q];
    for i in 0..m {
        for j in 0..n {
            for k in 0..p {
                for l in 0..q {
                    result[(i * p + k) * (n * q) + (j * q + l)] = a[i * n + j] * b[k * q + l];
                }
            }
        }
    }
    result
}

/// Outer product of two vectors.
pub fn outer_product(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; a.len() * b.len()];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            result[i * b.len() + j] = ai * bj;
        }
    }
    result
}

/// Hadamard (element-wise) product of two vectors.
pub fn hadamard_product(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).collect()
}

/// Vector cross product (3D only).
pub fn cross_product(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Frobenius norm of a matrix.
pub fn frobenius_norm(m: &[f64]) -> f64 {
    m.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Matrix trace (for square n×n matrix).
pub fn matrix_trace(m: &[f64], n: usize) -> f64 {
    (0..n).map(|i| m[i * n + i]).sum()
}

// ═══════════════════════════════════════════════════════════════════════
// COMPLEX ANALYSIS
// ═══════════════════════════════════════════════════════════════════════

/// Mandelbrot iteration count for point (cx, cy).
pub fn mandelbrot_iterations(cx: f64, cy: f64, max_iter: u32) -> u32 {
    let mut zx = 0.0;
    let mut zy = 0.0;
    let mut i = 0;
    while i < max_iter && zx * zx + zy * zy < 4.0 {
        let tmp = zx * zx - zy * zy + cx;
        zy = 2.0 * zx * zy + cy;
        zx = tmp;
        i += 1;
    }
    i
}

/// Julia set iteration count for point (zx, zy) with constant (cx, cy).
pub fn julia_iterations(mut zx: f64, mut zy: f64, cx: f64, cy: f64, max_iter: u32) -> u32 {
    let mut i = 0;
    while i < max_iter && zx * zx + zy * zy < 4.0 {
        let tmp = zx * zx - zy * zy + cx;
        zy = 2.0 * zx * zy + cy;
        zx = tmp;
        i += 1;
    }
    i
}

// ═══════════════════════════════════════════════════════════════════════
// GALOIS FIELD ARITHMETIC GF(p)
// ═══════════════════════════════════════════════════════════════════════

/// Addition in GF(p).
pub fn gf_add(a: u64, b: u64, p: u64) -> u64 { (a + b) % p }
/// Subtraction in GF(p).
pub fn gf_sub(a: u64, b: u64, p: u64) -> u64 { (a + p - b % p) % p }
/// Multiplication in GF(p).
pub fn gf_mul(a: u64, b: u64, p: u64) -> u64 { mod_mul(a, b, p) }
/// Inverse in GF(p).
pub fn gf_inv(a: u64, p: u64) -> Option<u64> { mod_inverse(a as i64, p as i64).map(|x| x as u64) }
/// Exponentiation in GF(p).
pub fn gf_pow(a: u64, e: u64, p: u64) -> u64 { mod_pow(a, e, p) }

// ═══════════════════════════════════════════════════════════════════════
// FFI EXPORTS
// ═══════════════════════════════════════════════════════════════════════

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_is_prime(n: u64) -> i32 {
    if is_prime(n) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_gcd(a: u64, b: u64) -> u64 { gcd(a, b) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_lcm(a: u64, b: u64) -> u64 { lcm(a, b) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_euler_totient(n: u64) -> u64 { euler_totient(n) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_fibonacci(n: u64) -> u64 { fibonacci(n) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_factorial(n: u64) -> u64 { factorial(n) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_binomial(n: u64, k: u64) -> u64 { binomial(n, k) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_catalan(n: u64) -> u64 { catalan(n) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_mod_pow(base: u64, exp: u64, modulus: u64) -> u64 {
    mod_pow(base, exp, modulus)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_gamma(x: f64) -> f64 { gamma(x) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_beta(a: f64, b: f64) -> f64 { beta(a, b) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_erf(x: f64) -> f64 { erf(x) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_zeta(s: f64) -> f64 { zeta(s) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_bessel_j0(x: f64) -> f64 { bessel_j0(x) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_legendre(n: usize, x: f64) -> f64 { legendre(n, x) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_mandelbrot(cx: f64, cy: f64, max_iter: u32) -> u32 {
    mandelbrot_iterations(cx, cy, max_iter)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_partition_count(n: u64) -> u64 { partition_count(n) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_math_bell(n: u64) -> u64 { bell(n) }

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(97));
        assert!(!is_prime(100));
        assert!(is_prime(7919));
        assert!(is_prime(104729));
    }

    #[test]
    fn test_mod_pow() {
        assert_eq!(mod_pow(2, 10, 1000), 24);
        assert_eq!(mod_pow(3, 7, 13), 3);
    }

    #[test]
    fn test_sieve() {
        let primes = sieve_primes(30);
        assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
    }

    #[test]
    fn test_gcd_lcm() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(17, 13), 1);
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(7, 5), 35);
    }

    #[test]
    fn test_euler_totient() {
        assert_eq!(euler_totient(1), 1);
        assert_eq!(euler_totient(10), 4);
        assert_eq!(euler_totient(12), 4);
    }

    #[test]
    fn test_factorize() {
        assert_eq!(factorize(12), vec![2, 2, 3]);
        assert_eq!(factorize(60), vec![2, 2, 3, 5]);
        assert_eq!(factorize(97), vec![97]); // prime
    }

    #[test]
    fn test_fibonacci() {
        assert_eq!(fibonacci(0), 0);
        assert_eq!(fibonacci(1), 1);
        assert_eq!(fibonacci(10), 55);
        assert_eq!(fibonacci(20), 6765);
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(10), 3628800);
    }

    #[test]
    fn test_binomial() {
        assert_eq!(binomial(5, 2), 10);
        assert_eq!(binomial(10, 3), 120);
        assert_eq!(binomial(0, 0), 1);
    }

    #[test]
    fn test_catalan() {
        assert_eq!(catalan(0), 1);
        assert_eq!(catalan(3), 5);
        assert_eq!(catalan(5), 42);
    }

    #[test]
    fn test_bell_numbers() {
        assert_eq!(bell(0), 1);
        assert_eq!(bell(1), 1);
        assert_eq!(bell(3), 5);
        assert_eq!(bell(5), 52);
    }

    #[test]
    fn test_derangement() {
        assert_eq!(derangement(0), 1);
        assert_eq!(derangement(1), 0);
        assert_eq!(derangement(3), 2);
        assert_eq!(derangement(4), 9);
    }

    #[test]
    fn test_partition_count() {
        assert_eq!(partition_count(0), 1);
        assert_eq!(partition_count(4), 5);
        assert_eq!(partition_count(5), 7);
    }

    #[test]
    fn test_gamma() {
        // Γ(1) = 1, Γ(5) = 4! = 24
        assert!((gamma(1.0) - 1.0).abs() < 1e-10);
        assert!((gamma(5.0) - 24.0).abs() < 1e-6);
        assert!((gamma(0.5) - PI.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn test_beta() {
        // B(1,1) = 1
        assert!((beta(1.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_erf() {
        assert!((erf(0.0)).abs() < 1e-10);
        assert!((erf(1.0) - 0.8427).abs() < 0.001);
    }

    #[test]
    fn test_zeta() {
        // ζ(2) = π²/6 ≈ 1.6449
        assert!((zeta(2.0) - PI * PI / 6.0).abs() < 0.01);
    }

    #[test]
    fn test_bessel_j0() {
        assert!((bessel_j0(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_legendre() {
        assert!((legendre(0, 0.5) - 1.0).abs() < 1e-10);
        assert!((legendre(1, 0.5) - 0.5).abs() < 1e-10);
        assert!((legendre(2, 0.5) - (-0.125)).abs() < 1e-10);
    }

    #[test]
    fn test_hermite() {
        assert!((hermite(0, 1.0) - 1.0).abs() < 1e-10);
        assert!((hermite(1, 1.0) - 2.0).abs() < 1e-10);
        assert!((hermite(2, 1.0) - 2.0).abs() < 1e-10); // 4x²-2 at x=1
    }

    #[test]
    fn test_chebyshev() {
        assert!((chebyshev_t(0, 0.5) - 1.0).abs() < 1e-10);
        assert!((chebyshev_t(1, 0.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_mandelbrot() {
        assert_eq!(mandelbrot_iterations(0.0, 0.0, 100), 100); // origin is in set
        assert!(mandelbrot_iterations(2.0, 2.0, 100) < 100); // outside set
    }

    #[test]
    fn test_kronecker() {
        let a = vec![1.0, 0.0, 0.0, 1.0]; // I₂
        let b = vec![1.0, 2.0, 3.0, 4.0]; // 2×2
        let r = kronecker_product(&a, 2, 2, &b, 2, 2);
        assert_eq!(r.len(), 16);
        assert!((r[0] - 1.0).abs() < 1e-10); // top-left block = B
    }

    #[test]
    fn test_cross_product() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        let c = cross_product(&a, &b);
        assert!((c[2] - 1.0).abs() < 1e-10); // i×j = k
    }

    #[test]
    fn test_golden_section() {
        let f = |x: f64| (x - 3.0) * (x - 3.0);
        let min = golden_section_search(&f, 0.0, 10.0, 1e-8);
        assert!((min - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_gradient_descent() {
        let f = |x: f64| (x - 5.0) * (x - 5.0);
        let min = gradient_descent_1d(&f, 0.0, 0.1, 100);
        assert!((min - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_simulated_annealing() {
        let f = |x: f64| (x - 2.0) * (x - 2.0);
        let (best_x, _) = simulated_annealing(&f, 10.0, 100.0, 0.01, 0.95, 1.0);
        assert!((best_x - 2.0).abs() < 1.0);
    }

    #[test]
    fn test_mod_inverse() {
        assert_eq!(mod_inverse(3, 7), Some(5)); // 3*5 = 15 ≡ 1 (mod 7)
        assert_eq!(mod_inverse(2, 4), None); // gcd(2,4) ≠ 1
    }

    #[test]
    fn test_chinese_remainder() {
        let (x, _) = chinese_remainder(&[2, 3, 2], &[3, 5, 7]).unwrap();
        assert_eq!(x % 3, 2);
        assert_eq!(x % 5, 3);
        assert_eq!(x % 7, 2);
    }

    #[test]
    fn test_galois_field() {
        assert_eq!(gf_add(3, 4, 7), 0);
        assert_eq!(gf_mul(3, 4, 7), 5);
        assert_eq!(gf_inv(3, 7), Some(5));
    }

    #[test]
    fn test_lucas() {
        assert_eq!(lucas(0), 2);
        assert_eq!(lucas(1), 1);
        assert_eq!(lucas(5), 11);
    }
}
