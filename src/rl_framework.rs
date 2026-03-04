//! Reinforcement Learning Framework — environments, policies, DQN, PPO, A2C.
//!
//! Provides a complete RL stack: environment protocol, policy networks,
//! replay buffers, DQN/PPO/A2C algorithms, multi-agent support,
//! and reward shaping utilities.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Environment Protocol ────────────────────────────────────────────────

/// An observation from the environment.
#[derive(Debug, Clone)]
pub struct Observation {
    pub state: Vec<f64>,
    pub reward: f64,
    pub done: bool,
    pub info: HashMap<String, f64>,
}

/// Environment interface (Gym-like).
pub trait Environment {
    fn reset(&mut self) -> Vec<f64>;
    fn step(&mut self, action: usize) -> Observation;
    fn n_actions(&self) -> usize;
    fn state_dim(&self) -> usize;
}

/// A transition (s, a, r, s', done) stored in replay buffer.
#[derive(Debug, Clone)]
pub struct Transition {
    pub state: Vec<f64>,
    pub action: usize,
    pub reward: f64,
    pub next_state: Vec<f64>,
    pub done: bool,
}

// ── Replay Buffer ──────────────────────────────────────────────────────

/// Circular replay buffer for off-policy learning.
#[derive(Debug, Clone)]
pub struct ReplayBuffer {
    pub capacity: usize,
    pub buffer: Vec<Transition>,
    pub pos: usize,
}

impl ReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        ReplayBuffer { capacity, buffer: Vec::with_capacity(capacity), pos: 0 }
    }

    pub fn push(&mut self, transition: Transition) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(transition);
        } else {
            self.buffer[self.pos] = transition;
        }
        self.pos = (self.pos + 1) % self.capacity;
    }

    pub fn len(&self) -> usize { self.buffer.len() }

    pub fn is_empty(&self) -> bool { self.buffer.is_empty() }

    /// Sample a random minibatch.
    pub fn sample(&self, batch_size: usize, rng: &mut SimpleRng) -> Vec<Transition> {
        let n = self.buffer.len();
        if n == 0 { return vec![]; }
        (0..batch_size).map(|_| {
            let idx = (rng.next_u64() as usize) % n;
            self.buffer[idx].clone()
        }).collect()
    }

    /// Prioritized sampling: higher TD-error → higher probability.
    pub fn sample_prioritized(&self, batch_size: usize, priorities: &[f64], rng: &mut SimpleRng) -> Vec<(usize, Transition)> {
        let n = priorities.len().min(self.buffer.len());
        if n == 0 { return vec![]; }
        let sum: f64 = priorities.iter().take(n).sum();
        if sum <= 0.0 {
            return self.sample(batch_size, rng).into_iter().enumerate().collect();
        }

        (0..batch_size).map(|_| {
            let threshold = rng.next_f64() * sum;
            let mut cumsum = 0.0;
            for i in 0..n {
                cumsum += priorities[i];
                if cumsum >= threshold {
                    return (i, self.buffer[i].clone());
                }
            }
            (n - 1, self.buffer[n - 1].clone())
        }).collect()
    }
}

// ── Q-Network (Tabular + Simple Linear) ────────────────────────────────

/// Tabular Q-values for discrete state/action spaces.
#[derive(Debug, Clone)]
pub struct TabularQ {
    pub q_table: HashMap<(Vec<i64>, usize), f64>,
    pub learning_rate: f64,
    pub gamma: f64,
    pub epsilon: f64,
}

impl TabularQ {
    pub fn new(learning_rate: f64, gamma: f64, epsilon: f64) -> Self {
        TabularQ { q_table: HashMap::new(), learning_rate, gamma, epsilon }
    }

    fn discretize(state: &[f64]) -> Vec<i64> {
        state.iter().map(|&s| (s * 10.0).round() as i64).collect()
    }

    pub fn get_q(&self, state: &[f64], action: usize) -> f64 {
        let key = (Self::discretize(state), action);
        *self.q_table.get(&key).unwrap_or(&0.0)
    }

