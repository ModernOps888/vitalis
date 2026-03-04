//! Simulation Environments — grid worlds, continuous control, code optimization.
//!
//! Provides configurable simulation environments for RL:
//! grid worlds, bandit problems, continuous control,
//! and a code optimization environment for self-improvement.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Grid World ──────────────────────────────────────────────────────────

/// Grid world environment: agent navigates to goal on a 2D grid.
#[derive(Debug, Clone)]
pub struct GridWorld {
    pub width: usize,
    pub height: usize,
    pub agent_pos: (usize, usize),
    pub goal_pos: (usize, usize),
    pub walls: Vec<(usize, usize)>,
    pub step_count: usize,
    pub max_steps: usize,
}

impl GridWorld {
    pub fn new(width: usize, height: usize, goal: (usize, usize)) -> Self {
        GridWorld {
            width, height,
            agent_pos: (0, 0),
            goal_pos: goal,
            walls: vec![],
            step_count: 0,
            max_steps: width * height * 2,
        }
    }

    pub fn add_wall(&mut self, x: usize, y: usize) {
        self.walls.push((x, y));
    }

    pub fn reset(&mut self) -> Vec<f64> {
        self.agent_pos = (0, 0);
        self.step_count = 0;
        self.state_vector()
    }

    /// Actions: 0=Up, 1=Right, 2=Down, 3=Left
    pub fn step(&mut self, action: usize) -> SimObs {
        self.step_count += 1;
        let (x, y) = self.agent_pos;

        let new_pos = match action {
            0 => (x, y.saturating_sub(1)),                // Up
            1 => ((x + 1).min(self.width - 1), y),        // Right
            2 => (x, (y + 1).min(self.height - 1)),       // Down
            3 => (x.saturating_sub(1), y),                 // Left
            _ => (x, y),
        };

        // Check wall collision
        if !self.walls.contains(&new_pos) {
            self.agent_pos = new_pos;
        }

        let reached_goal = self.agent_pos == self.goal_pos;
        let timed_out = self.step_count >= self.max_steps;
        let done = reached_goal || timed_out;

        let reward = if reached_goal { 10.0 } else { -0.1 };

        SimObs {
            state: self.state_vector(),
            reward,
            done,
        }
    }

    pub fn state_vector(&self) -> Vec<f64> {
        vec![
            self.agent_pos.0 as f64 / self.width as f64,
            self.agent_pos.1 as f64 / self.height as f64,
            self.goal_pos.0 as f64 / self.width as f64,
            self.goal_pos.1 as f64 / self.height as f64,
        ]
    }

    pub fn n_actions(&self) -> usize { 4 }
    pub fn state_dim(&self) -> usize { 4 }

    /// Manhattan distance to goal.
    pub fn distance_to_goal(&self) -> usize {
        let dx = if self.agent_pos.0 > self.goal_pos.0 { self.agent_pos.0 - self.goal_pos.0 } else { self.goal_pos.0 - self.agent_pos.0 };
        let dy = if self.agent_pos.1 > self.goal_pos.1 { self.agent_pos.1 - self.goal_pos.1 } else { self.goal_pos.1 - self.agent_pos.1 };
        dx + dy
    }
}

// ── Multi-Armed Bandit ──────────────────────────────────────────────────

/// K-armed bandit with configurable reward distributions.
#[derive(Debug, Clone)]
pub struct Bandit {
    pub n_arms: usize,
    pub means: Vec<f64>,
    pub stds: Vec<f64>,
    pub step_count: usize,
}

impl Bandit {
    pub fn new(means: Vec<f64>, stds: Vec<f64>) -> Self {
        let n = means.len();
        Bandit { n_arms: n, means, stds, step_count: 0 }
    }

    /// Create a bandit with evenly spaced means.
    pub fn uniform(n_arms: usize) -> Self {
        let means: Vec<f64> = (0..n_arms).map(|i| i as f64 / n_arms as f64).collect();
        let stds = vec![0.5; n_arms];
        Bandit::new(means, stds)
    }

    pub fn reset(&mut self) -> Vec<f64> {
        self.step_count = 0;
        vec![0.0] // Bandit has no meaningful state
    }

    pub fn step(&mut self, action: usize, rng: &mut SimpleRng) -> SimObs {
        self.step_count += 1;
        let reward = if action < self.n_arms {
            let u1 = rng.next_f64().max(1e-15);
            let u2 = rng.next_f64();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            self.means[action] + self.stds[action] * z
        } else {
            0.0
        };

        SimObs { state: vec![action as f64], reward, done: false }
    }

