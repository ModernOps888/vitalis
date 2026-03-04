//! Reward Model — RLHF preference modeling, reward signal design, and PPO.
//!
//! Provides Bradley-Terry preference models, reward signal composition,
//! Proximal Policy Optimization (PPO), and safety-constrained reward shaping.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Preference Pairs ────────────────────────────────────────────────────

/// A preference pair: response A vs response B, with a label.
#[derive(Debug, Clone)]
pub struct PreferencePair {
    pub prompt: String,
    pub response_a: String,
    pub response_b: String,
    pub preference: Preference,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Preference {
    PreferA,
    PreferB,
    Tie,
}

// ── Bradley-Terry Reward Model ──────────────────────────────────────────

/// A Bradley-Terry preference model for ranking responses.
///
/// Models P(a > b) = sigmoid(r(a) - r(b)) where r(x) is a learned reward.
#[derive(Debug, Clone)]
pub struct BradleyTerryModel {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub feature_dim: usize,
    pub learning_rate: f64,
}

impl BradleyTerryModel {
    pub fn new(feature_dim: usize, learning_rate: f64) -> Self {
        BradleyTerryModel {
            weights: vec![0.01; feature_dim],
            bias: 0.0,
            feature_dim,
            learning_rate,
        }
    }

    /// Compute reward score for a feature vector.
    pub fn reward(&self, features: &[f64]) -> f64 {
        let mut score = self.bias;
        for (w, f) in self.weights.iter().zip(features.iter()) {
            score += w * f;
        }
        score
    }

    /// Compute P(a preferred over b) using sigmoid.
    pub fn preference_prob(&self, features_a: &[f64], features_b: &[f64]) -> f64 {
        let r_a = self.reward(features_a);
        let r_b = self.reward(features_b);
        sigmoid(r_a - r_b)
    }

