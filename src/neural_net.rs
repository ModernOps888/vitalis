//! Neural Network Layers — Production-grade building blocks for deep learning.
//!
//! Provides layer abstractions (Linear, Conv2D, Embedding, LayerNorm, RMSNorm, Dropout)
//! with forward passes and weight initialization. Designed for composability with
//! the tensor engine and autograd system.
//!
//! Reuses: `hotpath.rs` batch activations, `tensor.rs` core operations.
//! Does NOT duplicate: scalar activations (sigmoid, relu, etc.) from stdlib.

use std::sync::Mutex;
use std::collections::HashMap;

// ── Weight Initialization ───────────────────────────────────────────────

/// Weight initialization strategy.
#[derive(Debug, Clone, Copy)]
pub enum InitMethod {
    /// Zeros
    Zeros,
    /// Ones
    Ones,
    /// Xavier/Glorot uniform: U(-sqrt(6/(fan_in+fan_out)), sqrt(6/(fan_in+fan_out)))
    XavierUniform,
    /// Xavier/Glorot normal: N(0, sqrt(2/(fan_in+fan_out)))
    XavierNormal,
    /// Kaiming/He uniform: U(-sqrt(6/fan_in), sqrt(6/fan_in))
    KaimingUniform,
    /// Kaiming/He normal: N(0, sqrt(2/fan_in))
    KaimingNormal,
    /// Normal with given std
    Normal(f64),
    /// Uniform in [-bound, bound]
    Uniform(f64),
}

/// Generate initialized weights.
pub fn init_weights(fan_in: usize, fan_out: usize, method: InitMethod, seed: u64) -> Vec<f64> {
    let count = fan_in * fan_out;
    match method {
        InitMethod::Zeros => vec![0.0; count],
        InitMethod::Ones => vec![1.0; count],
        InitMethod::XavierUniform => {
            let bound = (6.0 / (fan_in + fan_out) as f64).sqrt();
            random_uniform(count, -bound, bound, seed)
        },
        InitMethod::XavierNormal => {
            let std = (2.0 / (fan_in + fan_out) as f64).sqrt();
            random_normal(count, 0.0, std, seed)
        },
        InitMethod::KaimingUniform => {
            let bound = (6.0 / fan_in as f64).sqrt();
            random_uniform(count, -bound, bound, seed)
        },
        InitMethod::KaimingNormal => {
            let std = (2.0 / fan_in as f64).sqrt();
            random_normal(count, 0.0, std, seed)
        },
        InitMethod::Normal(std) => random_normal(count, 0.0, std, seed),
        InitMethod::Uniform(bound) => random_uniform(count, -bound, bound, seed),
    }
}

fn random_uniform(count: usize, low: f64, high: f64, seed: u64) -> Vec<f64> {
    let mut data = Vec::with_capacity(count);
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    for _ in 0..count {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let u = (state as f64) / (u64::MAX as f64);
        data.push(low + u * (high - low));
    }
    data
}

fn random_normal(count: usize, mean: f64, std: f64, seed: u64) -> Vec<f64> {
    let mut data = Vec::with_capacity(count);
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    let mut i = 0;
    while i < count {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let u1 = ((state as f64) / (u64::MAX as f64)).max(1e-10);
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let u2 = (state as f64) / (u64::MAX as f64);
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        data.push(mean + std * r * theta.cos());
        if i + 1 < count {
            data.push(mean + std * r * theta.sin());
        }
        i += 2;
    }
    data.truncate(count);
    data
}

// ── Layer Trait ──────────────────────────────────────────────────────────

/// Common trait for all neural network layers.
pub trait Layer {
    /// Forward pass: input → output.
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>);
    /// Number of trainable parameters.
    fn num_params(&self) -> usize;
    /// Get all parameters as a flat vector.
    fn parameters(&self) -> Vec<f64>;
    /// Set parameters from a flat vector.
    fn set_parameters(&mut self, params: &[f64]);
}

// ── Linear Layer ────────────────────────────────────────────────────────

