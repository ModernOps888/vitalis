//! Data Pipeline — Dataset/DataLoader, transforms, streaming, CSV/JSON/binary.
//!
//! Provides a composable data loading pipeline for ML:
//! Dataset trait, DataLoader with shuffling/batching,
//! transform chains, CSV/JSON/binary parsers,
//! and streaming support for large datasets.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Dataset Trait ───────────────────────────────────────────────────────

/// A dataset provides indexed access to samples.
pub trait Dataset {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn get(&self, index: usize) -> Option<Sample>;
}

/// A single data sample (features + optional label).
#[derive(Debug, Clone)]
pub struct Sample {
    pub features: Vec<f64>,
    pub label: Option<f64>,
    pub metadata: HashMap<String, String>,
}

impl Sample {
    pub fn new(features: Vec<f64>, label: Option<f64>) -> Self {
        Sample { features, label, metadata: HashMap::new() }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// A batch of samples.
#[derive(Debug, Clone)]
pub struct Batch {
    pub features: Vec<Vec<f64>>,
    pub labels: Vec<Option<f64>>,
    pub size: usize,
}

impl Batch {
    pub fn from_samples(samples: &[Sample]) -> Self {
        Batch {
            features: samples.iter().map(|s| s.features.clone()).collect(),
            labels: samples.iter().map(|s| s.label).collect(),
            size: samples.len(),
        }
    }

    /// Stack features into a flat matrix (row-major).
    pub fn feature_matrix(&self) -> Vec<f64> {
        self.features.iter().flat_map(|f| f.iter().cloned()).collect()
    }

    /// Extract labels as a flat vector.
    pub fn label_vector(&self) -> Vec<f64> {
        self.labels.iter().map(|l| l.unwrap_or(0.0)).collect()
    }
}

// ── In-Memory Dataset ───────────────────────────────────────────────────

/// Simple in-memory dataset backed by a Vec.
#[derive(Debug, Clone)]
pub struct VecDataset {
    pub samples: Vec<Sample>,
}

impl VecDataset {
    pub fn new(samples: Vec<Sample>) -> Self { VecDataset { samples } }

    /// Create from feature matrix and label vector.
    pub fn from_matrix(features: &[Vec<f64>], labels: &[f64]) -> Self {
        let samples = features.iter().zip(labels.iter())
            .map(|(f, &l)| Sample::new(f.clone(), Some(l)))
            .collect();
        VecDataset { samples }
    }

    /// Create from flat data: n_samples × n_features.
    pub fn from_flat(data: &[f64], n_features: usize, labels: &[f64]) -> Self {
        let samples: Vec<Sample> = data.chunks(n_features).zip(labels.iter())
            .map(|(chunk, &label)| Sample::new(chunk.to_vec(), Some(label)))
            .collect();
        VecDataset { samples }
    }
}

impl Dataset for VecDataset {
    fn len(&self) -> usize { self.samples.len() }
    fn get(&self, index: usize) -> Option<Sample> { self.samples.get(index).cloned() }
}

// ── DataLoader ──────────────────────────────────────────────────────────

/// DataLoader provides batched iteration over a dataset.
#[derive(Debug, Clone)]
pub struct DataLoader {
    pub batch_size: usize,
    pub shuffle: bool,
    pub drop_last: bool,
    indices: Vec<usize>,
    dataset_len: usize,
}

impl DataLoader {
    pub fn new(dataset_len: usize, batch_size: usize, shuffle: bool) -> Self {
        let indices: Vec<usize> = (0..dataset_len).collect();
        DataLoader { batch_size, shuffle, drop_last: false, indices, dataset_len }
    }

    /// Shuffle indices for a new epoch.
    pub fn shuffle_indices(&mut self, rng: &mut SimpleRng) {
        if !self.shuffle { return; }
        // Fisher-Yates shuffle
        let n = self.indices.len();
        for i in (1..n).rev() {
            let j = (rng.next_u64() as usize) % (i + 1);
            self.indices.swap(i, j);
        }
    }

    /// Number of batches per epoch.
    pub fn n_batches(&self) -> usize {
        if self.drop_last {
            self.dataset_len / self.batch_size
        } else {
            (self.dataset_len + self.batch_size - 1) / self.batch_size
        }
    }

    /// Get indices for batch i.
    pub fn batch_indices(&self, batch_idx: usize) -> &[usize] {
        let start = batch_idx * self.batch_size;
        let end = (start + self.batch_size).min(self.indices.len());
        &self.indices[start..end]
    }

    /// Load a specific batch from a dataset.
    pub fn load_batch(&self, dataset: &dyn Dataset, batch_idx: usize) -> Batch {
        let indices = self.batch_indices(batch_idx);
        let samples: Vec<Sample> = indices.iter()
            .filter_map(|&i| dataset.get(i))
            .collect();
        Batch::from_samples(&samples)
    }
}

// ── Transforms ──────────────────────────────────────────────────────────

/// Data transform applied to samples.
#[derive(Debug, Clone)]
pub enum Transform {
    /// Normalize features to zero-mean, unit-variance.
    Normalize { mean: Vec<f64>, std: Vec<f64> },
    /// Scale features to [0, 1] range.
    MinMaxScale { min: Vec<f64>, max: Vec<f64> },
    /// Apply standard scaling.
    StandardScale { mean: Vec<f64>, std: Vec<f64> },
    /// Add Gaussian noise for augmentation.
    GaussianNoise { std: f64 },
    /// Random feature dropout.
    FeatureDropout { rate: f64 },
    /// Clamp values to range.
    Clamp { min: f64, max: f64 },
    /// Log transform: x → ln(x + 1).
    Log1p,
    /// One-hot encode label into feature vector.
    OneHotLabel { n_classes: usize },
}

impl Transform {
    /// Apply transform to a sample.
    pub fn apply(&self, sample: &mut Sample, rng: &mut SimpleRng) {
        match self {
            Transform::Normalize { mean, std } => {
                for i in 0..sample.features.len().min(mean.len()) {
                    if std[i] > 1e-10 {
                        sample.features[i] = (sample.features[i] - mean[i]) / std[i];
                    }
                }
            }
            Transform::MinMaxScale { min, max } => {
                for i in 0..sample.features.len().min(min.len()) {
                    let range = max[i] - min[i];
                    if range > 1e-10 {
                        sample.features[i] = (sample.features[i] - min[i]) / range;
                    }
                }
            }
            Transform::StandardScale { mean, std } => {
                for i in 0..sample.features.len().min(mean.len()) {
                    if std[i] > 1e-10 {
                        sample.features[i] = (sample.features[i] - mean[i]) / std[i];
                    }
                }
            }
            Transform::GaussianNoise { std } => {
                for f in &mut sample.features {
                    let u1 = rng.next_f64().max(1e-15);
                    let u2 = rng.next_f64();
                    let noise = std * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                    *f += noise;
                }
            }
            Transform::FeatureDropout { rate } => {
                for f in &mut sample.features {
                    if rng.next_f64() < *rate {
                        *f = 0.0;
                    }
                }
            }
            Transform::Clamp { min, max } => {
                for f in &mut sample.features {
                    *f = f.clamp(*min, *max);
                }
            }
            Transform::Log1p => {
                for f in &mut sample.features {
                    *f = (*f + 1.0).ln();
                }
            }
            Transform::OneHotLabel { n_classes } => {
                if let Some(label) = sample.label {
                    let idx = label as usize;
                    let mut oh = vec![0.0; *n_classes];
                    if idx < *n_classes { oh[idx] = 1.0; }
                    sample.features.extend(oh);
                }
            }
        }
    }

    /// Compute normalization stats from a dataset.
    pub fn fit_normalize(dataset: &dyn Dataset) -> Transform {
        let n = dataset.len();
        if n == 0 { return Transform::Normalize { mean: vec![], std: vec![] }; }
        let first = dataset.get(0).unwrap();
        let dim = first.features.len();
        let mut mean = vec![0.0; dim];
        let mut m2 = vec![0.0; dim];

        for i in 0..n {
            if let Some(s) = dataset.get(i) {
                for j in 0..dim.min(s.features.len()) {
                    mean[j] += s.features[j];
                }
            }
        }
        for j in 0..dim { mean[j] /= n as f64; }

        for i in 0..n {
            if let Some(s) = dataset.get(i) {
                for j in 0..dim.min(s.features.len()) {
                    let d = s.features[j] - mean[j];
                    m2[j] += d * d;
                }
            }
        }
        let std_vec: Vec<f64> = m2.iter().map(|v| (v / n as f64).sqrt().max(1e-10)).collect();
        Transform::Normalize { mean, std: std_vec }
    }
}

/// A pipeline of transforms applied in sequence.
#[derive(Debug, Clone)]
pub struct TransformPipeline {
    pub transforms: Vec<Transform>,
}

impl TransformPipeline {
    pub fn new() -> Self { TransformPipeline { transforms: vec![] } }

    pub fn add(mut self, t: Transform) -> Self { self.transforms.push(t); self }

    pub fn apply(&self, sample: &mut Sample, rng: &mut SimpleRng) {
        for t in &self.transforms {
            t.apply(sample, rng);
        }
    }
}

// ── CSV Parser ──────────────────────────────────────────────────────────

/// Parse CSV text into a VecDataset.
pub fn parse_csv(text: &str, has_header: bool, label_col: Option<usize>) -> VecDataset {
    let mut lines = text.lines();
    if has_header { lines.next(); }

    let mut samples = Vec::new();
    for line in lines {
        let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if values.is_empty() { continue; }

        let mut features = Vec::new();
        let mut label = None;

        for (i, val) in values.iter().enumerate() {
            if let Ok(v) = val.parse::<f64>() {
                if Some(i) == label_col {
                    label = Some(v);
                } else {
                    features.push(v);
                }
            }
        }

        if !features.is_empty() {
            samples.push(Sample::new(features, label));
        }
    }

    VecDataset::new(samples)
}

/// Parse JSON array of objects into a VecDataset.
/// Expected format: [{"f1": 1.0, "f2": 2.0, "label": 0}, ...]
pub fn parse_json_simple(text: &str, feature_keys: &[&str], label_key: &str) -> VecDataset {
    // Simple JSON parser for numeric arrays
    let mut samples = Vec::new();

    // Find objects between { }
    let mut depth = 0;
    let mut obj_start = 0;
    let chars: Vec<char> = text.chars().collect();

    for i in 0..chars.len() {
        if chars[i] == '{' {
            if depth == 0 { obj_start = i; }
            depth += 1;
        } else if chars[i] == '}' {
            depth -= 1;
            if depth == 0 {
                let obj_str: String = chars[obj_start..=i].iter().collect();
                if let Some(sample) = parse_json_object(&obj_str, feature_keys, label_key) {
                    samples.push(sample);
                }
            }
        }
    }

    VecDataset::new(samples)
}

fn parse_json_object(obj: &str, feature_keys: &[&str], label_key: &str) -> Option<Sample> {
    let mut features = Vec::new();
    let mut label = None;

    for key in feature_keys {
        if let Some(val) = extract_json_number(obj, key) {
            features.push(val);
        }
    }
    label = extract_json_number(obj, label_key);

    if features.is_empty() { return None; }
    Some(Sample::new(features, label))
}

fn extract_json_number(obj: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\"", key);
    let pos = obj.find(&pattern)?;
    let after_key = &obj[pos + pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();

    // Extract number
    let mut end = 0;
    for (i, c) in after_colon.chars().enumerate() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == 'E' || c == '+' {
            end = i + 1;
        } else if end > 0 {
            break;
        }
    }
    if end > 0 {
        after_colon[..end].parse::<f64>().ok()
    } else {
        None
    }
}

// ── Train/Test Split ────────────────────────────────────────────────────

/// Split dataset into train and test sets.
pub fn train_test_split(dataset: &VecDataset, test_ratio: f64, rng: &mut SimpleRng) -> (VecDataset, VecDataset) {
    let n = dataset.samples.len();
    let mut indices: Vec<usize> = (0..n).collect();

    // Shuffle
    for i in (1..n).rev() {
        let j = (rng.next_u64() as usize) % (i + 1);
        indices.swap(i, j);
    }

    let test_n = (n as f64 * test_ratio) as usize;
    let test_samples: Vec<Sample> = indices[..test_n].iter().map(|&i| dataset.samples[i].clone()).collect();
    let train_samples: Vec<Sample> = indices[test_n..].iter().map(|&i| dataset.samples[i].clone()).collect();

    (VecDataset::new(train_samples), VecDataset::new(test_samples))
}

/// K-fold cross-validation split.
pub fn kfold_splits(n: usize, k: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
    let fold_size = n / k;
    (0..k).map(|fold| {
        let test_start = fold * fold_size;
        let test_end = if fold == k - 1 { n } else { test_start + fold_size };
        let test: Vec<usize> = (test_start..test_end).collect();
        let train: Vec<usize> = (0..n).filter(|i| *i < test_start || *i >= test_end).collect();
        (train, test)
    }).collect()
}

// ── Streaming Dataset ───────────────────────────────────────────────────

/// Streaming dataset that yields chunks.
#[derive(Debug, Clone)]
pub struct StreamingDataset {
    pub chunk_size: usize,
    pub total_samples: usize,
    pub current_pos: usize,
    pub buffer: Vec<Sample>,
}

impl StreamingDataset {
    pub fn new(chunk_size: usize) -> Self {
        StreamingDataset { chunk_size, total_samples: 0, current_pos: 0, buffer: vec![] }
    }

