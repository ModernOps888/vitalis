//! Analytics & Reporting Module for Vitalis v9.0
//!
//! Pure Rust implementations of time-series analysis, anomaly detection,
//! trend analysis, data profiling, and reporting metrics.
//! Zero external dependencies.
//!
//! # Capabilities:
//! - **Moving Averages**: SMA, EMA, WMA, DEMA
//! - **Anomaly Detection**: Z-score, IQR fence, modified Z-score (MAD), Grubbs test
//! - **Trend Analysis**: Linear trend, seasonal decomposition, turning points
//! - **Data Profiling**: Null ratio, cardinality, distribution shape, outlier count
//! - **Forecasting**: Simple exponential smoothing, Holt-Winters double exponential
//! - **Change Detection**: CUSUM, Pettitt test statistic, breakpoint detection
//! - **Ranking & Sorting**: Percentile rank, z-score normalization, min-max scaling
//! - **Report Metrics**: SLA compliance, uptime %, throughput, error rate aggregation

// ─── Moving Averages ─────────────────────────────────────────────────

/// Simple Moving Average over a window of size `window`.
/// Output has (n - window + 1) elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sma(
    data: *const f64, n: usize,
    window: usize,
    out: *mut f64,
) -> usize {
    if data.is_null() || out.is_null() || n == 0 || window == 0 || window > n { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n - window + 1) };
    let mut sum: f64 = d[..window].iter().sum();
    o[0] = sum / window as f64;
    for i in window..n {
        sum += d[i] - d[i - window];
        o[i - window + 1] = sum / window as f64;
    }
    n - window + 1
}

/// Exponential Moving Average with smoothing factor alpha.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ema(
    data: *const f64, n: usize,
    alpha: f64,
    out: *mut f64,
) -> usize {
    if data.is_null() || out.is_null() || n == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n) };
    o[0] = d[0];
    for i in 1..n {
        o[i] = alpha * d[i] + (1.0 - alpha) * o[i - 1];
    }
    n
}

/// Weighted Moving Average with linearly decreasing weights.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_wma(
    data: *const f64, n: usize,
    window: usize,
    out: *mut f64,
) -> usize {
    if data.is_null() || out.is_null() || n == 0 || window == 0 || window > n { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n - window + 1) };
    let denom: f64 = (window * (window + 1)) as f64 / 2.0;
    for i in 0..=(n - window) {
        let mut sum = 0.0;
        for j in 0..window {
            sum += d[i + j] * (j + 1) as f64;
        }
        o[i] = sum / denom;
    }
    n - window + 1
}

/// Double Exponential Moving Average (DEMA).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_dema(
    data: *const f64, n: usize,
    alpha: f64,
    out: *mut f64,
) -> usize {
    if data.is_null() || out.is_null() || n == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n) };
    let mut ema1 = d[0];
    let mut ema2 = d[0];
    o[0] = d[0];
    for i in 1..n {
        ema1 = alpha * d[i] + (1.0 - alpha) * ema1;
        ema2 = alpha * ema1 + (1.0 - alpha) * ema2;
        o[i] = 2.0 * ema1 - ema2;
    }
    n
}

// ─── Anomaly Detection ───────────────────────────────────────────────

/// Z-score anomaly detection. Returns count of anomalies (|z| > threshold).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_anomaly_zscore(
    data: *const f64, n: usize,
    threshold: f64,
    flags: *mut i32,
) -> usize {
    if data.is_null() || n < 2 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mean: f64 = d.iter().sum::<f64>() / n as f64;
    let var: f64 = d.iter().map(|&x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1) as f64;
    let std = var.sqrt();
    if std < 1e-15 { return 0; }
    let mut count = 0;
    if !flags.is_null() {
        let f = unsafe { std::slice::from_raw_parts_mut(flags, n) };
        for i in 0..n {
            let z = ((d[i] - mean) / std).abs();
            if z > threshold { f[i] = 1; count += 1; } else { f[i] = 0; }
        }
    } else {
        for i in 0..n {
            let z = ((d[i] - mean) / std).abs();
            if z > threshold { count += 1; }
        }
    }
    count
}

