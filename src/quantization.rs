//! Quantization — INT8, INT4, NF4 quantization for model compression.
//!
//! Provides weight quantization (symmetric/asymmetric), per-channel/per-group
//! quantization, NF4 (NormalFloat4) for QLoRA, and dynamic quantization.

use std::sync::Mutex;
use std::collections::HashMap;

// ── Quantization Types ──────────────────────────────────────────────────

/// Quantization mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantMode {
    /// 8-bit symmetric quantization.
    INT8Symmetric,
    /// 8-bit asymmetric quantization.
    INT8Asymmetric,
    /// 4-bit symmetric quantization.
    INT4Symmetric,
    /// 4-bit normal float (NF4) for QLoRA.
    NF4,
}

/// Quantized tensor metadata.
#[derive(Debug, Clone)]
pub struct QuantizedTensor {
    pub data: Vec<u8>,
    pub scales: Vec<f64>,
    pub zero_points: Vec<f64>,
    pub shape: Vec<usize>,
    pub mode: QuantMode,
    pub group_size: usize,
    pub num_elements: usize,
}

// ── NF4 Lookup Table ────────────────────────────────────────────────────

/// NF4 quantization levels (from QLoRA paper).
/// These are the 16 values of the normal float format optimized for
/// normally-distributed weights.
const NF4_LEVELS: [f64; 16] = [
    -1.0, -0.6961928009986877, -0.5250730514526367, -0.39491748809814453,
    -0.28444138169288635, -0.18477343022823334, -0.09105003625154495, 0.0,
    0.07958029955625534, 0.16093020141124725, 0.24611230194568634, 0.33791524171829224,
    0.44070982933044434, 0.5626170039176941, 0.7229568362236023, 1.0,
];

/// Find closest NF4 level via binary search.
fn quantize_nf4_value(value: f64) -> u8 {
    let mut best_idx = 0u8;
    let mut best_dist = f64::INFINITY;
    for (i, &level) in NF4_LEVELS.iter().enumerate() {
        let dist = (value - level).abs();
        if dist < best_dist {
            best_dist = dist;
            best_idx = i as u8;
        }
    }
    best_idx
}

/// Dequantize NF4 value.
fn dequantize_nf4(code: u8) -> f64 {
    NF4_LEVELS[(code & 0x0F) as usize]
}

// ── Quantization Functions ──────────────────────────────────────────────

/// Quantize weights using INT8 symmetric quantization.
pub fn quantize_int8_symmetric(weights: &[f64], group_size: usize) -> QuantizedTensor {
    let num_groups = (weights.len() + group_size - 1) / group_size;
    let mut data = vec![0u8; weights.len()];
    let mut scales = Vec::with_capacity(num_groups);

    for g in 0..num_groups {
        let start = g * group_size;
        let end = (start + group_size).min(weights.len());
        let group = &weights[start..end];

        let abs_max = group.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
        let scale = abs_max / 127.0;
        scales.push(scale);

        for (i, &val) in group.iter().enumerate() {
            let q = if scale > 0.0 {
                (val / scale).round().clamp(-128.0, 127.0) as i8
            } else {
                0i8
            };
            data[start + i] = q as u8;
        }
    }

    QuantizedTensor {
        data, scales,
        zero_points: vec![0.0; num_groups],
        shape: vec![weights.len()],
        mode: QuantMode::INT8Symmetric,
        group_size,
        num_elements: weights.len(),
    }
}

/// Quantize weights using INT8 asymmetric quantization.
pub fn quantize_int8_asymmetric(weights: &[f64], group_size: usize) -> QuantizedTensor {
    let num_groups = (weights.len() + group_size - 1) / group_size;
    let mut data = vec![0u8; weights.len()];
    let mut scales = Vec::with_capacity(num_groups);
    let mut zero_points = Vec::with_capacity(num_groups);

    for g in 0..num_groups {
        let start = g * group_size;
        let end = (start + group_size).min(weights.len());
        let group = &weights[start..end];

        let min_val = group.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = group.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let scale = (max_val - min_val) / 255.0;
        let zp = if scale > 0.0 { -min_val / scale } else { 0.0 };
        scales.push(scale);
        zero_points.push(zp);

        for (i, &val) in group.iter().enumerate() {
            let q = if scale > 0.0 {
                (val / scale + zp).round().clamp(0.0, 255.0) as u8
            } else {
                0u8
            };
            data[start + i] = q;
        }
    }

    QuantizedTensor {
        data, scales, zero_points,
        shape: vec![weights.len()],
        mode: QuantMode::INT8Asymmetric,
        group_size,
        num_elements: weights.len(),
    }
}

