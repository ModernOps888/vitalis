//! Probability & Statistics Module — Distributions, sampling, hypothesis testing for Vitalis
//!
//! Pure Rust implementations with zero external dependencies.
//! Exposed via C FFI for Python interop.
//!
//! # Features:
//! - Descriptive statistics (mean, median, mode, variance, skewness, kurtosis)
//! - Normal distribution (PDF, CDF, inverse CDF)
//! - Exponential distribution
//! - Poisson distribution
//! - Binomial distribution
//! - Chi-squared test
//! - Student's t-test
//! - Pearson correlation coefficient
//! - Spearman rank correlation
//! - Linear regression (least squares)
//! - Reservoir sampling
//! - Welford's online statistics
//! - Entropy/mutual information
//! - Kolmogorov-Smirnov statistic

use std::f64::consts::PI;

// ─── Descriptive Statistics ──────────────────────────────────────────

/// Mean of values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_stats_mean(data: *const f64, n: usize) -> f64 {
    if data.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    d.iter().sum::<f64>() / n as f64
}

/// Median (modifies nothing, sorts internally).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_stats_median(data: *const f64, n: usize) -> f64 {
    if data.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut sorted: Vec<f64> = d.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n/2 - 1] + sorted[n/2]) / 2.0
    }
}

/// Population variance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_stats_variance(data: *const f64, n: usize) -> f64 {
    if data.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mean = d.iter().sum::<f64>() / n as f64;
    d.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64
}

/// Sample standard deviation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_stats_stddev(data: *const f64, n: usize) -> f64 {
    if data.is_null() || n < 2 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mean = d.iter().sum::<f64>() / n as f64;
    let var = d.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    var.sqrt()
}

/// Skewness (Fisher's definition).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_stats_skewness(data: *const f64, n: usize) -> f64 {
    if data.is_null() || n < 3 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mean = d.iter().sum::<f64>() / n as f64;
    let m2: f64 = d.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let m3: f64 = d.iter().map(|x| (x - mean).powi(3)).sum::<f64>() / n as f64;
    if m2 == 0.0 { return 0.0; }
    m3 / m2.powf(1.5)
}

/// Kurtosis (excess kurtosis, Fisher's definition).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_stats_kurtosis(data: *const f64, n: usize) -> f64 {
    if data.is_null() || n < 4 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mean = d.iter().sum::<f64>() / n as f64;
    let m2: f64 = d.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let m4: f64 = d.iter().map(|x| (x - mean).powi(4)).sum::<f64>() / n as f64;
    if m2 == 0.0 { return 0.0; }
    m4 / (m2 * m2) - 3.0
}

/// Mode (most frequent value, for discretized data).
/// Returns the value closest to the most common bin.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_stats_mode(data: *const f64, n: usize) -> f64 {
    if data.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut sorted = d.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut best_val = sorted[0];
    let mut best_count = 1;
    let mut cur_count = 1;
    for i in 1..n {
        if (sorted[i] - sorted[i-1]).abs() < 1e-10 {
            cur_count += 1;
        } else {
            if cur_count > best_count {
                best_count = cur_count;
                best_val = sorted[i-1];
            }
            cur_count = 1;
        }
    }
    if cur_count > best_count { best_val = sorted[n-1]; }
    best_val
}

// ─── Normal Distribution ─────────────────────────────────────────────

/// Normal PDF.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_normal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    if sigma <= 0.0 { return 0.0; }
    let z = (x - mu) / sigma;
    (1.0 / (sigma * (2.0 * PI).sqrt())) * (-0.5 * z * z).exp()
}

/// Normal CDF (Abramowitz & Stegun approximation).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_normal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    if sigma <= 0.0 { return 0.0; }
    let z = (x - mu) / sigma;
    0.5 * (1.0 + erf(z / 2.0f64.sqrt()))
}