    pub fn select_action(&self, state: &[f64], n_actions: usize, rng: &mut SimpleRng) -> usize {
        if rng.next_f64() < self.epsilon {
            (rng.next_u64() as usize) % n_actions
        } else {
            (0..n_actions)
                .max_by(|&a, &b| self.get_q(state, a).partial_cmp(&self.get_q(state, b)).unwrap())
                .unwrap_or(0)
        }
    }

    /// Q-learning update: Q(s,a) ← Q(s,a) + α·(r + γ·max_a'Q(s',a') - Q(s,a))
    pub fn update(&mut self, state: &[f64], action: usize, reward: f64, next_state: &[f64], done: bool, n_actions: usize) {
        let max_next_q = if done {
            0.0
        } else {
            (0..n_actions).map(|a| self.get_q(next_state, a)).fold(f64::NEG_INFINITY, f64::max)
        };
        let current_q = self.get_q(state, action);
        let td_target = reward + self.gamma * max_next_q;
        let new_q = current_q + self.learning_rate * (td_target - current_q);
        let key = (Self::discretize(state), action);
        self.q_table.insert(key, new_q);
    }
}

// ── Linear Q-Network (DQN-like) ───────────────────────────────────────

/// Simple linear DQN: Q(s,a) = W[a]·s + b[a].
#[derive(Debug, Clone)]
pub struct LinearDQN {
    pub state_dim: usize,
    pub n_actions: usize,
    pub weights: Vec<f64>,    // n_actions × state_dim
    pub biases: Vec<f64>,     // n_actions
    pub target_weights: Vec<f64>,
    pub target_biases: Vec<f64>,
    pub learning_rate: f64,
    pub gamma: f64,
    pub epsilon: f64,
    pub update_counter: usize,
    pub target_update_freq: usize,
}

impl LinearDQN {
    pub fn new(state_dim: usize, n_actions: usize, lr: f64, gamma: f64) -> Self {
        let n = n_actions * state_dim;
        let weights = vec![0.0; n];
        let biases = vec![0.0; n_actions];
        LinearDQN {
            state_dim, n_actions,
            weights: weights.clone(), biases: biases.clone(),
            target_weights: weights, target_biases: biases,
            learning_rate: lr, gamma,
            epsilon: 1.0, update_counter: 0,
            target_update_freq: 100,
        }
    }

    pub fn q_values(&self, state: &[f64]) -> Vec<f64> {
        let mut q = self.biases.clone();
        for a in 0..self.n_actions {
            for i in 0..self.state_dim {
                q[a] += self.weights[a * self.state_dim + i] * state[i];
            }
        }
        q
    }

    fn target_q_values(&self, state: &[f64]) -> Vec<f64> {
        let mut q = self.target_biases.clone();
        for a in 0..self.n_actions {
            for i in 0..self.state_dim {
                q[a] += self.target_weights[a * self.state_dim + i] * state[i];
            }
        }
        q
    }

    pub fn select_action(&self, state: &[f64], rng: &mut SimpleRng) -> usize {
        if rng.next_f64() < self.epsilon {
            (rng.next_u64() as usize) % self.n_actions
        } else {
            let q = self.q_values(state);
            q.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()).unwrap().0
        }
    }

    /// Train on a minibatch of transitions.
    pub fn train_batch(&mut self, batch: &[Transition]) {
        for t in batch {
            let q = self.q_values(&t.state);
            let target_q_next = self.target_q_values(&t.next_state);
            let max_next = if t.done { 0.0 } else {
                target_q_next.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            };
            let td_target = t.reward + self.gamma * max_next;
            let td_error = td_target - q[t.action];

            // Update weights for the selected action
            for i in 0..self.state_dim {
                let idx = t.action * self.state_dim + i;
                self.weights[idx] += self.learning_rate * td_error * t.state[i];
            }
            self.biases[t.action] += self.learning_rate * td_error;
        }

        self.update_counter += 1;
        if self.update_counter % self.target_update_freq == 0 {
            self.target_weights = self.weights.clone();
            self.target_biases = self.biases.clone();
        }

        // Decay epsilon
        self.epsilon = (self.epsilon * 0.999).max(0.01);
    }
}

