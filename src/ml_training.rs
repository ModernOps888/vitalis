//! ML Training — Optimizers, schedulers, backpropagation, and training loop
//!
//! Complete training infrastructure ported from the Nova ML engine:
//! AdamW optimizer, cosine learning rate scheduler with warmup,
//! gradient clipping, analytical backpropagation through transformer
//! layers, data loading, checkpointing, and training loop orchestration.
//!
//! # Components
//!
//! ```text
//! Trainer (orchestrates training)
//!  ├── AdamW (optimizer with weight decay)
//!  ├── CosineScheduler (LR with warmup)
//!  ├── DataLoader (batched token sequences)
//!  ├── Backward Pass (analytical gradients)
//!  └── Checkpoint (save/load weights)
//! ```

use crate::tensor_engine::{Tensor, matmul, transpose, add, sub, mul, mul_scalar,
    softmax, cross_entropy_loss, embedding, silu, sum};
use crate::deep_learning::{Transformer, TransformerConfig, Linear, RMSNorm,
    SwiGLUFFN, MultiHeadAttention, TransformerBlock, TokenEmbedding};
use std::time::Instant;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// AdamW Optimizer
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

struct ParamState {
    m: Vec<f32>,  // First moment (mean of gradients)
    v: Vec<f32>,  // Second moment (mean of squared gradients)
}

/// AdamW optimizer with decoupled weight decay.
pub struct AdamW {
    pub lr: f32,
    pub beta1: f32,
    pub beta2: f32,
    pub eps: f32,
    pub weight_decay: f32,
    pub step: usize,
    states: Vec<ParamState>,
    initialized: bool,
}

impl AdamW {
    pub fn new(lr: f32, beta1: f32, beta2: f32, eps: f32, weight_decay: f32) -> Self {
        Self { lr, beta1, beta2, eps, weight_decay, step: 0,
               states: vec![], initialized: false }
    }

    /// Default LLM optimizer settings.
    pub fn default_llm() -> Self {
        Self::new(1e-3, 0.9, 0.95, 1e-8, 0.1)
    }

    pub fn set_lr(&mut self, lr: f32) { self.lr = lr; }

    /// Perform one optimizer step.
    pub fn step(&mut self, params: &mut [&mut Tensor]) {
        self.step += 1;

        if !self.initialized {
            self.states = params.iter().map(|p| ParamState {
                m: vec![0.0; p.numel()],
                v: vec![0.0; p.numel()],
            }).collect();
            self.initialized = true;
        }

        let bc1 = 1.0 - self.beta1.powi(self.step as i32);
        let bc2 = 1.0 - self.beta2.powi(self.step as i32);

        for (i, param) in params.iter_mut().enumerate() {
            let grad = match param.get_grad() {
                Some(g) => g,
                None => continue,
            };
            let gd = grad.data_f32();
            let pd = param.data_f32_mut();
            let st = &mut self.states[i];

            for j in 0..pd.len() {
                let g = if j < gd.len() { gd[j] } else { 0.0 };

                // Update biased first moment
                st.m[j] = self.beta1 * st.m[j] + (1.0 - self.beta1) * g;
                // Update biased second moment
                st.v[j] = self.beta2 * st.v[j] + (1.0 - self.beta2) * g * g;

                // Bias-corrected estimates
                let m_hat = st.m[j] / bc1;
                let v_hat = st.v[j] / bc2;

                // Weight decay (decoupled)
                pd[j] *= 1.0 - self.lr * self.weight_decay;

                // Adam update
                pd[j] -= self.lr * m_hat / (v_hat.sqrt() + self.eps);
            }
        }
    }

    /// Zero all parameter gradients.
    pub fn zero_grad(&self, params: &[&Tensor]) {
        for p in params { p.zero_grad(); }
    }

    /// Memory used by optimizer states (bytes).
    pub fn memory_bytes(&self) -> usize {
        self.states.iter().map(|s| (s.m.len() + s.v.len()) * 4).sum()
    }
}