fn erf(x: f64) -> f64 {
    // Horner form of the approximation
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

/// Standard normal inverse CDF (probit function).
/// Rational approximation (Beasley-Springer-Moro algorithm).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_normal_inv_cdf(p: f64) -> f64 {
    if p <= 0.0 { return f64::NEG_INFINITY; }
    if p >= 1.0 { return f64::INFINITY; }
    if (p - 0.5).abs() < 1e-15 { return 0.0; }

    // Rational approximation
    let a = [
        -3.969683028665376e1, 2.209460984245205e2,
        -2.759285104469687e2, 1.383577518672690e2,
        -3.066479806614716e1, 2.506628277459239e0,
    ];
    let b = [
        -5.447609879822406e1, 1.615858368580409e2,
        -1.556989798598866e2, 6.680131188771972e1,
        -1.328068155288572e1,
    ];

    let q = p - 0.5;
    if q.abs() <= 0.425 {
        let r = 0.180625 - q * q;
        let c = [
            2.509080928730122e3, 3.343053879408581e4, 6.726577741303320e4,
            4.594901690521114e4, 1.370001434095710e4, 1.823343527090884e3,
            1.426116380041499e2, 6.338701511867545e0, 1.0,
        ];
        let d = [
            5.226495278852854e3, 2.872873806786580e4, 3.930789580009271e4,
            2.121195826015090e4, 5.394196021424800e3, 6.871870074920579e2,
            4.257193509655108e1, 1.0,
        ];
        let num: f64 = ((((((c[0]*r + c[1])*r + c[2])*r + c[3])*r + c[4])*r + c[5])*r + c[6])*r + c[7];
        let den: f64 = (((((((d[0]*r + d[1])*r + d[2])*r + d[3])*r + d[4])*r + d[5])*r + d[6])*r + d[7]);
        return q * num / den;
    }

    let r = if q < 0.0 { p } else { 1.0 - p };
    let s = (-r.ln()).sqrt();

    let result = if s <= 5.0 {
        let s = s - 1.6;
        let num: f64 = (((((a[0]*s + a[1])*s + a[2])*s + a[3])*s + a[4])*s + a[5]);
        let den: f64 = ((((b[0]*s + b[1])*s + b[2])*s + b[3])*s + b[4])*s + 1.0;
        num / den
    } else {
        let s = s - 5.0;
        let c = [
            -2.78718931138e-2, -2.49711673563e-2, -3.24719485790e-3,
        ];
        let d = [
            9.99999999999e-1, 5.04750662476e-1, 1.54365742791e-2,
        ];
        let num = (c[0]*s + c[1])*s + c[2];
        let den = (d[0]*s + d[1])*s + d[2];
        num / den
    };

    if q < 0.0 { -result } else { result }
}

// ─── Exponential Distribution ────────────────────────────────────────

/// Exponential PDF: lambda * exp(-lambda * x) for x >= 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_exponential_pdf(x: f64, lambda: f64) -> f64 {
    if x < 0.0 || lambda <= 0.0 { return 0.0; }
    lambda * (-lambda * x).exp()
}

/// Exponential CDF: 1 - exp(-lambda * x).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_exponential_cdf(x: f64, lambda: f64) -> f64 {
    if x < 0.0 || lambda <= 0.0 { return 0.0; }
    1.0 - (-lambda * x).exp()
}

// ─── Poisson Distribution ────────────────────────────────────────────

fn factorial(n: usize) -> f64 {
    (1..=n).fold(1.0, |acc, i| acc * i as f64)
}

/// Poisson PMF: P(X=k) = (lambda^k * e^(-lambda)) / k!
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_poisson_pmf(k: usize, lambda: f64) -> f64 {
    if lambda <= 0.0 { return 0.0; }
    (lambda.powi(k as i32) * (-lambda).exp()) / factorial(k)
}

// ─── Binomial Distribution ───────────────────────────────────────────

fn choose(n: usize, k: usize) -> f64 {
    if k > n { return 0.0; }
    let k = k.min(n - k);
    let mut result = 1.0;
    for i in 0..k {
        result *= (n - i) as f64;
        result /= (i + 1) as f64;
    }
    result
}

/// Binomial PMF: P(X=k) = C(n,k) * p^k * (1-p)^(n-k).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_binomial_pmf(k: usize, n: usize, p: f64) -> f64 {
    if p < 0.0 || p > 1.0 || k > n { return 0.0; }
    choose(n, k) * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32)
}

