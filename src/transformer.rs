//! Transformer — Attention mechanisms, positional encodings, and transformer blocks.
//!
//! Provides multi-head attention (MHA), grouped-query attention (GQA),
//! RoPE/ALiBi positional encodings, FlashAttention-style tiling,
//! SwiGLU FFN, pre-norm transformer blocks, and KV-cache for inference.

use std::sync::Mutex;
use std::collections::HashMap;
use crate::tensor::Tensor;

// ── Positional Encodings ────────────────────────────────────────────────

/// Sinusoidal positional encoding (Vaswani et al.).
pub fn sinusoidal_pe(seq_len: usize, d_model: usize) -> Tensor {
    let mut data = vec![0.0; seq_len * d_model];
    for pos in 0..seq_len {
        for i in 0..d_model / 2 {
            let angle = pos as f64 / (10000.0_f64).powf(2.0 * i as f64 / d_model as f64);
            data[pos * d_model + 2 * i] = angle.sin();
            data[pos * d_model + 2 * i + 1] = angle.cos();
        }
    }
    Tensor::from_data(data, &[seq_len, d_model])
}

/// Rotary Position Embedding (RoPE) frequencies.
pub fn rope_frequencies(head_dim: usize, seq_len: usize, theta: f64) -> Vec<(f64, f64)> {
    let mut freqs = Vec::with_capacity(seq_len * (head_dim / 2));
    for pos in 0..seq_len {
        for i in 0..head_dim / 2 {
            let freq = pos as f64 / theta.powf(2.0 * i as f64 / head_dim as f64);
            freqs.push((freq.cos(), freq.sin()));
        }
    }
    freqs
}

/// Apply RoPE to query/key vectors.
pub fn apply_rope(data: &mut [f64], head_dim: usize, seq_len: usize, theta: f64) {
    let freqs = rope_frequencies(head_dim, seq_len, theta);
    let num_heads = data.len() / (seq_len * head_dim);
    for h in 0..num_heads {
        for pos in 0..seq_len {
            for i in 0..head_dim / 2 {
                let idx = h * seq_len * head_dim + pos * head_dim;
                let (cos, sin) = freqs[pos * (head_dim / 2) + i];
                let x0 = data[idx + 2 * i];
                let x1 = data[idx + 2 * i + 1];
                data[idx + 2 * i] = x0 * cos - x1 * sin;
                data[idx + 2 * i + 1] = x0 * sin + x1 * cos;
            }
        }
    }
}

/// ALiBi (Attention with Linear Biases) slopes.
pub fn alibi_slopes(num_heads: usize) -> Vec<f64> {
    let ratio = 2.0_f64.powf(-(8.0 / num_heads as f64));
    (0..num_heads).map(|h| ratio.powi((h + 1) as i32)).collect()
}

/// Compute ALiBi bias matrix for attention scores.
pub fn alibi_bias(seq_len: usize, slope: f64) -> Vec<f64> {
    let mut bias = vec![0.0; seq_len * seq_len];
    for i in 0..seq_len {
        for j in 0..seq_len {
            bias[i * seq_len + j] = slope * (j as f64 - i as f64);
        }
    }
    bias
}

// ── Attention ───────────────────────────────────────────────────────────

