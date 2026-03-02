//! Model Inference — Configuration, text generation, and sampling
//!
//! Complete inference pipeline for transformer language models:
//! model configuration presets, temperature/top-k/top-p sampling,
//! and autoregressive text generation. Ported from the Nova ML engine.
//!
//! # Example
//!
//! ```rust,ignore
//! use vitalis::model_inference::{ModelConfig, GenerateConfig, generate};
//! use vitalis::deep_learning::Transformer;
//! let config = ModelConfig::tiny_5m();
//! let model = Transformer::new(&config.to_transformer_config());
//! let gen_config = GenerateConfig::default();
//! let tokens = generate(&model, &[1], &gen_config);
//! ```

use crate::tensor_engine::{Tensor, Shape, matmul, transpose, softmax};
use crate::deep_learning::{Transformer, TransformerConfig};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Model Configuration
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Full model configuration including training hyperparameters.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    // Architecture
    pub name: String,
    pub vocab_size: usize,
    pub d_model: usize,
    pub n_layers: usize,
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub d_ff: usize,
    pub max_seq_len: usize,
    pub norm_eps: f32,
    pub rope_base: f32,

    // Training
    pub batch_size: usize,
    pub grad_accum_steps: usize,
    pub learning_rate: f32,
    pub min_lr: f32,
    pub warmup_steps: usize,
    pub total_steps: usize,
    pub weight_decay: f32,
    pub grad_clip: f32,
}

impl ModelConfig {
    /// ~1.8M params — fast iteration / debugging
    pub fn tiny_5m() -> Self {
        Self {
            name: "nova-tiny-5m".into(),
            vocab_size: 8000, d_model: 128, n_layers: 4, n_heads: 4,
            n_kv_heads: 4, d_ff: 344, max_seq_len: 512,
            norm_eps: 1e-5, rope_base: 10000.0,
            batch_size: 2, grad_accum_steps: 2,
            learning_rate: 1e-3, min_lr: 1e-4,
            warmup_steps: 500, total_steps: 50000,
            weight_decay: 0.1, grad_clip: 1.0,
        }
    }

    /// ~125M params — GPT-2 small equivalent
    pub fn small_125m() -> Self {
        Self {
            name: "nova-small-125m".into(),
            vocab_size: 32000, d_model: 768, n_layers: 12, n_heads: 12,
            n_kv_heads: 4, d_ff: 2048, max_seq_len: 2048,
            norm_eps: 1e-5, rope_base: 10000.0,
            batch_size: 8, grad_accum_steps: 4,
            learning_rate: 6e-4, min_lr: 6e-5,
            warmup_steps: 2000, total_steps: 100000,
            weight_decay: 0.1, grad_clip: 1.0,
        }
    }

    /// ~1B params — production scale
    pub fn medium_1b() -> Self {
        Self {
            name: "nova-medium-1b".into(),
            vocab_size: 32000, d_model: 2048, n_layers: 22, n_heads: 16,
            n_kv_heads: 4, d_ff: 5461, max_seq_len: 4096,
            norm_eps: 1e-5, rope_base: 500000.0,
            batch_size: 4, grad_accum_steps: 8,
            learning_rate: 3e-4, min_lr: 3e-5,
            warmup_steps: 2000, total_steps: 200000,
            weight_decay: 0.1, grad_clip: 1.0,
        }
    }

    /// ~3B params — large scale
    pub fn large_3b() -> Self {
        Self {
            name: "nova-large-3b".into(),
            vocab_size: 32000, d_model: 3200, n_layers: 26, n_heads: 32,
            n_kv_heads: 8, d_ff: 8640, max_seq_len: 4096,
            norm_eps: 1e-5, rope_base: 500000.0,
            batch_size: 2, grad_accum_steps: 16,
            learning_rate: 1.5e-4, min_lr: 1.5e-5,
            warmup_steps: 2000, total_steps: 300000,
            weight_decay: 0.1, grad_clip: 1.0,
        }
    }