// ── PPO (Proximal Policy Optimization) ──────────────────────────────────

/// PPO trajectory rollout.
#[derive(Debug, Clone)]
pub struct PPOTrajectory {
    pub states: Vec<Vec<f64>>,
    pub actions: Vec<usize>,
    pub rewards: Vec<f64>,
    pub log_probs: Vec<f64>,
    pub values: Vec<f64>,
    pub dones: Vec<bool>,
}

impl PPOTrajectory {
    pub fn new() -> Self {
        PPOTrajectory {
            states: vec![], actions: vec![], rewards: vec![],
            log_probs: vec![], values: vec![], dones: vec![],
        }
    }

    pub fn push(&mut self, state: Vec<f64>, action: usize, reward: f64, log_prob: f64, value: f64, done: bool) {
        self.states.push(state);
        self.actions.push(action);
        self.rewards.push(reward);
        self.log_probs.push(log_prob);
        self.values.push(value);
        self.dones.push(done);
    }

    /// Compute GAE (Generalized Advantage Estimation).
    pub fn compute_gae(&self, gamma: f64, lambda: f64, last_value: f64) -> (Vec<f64>, Vec<f64>) {
        let n = self.rewards.len();
        let mut advantages = vec![0.0; n];
        let mut returns = vec![0.0; n];
        let mut gae = 0.0;

        for t in (0..n).rev() {
            let next_value = if t == n - 1 { last_value } else { self.values[t + 1] };
            let delta = self.rewards[t] + gamma * next_value * (1.0 - self.dones[t] as u8 as f64) - self.values[t];
            gae = delta + gamma * lambda * (1.0 - self.dones[t] as u8 as f64) * gae;
            advantages[t] = gae;
            returns[t] = advantages[t] + self.values[t];
        }

        (advantages, returns)
    }
}

/// PPO configuration.
#[derive(Debug, Clone)]
pub struct PPOConfig {
    pub clip_ratio: f64,
    pub gamma: f64,
    pub lambda: f64,
    pub n_epochs: usize,
    pub value_coef: f64,
    pub entropy_coef: f64,
}

impl Default for PPOConfig {
    fn default() -> Self {
        PPOConfig {
            clip_ratio: 0.2, gamma: 0.99, lambda: 0.95,
            n_epochs: 4, value_coef: 0.5, entropy_coef: 0.01,
        }
    }
}

/// Compute PPO clipped surrogate loss.
pub fn ppo_loss(
    old_log_probs: &[f64],
    new_log_probs: &[f64],
    advantages: &[f64],
    clip_ratio: f64,
) -> f64 {
    let n = old_log_probs.len();
    let mut total_loss = 0.0;

    for i in 0..n {
        let ratio = (new_log_probs[i] - old_log_probs[i]).exp();
        let clipped = ratio.clamp(1.0 - clip_ratio, 1.0 + clip_ratio);
        let surr1 = ratio * advantages[i];
        let surr2 = clipped * advantages[i];
        total_loss += -surr1.min(surr2);
    }

    total_loss / n as f64
}

// ── A2C (Advantage Actor-Critic) ────────────────────────────────────────

/// Simple linear actor-critic for discrete actions.
#[derive(Debug, Clone)]
pub struct LinearActorCritic {
    pub state_dim: usize,
    pub n_actions: usize,
    pub policy_weights: Vec<f64>,   // n_actions × state_dim (logits)
    pub policy_biases: Vec<f64>,
    pub value_weights: Vec<f64>,    // 1 × state_dim
    pub value_bias: f64,
    pub learning_rate: f64,
}

