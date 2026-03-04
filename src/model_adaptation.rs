//! Model Adaptation — LoRA, QLoRA, prefix tuning, and adapter layers.
//!
//! Provides parameter-efficient fine-tuning (PEFT) methods for adapting
//! pre-trained models with minimal additional parameters.

use std::sync::Mutex;
use std::collections::HashMap;

// ── LoRA (Low-Rank Adaptation) ──────────────────────────────────────────

/// LoRA adapter for a single weight matrix.
/// W' = W + (alpha/r) * A @ B where A: [in, r], B: [r, out]
#[derive(Debug, Clone)]
pub struct LoRAAdapter {
    pub rank: usize,
    pub alpha: f64,
    pub in_features: usize,
    pub out_features: usize,
    pub a: Vec<f64>,  // [in_features, rank] — initialized from Kaiming
    pub b: Vec<f64>,  // [rank, out_features] — initialized to zero
    pub dropout: f64,
    pub scaling: f64,
}

impl LoRAAdapter {
    pub fn new(in_features: usize, out_features: usize, rank: usize, alpha: f64, dropout: f64) -> Self {
        let scaling = alpha / rank as f64;

        // Kaiming initialization for A
        let std_dev = (2.0 / in_features as f64).sqrt();
        let mut state: u64 = 0xDEAD_BEEF_CAFE;
        let mut rng = || -> f64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let u1 = (state as f64 / u64::MAX as f64).max(1e-10);
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let u2 = state as f64 / u64::MAX as f64;
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() * std_dev
        };

        let a: Vec<f64> = (0..in_features * rank).map(|_| rng()).collect();
        let b = vec![0.0; rank * out_features]; // Zero init for B

        LoRAAdapter {
            rank, alpha, in_features, out_features,
            a, b, dropout, scaling,
        }
    }

    /// Forward pass: compute the LoRA delta for input x.
    /// delta = x @ A @ B * scaling
    pub fn forward(&self, x: &[f64], batch_size: usize) -> Vec<f64> {
        // x @ A → [batch, rank]
        let mut intermediate = vec![0.0; batch_size * self.rank];
        for b in 0..batch_size {
            for r in 0..self.rank {
                let mut sum = 0.0;
                for i in 0..self.in_features {
                    sum += x[b * self.in_features + i] * self.a[i * self.rank + r];
                }
                intermediate[b * self.rank + r] = sum;
            }
        }

        // intermediate @ B → [batch, out_features]
        let mut output = vec![0.0; batch_size * self.out_features];
        for b in 0..batch_size {
            for o in 0..self.out_features {
                let mut sum = 0.0;
                for r in 0..self.rank {
                    sum += intermediate[b * self.rank + r] * self.b[r * self.out_features + o];
                }
                output[b * self.out_features + o] = sum * self.scaling;
            }
        }
        output
    }

    /// Number of trainable parameters (only A and B).
    pub fn num_params(&self) -> usize {
        self.in_features * self.rank + self.rank * self.out_features
    }

    /// Merge LoRA weights into base weight matrix (for inference).
    pub fn merge_into(&self, base_weight: &mut [f64]) {
        // base_weight += scaling * A @ B
        for i in 0..self.in_features {
            for o in 0..self.out_features {
                let mut sum = 0.0;
                for r in 0..self.rank {
                    sum += self.a[i * self.rank + r] * self.b[r * self.out_features + o];
                }
                base_weight[i * self.out_features + o] += sum * self.scaling;
            }
        }
    }
}

// ── QLoRA (Quantized LoRA) ──────────────────────────────────────────────

/// QLoRA: LoRA with 4-bit quantized base weights.
#[derive(Debug, Clone)]
pub struct QLoRAAdapter {
    pub lora: LoRAAdapter,
    pub quantized_base: Vec<u8>,   // 4-bit packed (2 values per byte)
    pub scale: Vec<f64>,           // Per-group quantization scale
    pub zero_point: Vec<f64>,      // Per-group zero point
    pub group_size: usize,
}

