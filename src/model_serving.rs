//! Model Serving — production inference, batched requests, versioning, ONNX export.
//!
//! Provides production ML model serving: model loading, batched inference,
//! request routing, model versioning, A/B testing,
//! caching, and ONNX model format export.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Model Artifact ──────────────────────────────────────────────────────

/// A servable model artifact with weights and metadata.
#[derive(Debug, Clone)]
pub struct ModelArtifact {
    pub name: String,
    pub version: usize,
    pub weights: Vec<f64>,
    pub biases: Vec<f64>,
    pub input_dim: usize,
    pub output_dim: usize,
    pub metadata: HashMap<String, String>,
}

impl ModelArtifact {
    pub fn new(name: &str, version: usize, input_dim: usize, output_dim: usize) -> Self {
        ModelArtifact {
            name: name.to_string(),
            version,
            weights: vec![0.0; input_dim * output_dim],
            biases: vec![0.0; output_dim],
            input_dim, output_dim,
            metadata: HashMap::new(),
        }
    }

    pub fn set_weights(&mut self, weights: Vec<f64>, biases: Vec<f64>) {
        self.weights = weights;
        self.biases = biases;
    }

    /// Simple linear inference: y = Wx + b
    pub fn predict(&self, input: &[f64]) -> Vec<f64> {
        let mut output = self.biases.clone();
        for j in 0..self.output_dim {
            for i in 0..self.input_dim.min(input.len()) {
                output[j] += self.weights[j * self.input_dim + i] * input[i];
            }
        }
        output
    }

    /// Softmax prediction (classification).
    pub fn predict_class(&self, input: &[f64]) -> usize {
        let logits = self.predict(input);
        logits.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()).unwrap().0
    }

    /// Predict with softmax probabilities.
    pub fn predict_proba(&self, input: &[f64]) -> Vec<f64> {
        let logits = self.predict(input);
        softmax(&logits)
    }
}

// ── Batched Inference ───────────────────────────────────────────────────

/// Request for model inference.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub id: u64,
    pub input: Vec<f64>,
    pub model_name: String,
    pub model_version: Option<usize>,
}

/// Response from model inference.
#[derive(Debug, Clone)]
pub struct InferenceResponse {
    pub id: u64,
    pub output: Vec<f64>,
    pub latency_us: u64,
    pub model_version: usize,
}

/// Batched inference engine.
#[derive(Debug, Clone)]
pub struct InferenceEngine {
    pub models: HashMap<String, Vec<ModelArtifact>>,
    pub max_batch_size: usize,
    pub total_requests: u64,
    pub total_latency_us: u64,
}

impl InferenceEngine {
    pub fn new(max_batch_size: usize) -> Self {
        InferenceEngine {
            models: HashMap::new(),
            max_batch_size,
            total_requests: 0,
            total_latency_us: 0,
        }
    }

    /// Load a model into the engine.
    pub fn load_model(&mut self, model: ModelArtifact) {
        self.models.entry(model.name.clone())
            .or_default()
            .push(model);
    }

    /// Get the latest version of a model.
    pub fn get_model(&self, name: &str, version: Option<usize>) -> Option<&ModelArtifact> {
        let versions = self.models.get(name)?;
        match version {
            Some(v) => versions.iter().find(|m| m.version == v),
            None => versions.last(),
        }
    }

    /// Process a single inference request.
    pub fn infer(&mut self, request: &InferenceRequest) -> Option<InferenceResponse> {
        let model = self.get_model(&request.model_name, request.model_version)?;
        let output = model.predict(&request.input);
        let version = model.version;

        self.total_requests += 1;
        self.total_latency_us += 1; // Simulated

        Some(InferenceResponse {
            id: request.id,
            output,
            latency_us: 1,
            model_version: version,
        })
    }

    /// Process a batch of requests.
    pub fn infer_batch(&mut self, requests: &[InferenceRequest]) -> Vec<Option<InferenceResponse>> {
        requests.iter().map(|req| self.infer(req)).collect()
    }

    /// Average latency per request (microseconds).
    pub fn avg_latency_us(&self) -> f64 {
        if self.total_requests == 0 { return 0.0; }
        self.total_latency_us as f64 / self.total_requests as f64
    }
}

