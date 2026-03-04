//! Experiment Tracking — run tracking, metrics, hyperparameter search, model registry.
//!
//! Provides MLOps experiment infrastructure: run logging,
//! metric history, hyperparameter search (grid, random, Bayesian),
//! reproducibility (seed + config snapshots), and model registry.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Experiment Run ──────────────────────────────────────────────────────

/// A single experiment run tracking metrics, params, and artifacts.
#[derive(Debug, Clone)]
pub struct Run {
    pub id: String,
    pub name: String,
    pub status: RunStatus,
    pub params: HashMap<String, f64>,
    pub str_params: HashMap<String, String>,
    pub metrics: HashMap<String, Vec<MetricEntry>>,
    pub tags: Vec<String>,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub seed: u64,
}

/// Current status of a run.
#[derive(Debug, Clone, PartialEq)]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A single metric measurement at a step.
#[derive(Debug, Clone)]
pub struct MetricEntry {
    pub step: usize,
    pub value: f64,
    pub timestamp: u64,
}

impl Run {
    pub fn new(id: &str, name: &str, seed: u64) -> Self {
        Run {
            id: id.to_string(),
            name: name.to_string(),
            status: RunStatus::Running,
            params: HashMap::new(),
            str_params: HashMap::new(),
            metrics: HashMap::new(),
            tags: vec![],
            start_time: 0,
            end_time: None,
            seed,
        }
    }

    /// Log a hyperparameter.
    pub fn log_param(&mut self, key: &str, value: f64) {
        self.params.insert(key.to_string(), value);
    }

    /// Log a string parameter.
    pub fn log_str_param(&mut self, key: &str, value: &str) {
        self.str_params.insert(key.to_string(), value.to_string());
    }

    /// Log a metric at a given step.
    pub fn log_metric(&mut self, name: &str, value: f64, step: usize) {
        self.metrics.entry(name.to_string()).or_default().push(MetricEntry {
            step, value, timestamp: 0,
        });
    }

    /// Get the latest value of a metric.
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        self.metrics.get(name)?.last().map(|e| e.value)
    }

    /// Get the best (minimum) value of a metric.
    pub fn best_metric(&self, name: &str) -> Option<f64> {
        self.metrics.get(name)?.iter().map(|e| e.value).reduce(f64::min)
    }

    /// Get the best (maximum) value of a metric.
    pub fn best_metric_max(&self, name: &str) -> Option<f64> {
        self.metrics.get(name)?.iter().map(|e| e.value).reduce(f64::max)
    }

    /// Get all values of a metric as a vector.
    pub fn metric_history(&self, name: &str) -> Vec<f64> {
        self.metrics.get(name).map(|entries| entries.iter().map(|e| e.value).collect()).unwrap_or_default()
    }

    /// Add a tag.
    pub fn add_tag(&mut self, tag: &str) {
        self.tags.push(tag.to_string());
    }

    /// Mark run as completed.
    pub fn complete(&mut self) {
        self.status = RunStatus::Completed;
        self.end_time = Some(0);
    }

    /// Mark run as failed.
    pub fn fail(&mut self) {
        self.status = RunStatus::Failed;
        self.end_time = Some(0);
    }
}

// ── Experiment (Group of Runs) ──────────────────────────────────────────

/// An experiment groups multiple runs together.
#[derive(Debug, Clone)]
pub struct Experiment {
    pub name: String,
    pub description: String,
    pub runs: Vec<Run>,
}

impl Experiment {
    pub fn new(name: &str, description: &str) -> Self {
        Experiment { name: name.to_string(), description: description.to_string(), runs: vec![] }
    }

    pub fn add_run(&mut self, run: Run) {
        self.runs.push(run);
    }

    /// Get the best run by a metric (minimize).
    pub fn best_run(&self, metric: &str) -> Option<&Run> {
        self.runs.iter()
            .filter(|r| r.status == RunStatus::Completed)
            .min_by(|a, b| {
                let va = a.best_metric(metric).unwrap_or(f64::INFINITY);
                let vb = b.best_metric(metric).unwrap_or(f64::INFINITY);
                va.partial_cmp(&vb).unwrap()
            })
    }

