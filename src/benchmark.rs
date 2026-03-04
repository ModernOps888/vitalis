//! Benchmark — Micro-benchmarking Framework & Regression Detection
//!
//! Provides statistically rigorous micro-benchmarking with warm-up,
//! outlier detection (modified Z-score, Tukey fences), confidence
//! intervals (Student's t), regression testing (Welch's t-test,
//! Cohen's d), comparison reports, and history tracking.

use std::collections::HashMap;

// ── Statistical Helpers ──────────────────────────────────────────────

/// Online Welford accumulator for computing mean and variance in a single pass.
#[derive(Debug, Clone)]
pub struct WelfordAccumulator {
    count: u64,
    mean: f64,
    m2: f64,
    min: f64,
    max: f64,
}

impl WelfordAccumulator {
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Add a sample.
    pub fn update(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
        if x < self.min {
            self.min = x;
        }
        if x > self.max {
            self.max = x;
        }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }

    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.m2 / (self.count - 1) as f64
    }

    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }

    pub fn min(&self) -> f64 {
        self.min
    }

    pub fn max(&self) -> f64 {
        self.max
    }
}

impl Default for WelfordAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute sorted percentiles from a sample array.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = p / 100.0 * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;
    if upper >= sorted.len() {
        return sorted[sorted.len() - 1];
    }
    sorted[lower] * (1.0 - frac) + sorted[upper] * frac
}

/// Compute median of a sorted slice.
pub fn median(sorted: &[f64]) -> f64 {
    percentile(sorted, 50.0)
}

/// Interquartile range.
pub fn iqr(sorted: &[f64]) -> f64 {
    percentile(sorted, 75.0) - percentile(sorted, 25.0)
}

/// Median Absolute Deviation.
pub fn mad(sorted: &[f64]) -> f64 {
    let med = median(sorted);
    let mut deviations: Vec<f64> = sorted.iter().map(|&x| (x - med).abs()).collect();
    deviations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    median(&deviations) * 1.4826 // consistency constant for normal dist
}

// ── Outlier Detection ────────────────────────────────────────────────

/// Outlier detection methods.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutlierMethod {
    /// Modified Z-score using MAD (robust to non-normal distributions).
    ModifiedZScore { threshold: f64 },
    /// Tukey fences: Q1 - k*IQR, Q3 + k*IQR.
    TukeyFences { k: f64 },
}

impl Default for OutlierMethod {
    fn default() -> Self {
        Self::TukeyFences { k: 1.5 }
    }
}