impl LinearActorCritic {
    pub fn new(state_dim: usize, n_actions: usize, lr: f64) -> Self {
        LinearActorCritic {
            state_dim, n_actions,
            policy_weights: vec![0.0; n_actions * state_dim],
            policy_biases: vec![0.0; n_actions],
            value_weights: vec![0.0; state_dim],
            value_bias: 0.0,
            learning_rate: lr,
        }
    }

    /// Compute action logits.
    pub fn logits(&self, state: &[f64]) -> Vec<f64> {
        let mut out = self.policy_biases.clone();
        for a in 0..self.n_actions {
            for i in 0..self.state_dim {
                out[a] += self.policy_weights[a * self.state_dim + i] * state[i];
            }
        }
        out
    }

    /// Softmax action probabilities.
    pub fn action_probs(&self, state: &[f64]) -> Vec<f64> {
        let logits = self.logits(state);
        softmax(&logits)
    }

    /// State value estimate.
    pub fn value(&self, state: &[f64]) -> f64 {
        let mut v = self.value_bias;
        for i in 0..self.state_dim {
            v += self.value_weights[i] * state[i];
        }
        v
    }

    /// Select action by sampling from softmax policy.
    pub fn select_action(&self, state: &[f64], rng: &mut SimpleRng) -> (usize, f64) {
        let probs = self.action_probs(state);
        let u = rng.next_f64();
        let mut cum = 0.0;
        for (i, &p) in probs.iter().enumerate() {
            cum += p;
            if u < cum {
                return (i, p.max(1e-8).ln());
            }
        }
        let last = probs.len() - 1;
        (last, probs[last].max(1e-8).ln())
    }

    /// A2C update step.
    pub fn update(&mut self, state: &[f64], action: usize, advantage: f64, td_target: f64) {
        let probs = self.action_probs(state);
        let v = self.value(state);

        // Policy gradient: ∇_θ log π(a|s) · A
        for a in 0..self.n_actions {
            let indicator = if a == action { 1.0 } else { 0.0 };
            let grad = indicator - probs[a]; // softmax gradient
            for i in 0..self.state_dim {
                self.policy_weights[a * self.state_dim + i] += self.learning_rate * advantage * grad * state[i];
            }
            self.policy_biases[a] += self.learning_rate * advantage * grad;
        }

        // Value update: MSE gradient
        let value_error = td_target - v;
        for i in 0..self.state_dim {
            self.value_weights[i] += self.learning_rate * value_error * state[i];
        }
        self.value_bias += self.learning_rate * value_error;
    }
}

// ── Reward Shaping ──────────────────────────────────────────────────────

/// Potential-based reward shaping: F(s,s') = γ·Φ(s') - Φ(s).
pub fn shaped_reward(reward: f64, potential_s: f64, potential_s_next: f64, gamma: f64) -> f64 {
    reward + gamma * potential_s_next - potential_s
}

/// Compute discounted returns from a reward sequence.
pub fn discounted_returns(rewards: &[f64], gamma: f64) -> Vec<f64> {
    let n = rewards.len();
    let mut returns = vec![0.0; n];
    let mut g = 0.0;
    for t in (0..n).rev() {
        g = rewards[t] + gamma * g;
        returns[t] = g;
    }
    returns
}

/// Normalize advantages to zero-mean unit-variance.
pub fn normalize_advantages(advantages: &mut [f64]) {
    if advantages.is_empty() { return; }
    let mean: f64 = advantages.iter().sum::<f64>() / advantages.len() as f64;
    let var: f64 = advantages.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / advantages.len() as f64;
    let std = var.sqrt().max(1e-8);
    for a in advantages.iter_mut() {
        *a = (*a - mean) / std;
    }
}

// ── Multi-Agent Support ─────────────────────────────────────────────────

/// Multi-agent environment handler.
#[derive(Debug, Clone)]
pub struct MultiAgentState {
    pub n_agents: usize,
    pub states: Vec<Vec<f64>>,
    pub rewards: Vec<f64>,
    pub dones: Vec<bool>,
}