    /// Convert to transformer config for model construction.
    pub fn to_transformer_config(&self) -> TransformerConfig {
        TransformerConfig {
            vocab_size: self.vocab_size,
            d_model: self.d_model,
            n_layers: self.n_layers,
            n_heads: self.n_heads,
            n_kv_heads: self.n_kv_heads,
            d_ff: self.d_ff,
            max_seq_len: self.max_seq_len,
            norm_eps: self.norm_eps,
            tie_weights: true,
        }
    }

    /// Estimate total parameter count.
    pub fn estimate_params(&self) -> usize {
        let e = self.vocab_size * self.d_model;
        let attn = self.d_model * self.d_model * 4; // Q, K, V, O
        let ffn = self.d_model * self.d_ff * 3; // gate, up, down (SwiGLU)
        let norm = self.d_model * 2; // attn_norm + ffn_norm
        let layer = attn + ffn + norm;
        e + layer * self.n_layers + self.d_model + e // emb + layers + final_norm + output
    }

    /// Estimate VRAM needed (bytes, FP32).
    pub fn estimate_vram_bytes(&self) -> usize {
        self.estimate_params() * 4
    }

    /// Print model summary.
    pub fn print_summary(&self) {
        let params = self.estimate_params();
        let (p_str, p_unit) = if params >= 1_000_000_000 {
            (params as f64 / 1e9, "B")
        } else if params >= 1_000_000 {
            (params as f64 / 1e6, "M")
        } else {
            (params as f64 / 1e3, "K")
        };
        println!("╔══════════════════════════════════════════════╗");
        println!("║  Model: {:<37}║", self.name);
        println!("╠══════════════════════════════════════════════╣");
        println!("║  Parameters: {:.1}{:<26}║", p_str, p_unit);
        println!("║  d_model:    {:<33}║", self.d_model);
        println!("║  n_layers:   {:<33}║", self.n_layers);
        println!("║  n_heads:    {:<33}║", self.n_heads);
        println!("║  n_kv_heads: {:<33}║", self.n_kv_heads);
        println!("║  d_ff:       {:<33}║", self.d_ff);
        println!("║  vocab_size: {:<33}║", self.vocab_size);
        println!("║  max_seq:    {:<33}║", self.max_seq_len);
        println!("║  VRAM (FP32):{:.1} MB{:<25}║", self.estimate_vram_bytes() as f64 / 1e6, "");
        println!("╚══════════════════════════════════════════════╝");
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Generation Configuration
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Configuration for text generation.
#[derive(Debug, Clone)]
pub struct GenerateConfig {
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_k: usize,
    pub top_p: f32,
    pub repetition_penalty: f32,
    pub eos_token: u32,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.8,
            top_k: 50,
            top_p: 0.9,
            repetition_penalty: 1.1,
            eos_token: 2,
        }
    }
}

impl GenerateConfig {
    /// Greedy (deterministic) generation.
    pub fn greedy() -> Self {
        Self { temperature: 0.0, top_k: 1, top_p: 1.0,
               repetition_penalty: 1.0, ..Default::default() }
    }

    /// Creative generation with high temperature.
    pub fn creative() -> Self {
        Self { temperature: 1.2, top_k: 100, top_p: 0.95,
               repetition_penalty: 1.2, ..Default::default() }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Sampling
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Simple xorshift RNG for sampling.
struct SampleRng(u32);
impl SampleRng {
    fn new(seed: u32) -> Self { Self(seed.max(1)) }
    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32) / (u32::MAX as f32)
    }
}