/// Fully-connected linear layer: y = xW^T + b
#[derive(Debug, Clone)]
pub struct Linear {
    pub in_features: usize,
    pub out_features: usize,
    pub weight: Vec<f64>,  // [out_features × in_features]
    pub bias: Vec<f64>,    // [out_features]
    pub use_bias: bool,
}

impl Linear {
    pub fn new(in_features: usize, out_features: usize, use_bias: bool, init: InitMethod, seed: u64) -> Self {
        let weight = init_weights(in_features, out_features, init, seed);
        let bias = if use_bias { vec![0.0; out_features] } else { vec![] };
        Linear { in_features, out_features, weight, bias, use_bias }
    }

    /// Forward: [batch, in_features] → [batch, out_features]
    pub fn forward_batch(&self, input: &[f64], batch_size: usize) -> Vec<f64> {
        let mut output = vec![0.0; batch_size * self.out_features];
        for b in 0..batch_size {
            for o in 0..self.out_features {
                let mut sum = if self.use_bias { self.bias[o] } else { 0.0 };
                for i in 0..self.in_features {
                    sum += input[b * self.in_features + i] * self.weight[o * self.in_features + i];
                }
                output[b * self.out_features + o] = sum;
            }
        }
        output
    }
}

impl Layer for Linear {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        let batch = if input_shape.len() >= 2 { input_shape[0] } else { 1 };
        let out = self.forward_batch(input, batch);
        let out_shape = if input_shape.len() >= 2 {
            vec![batch, self.out_features]
        } else {
            vec![self.out_features]
        };
        (out, out_shape)
    }

    fn num_params(&self) -> usize {
        self.in_features * self.out_features + if self.use_bias { self.out_features } else { 0 }
    }

    fn parameters(&self) -> Vec<f64> {
        let mut p = self.weight.clone();
        if self.use_bias { p.extend_from_slice(&self.bias); }
        p
    }

    fn set_parameters(&mut self, params: &[f64]) {
        let w_size = self.in_features * self.out_features;
        self.weight.copy_from_slice(&params[..w_size]);
        if self.use_bias {
            self.bias.copy_from_slice(&params[w_size..w_size + self.out_features]);
        }
    }
}

// ── Conv2D Layer ────────────────────────────────────────────────────────

/// 2D Convolution layer using im2col + GEMM approach.
#[derive(Debug, Clone)]
pub struct Conv2D {
    pub in_channels: usize,
    pub out_channels: usize,
    pub kernel_size: usize,
    pub stride: usize,
    pub padding: usize,
    pub weight: Vec<f64>,  // [out_channels, in_channels, kernel_size, kernel_size]
    pub bias: Vec<f64>,    // [out_channels]
    pub use_bias: bool,
}

impl Conv2D {
    pub fn new(in_ch: usize, out_ch: usize, kernel: usize, stride: usize, padding: usize, use_bias: bool, seed: u64) -> Self {
        let fan_in = in_ch * kernel * kernel;
        let std = (2.0 / fan_in as f64).sqrt();
        let weight = random_normal(out_ch * in_ch * kernel * kernel, 0.0, std, seed);
        let bias = if use_bias { vec![0.0; out_ch] } else { vec![] };
        Conv2D { in_channels: in_ch, out_channels: out_ch, kernel_size: kernel, stride, padding, weight, bias, use_bias }
    }