/// Detect outlier indices in a sample.
pub fn detect_outliers(samples: &[f64], method: OutlierMethod) -> Vec<usize> {
    if samples.len() < 4 {
        return vec![];
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    match method {
        OutlierMethod::ModifiedZScore { threshold } => {
            let med = median(&sorted);
            let mad_val = mad(&sorted);
            if mad_val < 1e-12 {
                return vec![];
            }
            samples
                .iter()
                .enumerate()
                .filter(|&(_, &x)| {
                    let z = 0.6745 * (x - med) / mad_val;
                    z.abs() > threshold
                })
                .map(|(i, _)| i)
                .collect()
        }
        OutlierMethod::TukeyFences { k } => {
            let q1 = percentile(&sorted, 25.0);
            let q3 = percentile(&sorted, 75.0);
            let iqr_val = q3 - q1;
            let lower = q1 - k * iqr_val;
            let upper = q3 + k * iqr_val;
            samples
                .iter()
                .enumerate()
                .filter(|&(_, &x)| x < lower || x > upper)
                .map(|(i, _)| i)
                .collect()
        }
    }
}

// ── Confidence Intervals ─────────────────────────────────────────────

/// Student's t critical values for two-tailed 95% CI.
/// Index by min(df, 120). Precomputed for common degrees of freedom.
fn t_critical_95(df: usize) -> f64 {
    // Exact values for common df; interpolate for others.
    const T_TABLE: [(usize, f64); 18] = [
        (1, 12.706), (2, 4.303), (3, 3.182), (4, 2.776),
        (5, 2.571), (6, 2.447), (7, 2.365), (8, 2.306),
        (9, 2.262), (10, 2.228), (15, 2.131), (20, 2.086),
        (25, 2.060), (30, 2.042), (40, 2.021), (60, 2.000),
        (120, 1.980), (usize::MAX, 1.960),
    ];

    for &(d, t) in &T_TABLE {
        if df <= d {
            return t;
        }
    }
    1.960 // z-score for large df
}

/// 95% confidence interval: (lower, upper).
pub fn confidence_interval_95(mean: f64, stddev: f64, n: usize) -> (f64, f64) {
    if n < 2 {
        return (mean, mean);
    }
    let t = t_critical_95(n - 1);
    let margin = t * stddev / (n as f64).sqrt();
    (mean - margin, mean + margin)
}

// ── Regression Detection ─────────────────────────────────────────────

/// Result of a regression test comparison.
#[derive(Debug, Clone)]
pub struct RegressionResult {
    /// Welch's t-test statistic.
    pub t_statistic: f64,
    /// Approximate degrees of freedom (Welch–Satterthwaite).
    pub degrees_of_freedom: f64,
    /// Cohen's d effect size.
    pub cohens_d: f64,
    /// Whether the difference is statistically significant at α=0.05.
    pub significant: bool,
    /// Direction of change.
    pub direction: ChangeDirection,
    /// Percentage change: (new_mean - old_mean) / old_mean * 100.
    pub percent_change: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChangeDirection {
    Faster,
    Slower,
    NoChange,
}

/// Welch's t-test comparing two independent samples.
pub fn welch_t_test(old: &[f64], new: &[f64]) -> RegressionResult {
    let n1 = old.len() as f64;
    let n2 = new.len() as f64;

    let mean1: f64 = old.iter().sum::<f64>() / n1;
    let mean2: f64 = new.iter().sum::<f64>() / n2;

    let var1 = old.iter().map(|&x| (x - mean1).powi(2)).sum::<f64>() / (n1 - 1.0).max(1.0);
    let var2 = new.iter().map(|&x| (x - mean2).powi(2)).sum::<f64>() / (n2 - 1.0).max(1.0);

    let se = (var1 / n1 + var2 / n2).sqrt();
    let t = if se > 1e-15 {
        (mean2 - mean1) / se
    } else if (mean2 - mean1).abs() > 1e-15 {
        // Zero variance but different means → infinite significance
        if mean2 > mean1 { 1e6 } else { -1e6 }
    } else {
        0.0
    };

    // Welch–Satterthwaite degrees of freedom
    let num = (var1 / n1 + var2 / n2).powi(2);
    let den = (var1 / n1).powi(2) / (n1 - 1.0).max(1.0)
        + (var2 / n2).powi(2) / (n2 - 1.0).max(1.0);
    let df = if den > 1e-15 { num / den } else { 1.0 };

    // Cohen's d (pooled)
    let s_pooled = ((var1 + var2) / 2.0).sqrt();
    let d = if s_pooled > 1e-15 {
        (mean2 - mean1) / s_pooled
    } else if (mean2 - mean1).abs() > 1e-15 {
        if mean2 > mean1 { 1e6 } else { -1e6 }
    } else {
        0.0
    };

    let t_crit = t_critical_95(df.floor() as usize);
    let significant = t.abs() > t_crit;

    let direction = if !significant {
        ChangeDirection::NoChange
    } else if mean2 < mean1 {
        ChangeDirection::Faster
    } else {
        ChangeDirection::Slower
    };

    let percent_change = if mean1.abs() > 1e-15 {
        (mean2 - mean1) / mean1 * 100.0
    } else {
        0.0
    };

    RegressionResult {
        t_statistic: t,
        degrees_of_freedom: df,
        cohens_d: d,
        significant,
        direction,
        percent_change,
    }
}

/// Effect size interpretation.
pub fn interpret_effect_size(d: f64) -> &'static str {
    let d_abs = d.abs();
    if d_abs < 0.2 {
        "negligible"
    } else if d_abs < 0.5 {
        "small"
    } else if d_abs < 0.8 {
        "medium"
    } else {
        "large"
    }
}

// ── Benchmark Result ─────────────────────────────────────────────────

/// Full statistical summary of a benchmark run.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub samples: Vec<f64>,
    pub mean_ns: f64,
    pub median_ns: f64,
    pub stddev_ns: f64,
    pub min_ns: f64,
    pub max_ns: f64,
    pub p5_ns: f64,
    pub p95_ns: f64,
    pub p99_ns: f64,
    pub iqr_ns: f64,
    pub ci_lower_ns: f64,
    pub ci_upper_ns: f64,
    pub outlier_count: usize,
}