    pub fn optimal_arm(&self) -> usize {
        self.means.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()).unwrap().0
    }

    pub fn regret(&self, action: usize) -> f64 {
        let best = self.means[self.optimal_arm()];
        best - self.means[action.min(self.n_arms - 1)]
    }
}

// ── Continuous Control (CartPole-like) ──────────────────────────────────

/// CartPole environment: balance a pole on a moving cart.
#[derive(Debug, Clone)]
pub struct CartPole {
    pub x: f64,          // cart position
    pub x_dot: f64,      // cart velocity
    pub theta: f64,      // pole angle (radians)
    pub theta_dot: f64,  // pole angular velocity
    pub step_count: usize,
    pub max_steps: usize,
    // Physics constants
    pub gravity: f64,
    pub cart_mass: f64,
    pub pole_mass: f64,
    pub pole_length: f64,
    pub force_mag: f64,
    pub dt: f64,
}

impl CartPole {
    pub fn new() -> Self {
        CartPole {
            x: 0.0, x_dot: 0.0, theta: 0.0, theta_dot: 0.0,
            step_count: 0, max_steps: 500,
            gravity: 9.8, cart_mass: 1.0, pole_mass: 0.1,
            pole_length: 0.5, force_mag: 10.0, dt: 0.02,
        }
    }

    pub fn reset(&mut self) -> Vec<f64> {
        self.x = 0.0;
        self.x_dot = 0.0;
        self.theta = 0.01; // Small initial angle
        self.theta_dot = 0.0;
        self.step_count = 0;
        self.state_vector()
    }

    /// Actions: 0=Push left, 1=Push right
    pub fn step(&mut self, action: usize) -> SimObs {
        self.step_count += 1;
        let force = if action == 1 { self.force_mag } else { -self.force_mag };

        let cos_theta = self.theta.cos();
        let sin_theta = self.theta.sin();
        let total_mass = self.cart_mass + self.pole_mass;
        let pole_ml = self.pole_mass * self.pole_length;

        // Physics (simplified Euler integration)
        let temp = (force + pole_ml * self.theta_dot * self.theta_dot * sin_theta) / total_mass;
        let theta_acc = (self.gravity * sin_theta - cos_theta * temp) /
            (self.pole_length * (4.0 / 3.0 - self.pole_mass * cos_theta * cos_theta / total_mass));
        let x_acc = temp - pole_ml * theta_acc * cos_theta / total_mass;

        self.x += self.dt * self.x_dot;
        self.x_dot += self.dt * x_acc;
        self.theta += self.dt * self.theta_dot;
        self.theta_dot += self.dt * theta_acc;

        let done = self.x.abs() > 2.4
            || self.theta.abs() > 0.209 // ~12 degrees
            || self.step_count >= self.max_steps;

        let reward = if !done || self.step_count >= self.max_steps { 1.0 } else { 0.0 };

        SimObs { state: self.state_vector(), reward, done }
    }

    pub fn state_vector(&self) -> Vec<f64> {
        vec![self.x, self.x_dot, self.theta, self.theta_dot]
    }

    pub fn n_actions(&self) -> usize { 2 }
    pub fn state_dim(&self) -> usize { 4 }
}

// ── Code Optimization Environment ───────────────────────────────────────

/// Environment for optimizing code: actions modify parameters, reward = performance.
#[derive(Debug, Clone)]
pub struct CodeOptEnv {
    pub params: Vec<f64>,
    pub param_names: Vec<String>,
    pub best_score: f64,
    pub current_score: f64,
    pub step_count: usize,
    pub max_steps: usize,
    pub bounds: Vec<(f64, f64)>,
}

impl CodeOptEnv {
    pub fn new(param_names: Vec<String>, bounds: Vec<(f64, f64)>) -> Self {
        let n = param_names.len();
        let params: Vec<f64> = bounds.iter().map(|(lo, hi)| (lo + hi) / 2.0).collect();
        CodeOptEnv {
            params,
            param_names,
            best_score: 0.0,
            current_score: 0.0,
            step_count: 0,
            max_steps: 200,
            bounds,
        }
    }

    pub fn reset(&mut self) -> Vec<f64> {
        self.params = self.bounds.iter().map(|(lo, hi)| (lo + hi) / 2.0).collect();
        self.best_score = 0.0;
        self.current_score = 0.0;
        self.step_count = 0;
        self.params.clone()
    }