    /// Get the best run by a metric (maximize).
    pub fn best_run_max(&self, metric: &str) -> Option<&Run> {
        self.runs.iter()
            .filter(|r| r.status == RunStatus::Completed)
            .max_by(|a, b| {
                let va = a.best_metric_max(metric).unwrap_or(f64::NEG_INFINITY);
                let vb = b.best_metric_max(metric).unwrap_or(f64::NEG_INFINITY);
                va.partial_cmp(&vb).unwrap()
            })
    }

    /// Summary statistics across all completed runs for a metric.
    pub fn metric_summary(&self, metric: &str) -> Option<MetricSummary> {
        let values: Vec<f64> = self.runs.iter()
            .filter(|r| r.status == RunStatus::Completed)
            .filter_map(|r| r.best_metric(metric))
            .collect();

        if values.is_empty() { return None; }
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        Some(MetricSummary { mean, std: var.sqrt(), min, max, count: values.len() })
    }
}

/// Summary statistics for a metric across runs.
#[derive(Debug, Clone)]
pub struct MetricSummary {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
}

// ── Hyperparameter Search ───────────────────────────────────────────────

/// Search space for a hyperparameter.
#[derive(Debug, Clone)]
pub enum ParamSpace {
    /// Continuous range [low, high].
    Continuous { low: f64, high: f64 },
    /// Log-uniform range [10^low, 10^high].
    LogUniform { low: f64, high: f64 },
    /// Discrete set of values.
    Discrete { values: Vec<f64> },
    /// Integer range [low, high].
    IntRange { low: i64, high: i64 },
}

impl ParamSpace {
    pub fn sample(&self, rng: &mut SimpleRng) -> f64 {
        match self {
            ParamSpace::Continuous { low, high } => low + (high - low) * rng.next_f64(),
            ParamSpace::LogUniform { low, high } => {
                let log_val = low + (high - low) * rng.next_f64();
                10.0_f64.powf(log_val)
            }
            ParamSpace::Discrete { values } => {
                if values.is_empty() { return 0.0; }
                values[(rng.next_u64() as usize) % values.len()]
            }
            ParamSpace::IntRange { low, high } => {
                let range = (high - low + 1) as u64;
                (low + (rng.next_u64() % range) as i64) as f64
            }
        }
    }
}

/// Grid search: enumerate all combinations.
pub fn grid_search(spaces: &[(&str, &[f64])]) -> Vec<HashMap<String, f64>> {
    let mut configs = vec![HashMap::new()];

    for &(name, values) in spaces {
        let mut new_configs = Vec::new();
        for config in &configs {
            for &val in values {
                let mut c = config.clone();
                c.insert(name.to_string(), val);
                new_configs.push(c);
            }
        }
        configs = new_configs;
    }

    configs
}

/// Random search: sample n configurations.
pub fn random_search(
    spaces: &[(&str, ParamSpace)],
    n_trials: usize,
    rng: &mut SimpleRng,
) -> Vec<HashMap<String, f64>> {
    (0..n_trials).map(|_| {
        let mut config = HashMap::new();
        for &(name, ref space) in spaces {
            config.insert(name.to_string(), space.sample(rng));
        }
        config
    }).collect()
}