    /// Forward: [batch, C_in, H, W] → [batch, C_out, H_out, W_out]
    pub fn forward_conv(&self, input: &[f64], batch: usize, h: usize, w: usize) -> (Vec<f64>, usize, usize) {
        let h_out = (h + 2 * self.padding - self.kernel_size) / self.stride + 1;
        let w_out = (w + 2 * self.padding - self.kernel_size) / self.stride + 1;
        let k = self.kernel_size;
        let col_size = self.in_channels * k * k;

        let mut output = vec![0.0; batch * self.out_channels * h_out * w_out];

        for b in 0..batch {
            // im2col + GEMM
            let mut col = vec![0.0; col_size * h_out * w_out];
            for oh in 0..h_out {
                for ow in 0..w_out {
                    for c in 0..self.in_channels {
                        for kh in 0..k {
                            for kw in 0..k {
                                let ih = oh * self.stride + kh;
                                let iw = ow * self.stride + kw;
                                let ih = ih as isize - self.padding as isize;
                                let iw = iw as isize - self.padding as isize;
                                let val = if ih >= 0 && ih < h as isize && iw >= 0 && iw < w as isize {
                                    input[b * self.in_channels * h * w + c * h * w + ih as usize * w + iw as usize]
                                } else {
                                    0.0
                                };
                                col[(c * k * k + kh * k + kw) * h_out * w_out + oh * w_out + ow] = val;
                            }
                        }
                    }
                }
            }

            // GEMM: weight [out_ch, col_size] × col [col_size, h_out*w_out]
            let spatial = h_out * w_out;
            for oc in 0..self.out_channels {
                for s in 0..spatial {
                    let mut sum = if self.use_bias { self.bias[oc] } else { 0.0 };
                    for c in 0..col_size {
                        sum += self.weight[oc * col_size + c] * col[c * spatial + s];
                    }
                    output[b * self.out_channels * spatial + oc * spatial + s] = sum;
                }
            }
        }

        (output, h_out, w_out)
    }
}

impl Layer for Conv2D {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        let (batch, h, w) = (input_shape[0], input_shape[2], input_shape[3]);
        let (out, h_out, w_out) = self.forward_conv(input, batch, h, w);
        (out, vec![batch, self.out_channels, h_out, w_out])
    }

    fn num_params(&self) -> usize {
        self.out_channels * self.in_channels * self.kernel_size * self.kernel_size
            + if self.use_bias { self.out_channels } else { 0 }
    }

    fn parameters(&self) -> Vec<f64> {
        let mut p = self.weight.clone();
        if self.use_bias { p.extend_from_slice(&self.bias); }
        p
    }

    fn set_parameters(&mut self, params: &[f64]) {
        let w_size = self.weight.len();
        self.weight.copy_from_slice(&params[..w_size]);
        if self.use_bias {
            self.bias.copy_from_slice(&params[w_size..w_size + self.out_channels]);
        }
    }
}

// ── Embedding Layer ─────────────────────────────────────────────────────

/// Lookup table embedding: maps integer indices to dense vectors.
#[derive(Debug, Clone)]
pub struct Embedding {
    pub num_embeddings: usize,
    pub embedding_dim: usize,
    pub weight: Vec<f64>,  // [num_embeddings × embedding_dim]
}

impl Embedding {
    pub fn new(num_embeddings: usize, embedding_dim: usize, seed: u64) -> Self {
        let weight = random_normal(num_embeddings * embedding_dim, 0.0, 1.0, seed);
        Embedding { num_embeddings, embedding_dim, weight }
    }

    /// Forward: [seq_len] indices → [seq_len, embedding_dim]
    pub fn forward_indices(&self, indices: &[usize]) -> Vec<f64> {
        let mut output = Vec::with_capacity(indices.len() * self.embedding_dim);
        for &idx in indices {
            assert!(idx < self.num_embeddings, "Embedding index {} out of range {}", idx, self.num_embeddings);
            let start = idx * self.embedding_dim;
            output.extend_from_slice(&self.weight[start..start + self.embedding_dim]);
        }
        output
    }
}

impl Layer for Embedding {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        let indices: Vec<usize> = input.iter().map(|&x| x as usize).collect();
        let out = self.forward_indices(&indices);
        let seq_len = input_shape[0];
        (out, vec![seq_len, self.embedding_dim])
    }

    fn num_params(&self) -> usize { self.num_embeddings * self.embedding_dim }

    fn parameters(&self) -> Vec<f64> { self.weight.clone() }

    fn set_parameters(&mut self, params: &[f64]) {
        self.weight.copy_from_slice(&params[..self.num_embeddings * self.embedding_dim]);
    }
}