impl BenchmarkResult {
    /// Compute full statistics from raw timing samples (in nanoseconds).
    pub fn from_samples(name: &str, samples: Vec<f64>, warmup: usize) -> Self {
        let mut sorted = samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut acc = WelfordAccumulator::new();
        for &s in &samples {
            acc.update(s);
        }

        let mean = acc.mean();
        let sd = acc.stddev();
        let (ci_lo, ci_hi) = confidence_interval_95(mean, sd, samples.len());
        let outliers = detect_outliers(&samples, OutlierMethod::default());

        Self {
            name: name.to_string(),
            iterations: samples.len(),
            warmup_iterations: warmup,
            mean_ns: mean,
            median_ns: median(&sorted),
            stddev_ns: sd,
            min_ns: acc.min(),
            max_ns: acc.max(),
            p5_ns: percentile(&sorted, 5.0),
            p95_ns: percentile(&sorted, 95.0),
            p99_ns: percentile(&sorted, 99.0),
            iqr_ns: iqr(&sorted),
            ci_lower_ns: ci_lo,
            ci_upper_ns: ci_hi,
            outlier_count: outliers.len(),
            samples,
        }
    }

    /// Throughput in operations/second.
    pub fn ops_per_sec(&self) -> f64 {
        if self.mean_ns > 0.0 {
            1e9 / self.mean_ns
        } else {
            0.0
        }
    }

    /// Coefficient of variation (relative stddev).
    pub fn cv(&self) -> f64 {
        if self.mean_ns > 0.0 {
            self.stddev_ns / self.mean_ns * 100.0
        } else {
            0.0
        }
    }

    /// Format as human-readable text.
    pub fn to_text(&self) -> String {
        format!(
            "{}: mean={:.0}ns ±{:.0}ns (95% CI [{:.0}, {:.0}]) \
             median={:.0}ns min={:.0}ns max={:.0}ns \
             p95={:.0}ns p99={:.0}ns \
             CV={:.1}% outliers={} ops/s={:.0}",
            self.name,
            self.mean_ns,
            self.stddev_ns,
            self.ci_lower_ns,
            self.ci_upper_ns,
            self.median_ns,
            self.min_ns,
            self.max_ns,
            self.p95_ns,
            self.p99_ns,
            self.cv(),
            self.outlier_count,
            self.ops_per_sec(),
        )
    }
}

// ── Comparison Report ────────────────────────────────────────────────

/// Comparison between baseline and current benchmark results.
#[derive(Debug, Clone)]
pub struct ComparisonReport {
    pub name: String,
    pub baseline_mean_ns: f64,
    pub current_mean_ns: f64,
    pub regression: RegressionResult,
    pub baseline_ci: (f64, f64),
    pub current_ci: (f64, f64),
}