/// Simple Bayesian optimization step: pick point with highest expected improvement.
pub fn bayesian_suggest(
    observed_x: &[Vec<f64>],
    observed_y: &[f64],
    bounds: &[(f64, f64)],
    n_candidates: usize,
    rng: &mut SimpleRng,
) -> Vec<f64> {
    if observed_x.is_empty() || observed_y.is_empty() {
        // No observations yet → random point
        return bounds.iter().map(|(lo, hi)| lo + (hi - lo) * rng.next_f64()).collect();
    }

    let best_y = observed_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let mut best_candidate = bounds.iter().map(|(lo, hi)| (lo + hi) / 2.0).collect::<Vec<_>>();
    let mut best_ei = f64::NEG_INFINITY;

    for _ in 0..n_candidates {
        let candidate: Vec<f64> = bounds.iter()
            .map(|(lo, hi)| lo + (hi - lo) * rng.next_f64())
            .collect();

        // Simple acquisition: negative distance-weighted estimate
        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;
        for (x, &y) in observed_x.iter().zip(observed_y.iter()) {
            let dist: f64 = candidate.iter().zip(x.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt()
                .max(1e-6);
            let w = 1.0 / dist;
            weighted_sum += w * y;
            weight_total += w;
        }
        let predicted = weighted_sum / weight_total;
        let ei = predicted - best_y;

        if ei > best_ei {
            best_ei = ei;
            best_candidate = candidate;
        }
    }

    best_candidate
}

// ── Model Registry ──────────────────────────────────────────────────────

/// A registered model version.
#[derive(Debug, Clone)]
pub struct ModelVersion {
    pub version: usize,
    pub run_id: String,
    pub metric_value: f64,
    pub stage: ModelStage,
    pub params: HashMap<String, f64>,
}

/// Deployment stage of a model.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelStage {
    Development,
    Staging,
    Production,
    Archived,
}

/// Model registry: tracks model versions and their deployment stages.
#[derive(Debug, Clone)]
pub struct ModelRegistry {
    pub models: HashMap<String, Vec<ModelVersion>>,
}

impl ModelRegistry {
    pub fn new() -> Self { ModelRegistry { models: HashMap::new() } }

    /// Register a new model version.
    pub fn register(&mut self, name: &str, run_id: &str, metric: f64, params: HashMap<String, f64>) -> usize {
        let versions = self.models.entry(name.to_string()).or_default();
        let version = versions.len() + 1;
        versions.push(ModelVersion {
            version,
            run_id: run_id.to_string(),
            metric_value: metric,
            stage: ModelStage::Development,
            params,
        });
        version
    }

    /// Promote a model version to a stage.
    pub fn promote(&mut self, name: &str, version: usize, stage: ModelStage) -> bool {
        if let Some(versions) = self.models.get_mut(name) {
            if let Some(v) = versions.iter_mut().find(|v| v.version == version) {
                v.stage = stage;
                return true;
            }
        }
        false
    }

    /// Get the current production model version.
    pub fn production_version(&self, name: &str) -> Option<&ModelVersion> {
        self.models.get(name)?
            .iter()
            .find(|v| v.stage == ModelStage::Production)
    }

    /// Get the best model by metric (minimize).
    pub fn best_version(&self, name: &str) -> Option<&ModelVersion> {
        self.models.get(name)?
            .iter()
            .min_by(|a, b| a.metric_value.partial_cmp(&b.metric_value).unwrap())
    }

    /// List all versions of a model.
    pub fn list_versions(&self, name: &str) -> &[ModelVersion] {
        self.models.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

// ── Reproducibility ─────────────────────────────────────────────────────

/// Config snapshot for reproducibility.
#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    pub seed: u64,
    pub params: HashMap<String, f64>,
    pub str_params: HashMap<String, String>,
    pub code_hash: String,
}

impl ConfigSnapshot {
    pub fn new(seed: u64) -> Self {
        ConfigSnapshot {
            seed,
            params: HashMap::new(),
            str_params: HashMap::new(),
            code_hash: String::new(),
        }
    }

    pub fn set_param(&mut self, key: &str, value: f64) {
        self.params.insert(key.to_string(), value);
    }

    pub fn set_code_hash(&mut self, hash: &str) {
        self.code_hash = hash.to_string();
    }

