//! Deep Learning — Neural network layers for transformer models
//!
//! A complete set of neural network primitives ported from the Nova ML engine:
//! Linear layers, RMSNorm/LayerNorm, token embeddings, multi-head attention
//! with RoPE and GQA, SwiGLU/GELU feed-forward networks, and a full
//! decoder-only transformer.
//!
//! # Layer Hierarchy
//!
//! ```text
//! Transformer
//!  ├── TokenEmbedding
//!  ├── TransformerBlock[]
//!  │    ├── RMSNorm (attention pre-norm)
//!  │    ├── MultiHeadAttention (RoPE, GQA, causal mask)
//!  │    ├── RMSNorm (FFN pre-norm)
//!  │    └── SwiGLUFFN (gate + up + down)
//!  ├── RMSNorm (final)
//!  └── Linear (output head)
//! ```

use crate::tensor_engine::{
    Tensor, Shape, add, sub, mul, mul_scalar, div, matmul, transpose, softmax,
    embedding, silu, exp, sum,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Linear Layer
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Fully-connected linear layer: y = xW^T + b
pub struct Linear {
    pub weight: Tensor,         // [out_features, in_features]
    pub bias: Option<Tensor>,   // [out_features]
    pub in_features: usize,
    pub out_features: usize,
}

impl Linear {
    /// Create with Kaiming initialization.
    pub fn new(in_features: usize, out_features: usize, use_bias: bool) -> Self {
        let weight = Tensor::kaiming_uniform(&[out_features, in_features], in_features)
            .requires_grad_();
        let bias = if use_bias {
            Some(Tensor::zeros(&[out_features]).requires_grad_())
        } else {
            None
        };
        Self { weight, bias, in_features, out_features }
    }

    /// Forward: x [*, in] → [*, out]
    pub fn forward(&self, x: &Tensor) -> Tensor {
        let dims = x.dims();
        let last = *dims.last().unwrap();
        assert_eq!(last, self.in_features, "Linear: input dim {} != {}", last, self.in_features);

        match dims.len() {
            2 => {
                let mut out = matmul(x, &transpose(&self.weight));
                if let Some(ref b) = self.bias {
                    out = add(&out, &b.reshape(&[1, self.out_features]));
                }
                out
            }
            3 => {
                let (batch, seq, _) = (dims[0], dims[1], dims[2]);
                let flat = x.reshape(&[batch * seq, self.in_features]);
                let mut out = matmul(&flat, &transpose(&self.weight));
                if let Some(ref b) = self.bias {
                    out = add(&out, &b.reshape(&[1, self.out_features]));
                }
                out.reshape(&[batch, seq, self.out_features])
            }
            _ => panic!("Linear: unsupported input ndim={}", dims.len()),
        }
    }

    pub fn parameters(&self) -> Vec<&Tensor> {
        let mut p = vec![&self.weight];
        if let Some(ref b) = self.bias { p.push(b); }
        p
    }

    pub fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        let mut p = vec![&mut self.weight];
        if let Some(ref mut b) = self.bias { p.push(b); }
        p
    }

    pub fn num_params(&self) -> usize {
        self.weight.numel() + self.bias.as_ref().map(|b| b.numel()).unwrap_or(0)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// RMSNorm
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Root Mean Square Layer Normalization (RMSNorm).
/// Used in LLaMA, Mistral, and modern transformer architectures.
pub struct RMSNorm {
    pub weight: Tensor,  // [dim]
    pub dim: usize,
    pub eps: f32,
}

impl RMSNorm {
    pub fn new(dim: usize, eps: f32) -> Self {
        Self {
            weight: Tensor::ones(&[dim]).requires_grad_(),
            dim,
            eps,
        }
    }

    /// Forward: x [*, dim] → [*, dim]
    pub fn forward(&self, x: &Tensor) -> Tensor {
        let d = x.data_f32();
        let xd = x.dims();
        let dim = *xd.last().unwrap();
        assert_eq!(dim, self.dim);
        let n = d.len() / dim;
        let w = self.weight.data_f32();
        let mut out = vec![0.0f32; d.len()];

        for i in 0..n {
            let row = &d[i * dim..(i + 1) * dim];
            let rms: f32 = (row.iter().map(|&x| x * x).sum::<f32>() / dim as f32 + self.eps).sqrt();
            let inv_rms = 1.0 / rms;
            for j in 0..dim {
                out[i * dim + j] = row[j] * inv_rms * w[j];
            }
        }
        Tensor::from_vec(out, xd)
    }

    pub fn parameters(&self) -> Vec<&Tensor> { vec![&self.weight] }
    pub fn parameters_mut(&mut self) -> Vec<&mut Tensor> { vec![&mut self.weight] }
    pub fn num_params(&self) -> usize { self.dim }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// LayerNorm
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Standard Layer Normalization (used in GPT-2, BERT).
pub struct LayerNorm {
    pub weight: Tensor,  // [dim]
    pub bias: Tensor,    // [dim]
    pub dim: usize,
    pub eps: f32,
}

impl LayerNorm {
    pub fn new(dim: usize, eps: f32) -> Self {
        Self {
            weight: Tensor::ones(&[dim]).requires_grad_(),
            bias: Tensor::zeros(&[dim]).requires_grad_(),
            dim,
            eps,
        }
    }

    /// Forward: x [*, dim] → [*, dim]
    pub fn forward(&self, x: &Tensor) -> Tensor {
        let d = x.data_f32();
        let xd = x.dims();
        let dim = *xd.last().unwrap();
        let n = d.len() / dim;
        let w = self.weight.data_f32();
        let b = self.bias.data_f32();
        let mut out = vec![0.0f32; d.len()];

        for i in 0..n {
            let row = &d[i * dim..(i + 1) * dim];
            let mean: f32 = row.iter().sum::<f32>() / dim as f32;
            let var: f32 = row.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / dim as f32;
            let inv_std = 1.0 / (var + self.eps).sqrt();
            for j in 0..dim {
                out[i * dim + j] = (row[j] - mean) * inv_std * w[j] + b[j];
            }
        }
        Tensor::from_vec(out, xd)
    }

    pub fn parameters(&self) -> Vec<&Tensor> { vec![&self.weight, &self.bias] }
    pub fn num_params(&self) -> usize { self.dim * 2 }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Token Embedding
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Token embedding table.
pub struct TokenEmbedding {
    pub weight: Tensor,     // [vocab_size, d_model]
    pub vocab_size: usize,
    pub d_model: usize,
}

impl TokenEmbedding {
    pub fn new(vocab_size: usize, d_model: usize) -> Self {
        let weight = Tensor::randn(&[vocab_size, d_model]).requires_grad_();
        // scale by 1/sqrt(d_model)
        let scale = 1.0 / (d_model as f32).sqrt();
        let weight = mul_scalar(&weight, scale).requires_grad_();
        Self { weight, vocab_size, d_model }
    }

    /// Single sequence: ids [seq] → [seq, d_model]
    pub fn forward(&self, token_ids: &[usize]) -> Tensor {
        embedding(&self.weight, token_ids)
    }

    /// Batch: ids [batch][seq] → [batch, seq, d_model]
    pub fn forward_batch(&self, batch_ids: &[Vec<usize>]) -> Tensor {
        let batch = batch_ids.len();
        let seq = batch_ids[0].len();
        let mut data = Vec::with_capacity(batch * seq * self.d_model);
        let wd = self.weight.data_f32();
        for ids in batch_ids {
            for &id in ids {
                let start = id * self.d_model;
                data.extend_from_slice(&wd[start..start + self.d_model]);
            }
        }
        Tensor::from_vec(data, &[batch, seq, self.d_model])
    }

    pub fn parameters(&self) -> Vec<&Tensor> { vec![&self.weight] }
    pub fn parameters_mut(&mut self) -> Vec<&mut Tensor> { vec![&mut self.weight] }
    pub fn num_params(&self) -> usize { self.vocab_size * self.d_model }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Multi-Head Attention with RoPE and GQA
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Multi-head attention with:
/// - Rotary Position Embeddings (RoPE)
/// - Grouped Query Attention (GQA)
/// - Causal masking
pub struct MultiHeadAttention {
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub head_dim: usize,
    pub d_model: usize,
    pub wq: Linear,
    pub wk: Linear,
    pub wv: Linear,
    pub wo: Linear,
    pub rope_base: f32,
    pub max_seq_len: usize,
    pub rope_cos: Tensor,  // [max_seq_len, head_dim/2]
    pub rope_sin: Tensor,  // [max_seq_len, head_dim/2]
}

impl MultiHeadAttention {
    pub fn new(d_model: usize, n_heads: usize, n_kv_heads: usize,
               rope_base: f32, max_seq_len: usize) -> Self {
        let head_dim = d_model / n_heads;
        let (cos, sin) = Self::precompute_rope(max_seq_len, head_dim, rope_base);

        Self {
            n_heads,
            n_kv_heads,
            head_dim,
            d_model,
            wq: Linear::new(d_model, n_heads * head_dim, false),
            wk: Linear::new(d_model, n_kv_heads * head_dim, false),
            wv: Linear::new(d_model, n_kv_heads * head_dim, false),
            wo: Linear::new(n_heads * head_dim, d_model, false),
            rope_base,
            max_seq_len,
            rope_cos: cos,
            rope_sin: sin,
        }
    }

    fn precompute_rope(max_len: usize, head_dim: usize, base: f32) -> (Tensor, Tensor) {
        let half = head_dim / 2;
        let mut cos_data = vec![0.0f32; max_len * half];
        let mut sin_data = vec![0.0f32; max_len * half];
        for pos in 0..max_len {
            for i in 0..half {
                let freq = 1.0 / base.powf(2.0 * i as f32 / head_dim as f32);
                let angle = pos as f32 * freq;
                cos_data[pos * half + i] = angle.cos();
                sin_data[pos * half + i] = angle.sin();
            }
        }
        (
            Tensor::from_vec(cos_data, &[max_len, half]),
            Tensor::from_vec(sin_data, &[max_len, half]),
        )
    }

    /// Forward: x [batch, seq, d_model] → [batch, seq, d_model]
    pub fn forward(&self, x: &Tensor, start_pos: usize, causal: bool) -> Tensor {
        let dims = x.dims();
        let (batch, seq, _) = (dims[0], dims[1], dims[2]);
        let hd = self.head_dim;
        let half = hd / 2;

        // Project Q, K, V
        let q = self.wq.forward(x); // [batch, seq, n_heads * head_dim]
        let k = self.wk.forward(x); // [batch, seq, n_kv_heads * head_dim]
        let v = self.wv.forward(x); // [batch, seq, n_kv_heads * head_dim]

        let qd = q.data_f32();
        let kd = k.data_f32();
        let vd = v.data_f32();
        let cos = self.rope_cos.data_f32();
        let sin = self.rope_sin.data_f32();

        // Apply RoPE
        let q_rope = self.apply_rope_batch(qd, cos, sin, batch, seq, self.n_heads, hd, half, start_pos);
        let k_rope = self.apply_rope_batch(kd, cos, sin, batch, seq, self.n_kv_heads, hd, half, start_pos);

        // GQA: repeat KV heads
        let reps = self.n_heads / self.n_kv_heads;
        let k_expanded = if reps > 1 { Self::repeat_kv(&k_rope, batch, seq, self.n_kv_heads, hd, reps) }
                          else { k_rope };
        let v_expanded = if reps > 1 { Self::repeat_kv(vd, batch, seq, self.n_kv_heads, hd, reps) }
                          else { vd.to_vec() };

        // Attention: Q @ K^T / sqrt(head_dim)
        let scale = 1.0 / (hd as f32).sqrt();
        let mut output = vec![0.0f32; batch * seq * self.n_heads * hd];

        for b in 0..batch {
            for h in 0..self.n_heads {
                // Compute attention scores
                let mut scores = vec![0.0f32; seq * seq];
                for i in 0..seq {
                    for j in 0..seq {
                        if causal && j > i { continue; }
                        let mut dot = 0.0f32;
                        for d in 0..hd {
                            let qi = q_rope[(b * self.n_heads + h) * seq * hd + i * hd + d];
                            let ki = k_expanded[(b * self.n_heads + h) * seq * hd + j * hd + d];
                            dot += qi * ki;
                        }
                        scores[i * seq + j] = dot * scale;
                    }
                    // Apply causal mask
                    if causal {
                        for j in (i + 1)..seq {
                            scores[i * seq + j] = f32::NEG_INFINITY;
                        }
                    }
                }

                // Softmax per row
                for i in 0..seq {
                    let row = &mut scores[i * seq..(i + 1) * seq];
                    let max_val = row.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let mut sum_exp = 0.0f32;
                    for v in row.iter_mut() {
                        *v = (*v - max_val).exp();
                        sum_exp += *v;
                    }
                    for v in row.iter_mut() { *v /= sum_exp; }
                }

                // Weighted sum of values
                for i in 0..seq {
                    for d in 0..hd {
                        let mut val = 0.0f32;
                        for j in 0..seq {
                            let vij = v_expanded[(b * self.n_heads + h) * seq * hd + j * hd + d];
                            val += scores[i * seq + j] * vij;
                        }
                        output[(b * seq + i) * self.n_heads * hd + h * hd + d] = val;
                    }
                }
            }
        }

        // Concatenate heads and project
        let attn_out = Tensor::from_vec(output, &[batch, seq, self.n_heads * hd]);
        self.wo.forward(&attn_out)
    }

    fn apply_rope_batch(&self, data: &[f32], cos: &[f32], sin: &[f32],
                        batch: usize, seq: usize, n_heads: usize,
                        hd: usize, half: usize, start_pos: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; batch * n_heads * seq * hd];
        for b in 0..batch {
            for s in 0..seq {
                let pos = start_pos + s;
                for h in 0..n_heads {
                    let src_off = (b * seq + s) * n_heads * hd + h * hd;
                    let dst_off = (b * n_heads + h) * seq * hd + s * hd;
                    for i in 0..half {
                        let x0 = data[src_off + i];
                        let x1 = data[src_off + half + i];
                        let c = cos[pos * half + i];
                        let si = sin[pos * half + i];
                        out[dst_off + i] = x0 * c - x1 * si;
                        out[dst_off + half + i] = x0 * si + x1 * c;
                    }
                }
            }
        }
        out
    }

    fn repeat_kv(data: &[f32], batch: usize, seq: usize, n_kv: usize,
                 hd: usize, reps: usize) -> Vec<f32> {
        let n_heads = n_kv * reps;
        let mut out = vec![0.0f32; batch * n_heads * seq * hd];
        for b in 0..batch {
            for kv in 0..n_kv {
                for r in 0..reps {
                    let h = kv * reps + r;
                    let src_off = (b * n_kv + kv) * seq * hd;
                    let dst_off = (b * n_heads + h) * seq * hd;
                    out[dst_off..dst_off + seq * hd]
                        .copy_from_slice(&data[src_off..src_off + seq * hd]);
                }
            }
        }
        out
    }

    pub fn parameters(&self) -> Vec<&Tensor> {
        let mut p = self.wq.parameters();
        p.extend(self.wk.parameters());
        p.extend(self.wv.parameters());
        p.extend(self.wo.parameters());
        p
    }

    pub fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        let mut p = self.wq.parameters_mut();
        p.extend(self.wk.parameters_mut());
        p.extend(self.wv.parameters_mut());
        p.extend(self.wo.parameters_mut());
        p
    }

    pub fn num_params(&self) -> usize {
        self.wq.num_params() + self.wk.num_params()
            + self.wv.num_params() + self.wo.num_params()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// SwiGLU Feed-Forward Network
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// SwiGLU feed-forward network (LLaMA-style).
/// FFN(x) = w_down(SiLU(w_gate(x)) * w_up(x))
pub struct SwiGLUFFN {
    pub w_gate: Linear,   // d_model → d_ff
    pub w_up: Linear,     // d_model → d_ff
    pub w_down: Linear,   // d_ff → d_model
    pub d_model: usize,
    pub d_ff: usize,
}

impl SwiGLUFFN {
    pub fn new(d_model: usize, d_ff: usize) -> Self {
        Self {
            w_gate: Linear::new(d_model, d_ff, false),
            w_up: Linear::new(d_model, d_ff, false),
            w_down: Linear::new(d_ff, d_model, false),
            d_model,
            d_ff,
        }
    }

    /// Forward: x [*, d_model] → [*, d_model]
    pub fn forward(&self, x: &Tensor) -> Tensor {
        let gate = self.w_gate.forward(x);
        let up = self.w_up.forward(x);
        let gate_act = silu(&gate);
        let hidden = mul(&gate_act, &up);
        self.w_down.forward(&hidden)
    }

    pub fn parameters(&self) -> Vec<&Tensor> {
        let mut p = self.w_gate.parameters();
        p.extend(self.w_up.parameters());
        p.extend(self.w_down.parameters());
        p
    }

    pub fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        let mut p = self.w_gate.parameters_mut();
        p.extend(self.w_up.parameters_mut());
        p.extend(self.w_down.parameters_mut());
        p
    }

    pub fn num_params(&self) -> usize {
        self.w_gate.num_params() + self.w_up.num_params() + self.w_down.num_params()
    }
}

/// GELU feed-forward network (GPT-2 style).
pub struct GeluFFN {
    pub w1: Linear,
    pub w2: Linear,
    pub d_model: usize,
    pub d_ff: usize,
}

impl GeluFFN {
    pub fn new(d_model: usize, d_ff: usize) -> Self {
        Self {
            w1: Linear::new(d_model, d_ff, true),
            w2: Linear::new(d_ff, d_model, true),
            d_model,
            d_ff,
        }
    }

    pub fn forward(&self, x: &Tensor) -> Tensor {
        use crate::tensor_engine::gelu;
        let h = gelu(&self.w1.forward(x));
        self.w2.forward(&h)
    }

    pub fn parameters(&self) -> Vec<&Tensor> {
        let mut p = self.w1.parameters();
        p.extend(self.w2.parameters());
        p
    }

    pub fn num_params(&self) -> usize {
        self.w1.num_params() + self.w2.num_params()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Transformer Block
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Single decoder transformer block with pre-norm architecture.
pub struct TransformerBlock {
    pub attn_norm: RMSNorm,
    pub attention: MultiHeadAttention,
    pub ffn_norm: RMSNorm,
    pub ffn: SwiGLUFFN,
    pub layer_idx: usize,
}

impl TransformerBlock {
    pub fn new(d_model: usize, n_heads: usize, n_kv_heads: usize,
               d_ff: usize, rope_base: f32, max_seq_len: usize,
               eps: f32, layer_idx: usize) -> Self {
        Self {
            attn_norm: RMSNorm::new(d_model, eps),
            attention: MultiHeadAttention::new(d_model, n_heads, n_kv_heads,
                                                rope_base, max_seq_len),
            ffn_norm: RMSNorm::new(d_model, eps),
            ffn: SwiGLUFFN::new(d_model, d_ff),
            layer_idx,
        }
    }

    /// Forward with residual connections.
    pub fn forward(&self, x: &Tensor, start_pos: usize, causal: bool) -> Tensor {
        // Attention sub-layer
        let normed = self.attn_norm.forward(x);
        let attn_out = self.attention.forward(&normed, start_pos, causal);
        let x = add(x, &attn_out);

        // FFN sub-layer
        let normed = self.ffn_norm.forward(&x);
        let ffn_out = self.ffn.forward(&normed);
        add(&x, &ffn_out)
    }

    pub fn parameters(&self) -> Vec<&Tensor> {
        let mut p = self.attn_norm.parameters();
        p.extend(self.attention.parameters());
        p.extend(self.ffn_norm.parameters());
        p.extend(self.ffn.parameters());
        p
    }

    pub fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        let mut p = self.attn_norm.parameters_mut();
        p.extend(self.attention.parameters_mut());
        p.extend(self.ffn_norm.parameters_mut());
        p.extend(self.ffn.parameters_mut());
        p
    }

    pub fn num_params(&self) -> usize {
        self.attn_norm.num_params() + self.attention.num_params()
            + self.ffn_norm.num_params() + self.ffn.num_params()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Transformer Config
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Configuration for a decoder-only transformer model.
#[derive(Clone, Debug)]
pub struct TransformerConfig {
    pub d_model: usize,
    pub n_layers: usize,
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub d_ff: usize,
    pub vocab_size: usize,
    pub max_seq_len: usize,
    pub norm_eps: f32,
    pub tie_weights: bool,
}

impl TransformerConfig {
    /// Tiny model for testing (~5M params).
    pub fn tiny() -> Self {
        Self {
            d_model: 128, n_layers: 4, n_heads: 4, n_kv_heads: 4,
            d_ff: 344, vocab_size: 8000, max_seq_len: 512,
            norm_eps: 1e-5, tie_weights: true,
        }
    }

    /// Small model (~125M params).
    pub fn small_125m() -> Self {
        Self {
            d_model: 768, n_layers: 12, n_heads: 12, n_kv_heads: 4,
            d_ff: 2048, vocab_size: 32000, max_seq_len: 2048,
            norm_eps: 1e-5, tie_weights: false,
        }
    }

    /// Medium model (~1B params).
    pub fn medium_1b() -> Self {
        Self {
            d_model: 2048, n_layers: 22, n_heads: 16, n_kv_heads: 4,
            d_ff: 5504, vocab_size: 32000, max_seq_len: 2048,
            norm_eps: 1e-5, tie_weights: false,
        }
    }

    /// Estimate total parameters.
    pub fn estimate_params(&self) -> usize {
        let emb = self.vocab_size * self.d_model;
        let per_layer = 4 * self.d_model * self.d_model // attention Q/K/V/O
            + 3 * self.d_model * self.d_ff              // SwiGLU gate/up/down
            + 2 * self.d_model;                         // 2 norms
        let total = emb + self.n_layers * per_layer + self.d_model;
        if self.tie_weights { total } else { total + self.vocab_size * self.d_model }
    }

    /// Estimated VRAM in GB for inference (fp32).
    pub fn estimate_vram_gb(&self) -> f64 {
        (self.estimate_params() * 4) as f64 / 1e9
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Transformer Model
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Full decoder-only transformer (GPT/LLaMA architecture).
pub struct Transformer {
    pub token_emb: TokenEmbedding,
    pub layers: Vec<TransformerBlock>,
    pub final_norm: RMSNorm,
    pub output_head: Linear,
    pub tie_weights: bool,
    pub config: TransformerConfig,
}

impl Transformer {
    /// Create a new transformer from config.
    pub fn new(config: TransformerConfig) -> Self {
        let token_emb = TokenEmbedding::new(config.vocab_size, config.d_model);
        let layers: Vec<_> = (0..config.n_layers).map(|i| {
            TransformerBlock::new(
                config.d_model, config.n_heads, config.n_kv_heads,
                config.d_ff, 10000.0, config.max_seq_len,
                config.norm_eps, i,
            )
        }).collect();
        let final_norm = RMSNorm::new(config.d_model, config.norm_eps);
        let output_head = Linear::new(config.d_model, config.vocab_size, false);

        let mut model = Self { token_emb, layers, final_norm, output_head,
                              tie_weights: config.tie_weights, config };

        // Tie embedding weights if configured
        if model.tie_weights {
            model.output_head.weight = model.token_emb.weight.clone();
        }
        model
    }

    /// Inference forward: token_ids [batch][seq] → logits [batch, seq, vocab]
    pub fn forward(&self, token_ids: &[Vec<usize>], start_pos: usize) -> Tensor {
        let mut hidden = self.token_emb.forward_batch(token_ids);
        for layer in &self.layers {
            hidden = layer.forward(&hidden, start_pos, true);
        }
        hidden = self.final_norm.forward(&hidden);
        self.output_head.forward(&hidden)
    }

    /// All trainable parameters.
    pub fn parameters(&self) -> Vec<&Tensor> {
        let mut p = self.token_emb.parameters();
        for layer in &self.layers {
            p.extend(layer.parameters());
        }
        p.extend(self.final_norm.parameters());
        if !self.tie_weights {
            p.extend(self.output_head.parameters());
        }
        p
    }

    pub fn parameters_mut(&mut self) -> Vec<&mut Tensor> {
        let mut p = self.token_emb.parameters_mut();
        for layer in &mut self.layers {
            p.extend(layer.parameters_mut());
        }
        p.extend(self.final_norm.parameters_mut());
        if !self.tie_weights {
            p.extend(self.output_head.parameters_mut());
        }
        p
    }

    pub fn num_params(&self) -> usize {
        let mut total = self.token_emb.num_params();
        for layer in &self.layers { total += layer.num_params(); }
        total += self.final_norm.num_params();
        if !self.tie_weights { total += self.output_head.num_params(); }
        total
    }

    /// Print model architecture summary.
    pub fn print_summary(&self) {
        println!("╔══════════════════════════════════════════════════╗");
        println!("║          Transformer Model Summary               ║");
        println!("╠══════════════════════════════════════════════════╣");
        println!("║ d_model:      {:<36}║", self.config.d_model);
        println!("║ n_layers:     {:<36}║", self.config.n_layers);
        println!("║ n_heads:      {:<36}║", self.config.n_heads);
        println!("║ n_kv_heads:   {:<36}║", self.config.n_kv_heads);
        println!("║ d_ff:         {:<36}║", self.config.d_ff);
        println!("║ vocab_size:   {:<36}║", self.config.vocab_size);
        println!("║ max_seq_len:  {:<36}║", self.config.max_seq_len);
        println!("║ tie_weights:  {:<36}║", self.config.tie_weights);
        println!("╠══════════════════════════════════════════════════╣");
        println!("║ TOTAL:        {:<10} ({:.1}M)              ║",
                 self.num_params(), self.num_params() as f64 / 1e6);
        println!("╚══════════════════════════════════════════════════╝");
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FFI — extern "C" functions for Vitalis stdlib
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a transformer model. Returns opaque handle.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_transformer_new(
    d_model: i64, n_layers: i64, n_heads: i64, vocab_size: i64,
) -> i64 {
    let config = TransformerConfig {
        d_model: d_model as usize,
        n_layers: n_layers as usize,
        n_heads: n_heads as usize,
        n_kv_heads: n_heads as usize,
        d_ff: (d_model as usize * 8 / 3 + 7) & !7,
        vocab_size: vocab_size as usize,
        max_seq_len: 2048,
        norm_eps: 1e-5,
        tie_weights: true,
    };
    let model = Box::new(Transformer::new(config));
    Box::into_raw(model) as i64
}

/// Get model parameter count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_transformer_params(handle: i64) -> i64 {
    let model = unsafe { &*(handle as *const Transformer) };
    model.num_params() as i64
}

/// Free a transformer model.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_transformer_free(handle: i64) {
    if handle != 0 {
        let _ = unsafe { Box::from_raw(handle as *mut Transformer) };
    }
}

/// RMSNorm forward pass on raw data.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rmsnorm(
    data: *const f64, weights: *const f64, out: *mut f64,
    n: i64, dim: i64, eps: f64,
) -> i64 {
    if data.is_null() || weights.is_null() || out.is_null() { return -1; }
    let (n, dim) = (n as usize, dim as usize);
    let d = unsafe { std::slice::from_raw_parts(data, n * dim) };
    let w = unsafe { std::slice::from_raw_parts(weights, dim) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n * dim) };

    for i in 0..n {
        let row = &d[i * dim..(i + 1) * dim];
        let rms: f64 = (row.iter().map(|&x| x * x).sum::<f64>() / dim as f64 + eps).sqrt();
        let inv = 1.0 / rms;
        for j in 0..dim {
            o[i * dim + j] = row[j] * inv * w[j];
        }
    }
    0
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_forward() {
        let linear = Linear::new(4, 3, true);
        let x = Tensor::randn(&[2, 4]);
        let y = linear.forward(&x);
        assert_eq!(y.dims(), &[2, 3]);
    }

    #[test]
    fn test_rmsnorm() {
        let norm = RMSNorm::new(4, 1e-5);
        let x = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], &[1, 4]);
        let y = norm.forward(&x);
        assert_eq!(y.dims(), &[1, 4]);
        // RMSNorm should produce values with mean-square close to 1
        let yd = y.data_f32();
        let rms: f32 = (yd.iter().map(|&x| x * x).sum::<f32>() / 4.0).sqrt();
        assert!((rms - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_embedding() {
        let emb = TokenEmbedding::new(100, 32);
        let out = emb.forward(&[0, 1, 2]);
        assert_eq!(out.dims(), &[3, 32]);
    }

    #[test]
    fn test_swiglu_ffn() {
        let ffn = SwiGLUFFN::new(64, 128);
        let x = Tensor::randn(&[2, 4, 64]);
        let y = ffn.forward(&x);
        assert_eq!(y.dims(), &[2, 4, 64]);
    }

    #[test]
    fn test_transformer_config() {
        let cfg = TransformerConfig::tiny();
        let params = cfg.estimate_params();
        assert!(params > 1_000_000 && params < 10_000_000);
    }

    #[test]
    fn test_transformer_forward() {
        let config = TransformerConfig {
            d_model: 32, n_layers: 2, n_heads: 2, n_kv_heads: 2,
            d_ff: 64, vocab_size: 100, max_seq_len: 64,
            norm_eps: 1e-5, tie_weights: true,
        };
        let model = Transformer::new(config);
        let ids = vec![vec![1, 2, 3, 4]];
        let logits = model.forward(&ids, 0);
        assert_eq!(logits.dims(), &[1, 4, 100]);
    }
}
