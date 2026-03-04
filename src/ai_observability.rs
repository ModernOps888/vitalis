//! AI Observability — drift detection, fairness metrics, SHAP, safety guardrails, A/B testing.
//!
//! Provides production AI monitoring: data/concept drift detection,
//! fairness & bias metrics, SHAP-like feature attribution,
//! safety guardrails, and A/B test significance testing.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Drift Detection ─────────────────────────────────────────────────────

/// Drift detector using statistical tests.
#[derive(Debug, Clone)]
pub struct DriftDetector {
    pub reference_mean: Vec<f64>,
    pub reference_std: Vec<f64>,
    pub reference_n: usize,
    pub threshold: f64,
    pub feature_names: Vec<String>,
}

impl DriftDetector {
    /// Fit drift detector on reference (training) data.
    pub fn fit(data: &[Vec<f64>], feature_names: Vec<String>, threshold: f64) -> Self {
        let n = data.len();
        if n == 0 || data[0].is_empty() {
            return DriftDetector {
                reference_mean: vec![], reference_std: vec![],
                reference_n: 0, threshold, feature_names,
            };
        }
        let dim = data[0].len();
        let mut mean = vec![0.0; dim];
        for row in data {
            for (i, &v) in row.iter().enumerate() {
                if i < dim { mean[i] += v; }
            }
        }
        for m in &mut mean { *m /= n as f64; }

        let mut var = vec![0.0; dim];
        for row in data {
            for (i, &v) in row.iter().enumerate() {
                if i < dim {
                    let d = v - mean[i];
                    var[i] += d * d;
                }
            }
        }
        let std_vec: Vec<f64> = var.iter().map(|v| (v / n as f64).sqrt().max(1e-10)).collect();

        DriftDetector { reference_mean: mean, reference_std: std_vec, reference_n: n, threshold, feature_names }
    }

    /// Check if new data has drifted from reference.
    pub fn detect(&self, new_data: &[Vec<f64>]) -> DriftResult {
        let n = new_data.len();
        if n == 0 || self.reference_mean.is_empty() {
            return DriftResult { is_drifted: false, drift_score: 0.0, feature_scores: vec![] };
        }
        let dim = self.reference_mean.len();
        let mut new_mean = vec![0.0; dim];
        for row in new_data {
            for (i, &v) in row.iter().enumerate() {
                if i < dim { new_mean[i] += v; }
            }
        }
        for m in &mut new_mean { *m /= n as f64; }

        // Per-feature z-scores (standardized mean difference)
        let feature_scores: Vec<f64> = (0..dim).map(|i| {
            let se = self.reference_std[i] / (n as f64).sqrt();
            if se > 1e-10 { ((new_mean[i] - self.reference_mean[i]) / se).abs() } else { 0.0 }
        }).collect();

        let drift_score: f64 = feature_scores.iter().cloned().fold(0.0, f64::max);
        let is_drifted = drift_score > self.threshold;

        DriftResult { is_drifted, drift_score, feature_scores }
    }
}

/// Result of drift detection.
#[derive(Debug, Clone)]
pub struct DriftResult {
    pub is_drifted: bool,
    pub drift_score: f64,
    pub feature_scores: Vec<f64>,
}

// ── Population Stability Index (PSI) ────────────────────────────────────

/// Compute PSI between reference and target distributions.
pub fn population_stability_index(reference: &[f64], target: &[f64], n_bins: usize) -> f64 {
    if reference.is_empty() || target.is_empty() || n_bins == 0 { return 0.0; }

    let min_val = reference.iter().chain(target.iter()).cloned().fold(f64::INFINITY, f64::min);
    let max_val = reference.iter().chain(target.iter()).cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;
    if range <= 0.0 { return 0.0; }

    let bin_width = range / n_bins as f64;

    let ref_counts = bin_counts(reference, min_val, bin_width, n_bins);
    let tgt_counts = bin_counts(target, min_val, bin_width, n_bins);

    let ref_total = reference.len() as f64;
    let tgt_total = target.len() as f64;

    let mut psi = 0.0;
    for i in 0..n_bins {
        let ref_pct = (ref_counts[i] as f64 / ref_total).max(0.0001);
        let tgt_pct = (tgt_counts[i] as f64 / tgt_total).max(0.0001);
        psi += (tgt_pct - ref_pct) * (tgt_pct / ref_pct).ln();
    }
    psi
}