impl ComparisonReport {
    pub fn compare(baseline: &BenchmarkResult, current: &BenchmarkResult) -> Self {
        let regression = welch_t_test(&baseline.samples, &current.samples);
        Self {
            name: current.name.clone(),
            baseline_mean_ns: baseline.mean_ns,
            current_mean_ns: current.mean_ns,
            regression,
            baseline_ci: (baseline.ci_lower_ns, baseline.ci_upper_ns),
            current_ci: (current.ci_lower_ns, current.ci_upper_ns),
        }
    }

    pub fn to_text(&self) -> String {
        let arrow = match self.regression.direction {
            ChangeDirection::Faster => "▼ faster",
            ChangeDirection::Slower => "▲ slower",
            ChangeDirection::NoChange => "≈ no change",
        };
        let sig = if self.regression.significant {
            "significant"
        } else {
            "not significant"
        };
        format!(
            "{}: {:.0}ns → {:.0}ns ({:+.1}%, {}, {} [d={:.2}, {}])",
            self.name,
            self.baseline_mean_ns,
            self.current_mean_ns,
            self.regression.percent_change,
            arrow,
            sig,
            self.regression.cohens_d,
            interpret_effect_size(self.regression.cohens_d),
        )
    }
}

// ── Benchmark Suite ──────────────────────────────────────────────────

/// Configuration for running benchmarks.
#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub warmup_iterations: usize,
    pub measurement_iterations: usize,
    pub outlier_method: OutlierMethod,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 100,
            measurement_iterations: 1000,
            outlier_method: OutlierMethod::TukeyFences { k: 1.5 },
        }
    }
}

/// A single benchmark definition.
#[derive(Debug, Clone)]
pub struct BenchmarkDef {
    pub name: String,
    pub group: String,
    pub tags: Vec<String>,
}

/// Benchmark suite managing multiple benchmarks and their history.
#[derive(Debug, Clone, Default)]
pub struct BenchmarkSuite {
    pub benchmarks: Vec<BenchmarkDef>,
    pub results: HashMap<String, Vec<BenchmarkResult>>,
    pub baselines: HashMap<String, BenchmarkResult>,
    pub config: BenchConfig,
}

impl BenchmarkSuite {
    pub fn new(config: BenchConfig) -> Self {
        Self {
            benchmarks: Vec::new(),
            results: HashMap::new(),
            baselines: HashMap::new(),
            config,
        }
    }

    /// Register a benchmark.
    pub fn add(&mut self, name: &str, group: &str, tags: &[&str]) {
        self.benchmarks.push(BenchmarkDef {
            name: name.to_string(),
            group: group.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
        });
    }

    /// Record a result.
    pub fn record(&mut self, result: BenchmarkResult) {
        self.results
            .entry(result.name.clone())
            .or_default()
            .push(result);
    }

    /// Set a baseline for regression comparison.
    pub fn set_baseline(&mut self, name: &str, result: BenchmarkResult) {
        self.baselines.insert(name.to_string(), result);
    }

    /// Compare latest result against baseline.
    pub fn compare(&self, name: &str) -> Option<ComparisonReport> {
        let baseline = self.baselines.get(name)?;
        let history = self.results.get(name)?;
        let current = history.last()?;
        Some(ComparisonReport::compare(baseline, current))
    }

    /// Filter benchmarks by group.
    pub fn by_group(&self, group: &str) -> Vec<&BenchmarkDef> {
        self.benchmarks
            .iter()
            .filter(|b| b.group == group)
            .collect()
    }