/// Quantize weights using INT4 symmetric quantization (packed 2 values per byte).
pub fn quantize_int4_symmetric(weights: &[f64], group_size: usize) -> QuantizedTensor {
    let num_groups = (weights.len() + group_size - 1) / group_size;
    let mut data = vec![0u8; (weights.len() + 1) / 2];
    let mut scales = Vec::with_capacity(num_groups);

    for g in 0..num_groups {
        let start = g * group_size;
        let end = (start + group_size).min(weights.len());
        let group = &weights[start..end];

        let abs_max = group.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
        let scale = abs_max / 7.0; // 4-bit signed: -8..7
        scales.push(scale);

        for (i, &val) in group.iter().enumerate() {
            let q = if scale > 0.0 {
                (val / scale).round().clamp(-8.0, 7.0) as i8
            } else {
                0i8
            };
            let idx = start + i;
            let packed = (q + 8) as u8; // Shift to 0..15
            if idx % 2 == 0 {
                data[idx / 2] = packed;
            } else {
                data[idx / 2] |= packed << 4;
            }
        }
    }

    QuantizedTensor {
        data, scales,
        zero_points: vec![0.0; num_groups],
        shape: vec![weights.len()],
        mode: QuantMode::INT4Symmetric,
        group_size,
        num_elements: weights.len(),
    }
}

/// Quantize weights using NF4 (NormalFloat4) for QLoRA.
pub fn quantize_nf4(weights: &[f64], group_size: usize) -> QuantizedTensor {
    let num_groups = (weights.len() + group_size - 1) / group_size;
    let mut data = vec![0u8; (weights.len() + 1) / 2];
    let mut scales = Vec::with_capacity(num_groups);

    for g in 0..num_groups {
        let start = g * group_size;
        let end = (start + group_size).min(weights.len());
        let group = &weights[start..end];

        let abs_max = group.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
        scales.push(abs_max);

        for (i, &val) in group.iter().enumerate() {
            let normalized = if abs_max > 0.0 { val / abs_max } else { 0.0 };
            let q = quantize_nf4_value(normalized);
            let idx = start + i;
            if idx % 2 == 0 {
                data[idx / 2] = q;
            } else {
                data[idx / 2] |= q << 4;
            }
        }
    }

    QuantizedTensor {
        data, scales,
        zero_points: vec![0.0; num_groups],
        shape: vec![weights.len()],
        mode: QuantMode::NF4,
        group_size,
        num_elements: weights.len(),
    }
}

// ── Dequantization Functions ────────────────────────────────────────────

/// Dequantize a quantized tensor back to f64.
pub fn dequantize(qt: &QuantizedTensor) -> Vec<f64> {
    let mut output = vec![0.0; qt.num_elements];
    match qt.mode {
        QuantMode::INT8Symmetric => {
            for i in 0..qt.num_elements {
                let g = i / qt.group_size;
                let q = qt.data[i] as i8 as f64;
                output[i] = q * qt.scales[g];
            }
        }
        QuantMode::INT8Asymmetric => {
            for i in 0..qt.num_elements {
                let g = i / qt.group_size;
                let q = qt.data[i] as f64;
                output[i] = qt.scales[g] * (q - qt.zero_points[g]);
            }
        }
        QuantMode::INT4Symmetric => {
            for i in 0..qt.num_elements {
                let g = i / qt.group_size;
                let byte = qt.data[i / 2];
                let packed = if i % 2 == 0 { byte & 0x0F } else { (byte >> 4) & 0x0F };
                let q = packed as i8 - 8; // Unshift from 0..15 to -8..7
                output[i] = q as f64 * qt.scales[g];
            }
        }
        QuantMode::NF4 => {
            for i in 0..qt.num_elements {
                let g = i / qt.group_size;
                let byte = qt.data[i / 2];
                let code = if i % 2 == 0 { byte & 0x0F } else { (byte >> 4) & 0x0F };
                output[i] = dequantize_nf4(code) * qt.scales[g];
            }
        }
    }
    output
}