    /// Actions encode (param_index, direction): action = param_idx * 2 + direction
    pub fn step(&mut self, action: usize, score_fn: &dyn Fn(&[f64]) -> f64) -> SimObs {
        self.step_count += 1;
        let n = self.params.len();
        let param_idx = (action / 2) % n;
        let direction = if action % 2 == 0 { 0.1 } else { -0.1 };

        self.params[param_idx] += direction;
        // Clamp to bounds
        let (lo, hi) = self.bounds[param_idx];
        self.params[param_idx] = self.params[param_idx].clamp(lo, hi);

        self.current_score = score_fn(&self.params);
        let improvement = self.current_score - self.best_score;
        if self.current_score > self.best_score {
            self.best_score = self.current_score;
        }

        let done = self.step_count >= self.max_steps;

        SimObs {
            state: self.params.clone(),
            reward: improvement,
            done,
        }
    }

    pub fn n_actions(&self) -> usize { self.params.len() * 2 }
    pub fn state_dim(&self) -> usize { self.params.len() }
}

// ── UCB Bandit Solver ───────────────────────────────────────────────────

/// Upper Confidence Bound algorithm for bandits.
#[derive(Debug, Clone)]
pub struct UCBSolver {
    pub n_arms: usize,
    pub counts: Vec<usize>,
    pub values: Vec<f64>,
    pub total_count: usize,
    pub c: f64,  // exploration parameter
}

impl UCBSolver {
    pub fn new(n_arms: usize, c: f64) -> Self {
        UCBSolver { n_arms, counts: vec![0; n_arms], values: vec![0.0; n_arms], total_count: 0, c }
    }

    pub fn select_arm(&self) -> usize {
        // Pull each arm at least once
        for i in 0..self.n_arms {
            if self.counts[i] == 0 { return i; }
        }
        (0..self.n_arms).max_by(|&a, &b| {
            let ucb_a = self.values[a] + self.c * ((self.total_count as f64).ln() / self.counts[a] as f64).sqrt();
            let ucb_b = self.values[b] + self.c * ((self.total_count as f64).ln() / self.counts[b] as f64).sqrt();
            ucb_a.partial_cmp(&ucb_b).unwrap()
        }).unwrap()
    }

    pub fn update(&mut self, arm: usize, reward: f64) {
        self.counts[arm] += 1;
        self.total_count += 1;
        let n = self.counts[arm] as f64;
        self.values[arm] += (reward - self.values[arm]) / n;
    }
}

// ── Common Types ────────────────────────────────────────────────────────

/// Simulation observation (shared by all environments).
#[derive(Debug, Clone)]
pub struct SimObs {
    pub state: Vec<f64>,
    pub reward: f64,
    pub done: bool,
}

/// Simple RNG for environments.
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

// ── FFI Interface ───────────────────────────────────────────────────────

static SIM_STORES: Mutex<Option<HashMap<i64, GridWorld>>> = Mutex::new(None);

fn sim_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, GridWorld>>> {
    SIM_STORES.lock().unwrap()
}