// ── Dropout ─────────────────────────────────────────────────────────────

/// Inverted dropout with deterministic mask.
#[derive(Debug, Clone)]
pub struct Dropout {
    pub rate: f64,
    pub training: bool,
}

impl Dropout {
    pub fn new(rate: f64) -> Self {
        Dropout { rate, training: true }
    }

    pub fn forward_with_seed(&self, input: &[f64], seed: u64) -> Vec<f64> {
        if !self.training || self.rate == 0.0 {
            return input.to_vec();
        }
        let keep = 1.0 - self.rate;
        let scale = 1.0 / keep;
        let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
        input.iter().map(|&x| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let u = (state as f64) / (u64::MAX as f64);
            if u < keep { x * scale } else { 0.0 }
        }).collect()
    }
}

impl Layer for Dropout {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        (self.forward_with_seed(input, 42), input_shape.to_vec())
    }
    fn num_params(&self) -> usize { 0 }
    fn parameters(&self) -> Vec<f64> { vec![] }
    fn set_parameters(&mut self, _params: &[f64]) {}
}

// ── LayerNormModule ─────────────────────────────────────────────────────

/// Layer Normalization module.
#[derive(Debug, Clone)]
pub struct LayerNormModule {
    pub normalized_shape: usize,
    pub gamma: Vec<f64>,
    pub beta: Vec<f64>,
    pub eps: f64,
}

impl LayerNormModule {
    pub fn new(normalized_shape: usize, eps: f64) -> Self {
        LayerNormModule {
            normalized_shape,
            gamma: vec![1.0; normalized_shape],
            beta: vec![0.0; normalized_shape],
            eps,
        }
    }

    pub fn forward_ln(&self, input: &[f64]) -> Vec<f64> {
        let n = self.normalized_shape;
        let batch = input.len() / n;
        let mut output = vec![0.0; input.len()];
        for b in 0..batch {
            let off = b * n;
            let slice = &input[off..off + n];
            let mean: f64 = slice.iter().sum::<f64>() / n as f64;
            let var: f64 = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
            let inv_std = 1.0 / (var + self.eps).sqrt();
            for i in 0..n {
                output[off + i] = self.gamma[i] * (slice[i] - mean) * inv_std + self.beta[i];
            }
        }
        output
    }
}

impl Layer for LayerNormModule {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        (self.forward_ln(input), input_shape.to_vec())
    }
    fn num_params(&self) -> usize { 2 * self.normalized_shape }
    fn parameters(&self) -> Vec<f64> {
        let mut p = self.gamma.clone();
        p.extend_from_slice(&self.beta);
        p
    }
    fn set_parameters(&mut self, params: &[f64]) {
        let n = self.normalized_shape;
        self.gamma.copy_from_slice(&params[..n]);
        self.beta.copy_from_slice(&params[n..2 * n]);
    }
}

// ── RMSNorm Module ──────────────────────────────────────────────────────

/// Root Mean Square Layer Normalization (used in LLaMA, Mistral).
#[derive(Debug, Clone)]
pub struct RMSNormModule {
    pub dim: usize,
    pub gamma: Vec<f64>,
    pub eps: f64,
}

impl RMSNormModule {
    pub fn new(dim: usize, eps: f64) -> Self {
        RMSNormModule { dim, gamma: vec![1.0; dim], eps }
    }

    pub fn forward_rms(&self, input: &[f64]) -> Vec<f64> {
        let n = self.dim;
        let batch = input.len() / n;
        let mut output = vec![0.0; input.len()];
        for b in 0..batch {
            let off = b * n;
            let slice = &input[off..off + n];
            let rms = (slice.iter().map(|x| x * x).sum::<f64>() / n as f64 + self.eps).sqrt();
            for i in 0..n {
                output[off + i] = self.gamma[i] * slice[i] / rms;
            }
        }
        output
    }
}