/// Compute quantization error (RMSE).
pub fn quantization_error(original: &[f64], quantized: &QuantizedTensor) -> f64 {
    let dequantized = dequantize(quantized);
    let n = original.len() as f64;
    let mse = original.iter().zip(dequantized.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>() / n;
    mse.sqrt()
}

/// Compute compression ratio.
pub fn compression_ratio(mode: QuantMode, num_elements: usize) -> f64 {
    let original_bits = num_elements * 64; // f64 = 64 bits
    let compressed_bits = match mode {
        QuantMode::INT8Symmetric | QuantMode::INT8Asymmetric => num_elements * 8,
        QuantMode::INT4Symmetric | QuantMode::NF4 => num_elements * 4,
    };
    original_bits as f64 / compressed_bits as f64
}

// ── Dynamic Quantization ────────────────────────────────────────────────

/// Dynamically quantize activations during inference.
pub fn dynamic_quantize_int8(activations: &[f64]) -> (Vec<i8>, f64, f64) {
    let min_val = activations.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = activations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let scale = (max_val - min_val) / 255.0;
    let zero_point = if scale > 0.0 { -min_val / scale - 128.0 } else { 0.0 };

    let quantized: Vec<i8> = activations.iter().map(|&x| {
        if scale > 0.0 {
            (x / scale + zero_point + 128.0).round().clamp(0.0, 255.0) as u8 as i8
        } else {
            0i8
        }
    }).collect();

    (quantized, scale, zero_point)
}

/// Quantized matrix multiply (INT8 × INT8 → f64).
pub fn quantized_matmul_int8(
    a: &[i8], a_scale: f64,
    b: &[i8], b_scale: f64,
    m: usize, k: usize, n: usize,
) -> Vec<f64> {
    let mut output = vec![0.0; m * n];
    let combined_scale = a_scale * b_scale;

    for i in 0..m {
        for j in 0..n {
            let mut acc: i32 = 0;
            for kk in 0..k {
                acc += a[i * k + kk] as i32 * b[kk * n + j] as i32;
            }
            output[i * n + j] = acc as f64 * combined_scale;
        }
    }
    output
}

// ── FFI Interface ───────────────────────────────────────────────────────

static QUANT_STORE: Mutex<Option<HashMap<i64, QuantizedTensor>>> = Mutex::new(None);

fn with_quant<R>(f: impl FnOnce(&mut HashMap<i64, QuantizedTensor>) -> R) -> R {
    let mut guard = QUANT_STORE.lock().unwrap();
    if guard.is_none() { *guard = Some(HashMap::new()); }
    f(guard.as_mut().unwrap())
}

fn next_quant_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_quantize_int8(weights: *const f64, count: i64, group_size: i64) -> i64 {
    let w = unsafe { std::slice::from_raw_parts(weights, count as usize) };
    let qt = quantize_int8_symmetric(w, group_size as usize);
    let id = next_quant_id();
    with_quant(|s| s.insert(id, qt));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_quantize_int4(weights: *const f64, count: i64, group_size: i64) -> i64 {
    let w = unsafe { std::slice::from_raw_parts(weights, count as usize) };
    let qt = quantize_int4_symmetric(w, group_size as usize);
    let id = next_quant_id();
    with_quant(|s| s.insert(id, qt));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_quantize_nf4_ffi(weights: *const f64, count: i64, group_size: i64) -> i64 {
    let w = unsafe { std::slice::from_raw_parts(weights, count as usize) };
    let qt = quantize_nf4(w, group_size as usize);
    let id = next_quant_id();
    with_quant(|s| s.insert(id, qt));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_quantize_error(quant_id: i64, original: *const f64, count: i64) -> f64 {
    let w = unsafe { std::slice::from_raw_parts(original, count as usize) };
    with_quant(|s| {
        s.get(&quant_id).map_or(-1.0, |qt| quantization_error(w, qt))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_quantize_free(quant_id: i64) {
    with_quant(|s| { s.remove(&quant_id); });
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int8_symmetric() {
        let weights: Vec<f64> = (0..16).map(|i| (i as f64 - 8.0) * 0.1).collect();
        let qt = quantize_int8_symmetric(&weights, 16);
        let dq = dequantize(&qt);
        for (o, d) in weights.iter().zip(dq.iter()) {
            assert!((o - d).abs() < 0.01, "orig={}, deq={}", o, d);
        }
    }

    #[test]
    fn test_int8_asymmetric() {
        let weights = vec![0.1, 0.5, 0.9, 0.3];
        let qt = quantize_int8_asymmetric(&weights, 4);
        let dq = dequantize(&qt);
        for (o, d) in weights.iter().zip(dq.iter()) {
            assert!((o - d).abs() < 0.01);
        }
    }

    #[test]
    fn test_int4_symmetric() {
        let weights: Vec<f64> = (0..8).map(|i| (i as f64 - 4.0) * 0.2).collect();
        let qt = quantize_int4_symmetric(&weights, 8);
        let dq = dequantize(&qt);
        for (o, d) in weights.iter().zip(dq.iter()) {
            assert!((o - d).abs() < 0.2, "orig={}, deq={}", o, d);
        }
    }

    #[test]
    fn test_nf4_quantization() {
        let weights: Vec<f64> = (0..8).map(|i| (i as f64 - 4.0) * 0.1).collect();
        let qt = quantize_nf4(&weights, 8);
        let dq = dequantize(&qt);
        // NF4 has larger error tolerance
        for (o, d) in weights.iter().zip(dq.iter()) {
            assert!((o - d).abs() < 0.3, "orig={}, deq={}", o, d);
        }
    }

    #[test]
    fn test_nf4_levels() {
        // NF4 levels should be monotonically increasing
        for i in 1..NF4_LEVELS.len() {
            assert!(NF4_LEVELS[i] > NF4_LEVELS[i-1]);
        }
        // Should span [-1, 1]
        assert_eq!(NF4_LEVELS[0], -1.0);
        assert_eq!(NF4_LEVELS[15], 1.0);
    }

    #[test]
    fn test_compression_ratio() {
        assert!((compression_ratio(QuantMode::INT8Symmetric, 100) - 8.0).abs() < 1e-10);
        assert!((compression_ratio(QuantMode::INT4Symmetric, 100) - 16.0).abs() < 1e-10);
        assert!((compression_ratio(QuantMode::NF4, 100) - 16.0).abs() < 1e-10);
    }

    #[test]
    fn test_quantization_error() {
        let weights: Vec<f64> = (0..100).map(|i| (i as f64 - 50.0) * 0.01).collect();
        let qt_int8 = quantize_int8_symmetric(&weights, 100);
        let qt_int4 = quantize_int4_symmetric(&weights, 100);
        let err_int8 = quantization_error(&weights, &qt_int8);
        let err_int4 = quantization_error(&weights, &qt_int4);
        // INT8 should have lower error than INT4
        assert!(err_int8 < err_int4, "int8={}, int4={}", err_int8, err_int4);
    }

    #[test]
    fn test_dynamic_quantize() {
        let activations = vec![0.1, 0.5, 0.9, -0.3, -0.7];
        let (quantized, scale, _zp) = dynamic_quantize_int8(&activations);
        assert_eq!(quantized.len(), 5);
        assert!(scale > 0.0);
    }

    #[test]
    fn test_quantized_matmul() {
        let a: Vec<i8> = vec![1, 2, 3, 4]; // 2x2
        let b: Vec<i8> = vec![5, 6, 7, 8]; // 2x2
        let result = quantized_matmul_int8(&a, 0.1, &b, 0.1, 2, 2, 2);
        assert_eq!(result.len(), 4);
        // (1*5+2*7)*0.01 = 19*0.01 = 0.19
        assert!((result[0] - 0.19).abs() < 1e-10);
    }

    #[test]
    fn test_group_quantization() {
        let weights: Vec<f64> = (0..32).map(|i| (i as f64 - 16.0) * 0.1).collect();
        // Group size 8: should produce 4 groups
        let qt = quantize_int8_symmetric(&weights, 8);
        assert_eq!(qt.scales.len(), 4);
    }

    #[test]
    fn test_zero_weights() {
        let weights = vec![0.0; 16];
        let qt = quantize_int8_symmetric(&weights, 16);
        let dq = dequantize(&qt);
        assert!(dq.iter().all(|&v| v.abs() < 1e-10));
    }

    #[test]
    fn test_ffi_int8() {
        let weights = [0.5f64, -0.3, 0.7, -0.1];
        let id = vitalis_quantize_int8(weights.as_ptr(), 4, 4);
        assert!(id > 0);
        let err = vitalis_quantize_error(id, weights.as_ptr(), 4);
        assert!(err < 0.01);
        vitalis_quantize_free(id);
    }

    #[test]
    fn test_ffi_int4() {
        let weights = [0.5f64, -0.3, 0.7, -0.1];
        let id = vitalis_quantize_int4(weights.as_ptr(), 4, 4);
        assert!(id > 0);
        vitalis_quantize_free(id);
    }

    #[test]
    fn test_ffi_nf4() {
        let weights = [0.5f64, -0.3, 0.7, -0.1];
        let id = vitalis_quantize_nf4_ffi(weights.as_ptr(), 4, 4);
        assert!(id > 0);
        vitalis_quantize_free(id);
    }

    #[test]
    fn test_all_modes_roundtrip() {
        let weights: Vec<f64> = (0..32).map(|i| (i as f64 - 16.0) * 0.05).collect();
        for mode_fn in &[
            quantize_int8_symmetric as fn(&[f64], usize) -> QuantizedTensor,
            quantize_int8_asymmetric,
            quantize_int4_symmetric,
            quantize_nf4,
        ] {
            let qt = mode_fn(&weights, 8);
            let dq = dequantize(&qt);
            assert_eq!(dq.len(), 32);
            assert!(dq.iter().all(|v| v.is_finite()));
        }
    }
}