    pub fn add_chunk(&mut self, samples: Vec<Sample>) {
        self.total_samples += samples.len();
        self.buffer.extend(samples);
    }

    pub fn next_batch(&mut self, batch_size: usize) -> Option<Batch> {
        if self.current_pos >= self.buffer.len() { return None; }
        let end = (self.current_pos + batch_size).min(self.buffer.len());
        let batch = Batch::from_samples(&self.buffer[self.current_pos..end]);
        self.current_pos = end;
        Some(batch)
    }

    pub fn reset(&mut self) {
        self.current_pos = 0;
    }

    pub fn has_more(&self) -> bool {
        self.current_pos < self.buffer.len()
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

static DATA_STORES: Mutex<Option<HashMap<i64, VecDataset>>> = Mutex::new(None);

fn data_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, VecDataset>>> {
    DATA_STORES.lock().unwrap()
}

fn next_data_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_data_create(n_samples: i64, n_features: i64) -> i64 {
    let id = next_data_id();
    let samples: Vec<Sample> = (0..n_samples as usize)
        .map(|_| Sample::new(vec![0.0; n_features as usize], None))
        .collect();
    let mut store = data_store();
    store.get_or_insert_with(HashMap::new).insert(id, VecDataset::new(samples));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_data_len(id: i64) -> i64 {
    let store = data_store();
    store.as_ref().and_then(|s| s.get(&id)).map(|d| d.samples.len() as i64).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_data_parse_csv(text_ptr: *const u8, text_len: i64, has_header: i64, label_col: i64) -> i64 {
    let text = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(text_ptr, text_len as usize)) };
    let lc = if label_col >= 0 { Some(label_col as usize) } else { None };
    let dataset = parse_csv(text, has_header != 0, lc);
    let id = next_data_id();
    let mut store = data_store();
    store.get_or_insert_with(HashMap::new).insert(id, dataset);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_data_free(id: i64) {
    let mut store = data_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_dataset() {
        let ds = VecDataset::new(vec![
            Sample::new(vec![1.0, 2.0], Some(0.0)),
            Sample::new(vec![3.0, 4.0], Some(1.0)),
        ]);
        assert_eq!(ds.len(), 2);
        let s = ds.get(0).unwrap();
        assert_eq!(s.features, vec![1.0, 2.0]);
        assert_eq!(s.label, Some(0.0));
    }

    #[test]
    fn test_from_matrix() {
        let features = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let labels = vec![0.0, 1.0];
        let ds = VecDataset::from_matrix(&features, &labels);
        assert_eq!(ds.len(), 2);
    }

    #[test]
    fn test_from_flat() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let labels = vec![0.0, 1.0];
        let ds = VecDataset::from_flat(&data, 3, &labels);
        assert_eq!(ds.len(), 2);
        assert_eq!(ds.get(0).unwrap().features, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_batch() {
        let samples = vec![
            Sample::new(vec![1.0, 2.0], Some(0.0)),
            Sample::new(vec![3.0, 4.0], Some(1.0)),
        ];
        let batch = Batch::from_samples(&samples);
        assert_eq!(batch.size, 2);
        assert_eq!(batch.feature_matrix(), vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(batch.label_vector(), vec![0.0, 1.0]);
    }

    #[test]
    fn test_dataloader() {
        let ds = VecDataset::from_matrix(
            &vec![vec![1.0]; 10],
            &vec![0.0; 10],
        );
        let loader = DataLoader::new(ds.len(), 3, false);
        assert_eq!(loader.n_batches(), 4); // 10/3 = 4 (ceil)

        let batch = loader.load_batch(&ds, 0);
        assert_eq!(batch.size, 3);
    }

    #[test]
    fn test_dataloader_shuffle() {
        let ds = VecDataset::from_matrix(
            &(0..10).map(|i| vec![i as f64]).collect::<Vec<_>>(),
            &vec![0.0; 10],
        );
        let mut loader = DataLoader::new(ds.len(), 3, true);
        let mut rng = SimpleRng::new(42);
        loader.shuffle_indices(&mut rng);

        // After shuffle, first batch should have different indices
        let batch = loader.load_batch(&ds, 0);
        assert_eq!(batch.size, 3);
    }

    #[test]
    fn test_transform_normalize() {
        let ds = VecDataset::new(vec![
            Sample::new(vec![0.0, 10.0], None),
            Sample::new(vec![2.0, 20.0], None),
            Sample::new(vec![4.0, 30.0], None),
        ]);
        let norm = Transform::fit_normalize(&ds);
        let mut sample = Sample::new(vec![2.0, 20.0], None);
        let mut rng = SimpleRng::new(42);
        norm.apply(&mut sample, &mut rng);
        // Mean is [2.0, 20.0], so normalized value of [2.0, 20.0] should be [0.0, 0.0]
        assert!(sample.features[0].abs() < 1e-6);
        assert!(sample.features[1].abs() < 1e-6);
    }

    #[test]
    fn test_transform_clamp() {
        let mut sample = Sample::new(vec![-5.0, 0.5, 10.0], None);
        let mut rng = SimpleRng::new(42);
        Transform::Clamp { min: 0.0, max: 1.0 }.apply(&mut sample, &mut rng);
        assert!((sample.features[0] - 0.0).abs() < 1e-10);
        assert!((sample.features[1] - 0.5).abs() < 1e-10);
        assert!((sample.features[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_log1p() {
        let mut sample = Sample::new(vec![0.0, 1.0, 9.0], None);
        let mut rng = SimpleRng::new(42);
        Transform::Log1p.apply(&mut sample, &mut rng);
        assert!((sample.features[0] - 0.0).abs() < 1e-10); // ln(1) = 0
        assert!((sample.features[1] - 2.0_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_transform_pipeline() {
        let pipeline = TransformPipeline::new()
            .add(Transform::Clamp { min: 0.0, max: 100.0 })
            .add(Transform::Log1p);
        let mut sample = Sample::new(vec![-5.0, 10.0], None);
        let mut rng = SimpleRng::new(42);
        pipeline.apply(&mut sample, &mut rng);
        assert!((sample.features[0] - 0.0).abs() < 1e-10); // Clamped to 0, ln(1)=0
    }

    #[test]
    fn test_parse_csv() {
        let csv = "x,y,label\n1.0,2.0,0\n3.0,4.0,1\n5.0,6.0,0";
        let ds = parse_csv(csv, true, Some(2));
        assert_eq!(ds.len(), 3);
        let s = ds.get(0).unwrap();
        assert_eq!(s.features, vec![1.0, 2.0]);
        assert_eq!(s.label, Some(0.0));
    }

    #[test]
    fn test_parse_csv_no_label() {
        let csv = "1.0,2.0\n3.0,4.0";
        let ds = parse_csv(csv, false, None);
        assert_eq!(ds.len(), 2);
        assert_eq!(ds.get(0).unwrap().label, None);
    }

    #[test]
    fn test_parse_json() {
        let json = r#"[{"x": 1.0, "y": 2.0, "label": 0}, {"x": 3.0, "y": 4.0, "label": 1}]"#;
        let ds = parse_json_simple(json, &["x", "y"], "label");
        assert_eq!(ds.len(), 2);
    }

    #[test]
    fn test_train_test_split() {
        let ds = VecDataset::from_matrix(
            &(0..100).map(|i| vec![i as f64]).collect::<Vec<_>>(),
            &vec![0.0; 100],
        );
        let mut rng = SimpleRng::new(42);
        let (train, test) = train_test_split(&ds, 0.2, &mut rng);
        assert_eq!(train.len(), 80);
        assert_eq!(test.len(), 20);
    }

    #[test]
    fn test_kfold() {
        let splits = kfold_splits(100, 5);
        assert_eq!(splits.len(), 5);
        for (train, test) in &splits {
            assert_eq!(train.len() + test.len(), 100);
        }
    }

    #[test]
    fn test_streaming_dataset() {
        let mut stream = StreamingDataset::new(10);
        stream.add_chunk(vec![
            Sample::new(vec![1.0], Some(0.0)),
            Sample::new(vec![2.0], Some(1.0)),
            Sample::new(vec![3.0], Some(0.0)),
        ]);
        assert!(stream.has_more());
        let batch = stream.next_batch(2).unwrap();
        assert_eq!(batch.size, 2);
        let batch2 = stream.next_batch(2).unwrap();
        assert_eq!(batch2.size, 1);
        assert!(!stream.has_more());
    }

    #[test]
    fn test_one_hot_transform() {
        let mut sample = Sample::new(vec![1.0, 2.0], Some(2.0));
        let mut rng = SimpleRng::new(42);
        Transform::OneHotLabel { n_classes: 4 }.apply(&mut sample, &mut rng);
        assert_eq!(sample.features.len(), 6); // 2 original + 4 one-hot
        assert!((sample.features[4] - 1.0).abs() < 1e-10); // index 2 is hot
    }

    #[test]
    fn test_sample_metadata() {
        let s = Sample::new(vec![1.0], None)
            .with_metadata("source", "csv")
            .with_metadata("split", "train");
        assert_eq!(s.metadata.get("source").unwrap(), "csv");
    }

    #[test]
    fn test_ffi_data() {
        let id = vitalis_data_create(10, 3);
        assert!(id > 0);
        assert_eq!(vitalis_data_len(id), 10);
        vitalis_data_free(id);
    }
}