/// Scaled dot-product attention.
/// scores = softmax((Q @ K^T) / sqrt(d_k) + mask) @ V
pub fn scaled_dot_product_attention(
    query: &[f64],   // [seq_q, d_k]
    key: &[f64],     // [seq_k, d_k]
    value: &[f64],   // [seq_k, d_v]
    seq_q: usize,
    seq_k: usize,
    d_k: usize,
    d_v: usize,
    mask: Option<&[f64]>, // [seq_q, seq_k] or None
) -> Vec<f64> {
    let scale = 1.0 / (d_k as f64).sqrt();

    // Q @ K^T → [seq_q, seq_k]
    let mut scores = vec![0.0; seq_q * seq_k];
    for i in 0..seq_q {
        for j in 0..seq_k {
            let mut dot = 0.0;
            for k in 0..d_k {
                dot += query[i * d_k + k] * key[j * d_k + k];
            }
            scores[i * seq_k + j] = dot * scale;
        }
    }

    // Apply mask
    if let Some(m) = mask {
        for i in 0..scores.len() {
            if m[i] == 0.0 {
                scores[i] = f64::NEG_INFINITY;
            }
        }
    }

    // Row-wise softmax
    for i in 0..seq_q {
        let row = &mut scores[i * seq_k..(i + 1) * seq_k];
        let max = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mut sum = 0.0;
        for v in row.iter_mut() {
            *v = (*v - max).exp();
            sum += *v;
        }
        if sum > 0.0 {
            for v in row.iter_mut() {
                *v /= sum;
            }
        }
    }

    // scores @ V → [seq_q, d_v]
    let mut output = vec![0.0; seq_q * d_v];
    for i in 0..seq_q {
        for j in 0..d_v {
            let mut sum = 0.0;
            for k in 0..seq_k {
                sum += scores[i * seq_k + k] * value[k * d_v + j];
            }
            output[i * d_v + j] = sum;
        }
    }
    output
}

/// Causal mask (lower triangular).
pub fn causal_mask(seq_len: usize) -> Vec<f64> {
    let mut mask = vec![0.0; seq_len * seq_len];
    for i in 0..seq_len {
        for j in 0..=i {
            mask[i * seq_len + j] = 1.0;
        }
    }
    mask
}

/// Multi-Head Attention (MHA).
#[derive(Debug, Clone)]
pub struct MultiHeadAttention {
    pub d_model: usize,
    pub num_heads: usize,
    pub head_dim: usize,
    pub w_q: Vec<f64>, // [d_model, d_model]
    pub w_k: Vec<f64>,
    pub w_v: Vec<f64>,
    pub w_o: Vec<f64>, // [d_model, d_model]
    pub use_rope: bool,
    pub rope_theta: f64,
}

impl MultiHeadAttention {
    pub fn new(d_model: usize, num_heads: usize, use_rope: bool) -> Self {
        let head_dim = d_model / num_heads;
        let scale = (2.0 / (d_model + d_model) as f64).sqrt(); // Xavier
        let mut state: u64 = 0xDEADBEEF;
        let mut rng = || -> f64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as f64 / u64::MAX as f64) * 2.0 * scale - scale
        };
        let n = d_model * d_model;
        MultiHeadAttention {
            d_model, num_heads, head_dim,
            w_q: (0..n).map(|_| rng()).collect(),
            w_k: (0..n).map(|_| rng()).collect(),
            w_v: (0..n).map(|_| rng()).collect(),
            w_o: (0..n).map(|_| rng()).collect(),
            use_rope,
            rope_theta: 10000.0,
        }
    }

    /// Forward pass: input [seq_len, d_model] → output [seq_len, d_model].
    pub fn forward(&self, input: &[f64], seq_len: usize, mask: Option<&[f64]>) -> Vec<f64> {
        let d = self.d_model;

        // Project Q, K, V
        let mut q = mat_vec_batch(input, &self.w_q, seq_len, d, d);
        let mut k = mat_vec_batch(input, &self.w_k, seq_len, d, d);
        let v = mat_vec_batch(input, &self.w_v, seq_len, d, d);

        // Apply RoPE
        if self.use_rope {
            apply_rope(&mut q, self.head_dim, seq_len, self.rope_theta);
            apply_rope(&mut k, self.head_dim, seq_len, self.rope_theta);
        }

        // Multi-head attention
        let mut all_heads = vec![0.0; seq_len * d];
        for h in 0..self.num_heads {
            // Extract head slices
            let mut q_h = vec![0.0; seq_len * self.head_dim];
            let mut k_h = vec![0.0; seq_len * self.head_dim];
            let mut v_h = vec![0.0; seq_len * self.head_dim];
            for s in 0..seq_len {
                for i in 0..self.head_dim {
                    q_h[s * self.head_dim + i] = q[s * d + h * self.head_dim + i];
                    k_h[s * self.head_dim + i] = k[s * d + h * self.head_dim + i];
                    v_h[s * self.head_dim + i] = v[s * d + h * self.head_dim + i];
                }
            }

            let attn_out = scaled_dot_product_attention(
                &q_h, &k_h, &v_h, seq_len, seq_len, self.head_dim, self.head_dim, mask,
            );

            // Write back to concat buffer
            for s in 0..seq_len {
                for i in 0..self.head_dim {
                    all_heads[s * d + h * self.head_dim + i] = attn_out[s * self.head_dim + i];
                }
            }
        }

        // Output projection
        mat_vec_batch(&all_heads, &self.w_o, seq_len, d, d)
    }

    pub fn num_params(&self) -> usize {
        4 * self.d_model * self.d_model
    }
}