fn bin_counts(data: &[f64], min_val: f64, bin_width: f64, n_bins: usize) -> Vec<usize> {
    let mut counts = vec![0; n_bins];
    for &v in data {
        let bin = ((v - min_val) / bin_width).floor() as usize;
        let bin = bin.min(n_bins - 1);
        counts[bin] += 1;
    }
    counts
}

// ── Fairness Metrics ────────────────────────────────────────────────────

/// Fairness evaluation across protected groups.
#[derive(Debug, Clone)]
pub struct FairnessReport {
    pub demographic_parity_diff: f64,
    pub equalized_odds_diff: f64,
    pub disparate_impact_ratio: f64,
    pub group_metrics: HashMap<String, GroupMetrics>,
}

/// Metrics for a single demographic group.
#[derive(Debug, Clone)]
pub struct GroupMetrics {
    pub positive_rate: f64,
    pub true_positive_rate: f64,
    pub false_positive_rate: f64,
    pub count: usize,
}

/// Compute fairness metrics across two groups.
pub fn compute_fairness(
    predictions: &[f64],
    labels: &[f64],
    group_membership: &[usize], // 0 or 1
) -> FairnessReport {
    let mut groups: HashMap<usize, (Vec<f64>, Vec<f64>)> = HashMap::new();

    for i in 0..predictions.len() {
        let group = if i < group_membership.len() { group_membership[i] } else { 0 };
        let entry = groups.entry(group).or_insert_with(|| (vec![], vec![]));
        entry.0.push(predictions[i]);
        entry.1.push(labels[i]);
    }

    let mut group_metrics_map = HashMap::new();

    for (&group, (preds, lbls)) in &groups {
        let n = preds.len();
        let positives = preds.iter().filter(|&&p| p >= 0.5).count() as f64;
        let positive_rate = positives / n as f64;

        let tp = preds.iter().zip(lbls.iter()).filter(|&(p, l)| *p >= 0.5 && *l >= 0.5).count() as f64;
        let fp = preds.iter().zip(lbls.iter()).filter(|&(p, l)| *p >= 0.5 && *l < 0.5).count() as f64;
        let actual_pos = lbls.iter().filter(|&&l| l >= 0.5).count() as f64;
        let actual_neg = lbls.iter().filter(|&&l| l < 0.5).count() as f64;

        let tpr = if actual_pos > 0.0 { tp / actual_pos } else { 0.0 };
        let fpr = if actual_neg > 0.0 { fp / actual_neg } else { 0.0 };

        group_metrics_map.insert(format!("group_{}", group), GroupMetrics {
            positive_rate, true_positive_rate: tpr, false_positive_rate: fpr, count: n,
        });
    }

    let rates: Vec<f64> = group_metrics_map.values().map(|g| g.positive_rate).collect();
    let tprs: Vec<f64> = group_metrics_map.values().map(|g| g.true_positive_rate).collect();

    let dp_diff = if rates.len() >= 2 { (rates[0] - rates[1]).abs() } else { 0.0 };
    let eo_diff = if tprs.len() >= 2 { (tprs[0] - tprs[1]).abs() } else { 0.0 };
    let di_ratio = if rates.len() >= 2 && rates[0] > 0.0 && rates[1] > 0.0 {
        rates.iter().cloned().reduce(f64::min).unwrap() / rates.iter().cloned().reduce(f64::max).unwrap()
    } else { 1.0 };

    FairnessReport {
        demographic_parity_diff: dp_diff,
        equalized_odds_diff: eo_diff,
        disparate_impact_ratio: di_ratio,
        group_metrics: group_metrics_map,
    }
}

// ── SHAP-like Feature Attribution ───────────────────────────────────────