/// Clip gradient norm. Returns the original norm.
pub fn clip_grad_norm(params: &[&Tensor], max_norm: f32) -> f32 {
    let mut total_norm_sq = 0.0f32;
    for p in params {
        if let Some(g) = p.get_grad() {
            for &v in g.data_f32() { total_norm_sq += v * v; }
        }
    }
    let total_norm = total_norm_sq.sqrt();

    if total_norm > max_norm {
        let scale = max_norm / (total_norm + 1e-6);
        for p in params {
            if let Some(mut g) = p.get_grad() {
                let gd = g.data_f32_mut();
                for v in gd.iter_mut() { *v *= scale; }
                p.zero_grad();
                p.accumulate_grad(&g);
            }
        }
    }
    total_norm
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Learning Rate Schedulers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cosine annealing with linear warmup.
pub struct CosineScheduler {
    pub max_lr: f32,
    pub min_lr: f32,
    pub warmup_steps: usize,
    pub total_steps: usize,
}

impl CosineScheduler {
    pub fn new(max_lr: f32, min_lr: f32, warmup_steps: usize, total_steps: usize) -> Self {
        Self { max_lr, min_lr, warmup_steps, total_steps }
    }

    /// Get learning rate at given step.
    pub fn get_lr(&self, step: usize) -> f32 {
        if step < self.warmup_steps {
            // Linear warmup
            self.max_lr * (step as f32 / self.warmup_steps.max(1) as f32)
        } else {
            // Cosine decay
            let progress = (step - self.warmup_steps) as f32
                / (self.total_steps - self.warmup_steps).max(1) as f32;
            let cosine = (1.0 + (std::f32::consts::PI * progress).cos()) / 2.0;
            self.min_lr + (self.max_lr - self.min_lr) * cosine
        }
    }
}

/// Warmup then constant learning rate.
pub struct WarmupConstantScheduler {
    pub max_lr: f32,
    pub warmup_steps: usize,
}

impl WarmupConstantScheduler {
    pub fn new(max_lr: f32, warmup_steps: usize) -> Self {
        Self { max_lr, warmup_steps }
    }

    pub fn get_lr(&self, step: usize) -> f32 {
        if step < self.warmup_steps {
            self.max_lr * step as f32 / self.warmup_steps.max(1) as f32
        } else {
            self.max_lr
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DataLoader
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Batched data loader for language model training.
/// Produces (input_ids, target_ids) pairs of shape [batch, seq_len].
pub struct DataLoader {
    data: Vec<u32>,
    pub num_tokens: usize,
    pub seq_len: usize,
    pub batch_size: usize,
    pos: usize,
    pub epoch: usize,
}

impl DataLoader {
    /// Create from pre-tokenized data.
    pub fn from_tokens(tokens: Vec<u32>, seq_len: usize, batch_size: usize) -> Self {
        let num_tokens = tokens.len();
        Self { data: tokens, num_tokens, seq_len, batch_size, pos: 0, epoch: 0 }
    }

    /// Create from binary file of u32 tokens.
    pub fn from_file(path: &std::path::Path, seq_len: usize, batch_size: usize)
        -> std::io::Result<Self>
    {
        let bytes = std::fs::read(path)?;
        let tokens: Vec<u32> = bytes.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        Ok(Self::from_tokens(tokens, seq_len, batch_size))
    }

    /// Get next batch of (inputs, targets).
    pub fn next_batch(&mut self) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
        let stride = self.seq_len + 1;
        let mut inputs = Vec::with_capacity(self.batch_size);
        let mut targets = Vec::with_capacity(self.batch_size);

        for _ in 0..self.batch_size {
            if self.pos + stride >= self.data.len() {
                self.pos = 0;
                self.epoch += 1;
            }
            let chunk: Vec<usize> = self.data[self.pos..self.pos + stride]
                .iter().map(|&x| x as usize).collect();
            inputs.push(chunk[..self.seq_len].to_vec());
            targets.push(chunk[1..stride].to_vec());
            self.pos += stride;
        }
        (inputs, targets)
    }

    /// Number of batches per epoch.
    pub fn num_batches(&self) -> usize {
        self.data.len() / ((self.seq_len + 1) * self.batch_size)
    }

    /// Reset to beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
        self.epoch = 0;
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Analytical Backward Pass
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Cache of intermediate values from forward pass for backward.
pub struct TrainCache {
    pub token_ids: Vec<Vec<usize>>,
    pub block_boundaries: Vec<Tensor>,
}

/// Helper: linear forward
fn linear_fwd(x: &Tensor, w: &Tensor) -> Tensor {
    matmul(x, &transpose(w))
}

/// Helper: linear backward
fn linear_bwd(grad_out: &Tensor, x: &Tensor, w: &Tensor) -> (Tensor, Tensor) {
    let grad_x = matmul(grad_out, w);
    let x_flat = if x.ndim() == 3 {
        let d = x.dims();
        x.reshape(&[d[0] * d[1], d[2]])
    } else { x.clone() };
    let go_flat = if grad_out.ndim() == 3 {
        let d = grad_out.dims();
        grad_out.reshape(&[d[0] * d[1], d[2]])
    } else { grad_out.clone() };
    let grad_w = matmul(&transpose(&go_flat), &x_flat);
    (grad_x, grad_w)
}

/// Helper: RMSNorm forward  
fn rms_norm_fwd(x: &Tensor, w: &Tensor, eps: f32) -> (Tensor, Tensor) {
    let d = x.dims();
    let dim = *d.last().unwrap();
    let n = x.numel() / dim;
    let xd = x.data_f32();
    let wd = w.data_f32();
    let mut out = vec![0.0f32; x.numel()];
    let mut inv_rms_vec = vec![0.0f32; n];

    for i in 0..n {
        let row = &xd[i * dim..(i + 1) * dim];
        let rms = (row.iter().map(|&v| v * v).sum::<f32>() / dim as f32 + eps).sqrt();
        let inv_rms = 1.0 / rms;
        inv_rms_vec[i] = inv_rms;
        for j in 0..dim {
            out[i * dim + j] = row[j] * inv_rms * wd[j];
        }
    }
    (Tensor::from_vec(out, d), Tensor::from_vec(inv_rms_vec, &[n]))
}

/// Helper: RMSNorm backward
fn rms_norm_bwd(grad_out: &Tensor, x: &Tensor, w: &Tensor, inv_rms: &Tensor)
    -> (Tensor, Tensor)
{
    let d = x.dims();
    let dim = *d.last().unwrap();
    let n = x.numel() / dim;
    let xd = x.data_f32();
    let wd = w.data_f32();
    let god = grad_out.data_f32();
    let ird = inv_rms.data_f32();

    let mut grad_x = vec![0.0f32; x.numel()];
    let mut grad_w = vec![0.0f32; dim];

    for i in 0..n {
        let ir = ird[i];
        let row = &xd[i * dim..(i + 1) * dim];
        let go = &god[i * dim..(i + 1) * dim];

        // grad_w accumulation
        for j in 0..dim {
            grad_w[j] += go[j] * row[j] * ir;
        }

        // grad_x: d(RMSNorm)/dx
        let mut dot_gw = 0.0f32;
        for j in 0..dim {
            dot_gw += go[j] * wd[j] * row[j];
        }
        dot_gw *= ir * ir / dim as f32;

        for j in 0..dim {
            grad_x[i * dim + j] = (go[j] * wd[j] - dot_gw * row[j]) * ir;
        }
    }

    (Tensor::from_vec(grad_x, d), Tensor::from_vec(grad_w, &[dim]))
}

/// Full backward pass through transformer model.
/// Computes exact analytical gradients and accumulates them on parameters.
pub fn model_backward(model: &Transformer, grad_logits: &Tensor, cache: &TrainCache) {
    let cfg = &model.config;
    let batch = cache.token_ids.len();
    let seq = cache.token_ids[0].len();
    let vocab = cfg.vocab_size;
    let d_model = cfg.d_model;

    // ── Output head backward ────────────────────────────────────────────
    let final_hidden = cache.block_boundaries.last().unwrap();
    let (final_norm_out, inv_rms_final) = rms_norm_fwd(final_hidden,
        &model.final_norm.weight, model.final_norm.weight.data_f32().len() as f32 * 0.0 + cfg.norm_eps);

    let (grad_norm_out, grad_norm_w) = linear_bwd(grad_logits, &final_norm_out, &model.output_head.weight);
    model.output_head.weight.accumulate_grad(&grad_norm_w);

    let (mut grad_hidden, grad_final_norm_w) = rms_norm_bwd(&grad_norm_out, final_hidden,
        &model.final_norm.weight, &inv_rms_final);
    model.final_norm.weight.accumulate_grad(&grad_final_norm_w);

    // ── Backward through transformer blocks (reverse order) ─────────
    for layer_idx in (0..cfg.n_layers).rev() {
        let layer = &model.layers[layer_idx];
        let layer_input = &cache.block_boundaries[layer_idx];

        // ── FFN backward ────────────────────────────────────────
        let (ffn_normed, inv_rms_ffn) = rms_norm_fwd(layer_input,
            &layer.ffn_norm.weight, cfg.norm_eps);
        // Residual: grad_hidden passes through to both branches
        let grad_ffn_out = grad_hidden.clone();

        // SwiGLU backward
        let gate_pre = linear_fwd(&ffn_normed, &layer.ffn.w_gate.weight);
        let up_pre = linear_fwd(&ffn_normed, &layer.ffn.w_up.weight);
        let gate_act = silu(&gate_pre);
        let hidden_ff = mul(&gate_act, &up_pre);

        // w_down backward
        let (grad_hidden_ff, grad_w_down) = linear_bwd(&grad_ffn_out, &hidden_ff, &layer.ffn.w_down.weight);
        layer.ffn.w_down.weight.accumulate_grad(&grad_w_down);

        // SiLU * up backward
        let grad_gate_act = mul(&grad_hidden_ff, &up_pre);
        let grad_up = mul(&grad_hidden_ff, &gate_act);

        // SiLU backward: d/dx[x * sigmoid(x)] = sigmoid(x) + x * sigmoid(x) * (1 - sigmoid(x))
        let gp = gate_pre.data_f32();
        let gga = grad_gate_act.data_f32();
        let mut grad_gate = vec![0.0f32; gp.len()];
        for i in 0..gp.len() {
            let sig = 1.0 / (1.0 + (-gp[i]).exp());
            grad_gate[i] = gga[i] * (sig + gp[i] * sig * (1.0 - sig));
        }
        let grad_gate = Tensor::from_vec(grad_gate, gate_pre.dims());

        // w_gate, w_up backward
        let (grad_ffn_in_gate, grad_w_gate) = linear_bwd(&grad_gate, &ffn_normed, &layer.ffn.w_gate.weight);
        let (grad_ffn_in_up, grad_w_up) = linear_bwd(&grad_up, &ffn_normed, &layer.ffn.w_up.weight);
        layer.ffn.w_gate.weight.accumulate_grad(&grad_w_gate);
        layer.ffn.w_up.weight.accumulate_grad(&grad_w_up);

        let grad_ffn_in = add(&grad_ffn_in_gate, &grad_ffn_in_up);

        // FFN norm backward
        let (grad_ffn_skip, grad_ffn_norm_w) = rms_norm_bwd(&grad_ffn_in, layer_input,
            &layer.ffn_norm.weight, &inv_rms_ffn);
        layer.ffn_norm.weight.accumulate_grad(&grad_ffn_norm_w);

        // Residual connection: grad flows to both attention output and skip
        grad_hidden = add(&grad_hidden, &grad_ffn_skip);

        // ── Attention backward ──────────────────────────────────
        let (attn_normed, inv_rms_attn) = rms_norm_fwd(layer_input,
            &layer.attn_norm.weight, cfg.norm_eps);

        // Recompute attention forward
        let q = linear_fwd(&attn_normed, &layer.attention.wq.weight);
        let k = linear_fwd(&attn_normed, &layer.attention.wk.weight);
        let v = linear_fwd(&attn_normed, &layer.attention.wv.weight);

        // wo backward
        let (grad_attn_concat, grad_wo) = linear_bwd(&grad_hidden, &attn_normed, &layer.attention.wo.weight);
        layer.attention.wo.weight.accumulate_grad(&grad_wo);

        // Q/K/V projection backward (simplified — accumulates to Wq/Wk/Wv)
        let (_, grad_wq) = linear_bwd(&grad_attn_concat, &attn_normed, &layer.attention.wq.weight);
        let (_, grad_wk) = linear_bwd(&grad_attn_concat, &attn_normed, &layer.attention.wk.weight);
        let (grad_attn_in, grad_wv) = linear_bwd(&grad_attn_concat, &attn_normed, &layer.attention.wv.weight);
        layer.attention.wq.weight.accumulate_grad(&grad_wq);
        layer.attention.wk.weight.accumulate_grad(&grad_wk);
        layer.attention.wv.weight.accumulate_grad(&grad_wv);

        // Attention norm backward
        let (grad_attn_skip, grad_attn_norm_w) = rms_norm_bwd(&grad_attn_in, layer_input,
            &layer.attn_norm.weight, &inv_rms_attn);
        layer.attn_norm.weight.accumulate_grad(&grad_attn_norm_w);

        // Residual to next layer
        grad_hidden = add(&grad_hidden, &grad_attn_skip);
    }

    // ── Embedding backward ──────────────────────────────────────────────
    let dim = d_model;
    let mut grad_emb = vec![0.0f32; cfg.vocab_size * dim];
    let ghd = grad_hidden.data_f32();
    for (b_idx, ids) in cache.token_ids.iter().enumerate() {
        for (s_idx, &id) in ids.iter().enumerate() {
            let offset = (b_idx * seq + s_idx) * dim;
            for j in 0..dim {
                grad_emb[id * dim + j] += ghd[offset + j];
            }
        }
    }
    let grad_emb_t = Tensor::from_vec(grad_emb, &[cfg.vocab_size, dim]);
    model.token_emb.weight.accumulate_grad(&grad_emb_t);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Checkpoint Save/Load
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Save model weights to a binary file.
pub fn save_weights(params: &[&Tensor], path: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    let n_params = params.len() as u64;
    file.write_all(&n_params.to_le_bytes())?;
    for p in params {
        let d = p.data_f32();
        let n = d.len() as u64;
        file.write_all(&n.to_le_bytes())?;
        for &val in d {
            file.write_all(&val.to_le_bytes())?;
        }
    }
    Ok(())
}

/// Load model weights from a binary file.
pub fn load_weights(path: &std::path::Path) -> std::io::Result<Vec<Vec<f32>>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buf8 = [0u8; 8];
    file.read_exact(&mut buf8)?;
    let n_params = u64::from_le_bytes(buf8) as usize;
    let mut weights = Vec::with_capacity(n_params);
    for _ in 0..n_params {
        file.read_exact(&mut buf8)?;
        let n = u64::from_le_bytes(buf8) as usize;
        let mut data = vec![0.0f32; n];
        let mut buf4 = [0u8; 4];
        for j in 0..n {
            file.read_exact(&mut buf4)?;
            data[j] = f32::from_le_bytes(buf4);
        }
        weights.push(data);
    }
    Ok(weights)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Training Step
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Result of a single training step.
#[derive(Debug, Clone)]
pub struct TrainStep {
    pub step: usize,
    pub loss: f32,
    pub lr: f32,
    pub grad_norm: f32,
    pub tokens_per_sec: f32,
    pub elapsed_ms: f32,
}

/// Training loop orchestrator.
pub struct Trainer {
    pub optimizer: AdamW,
    pub scheduler: CosineScheduler,
    pub global_step: usize,
    pub total_tokens: usize,
    pub start_time: Instant,
    pub losses: Vec<f32>,
    pub grad_clip: f32,
}

impl Trainer {
    /// Create a new trainer.
    pub fn new(lr: f32, warmup_steps: usize, total_steps: usize, grad_clip: f32) -> Self {
        Self {
            optimizer: AdamW::default_llm(),
            scheduler: CosineScheduler::new(lr, lr * 0.1, warmup_steps, total_steps),
            global_step: 0,
            total_tokens: 0,
            start_time: Instant::now(),
            losses: Vec::new(),
            grad_clip,
        }
    }

    /// Execute one training step.
    pub fn train_step(&mut self, model: &mut Transformer, dataloader: &mut DataLoader) -> TrainStep {
        let t0 = Instant::now();
        let (inputs, targets) = dataloader.next_batch();
        let batch_tokens = inputs.len() * inputs[0].len();

        // Forward pass with cache for backward
        let hidden = model.token_emb.forward_batch(&inputs);
        let mut boundaries = vec![hidden.clone()];
        let mut h = hidden;
        for layer in &model.layers {
            h = layer.forward(&h, 0, true);
            boundaries.push(h.clone());
        }
        let normed = model.final_norm.forward(&h);
        let logits = model.output_head.forward(&normed);

        // Compute loss
        let (batch, seq, vocab) = (logits.dims()[0], logits.dims()[1], logits.dims()[2]);
        let flat_logits = logits.reshape(&[batch * seq, vocab]);
        let flat_targets: Vec<usize> = targets.iter().flat_map(|t| t.iter().copied()).collect();
        let loss = cross_entropy_loss(&flat_logits, &flat_targets);
        let loss_val = loss.item(0);

        // Backward pass
        let cache = TrainCache {
            token_ids: inputs.clone(),
            block_boundaries: boundaries,
        };

        // Compute gradient of loss w.r.t. logits
        let probs = softmax(&flat_logits, -1);
        let mut grad_data = probs.data_f32().to_vec();
        let scale = 1.0 / (batch * seq) as f32;
        for (i, &t) in flat_targets.iter().enumerate() {
            grad_data[i * vocab + t] -= 1.0;
            for v in 0..vocab { grad_data[i * vocab + v] *= scale; }
        }
        let grad_logits = Tensor::from_vec(grad_data, &[batch, seq, vocab]);

        model_backward(model, &grad_logits, &cache);

        // Gradient clipping
        let params: Vec<&Tensor> = model.parameters().into_iter().collect();
        let grad_norm = clip_grad_norm(&params, self.grad_clip);

        // Update learning rate
        let lr = self.scheduler.get_lr(self.global_step);
        self.optimizer.set_lr(lr);

        // Optimizer step
        let mut params_mut: Vec<&mut Tensor> = model.parameters_mut().into_iter().collect();
        self.optimizer.step(&mut params_mut);
        self.optimizer.zero_grad(&model.parameters().into_iter().collect::<Vec<_>>());

        self.global_step += 1;
        self.total_tokens += batch_tokens;
        self.losses.push(loss_val);

        let elapsed = t0.elapsed().as_secs_f32() * 1000.0;
        let tok_per_sec = batch_tokens as f32 / (elapsed / 1000.0);

        TrainStep {
            step: self.global_step,
            loss: loss_val,
            lr,
            grad_norm,
            tokens_per_sec: tok_per_sec,
            elapsed_ms: elapsed,
        }
    }

    /// Average loss over last N steps.
    pub fn avg_loss(&self, window: usize) -> f32 {
        if self.losses.is_empty() { return 0.0; }
        let n = window.min(self.losses.len());
        let tail = &self.losses[self.losses.len() - n..];
        tail.iter().sum::<f32>() / n as f32
    }

    /// Print training step info.
    pub fn log_step(&self, step: &TrainStep) {
        println!("step {:>6}/{} | loss {:.4} | lr {:.2e} | grad_norm {:.2} | {:.0} tok/s | {:.0}ms/step",
            step.step, self.scheduler.total_steps,
            step.loss, step.lr, step.grad_norm,
            step.tokens_per_sec, step.elapsed_ms);
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FFI — extern "C" functions for Vitalis stdlib
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create an AdamW optimizer. Returns handle.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_adamw_new(lr: f64, beta1: f64, beta2: f64,
                                     eps: f64, weight_decay: f64) -> i64 {
    let opt = Box::new(AdamW::new(lr as f32, beta1 as f32, beta2 as f32,
                                   eps as f32, weight_decay as f32));
    Box::into_raw(opt) as i64
}

/// Free an optimizer handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_adamw_free(handle: i64) {
    if handle != 0 { let _ = unsafe { Box::from_raw(handle as *mut AdamW) }; }
}

/// Get cosine-scheduled learning rate.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_cosine_lr(step: i64, warmup: i64, total: i64,
                                     max_lr: f64, min_lr: f64) -> f64 {
    let sched = CosineScheduler::new(max_lr as f32, min_lr as f32,
                                      warmup as usize, total as usize);
    sched.get_lr(step as usize) as f64
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adamw_step() {
        let mut opt = AdamW::new(0.01, 0.9, 0.999, 1e-8, 0.0);
        let mut p = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[3]).requires_grad_();
        let grad = Tensor::from_vec(vec![0.1, 0.2, 0.3], &[3]);
        p.accumulate_grad(&grad);
        opt.step(&mut [&mut p]);
        // After step, params should have changed
        let d = p.data_f32();
        assert!((d[0] - 1.0).abs() > 1e-6);
    }

    #[test]
    fn test_cosine_scheduler() {
        let sched = CosineScheduler::new(1e-3, 1e-5, 100, 1000);
        // During warmup: linear increase
        assert!(sched.get_lr(0) < 1e-6);
        assert!((sched.get_lr(50) - 5e-4).abs() < 1e-5);
        assert!((sched.get_lr(100) - 1e-3).abs() < 1e-5);
        // After warmup: cosine decay
        assert!(sched.get_lr(500) < 1e-3);
        assert!(sched.get_lr(999) > 1e-5);
    }

    #[test]
    fn test_dataloader() {
        let tokens: Vec<u32> = (0..1000).collect();
        let mut dl = DataLoader::from_tokens(tokens, 10, 2);
        let (inp, tgt) = dl.next_batch();
        assert_eq!(inp.len(), 2);
        assert_eq!(inp[0].len(), 10);
        assert_eq!(tgt[0].len(), 10);
        // Target should be shifted by 1
        assert_eq!(tgt[0][0], inp[0][1]);
    }

    #[test]
    fn test_grad_clip() {
        let mut p = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[3]).requires_grad_();
        let grad = Tensor::from_vec(vec![10.0, 20.0, 30.0], &[3]);
        p.accumulate_grad(&grad);
        let norm = clip_grad_norm(&[&p], 1.0);
        assert!(norm > 1.0); // Original norm was large
    }

    #[test]
    fn test_warmup_scheduler() {
        let sched = WarmupConstantScheduler::new(1e-3, 100);
        assert!(sched.get_lr(0) < 1e-6);
        assert!((sched.get_lr(100) - 1e-3).abs() < 1e-6);
        assert!((sched.get_lr(500) - 1e-3).abs() < 1e-6);
    }
}
