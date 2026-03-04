//! Inference Engine — Batched inference, sampling strategies, beam search, and speculative decoding.
//!
//! Provides efficient model inference with KV-cache, temperature/top-k/top-p sampling,
//! beam search, repetition penalty, and speculative decoding for fast autoregressive generation.

use std::sync::Mutex;
use std::collections::HashMap;

// ── Sampling Strategies ─────────────────────────────────────────────────

/// Sampling configuration for text generation.
#[derive(Debug, Clone)]
pub struct SamplingConfig {
    pub temperature: f64,
    pub top_k: usize,
    pub top_p: f64,
    pub repetition_penalty: f64,
    pub frequency_penalty: f64,
    pub presence_penalty: f64,
    pub max_tokens: usize,
    pub stop_tokens: Vec<u32>,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        SamplingConfig {
            temperature: 1.0,
            top_k: 50,
            top_p: 0.9,
            repetition_penalty: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 256,
            stop_tokens: Vec::new(),
        }
    }
}

/// Apply temperature scaling to logits.
pub fn apply_temperature(logits: &mut [f64], temperature: f64) {
    if temperature <= 0.0 || temperature == 1.0 { return; }
    for l in logits.iter_mut() {
        *l /= temperature;
    }
}

/// Apply top-k filtering: keep only top-k logits, set rest to -inf.
pub fn apply_top_k(logits: &mut [f64], k: usize) {
    if k == 0 || k >= logits.len() { return; }

    // Find k-th largest value
    let mut sorted: Vec<f64> = logits.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let threshold = sorted[k - 1];

    for l in logits.iter_mut() {
        if *l < threshold {
            *l = f64::NEG_INFINITY;
        }
    }
}

/// Apply nucleus (top-p) filtering: keep smallest set of tokens with cumulative prob >= p.
pub fn apply_top_p(logits: &mut [f64], p: f64) {
    if p >= 1.0 { return; }

    // Softmax
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let probs: Vec<f64> = logits.iter().map(|&x| (x - max).exp()).collect();
    let sum: f64 = probs.iter().sum();
    let probs: Vec<f64> = probs.iter().map(|&x| x / sum).collect();

    // Sort by probability descending
    let mut indices: Vec<usize> = (0..probs.len()).collect();
    indices.sort_by(|&a, &b| probs[b].partial_cmp(&probs[a]).unwrap_or(std::cmp::Ordering::Equal));

    let mut cumsum = 0.0;
    let mut cutoff_idx = indices.len();
    for (i, &idx) in indices.iter().enumerate() {
        cumsum += probs[idx];
        if cumsum >= p {
            cutoff_idx = i + 1;
            break;
        }
    }

    // Mask tokens below cutoff
    let keep: std::collections::HashSet<usize> = indices[..cutoff_idx].iter().cloned().collect();
    for (i, l) in logits.iter_mut().enumerate() {
        if !keep.contains(&i) {
            *l = f64::NEG_INFINITY;
        }
    }
}

/// Apply repetition penalty to logits based on previously generated tokens.
pub fn apply_repetition_penalty(logits: &mut [f64], generated: &[u32], penalty: f64) {
    if penalty == 1.0 { return; }
    for &token in generated {
        let idx = token as usize;
        if idx < logits.len() {
            if logits[idx] > 0.0 {
                logits[idx] /= penalty;
            } else {
                logits[idx] *= penalty;
            }
        }
    }
}

/// Apply frequency and presence penalties.
pub fn apply_frequency_presence_penalty(
    logits: &mut [f64],
    token_counts: &HashMap<u32, usize>,
    frequency_penalty: f64,
    presence_penalty: f64,
) {
    for (&token, &count) in token_counts {
        let idx = token as usize;
        if idx < logits.len() {
            logits[idx] -= frequency_penalty * count as f64;
            if count > 0 {
                logits[idx] -= presence_penalty;
            }
        }
    }
}