    /// Train on a batch of preference pairs (feature representation).
    pub fn train_step(&mut self, pairs: &[(Vec<f64>, Vec<f64>, Preference)]) -> f64 {
        let mut total_loss = 0.0;

        for (feat_a, feat_b, pref) in pairs {
            let r_a = self.reward(feat_a);
            let r_b = self.reward(feat_b);

            let (target, label_val) = match pref {
                Preference::PreferA => (1.0_f64, 1.0_f64),
                Preference::PreferB => (0.0, 0.0),
                Preference::Tie => (0.5, 0.5),
            };

            let prob = sigmoid(r_a - r_b);
            // Binary cross-entropy loss
            let loss = -(target * prob.max(1e-10).ln() + (1.0 - target) * (1.0 - prob).max(1e-10).ln());
            total_loss += loss;

            // Gradient: d_loss/d_r_a = prob - target, d_loss/d_r_b = target - prob
            let grad = prob - label_val;

            // Update weights
            for i in 0..self.weights.len() {
                let grad_a = if i < feat_a.len() { grad * feat_a[i] } else { 0.0 };
                let grad_b = if i < feat_b.len() { -grad * feat_b[i] } else { 0.0 };
                self.weights[i] -= self.learning_rate * (grad_a + grad_b);
            }
            self.bias -= self.learning_rate * grad;
        }

        total_loss / pairs.len().max(1) as f64
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

// ── Reward Signals ──────────────────────────────────────────────────────

/// Reward signal component.
#[derive(Debug, Clone)]
pub struct RewardSignal {
    pub name: String,
    pub weight: f64,
    pub kind: RewardKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RewardKind {
    Correctness,
    Safety,
    Helpfulness,
    Conciseness,
    Coherence,
    Custom,
}

/// Composite reward from multiple signals.
#[derive(Debug, Clone)]
pub struct CompositeReward {
    pub signals: Vec<RewardSignal>,
}

impl CompositeReward {
    pub fn new() -> Self { CompositeReward { signals: Vec::new() } }

    pub fn add_signal(&mut self, name: &str, weight: f64, kind: RewardKind) {
        self.signals.push(RewardSignal {
            name: name.to_string(),
            weight,
            kind,
        });
    }

    /// Compute weighted composite reward from individual scores.
    pub fn compute(&self, scores: &HashMap<String, f64>) -> f64 {
        let mut total = 0.0;
        let mut weight_sum = 0.0;
        for signal in &self.signals {
            if let Some(&score) = scores.get(&signal.name) {
                total += signal.weight * score;
                weight_sum += signal.weight.abs();
            }
        }
        if weight_sum > 0.0 { total / weight_sum } else { 0.0 }
    }

    /// Check if safety constraints are satisfied (all Safety signals > threshold).
    pub fn safety_satisfied(&self, scores: &HashMap<String, f64>, threshold: f64) -> bool {
        for signal in &self.signals {
            if signal.kind == RewardKind::Safety {
                if let Some(&score) = scores.get(&signal.name) {
                    if score < threshold {
                        return false;
                    }
                }
            }
        }
        true
    }
}

impl Default for CompositeReward {
    fn default() -> Self { Self::new() }
}

// ── PPO (Proximal Policy Optimization) ──────────────────────────────────

/// PPO step result.
#[derive(Debug, Clone)]
pub struct PPOStep {
    pub state: Vec<f64>,
    pub action: usize,
    pub log_prob: f64,
    pub reward: f64,
    pub value: f64,
    pub done: bool,
}

/// Generalized Advantage Estimation (GAE).
pub fn compute_gae(rewards: &[f64], values: &[f64], dones: &[bool], gamma: f64, lam: f64) -> Vec<f64> {
    let n = rewards.len();
    let mut advantages = vec![0.0; n];
    let mut last_gae = 0.0;

    for t in (0..n).rev() {
        let next_value = if t + 1 < n && !dones[t] { values[t + 1] } else { 0.0 };
        let delta = rewards[t] + gamma * next_value - values[t];
        let mask = if dones[t] { 0.0 } else { 1.0 };
        last_gae = delta + gamma * lam * mask * last_gae;
        advantages[t] = last_gae;
    }
    advantages
}

/// Compute PPO clipped policy loss.
pub fn ppo_policy_loss(
    old_log_probs: &[f64],
    new_log_probs: &[f64],
    advantages: &[f64],
    clip_epsilon: f64,
) -> f64 {
    let n = old_log_probs.len().min(new_log_probs.len()).min(advantages.len());
    let mut total_loss = 0.0;

    for i in 0..n {
        let ratio = (new_log_probs[i] - old_log_probs[i]).exp();
        let clipped_ratio = ratio.clamp(1.0 - clip_epsilon, 1.0 + clip_epsilon);
        let surr1 = ratio * advantages[i];
        let surr2 = clipped_ratio * advantages[i];
        total_loss += surr1.min(surr2);
    }

    -total_loss / n.max(1) as f64 // Negative because we minimize loss
}

/// Compute value function loss (MSE).
pub fn value_loss(predicted: &[f64], targets: &[f64]) -> f64 {
    let n = predicted.len().min(targets.len());
    let mut total = 0.0;
    for i in 0..n {
        let diff = predicted[i] - targets[i];
        total += diff * diff;
    }
    total / n.max(1) as f64
}

// ── Reward Shaping ──────────────────────────────────────────────────────

/// Potential-based reward shaping (preserves optimal policy).
pub fn shaped_reward(
    reward: f64,
    potential_current: f64,
    potential_next: f64,
    gamma: f64,
) -> f64 {
    reward + gamma * potential_next - potential_current
}

/// Curiosity-based intrinsic reward (prediction error).
pub fn curiosity_reward(
    predicted_next_state: &[f64],
    actual_next_state: &[f64],
    scale: f64,
) -> f64 {
    let mse: f64 = predicted_next_state.iter()
        .zip(actual_next_state.iter())
        .map(|(p, a)| (p - a) * (p - a))
        .sum::<f64>() / predicted_next_state.len().max(1) as f64;
    scale * mse.sqrt()
}

/// Entropy bonus for exploration encouragement.
pub fn entropy_bonus(probs: &[f64]) -> f64 {
    let mut entropy = 0.0;
    for &p in probs {
        if p > 0.0 {
            entropy -= p * p.ln();
        }
    }
    entropy
}

// ── KL Divergence for RLHF ─────────────────────────────────────────────

/// KL divergence between two distributions (for constraining policy updates).
pub fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    let mut kl = 0.0;
    for (pi, qi) in p.iter().zip(q.iter()) {
        if *pi > 0.0 && *qi > 0.0 {
            kl += pi * (pi / qi).ln();
        }
    }
    kl
}

/// KL penalty for RLHF (penalizes deviation from reference policy).
pub fn kl_penalty(log_prob_new: f64, log_prob_ref: f64, beta: f64) -> f64 {
    beta * (log_prob_new - log_prob_ref)
}

// ── FFI Interface ───────────────────────────────────────────────────────

static REWARD_STORE: Mutex<Option<HashMap<i64, BradleyTerryModel>>> = Mutex::new(None);

fn reward_store_insert(model: BradleyTerryModel) -> i64 {
    let mut guard = REWARD_STORE.lock().unwrap();
    let store = guard.get_or_insert_with(HashMap::new);
    let id = store.len() as i64;
    store.insert(id, model);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_reward_create(feature_dim: i64, learning_rate: f64) -> i64 {
    reward_store_insert(BradleyTerryModel::new(feature_dim as usize, learning_rate))
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_reward_score(model_id: i64, features: *const f64, feature_len: i64) -> f64 {
    let guard = REWARD_STORE.lock().unwrap();
    let feats = unsafe { std::slice::from_raw_parts(features, feature_len as usize) };
    guard.as_ref()
        .and_then(|s| s.get(&model_id))
        .map(|m| m.reward(feats))
        .unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_reward_ppo_loss(
    old_lp: *const f64, new_lp: *const f64, advs: *const f64, n: i64, clip: f64
) -> f64 {
    let old = unsafe { std::slice::from_raw_parts(old_lp, n as usize) };
    let new = unsafe { std::slice::from_raw_parts(new_lp, n as usize) };
    let advantages = unsafe { std::slice::from_raw_parts(advs, n as usize) };
    ppo_policy_loss(old, new, advantages, clip)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_reward_gae(
    rewards: *const f64, values: *const f64, n: i64, gamma: f64, lam: f64
) -> f64 {
    let r = unsafe { std::slice::from_raw_parts(rewards, n as usize) };
    let v = unsafe { std::slice::from_raw_parts(values, n as usize) };
    let dones = vec![false; n as usize]; // Assume no episode termination
    let gae = compute_gae(r, v, &dones, gamma, lam);
    gae.iter().sum::<f64>() / gae.len().max(1) as f64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_reward_free(model_id: i64) {
    let mut guard = REWARD_STORE.lock().unwrap();
    if let Some(store) = guard.as_mut() {
        store.remove(&model_id);
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid_bounds() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
        assert!(sigmoid(100.0) > 0.99);
        assert!(sigmoid(-100.0) < 0.01);
    }

    #[test]
    fn test_bradley_terry_reward() {
        let model = BradleyTerryModel::new(3, 0.01);
        let score = model.reward(&[1.0, 2.0, 3.0]);
        // weights are [0.01, 0.01, 0.01], bias=0
        let expected = 0.01 + 0.02 + 0.03;
        assert!((score - expected).abs() < 1e-10);
    }

    #[test]
    fn test_preference_prob() {
        let mut model = BradleyTerryModel::new(2, 0.01);
        model.weights = vec![1.0, 0.0];
        // r_a = 1*10 = 10, r_b = 1*0 = 0, P(a>b) = sigmoid(10) ≈ 1
        let prob = model.preference_prob(&[10.0, 0.0], &[0.0, 0.0]);
        assert!(prob > 0.99);
    }

    #[test]
    fn test_train_step() {
        let mut model = BradleyTerryModel::new(2, 0.1);
        let pairs = vec![
            (vec![1.0, 0.0], vec![0.0, 1.0], Preference::PreferA),
            (vec![0.0, 1.0], vec![1.0, 0.0], Preference::PreferB),
        ];
        let loss1 = model.train_step(&pairs);
        let loss2 = model.train_step(&pairs);
        assert!(loss2 <= loss1 + 0.1); // Loss should generally decrease
    }

    #[test]
    fn test_composite_reward() {
        let mut reward = CompositeReward::new();
        reward.add_signal("correctness", 2.0, RewardKind::Correctness);
        reward.add_signal("safety", 1.0, RewardKind::Safety);

        let mut scores = HashMap::new();
        scores.insert("correctness".to_string(), 0.8);
        scores.insert("safety".to_string(), 0.9);

        let r = reward.compute(&scores);
        // (2.0*0.8 + 1.0*0.9) / (2.0+1.0) = 2.5/3.0
        assert!((r - 2.5 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_safety_constraint() {
        let mut reward = CompositeReward::new();
        reward.add_signal("safety", 1.0, RewardKind::Safety);

        let mut scores = HashMap::new();
        scores.insert("safety".to_string(), 0.3);
        assert!(!reward.safety_satisfied(&scores, 0.5));

        scores.insert("safety".to_string(), 0.8);
        assert!(reward.safety_satisfied(&scores, 0.5));
    }

    #[test]
    fn test_gae_simple() {
        let rewards = vec![1.0, 0.0, 1.0];
        let values = vec![0.5, 0.3, 0.8];
        let dones = vec![false, false, true];
        let gae = compute_gae(&rewards, &values, &dones, 0.99, 0.95);
        assert_eq!(gae.len(), 3);
        // Last step (done=true): delta = 1.0 + 0*0 - 0.8 = 0.2
        assert!((gae[2] - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_no_clip() {
        // Same log probs → ratio = 1 → no clipping
        let old_lp = vec![0.0; 3];
        let new_lp = vec![0.0; 3];
        let advs = vec![1.0, 2.0, 3.0];
        let loss = ppo_policy_loss(&old_lp, &new_lp, &advs, 0.2);
        // loss = -mean(advantages) = -2.0
        assert!((loss - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_clips_large_ratio() {
        let old_lp = vec![0.0];
        let new_lp = vec![1.0]; // ratio = e^1 ≈ 2.718
        let advs = vec![1.0];
        let loss = ppo_policy_loss(&old_lp, &new_lp, &advs, 0.2);
        // ratio = 2.718, clipped = 1.2, surr1=2.718, surr2=1.2
        // loss = -min(2.718, 1.2) = -1.2
        assert!((loss - (-1.2)).abs() < 1e-10);
    }

    #[test]
    fn test_value_loss() {
        let predicted = vec![1.0, 2.0, 3.0];
        let targets = vec![1.0, 2.0, 3.0];
        assert!(value_loss(&predicted, &targets) < 1e-10);

        let targets2 = vec![2.0, 3.0, 4.0];
        assert!((value_loss(&predicted, &targets2) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_shaped_reward() {
        let r = shaped_reward(1.0, 0.5, 0.8, 0.99);
        // 1.0 + 0.99*0.8 - 0.5 = 1.0 + 0.792 - 0.5 = 1.292
        assert!((r - 1.292).abs() < 1e-10);
    }

    #[test]
    fn test_curiosity_reward() {
        let predicted = vec![1.0, 2.0, 3.0];
        let actual = vec![1.0, 2.0, 3.0];
        assert!(curiosity_reward(&predicted, &actual, 1.0) < 1e-10);

        let actual2 = vec![2.0, 3.0, 4.0];
        assert!(curiosity_reward(&predicted, &actual2, 1.0) > 0.0);
    }

    #[test]
    fn test_entropy_bonus() {
        let uniform = vec![0.25, 0.25, 0.25, 0.25];
        let peaked = vec![0.97, 0.01, 0.01, 0.01];
        assert!(entropy_bonus(&uniform) > entropy_bonus(&peaked));
    }

    #[test]
    fn test_kl_divergence_same() {
        let p = vec![0.5, 0.5];
        assert!(kl_divergence(&p, &p) < 1e-10);
    }

    #[test]
    fn test_kl_divergence_different() {
        let p = vec![0.9, 0.1];
        let q = vec![0.5, 0.5];
        assert!(kl_divergence(&p, &q) > 0.0);
    }

    #[test]
    fn test_kl_penalty() {
        let pen = kl_penalty(0.5, 0.3, 0.1);
        assert!((pen - 0.1 * 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_reward_create() {
        let id = vitalis_reward_create(5, 0.01);
        assert!(id >= 0);
        let features = [1.0f64, 2.0, 3.0, 4.0, 5.0];
        let score = vitalis_reward_score(id, features.as_ptr(), 5);
        assert!(score.is_finite());
        vitalis_reward_free(id);
    }

    #[test]
    fn test_ffi_ppo_loss() {
        let old_lp = [0.0f64; 2];
        let new_lp = [0.0f64; 2];
        let advs = [1.0f64, 2.0];
        let loss = vitalis_reward_ppo_loss(old_lp.as_ptr(), new_lp.as_ptr(), advs.as_ptr(), 2, 0.2);
        assert!((loss - (-1.5)).abs() < 1e-10);
    }

    #[test]
    fn test_ffi_gae() {
        let rewards = [1.0f64, 1.0, 1.0];
        let values = [0.5f64, 0.5, 0.5];
        let avg = vitalis_reward_gae(rewards.as_ptr(), values.as_ptr(), 3, 0.99, 0.95);
        assert!(avg.is_finite());
    }
}