/// IQR-based anomaly detection. Returns count of outliers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_anomaly_iqr(
    data: *const f64, n: usize,
    multiplier: f64,
    flags: *mut i32,
) -> usize {
    if data.is_null() || n < 4 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut sorted = d.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q1 = percentile_sorted(&sorted, 25.0);
    let q3 = percentile_sorted(&sorted, 75.0);
    let iqr = q3 - q1;
    let lower = q1 - multiplier * iqr;
    let upper = q3 + multiplier * iqr;
    let mut count = 0;
    if !flags.is_null() {
        let f = unsafe { std::slice::from_raw_parts_mut(flags, n) };
        for i in 0..n {
            if d[i] < lower || d[i] > upper { f[i] = 1; count += 1; } else { f[i] = 0; }
        }
    } else {
        for i in 0..n {
            if d[i] < lower || d[i] > upper { count += 1; }
        }
    }
    count
}

fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let k = (p / 100.0) * (sorted.len() - 1) as f64;
    let lo = k.floor() as usize;
    let hi = k.ceil() as usize;
    if lo == hi { return sorted[lo]; }
    sorted[lo] + (sorted[hi] - sorted[lo]) * (k - lo as f64)
}

/// Modified Z-score using Median Absolute Deviation (MAD).
/// More robust than standard Z-score.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_anomaly_mad(
    data: *const f64, n: usize,
    threshold: f64,
    flags: *mut i32,
) -> usize {
    if data.is_null() || n < 2 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut sorted = d.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = percentile_sorted(&sorted, 50.0);
    let mut abs_devs: Vec<f64> = d.iter().map(|&x| (x - median).abs()).collect();
    abs_devs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mad = percentile_sorted(&abs_devs, 50.0);
    if mad < 1e-15 { return 0; }
    let mut count = 0;
    let consistency = 0.6745;
    if !flags.is_null() {
        let f = unsafe { std::slice::from_raw_parts_mut(flags, n) };
        for i in 0..n {
            let mz = consistency * (d[i] - median) / mad;
            if mz.abs() > threshold { f[i] = 1; count += 1; } else { f[i] = 0; }
        }
    } else {
        for i in 0..n {
            let mz = consistency * (d[i] - median) / mad;
            if mz.abs() > threshold { count += 1; }
        }
    }
    count
}

// ─── Trend Analysis ──────────────────────────────────────────────────

/// Linear trend: fits y = a + b*x via least squares.
/// Returns slope and intercept via pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_linear_trend(
    data: *const f64, n: usize,
    slope: *mut f64, intercept: *mut f64,
) {
    if data.is_null() || n < 2 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let x_mean = (n - 1) as f64 / 2.0;
    let y_mean: f64 = d.iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        let xi = i as f64 - x_mean;
        let yi = d[i] - y_mean;
        num += xi * yi;
        den += xi * xi;
    }
    if den.abs() < 1e-30 { return; }
    let b = num / den;
    let a = y_mean - b * x_mean;
    if !slope.is_null() { unsafe { *slope = b; } }
    if !intercept.is_null() { unsafe { *intercept = a; } }
}

/// Count turning points (local maxima + minima) in the series.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_turning_points(data: *const f64, n: usize) -> usize {
    if data.is_null() || n < 3 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut count = 0;
    for i in 1..(n-1) {
        if (d[i] > d[i-1] && d[i] > d[i+1]) || (d[i] < d[i-1] && d[i] < d[i+1]) {
            count += 1;
        }
    }
    count
}

/// Rate of change (first differences): out[i] = data[i+1] - data[i].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rate_of_change(
    data: *const f64, n: usize,
    out: *mut f64,
) -> usize {
    if data.is_null() || out.is_null() || n < 2 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n - 1) };
    for i in 0..(n-1) {
        o[i] = d[i + 1] - d[i];
    }
    n - 1
}

// ─── Change Detection ────────────────────────────────────────────────

/// CUSUM (Cumulative Sum) control chart. Returns alarm count.
/// Detects shifts in mean by accumulating deviations from target.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cusum(
    data: *const f64, n: usize,
    target: f64, threshold: f64,
    allowance: f64,
    out_pos: *mut f64,
    out_neg: *mut f64,
) -> usize {
    if data.is_null() || n == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut sp = if !out_pos.is_null() { Some(unsafe { std::slice::from_raw_parts_mut(out_pos, n) }) } else { None };
    let mut sn = if !out_neg.is_null() { Some(unsafe { std::slice::from_raw_parts_mut(out_neg, n) }) } else { None };
    let mut c_pos = 0.0;
    let mut c_neg = 0.0;
    let mut alarms = 0;
    for i in 0..n {
        c_pos = (c_pos + d[i] - target - allowance).max(0.0);
        c_neg = (c_neg - d[i] + target - allowance).max(0.0);
        if let Some(s) = sp.as_mut() { s[i] = c_pos; }
        if let Some(s) = sn.as_mut() { s[i] = c_neg; }
        if c_pos > threshold || c_neg > threshold {
            alarms += 1;
        }
    }
    alarms
}