impl MultiAgentState {
    pub fn new(n_agents: usize, state_dim: usize) -> Self {
        MultiAgentState {
            n_agents,
            states: vec![vec![0.0; state_dim]; n_agents],
            rewards: vec![0.0; n_agents],
            dones: vec![false; n_agents],
        }
    }

    pub fn total_reward(&self) -> f64 { self.rewards.iter().sum() }
    pub fn all_done(&self) -> bool { self.dones.iter().all(|&d| d) }
}

// ── Simple RNG ──────────────────────────────────────────────────────────

/// LCG random number generator.
#[derive(Debug, Clone)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self { SimpleRng { state: seed.wrapping_add(1) } }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ── Utility ─────────────────────────────────────────────────────────────

fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&l| (l - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

// ── FFI Interface ───────────────────────────────────────────────────────

static RL_STORES: Mutex<Option<HashMap<i64, TabularQ>>> = Mutex::new(None);

fn rl_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, TabularQ>>> {
    RL_STORES.lock().unwrap()
}

fn next_rl_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_rl_create(lr: f64, gamma: f64, epsilon: f64) -> i64 {
    let id = next_rl_id();
    let mut store = rl_store();
    store.get_or_insert_with(HashMap::new).insert(id, TabularQ::new(lr, gamma, epsilon));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_rl_get_q(id: i64, state_val: f64, action: i64) -> f64 {
    let store = rl_store();
    store.as_ref().and_then(|s| s.get(&id)).map(|q| q.get_q(&[state_val], action as usize)).unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_rl_update(id: i64, state_val: f64, action: i64, reward: f64, next_state_val: f64, done: i64, n_actions: i64) {
    let mut store = rl_store();
    if let Some(s) = store.as_mut() {
        if let Some(q) = s.get_mut(&id) {
            q.update(&[state_val], action as usize, reward, &[next_state_val], done != 0, n_actions as usize);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_rl_free(id: i64) {
    let mut store = rl_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_buffer() {
        let mut buf = ReplayBuffer::new(100);
        for i in 0..10 {
            buf.push(Transition {
                state: vec![i as f64], action: 0, reward: 1.0,
                next_state: vec![(i + 1) as f64], done: false,
            });
        }
        assert_eq!(buf.len(), 10);
        let mut rng = SimpleRng::new(42);
        let batch = buf.sample(5, &mut rng);
        assert_eq!(batch.len(), 5);
    }

    #[test]
    fn test_replay_buffer_circular() {
        let mut buf = ReplayBuffer::new(5);
        for i in 0..10 {
            buf.push(Transition {
                state: vec![i as f64], action: 0, reward: 1.0,
                next_state: vec![0.0], done: false,
            });
        }
        assert_eq!(buf.len(), 5); // Capped at capacity
    }

    #[test]
    fn test_tabular_q() {
        let mut q = TabularQ::new(0.1, 0.99, 0.0);
        q.update(&[0.0], 0, 1.0, &[1.0], false, 2);
        let val = q.get_q(&[0.0], 0);
        assert!(val > 0.0);
    }

    #[test]
    fn test_tabular_q_select() {
        let mut q = TabularQ::new(0.1, 0.99, 0.0);
        q.update(&[0.0], 1, 10.0, &[0.0], true, 3);
        let mut rng = SimpleRng::new(42);
        let action = q.select_action(&[0.0], 3, &mut rng);
        assert_eq!(action, 1); // epsilon=0, should pick best
    }

    #[test]
    fn test_linear_dqn() {
        let mut dqn = LinearDQN::new(4, 2, 0.01, 0.99);
        let batch = vec![Transition {
            state: vec![1.0, 0.0, 0.0, 0.0],
            action: 0, reward: 1.0,
            next_state: vec![0.0, 1.0, 0.0, 0.0],
            done: false,
        }];
        dqn.train_batch(&batch);
        let q = dqn.q_values(&[1.0, 0.0, 0.0, 0.0]);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn test_ppo_trajectory() {
        let mut traj = PPOTrajectory::new();
        for i in 0..5 {
            traj.push(vec![i as f64], 0, 1.0, -0.5, 0.5, i == 4);
        }
        let (adv, ret) = traj.compute_gae(0.99, 0.95, 0.0);
        assert_eq!(adv.len(), 5);
        assert_eq!(ret.len(), 5);
        assert!(ret[0] > ret[4]); // Earlier returns should be higher
    }

    #[test]
    fn test_ppo_loss() {
        let old_lp = vec![-0.5, -0.3, -0.7];
        let new_lp = vec![-0.4, -0.35, -0.6];
        let advantages = vec![1.0, -0.5, 0.3];
        let loss = ppo_loss(&old_lp, &new_lp, &advantages, 0.2);
        assert!(loss.is_finite());
    }

    #[test]
    fn test_actor_critic() {
        let mut ac = LinearActorCritic::new(4, 3, 0.01);
        let state = vec![1.0, 0.5, -0.3, 0.8];
        let probs = ac.action_probs(&state);
        assert_eq!(probs.len(), 3);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);

        let v = ac.value(&state);
        ac.update(&state, 1, 0.5, 1.0);
        let v2 = ac.value(&state);
        assert!((v2 - v).abs() > 0.0); // Value should change
    }

    #[test]
    fn test_actor_critic_select() {
        let ac = LinearActorCritic::new(2, 3, 0.01);
        let mut rng = SimpleRng::new(42);
        let (action, log_prob) = ac.select_action(&[1.0, 0.0], &mut rng);
        assert!(action < 3);
        assert!(log_prob <= 0.0);
    }

    #[test]
    fn test_discounted_returns() {
        let rewards = vec![1.0, 1.0, 1.0, 1.0];
        let returns = discounted_returns(&rewards, 0.99);
        assert!(returns[0] > returns[3]);
        assert!((returns[3] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_advantages() {
        let mut adv = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        normalize_advantages(&mut adv);
        let mean: f64 = adv.iter().sum::<f64>() / adv.len() as f64;
        assert!(mean.abs() < 1e-10); // zero-mean
    }

    #[test]
    fn test_shaped_reward() {
        let r = shaped_reward(1.0, 0.5, 0.8, 0.99);
        assert!((r - (1.0 + 0.99 * 0.8 - 0.5)).abs() < 1e-10);
    }

    #[test]
    fn test_multi_agent() {
        let mas = MultiAgentState::new(3, 4);
        assert_eq!(mas.n_agents, 3);
        assert_eq!(mas.total_reward(), 0.0);
        assert!(!mas.all_done());
    }

    #[test]
    fn test_prioritized_sampling() {
        let mut buf = ReplayBuffer::new(100);
        for i in 0..10 {
            buf.push(Transition {
                state: vec![i as f64], action: 0, reward: i as f64,
                next_state: vec![0.0], done: false,
            });
        }
        let priorities: Vec<f64> = (0..10).map(|i| (i + 1) as f64).collect();
        let mut rng = SimpleRng::new(42);
        let sampled = buf.sample_prioritized(5, &priorities, &mut rng);
        assert_eq!(sampled.len(), 5);
    }

    #[test]
    fn test_ffi_create_free() {
        let id = vitalis_rl_create(0.1, 0.99, 0.1);
        assert!(id > 0);
        vitalis_rl_update(id, 0.0, 0, 1.0, 1.0, 0, 2);
        let q = vitalis_rl_get_q(id, 0.0, 0);
        assert!(q > 0.0);
        vitalis_rl_free(id);
    }

    #[test]
    fn test_softmax() {
        let probs = softmax(&[1.0, 2.0, 3.0]);
        assert_eq!(probs.len(), 3);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
        assert!(probs[2] > probs[1]);
        assert!(probs[1] > probs[0]);
    }
}