fn next_sim_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_sim_grid_new(width: i64, height: i64, goal_x: i64, goal_y: i64) -> i64 {
    let id = next_sim_id();
    let env = GridWorld::new(width as usize, height as usize, (goal_x as usize, goal_y as usize));
    let mut store = sim_store();
    store.get_or_insert_with(HashMap::new).insert(id, env);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_sim_grid_step(id: i64, action: i64, out_reward: *mut f64, out_done: *mut i64) {
    let mut store = sim_store();
    if let Some(s) = store.as_mut() {
        if let Some(env) = s.get_mut(&id) {
            let obs = env.step(action as usize);
            unsafe {
                *out_reward = obs.reward;
                *out_done = if obs.done { 1 } else { 0 };
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_sim_grid_reset(id: i64) {
    let mut store = sim_store();
    if let Some(s) = store.as_mut() {
        if let Some(env) = s.get_mut(&id) {
            env.reset();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_sim_free(id: i64) {
    let mut store = sim_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_world_basic() {
        let mut gw = GridWorld::new(5, 5, (4, 4));
        let state = gw.reset();
        assert_eq!(state.len(), 4);
        assert_eq!(gw.agent_pos, (0, 0));
    }

    #[test]
    fn test_grid_world_movement() {
        let mut gw = GridWorld::new(5, 5, (4, 4));
        gw.reset();
        gw.step(1); // Right
        assert_eq!(gw.agent_pos, (1, 0));
        gw.step(2); // Down
        assert_eq!(gw.agent_pos, (1, 1));
    }

    #[test]
    fn test_grid_world_wall() {
        let mut gw = GridWorld::new(5, 5, (4, 4));
        gw.add_wall(1, 0);
        gw.reset();
        gw.step(1); // Try to go right into wall
        assert_eq!(gw.agent_pos, (0, 0)); // Blocked!
    }

    #[test]
    fn test_grid_world_goal() {
        let mut gw = GridWorld::new(3, 3, (1, 0));
        gw.reset();
        let obs = gw.step(1); // Right to (1,0) = goal
        assert!(obs.done);
        assert!((obs.reward - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_grid_world_distance() {
        let mut gw = GridWorld::new(5, 5, (4, 4));
        gw.reset();
        assert_eq!(gw.distance_to_goal(), 8);
        gw.step(1); // Right
        assert_eq!(gw.distance_to_goal(), 7);
    }

    #[test]
    fn test_bandit() {
        let mut bandit = Bandit::new(vec![0.3, 0.5, 0.8], vec![0.1, 0.1, 0.1]);
        bandit.reset();
        assert_eq!(bandit.optimal_arm(), 2);
        assert!((bandit.regret(0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_bandit_sample() {
        let mut bandit = Bandit::new(vec![0.0, 5.0], vec![0.1, 0.1]);
        let mut rng = SimpleRng::new(42);
        let mut sum = 0.0;
        for _ in 0..100 {
            let obs = bandit.step(1, &mut rng);
            sum += obs.reward;
        }
        let mean = sum / 100.0;
        assert!((mean - 5.0).abs() < 0.5); // Should be near 5.0
    }

    #[test]
    fn test_cartpole() {
        let mut cp = CartPole::new();
        let state = cp.reset();
        assert_eq!(state.len(), 4);
        assert!((state[2] - 0.01).abs() < 1e-10); // initial theta

        // Take a step
        let obs = cp.step(1);
        assert!(!obs.done);
        assert!((obs.reward - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cartpole_failure() {
        let mut cp = CartPole::new();
        cp.reset();
        // Push consistently in one direction → pole falls
        let mut done = false;
        for _ in 0..200 {
            let obs = cp.step(1);
            if obs.done {
                done = true;
                break;
            }
        }
        assert!(done); // Should fail eventually
    }

    #[test]
    fn test_code_opt_env() {
        let mut env = CodeOptEnv::new(
            vec!["lr".into(), "momentum".into()],
            vec![(0.0, 1.0), (0.0, 1.0)],
        );
        let state = env.reset();
        assert_eq!(state.len(), 2);

        let score_fn = |params: &[f64]| -> f64 {
            -(params[0] - 0.3).powi(2) - (params[1] - 0.9).powi(2)
        };
        let obs = env.step(0, &score_fn); // Increase param 0
        assert!(!obs.done);
    }

    #[test]
    fn test_ucb_solver() {
        let mut ucb = UCBSolver::new(3, 2.0);
        // First 3 selections should try each arm
        assert_eq!(ucb.select_arm(), 0);
        ucb.update(0, 0.5);
        assert_eq!(ucb.select_arm(), 1);
        ucb.update(1, 0.1);
        assert_eq!(ucb.select_arm(), 2);
        ucb.update(2, 0.9);

        // After seeing all, should eventually prefer arm 2
        for _ in 0..20 {
            let arm = ucb.select_arm();
            let reward = if arm == 2 { 0.9 } else { 0.1 };
            ucb.update(arm, reward);
        }
        // Arm 2 should have highest value
        assert!(ucb.values[2] > ucb.values[0]);
        assert!(ucb.values[2] > ucb.values[1]);
    }

    #[test]
    fn test_bandit_uniform() {
        let bandit = Bandit::uniform(5);
        assert_eq!(bandit.n_arms, 5);
        assert_eq!(bandit.optimal_arm(), 4); // highest mean
    }

    #[test]
    fn test_ffi_grid() {
        let id = vitalis_sim_grid_new(5, 5, 4, 4);
        assert!(id > 0);

        vitalis_sim_grid_reset(id);

        let mut reward = 0.0;
        let mut done = 0i64;
        vitalis_sim_grid_step(id, 1, &mut reward, &mut done);
        assert!((reward - (-0.1)).abs() < 1e-10);
        assert_eq!(done, 0);

        vitalis_sim_free(id);
    }

    #[test]
    fn test_grid_boundary() {
        let mut gw = GridWorld::new(3, 3, (2, 2));
        gw.reset();
        gw.step(0); // Up from (0,0) → stays at (0,0)
        assert_eq!(gw.agent_pos, (0, 0));
        gw.step(3); // Left from (0,0) → stays at (0,0)
        assert_eq!(gw.agent_pos, (0, 0));
    }
}