impl QLoRAAdapter {
    /// Create QLoRA from full-precision base weights.
    pub fn from_weights(
        base_weights: &[f64],
        in_features: usize,
        out_features: usize,
        rank: usize,
        alpha: f64,
        group_size: usize,
    ) -> Self {
        let lora = LoRAAdapter::new(in_features, out_features, rank, alpha, 0.0);

        // 4-bit quantization in groups
        let total = base_weights.len();
        let num_groups = (total + group_size - 1) / group_size;
        let mut quantized = vec![0u8; (total + 1) / 2];
        let mut scale = Vec::with_capacity(num_groups);
        let mut zero_point = Vec::with_capacity(num_groups);

        for g in 0..num_groups {
            let start = g * group_size;
            let end = (start + group_size).min(total);
            let group = &base_weights[start..end];

            let min_val = group.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_val = group.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let s = (max_val - min_val) / 15.0; // 4-bit = 16 levels
            let zp = if s > 0.0 { -min_val / s } else { 0.0 };

            scale.push(s);
            zero_point.push(zp);

            for (i, &val) in group.iter().enumerate() {
                let q = if s > 0.0 {
                    ((val / s + zp).round().clamp(0.0, 15.0)) as u8
                } else {
                    0u8
                };
                let idx = start + i;
                if idx % 2 == 0 {
                    quantized[idx / 2] = q;
                } else {
                    quantized[idx / 2] |= q << 4;
                }
            }
        }

        QLoRAAdapter { lora, quantized_base: quantized, scale, zero_point, group_size }
    }

    /// Dequantize base weights.
    pub fn dequantize(&self, out: &mut [f64]) {
        let total = out.len();
        for i in 0..total {
            let byte = self.quantized_base[i / 2];
            let q = if i % 2 == 0 { byte & 0x0F } else { (byte >> 4) & 0x0F };
            let g = i / self.group_size;
            let s = self.scale.get(g).copied().unwrap_or(1.0);
            let zp = self.zero_point.get(g).copied().unwrap_or(0.0);
            out[i] = s * (q as f64 - zp);
        }
    }

    /// Forward: dequantize + base matmul + LoRA delta.
    pub fn forward(&self, x: &[f64], batch_size: usize) -> Vec<f64> {
        let total = self.lora.in_features * self.lora.out_features;
        let mut base = vec![0.0; total];
        self.dequantize(&mut base);

        // x @ base_weight
        let mut output = vec![0.0; batch_size * self.lora.out_features];
        for b in 0..batch_size {
            for o in 0..self.lora.out_features {
                let mut sum = 0.0;
                for i in 0..self.lora.in_features {
                    sum += x[b * self.lora.in_features + i] * base[i * self.lora.out_features + o];
                }
                output[b * self.lora.out_features + o] = sum;
            }
        }

        // Add LoRA delta
        let delta = self.lora.forward(x, batch_size);
        for i in 0..output.len() {
            output[i] += delta[i];
        }
        output
    }
}

// ── Prefix Tuning ───────────────────────────────────────────────────────

/// Prefix tuning: prepend learnable prefix vectors to attention keys/values.
#[derive(Debug, Clone)]
pub struct PrefixTuning {
    pub prefix_length: usize,
    pub d_model: usize,
    pub num_layers: usize,
    pub prefix_keys: Vec<f64>,   // [num_layers, prefix_length, d_model]
    pub prefix_values: Vec<f64>, // [num_layers, prefix_length, d_model]
}

impl PrefixTuning {
    pub fn new(prefix_length: usize, d_model: usize, num_layers: usize) -> Self {
        let std_dev = 0.02;
        let mut state: u64 = 0xFEED_FACE;
        let mut rng = || -> f64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let u1 = (state as f64 / u64::MAX as f64).max(1e-10);
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let u2 = state as f64 / u64::MAX as f64;
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() * std_dev
        };

        let n = num_layers * prefix_length * d_model;
        PrefixTuning {
            prefix_length, d_model, num_layers,
            prefix_keys: (0..n).map(|_| rng()).collect(),
            prefix_values: (0..n).map(|_| rng()).collect(),
        }
    }

    /// Get prefix keys for a specific layer.
    pub fn get_keys(&self, layer: usize) -> &[f64] {
        let start = layer * self.prefix_length * self.d_model;
        let end = start + self.prefix_length * self.d_model;
        &self.prefix_keys[start..end]
    }

    /// Get prefix values for a specific layer.
    pub fn get_values(&self, layer: usize) -> &[f64] {
        let start = layer * self.prefix_length * self.d_model;
        let end = start + self.prefix_length * self.d_model;
        &self.prefix_values[start..end]
    }

    pub fn num_params(&self) -> usize {
        2 * self.num_layers * self.prefix_length * self.d_model
    }
}

// ── Adapter Layer ───────────────────────────────────────────────────────