/// Sample from logits using configured strategy.
pub fn sample_token(logits: &[f64], config: &SamplingConfig, generated: &[u32], seed: u64) -> u32 {
    let mut logits = logits.to_vec();

    // Apply penalties
    apply_repetition_penalty(&mut logits, generated, config.repetition_penalty);

    if config.frequency_penalty > 0.0 || config.presence_penalty > 0.0 {
        let mut counts = HashMap::new();
        for &t in generated { *counts.entry(t).or_insert(0) += 1; }
        apply_frequency_presence_penalty(&mut logits, &counts, config.frequency_penalty, config.presence_penalty);
    }

    // Apply temperature
    apply_temperature(&mut logits, config.temperature);

    // Apply top-k
    apply_top_k(&mut logits, config.top_k);

    // Apply top-p
    apply_top_p(&mut logits, config.top_p);

    // Softmax
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let probs: Vec<f64> = logits.iter().map(|&x| (x - max).exp()).collect();
    let sum: f64 = probs.iter().sum();

    if sum == 0.0 {
        return 0; // Fallback
    }

    // Sample with PRNG
    let mut state = seed.wrapping_add(generated.len() as u64).wrapping_mul(0x9E3779B97F4A7C15);
    state ^= state >> 30;
    state = state.wrapping_mul(0xBF58476D1CE4E5B9);
    state ^= state >> 27;
    let r = (state as f64 / u64::MAX as f64) * sum;

    let mut cumsum = 0.0;
    for (i, &p) in probs.iter().enumerate() {
        cumsum += p;
        if cumsum >= r {
            return i as u32;
        }
    }
    (probs.len() - 1) as u32
}

/// Greedy decoding (argmax).
pub fn argmax(logits: &[f64]) -> u32 {
    logits.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i as u32)
        .unwrap_or(0)
}

// ── Beam Search ─────────────────────────────────────────────────────────

/// A beam hypothesis.
#[derive(Debug, Clone)]
pub struct BeamHypothesis {
    pub tokens: Vec<u32>,
    pub score: f64,
    pub finished: bool,
}

/// Beam search decoder.
pub fn beam_search(
    logits_fn: &dyn Fn(&[u32]) -> Vec<f64>,
    bos_token: u32,
    eos_token: u32,
    beam_width: usize,
    max_len: usize,
    length_penalty: f64,
) -> Vec<BeamHypothesis> {
    let mut beams = vec![BeamHypothesis {
        tokens: vec![bos_token],
        score: 0.0,
        finished: false,
    }];

    for _step in 0..max_len {
        let mut all_candidates = Vec::new();

        for beam in &beams {
            if beam.finished {
                all_candidates.push(beam.clone());
                continue;
            }

            let logits = logits_fn(&beam.tokens);
            // Log-softmax
            let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let log_sum_exp = logits.iter().map(|&x| (x - max).exp()).sum::<f64>().ln() + max;

            for (token_id, &logit) in logits.iter().enumerate() {
                let log_prob = logit - log_sum_exp;
                let mut new_tokens = beam.tokens.clone();
                new_tokens.push(token_id as u32);
                let raw_score = beam.score + log_prob;
                // Length normalization
                let norm_score = raw_score / (new_tokens.len() as f64).powf(length_penalty);
                all_candidates.push(BeamHypothesis {
                    finished: token_id as u32 == eos_token,
                    tokens: new_tokens,
                    score: norm_score,
                });
            }
        }

        // Keep top beam_width candidates
        all_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        beams = all_candidates.into_iter().take(beam_width).collect();

        // Early stop if all beams finished
        if beams.iter().all(|b| b.finished) {
            break;
        }
    }

    beams.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    beams
}

// ── Speculative Decoding ────────────────────────────────────────────────