/// Sample a token from logits using temperature + top-k + top-p.
pub fn sample_token(logits: &[f32], config: &GenerateConfig,
                     generated: &[u32], rng_seed: u32) -> u32 {
    let vocab = logits.len();
    let mut scores = logits.to_vec();

    // Repetition penalty
    if config.repetition_penalty != 1.0 {
        for &token in generated {
            let idx = token as usize;
            if idx < vocab {
                if scores[idx] > 0.0 {
                    scores[idx] /= config.repetition_penalty;
                } else {
                    scores[idx] *= config.repetition_penalty;
                }
            }
        }
    }

    // Greedy
    if config.temperature <= 0.0 || config.top_k == 1 {
        return scores.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i as u32)
            .unwrap_or(0);
    }

    // Temperature scaling
    for s in scores.iter_mut() { *s /= config.temperature; }

    // Stable softmax
    let max_s = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut probs: Vec<(usize, f32)> = scores.iter().enumerate()
        .map(|(i, &s)| (i, (s - max_s).exp()))
        .collect();

    // Top-k filtering
    if config.top_k > 0 && config.top_k < vocab {
        probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        probs.truncate(config.top_k);
    }

    // Normalize
    let total: f32 = probs.iter().map(|p| p.1).sum();
    for p in probs.iter_mut() { p.1 /= total; }

    // Top-p (nucleus) filtering
    if config.top_p < 1.0 {
        probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let mut cumsum = 0.0;
        let mut cutoff = probs.len();
        for (i, &(_, prob)) in probs.iter().enumerate() {
            cumsum += prob;
            if cumsum >= config.top_p {
                cutoff = i + 1;
                break;
            }
        }
        probs.truncate(cutoff);

        // Re-normalize
        let total: f32 = probs.iter().map(|p| p.1).sum();
        for p in probs.iter_mut() { p.1 /= total; }
    }

    // Weighted random sampling
    let mut rng = SampleRng::new(rng_seed);
    let r = rng.next_f32();
    let mut cumulative = 0.0;
    for &(idx, prob) in &probs {
        cumulative += prob;
        if r <= cumulative { return idx as u32; }
    }
    probs.last().map(|&(i, _)| i as u32).unwrap_or(0)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Generation
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Generate tokens autoregressively.
pub fn generate(model: &Transformer, prompt_tokens: &[u32],
                config: &GenerateConfig) -> Vec<u32> {
    let mut generated: Vec<u32> = prompt_tokens.to_vec();
    let mut rng_seed = 42u32;

    for step in 0..config.max_tokens {
        let seq = &generated;
        let input_ids: Vec<Vec<usize>> = vec![seq.iter().map(|&t| t as usize).collect()];

        // Forward pass
        let logits = model.forward(&input_ids, 0);
        let dims = logits.dims();
        let vocab = *dims.last().unwrap();

        // Get logits for last position
        let all_data = logits.data_f32();
        let last_pos_start = (dims[1] - 1) * vocab;
        let last_logits = &all_data[last_pos_start..last_pos_start + vocab];

        // Sample
        rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
        let next_token = sample_token(last_logits, config, &generated, rng_seed);

        if next_token == config.eos_token { break; }
        generated.push(next_token);

        // Truncate if exceeding max sequence length
        if generated.len() >= model.config.max_seq_len {
            break;
        }
    }

    generated
}

/// Generate and return structured output.
#[derive(Debug)]
pub struct GenerationResult {
    pub tokens: Vec<u32>,
    pub num_generated: usize,
    pub prompt_len: usize,
    pub stopped_by_eos: bool,
}