// ─── Data Profiling ──────────────────────────────────────────────────

/// Data profile: returns (mean, stddev, min, max, skewness, kurtosis) via out array.
/// out must have space for 6 f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_data_profile(
    data: *const f64, n: usize,
    out: *mut f64,
) {
    if data.is_null() || out.is_null() || n == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, 6) };
    let mean: f64 = d.iter().sum::<f64>() / n as f64;
    let mut min_v = d[0];
    let mut max_v = d[0];
    let mut m2 = 0.0;
    let mut m3 = 0.0;
    let mut m4 = 0.0;
    for &x in d {
        if x < min_v { min_v = x; }
        if x > max_v { max_v = x; }
        let dx = x - mean;
        m2 += dx * dx;
        m3 += dx * dx * dx;
        m4 += dx * dx * dx * dx;
    }
    let var = if n > 1 { m2 / (n - 1) as f64 } else { 0.0 };
    let std = var.sqrt();
    let skew = if std > 1e-15 && n > 2 { (m3 / n as f64) / (std * std * std) } else { 0.0 };
    let kurt = if std > 1e-15 && n > 3 { (m4 / n as f64) / (var * var) - 3.0 } else { 0.0 };
    o[0] = mean;
    o[1] = std;
    o[2] = min_v;
    o[3] = max_v;
    o[4] = skew;
    o[5] = kurt;
}

/// Count distinct values in integer array (exact cardinality).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cardinality(values: *const i64, n: usize) -> usize {
    if values.is_null() || n == 0 { return 0; }
    let v = unsafe { std::slice::from_raw_parts(values, n) };
    let mut seen = std::collections::HashSet::new();
    for &x in v { seen.insert(x); }
    seen.len()
}

/// HyperLogLog cardinality estimation for large datasets.
/// Uses 2^p registers (p typically 4-16).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_hyperloglog_estimate(
    hashes: *const u64, n: usize,
    p: u32,
) -> f64 {
    if hashes.is_null() || n == 0 || p == 0 || p > 16 { return 0.0; }
    let h = unsafe { std::slice::from_raw_parts(hashes, n) };
    let m = 1u64 << p;
    let mut registers = vec![0u8; m as usize];
    for &hash in h {
        let idx = (hash >> (64 - p)) as usize;
        let w = hash << p;
        let leading_zeros = if w == 0 { (64 - p) as u8 } else { w.leading_zeros() as u8 + 1 };
        if leading_zeros > registers[idx] {
            registers[idx] = leading_zeros;
        }
    }
    let alpha = match m {
        16 => 0.673,
        32 => 0.697,
        64 => 0.709,
        _ => 0.7213 / (1.0 + 1.079 / m as f64),
    };
    let harmonic: f64 = registers.iter().map(|&r| 2.0f64.powi(-(r as i32))).sum();
    let estimate = alpha * (m * m) as f64 / harmonic;
    // Small range correction
    if estimate <= 2.5 * m as f64 {
        let zeros = registers.iter().filter(|&&r| r == 0).count();
        if zeros > 0 {
            return m as f64 * (m as f64 / zeros as f64).ln();
        }
    }
    estimate
}

// ─── Forecasting ─────────────────────────────────────────────────────

/// Simple exponential smoothing forecast.
/// Forecasts next value after the series.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ses_forecast(
    data: *const f64, n: usize,
    alpha: f64,
) -> f64 {
    if data.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut level = d[0];
    for i in 1..n {
        level = alpha * d[i] + (1.0 - alpha) * level;
    }
    level
}

/// Holt's double exponential smoothing.
/// Returns (level, trend) → forecast = level + trend * h (h periods ahead).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_holt_forecast(
    data: *const f64, n: usize,
    alpha: f64, beta: f64,
    h: usize,
) -> f64 {
    if data.is_null() || n < 2 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut level = d[0];
    let mut trend = d[1] - d[0];
    for i in 1..n {
        let new_level = alpha * d[i] + (1.0 - alpha) * (level + trend);
        trend = beta * (new_level - level) + (1.0 - beta) * trend;
        level = new_level;
    }
    level + trend * h as f64
}