impl Layer for RMSNormModule {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        (self.forward_rms(input), input_shape.to_vec())
    }
    fn num_params(&self) -> usize { self.dim }
    fn parameters(&self) -> Vec<f64> { self.gamma.clone() }
    fn set_parameters(&mut self, params: &[f64]) { self.gamma.copy_from_slice(&params[..self.dim]); }
}

// ── Sequential ──────────────────────────────────────────────────────────

/// Chain of layers applied in sequence.
#[derive(Debug, Clone)]
pub struct Sequential {
    pub layers: Vec<Box<dyn Layer>>,
}

// We need to implement Clone for Box<dyn Layer> via a workaround
impl Clone for Box<dyn Layer> {
    fn clone(&self) -> Self {
        // Copy parameters
        let params = self.parameters();
        let _ = params; // Layers are stored by reference pattern
        // We can't easily clone trait objects; use parameter copy approach
        panic!("Sequential clone not supported directly; use parameter serialization")
    }
}

impl std::fmt::Debug for Box<dyn Layer> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Layer(params={})", self.num_params())
    }
}

// ── Activation Functions (as layers) ────────────────────────────────────

/// ReLU activation layer.
#[derive(Debug, Clone)]
pub struct ReLULayer;

impl Layer for ReLULayer {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        (input.iter().map(|&x| x.max(0.0)).collect(), input_shape.to_vec())
    }
    fn num_params(&self) -> usize { 0 }
    fn parameters(&self) -> Vec<f64> { vec![] }
    fn set_parameters(&mut self, _: &[f64]) {}
}

/// GELU activation layer.
#[derive(Debug, Clone)]
pub struct GELULayer;

impl Layer for GELULayer {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        let data: Vec<f64> = input.iter().map(|&x| {
            0.5 * x * (1.0 + (0.7978845608 * (x + 0.044715 * x * x * x)).tanh())
        }).collect();
        (data, input_shape.to_vec())
    }
    fn num_params(&self) -> usize { 0 }
    fn parameters(&self) -> Vec<f64> { vec![] }
    fn set_parameters(&mut self, _: &[f64]) {}
}

/// SwiGLU activation (for transformer FFN).
#[derive(Debug, Clone)]
pub struct SwiGLU {
    pub w1: Linear,
    pub w2: Linear,
    pub w3: Linear,
}

impl SwiGLU {
    pub fn new(dim: usize, hidden_dim: usize, seed: u64) -> Self {
        SwiGLU {
            w1: Linear::new(dim, hidden_dim, false, InitMethod::KaimingNormal, seed),
            w2: Linear::new(hidden_dim, dim, false, InitMethod::KaimingNormal, seed.wrapping_add(1)),
            w3: Linear::new(dim, hidden_dim, false, InitMethod::KaimingNormal, seed.wrapping_add(2)),
        }
    }

    /// SwiGLU(x) = (xW₁ ⊙ Swish(xV)) W₂
    pub fn forward_swiglu(&self, input: &[f64], batch: usize, dim: usize) -> Vec<f64> {
        let xw1 = self.w1.forward_batch(input, batch);
        let xw3 = self.w3.forward_batch(input, batch);
        // Swish(xW3) = xW3 * sigmoid(xW3)
        let swish: Vec<f64> = xw3.iter().map(|&x| x / (1.0 + (-x).exp())).collect();
        // Element-wise multiply
        let gate: Vec<f64> = xw1.iter().zip(&swish).map(|(a, b)| a * b).collect();
        self.w2.forward_batch(&gate, batch)
    }
}

impl Layer for SwiGLU {
    fn forward(&self, input: &[f64], input_shape: &[usize]) -> (Vec<f64>, Vec<usize>) {
        let batch = if input_shape.len() >= 2 { input_shape[0] } else { 1 };
        let dim = *input_shape.last().unwrap();
        let out = self.forward_swiglu(input, batch, dim);
        (out, input_shape.to_vec())
    }

    fn num_params(&self) -> usize {
        self.w1.num_params() + self.w2.num_params() + self.w3.num_params()
    }