// ── Model Router (A/B Testing + Canary) ─────────────────────────────────

/// Traffic routing rule for A/B testing.
#[derive(Debug, Clone)]
pub struct RoutingRule {
    pub name: String,
    pub model_a: String,
    pub version_a: usize,
    pub model_b: String,
    pub version_b: usize,
    pub traffic_split: f64, // fraction going to model_b
}

/// Model router for canary deployments and A/B tests.
#[derive(Debug, Clone)]
pub struct ModelRouter {
    pub rules: Vec<RoutingRule>,
    pub request_counter: u64,
}

impl ModelRouter {
    pub fn new() -> Self { ModelRouter { rules: vec![], request_counter: 0 } }

    pub fn add_rule(&mut self, rule: RoutingRule) {
        self.rules.push(rule);
    }

    /// Route a request using deterministic hashing.
    pub fn route(&mut self, model_name: &str) -> Option<(String, usize)> {
        self.request_counter += 1;
        for rule in &self.rules {
            if rule.name == model_name {
                // Use request counter for deterministic splitting
                let fraction = (self.request_counter % 100) as f64 / 100.0;
                if fraction < rule.traffic_split {
                    return Some((rule.model_b.clone(), rule.version_b));
                } else {
                    return Some((rule.model_a.clone(), rule.version_a));
                }
            }
        }
        None
    }
}

// ── Prediction Cache ────────────────────────────────────────────────────

/// LRU-style cache for model predictions.
#[derive(Debug, Clone)]
pub struct PredictionCache {
    pub cache: HashMap<u64, Vec<f64>>,
    pub max_size: usize,
    pub hits: u64,
    pub misses: u64,
}

impl PredictionCache {
    pub fn new(max_size: usize) -> Self {
        PredictionCache { cache: HashMap::new(), max_size, hits: 0, misses: 0 }
    }

    fn hash_input(input: &[f64]) -> u64 {
        let mut h: u64 = 14695981039346656037;
        for &v in input {
            h ^= v.to_bits();
            h = h.wrapping_mul(1099511628211);
        }
        h
    }