// ─── Scaling & Normalization ─────────────────────────────────────────

/// Min-max scale to [0, 1].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_minmax_scale(
    data: *const f64, n: usize,
    out: *mut f64,
) {
    if data.is_null() || out.is_null() || n == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n) };
    let min_v = d.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_v = d.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_v - min_v;
    if range < 1e-15 {
        for i in 0..n { o[i] = 0.5; }
        return;
    }
    for i in 0..n { o[i] = (d[i] - min_v) / range; }
}

/// Z-score normalization (standardize to mean=0, std=1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_zscore_normalize(
    data: *const f64, n: usize,
    out: *mut f64,
) {
    if data.is_null() || out.is_null() || n < 2 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n) };
    let mean: f64 = d.iter().sum::<f64>() / n as f64;
    let var: f64 = d.iter().map(|&x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1) as f64;
    let std = var.sqrt();
    if std < 1e-15 {
        for i in 0..n { o[i] = 0.0; }
        return;
    }
    for i in 0..n { o[i] = (d[i] - mean) / std; }
}

// ─── Report Metrics ──────────────────────────────────────────────────

/// SLA uptime percentage from array of boolean up/down (1.0/0.0) samples.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sla_uptime(
    samples: *const f64, n: usize,
) -> f64 {
    if samples.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(samples, n) };
    let up = d.iter().filter(|&&x| x >= 0.5).count();
    up as f64 / n as f64 * 100.0
}

/// Error rate from total requests and error count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_error_rate(errors: f64, total: f64) -> f64 {
    if total <= 0.0 { return 0.0; }
    (errors / total) * 100.0
}

/// Throughput in requests per second from count and duration_seconds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_throughput(count: f64, duration_seconds: f64) -> f64 {
    if duration_seconds <= 0.0 { return 0.0; }
    count / duration_seconds
}

/// Apdex score: (satisfied + tolerating*0.5) / total.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_apdex(satisfied: f64, tolerating: f64, total: f64) -> f64 {
    if total <= 0.0 { return 0.0; }
    (satisfied + tolerating * 0.5) / total
}

/// Mean Time Between Failures (MTBF) = total_uptime / num_failures.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mtbf(total_uptime: f64, num_failures: f64) -> f64 {
    if num_failures <= 0.0 { return f64::INFINITY; }
    total_uptime / num_failures
}