/// Grouped-Query Attention (GQA).
#[derive(Debug, Clone)]
pub struct GroupedQueryAttention {
    pub d_model: usize,
    pub num_q_heads: usize,
    pub num_kv_heads: usize,
    pub head_dim: usize,
    pub w_q: Vec<f64>,  // [d_model, num_q_heads * head_dim]
    pub w_k: Vec<f64>,  // [d_model, num_kv_heads * head_dim]
    pub w_v: Vec<f64>,
    pub w_o: Vec<f64>,  // [num_q_heads * head_dim, d_model]
    pub rope_theta: f64,
}

impl GroupedQueryAttention {
    pub fn new(d_model: usize, num_q_heads: usize, num_kv_heads: usize) -> Self {
        let head_dim = d_model / num_q_heads;
        let scale = (2.0 / d_model as f64).sqrt();
        let mut state: u64 = 0xCAFEBABE;
        let mut rng = || -> f64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as f64 / u64::MAX as f64) * 2.0 * scale - scale
        };
        GroupedQueryAttention {
            d_model, num_q_heads, num_kv_heads, head_dim,
            w_q: (0..d_model * num_q_heads * head_dim).map(|_| rng()).collect(),
            w_k: (0..d_model * num_kv_heads * head_dim).map(|_| rng()).collect(),
            w_v: (0..d_model * num_kv_heads * head_dim).map(|_| rng()).collect(),
            w_o: (0..num_q_heads * head_dim * d_model).map(|_| rng()).collect(),
            rope_theta: 10000.0,
        }
    }

    /// Forward with GQA: each KV head is shared across `num_q_heads/num_kv_heads` query heads.
    pub fn forward(&self, input: &[f64], seq_len: usize, mask: Option<&[f64]>) -> Vec<f64> {
        let q_dim = self.num_q_heads * self.head_dim;
        let kv_dim = self.num_kv_heads * self.head_dim;

        let mut q = mat_vec_batch(input, &self.w_q, seq_len, self.d_model, q_dim);
        let mut k = mat_vec_batch(input, &self.w_k, seq_len, self.d_model, kv_dim);
        let v = mat_vec_batch(input, &self.w_v, seq_len, self.d_model, kv_dim);

        apply_rope(&mut q, self.head_dim, seq_len, self.rope_theta);
        apply_rope(&mut k, self.head_dim, seq_len, self.rope_theta);

        let heads_per_kv = self.num_q_heads / self.num_kv_heads;
        let mut all_heads = vec![0.0; seq_len * q_dim];

        for qh in 0..self.num_q_heads {
            let kvh = qh / heads_per_kv;
            let mut q_h = vec![0.0; seq_len * self.head_dim];
            let mut k_h = vec![0.0; seq_len * self.head_dim];
            let mut v_h = vec![0.0; seq_len * self.head_dim];

            for s in 0..seq_len {
                for i in 0..self.head_dim {
                    q_h[s * self.head_dim + i] = q[s * q_dim + qh * self.head_dim + i];
                    k_h[s * self.head_dim + i] = k[s * kv_dim + kvh * self.head_dim + i];
                    v_h[s * self.head_dim + i] = v[s * kv_dim + kvh * self.head_dim + i];
                }
            }

            let attn_out = scaled_dot_product_attention(
                &q_h, &k_h, &v_h, seq_len, seq_len, self.head_dim, self.head_dim, mask,
            );

            for s in 0..seq_len {
                for i in 0..self.head_dim {
                    all_heads[s * q_dim + qh * self.head_dim + i] = attn_out[s * self.head_dim + i];
                }
            }
        }

        mat_vec_batch(&all_heads, &self.w_o, seq_len, q_dim, self.d_model)
    }
}