    /// Filter benchmarks by tag.
    pub fn by_tag(&self, tag: &str) -> Vec<&BenchmarkDef> {
        self.benchmarks
            .iter()
            .filter(|b| b.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Generate summary report across all benchmarks.
    pub fn summary_report(&self) -> String {
        let mut report = String::from("=== Benchmark Suite Report ===\n");
        report.push_str(&format!("Benchmarks: {}\n", self.benchmarks.len()));

        for bdef in &self.benchmarks {
            if let Some(history) = self.results.get(&bdef.name) {
                if let Some(latest) = history.last() {
                    report.push_str(&format!("  {}\n", latest.to_text()));
                    if let Some(comparison) = self.compare(&bdef.name) {
                        report.push_str(&format!("    vs baseline: {}\n", comparison.to_text()));
                    }
                }
            }
        }
        report
    }
}

// ── History Tracking ─────────────────────────────────────────────────

/// Trend analysis over benchmark history.
#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    pub name: String,
    /// Linear regression slope (ns per run).
    pub slope: f64,
    /// R² goodness of fit.
    pub r_squared: f64,
    /// Trend direction.
    pub trend: Trend,
    /// Number of data points.
    pub data_points: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Trend {
    Improving,
    Degrading,
    Stable,
}

impl TrendAnalysis {
    /// Compute linear trend from a series of benchmark means.
    pub fn analyze(name: &str, means: &[f64]) -> Self {
        let n = means.len();
        if n < 3 {
            return Self {
                name: name.to_string(),
                slope: 0.0,
                r_squared: 0.0,
                trend: Trend::Stable,
                data_points: n,
            };
        }

        // Simple linear regression: y = a + b*x
        let x_bar = (n - 1) as f64 / 2.0;
        let y_bar: f64 = means.iter().sum::<f64>() / n as f64;

        let mut ss_xy = 0.0;
        let mut ss_xx = 0.0;
        let mut ss_yy = 0.0;

        for (i, &y) in means.iter().enumerate() {
            let xi = i as f64 - x_bar;
            let yi = y - y_bar;
            ss_xy += xi * yi;
            ss_xx += xi * xi;
            ss_yy += yi * yi;
        }

        let slope = if ss_xx > 1e-15 {
            ss_xy / ss_xx
        } else {
            0.0
        };
        let r_squared = if ss_xx > 1e-15 && ss_yy > 1e-15 {
            (ss_xy * ss_xy) / (ss_xx * ss_yy)
        } else {
            0.0
        };

        // Classify: significant slope relative to mean
        let relative_slope = if y_bar.abs() > 1e-15 {
            slope / y_bar * 100.0
        } else {
            0.0
        };

        let trend = if r_squared < 0.3 || relative_slope.abs() < 1.0 {
            Trend::Stable
        } else if slope < 0.0 {
            Trend::Improving // Getting faster (lower ns)
        } else {
            Trend::Degrading // Getting slower (higher ns)
        };

        Self {
            name: name.to_string(),
            slope,
            r_squared,
            trend,
            data_points: n,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Welford Accumulator ──────────────────────────────────────────

    #[test]
    fn test_welford_basic() {
        let mut acc = WelfordAccumulator::new();
        for &v in &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0] {
            acc.update(v);
        }
        assert!((acc.mean() - 5.0).abs() < 1e-10);
        // Sample variance: 32/7 ≈ 4.571
        assert!((acc.variance() - 32.0 / 7.0).abs() < 1e-10);
        assert!((acc.stddev() - (32.0_f64 / 7.0).sqrt()).abs() < 1e-10);
        assert!((acc.min() - 2.0).abs() < 1e-10);
        assert!((acc.max() - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_welford_single() {
        let mut acc = WelfordAccumulator::new();
        acc.update(42.0);
        assert!((acc.mean() - 42.0).abs() < 1e-10);
        assert!(acc.variance() < 1e-10);
    }

    // ── Percentiles ──────────────────────────────────────────────────

    #[test]
    fn test_percentile() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert!((median(&sorted) - 5.5).abs() < 1e-10);
        assert!((percentile(&sorted, 25.0) - 3.25).abs() < 1e-10);
        assert!((percentile(&sorted, 75.0) - 7.75).abs() < 1e-10);
    }

    #[test]
    fn test_iqr() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert!((iqr(&sorted) - 4.5).abs() < 1e-10);
    }

    // ── Outlier Detection ────────────────────────────────────────────

    #[test]
    fn test_tukey_outliers() {
        let mut samples = vec![10.0; 20];
        samples.push(100.0); // Clear outlier
        let outliers = detect_outliers(&samples, OutlierMethod::TukeyFences { k: 1.5 });
        assert!(!outliers.is_empty());
        assert!(outliers.contains(&20));
    }

    #[test]
    fn test_no_outliers() {
        let samples = vec![10.0, 10.1, 9.9, 10.2, 9.8];
        let outliers = detect_outliers(&samples, OutlierMethod::TukeyFences { k: 1.5 });
        assert!(outliers.is_empty());
    }

    #[test]
    fn test_modified_z_outliers() {
        // Use values with non-zero MAD for modified Z-score
        let mut samples: Vec<f64> = (0..20).map(|i| 50.0 + (i as f64) * 0.5).collect();
        samples.push(500.0);
        let outliers = detect_outliers(
            &samples,
            OutlierMethod::ModifiedZScore { threshold: 3.5 },
        );
        assert!(!outliers.is_empty());
    }

    // ── Confidence Intervals ─────────────────────────────────────────

    #[test]
    fn test_confidence_interval() {
        let (lo, hi) = confidence_interval_95(100.0, 10.0, 30);
        assert!(lo < 100.0);
        assert!(hi > 100.0);
        assert!((hi - lo) > 0.0);
    }

    #[test]
    fn test_ci_narrows_with_n() {
        let (lo1, hi1) = confidence_interval_95(100.0, 10.0, 10);
        let (lo2, hi2) = confidence_interval_95(100.0, 10.0, 100);
        let width1 = hi1 - lo1;
        let width2 = hi2 - lo2;
        assert!(width2 < width1); // More samples → narrower CI
    }

    // ── Regression Detection ─────────────────────────────────────────

    #[test]
    fn test_welch_no_regression() {
        let old = vec![100.0, 101.0, 99.0, 100.5, 100.2];
        let new = vec![100.1, 99.9, 100.3, 99.8, 100.4];
        let result = welch_t_test(&old, &new);
        assert_eq!(result.direction, ChangeDirection::NoChange);
    }

    #[test]
    fn test_welch_regression() {
        let old = vec![100.0; 30];
        let new = vec![200.0; 30]; // Clear regression
        let result = welch_t_test(&old, &new);
        assert!(result.significant);
        assert_eq!(result.direction, ChangeDirection::Slower);
        assert!(result.percent_change > 50.0);
    }

    #[test]
    fn test_welch_improvement() {
        let old = vec![200.0; 30];
        let new = vec![100.0; 30]; // Clear improvement
        let result = welch_t_test(&old, &new);
        assert!(result.significant);
        assert_eq!(result.direction, ChangeDirection::Faster);
    }

    #[test]
    fn test_effect_size() {
        assert_eq!(interpret_effect_size(0.1), "negligible");
        assert_eq!(interpret_effect_size(0.3), "small");
        assert_eq!(interpret_effect_size(0.6), "medium");
        assert_eq!(interpret_effect_size(1.0), "large");
    }

    // ── Benchmark Result ─────────────────────────────────────────────

    #[test]
    fn test_benchmark_result() {
        let samples: Vec<f64> = (0..100).map(|i| 1000.0 + (i as f64) * 0.1).collect();
        let result = BenchmarkResult::from_samples("test_bench", samples, 10);
        assert!(result.mean_ns > 0.0);
        assert!(result.median_ns > 0.0);
        assert!(result.min_ns <= result.max_ns);
        assert!(result.p5_ns <= result.p95_ns);
        assert!(result.ci_lower_ns <= result.ci_upper_ns);
    }

    #[test]
    fn test_benchmark_ops_per_sec() {
        let samples = vec![1000.0; 50]; // 1000ns = 1μs → 1M ops/s
        let result = BenchmarkResult::from_samples("ops", samples, 0);
        assert!((result.ops_per_sec() - 1e6).abs() < 1e3);
    }

    #[test]
    fn test_benchmark_text() {
        let samples = vec![100.0; 50];
        let result = BenchmarkResult::from_samples("fmt", samples, 5);
        let text = result.to_text();
        assert!(text.contains("fmt"));
        assert!(text.contains("mean="));
    }

    // ── Comparison Report ────────────────────────────────────────────

    #[test]
    fn test_comparison_report() {
        let old_samples = vec![100.0; 30];
        let new_samples = vec![200.0; 30];
        let baseline = BenchmarkResult::from_samples("cmp", old_samples, 0);
        let current = BenchmarkResult::from_samples("cmp", new_samples, 0);
        let report = ComparisonReport::compare(&baseline, &current);
        assert!(report.regression.significant);
        let text = report.to_text();
        assert!(text.contains("slower") || text.contains("faster"));
    }

    // ── Benchmark Suite ──────────────────────────────────────────────

    #[test]
    fn test_suite_add_record() {
        let mut suite = BenchmarkSuite::new(BenchConfig::default());
        suite.add("bench_a", "group1", &["fast", "cpu"]);
        assert_eq!(suite.benchmarks.len(), 1);

        let result = BenchmarkResult::from_samples("bench_a", vec![100.0; 10], 0);
        suite.record(result);
        assert_eq!(suite.results.get("bench_a").unwrap().len(), 1);
    }

    #[test]
    fn test_suite_by_group() {
        let mut suite = BenchmarkSuite::new(BenchConfig::default());
        suite.add("a", "math", &[]);
        suite.add("b", "io", &[]);
        suite.add("c", "math", &[]);
        assert_eq!(suite.by_group("math").len(), 2);
    }

    #[test]
    fn test_suite_by_tag() {
        let mut suite = BenchmarkSuite::new(BenchConfig::default());
        suite.add("a", "g", &["hot"]);
        suite.add("b", "g", &["cold"]);
        suite.add("c", "g", &["hot", "cold"]);
        assert_eq!(suite.by_tag("hot").len(), 2);
        assert_eq!(suite.by_tag("cold").len(), 2);
    }

    #[test]
    fn test_suite_regression_compare() {
        let mut suite = BenchmarkSuite::new(BenchConfig::default());
        suite.add("bench", "core", &[]);

        let baseline = BenchmarkResult::from_samples("bench", vec![100.0; 30], 0);
        suite.set_baseline("bench", baseline);

        let current = BenchmarkResult::from_samples("bench", vec![200.0; 30], 0);
        suite.record(current);

        let cmp = suite.compare("bench").unwrap();
        assert!(cmp.regression.significant);
    }

    // ── Trend Analysis ───────────────────────────────────────────────

    #[test]
    fn test_trend_stable() {
        let means = vec![100.0, 100.1, 99.9, 100.0, 100.2, 99.8];
        let trend = TrendAnalysis::analyze("stable", &means);
        assert_eq!(trend.trend, Trend::Stable);
    }

    #[test]
    fn test_trend_degrading() {
        let means: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 10.0).collect();
        let trend = TrendAnalysis::analyze("degrading", &means);
        assert_eq!(trend.trend, Trend::Degrading);
        assert!(trend.r_squared > 0.9);
    }

    #[test]
    fn test_trend_improving() {
        let means: Vec<f64> = (0..20).map(|i| 200.0 - i as f64 * 8.0).collect();
        let trend = TrendAnalysis::analyze("improving", &means);
        assert_eq!(trend.trend, Trend::Improving);
    }

    #[test]
    fn test_summary_report() {
        let mut suite = BenchmarkSuite::new(BenchConfig::default());
        suite.add("b1", "g", &[]);
        let result = BenchmarkResult::from_samples("b1", vec![100.0; 10], 0);
        suite.record(result);
        let report = suite.summary_report();
        assert!(report.contains("Benchmark Suite Report"));
    }
}