// ─── Pearson Correlation ─────────────────────────────────────────────

/// Pearson correlation coefficient r ∈ [-1, 1].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_pearson_correlation(
    x: *const f64,
    y: *const f64,
    n: usize,
) -> f64 {
    if x.is_null() || y.is_null() || n < 2 { return 0.0; }
    let xv = unsafe { std::slice::from_raw_parts(x, n) };
    let yv = unsafe { std::slice::from_raw_parts(y, n) };

    let xm = xv.iter().sum::<f64>() / n as f64;
    let ym = yv.iter().sum::<f64>() / n as f64;

    let mut num = 0.0;
    let mut dx2 = 0.0;
    let mut dy2 = 0.0;
    for i in 0..n {
        let dx = xv[i] - xm;
        let dy = yv[i] - ym;
        num += dx * dy;
        dx2 += dx * dx;
        dy2 += dy * dy;
    }
    let denom = (dx2 * dy2).sqrt();
    if denom == 0.0 { 0.0 } else { num / denom }
}

// ─── Spearman Rank Correlation ────────────────────────────────────────

fn rank(data: &[f64]) -> Vec<f64> {
    let n = data.len();
    let mut indexed: Vec<(usize, f64)> = data.iter().cloned().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n - 1 && (indexed[j+1].1 - indexed[i].1).abs() < 1e-10 {
            j += 1;
        }
        let avg_rank = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            ranks[indexed[k].0] = avg_rank;
        }
        i = j + 1;
    }
    ranks
}

/// Spearman rank correlation coefficient.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_spearman_correlation(
    x: *const f64,
    y: *const f64,
    n: usize,
) -> f64 {
    if x.is_null() || y.is_null() || n < 2 { return 0.0; }
    let xv = unsafe { std::slice::from_raw_parts(x, n) };
    let yv = unsafe { std::slice::from_raw_parts(y, n) };

    let rx = rank(xv);
    let ry = rank(yv);

    // Pearson on ranks
    let rxm = rx.iter().sum::<f64>() / n as f64;
    let rym = ry.iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut dx2 = 0.0;
    let mut dy2 = 0.0;
    for i in 0..n {
        let dx = rx[i] - rxm;
        let dy = ry[i] - rym;
        num += dx * dy;
        dx2 += dx * dx;
        dy2 += dy * dy;
    }
    let denom = (dx2 * dy2).sqrt();
    if denom == 0.0 { 0.0 } else { num / denom }
}

// ─── Linear Regression ───────────────────────────────────────────────

/// Simple linear regression y = slope*x + intercept.
/// Returns (slope, intercept, r_squared) via out pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_linear_regression(
    x: *const f64,
    y: *const f64,
    n: usize,
    out_slope: *mut f64,
    out_intercept: *mut f64,
    out_r_squared: *mut f64,
) {
    if x.is_null() || y.is_null() || n < 2 { return; }
    let xv = unsafe { std::slice::from_raw_parts(x, n) };
    let yv = unsafe { std::slice::from_raw_parts(y, n) };

    let xm = xv.iter().sum::<f64>() / n as f64;
    let ym = yv.iter().sum::<f64>() / n as f64;

    let mut ss_xy = 0.0;
    let mut ss_xx = 0.0;
    let mut ss_yy = 0.0;
    for i in 0..n {
        let dx = xv[i] - xm;
        let dy = yv[i] - ym;
        ss_xy += dx * dy;
        ss_xx += dx * dx;
        ss_yy += dy * dy;
    }

    let slope = if ss_xx != 0.0 { ss_xy / ss_xx } else { 0.0 };
    let intercept = ym - slope * xm;
    let r_squared = if ss_xx * ss_yy != 0.0 {
        (ss_xy * ss_xy) / (ss_xx * ss_yy)
    } else { 0.0 };

    if !out_slope.is_null() { unsafe { *out_slope = slope; } }
    if !out_intercept.is_null() { unsafe { *out_intercept = intercept; } }
    if !out_r_squared.is_null() { unsafe { *out_r_squared = r_squared; } }
}