// ── Flash Attention (tiled, memory-efficient) ───────────────────────────

/// Flash Attention approximation using tiling for memory efficiency.
/// Processes attention in blocks to avoid materializing full N×N attention matrix.
pub fn flash_attention_tiled(
    query: &[f64],   // [seq_q, d_k]
    key: &[f64],     // [seq_k, d_k]
    value: &[f64],   // [seq_k, d_v]
    seq_q: usize,
    seq_k: usize,
    d_k: usize,
    d_v: usize,
    block_size: usize,
) -> Vec<f64> {
    let scale = 1.0 / (d_k as f64).sqrt();
    let mut output = vec![0.0; seq_q * d_v];
    let mut row_max = vec![f64::NEG_INFINITY; seq_q];
    let mut row_sum = vec![0.0; seq_q];

    // Process key/value blocks
    for kb_start in (0..seq_k).step_by(block_size) {
        let kb_end = (kb_start + block_size).min(seq_k);

        for qi in 0..seq_q {
            let mut block_max = f64::NEG_INFINITY;
            let mut block_scores = Vec::with_capacity(kb_end - kb_start);

            // Compute attention scores for this block
            for kj in kb_start..kb_end {
                let mut dot = 0.0;
                for d in 0..d_k {
                    dot += query[qi * d_k + d] * key[kj * d_k + d];
                }
                let score = dot * scale;
                block_max = block_max.max(score);
                block_scores.push(score);
            }

            // Online softmax update (Milakov & Gimelshein, 2018)
            let old_max = row_max[qi];
            let new_max = old_max.max(block_max);

            // Rescale previous output
            let scale_old = (old_max - new_max).exp();
            for d in 0..d_v {
                output[qi * d_v + d] *= scale_old;
            }
            row_sum[qi] *= scale_old;

            // Accumulate block contribution
            for (j, &score) in block_scores.iter().enumerate() {
                let kj = kb_start + j;
                let w = (score - new_max).exp();
                for d in 0..d_v {
                    output[qi * d_v + d] += w * value[kj * d_v + d];
                }
                row_sum[qi] += w;
            }
            row_max[qi] = new_max;
        }
    }

    // Normalize
    for qi in 0..seq_q {
        if row_sum[qi] > 0.0 {
            for d in 0..d_v {
                output[qi * d_v + d] /= row_sum[qi];
            }
        }
    }

    output
}

// ── KV Cache ────────────────────────────────────────────────────────────

/// KV Cache for autoregressive inference.
#[derive(Debug, Clone)]
pub struct KVCache {
    pub k_cache: Vec<f64>, // [cached_len, d_k]
    pub v_cache: Vec<f64>, // [cached_len, d_v]
    pub d_k: usize,
    pub d_v: usize,
    pub cached_len: usize,
}

impl KVCache {
    pub fn new(d_k: usize, d_v: usize) -> Self {
        KVCache { k_cache: Vec::new(), v_cache: Vec::new(), d_k, d_v, cached_len: 0 }
    }

    /// Append new key/value entries to cache.
    pub fn append(&mut self, new_k: &[f64], new_v: &[f64], new_len: usize) {
        self.k_cache.extend_from_slice(&new_k[..new_len * self.d_k]);
        self.v_cache.extend_from_slice(&new_v[..new_len * self.d_v]);
        self.cached_len += new_len;
    }

    /// Get all cached keys.
    pub fn keys(&self) -> &[f64] {
        &self.k_cache
    }

    /// Get all cached values.
    pub fn values(&self) -> &[f64] {
        &self.v_cache
    }

    pub fn clear(&mut self) {
        self.k_cache.clear();
        self.v_cache.clear();
        self.cached_len = 0;
    }
}

// ── Transformer Block ───────────────────────────────────────────────────

/// Feed-forward network with SwiGLU activation.
#[derive(Debug, Clone)]
pub struct SwiGLUFFN {
    pub d_model: usize,
    pub d_ff: usize,
    pub w_gate: Vec<f64>,   // [d_model, d_ff]
    pub w_up: Vec<f64>,     // [d_model, d_ff]
    pub w_down: Vec<f64>,   // [d_ff, d_model]
}