/// Compute SHAP-like feature attributions via permutation.
pub fn permutation_importance(
    predict_fn: &dyn Fn(&[f64]) -> f64,
    data: &[Vec<f64>],
    labels: &[f64],
    n_repeats: usize,
    rng: &mut SimpleRng,
) -> Vec<f64> {
    if data.is_empty() { return vec![]; }
    let n_features = data[0].len();
    let n = data.len();

    // Baseline error
    let baseline_error: f64 = data.iter().zip(labels.iter())
        .map(|(x, &y)| (predict_fn(x) - y).powi(2))
        .sum::<f64>() / n as f64;

    let mut importances = vec![0.0; n_features];

    for feat in 0..n_features {
        let mut perm_error = 0.0;
        for _ in 0..n_repeats {
            // Create permuted data
            for i in 0..n {
                let j = (rng.next_u64() as usize) % n;
                let mut x_perm = data[i].clone();
                x_perm[feat] = data[j][feat]; // Permute feature
                perm_error += (predict_fn(&x_perm) - labels[i]).powi(2);
            }
        }
        perm_error /= (n * n_repeats) as f64;
        importances[feat] = perm_error - baseline_error;
    }

    importances
}

/// Simple SHAP approximation using marginal contributions.
pub fn shap_values(
    predict_fn: &dyn Fn(&[f64]) -> f64,
    instance: &[f64],
    baseline: &[f64],
    n_samples: usize,
    rng: &mut SimpleRng,
) -> Vec<f64> {
    let n_features = instance.len();
    let mut shap = vec![0.0; n_features];

    for _ in 0..n_samples {
        // Random feature order
        let mut order: Vec<usize> = (0..n_features).collect();
        for i in (1..n_features).rev() {
            let j = (rng.next_u64() as usize) % (i + 1);
            order.swap(i, j);
        }

        let mut x = baseline.to_vec();
        let mut prev_val = predict_fn(&x);

        for &feat in &order {
            x[feat] = instance[feat];
            let new_val = predict_fn(&x);
            shap[feat] += new_val - prev_val;
            prev_val = new_val;
        }
    }

    for s in &mut shap { *s /= n_samples as f64; }
    shap
}

// ── Safety Guardrails ───────────────────────────────────────────────────

/// Safety guardrail configuration.
#[derive(Debug, Clone)]
pub struct SafetyGuardrail {
    pub name: String,
    pub check: GuardrailCheck,
    pub action: GuardrailAction,
}

/// Type of safety check.
#[derive(Debug, Clone)]
pub enum GuardrailCheck {
    /// Output confidence must be above threshold.
    MinConfidence(f64),
    /// Output must be within range.
    OutputRange { min: f64, max: f64 },
    /// Input features must be within training range.
    InputRange { min: Vec<f64>, max: Vec<f64> },
    /// Prediction entropy must be below threshold.
    MaxEntropy(f64),
    /// Custom predicate returns true/false.
    Custom(String),
}

/// Action taken when guardrail is triggered.
#[derive(Debug, Clone)]
pub enum GuardrailAction {
    Block,
    Fallback(Vec<f64>),
    Flag,
    Log,
}

/// Evaluate safety guardrails on a prediction.
pub fn check_guardrails(
    input: &[f64],
    output: &[f64],
    guardrails: &[SafetyGuardrail],
) -> Vec<GuardrailViolation> {
    let mut violations = Vec::new();

    for rail in guardrails {
        let violated = match &rail.check {
            GuardrailCheck::MinConfidence(threshold) => {
                let max_conf = output.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                max_conf < *threshold
            }
            GuardrailCheck::OutputRange { min, max } => {
                output.iter().any(|&v| v < *min || v > *max)
            }
            GuardrailCheck::InputRange { min, max } => {
                input.iter().enumerate().any(|(i, &v)| {
                    (i < min.len() && v < min[i]) || (i < max.len() && v > max[i])
                })
            }
            GuardrailCheck::MaxEntropy(threshold) => {
                let probs = softmax(output);
                let entropy: f64 = -probs.iter().map(|&p| if p > 1e-10 { p * p.ln() } else { 0.0 }).sum::<f64>();
                entropy > *threshold
            }
            GuardrailCheck::Custom(_) => false, // Custom checks evaluated externally
        };

        if violated {
            violations.push(GuardrailViolation {
                guardrail_name: rail.name.clone(),
                action: rail.action.clone(),
            });
        }
    }

    violations
}