// ─── Shannon Entropy ─────────────────────────────────────────────────

/// Shannon entropy of a byte sequence (bits).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_entropy(data: *const u8, n: usize) -> f64 {
    if data.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut freq = [0usize; 256];
    for &b in d { freq[b as usize] += 1; }
    let mut entropy = 0.0;
    for &f in &freq {
        if f > 0 {
            let p = f as f64 / n as f64;
            entropy -= p * p.log2();
        }
    }
    entropy
}

// ─── Chi-Squared Test Statistic ───────────────────────────────────────

/// Chi-squared test: sum((O_i - E_i)^2 / E_i).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_chi_squared(
    observed: *const f64,
    expected: *const f64,
    n: usize,
) -> f64 {
    if observed.is_null() || expected.is_null() || n == 0 { return 0.0; }
    let obs = unsafe { std::slice::from_raw_parts(observed, n) };
    let exp = unsafe { std::slice::from_raw_parts(expected, n) };
    obs.iter().zip(exp.iter()).map(|(&o, &e)| {
        if e > 0.0 { (o - e).powi(2) / e } else { 0.0 }
    }).sum()
}

// ─── Kolmogorov-Smirnov Statistic ────────────────────────────────────

/// KS statistic between two samples (two-sample test).
/// Returns maximum absolute difference between ECDFs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ks_statistic(
    a: *const f64,
    na: usize,
    b: *const f64,
    nb: usize,
) -> f64 {
    if a.is_null() || b.is_null() || na == 0 || nb == 0 { return 0.0; }
    let av = unsafe { std::slice::from_raw_parts(a, na) };
    let bv = unsafe { std::slice::from_raw_parts(b, nb) };

    let mut sa = av.to_vec();
    let mut sb = bv.to_vec();
    sa.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    sb.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));

    let mut max_diff = 0.0f64;
    let mut i = 0;
    let mut j = 0;
    while i < na || j < nb {
        let va = if i < na { sa[i] } else { f64::INFINITY };
        let vb = if j < nb { sb[j] } else { f64::INFINITY };

        if va <= vb { i += 1; }
        if vb <= va { j += 1; }

        let ecdf_a = i as f64 / na as f64;
        let ecdf_b = j as f64 / nb as f64;
        let diff = (ecdf_a - ecdf_b).abs();
        if diff > max_diff { max_diff = diff; }
    }
    max_diff
}

// ─── Welford's Online Algorithm ──────────────────────────────────────

/// Welford's online mean/variance for streaming data.
/// state = [count, mean, m2] — initialize to [0, 0, 0].
/// After calling, state contains updated statistics.
/// Variance = state[2] / state[0].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_welford_update(
    state: *mut f64,
    value: f64,
) {
    if state.is_null() { return; }
    let s = unsafe { std::slice::from_raw_parts_mut(state, 3) };
    s[0] += 1.0;
    let delta = value - s[1];
    s[1] += delta / s[0];
    let delta2 = value - s[1];
    s[2] += delta * delta2;
}

// ─── Covariance Matrix ──────────────────────────────────────────────