impl SwiGLUFFN {
    pub fn new(d_model: usize, d_ff: usize) -> Self {
        let scale = (2.0 / d_model as f64).sqrt();
        let mut state: u64 = 0xBEEFCAFE;
        let mut rng = || -> f64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as f64 / u64::MAX as f64) * 2.0 * scale - scale
        };
        SwiGLUFFN {
            d_model, d_ff,
            w_gate: (0..d_model * d_ff).map(|_| rng()).collect(),
            w_up: (0..d_model * d_ff).map(|_| rng()).collect(),
            w_down: (0..d_ff * d_model).map(|_| rng()).collect(),
        }
    }

    pub fn forward(&self, input: &[f64], seq_len: usize) -> Vec<f64> {
        let gate = mat_vec_batch(input, &self.w_gate, seq_len, self.d_model, self.d_ff);
        let up = mat_vec_batch(input, &self.w_up, seq_len, self.d_model, self.d_ff);

        // SwiGLU: gate * silu(gate_proj) * up_proj
        let mut hidden = vec![0.0; seq_len * self.d_ff];
        for i in 0..seq_len * self.d_ff {
            let silu = gate[i] * (1.0 / (1.0 + (-gate[i]).exp())); // silu = x * sigmoid(x)
            hidden[i] = silu * up[i];
        }

        mat_vec_batch(&hidden, &self.w_down, seq_len, self.d_ff, self.d_model)
    }
}

/// RMS Normalization.
pub fn rms_norm(data: &[f64], weight: &[f64], d: usize, eps: f64) -> Vec<f64> {
    let seq_len = data.len() / d;
    let mut output = vec![0.0; data.len()];
    for s in 0..seq_len {
        let row = &data[s * d..(s + 1) * d];
        let rms = (row.iter().map(|x| x * x).sum::<f64>() / d as f64 + eps).sqrt();
        for i in 0..d {
            output[s * d + i] = row[i] / rms * weight[i];
        }
    }
    output
}

/// Pre-Norm Transformer Block (RMSNorm → Attn → Residual → RMSNorm → FFN → Residual).
#[derive(Debug, Clone)]
pub struct TransformerBlock {
    pub attn: MultiHeadAttention,
    pub ffn: SwiGLUFFN,
    pub norm1_weight: Vec<f64>,
    pub norm2_weight: Vec<f64>,
    pub d_model: usize,
    pub eps: f64,
}

impl TransformerBlock {
    pub fn new(d_model: usize, num_heads: usize, d_ff: usize, use_rope: bool) -> Self {
        TransformerBlock {
            attn: MultiHeadAttention::new(d_model, num_heads, use_rope),
            ffn: SwiGLUFFN::new(d_model, d_ff),
            norm1_weight: vec![1.0; d_model],
            norm2_weight: vec![1.0; d_model],
            d_model,
            eps: 1e-6,
        }
    }

    pub fn forward(&self, input: &[f64], seq_len: usize, mask: Option<&[f64]>) -> Vec<f64> {
        // Pre-norm attention
        let normed = rms_norm(input, &self.norm1_weight, self.d_model, self.eps);
        let attn_out = self.attn.forward(&normed, seq_len, mask);

        // Residual
        let mut residual: Vec<f64> = input.iter().zip(attn_out.iter()).map(|(a, b)| a + b).collect();

        // Pre-norm FFN
        let normed2 = rms_norm(&residual, &self.norm2_weight, self.d_model, self.eps);
        let ffn_out = self.ffn.forward(&normed2, seq_len);

        // Residual
        for (r, f) in residual.iter_mut().zip(ffn_out.iter()) {
            *r += f;
        }
        residual
    }

    pub fn num_params(&self) -> usize {
        self.attn.num_params()
            + self.d_model * self.ffn.d_ff * 3 // SwiGLU: gate, up, down
            + self.d_model * 2 // norms
    }
}

/// Full Transformer model (stack of blocks).
#[derive(Debug, Clone)]
pub struct TransformerModel {
    pub blocks: Vec<TransformerBlock>,
    pub d_model: usize,
    pub num_layers: usize,
}