    /// Check if two configs are identical (for reproducibility verification).
    pub fn matches(&self, other: &ConfigSnapshot) -> bool {
        self.seed == other.seed
            && self.params == other.params
            && self.code_hash == other.code_hash
    }
}

// ── Simple RNG ──────────────────────────────────────────────────────────

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

static EXP_STORES: Mutex<Option<HashMap<i64, Run>>> = Mutex::new(None);

fn exp_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, Run>>> {
    EXP_STORES.lock().unwrap()
}

fn next_exp_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_exp_create(seed: i64) -> i64 {
    let id = next_exp_id();
    let run = Run::new(&format!("run_{}", id), "experiment", seed as u64);
    let mut store = exp_store();
    store.get_or_insert_with(HashMap::new).insert(id, run);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_exp_log_metric(id: i64, step: i64, value: f64) {
    let mut store = exp_store();
    if let Some(s) = store.as_mut() {
        if let Some(run) = s.get_mut(&id) {
            run.log_metric("loss", value, step as usize);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_exp_get_metric(id: i64) -> f64 {
    let store = exp_store();
    store.as_ref().and_then(|s| s.get(&id))
        .and_then(|r| r.get_metric("loss"))
        .unwrap_or(f64::NAN)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_exp_complete(id: i64) {
    let mut store = exp_store();
    if let Some(s) = store.as_mut() {
        if let Some(run) = s.get_mut(&id) {
            run.complete();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_exp_free(id: i64) {
    let mut store = exp_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_basic() {
        let mut run = Run::new("r1", "test_run", 42);
        run.log_param("lr", 0.001);
        run.log_metric("loss", 2.5, 0);
        run.log_metric("loss", 1.5, 1);
        run.log_metric("loss", 0.8, 2);

        assert_eq!(run.get_metric("loss"), Some(0.8));
        assert_eq!(run.best_metric("loss"), Some(0.8));
        assert_eq!(run.metric_history("loss"), vec![2.5, 1.5, 0.8]);
    }

    #[test]
    fn test_run_status() {
        let mut run = Run::new("r1", "test", 42);
        assert_eq!(run.status, RunStatus::Running);
        run.complete();
        assert_eq!(run.status, RunStatus::Completed);
    }

    #[test]
    fn test_run_tags() {
        let mut run = Run::new("r1", "test", 42);
        run.add_tag("baseline");
        run.add_tag("v2");
        assert_eq!(run.tags.len(), 2);
    }

    #[test]
    fn test_experiment() {
        let mut exp = Experiment::new("test_exp", "Testing");

        let mut r1 = Run::new("r1", "run1", 42);
        r1.log_metric("loss", 0.5, 0);
        r1.complete();
        exp.add_run(r1);

        let mut r2 = Run::new("r2", "run2", 43);
        r2.log_metric("loss", 0.3, 0);
        r2.complete();
        exp.add_run(r2);

        let best = exp.best_run("loss").unwrap();
        assert_eq!(best.id, "r2");
    }

    #[test]
    fn test_metric_summary() {
        let mut exp = Experiment::new("test", "test");
        for i in 0..5 {
            let mut r = Run::new(&format!("r{}", i), "run", i as u64);
            r.log_metric("acc", 0.8 + i as f64 * 0.02, 0);
            r.complete();
            exp.add_run(r);
        }
        let summary = exp.metric_summary("acc").unwrap();
        assert_eq!(summary.count, 5);
        assert!(summary.mean > 0.8);
        assert!(summary.std > 0.0);
    }

    #[test]
    fn test_grid_search() {
        let configs = grid_search(&[
            ("lr", &[0.01, 0.001]),
            ("batch", &[32.0, 64.0]),
        ]);
        assert_eq!(configs.len(), 4); // 2 × 2
        assert!(configs.iter().all(|c| c.contains_key("lr") && c.contains_key("batch")));
    }

    #[test]
    fn test_random_search() {
        let mut rng = SimpleRng::new(42);
        let configs = random_search(&[
            ("lr", ParamSpace::LogUniform { low: -4.0, high: -1.0 }),
            ("hidden", ParamSpace::IntRange { low: 32, high: 256 }),
        ], 10, &mut rng);
        assert_eq!(configs.len(), 10);
        for c in &configs {
            let lr = c["lr"];
            assert!(lr >= 1e-4 && lr <= 0.1);
        }
    }

    #[test]
    fn test_param_space_discrete() {
        let space = ParamSpace::Discrete { values: vec![0.1, 0.01, 0.001] };
        let mut rng = SimpleRng::new(42);
        let val = space.sample(&mut rng);
        assert!(val == 0.1 || val == 0.01 || val == 0.001);
    }

    #[test]
    fn test_bayesian_suggest() {
        let mut rng = SimpleRng::new(42);
        let observed_x = vec![vec![0.1], vec![0.5], vec![0.9]];
        let observed_y = vec![0.5, 0.9, 0.3];
        let bounds = vec![(0.0, 1.0)];
        let suggestion = bayesian_suggest(&observed_x, &observed_y, &bounds, 100, &mut rng);
        assert_eq!(suggestion.len(), 1);
        assert!(suggestion[0] >= 0.0 && suggestion[0] <= 1.0);
    }

    #[test]
    fn test_model_registry() {
        let mut reg = ModelRegistry::new();
        let v1 = reg.register("my_model", "run1", 0.8, HashMap::new());
        let v2 = reg.register("my_model", "run2", 0.6, HashMap::new());

        assert_eq!(v1, 1);
        assert_eq!(v2, 2);

        let best = reg.best_version("my_model").unwrap();
        assert_eq!(best.version, 2); // 0.6 < 0.8

        reg.promote("my_model", 2, ModelStage::Production);
        let prod = reg.production_version("my_model").unwrap();
        assert_eq!(prod.version, 2);
    }

    #[test]
    fn test_model_registry_stages() {
        let mut reg = ModelRegistry::new();
        reg.register("m", "r1", 1.0, HashMap::new());
        reg.promote("m", 1, ModelStage::Staging);

        let versions = reg.list_versions("m");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].stage, ModelStage::Staging);
    }

    #[test]
    fn test_config_snapshot() {
        let mut cfg1 = ConfigSnapshot::new(42);
        cfg1.set_param("lr", 0.001);
        cfg1.set_code_hash("abc123");

        let mut cfg2 = ConfigSnapshot::new(42);
        cfg2.set_param("lr", 0.001);
        cfg2.set_code_hash("abc123");

        assert!(cfg1.matches(&cfg2));
    }

    #[test]
    fn test_config_mismatch() {
        let mut cfg1 = ConfigSnapshot::new(42);
        cfg1.set_param("lr", 0.001);
        let mut cfg2 = ConfigSnapshot::new(42);
        cfg2.set_param("lr", 0.01);
        assert!(!cfg1.matches(&cfg2));
    }

    #[test]
    fn test_ffi_experiment() {
        let id = vitalis_exp_create(42);
        assert!(id > 0);
        vitalis_exp_log_metric(id, 0, 2.5);
        vitalis_exp_log_metric(id, 1, 1.5);
        let val = vitalis_exp_get_metric(id);
        assert!((val - 1.5).abs() < 1e-10);
        vitalis_exp_complete(id);
        vitalis_exp_free(id);
    }

    #[test]
    fn test_best_run_max() {
        let mut exp = Experiment::new("test", "test");
        let mut r1 = Run::new("r1", "run1", 42);
        r1.log_metric("acc", 0.8, 0);
        r1.complete();
        exp.add_run(r1);

        let mut r2 = Run::new("r2", "run2", 43);
        r2.log_metric("acc", 0.95, 0);
        r2.complete();
        exp.add_run(r2);

        let best = exp.best_run_max("acc").unwrap();
        assert_eq!(best.id, "r2");
    }

    #[test]
    fn test_empty_experiment() {
        let exp = Experiment::new("empty", "no runs");
        assert!(exp.best_run("loss").is_none());
        assert!(exp.metric_summary("loss").is_none());
    }
}