pub fn generate_with_info(model: &Transformer, prompt_tokens: &[u32],
                           config: &GenerateConfig) -> GenerationResult {
    let prompt_len = prompt_tokens.len();
    let mut generated: Vec<u32> = prompt_tokens.to_vec();
    let mut stopped_by_eos = false;
    let mut rng_seed = 42u32;

    for _ in 0..config.max_tokens {
        let input_ids: Vec<Vec<usize>> = vec![generated.iter().map(|&t| t as usize).collect()];
        let logits = model.forward(&input_ids, 0);
        let dims = logits.dims();
        let vocab = *dims.last().unwrap();
        let all_data = logits.data_f32();
        let last_pos_start = (dims[1] - 1) * vocab;
        let last_logits = &all_data[last_pos_start..last_pos_start + vocab];

        rng_seed = rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
        let next_token = sample_token(last_logits, config, &generated, rng_seed);

        if next_token == config.eos_token {
            stopped_by_eos = true;
            break;
        }
        generated.push(next_token);
        if generated.len() >= model.config.max_seq_len { break; }
    }

    GenerationResult {
        num_generated: generated.len() - prompt_len,
        prompt_len,
        stopped_by_eos,
        tokens: generated,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FFI — extern "C" functions for Vitalis stdlib
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Get estimated parameter count for a preset model config.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_model_params(preset: i64) -> i64 {
    let cfg = match preset {
        0 => ModelConfig::tiny_5m(),
        1 => ModelConfig::small_125m(),
        2 => ModelConfig::medium_1b(),
        3 => ModelConfig::large_3b(),
        _ => return 0,
    };
    cfg.estimate_params() as i64
}

/// Get estimated VRAM in bytes for a preset.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_model_vram(preset: i64) -> i64 {
    let cfg = match preset {
        0 => ModelConfig::tiny_5m(),
        1 => ModelConfig::small_125m(),
        2 => ModelConfig::medium_1b(),
        3 => ModelConfig::large_3b(),
        _ => return 0,
    };
    cfg.estimate_vram_bytes() as i64
}

/// Sample a token from logits array (C pointer + length).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sample_token(logits_ptr: *const f32, vocab_size: i64,
                                               temperature: f64, top_k: i64, seed: i64) -> i64 {
    if logits_ptr.is_null() { return 0; }
    let logits = unsafe { std::slice::from_raw_parts(logits_ptr, vocab_size as usize) };
    let config = GenerateConfig {
        temperature: temperature as f32,
        top_k: top_k as usize,
        ..Default::default()
    };
    sample_token(logits, &config, &[], seed as u32) as i64
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_presets() {
        let tiny = ModelConfig::tiny_5m();
        assert_eq!(tiny.d_model, 128);
        assert!(tiny.estimate_params() > 1_000_000);

        let small = ModelConfig::small_125m();
        assert!(small.estimate_params() > 100_000_000);

        let medium = ModelConfig::medium_1b();
        assert!(medium.estimate_params() > 500_000_000);
    }

    #[test]
    fn test_greedy_sampling() {
        let logits = vec![0.1, 0.2, 0.9, 0.3, 0.05];
        let config = GenerateConfig::greedy();
        let token = sample_token(&logits, &config, &[], 42);
        assert_eq!(token, 2); // Index of max value
    }

    #[test]
    fn test_temperature_sampling() {
        let logits = vec![1.0, 2.0, 5.0, 0.5];
        let config = GenerateConfig {
            temperature: 0.3,
            top_k: 4,
            top_p: 1.0,
            ..Default::default()
        };
        // Should strongly favor index 2 (highest logit) with very low temp
        let mut counts = vec![0u32; 4];
        for seed in 1..1000 {
            let t = sample_token(&logits, &config, &[], seed);
            if (t as usize) < 4 { counts[t as usize] += 1; }
        }
        assert!(counts[2] > counts[0], "Expected token 2 to be sampled more than token 0, got {:?}", counts);
    }

    #[test]
    fn test_repetition_penalty() {
        let logits = vec![1.0, 1.0, 1.0, 1.0];
        let config = GenerateConfig {
            temperature: 0.0,
            repetition_penalty: 2.0,
            top_k: 1,
            ..Default::default()
        };
        // Token 0 was generated before → gets penalized
        let token = sample_token(&logits, &config, &[0], 42);
        assert_ne!(token, 0);
    }

    #[test]
    fn test_to_transformer_config() {
        let cfg = ModelConfig::tiny_5m();
        let tc = cfg.to_transformer_config();
        assert_eq!(tc.d_model, cfg.d_model);
        assert_eq!(tc.n_layers, cfg.n_layers);
        assert_eq!(tc.vocab_size, cfg.vocab_size);
    }

    #[test]
    fn test_generate_config_defaults() {
        let cfg = GenerateConfig::default();
        assert!(cfg.temperature > 0.0);
        assert!(cfg.top_k > 0);
        assert!(cfg.max_tokens > 0);
    }

    #[test]
    fn test_ffi_model_params() {
        let params = vitalis_model_params(0);
        assert!(params > 1_000_000);
        let invalid = vitalis_model_params(99);
        assert_eq!(invalid, 0);
    }
}