impl TransformerModel {
    pub fn new(d_model: usize, num_heads: usize, num_layers: usize, d_ff: usize, use_rope: bool) -> Self {
        TransformerModel {
            blocks: (0..num_layers).map(|_| TransformerBlock::new(d_model, num_heads, d_ff, use_rope)).collect(),
            d_model,
            num_layers,
        }
    }

    pub fn forward(&self, input: &[f64], seq_len: usize, mask: Option<&[f64]>) -> Vec<f64> {
        let mut hidden = input.to_vec();
        for block in &self.blocks {
            hidden = block.forward(&hidden, seq_len, mask);
        }
        hidden
    }

    pub fn total_params(&self) -> usize {
        self.blocks.iter().map(|b| b.num_params()).sum()
    }
}

// ── Helper: batched matrix multiply ─────────────────────────────────────

/// Matrix multiply: input [rows, cols_in] × weight [cols_in, cols_out] → [rows, cols_out]
fn mat_vec_batch(input: &[f64], weight: &[f64], rows: usize, cols_in: usize, cols_out: usize) -> Vec<f64> {
    let mut output = vec![0.0; rows * cols_out];
    for r in 0..rows {
        for co in 0..cols_out {
            let mut sum = 0.0;
            for ci in 0..cols_in {
                sum += input[r * cols_in + ci] * weight[ci * cols_out + co];
            }
            output[r * cols_out + co] = sum;
        }
    }
    output
}

// ── FFI Interface ───────────────────────────────────────────────────────

static TRANSFORMER_STORE: Mutex<Option<HashMap<i64, TransformerModel>>> = Mutex::new(None);
static KV_CACHE_STORE: Mutex<Option<HashMap<i64, Vec<KVCache>>>> = Mutex::new(None);

fn with_transformers<R>(f: impl FnOnce(&mut HashMap<i64, TransformerModel>) -> R) -> R {
    let mut guard = TRANSFORMER_STORE.lock().unwrap();
    if guard.is_none() { *guard = Some(HashMap::new()); }
    f(guard.as_mut().unwrap())
}