/// Bottleneck adapter layer (Houlsby et al.).
/// down_project → nonlinearity → up_project + residual
#[derive(Debug, Clone)]
pub struct AdapterLayer {
    pub d_model: usize,
    pub bottleneck: usize,
    pub w_down: Vec<f64>, // [d_model, bottleneck]
    pub w_up: Vec<f64>,   // [bottleneck, d_model]
    pub bias_down: Vec<f64>,
    pub bias_up: Vec<f64>,
}

impl AdapterLayer {
    pub fn new(d_model: usize, bottleneck: usize) -> Self {
        let scale = (2.0 / d_model as f64).sqrt();
        let mut state: u64 = 0x1234_5678;
        let mut rng = || -> f64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as f64 / u64::MAX as f64) * 2.0 * scale - scale
        };
        AdapterLayer {
            d_model, bottleneck,
            w_down: (0..d_model * bottleneck).map(|_| rng()).collect(),
            w_up: (0..bottleneck * d_model).map(|_| rng()).collect(),
            bias_down: vec![0.0; bottleneck],
            bias_up: vec![0.0; d_model],
        }
    }

    /// Forward: x → down → relu → up + x
    pub fn forward(&self, x: &[f64], batch_size: usize) -> Vec<f64> {
        let mut output = vec![0.0; batch_size * self.d_model];

        for b in 0..batch_size {
            // Down projection
            let mut hidden = vec![0.0; self.bottleneck];
            for j in 0..self.bottleneck {
                let mut sum = self.bias_down[j];
                for i in 0..self.d_model {
                    sum += x[b * self.d_model + i] * self.w_down[i * self.bottleneck + j];
                }
                hidden[j] = sum.max(0.0); // ReLU
            }

            // Up projection + residual
            for i in 0..self.d_model {
                let mut sum = self.bias_up[i];
                for j in 0..self.bottleneck {
                    sum += hidden[j] * self.w_up[j * self.d_model + i];
                }
                output[b * self.d_model + i] = sum + x[b * self.d_model + i]; // Residual
            }
        }
        output
    }

    pub fn num_params(&self) -> usize {
        self.d_model * self.bottleneck * 2 + self.bottleneck + self.d_model
    }
}

// ── FFI Interface ───────────────────────────────────────────────────────

static LORA_STORE: Mutex<Option<HashMap<i64, LoRAAdapter>>> = Mutex::new(None);

fn with_lora<R>(f: impl FnOnce(&mut HashMap<i64, LoRAAdapter>) -> R) -> R {
    let mut guard = LORA_STORE.lock().unwrap();
    if guard.is_none() { *guard = Some(HashMap::new()); }
    f(guard.as_mut().unwrap())
}