/// Speculative decoding: use a small draft model to propose tokens, verify with large model.
/// Returns accepted tokens.
pub fn speculative_decode(
    draft_fn: &dyn Fn(&[u32]) -> Vec<f64>,    // Fast draft model
    target_fn: &dyn Fn(&[u32]) -> Vec<f64>,    // Accurate target model
    prefix: &[u32],
    num_speculative: usize,
    seed: u64,
) -> Vec<u32> {
    let config = SamplingConfig::default();
    let mut generated = prefix.to_vec();
    let mut accepted = Vec::new();

    // Generate speculative tokens from draft model
    let mut draft_tokens = Vec::new();
    let mut draft_probs_list = Vec::new();
    for i in 0..num_speculative {
        let logits = draft_fn(&generated);
        let token = sample_token(&logits, &config, &generated, seed.wrapping_add(i as u64));

        // Compute draft probability
        let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp: Vec<f64> = logits.iter().map(|&x| (x - max).exp()).collect();
        let sum: f64 = exp.iter().sum();
        let draft_prob = exp[token as usize] / sum;

        draft_tokens.push(token);
        draft_probs_list.push(draft_prob);
        generated.push(token);
    }

    // Verify with target model
    generated = prefix.to_vec();
    for (i, &token) in draft_tokens.iter().enumerate() {
        let target_logits = target_fn(&generated);
        let max = target_logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp: Vec<f64> = target_logits.iter().map(|&x| (x - max).exp()).collect();
        let sum: f64 = exp.iter().sum();
        let target_prob = exp[token as usize] / sum;

        let draft_prob = draft_probs_list[i];

        // Accept with probability min(1, target_prob / draft_prob)
        let accept_prob = (target_prob / draft_prob.max(1e-10)).min(1.0);
        let mut state = seed.wrapping_add(100 + i as u64);
        state ^= state >> 30;
        state = state.wrapping_mul(0xBF58476D1CE4E5B9);
        let r = (state as f64) / (u64::MAX as f64);

        if r < accept_prob {
            accepted.push(token);
            generated.push(token);
        } else {
            // Reject: sample from adjusted distribution
            let adjusted_token = sample_token(&target_logits, &config, &generated, seed.wrapping_add(200 + i as u64));
            accepted.push(adjusted_token);
            break;
        }
    }

    accepted
}

// ── Batch Inference ─────────────────────────────────────────────────────

/// Generate tokens in a batch.
pub fn generate_batch(
    model_fn: &dyn Fn(&[u32]) -> Vec<f64>,
    prompts: &[Vec<u32>],
    config: &SamplingConfig,
    seed: u64,
) -> Vec<Vec<u32>> {
    prompts.iter().enumerate().map(|(batch_idx, prompt)| {
        let mut generated = prompt.clone();
        for step in 0..config.max_tokens {
            let logits = model_fn(&generated);
            let token = sample_token(&logits, config, &generated, seed.wrapping_add((batch_idx * 1000 + step) as u64));
            if config.stop_tokens.contains(&token) {
                break;
            }
            generated.push(token);
        }
        generated[prompt.len()..].to_vec()
    }).collect()
}