fn next_transformer_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_transformer_create(d_model: i64, num_heads: i64, num_layers: i64, d_ff: i64, use_rope: i64) -> i64 {
    let model = TransformerModel::new(
        d_model as usize, num_heads as usize, num_layers as usize, d_ff as usize, use_rope != 0,
    );
    let id = next_transformer_id();
    with_transformers(|s| s.insert(id, model));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_transformer_params(model_id: i64) -> i64 {
    with_transformers(|s| s.get(&model_id).map_or(0, |m| m.total_params() as i64))
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_attention_sdpa(
    query: *const f64, key: *const f64, value: *const f64,
    seq_q: i64, seq_k: i64, d_k: i64, d_v: i64,
    output: *mut f64, causal: i64,
) {
    let sq = seq_q as usize;
    let sk = seq_k as usize;
    let dk = d_k as usize;
    let dv = d_v as usize;
    let q = unsafe { std::slice::from_raw_parts(query, sq * dk) };
    let k = unsafe { std::slice::from_raw_parts(key, sk * dk) };
    let v = unsafe { std::slice::from_raw_parts(value, sk * dv) };
    let mask = if causal != 0 { Some(causal_mask(sq.max(sk))) } else { None };
    let result = scaled_dot_product_attention(q, k, v, sq, sk, dk, dv, mask.as_deref());
    unsafe { std::ptr::copy_nonoverlapping(result.as_ptr(), output, result.len()); }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_flash_attention(
    query: *const f64, key: *const f64, value: *const f64,
    seq_q: i64, seq_k: i64, d_k: i64, d_v: i64,
    output: *mut f64, block_size: i64,
) {
    let sq = seq_q as usize;
    let sk = seq_k as usize;
    let dk = d_k as usize;
    let dv = d_v as usize;
    let q = unsafe { std::slice::from_raw_parts(query, sq * dk) };
    let k = unsafe { std::slice::from_raw_parts(key, sk * dk) };
    let v = unsafe { std::slice::from_raw_parts(value, sk * dv) };
    let result = flash_attention_tiled(q, k, v, sq, sk, dk, dv, block_size as usize);
    unsafe { std::ptr::copy_nonoverlapping(result.as_ptr(), output, result.len()); }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_rope_apply(data: *mut f64, count: i64, head_dim: i64, seq_len: i64) {
    let d = unsafe { std::slice::from_raw_parts_mut(data, count as usize) };
    apply_rope(d, head_dim as usize, seq_len as usize, 10000.0);
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sinusoidal_pe() {
        let pe = sinusoidal_pe(8, 16);
        assert_eq!(pe.shape, vec![8, 16]);
        // Position 0 should have sin(0)=0
        assert!((pe.data[0] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_rope_frequencies() {
        let freqs = rope_frequencies(8, 4, 10000.0);
        assert_eq!(freqs.len(), 4 * 4); // seq_len * head_dim/2
    }

    #[test]
    fn test_alibi_slopes() {
        let slopes = alibi_slopes(8);
        assert_eq!(slopes.len(), 8);
        // Each slope should be smaller than previous
        for i in 1..slopes.len() {
            assert!(slopes[i] < slopes[i - 1]);
        }
    }

    #[test]
    fn test_causal_mask() {
        let mask = causal_mask(4);
        assert_eq!(mask.len(), 16);
        // Upper-right triangle should be 0
        assert_eq!(mask[0 * 4 + 3], 0.0); // row 0, col 3
        assert_eq!(mask[3 * 4 + 3], 1.0); // row 3, col 3
        assert_eq!(mask[3 * 4 + 0], 1.0); // row 3, col 0
    }

    #[test]
    fn test_sdpa_basic() {
        // Simple identity-like test
        let d = 4;
        let seq = 2;
        let query = vec![1.0, 0.0, 0.0, 0.0,  0.0, 1.0, 0.0, 0.0];
        let key = query.clone();
        let value: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0,  5.0, 6.0, 7.0, 8.0];
        let out = scaled_dot_product_attention(&query, &key, &value, seq, seq, d, d, None);
        assert_eq!(out.len(), seq * d);
        // Output should be weighted combination of values
        assert!(out.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_sdpa_with_mask() {
        let d = 2;
        let seq = 3;
        let q = vec![1.0; seq * d];
        let k = vec![1.0; seq * d];
        let v: Vec<f64> = (0..seq * d).map(|i| i as f64).collect();
        let mask = causal_mask(seq);
        let out = scaled_dot_product_attention(&q, &k, &v, seq, seq, d, d, Some(&mask));
        assert_eq!(out.len(), seq * d);
        // First row can only attend to first position
        // Last row can attend to all positions
    }

    #[test]
    fn test_flash_attention_matches_sdpa() {
        let d = 4;
        let seq = 8;
        let mut state: u64 = 12345;
        let mut rng = || -> f64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as f64 / u64::MAX as f64) * 2.0 - 1.0
        };
        let q: Vec<f64> = (0..seq * d).map(|_| rng()).collect();
        let k: Vec<f64> = (0..seq * d).map(|_| rng()).collect();
        let v: Vec<f64> = (0..seq * d).map(|_| rng()).collect();

        let regular = scaled_dot_product_attention(&q, &k, &v, seq, seq, d, d, None);
        let flash = flash_attention_tiled(&q, &k, &v, seq, seq, d, d, 4);

        for (r, f) in regular.iter().zip(flash.iter()) {
            assert!((r - f).abs() < 1e-10, "Flash attention mismatch: {} vs {}", r, f);
        }
    }

    #[test]
    fn test_mha_forward() {
        let d_model = 16;
        let num_heads = 4;
        let seq_len = 4;
        let mha = MultiHeadAttention::new(d_model, num_heads, true);
        let input = vec![0.1; seq_len * d_model];
        let output = mha.forward(&input, seq_len, None);
        assert_eq!(output.len(), seq_len * d_model);
        assert!(output.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn test_gqa_forward() {
        let d_model = 16;
        let gqa = GroupedQueryAttention::new(d_model, 4, 2); // 4 Q heads, 2 KV heads
        let seq_len = 4;
        let input = vec![0.1; seq_len * d_model];
        let output = gqa.forward(&input, seq_len, None);
        assert_eq!(output.len(), seq_len * d_model);
    }

    #[test]
    fn test_swiglu_ffn() {
        let d_model = 16;
        let d_ff = 32;
        let seq = 4;
        let ffn = SwiGLUFFN::new(d_model, d_ff);
        let input = vec![0.1; seq * d_model];
        let output = ffn.forward(&input, seq);
        assert_eq!(output.len(), seq * d_model);
    }

    #[test]
    fn test_rms_norm() {
        let d = 4;
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let weight = vec![1.0; d];
        let normed = rms_norm(&data, &weight, d, 1e-6);
        // RMS of [1,2,3,4] = sqrt((1+4+9+16)/4) = sqrt(30/4) ≈ 2.7386
        let rms_val = (30.0_f64 / 4.0).sqrt();
        assert!((normed[0] - 1.0 / rms_val).abs() < 1e-4);
    }

    #[test]
    fn test_transformer_block() {
        let d_model = 16;
        let num_heads = 4;
        let d_ff = 32;
        let seq = 4;
        let block = TransformerBlock::new(d_model, num_heads, d_ff, true);
        let input = vec![0.1; seq * d_model];
        let output = block.forward(&input, seq, None);
        assert_eq!(output.len(), seq * d_model);
    }

    #[test]
    fn test_transformer_model() {
        let model = TransformerModel::new(16, 4, 2, 32, true);
        let seq = 4;
        let input = vec![0.1; seq * 16];
        let output = model.forward(&input, seq, None);
        assert_eq!(output.len(), seq * 16);
        assert!(model.total_params() > 0);
    }

    #[test]
    fn test_kv_cache() {
        let mut cache = KVCache::new(4, 4);
        let k = vec![1.0; 4]; // 1 token, d_k=4
        let v = vec![2.0; 4];
        cache.append(&k, &v, 1);
        assert_eq!(cache.cached_len, 1);
        cache.append(&k, &v, 1);
        assert_eq!(cache.cached_len, 2);
        assert_eq!(cache.keys().len(), 8);
    }

    #[test]
    fn test_causal_attention_ordering() {
        // Verify causal attention: output at position i should NOT depend on positions > i
        let d = 2;
        let seq = 4;
        let q = vec![1.0; seq * d];
        let k = vec![1.0; seq * d];
        let mut v1: Vec<f64> = (0..seq * d).map(|i| i as f64).collect();
        let mask = causal_mask(seq);

        let out1 = scaled_dot_product_attention(&q, &k, &v1, seq, seq, d, d, Some(&mask));

        // Change last position value — should not affect first position output
        v1[(seq - 1) * d] += 100.0;
        let out2 = scaled_dot_product_attention(&q, &k, &v1, seq, seq, d, d, Some(&mask));

        // First position output should be identical
        for i in 0..d {
            assert!((out1[i] - out2[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_ffi_transformer_create() {
        let id = vitalis_transformer_create(16, 4, 2, 32, 1);
        assert!(id > 0);
        let params = vitalis_transformer_params(id);
        assert!(params > 0);
    }

    #[test]
    fn test_ffi_sdpa() {
        let d = 4;
        let seq = 2;
        let q = vec![0.5f64; seq * d];
        let k = vec![0.5f64; seq * d];
        let v = vec![1.0f64; seq * d];
        let mut out = vec![0.0f64; seq * d];
        vitalis_attention_sdpa(q.as_ptr(), k.as_ptr(), v.as_ptr(), seq as i64, seq as i64, d as i64, d as i64, out.as_mut_ptr(), 0);
        assert!(out.iter().all(|&x| (x - 1.0).abs() < 1e-6));
    }

    #[test]
    fn test_ffi_rope() {
        let head_dim = 4;
        let seq = 2;
        let num_heads = 1;
        let mut data = vec![1.0f64; num_heads * seq * head_dim];
        let original = data.clone();
        vitalis_rope_apply(data.as_mut_ptr(), data.len() as i64, head_dim as i64, seq as i64);
        // Position 0 should have cos(0)=1, sin(0)=0, so pairs unchanged
        // But position 1 should be rotated
        assert!((data[0] - original[0]).abs() < 1e-10); // pos 0 unchanged
    }
}