    pub fn get(&mut self, input: &[f64]) -> Option<&Vec<f64>> {
        let key = Self::hash_input(input);
        if self.cache.contains_key(&key) {
            self.hits += 1;
            self.cache.get(&key)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn put(&mut self, input: &[f64], output: Vec<f64>) {
        if self.cache.len() >= self.max_size {
            // Evict first key (simple eviction)
            if let Some(&key) = self.cache.keys().next() {
                self.cache.remove(&key);
            }
        }
        let key = Self::hash_input(input);
        self.cache.insert(key, output);
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { return 0.0; }
        self.hits as f64 / total as f64
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

// ── ONNX Export (Simplified) ────────────────────────────────────────────

/// Simplified ONNX model representation for export.
#[derive(Debug, Clone)]
pub struct OnnxModel {
    pub ir_version: u32,
    pub producer: String,
    pub model_version: u64,
    pub nodes: Vec<OnnxNode>,
    pub inputs: Vec<OnnxTensor>,
    pub outputs: Vec<OnnxTensor>,
    pub initializers: Vec<OnnxTensor>,
}

/// ONNX computation graph node.
#[derive(Debug, Clone)]
pub struct OnnxNode {
    pub op_type: String,
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

/// ONNX tensor descriptor.
#[derive(Debug, Clone)]
pub struct OnnxTensor {
    pub name: String,
    pub dims: Vec<usize>,
    pub data: Vec<f64>,
}

impl OnnxModel {
    /// Export a simple linear model to ONNX-like representation.
    pub fn from_linear(model: &ModelArtifact) -> Self {
        let weight_tensor = OnnxTensor {
            name: "weight".to_string(),
            dims: vec![model.output_dim, model.input_dim],
            data: model.weights.clone(),
        };
        let bias_tensor = OnnxTensor {
            name: "bias".to_string(),
            dims: vec![model.output_dim],
            data: model.biases.clone(),
        };
        let input_tensor = OnnxTensor {
            name: "input".to_string(),
            dims: vec![1, model.input_dim],
            data: vec![],
        };
        let output_tensor = OnnxTensor {
            name: "output".to_string(),
            dims: vec![1, model.output_dim],
            data: vec![],
        };

        let matmul_node = OnnxNode {
            op_type: "MatMul".to_string(),
            name: "matmul_0".to_string(),
            inputs: vec!["input".to_string(), "weight".to_string()],
            outputs: vec!["matmul_out".to_string()],
        };
        let add_node = OnnxNode {
            op_type: "Add".to_string(),
            name: "add_0".to_string(),
            inputs: vec!["matmul_out".to_string(), "bias".to_string()],
            outputs: vec!["output".to_string()],
        };

        OnnxModel {
            ir_version: 7,
            producer: "vitalis".to_string(),
            model_version: model.version as u64,
            nodes: vec![matmul_node, add_node],
            inputs: vec![input_tensor],
            outputs: vec![output_tensor],
            initializers: vec![weight_tensor, bias_tensor],
        }
    }

    pub fn n_nodes(&self) -> usize { self.nodes.len() }
    pub fn n_params(&self) -> usize {
        self.initializers.iter().map(|t| t.data.len()).sum()
    }
}

// ── Health Check ────────────────────────────────────────────────────────

/// Server health status.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub models_loaded: usize,
    pub total_requests: u64,
    pub avg_latency_us: f64,
    pub cache_hit_rate: f64,
}

impl InferenceEngine {
    pub fn health(&self) -> HealthStatus {
        let models_loaded = self.models.values().map(|v| v.len()).sum();
        HealthStatus {
            is_healthy: models_loaded > 0,
            models_loaded,
            total_requests: self.total_requests,
            avg_latency_us: self.avg_latency_us(),
            cache_hit_rate: 0.0,
        }
    }
}

// ── Utility ─────────────────────────────────────────────────────────────

fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&l| (l - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

// ── FFI Interface ───────────────────────────────────────────────────────

static SERVE_STORES: Mutex<Option<HashMap<i64, InferenceEngine>>> = Mutex::new(None);

fn serve_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, InferenceEngine>>> {
    SERVE_STORES.lock().unwrap()
}

fn next_serve_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_serve_create(max_batch: i64) -> i64 {
    let id = next_serve_id();
    let engine = InferenceEngine::new(max_batch as usize);
    let mut store = serve_store();
    store.get_or_insert_with(HashMap::new).insert(id, engine);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_serve_load_model(id: i64, input_dim: i64, output_dim: i64, version: i64) -> i64 {
    let mut store = serve_store();
    if let Some(s) = store.as_mut() {
        if let Some(engine) = s.get_mut(&id) {
            let model = ModelArtifact::new("model", version as usize, input_dim as usize, output_dim as usize);
            engine.load_model(model);
            return 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_serve_predict(id: i64, input_ptr: *const f64, input_len: i64, output_ptr: *mut f64, output_len: i64) -> i64 {
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len as usize) };
    let mut store = serve_store();
    if let Some(s) = store.as_mut() {
        if let Some(engine) = s.get_mut(&id) {
            let req = InferenceRequest {
                id: 0,
                input: input.to_vec(),
                model_name: "model".to_string(),
                model_version: None,
            };
            if let Some(resp) = engine.infer(&req) {
                let out_slice = unsafe { std::slice::from_raw_parts_mut(output_ptr, output_len as usize) };
                for (i, &v) in resp.output.iter().enumerate() {
                    if i < out_slice.len() { out_slice[i] = v; }
                }
                return 1;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_serve_free(id: i64) {
    let mut store = serve_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_artifact() {
        let mut model = ModelArtifact::new("test", 1, 3, 2);
        model.set_weights(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0], vec![0.1, 0.2]);
        let output = model.predict(&[1.0, 2.0, 3.0]);
        assert_eq!(output.len(), 2);
        assert!((output[0] - 1.1).abs() < 1e-10); // 1*1 + 0*2 + 0*3 + 0.1
        assert!((output[1] - 2.2).abs() < 1e-10); // 0*1 + 1*2 + 0*3 + 0.2
    }

    #[test]
    fn test_predict_class() {
        let mut model = ModelArtifact::new("test", 1, 2, 3);
        model.set_weights(vec![1.0, 0.0, 0.0, 1.0, -1.0, -1.0], vec![0.0, 0.0, 0.0]);
        let cls = model.predict_class(&[5.0, 1.0]);
        assert_eq!(cls, 0); // First class has highest logit (5.0)
    }

    #[test]
    fn test_predict_proba() {
        let model = ModelArtifact::new("test", 1, 2, 2);
        let probs = model.predict_proba(&[0.0, 0.0]);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_inference_engine() {
        let mut engine = InferenceEngine::new(32);
        let model = ModelArtifact::new("my_model", 1, 4, 2);
        engine.load_model(model);

        let req = InferenceRequest {
            id: 1,
            input: vec![1.0, 2.0, 3.0, 4.0],
            model_name: "my_model".to_string(),
            model_version: None,
        };
        let resp = engine.infer(&req).unwrap();
        assert_eq!(resp.output.len(), 2);
    }

    #[test]
    fn test_batch_inference() {
        let mut engine = InferenceEngine::new(32);
        engine.load_model(ModelArtifact::new("m", 1, 2, 1));

        let requests: Vec<InferenceRequest> = (0..5).map(|i| InferenceRequest {
            id: i,
            input: vec![i as f64, 0.0],
            model_name: "m".to_string(),
            model_version: None,
        }).collect();

        let responses = engine.infer_batch(&requests);
        assert_eq!(responses.len(), 5);
        assert!(responses.iter().all(|r| r.is_some()));
    }

    #[test]
    fn test_model_versioning() {
        let mut engine = InferenceEngine::new(32);
        engine.load_model(ModelArtifact::new("m", 1, 2, 1));
        engine.load_model(ModelArtifact::new("m", 2, 2, 1));

        let v1 = engine.get_model("m", Some(1)).unwrap();
        assert_eq!(v1.version, 1);

        let latest = engine.get_model("m", None).unwrap();
        assert_eq!(latest.version, 2);
    }

    #[test]
    fn test_prediction_cache() {
        let mut cache = PredictionCache::new(100);
        let input = vec![1.0, 2.0, 3.0];
        assert!(cache.get(&input).is_none());

        cache.put(&input, vec![0.5]);
        assert_eq!(cache.get(&input).unwrap(), &vec![0.5]);
        assert!(cache.hit_rate() > 0.0);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = PredictionCache::new(2);
        cache.put(&[1.0], vec![1.0]);
        cache.put(&[2.0], vec![2.0]);
        cache.put(&[3.0], vec![3.0]); // Should evict one
        assert!(cache.cache.len() <= 2);
    }

    #[test]
    fn test_model_router() {
        let mut router = ModelRouter::new();
        router.add_rule(RoutingRule {
            name: "model".to_string(),
            model_a: "model".to_string(), version_a: 1,
            model_b: "model".to_string(), version_b: 2,
            traffic_split: 0.2,
        });

        let mut v1_count = 0;
        let mut v2_count = 0;
        for _ in 0..100 {
            let (_, version) = router.route("model").unwrap();
            if version == 1 { v1_count += 1; } else { v2_count += 1; }
        }
        assert!(v1_count > v2_count); // 80% to v1
    }

    #[test]
    fn test_onnx_export() {
        let model = ModelArtifact::new("test", 1, 4, 2);
        let onnx = OnnxModel::from_linear(&model);
        assert_eq!(onnx.n_nodes(), 2);
        assert_eq!(onnx.n_params(), 4 * 2 + 2); // weights + biases
        assert_eq!(onnx.producer, "vitalis");
    }

    #[test]
    fn test_health_check() {
        let mut engine = InferenceEngine::new(32);
        let health = engine.health();
        assert!(!health.is_healthy); // No models loaded

        engine.load_model(ModelArtifact::new("m", 1, 2, 1));
        let health = engine.health();
        assert!(health.is_healthy);
    }

    #[test]
    fn test_ffi_serve() {
        let id = vitalis_serve_create(32);
        assert!(id > 0);
        vitalis_serve_load_model(id, 3, 2, 1);

        let input = vec![1.0, 2.0, 3.0];
        let mut output = vec![0.0; 2];
        let ok = vitalis_serve_predict(id, input.as_ptr(), 3, output.as_mut_ptr(), 2);
        assert_eq!(ok, 1);

        vitalis_serve_free(id);
    }
}