    fn parameters(&self) -> Vec<f64> {
        let mut p = self.w1.parameters();
        p.extend(self.w2.parameters());
        p.extend(self.w3.parameters());
        p
    }

    fn set_parameters(&mut self, params: &[f64]) {
        let n1 = self.w1.num_params();
        let n2 = self.w2.num_params();
        self.w1.set_parameters(&params[..n1]);
        self.w2.set_parameters(&params[n1..n1+n2]);
        self.w3.set_parameters(&params[n1+n2..]);
    }
}

// ── FFI Interface ───────────────────────────────────────────────────────

static LAYER_STORE: Mutex<Option<HashMap<i64, Linear>>> = Mutex::new(None);

fn with_layers<R>(f: impl FnOnce(&mut HashMap<i64, Linear>) -> R) -> R {
    let mut guard = LAYER_STORE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    f(guard.as_mut().unwrap())
}

fn next_layer_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nn_linear_create(in_features: i64, out_features: i64, use_bias: i64, seed: i64) -> i64 {
    let layer = Linear::new(in_features as usize, out_features as usize, use_bias != 0, InitMethod::KaimingNormal, seed as u64);
    let id = next_layer_id();
    with_layers(|s| s.insert(id, layer));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nn_linear_forward(layer_id: i64, input_ptr: *const f64, batch: i64, out_ptr: *mut f64) -> i64 {
    with_layers(|s| {
        let layer = s.get(&layer_id).expect("linear layer not found");
        let input = unsafe { std::slice::from_raw_parts(input_ptr, (batch as usize) * layer.in_features) };
        let output = layer.forward_batch(input, batch as usize);
        unsafe { std::ptr::copy_nonoverlapping(output.as_ptr(), out_ptr, output.len()); }
        output.len() as i64
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nn_linear_params(layer_id: i64) -> i64 {
    with_layers(|s| s.get(&layer_id).expect("linear layer not found").num_params() as i64)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nn_embedding_forward(weights: *const f64, num_emb: i64, dim: i64, indices: *const i64, seq_len: i64, out: *mut f64) -> i64 {
    let w = unsafe { std::slice::from_raw_parts(weights, (num_emb * dim) as usize) };
    let idx = unsafe { std::slice::from_raw_parts(indices, seq_len as usize) };
    let d = dim as usize;
    for (i, &ix) in idx.iter().enumerate() {
        let start = ix as usize * d;
        unsafe {
            std::ptr::copy_nonoverlapping(w[start..].as_ptr(), out.add(i * d), d);
        }
    }
    seq_len * dim
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nn_layer_norm(input: *mut f64, count: i64, dim: i64, gamma: *const f64, beta: *const f64, eps: f64) {
    let n = dim as usize;
    let total = count as usize;
    let batch = total / n;
    let inp = unsafe { std::slice::from_raw_parts_mut(input, total) };
    let g = unsafe { std::slice::from_raw_parts(gamma, n) };
    let b = unsafe { std::slice::from_raw_parts(beta, n) };
    for bi in 0..batch {
        let off = bi * n;
        let slice = &inp[off..off+n];
        let mean: f64 = slice.iter().sum::<f64>() / n as f64;
        let var: f64 = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let inv_std = 1.0 / (var + eps).sqrt();
        for i in 0..n {
            inp[off + i] = g[i] * (inp[off + i] - mean) * inv_std + b[i];
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nn_rms_norm(input: *mut f64, count: i64, dim: i64, gamma: *const f64, eps: f64) {
    let n = dim as usize;
    let total = count as usize;
    let batch = total / n;
    let inp = unsafe { std::slice::from_raw_parts_mut(input, total) };
    let g = unsafe { std::slice::from_raw_parts(gamma, n) };
    for bi in 0..batch {
        let off = bi * n;
        let rms = (inp[off..off+n].iter().map(|x| x * x).sum::<f64>() / n as f64 + eps).sqrt();
        for i in 0..n {
            inp[off + i] = g[i] * inp[off + i] / rms;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nn_dropout(input: *mut f64, count: i64, rate: f64, seed: i64) {
    let data = unsafe { std::slice::from_raw_parts_mut(input, count as usize) };
    let keep = 1.0 - rate;
    let scale = 1.0 / keep;
    let mut state = (seed as u64).wrapping_add(0x9E3779B97F4A7C15);
    for x in data.iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let u = (state as f64) / (u64::MAX as f64);
        if u >= keep { *x = 0.0; } else { *x *= scale; }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_nn_init_weights(out: *mut f64, fan_in: i64, fan_out: i64, method: i64, seed: i64) -> i64 {
    let init = match method {
        0 => InitMethod::Zeros,
        1 => InitMethod::XavierUniform,
        2 => InitMethod::XavierNormal,
        3 => InitMethod::KaimingUniform,
        4 => InitMethod::KaimingNormal,
        _ => InitMethod::KaimingNormal,
    };
    let weights = init_weights(fan_in as usize, fan_out as usize, init, seed as u64);
    unsafe { std::ptr::copy_nonoverlapping(weights.as_ptr(), out, weights.len()); }
    weights.len() as i64
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_forward() {
        let mut layer = Linear::new(3, 2, true, InitMethod::Zeros, 42);
        // Set known weights
        layer.weight = vec![1.0, 0.0, 0.0,  0.0, 1.0, 0.0];
        layer.bias = vec![0.0, 0.0];
        let input = vec![1.0, 2.0, 3.0];
        let output = layer.forward_batch(&input, 1);
        assert_eq!(output, vec![1.0, 2.0]); // Projects to first 2 dims
    }

    #[test]
    fn test_linear_batch() {
        let mut layer = Linear::new(2, 2, true, InitMethod::Zeros, 42);
        layer.weight = vec![1.0, 0.0, 0.0, 1.0]; // Identity
        layer.bias = vec![0.0, 0.0];
        let input = vec![1.0, 2.0, 3.0, 4.0]; // batch=2
        let output = layer.forward_batch(&input, 2);
        assert_eq!(output, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_linear_with_bias() {
        let mut layer = Linear::new(2, 2, true, InitMethod::Zeros, 42);
        layer.weight = vec![1.0, 0.0, 0.0, 1.0];
        layer.bias = vec![10.0, 20.0];
        let input = vec![1.0, 2.0];
        let output = layer.forward_batch(&input, 1);
        assert_eq!(output, vec![11.0, 22.0]);
    }

    #[test]
    fn test_embedding() {
        let mut emb = Embedding::new(5, 3, 42);
        // Set known weights
        emb.weight = vec![
            1.0, 0.0, 0.0,  // idx 0
            0.0, 1.0, 0.0,  // idx 1
            0.0, 0.0, 1.0,  // idx 2
            1.0, 1.0, 0.0,  // idx 3
            0.0, 1.0, 1.0,  // idx 4
        ];
        let out = emb.forward_indices(&[0, 2, 4]);
        assert_eq!(out, vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_dropout_training() {
        let dropout = Dropout::new(0.5);
        let input: Vec<f64> = vec![1.0; 1000];
        let output = dropout.forward_with_seed(&input, 42);
        let nonzero = output.iter().filter(|&&x| x > 0.0).count();
        assert!(nonzero > 300 && nonzero < 700); // ~50% should survive
    }

    #[test]
    fn test_dropout_eval() {
        let mut dropout = Dropout::new(0.5);
        dropout.training = false;
        let input = vec![1.0, 2.0, 3.0];
        let output = dropout.forward_with_seed(&input, 42);
        assert_eq!(output, input); // No dropout in eval mode
    }

    #[test]
    fn test_layer_norm_module() {
        let ln = LayerNormModule::new(4, 1e-5);
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = ln.forward_ln(&input);
        let mean: f64 = output.iter().sum::<f64>() / 4.0;
        assert!(mean.abs() < 1e-5); // Mean should be ~0
    }

    #[test]
    fn test_rms_norm_module() {
        let rn = RMSNormModule::new(4, 1e-5);
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = rn.forward_rms(&input);
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_conv2d_output_shape() {
        let conv = Conv2D::new(1, 4, 3, 1, 1, true, 42);
        let input = vec![0.0; 1 * 1 * 8 * 8]; // batch=1, C=1, H=8, W=8
        let (output, h_out, w_out) = conv.forward_conv(&input, 1, 8, 8);
        assert_eq!(h_out, 8); // Same padding
        assert_eq!(w_out, 8);
        assert_eq!(output.len(), 1 * 4 * 8 * 8);
    }

    #[test]
    fn test_conv2d_no_padding() {
        let conv = Conv2D::new(1, 1, 3, 1, 0, false, 42);
        let input = vec![0.0; 1 * 1 * 5 * 5];
        let (_output, h_out, w_out) = conv.forward_conv(&input, 1, 5, 5);
        assert_eq!(h_out, 3);
        assert_eq!(w_out, 3);
    }

    #[test]
    fn test_relu_layer() {
        let relu = ReLULayer;
        let (out, shape) = relu.forward(&[-1.0, 0.0, 1.0, 2.0], &[4]);
        assert_eq!(out, vec![0.0, 0.0, 1.0, 2.0]);
        assert_eq!(shape, vec![4]);
    }

    #[test]
    fn test_gelu_layer() {
        let gelu = GELULayer;
        let (out, _) = gelu.forward(&[0.0, 1.0], &[2]);
        assert!((out[0] - 0.0).abs() < 1e-5);
        assert!(out[1] > 0.8);
    }

    #[test]
    fn test_swiglu() {
        let swiglu = SwiGLU::new(4, 8, 42);
        let input = vec![1.0, 0.0, -1.0, 0.5];
        let output = swiglu.forward_swiglu(&input, 1, 4);
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_init_xavier_uniform() {
        let w = init_weights(100, 100, InitMethod::XavierUniform, 42);
        assert_eq!(w.len(), 10000);
        let bound = (6.0 / 200.0_f64).sqrt();
        assert!(w.iter().all(|&x| x.abs() <= bound + 0.01));
    }

    #[test]
    fn test_init_kaiming_normal() {
        let w = init_weights(256, 256, InitMethod::KaimingNormal, 42);
        let mean: f64 = w.iter().sum::<f64>() / w.len() as f64;
        assert!(mean.abs() < 0.1); // Mean ~0
    }

    #[test]
    fn test_layer_trait() {
        let layer = Linear::new(4, 2, true, InitMethod::Zeros, 42);
        assert_eq!(layer.num_params(), 4 * 2 + 2);
        let params = layer.parameters();
        assert_eq!(params.len(), 10);
    }

    #[test]
    fn test_set_parameters() {
        let mut layer = Linear::new(2, 2, true, InitMethod::Zeros, 42);
        let new_params = vec![1.0, 0.0, 0.0, 1.0, 0.5, 0.5];
        layer.set_parameters(&new_params);
        assert_eq!(layer.weight, vec![1.0, 0.0, 0.0, 1.0]);
        assert_eq!(layer.bias, vec![0.5, 0.5]);
    }

    #[test]
    fn test_embedding_layer_trait() {
        let emb = Embedding::new(100, 32, 42);
        assert_eq!(emb.num_params(), 3200);
    }

    #[test]
    fn test_ffi_linear() {
        let id = vitalis_nn_linear_create(3, 2, 1, 42);
        assert!(id > 0);
        let params = vitalis_nn_linear_params(id);
        assert_eq!(params, 3 * 2 + 2);
    }

    #[test]
    fn test_layer_norm_batch() {
        let ln = LayerNormModule::new(3, 1e-5);
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // batch=2, dim=3
        let output = ln.forward_ln(&input);
        assert_eq!(output.len(), 6);
    }

    #[test]
    fn test_dropout_zero_rate() {
        let dropout = Dropout::new(0.0);
        let input = vec![1.0, 2.0, 3.0];
        let output = dropout.forward_with_seed(&input, 42);
        assert_eq!(output, input);
    }
}