fn next_lora_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lora_create(in_feat: i64, out_feat: i64, rank: i64, alpha: f64) -> i64 {
    let adapter = LoRAAdapter::new(in_feat as usize, out_feat as usize, rank as usize, alpha, 0.0);
    let id = next_lora_id();
    with_lora(|s| s.insert(id, adapter));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lora_params(lora_id: i64) -> i64 {
    with_lora(|s| s.get(&lora_id).map_or(0, |a| a.num_params() as i64))
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lora_merge(lora_id: i64, base_weight: *mut f64, count: i64) {
    with_lora(|s| {
        if let Some(adapter) = s.get(&lora_id) {
            let w = unsafe { std::slice::from_raw_parts_mut(base_weight, count as usize) };
            adapter.merge_into(w);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_lora_free(lora_id: i64) {
    with_lora(|s| { s.remove(&lora_id); });
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lora_create() {
        let lora = LoRAAdapter::new(64, 64, 4, 1.0, 0.0);
        assert_eq!(lora.num_params(), 64 * 4 + 4 * 64);
    }

    #[test]
    fn test_lora_forward() {
        let lora = LoRAAdapter::new(8, 8, 2, 1.0, 0.0);
        let x = vec![1.0; 8]; // batch_size=1
        let delta = lora.forward(&x, 1);
        assert_eq!(delta.len(), 8);
        // B is zero-initialized, so delta should be zero initially
        assert!(delta.iter().all(|&v| v.abs() < 1e-10));
    }

    #[test]
    fn test_lora_merge() {
        let mut lora = LoRAAdapter::new(4, 4, 2, 1.0, 0.0);
        // Set B to non-zero
        for b in lora.b.iter_mut() {
            *b = 0.1;
        }
        let mut base = vec![1.0; 16]; // 4x4
        lora.merge_into(&mut base);
        // Base weights should have changed
        assert!(base.iter().any(|&v| (v - 1.0).abs() > 1e-10));
    }

    #[test]
    fn test_lora_scaling() {
        let lora = LoRAAdapter::new(8, 8, 4, 2.0, 0.0);
        assert!((lora.scaling - 0.5).abs() < 1e-10); // alpha/rank = 2/4
    }

    #[test]
    fn test_qlora_create() {
        let weights = vec![0.5; 16]; // 4x4
        let qlora = QLoRAAdapter::from_weights(&weights, 4, 4, 2, 1.0, 8);
        assert!(!qlora.quantized_base.is_empty());
    }

    #[test]
    fn test_qlora_roundtrip() {
        let weights: Vec<f64> = (0..16).map(|i| i as f64 * 0.1).collect();
        let qlora = QLoRAAdapter::from_weights(&weights, 4, 4, 2, 1.0, 8);
        let mut dequantized = vec![0.0; 16];
        qlora.dequantize(&mut dequantized);
        // Should be approximately equal (quantization noise)
        for (orig, deq) in weights.iter().zip(dequantized.iter()) {
            assert!((orig - deq).abs() < 0.15, "orig={}, deq={}", orig, deq);
        }
    }

    #[test]
    fn test_qlora_forward() {
        let weights = vec![0.1; 16]; // 4x4
        let qlora = QLoRAAdapter::from_weights(&weights, 4, 4, 2, 1.0, 8);
        let x = vec![1.0; 4]; // batch_size=1
        let output = qlora.forward(&x, 1);
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn test_prefix_tuning() {
        let prefix = PrefixTuning::new(4, 16, 6);
        assert_eq!(prefix.num_params(), 2 * 6 * 4 * 16);
        let keys = prefix.get_keys(0);
        assert_eq!(keys.len(), 4 * 16);
        let values = prefix.get_values(5);
        assert_eq!(values.len(), 4 * 16);
    }

    #[test]
    fn test_adapter_layer() {
        let adapter = AdapterLayer::new(16, 4);
        let x = vec![0.5; 16]; // batch_size=1
        let output = adapter.forward(&x, 1);
        assert_eq!(output.len(), 16);
        // With residual, output should not be zero
        assert!(output.iter().any(|&v| v.abs() > 0.01));
    }

    #[test]
    fn test_adapter_residual() {
        let adapter = AdapterLayer::new(8, 2);
        let x = vec![1.0; 8];
        let output = adapter.forward(&x, 1);
        // Due to residual connection, output > input for positive weights
        assert!(output.iter().any(|&v| v >= 1.0));
    }

    #[test]
    fn test_adapter_params() {
        let adapter = AdapterLayer::new(16, 4);
        // 16*4 + 4*16 + 4 + 16 = 148
        assert_eq!(adapter.num_params(), 16 * 4 * 2 + 4 + 16);
    }

    #[test]
    fn test_ffi_lora() {
        let id = vitalis_lora_create(32, 32, 4, 1.0);
        assert!(id > 0);
        let params = vitalis_lora_params(id);
        assert_eq!(params, (32 * 4 + 4 * 32) as i64);
        vitalis_lora_free(id);
    }

    #[test]
    fn test_ffi_lora_merge() {
        let id = vitalis_lora_create(4, 4, 2, 1.0);
        let mut base = vec![1.0f64; 16];
        vitalis_lora_merge(id, base.as_mut_ptr(), 16);
        // B is zero-init, so base should be unchanged
        assert!(base.iter().all(|&v| (v - 1.0).abs() < 1e-10));
    }

    #[test]
    fn test_prefix_tuning_layers() {
        let prefix = PrefixTuning::new(8, 32, 12);
        for layer in 0..12 {
            let k = prefix.get_keys(layer);
            let v = prefix.get_values(layer);
            assert_eq!(k.len(), 8 * 32);
            assert_eq!(v.len(), 8 * 32);
        }
    }

    #[test]
    fn test_lora_batch_forward() {
        let mut lora = LoRAAdapter::new(4, 4, 2, 1.0, 0.0);
        for b in lora.b.iter_mut() { *b = 0.1; }
        let x = vec![1.0; 8]; // batch_size=2, in_features=4
        let delta = lora.forward(&x, 2);
        assert_eq!(delta.len(), 8);
    }
}