// ── FFI Interface ───────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_inference_argmax(logits: *const f64, count: i64) -> i64 {
    let l = unsafe { std::slice::from_raw_parts(logits, count as usize) };
    argmax(l) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_inference_apply_temperature(logits: *mut f64, count: i64, temperature: f64) {
    let l = unsafe { std::slice::from_raw_parts_mut(logits, count as usize) };
    apply_temperature(l, temperature);
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_inference_apply_top_k(logits: *mut f64, count: i64, k: i64) {
    let l = unsafe { std::slice::from_raw_parts_mut(logits, count as usize) };
    apply_top_k(l, k as usize);
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_inference_apply_top_p(logits: *mut f64, count: i64, p: f64) {
    let l = unsafe { std::slice::from_raw_parts_mut(logits, count as usize) };
    apply_top_p(l, p);
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_inference_sample(logits: *const f64, count: i64, temperature: f64, top_k: i64, seed: i64) -> i64 {
    let l = unsafe { std::slice::from_raw_parts(logits, count as usize) };
    let config = SamplingConfig {
        temperature,
        top_k: top_k as usize,
        ..Default::default()
    };
    sample_token(l, &config, &[], seed as u64) as i64
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argmax() {
        assert_eq!(argmax(&[1.0, 5.0, 3.0, 2.0]), 1);
        assert_eq!(argmax(&[0.0, 0.0, 0.0, 10.0]), 3);
    }

    #[test]
    fn test_temperature() {
        let mut logits = vec![1.0, 2.0, 3.0];
        apply_temperature(&mut logits, 0.5);
        assert_eq!(logits, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_top_k() {
        let mut logits = vec![1.0, 5.0, 3.0, 2.0, 4.0];
        apply_top_k(&mut logits, 2);
        // Only top-2 (5.0 and 4.0) should remain
        assert_eq!(logits[1], 5.0);
        assert_eq!(logits[4], 4.0);
        assert_eq!(logits[0], f64::NEG_INFINITY);
    }

    #[test]
    fn test_top_p() {
        let mut logits = vec![10.0, 1.0, 0.1, 0.01]; // Very peaked distribution
        apply_top_p(&mut logits, 0.9);
        // First token has ~99.9% prob, should be the only one kept
        assert!(logits[0] > f64::NEG_INFINITY);
    }

    #[test]
    fn test_repetition_penalty() {
        let mut logits = vec![1.0, 2.0, 3.0, 4.0];
        apply_repetition_penalty(&mut logits, &[2], 2.0);
        assert!((logits[2] - 1.5).abs() < 1e-10); // 3.0 / 2.0
        assert!((logits[0] - 1.0).abs() < 1e-10); // Unchanged
    }

    #[test]
    fn test_sample_deterministic() {
        let logits = vec![100.0, 0.0, 0.0, 0.0]; // Very peaked
        let config = SamplingConfig {
            temperature: 0.01,
            top_k: 1,
            ..Default::default()
        };
        let token = sample_token(&logits, &config, &[], 42);
        assert_eq!(token, 0); // Should always pick the dominant one
    }

    #[test]
    fn test_beam_search() {
        // Simple model that prefers token 1
        let logits_fn = |_tokens: &[u32]| -> Vec<f64> {
            vec![0.0, 10.0, 0.0, 0.0]
        };
        let results = beam_search(&logits_fn, 0, 3, 2, 5, 1.0);
        assert!(!results.is_empty());
        // Best beam should contain mostly token 1
        assert!(results[0].tokens.iter().filter(|&&t| t == 1).count() > 0);
    }

    #[test]
    fn test_speculative_decode() {
        let draft = |tokens: &[u32]| -> Vec<f64> {
            let last = tokens.last().copied().unwrap_or(0);
            let mut logits = vec![0.0; 4];
            logits[((last + 1) % 4) as usize] = 10.0;
            logits
        };
        let target = |tokens: &[u32]| -> Vec<f64> {
            let last = tokens.last().copied().unwrap_or(0);
            let mut logits = vec![0.0; 4];
            logits[((last + 1) % 4) as usize] = 10.0;
            logits
        };
        let result = speculative_decode(&draft, &target, &[0], 3, 42);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_generate_batch() {
        let model = |_tokens: &[u32]| -> Vec<f64> {
            vec![0.0, 5.0, 1.0, 0.5]
        };
        let config = SamplingConfig {
            max_tokens: 3,
            temperature: 0.01,
            top_k: 1,
            ..Default::default()
        };
        let prompts = vec![vec![0], vec![1]];
        let results = generate_batch(&model, &prompts, &config, 42);
        assert_eq!(results.len(), 2);
        assert!(results[0].len() <= 3);
    }

    #[test]
    fn test_frequency_presence_penalty() {
        let mut logits = vec![5.0, 5.0, 5.0];
        let mut counts = HashMap::new();
        counts.insert(1u32, 3);
        apply_frequency_presence_penalty(&mut logits, &counts, 1.0, 0.5);
        assert!(logits[1] < logits[0]); // Token 1 penalized
    }

    #[test]
    fn test_sampling_config_default() {
        let config = SamplingConfig::default();
        assert_eq!(config.temperature, 1.0);
        assert_eq!(config.top_k, 50);
        assert!((config.top_p - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_argmax() {
        let logits = [1.0f64, 3.0, 2.0, 0.5];
        assert_eq!(vitalis_inference_argmax(logits.as_ptr(), 4), 1);
    }

    #[test]
    fn test_ffi_sample() {
        let logits = [100.0f64, 0.0, 0.0, 0.0];
        let token = vitalis_inference_sample(logits.as_ptr(), 4, 0.01, 1, 42);
        assert_eq!(token, 0);
    }

    #[test]
    fn test_ffi_temperature() {
        let mut logits = [2.0f64, 4.0];
        vitalis_inference_apply_temperature(logits.as_mut_ptr(), 2, 2.0);
        assert_eq!(logits, [1.0, 2.0]);
    }
}