/// Record of a guardrail violation.
#[derive(Debug, Clone)]
pub struct GuardrailViolation {
    pub guardrail_name: String,
    pub action: GuardrailAction,
}

// ── A/B Test Significance ───────────────────────────────────────────────

/// A/B test result with statistical significance.
#[derive(Debug, Clone)]
pub struct ABTestResult {
    pub control_mean: f64,
    pub treatment_mean: f64,
    pub z_score: f64,
    pub p_value: f64,
    pub is_significant: bool,
    pub lift: f64,
}

/// Two-sample z-test for A/B test significance.
pub fn ab_test(control: &[f64], treatment: &[f64], alpha: f64) -> ABTestResult {
    let n1 = control.len() as f64;
    let n2 = treatment.len() as f64;

    let mean1 = control.iter().sum::<f64>() / n1;
    let mean2 = treatment.iter().sum::<f64>() / n2;

    let var1 = control.iter().map(|v| (v - mean1).powi(2)).sum::<f64>() / n1;
    let var2 = treatment.iter().map(|v| (v - mean2).powi(2)).sum::<f64>() / n2;

    let se = (var1 / n1 + var2 / n2).sqrt();
    let z = if se > 1e-10 { (mean2 - mean1) / se } else { 0.0 };
    let p_value = 2.0 * standard_normal_cdf(-z.abs());

    let lift = if mean1.abs() > 1e-10 { (mean2 - mean1) / mean1 } else { 0.0 };

    ABTestResult {
        control_mean: mean1,
        treatment_mean: mean2,
        z_score: z,
        p_value,
        is_significant: p_value < alpha,
        lift,
    }
}

/// Standard normal CDF approximation.
fn standard_normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Error function approximation (Abramowitz and Stegun).
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * x.abs());
    let poly = t * (0.254829592 + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    let result = 1.0 - poly * (-x * x).exp();
    if x >= 0.0 { result } else { -result }
}

// ── Alert System ────────────────────────────────────────────────────────

/// Monitoring alert.
#[derive(Debug, Clone)]
pub struct Alert {
    pub name: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub metric_value: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Monitor that checks metrics against thresholds.
#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub metric_name: String,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub direction: MonitorDirection,
}

#[derive(Debug, Clone)]
pub enum MonitorDirection {
    Above, // Alert when metric goes above threshold
    Below, // Alert when metric goes below threshold
}

impl Monitor {
    pub fn check(&self, value: f64) -> Option<Alert> {
        let violated = match self.direction {
            MonitorDirection::Above => {
                if value > self.critical_threshold {
                    Some(AlertSeverity::Critical)
                } else if value > self.warning_threshold {
                    Some(AlertSeverity::Warning)
                } else { None }
            }
            MonitorDirection::Below => {
                if value < self.critical_threshold {
                    Some(AlertSeverity::Critical)
                } else if value < self.warning_threshold {
                    Some(AlertSeverity::Warning)
                } else { None }
            }
        };

        violated.map(|severity| Alert {
            name: self.name.clone(),
            severity,
            message: format!("{}: {} = {:.4} (threshold: {:.4})", self.name, self.metric_name, value, self.warning_threshold),
            metric_value: value,
            threshold: self.warning_threshold,
        })
    }
}

// ── Utility ─────────────────────────────────────────────────────────────

fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&l| (l - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

#[derive(Debug, Clone)]
pub struct SimpleRng { state: u64 }
impl SimpleRng {
    pub fn new(seed: u64) -> Self { SimpleRng { state: seed.wrapping_add(1) } }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }
    pub fn next_f64(&mut self) -> f64 { (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 }
}

// ── FFI Interface ───────────────────────────────────────────────────────

static OBS_STORES: Mutex<Option<HashMap<i64, DriftDetector>>> = Mutex::new(None);

fn obs_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, DriftDetector>>> {
    OBS_STORES.lock().unwrap()
}

fn next_obs_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_obs_create_drift(n_features: i64, threshold: f64) -> i64 {
    let id = next_obs_id();
    let detector = DriftDetector {
        reference_mean: vec![0.0; n_features as usize],
        reference_std: vec![1.0; n_features as usize],
        reference_n: 100,
        threshold,
        feature_names: (0..n_features as usize).map(|i| format!("f{}", i)).collect(),
    };
    let mut store = obs_store();
    store.get_or_insert_with(HashMap::new).insert(id, detector);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_obs_check_drift(id: i64, data_ptr: *const f64, n_samples: i64, n_features: i64) -> f64 {
    let store = obs_store();
    let detector = match store.as_ref().and_then(|s| s.get(&id)) {
        Some(d) => d.clone(),
        None => return -1.0,
    };
    drop(store);

    let data = unsafe { std::slice::from_raw_parts(data_ptr, (n_samples * n_features) as usize) };
    let rows: Vec<Vec<f64>> = data.chunks(n_features as usize).map(|c| c.to_vec()).collect();
    let result = detector.detect(&rows);
    result.drift_score
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_obs_free(id: i64) {
    let mut store = obs_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_detector_no_drift() {
        let data: Vec<Vec<f64>> = (0..100).map(|i| vec![i as f64 / 100.0, (i as f64 / 50.0).sin()]).collect();
        let detector = DriftDetector::fit(&data, vec!["x".into(), "sin_x".into()], 3.0);

        // Use same distribution (same range) — should not drift
        let same_data: Vec<Vec<f64>> = (0..100).map(|i| vec![i as f64 / 100.0, (i as f64 / 50.0).sin()]).collect();
        let result = detector.detect(&same_data);
        assert!(!result.is_drifted);
    }

    #[test]
    fn test_drift_detector_drift() {
        // Reference around mean=0.5 with some spread
        let reference: Vec<Vec<f64>> = (0..100).map(|i| vec![i as f64 / 100.0]).collect();
        let detector = DriftDetector::fit(&reference, vec!["x".into()], 2.0);

        // Far away from reference mean — should drift
        let drifted: Vec<Vec<f64>> = (0..100).map(|_| vec![100.0]).collect();
        let result = detector.detect(&drifted);
        assert!(result.is_drifted);
        assert!(result.drift_score > 2.0);
    }

    #[test]
    fn test_psi() {
        let ref_data: Vec<f64> = (0..100).map(|i| i as f64 / 100.0).collect();
        let same_data = ref_data.clone();
        let psi = population_stability_index(&ref_data, &same_data, 10);
        assert!(psi < 0.1); // Same distribution → low PSI

        let shifted: Vec<f64> = (0..100).map(|i| i as f64 / 100.0 + 5.0).collect();
        let psi_drift = population_stability_index(&ref_data, &shifted, 10);
        assert!(psi_drift > psi); // Shifted → higher PSI
    }

    #[test]
    fn test_fairness() {
        let predictions = vec![1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let labels = vec![1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
        let groups = vec![0, 0, 0, 1, 1, 1];
        let report = compute_fairness(&predictions, &labels, &groups);
        assert!(report.demographic_parity_diff >= 0.0);
        assert!(report.disparate_impact_ratio >= 0.0);
        assert_eq!(report.group_metrics.len(), 2);
    }

    #[test]
    fn test_fairness_equal() {
        let predictions = vec![1.0, 0.0, 1.0, 0.0];
        let labels = vec![1.0, 0.0, 1.0, 0.0];
        let groups = vec![0, 0, 1, 1];
        let report = compute_fairness(&predictions, &labels, &groups);
        assert!((report.demographic_parity_diff - 0.0).abs() < 1e-10); // Equal rates
    }

    #[test]
    fn test_permutation_importance() {
        // y = 2*x0 + 0*x1 → feature 0 is important, feature 1 is not
        let data = vec![vec![1.0, 0.5], vec![2.0, 0.3], vec![3.0, 0.7], vec![4.0, 0.1]];
        let labels = vec![2.0, 4.0, 6.0, 8.0];
        let mut rng = SimpleRng::new(42);

        let importances = permutation_importance(
            &|x: &[f64]| 2.0 * x[0],
            &data, &labels, 5, &mut rng,
        );
        assert_eq!(importances.len(), 2);
        assert!(importances[0] > importances[1]); // Feature 0 more important
    }

    #[test]
    fn test_shap_values() {
        let mut rng = SimpleRng::new(42);
        let shap = shap_values(
            &|x: &[f64]| x[0] + 2.0 * x[1],
            &[1.0, 1.0],
            &[0.0, 0.0],
            100,
            &mut rng,
        );
        assert_eq!(shap.len(), 2);
        // SHAP values should sum to prediction difference
        let sum: f64 = shap.iter().sum();
        assert!((sum - 3.0).abs() < 0.5); // f(1,1) - f(0,0) = 3
    }

    #[test]
    fn test_guardrail_confidence() {
        let rails = vec![SafetyGuardrail {
            name: "min_confidence".into(),
            check: GuardrailCheck::MinConfidence(0.8),
            action: GuardrailAction::Block,
        }];
        let violations = check_guardrails(&[1.0], &[0.3, 0.3, 0.4], &rails);
        assert_eq!(violations.len(), 1); // Max confidence 0.4 < 0.8

        let violations2 = check_guardrails(&[1.0], &[0.1, 0.0, 0.9], &rails);
        assert_eq!(violations2.len(), 0); // Max confidence 0.9 > 0.8
    }

    #[test]
    fn test_guardrail_output_range() {
        let rails = vec![SafetyGuardrail {
            name: "output_range".into(),
            check: GuardrailCheck::OutputRange { min: 0.0, max: 1.0 },
            action: GuardrailAction::Flag,
        }];
        let viol = check_guardrails(&[1.0], &[0.5], &rails);
        assert_eq!(viol.len(), 0);

        let viol2 = check_guardrails(&[1.0], &[1.5], &rails);
        assert_eq!(viol2.len(), 1);
    }

    #[test]
    fn test_ab_test_significant() {
        let control: Vec<f64> = (0..100).map(|i| 0.5 + (i as f64 * 0.001)).collect();
        let treatment: Vec<f64> = (0..100).map(|i| 0.8 + (i as f64 * 0.001)).collect();
        let result = ab_test(&control, &treatment, 0.05);
        assert!(result.is_significant);
        assert!(result.lift > 0.0);
    }

    #[test]
    fn test_ab_test_not_significant() {
        let control = vec![0.5, 0.51, 0.49, 0.50, 0.52];
        let treatment = vec![0.50, 0.51, 0.50, 0.49, 0.51];
        let result = ab_test(&control, &treatment, 0.05);
        assert!(!result.is_significant);
    }

    #[test]
    fn test_monitor_above() {
        let mon = Monitor {
            name: "latency".into(),
            metric_name: "p99_latency_ms".into(),
            warning_threshold: 100.0,
            critical_threshold: 500.0,
            direction: MonitorDirection::Above,
        };
        assert!(mon.check(50.0).is_none());
        assert_eq!(mon.check(200.0).unwrap().severity, AlertSeverity::Warning);
        assert_eq!(mon.check(600.0).unwrap().severity, AlertSeverity::Critical);
    }

    #[test]
    fn test_monitor_below() {
        let mon = Monitor {
            name: "accuracy".into(),
            metric_name: "accuracy".into(),
            warning_threshold: 0.9,
            critical_threshold: 0.7,
            direction: MonitorDirection::Below,
        };
        assert!(mon.check(0.95).is_none());
        assert_eq!(mon.check(0.85).unwrap().severity, AlertSeverity::Warning);
        assert_eq!(mon.check(0.5).unwrap().severity, AlertSeverity::Critical);
    }

    #[test]
    fn test_ffi_drift() {
        let id = vitalis_obs_create_drift(2, 3.0);
        assert!(id > 0);
        let data = vec![0.0f64, 0.0, 0.0, 0.0]; // 2 samples × 2 features, at reference mean
        let score = vitalis_obs_check_drift(id, data.as_ptr(), 2, 2);
        assert!(score >= 0.0);
        vitalis_obs_free(id);
    }

    #[test]
    fn test_erf() {
        assert!((erf(0.0) - 0.0).abs() < 1e-6);
        assert!((erf(1.0) - 0.8427).abs() < 0.01);
    }

    #[test]
    fn test_standard_normal_cdf() {
        assert!((standard_normal_cdf(0.0) - 0.5).abs() < 1e-6);
        assert!(standard_normal_cdf(3.0) > 0.99);
        assert!(standard_normal_cdf(-3.0) < 0.01);
    }
}