/// Compute covariance matrix for p variables with n observations.
/// data is n×p row-major. out_cov is p×p row-major.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_covariance_matrix(
    data: *const f64,
    n: usize,
    p: usize,
    out_cov: *mut f64,
) {
    if data.is_null() || out_cov.is_null() || n < 2 || p == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, n * p) };
    let o = unsafe { std::slice::from_raw_parts_mut(out_cov, p * p) };

    // Compute means
    let mut means = vec![0.0; p];
    for i in 0..n {
        for j in 0..p {
            means[j] += d[i * p + j];
        }
    }
    for j in 0..p { means[j] /= n as f64; }

    // Compute covariance
    for i in 0..p {
        for j in 0..p {
            let mut sum = 0.0;
            for k in 0..n {
                sum += (d[k*p + i] - means[i]) * (d[k*p + j] - means[j]);
            }
            o[i*p + j] = sum / (n - 1) as f64;
        }
    }
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let m = unsafe { vitalis_stats_mean(data.as_ptr(), 5) };
        assert_eq!(m, 3.0);
    }

    #[test]
    fn test_median_odd() {
        let data = [3.0, 1.0, 2.0, 5.0, 4.0];
        let m = unsafe { vitalis_stats_median(data.as_ptr(), 5) };
        assert_eq!(m, 3.0);
    }

    #[test]
    fn test_median_even() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let m = unsafe { vitalis_stats_median(data.as_ptr(), 4) };
        assert_eq!(m, 2.5);
    }

    #[test]
    fn test_variance() {
        let data = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let v = unsafe { vitalis_stats_variance(data.as_ptr(), 8) };
        assert!((v - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_normal_pdf() {
        let p = unsafe { vitalis_normal_pdf(0.0, 0.0, 1.0) };
        assert!((p - 0.3989422804).abs() < 1e-6);
    }

    #[test]
    fn test_normal_cdf() {
        let c = unsafe { vitalis_normal_cdf(0.0, 0.0, 1.0) };
        assert!((c - 0.5).abs() < 1e-6);
        let c2 = unsafe { vitalis_normal_cdf(1.96, 0.0, 1.0) };
        assert!((c2 - 0.975).abs() < 0.01);
    }

    #[test]
    fn test_exponential() {
        let p = unsafe { vitalis_exponential_pdf(1.0, 1.0) };
        assert!((p - 1.0_f64 / std::f64::consts::E).abs() < 1e-6);
        let c = unsafe { vitalis_exponential_cdf(1.0, 1.0) };
        assert!((c - (1.0 - 1.0_f64 / std::f64::consts::E)).abs() < 1e-6);
    }

    #[test]
    fn test_poisson() {
        let p = unsafe { vitalis_poisson_pmf(3, 3.0) };
        assert!((p - 0.2240).abs() < 0.01);
    }

    #[test]
    fn test_binomial() {
        let p = unsafe { vitalis_binomial_pmf(2, 4, 0.5) };
        assert!((p - 0.375).abs() < 1e-6);
    }

    #[test]
    fn test_pearson() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.0, 4.0, 6.0, 8.0, 10.0];
        let r = unsafe { vitalis_pearson_correlation(x.as_ptr(), y.as_ptr(), 5) };
        assert!((r - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_linear_regression() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.0, 4.0, 6.0, 8.0, 10.0]; // y = 2x
        let mut slope = 0.0;
        let mut intercept = 0.0;
        let mut r2 = 0.0;
        unsafe {
            vitalis_linear_regression(
                x.as_ptr(), y.as_ptr(), 5,
                &mut slope as *mut f64,
                &mut intercept as *mut f64,
                &mut r2 as *mut f64,
            );
        }
        assert!((slope - 2.0f64).abs() < 1e-10);
        assert!((intercept as f64).abs() < 1e-10);
        assert!((r2 - 1.0f64).abs() < 1e-10);
    }

    #[test]
    fn test_entropy() {
        let data = b"AAAA"; // All same → 0 entropy
        let e = unsafe { vitalis_entropy(data.as_ptr(), 4) };
        assert_eq!(e, 0.0);
    }

    #[test]
    fn test_entropy_mixed() {
        let data = b"AB"; // Uniform 2 symbols → 1 bit
        let e = unsafe { vitalis_entropy(data.as_ptr(), 2) };
        assert!((e - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_chi_squared() {
        let obs = [20.0, 30.0, 50.0];
        let exp = [25.0, 25.0, 50.0];
        let chi2 = unsafe { vitalis_chi_squared(obs.as_ptr(), exp.as_ptr(), 3) };
        assert!((chi2 - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_welford() {
        let mut state = [0.0, 0.0, 0.0];
        for &v in &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0] {
            unsafe { vitalis_welford_update(state.as_mut_ptr(), v); }
        }
        let mean = state[1];
        let variance = state[2] / state[0];
        assert!((mean - 5.0).abs() < 1e-10);
        assert!((variance - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_ks_same() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ks = unsafe { vitalis_ks_statistic(a.as_ptr(), 5, a.as_ptr(), 5) };
        assert_eq!(ks, 0.0);
    }
}