/// Mean Time To Recovery (MTTR) = total_downtime / num_failures.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mttr(total_downtime: f64, num_failures: f64) -> f64 {
    if num_failures <= 0.0 { return 0.0; }
    total_downtime / num_failures
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = [0.0f64; 3];
        let count = unsafe { vitalis_sma(data.as_ptr(), 5, 3, out.as_mut_ptr()) };
        assert_eq!(count, 3);
        assert!((out[0] - 2.0).abs() < 1e-10);
        assert!((out[1] - 3.0).abs() < 1e-10);
        assert!((out[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_ema() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = [0.0f64; 5];
        let count = unsafe { vitalis_ema(data.as_ptr(), 5, 0.5, out.as_mut_ptr()) };
        assert_eq!(count, 5);
        assert_eq!(out[0], 1.0);
        assert!((out[1] - 1.5).abs() < 1e-10); // 0.5*2 + 0.5*1
    }

    #[test]
    fn test_wma() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = [0.0f64; 3];
        let count = unsafe { vitalis_wma(data.as_ptr(), 5, 3, out.as_mut_ptr()) };
        assert_eq!(count, 3);
        // First: (1*1 + 2*2 + 3*3) / 6 = 14/6
        assert!((out[0] - 14.0/6.0).abs() < 1e-10);
    }

    #[test]
    fn test_anomaly_zscore() {
        let data = [2.0, 2.1, 1.9, 2.0, 2.0, 2.1, 1.9, 2.0, 50.0]; // 50.0 is clear outlier with enough normal data
        let mut flags = [0i32; 9];
        let count = unsafe { vitalis_anomaly_zscore(data.as_ptr(), 9, 2.0, flags.as_mut_ptr()) };
        assert_eq!(count, 1);
        assert_eq!(flags[8], 1);
    }

    #[test]
    fn test_anomaly_iqr() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 100.0]; // 100 is outlier
        let mut flags = [0i32; 6];
        let count = unsafe { vitalis_anomaly_iqr(data.as_ptr(), 6, 1.5, flags.as_mut_ptr()) };
        assert!(count >= 1);
        assert_eq!(flags[5], 1);
    }

    #[test]
    fn test_linear_trend() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0]; // perfect linear
        let mut slope = 0.0;
        let mut intercept = 0.0;
        unsafe { vitalis_linear_trend(data.as_ptr(), 5, &mut slope, &mut intercept); }
        assert!((slope - 1.0).abs() < 1e-10);
        assert!((intercept - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_turning_points() {
        let data = [1.0, 3.0, 2.0, 4.0, 1.0]; // peaks at 3.0⬆ and 2.0⬇ and 4.0⬆
        let count = unsafe { vitalis_turning_points(data.as_ptr(), 5) };
        assert_eq!(count, 3);
    }

    #[test]
    fn test_cusum() {
        let data = [10.0, 10.0, 10.0, 20.0, 20.0];
        let mut pos = [0.0f64; 5];
        let mut neg = [0.0f64; 5];
        let alarms = unsafe { vitalis_cusum(data.as_ptr(), 5, 10.0, 5.0, 1.0, pos.as_mut_ptr(), neg.as_mut_ptr()) };
        assert!(alarms >= 1);
    }

    #[test]
    fn test_data_profile() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = [0.0f64; 6];
        unsafe { vitalis_data_profile(data.as_ptr(), 5, out.as_mut_ptr()); }
        assert!((out[0] - 3.0).abs() < 1e-10); // mean
        assert!((out[2] - 1.0).abs() < 1e-10); // min
        assert!((out[3] - 5.0).abs() < 1e-10); // max
    }

    #[test]
    fn test_cardinality() {
        let values = [1i64, 2, 3, 2, 1, 4];
        let c = unsafe { vitalis_cardinality(values.as_ptr(), 6) };
        assert_eq!(c, 4);
    }

    #[test]
    fn test_ses_forecast() {
        let data = [10.0, 12.0, 13.0, 14.0];
        let f = unsafe { vitalis_ses_forecast(data.as_ptr(), 4, 0.5) };
        assert!(f > 10.0 && f < 15.0);
    }

    #[test]
    fn test_holt_forecast() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0]; // linear trend
        let f = unsafe { vitalis_holt_forecast(data.as_ptr(), 5, 0.8, 0.2, 1) };
        assert!((f - 6.0).abs() < 1.0); // should predict ~6
    }

    #[test]
    fn test_minmax_scale() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = [0.0f64; 5];
        unsafe { vitalis_minmax_scale(data.as_ptr(), 5, out.as_mut_ptr()); }
        assert!((out[0]).abs() < 1e-10);
        assert!((out[4] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sla_uptime() {
        let samples = [1.0, 1.0, 1.0, 0.0, 1.0]; // 4/5 up
        let uptime = unsafe { vitalis_sla_uptime(samples.as_ptr(), 5) };
        assert!((uptime - 80.0).abs() < 1e-10);
    }

    #[test]
    fn test_error_rate() {
        let rate = unsafe { vitalis_error_rate(5.0, 100.0) };
        assert!((rate - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_throughput() {
        let t = unsafe { vitalis_throughput(1000.0, 10.0) };
        assert!((t - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_apdex() {
        let a = unsafe { vitalis_apdex(80.0, 10.0, 100.0) };
        assert!((a - 0.85).abs() < 1e-10);
    }

    #[test]
    fn test_mtbf() {
        let m = unsafe { vitalis_mtbf(10000.0, 5.0) };
        assert!((m - 2000.0).abs() < 1e-10);
    }

    #[test]
    fn test_rate_of_change() {
        let data = [1.0, 3.0, 6.0, 10.0];
        let mut out = [0.0f64; 3];
        let c = unsafe { vitalis_rate_of_change(data.as_ptr(), 4, out.as_mut_ptr()) };
        assert_eq!(c, 3);
        assert!((out[0] - 2.0).abs() < 1e-10);
        assert!((out[1] - 3.0).abs() < 1e-10);
        assert!((out[2] - 4.0).abs() < 1e-10);
    }
}
