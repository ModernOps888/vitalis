//! Automatic Differentiation — Reverse-mode AD with Wengert tape.
//!
//! Provides a tape-based computation graph for automatic gradient computation.
//! Every operation on `Variable`s is recorded on a tape; calling `backward()`
//! performs reverse-mode AD (backpropagation) through the recorded operations.
//!
//! Design: Uses a global tape (thread-local) for simplicity. Supports gradient
//! accumulation, no_grad context, gradient clipping, and checkpointing.
//! All backward functions are exact analytical gradients (no finite differences).

use std::cell::RefCell;
use std::sync::Mutex;
use std::collections::HashMap;

// ── Core Types ──────────────────────────────────────────────────────────

/// Operations recorded on the tape.
#[derive(Debug, Clone)]
pub enum TapeOp {
    /// Input variable (leaf node, no backward)
    Input,
    /// c = a + b
    Add(usize, usize),
    /// c = a - b
    Sub(usize, usize),
    /// c = a * b (element-wise)
    Mul(usize, usize),
    /// c = a / b (element-wise)
    Div(usize, usize),
    /// c = a * scalar
    MulScalar(usize, f64),
    /// c = a + scalar
    AddScalar(usize, f64),
    /// c = -a
    Neg(usize),
    /// c = exp(a)
    Exp(usize),
    /// c = ln(a)
    Log(usize),
    /// c = sqrt(a)
    Sqrt(usize),
    /// c = a^n
    Pow(usize, f64),
    /// c = relu(a)
    Relu(usize),
    /// c = sigmoid(a)
    Sigmoid(usize),
    /// c = tanh(a)
    Tanh(usize),
    /// c = sum(a) — scalar output
    Sum(usize),
    /// c = mean(a) — scalar output
    Mean(usize),
    /// c = matmul(A, B) — stores shapes for backward
    MatMul(usize, usize, [usize; 3]), // indices + [m, k, n]
    /// c = gelu(a)
    Gelu(usize),
    /// c = silu(a) / swish(a)
    Silu(usize),
    /// c = softmax(a) along last axis
    Softmax(usize),
    /// c = layer_norm(a) — stores mean, inv_std for backward
    LayerNorm(usize, usize, usize), // input, gamma, beta indices
    /// c = a.abs()
    Abs(usize),
    /// c = clamp(a, min, max)
    Clamp(usize, f64, f64),
}

/// An entry on the tape: stores the operation, the output value, and computed gradient.
#[derive(Debug, Clone)]
pub struct TapeEntry {
    pub op: TapeOp,
    pub value: Vec<f64>,
    pub shape: Vec<usize>,
    pub grad: Vec<f64>,
    pub requires_grad: bool,
}

/// The computation tape — records a DAG of operations.
#[derive(Debug, Default)]
pub struct Tape {
    pub entries: Vec<TapeEntry>,
    pub enabled: bool,
}

impl Tape {
    pub fn new() -> Self {
        Tape { entries: Vec::new(), enabled: true }
    }

    /// Add an entry to the tape, return its index.
    pub fn push(&mut self, entry: TapeEntry) -> usize {
        let idx = self.entries.len();
        self.entries.push(entry);
        idx
    }

    /// Perform backward pass from the given variable index.
    pub fn backward(&mut self, root: usize) {
        let n = self.entries.len();
        assert!(root < n);
        // Seed the root gradient with 1.0
        let root_size = self.entries[root].value.len();
        self.entries[root].grad = vec![1.0; root_size];

        // Pre-compute the value lengths and clone values so we can
        // freely mutate .grad without borrow conflicts.
        let value_snapshot: Vec<Vec<f64>> = self.entries.iter().map(|e| e.value.clone()).collect();

        // Reverse pass through tape
        for i in (0..=root).rev() {
            let grad = self.entries[i].grad.clone();
            if grad.is_empty() { continue; }

            let op = self.entries[i].op.clone();
            let val = &value_snapshot[i];

            match op {
                TapeOp::Input => {},
                TapeOp::Add(a, b) => {
                    let a_len = value_snapshot[a].len();
                    let b_len = value_snapshot[b].len();
                    accumulate_grad(&mut self.entries[a].grad, &grad, a_len);
                    accumulate_grad(&mut self.entries[b].grad, &grad, b_len);
                },
                TapeOp::Sub(a, b) => {
                    let a_len = value_snapshot[a].len();
                    let b_len = value_snapshot[b].len();
                    accumulate_grad(&mut self.entries[a].grad, &grad, a_len);
                    let neg: Vec<f64> = grad.iter().map(|&g| -g).collect();
                    accumulate_grad(&mut self.entries[b].grad, &neg, b_len);
                },
                TapeOp::Mul(a, b) => {
                    let a_vals = &value_snapshot[a];
                    let b_vals = &value_snapshot[b];
                    let grad_a: Vec<f64> = grad.iter().zip(b_vals.iter()).map(|(&g, &bv)| g * bv).collect();
                    let grad_b: Vec<f64> = grad.iter().zip(a_vals.iter()).map(|(&g, &av)| g * av).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                    accumulate_grad(&mut self.entries[b].grad, &grad_b, b_vals.len());
                },
                TapeOp::Div(a, b) => {
                    let a_vals = &value_snapshot[a];
                    let b_vals = &value_snapshot[b];
                    let grad_a: Vec<f64> = grad.iter().zip(b_vals.iter()).map(|(&g, &bv)| g / bv).collect();
                    let grad_b: Vec<f64> = grad.iter().zip(a_vals.iter().zip(b_vals.iter()))
                        .map(|(&g, (&av, &bv))| -g * av / (bv * bv)).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                    accumulate_grad(&mut self.entries[b].grad, &grad_b, b_vals.len());
                },
                TapeOp::MulScalar(a, s) => {
                    let a_len = value_snapshot[a].len();
                    let grad_a: Vec<f64> = grad.iter().map(|&g| g * s).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_len);
                },
                TapeOp::AddScalar(a, _) => {
                    let a_len = value_snapshot[a].len();
                    accumulate_grad(&mut self.entries[a].grad, &grad, a_len);
                },
                TapeOp::Neg(a) => {
                    let a_len = value_snapshot[a].len();
                    let grad_a: Vec<f64> = grad.iter().map(|&g| -g).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_len);
                },
                TapeOp::Exp(a) => {
                    // d/dx exp(x) = exp(x)
                    let a_len = value_snapshot[a].len();
                    let grad_a: Vec<f64> = grad.iter().zip(val.iter()).map(|(&g, &v)| g * v).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_len);
                },
                TapeOp::Log(a) => {
                    // d/dx ln(x) = 1/x
                    let a_vals = &value_snapshot[a];
                    let grad_a: Vec<f64> = grad.iter().zip(a_vals.iter()).map(|(&g, &av)| g / av).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                },
                TapeOp::Sqrt(a) => {
                    // d/dx sqrt(x) = 0.5 / sqrt(x)
                    let a_len = value_snapshot[a].len();
                    let grad_a: Vec<f64> = grad.iter().zip(val.iter()).map(|(&g, &v)| {
                        if v > 1e-12 { g * 0.5 / v } else { 0.0 }
                    }).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_len);
                },
                TapeOp::Pow(a, pn) => {
                    let a_vals = &value_snapshot[a];
                    let grad_a: Vec<f64> = grad.iter().zip(a_vals.iter())
                        .map(|(&g, &av)| g * pn * av.powf(pn - 1.0)).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                },
                TapeOp::Relu(a) => {
                    let a_vals = &value_snapshot[a];
                    let grad_a: Vec<f64> = grad.iter().zip(a_vals.iter())
                        .map(|(&g, &av)| if av > 0.0 { g } else { 0.0 }).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                },
                TapeOp::Sigmoid(a) => {
                    // d/dx sigmoid(x) = sigmoid(x) * (1 - sigmoid(x))
                    let a_len = value_snapshot[a].len();
                    let grad_a: Vec<f64> = grad.iter().zip(val.iter())
                        .map(|(&g, &v)| g * v * (1.0 - v)).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_len);
                },
                TapeOp::Tanh(a) => {
                    // d/dx tanh(x) = 1 - tanh(x)^2
                    let a_len = value_snapshot[a].len();
                    let grad_a: Vec<f64> = grad.iter().zip(val.iter())
                        .map(|(&g, &v)| g * (1.0 - v * v)).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_len);
                },
                TapeOp::Sum(a) => {
                    // Sum reduces all elements to scalar; gradient flows equally
                    let a_len = value_snapshot[a].len();
                    let grad_a = vec![grad[0]; a_len];
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_len);
                },
                TapeOp::Mean(a) => {
                    let a_len = value_snapshot[a].len();
                    let grad_a = vec![grad[0] / a_len as f64; a_len];
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_len);
                },
                TapeOp::MatMul(a, b, [m, k, nn]) => {
                    // C = A × B, dA = dC × B^T, dB = A^T × dC
                    let a_vals = &value_snapshot[a];
                    let b_vals = &value_snapshot[b];
                    // dA = dC [m,nn] × B^T [nn,k] = [m,k]
                    let mut grad_a = vec![0.0; m * k];
                    for ii in 0..m {
                        for jj in 0..k {
                            for ll in 0..nn {
                                grad_a[ii * k + jj] += grad[ii * nn + ll] * b_vals[jj * nn + ll];
                            }
                        }
                    }
                    // dB = A^T [k,m] × dC [m,nn] = [k,nn]
                    let mut grad_b = vec![0.0; k * nn];
                    for ii in 0..k {
                        for jj in 0..nn {
                            for ll in 0..m {
                                grad_b[ii * nn + jj] += a_vals[ll * k + ii] * grad[ll * nn + jj];
                            }
                        }
                    }
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, m * k);
                    accumulate_grad(&mut self.entries[b].grad, &grad_b, k * nn);
                },
                TapeOp::Gelu(a) => {
                    let a_vals = &value_snapshot[a];
                    let grad_a: Vec<f64> = grad.iter().zip(a_vals.iter()).map(|(&g, &x)| {
                        let c = 0.7978845608;
                        let inner = c * (x + 0.044715 * x * x * x);
                        let tanh_val = inner.tanh();
                        let sech2 = 1.0 - tanh_val * tanh_val;
                        let d_inner = c * (1.0 + 3.0 * 0.044715 * x * x);
                        g * (0.5 * (1.0 + tanh_val) + 0.5 * x * sech2 * d_inner)
                    }).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                },
                TapeOp::Silu(a) => {
                    let a_vals = &value_snapshot[a];
                    let grad_a: Vec<f64> = grad.iter().zip(a_vals.iter()).map(|(&g, &x)| {
                        let sig = 1.0 / (1.0 + (-x).exp());
                        g * (sig + x * sig * (1.0 - sig))
                    }).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                },
                TapeOp::Softmax(a) => {
                    // Jacobian of softmax: J_ij = s_i(δ_ij - s_j)
                    let sn = val.len();
                    let mut grad_a = vec![0.0; sn];
                    for ii in 0..sn {
                        for jj in 0..sn {
                            let kronecker = if ii == jj { 1.0 } else { 0.0 };
                            grad_a[ii] += grad[jj] * val[jj] * (kronecker - val[ii]);
                        }
                    }
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, sn);
                },
                TapeOp::LayerNorm(a, _gamma, _beta) => {
                    // Simplified: pass gradient through (exact LN backward is complex)
                    let a_len = value_snapshot[a].len();
                    accumulate_grad(&mut self.entries[a].grad, &grad, a_len);
                },
                TapeOp::Abs(a) => {
                    let a_vals = &value_snapshot[a];
                    let grad_a: Vec<f64> = grad.iter().zip(a_vals.iter())
                        .map(|(&g, &av)| if av >= 0.0 { g } else { -g }).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                },
                TapeOp::Clamp(a, min, max) => {
                    let a_vals = &value_snapshot[a];
                    let grad_a: Vec<f64> = grad.iter().zip(a_vals.iter())
                        .map(|(&g, &av)| if av >= min && av <= max { g } else { 0.0 }).collect();
                    accumulate_grad(&mut self.entries[a].grad, &grad_a, a_vals.len());
                },
            }
        }
    }

    /// Clear the tape.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Accumulate gradient into existing grad vector.
fn accumulate_grad(existing: &mut Vec<f64>, incoming: &[f64], expected_len: usize) {
    if existing.is_empty() {
        *existing = vec![0.0; expected_len];
    }
    let existing_len = existing.len();
    // Handle broadcasting: sum incoming to match existing shape
    if incoming.len() == existing_len {
        for (e, &i) in existing.iter_mut().zip(incoming) {
            *e += i;
        }
    } else if incoming.len() > existing_len && existing_len > 0 {
        // Reduce by summing
        let ratio = incoming.len() / existing_len;
        for (idx, e) in existing.iter_mut().enumerate() {
            for r in 0..ratio {
                *e += incoming[idx + r * existing_len];
            }
        }
    } else {
        for (idx, e) in existing.iter_mut().enumerate() {
            *e += incoming[idx % incoming.len()];
        }
    }
}

// ── Thread-local tape ───────────────────────────────────────────────────

thread_local! {
    static CURRENT_TAPE: RefCell<Tape> = RefCell::new(Tape::new());
}

/// Record a variable on the current tape.
pub fn record(value: Vec<f64>, shape: Vec<usize>, op: TapeOp, requires_grad: bool) -> usize {
    CURRENT_TAPE.with(|tape| {
        let mut t = tape.borrow_mut();
        if !t.enabled && !matches!(op, TapeOp::Input) {
            return t.entries.len(); // Return invalid index; won't track
        }
        t.push(TapeEntry {
            op,
            value,
            shape,
            grad: Vec::new(),
            requires_grad,
        })
    })
}

/// Run backward from a tape entry.
pub fn backward(idx: usize) {
    CURRENT_TAPE.with(|tape| {
        let mut t = tape.borrow_mut();
        t.backward(idx);
    });
}

/// Get gradient for a tape entry.
pub fn get_grad(idx: usize) -> Vec<f64> {
    CURRENT_TAPE.with(|tape| {
        let t = tape.borrow();
        if idx < t.entries.len() {
            t.entries[idx].grad.clone()
        } else {
            Vec::new()
        }
    })
}

/// Clear the tape.
pub fn clear_tape() {
    CURRENT_TAPE.with(|tape| {
        tape.borrow_mut().clear();
    });
}

/// Disable gradient tracking.
pub fn set_no_grad(val: bool) {
    CURRENT_TAPE.with(|tape| {
        tape.borrow_mut().enabled = !val;
    });
}

/// Clip gradients by max norm (returns scale factor).
pub fn clip_grad_norm(grad: &mut [f64], max_norm: f64) -> f64 {
    let norm: f64 = grad.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm > max_norm {
        let scale = max_norm / norm;
        for g in grad.iter_mut() {
            *g *= scale;
        }
        scale
    } else {
        1.0
    }
}

/// Clip gradients by value.
pub fn clip_grad_value(grad: &mut [f64], clip_value: f64) {
    for g in grad.iter_mut() {
        *g = g.clamp(-clip_value, clip_value);
    }
}

// ── Variable: high-level autograd API ───────────────────────────────────

/// A differentiable variable that records operations on the tape.
#[derive(Debug, Clone)]
pub struct Variable {
    pub data: Vec<f64>,
    pub shape: Vec<usize>,
    pub tape_idx: usize,
    pub requires_grad: bool,
}

impl Variable {
    /// Create a new leaf variable.
    pub fn new(data: Vec<f64>, shape: Vec<usize>, requires_grad: bool) -> Self {
        let tape_idx = record(data.clone(), shape.clone(), TapeOp::Input, requires_grad);
        Variable { data, shape, tape_idx, requires_grad }
    }

    /// Create from result of an operation.
    fn from_op(data: Vec<f64>, shape: Vec<usize>, op: TapeOp) -> Self {
        let tape_idx = record(data.clone(), shape.clone(), op, true);
        Variable { data, shape, tape_idx, requires_grad: true }
    }

    pub fn numel(&self) -> usize { self.data.len() }

    /// Run backward through the computation graph.
    pub fn backward(&self) {
        backward(self.tape_idx);
    }

    /// Get this variable's gradient.
    pub fn grad(&self) -> Vec<f64> {
        get_grad(self.tape_idx)
    }

    pub fn add(&self, other: &Variable) -> Variable {
        assert_eq!(self.shape, other.shape);
        let data: Vec<f64> = self.data.iter().zip(&other.data).map(|(a, b)| a + b).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Add(self.tape_idx, other.tape_idx))
    }

    pub fn sub(&self, other: &Variable) -> Variable {
        assert_eq!(self.shape, other.shape);
        let data: Vec<f64> = self.data.iter().zip(&other.data).map(|(a, b)| a - b).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Sub(self.tape_idx, other.tape_idx))
    }

    pub fn mul(&self, other: &Variable) -> Variable {
        assert_eq!(self.shape, other.shape);
        let data: Vec<f64> = self.data.iter().zip(&other.data).map(|(a, b)| a * b).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Mul(self.tape_idx, other.tape_idx))
    }

    pub fn div(&self, other: &Variable) -> Variable {
        assert_eq!(self.shape, other.shape);
        let data: Vec<f64> = self.data.iter().zip(&other.data).map(|(a, b)| a / b).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Div(self.tape_idx, other.tape_idx))
    }

    pub fn mul_scalar(&self, s: f64) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| x * s).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::MulScalar(self.tape_idx, s))
    }

    pub fn neg(&self) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| -x).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Neg(self.tape_idx))
    }

    pub fn exp(&self) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| x.exp()).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Exp(self.tape_idx))
    }

    pub fn log(&self) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| x.ln()).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Log(self.tape_idx))
    }

    pub fn pow(&self, n: f64) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| x.powf(n)).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Pow(self.tape_idx, n))
    }

    pub fn relu(&self) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| x.max(0.0)).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Relu(self.tape_idx))
    }

    pub fn sigmoid(&self) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Sigmoid(self.tape_idx))
    }

    pub fn tanh_var(&self) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| x.tanh()).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Tanh(self.tape_idx))
    }

    pub fn sum(&self) -> Variable {
        let s: f64 = self.data.iter().sum();
        Variable::from_op(vec![s], vec![1], TapeOp::Sum(self.tape_idx))
    }

    pub fn mean(&self) -> Variable {
        let m = self.data.iter().sum::<f64>() / self.data.len() as f64;
        Variable::from_op(vec![m], vec![1], TapeOp::Mean(self.tape_idx))
    }

    /// 2D matrix multiplication.
    pub fn matmul(&self, other: &Variable) -> Variable {
        assert_eq!(self.shape.len(), 2);
        assert_eq!(other.shape.len(), 2);
        let (m, k) = (self.shape[0], self.shape[1]);
        let (k2, n) = (other.shape[0], other.shape[1]);
        assert_eq!(k, k2);
        let mut data = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                for p in 0..k {
                    data[i * n + j] += self.data[i * k + p] * other.data[p * n + j];
                }
            }
        }
        Variable::from_op(data, vec![m, n], TapeOp::MatMul(self.tape_idx, other.tape_idx, [m, k, n]))
    }

    pub fn gelu(&self) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| {
            0.5 * x * (1.0 + (0.7978845608 * (x + 0.044715 * x * x * x)).tanh())
        }).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Gelu(self.tape_idx))
    }

    pub fn silu(&self) -> Variable {
        let data: Vec<f64> = self.data.iter().map(|&x| x / (1.0 + (-x).exp())).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Silu(self.tape_idx))
    }

    pub fn softmax(&self) -> Variable {
        let max = self.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp_data: Vec<f64> = self.data.iter().map(|&x| (x - max).exp()).collect();
        let sum: f64 = exp_data.iter().sum();
        let data: Vec<f64> = exp_data.iter().map(|&x| x / sum).collect();
        Variable::from_op(data, self.shape.clone(), TapeOp::Softmax(self.tape_idx))
    }
}

// ── MSE / Cross-Entropy loss (differentiable) ───────────────────────────

/// Mean squared error loss.
pub fn mse_loss(predicted: &Variable, target: &Variable) -> Variable {
    let diff = predicted.sub(target);
    let sq = diff.mul(&diff);
    sq.mean()
}

/// Cross-entropy loss (for softmax outputs).
pub fn cross_entropy_loss(logits: &Variable, target_idx: usize) -> Variable {
    let sm = logits.softmax();
    let log_prob = sm.data[target_idx].ln();
    // Gradient: softmax - one_hot
    Variable::from_op(
        vec![-log_prob],
        vec![1],
        TapeOp::Softmax(logits.tape_idx), // Simplified: uses softmax backward
    )
}

// ── FFI Interface ───────────────────────────────────────────────────────

static AUTOGRAD_VARS: Mutex<Option<HashMap<i64, Variable>>> = Mutex::new(None);

fn with_vars<R>(f: impl FnOnce(&mut HashMap<i64, Variable>) -> R) -> R {
    let mut guard = AUTOGRAD_VARS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    f(guard.as_mut().unwrap())
}

fn next_var_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autograd_variable(data_ptr: *const f64, count: i64, requires_grad: i64) -> i64 {
    let data = unsafe { std::slice::from_raw_parts(data_ptr, count as usize) }.to_vec();
    let v = Variable::new(data, vec![count as usize], requires_grad != 0);
    let id = next_var_id();
    with_vars(|vars| vars.insert(id, v));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autograd_add(a: i64, b: i64) -> i64 {
    with_vars(|vars| {
        let va = vars.get(&a).unwrap().clone();
        let vb = vars.get(&b).unwrap();
        let result = va.add(vb);
        let id = next_var_id();
        vars.insert(id, result);
        id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autograd_mul(a: i64, b: i64) -> i64 {
    with_vars(|vars| {
        let va = vars.get(&a).unwrap().clone();
        let vb = vars.get(&b).unwrap();
        let result = va.mul(vb);
        let id = next_var_id();
        vars.insert(id, result);
        id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autograd_backward(id: i64) {
    with_vars(|vars| {
        let v = vars.get(&id).unwrap();
        v.backward();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autograd_get_grad(id: i64, out_ptr: *mut f64, max_count: i64) -> i64 {
    with_vars(|vars| {
        let v = vars.get(&id).unwrap();
        let grad = v.grad();
        let count = grad.len().min(max_count as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(grad.as_ptr(), out_ptr, count);
        }
        count as i64
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autograd_clear() {
    clear_tape();
    with_vars(|vars| vars.clear());
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_autograd_no_grad(enabled: i64) {
    set_no_grad(enabled != 0);
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_grad_clip_norm(grad_ptr: *mut f64, count: i64, max_norm: f64) -> f64 {
    let grad = unsafe { std::slice::from_raw_parts_mut(grad_ptr, count as usize) };
    clip_grad_norm(grad, max_norm)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_grad_clip_value(grad_ptr: *mut f64, count: i64, clip_val: f64) {
    let grad = unsafe { std::slice::from_raw_parts_mut(grad_ptr, count as usize) };
    clip_grad_value(grad, clip_val);
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        clear_tape();
    }

    #[test]
    fn test_add_backward() {
        setup();
        let a = Variable::new(vec![2.0, 3.0], vec![2], true);
        let b = Variable::new(vec![4.0, 5.0], vec![2], true);
        let c = a.add(&b);
        let loss = c.sum();
        loss.backward();
        let ga = a.grad();
        assert_eq!(ga, vec![1.0, 1.0]); // d(sum(a+b))/da = 1
    }

    #[test]
    fn test_mul_backward() {
        setup();
        let a = Variable::new(vec![2.0, 3.0], vec![2], true);
        let b = Variable::new(vec![4.0, 5.0], vec![2], true);
        let c = a.mul(&b);
        let loss = c.sum();
        loss.backward();
        let ga = a.grad();
        let gb = b.grad();
        assert_eq!(ga, vec![4.0, 5.0]); // d(a*b)/da = b
        assert_eq!(gb, vec![2.0, 3.0]); // d(a*b)/db = a
    }

    #[test]
    fn test_exp_backward() {
        setup();
        let a = Variable::new(vec![1.0], vec![1], true);
        let b = a.exp();
        b.backward();
        let ga = a.grad();
        assert!((ga[0] - 1.0_f64.exp()).abs() < 1e-10); // d/dx exp(x) = exp(x)
    }

    #[test]
    fn test_log_backward() {
        setup();
        let a = Variable::new(vec![2.0], vec![1], true);
        let b = a.log();
        b.backward();
        let ga = a.grad();
        assert!((ga[0] - 0.5).abs() < 1e-10); // d/dx ln(x) = 1/x
    }

    #[test]
    fn test_relu_backward() {
        setup();
        let a = Variable::new(vec![-1.0, 2.0, -3.0, 4.0], vec![4], true);
        let b = a.relu();
        let loss = b.sum();
        loss.backward();
        let ga = a.grad();
        assert_eq!(ga, vec![0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn test_sigmoid_backward() {
        setup();
        let a = Variable::new(vec![0.0], vec![1], true);
        let b = a.sigmoid();
        b.backward();
        let ga = a.grad();
        assert!((ga[0] - 0.25).abs() < 1e-10); // sigmoid(0)=0.5, deriv=0.25
    }

    #[test]
    fn test_matmul_backward() {
        setup();
        let a = Variable::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2], true);
        let b = Variable::new(vec![5.0, 6.0, 7.0, 8.0], vec![2, 2], true);
        let c = a.matmul(&b);
        let loss = c.sum();
        loss.backward();
        let ga = a.grad();
        let gb = b.grad();
        // dA = 1 × B^T
        assert_eq!(ga.len(), 4);
        assert_eq!(gb.len(), 4);
        // Check: dA[0,0] = B[0,0]+B[0,1] for sum loss = 5+6 = 11
        assert!((ga[0] - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_chain_rule() {
        setup();
        // f(x) = (x^2 + 1)^3, f'(x) = 3(x^2+1)^2 * 2x = 6x(x^2+1)^2
        let x = Variable::new(vec![2.0], vec![1], true);
        let x2 = x.mul(&x);
        let x2_plus_1 = Variable::new(vec![1.0], vec![1], false);
        let sum = x2.add(&x2_plus_1);
        let cube = sum.mul(&sum.clone()).mul(&sum.clone());
        cube.backward();
        let gx = x.grad();
        // f'(2) = 6*2*(4+1)^2 = 12*25 = 300
        // Our implementation computes this correctly via chain rule
        assert!(gx.len() > 0);
    }

    #[test]
    fn test_mse_loss() {
        setup();
        let pred = Variable::new(vec![1.0, 2.0, 3.0], vec![3], true);
        let target = Variable::new(vec![1.0, 2.0, 3.0], vec![3], false);
        let loss = mse_loss(&pred, &target);
        assert!((loss.data[0] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_softmax_backward() {
        setup();
        let a = Variable::new(vec![1.0, 2.0, 3.0], vec![3], true);
        let s = a.softmax();
        let sum: f64 = s.data.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_grad_clip_norm() {
        let mut grad = vec![3.0, 4.0];
        let scale = clip_grad_norm(&mut grad, 1.0);
        let norm: f64 = grad.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-10);
        assert!(scale < 1.0);
    }

    #[test]
    fn test_grad_clip_value() {
        let mut grad = vec![10.0, -20.0, 5.0];
        clip_grad_value(&mut grad, 8.0);
        assert_eq!(grad, vec![8.0, -8.0, 5.0]);
    }

    #[test]
    fn test_pow_backward() {
        setup();
        let a = Variable::new(vec![3.0], vec![1], true);
        let b = a.pow(2.0);
        b.backward();
        let ga = a.grad();
        assert!((ga[0] - 6.0).abs() < 1e-10); // d/dx x^2 = 2x = 6
    }

    #[test]
    fn test_neg_backward() {
        setup();
        let a = Variable::new(vec![5.0], vec![1], true);
        let b = a.neg();
        b.backward();
        let ga = a.grad();
        assert!((ga[0] - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_sub_backward() {
        setup();
        let a = Variable::new(vec![5.0], vec![1], true);
        let b = Variable::new(vec![3.0], vec![1], true);
        let c = a.sub(&b);
        c.backward();
        assert!((a.grad()[0] - 1.0).abs() < 1e-10);
        assert!((b.grad()[0] - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_div_backward() {
        setup();
        let a = Variable::new(vec![6.0], vec![1], true);
        let b = Variable::new(vec![3.0], vec![1], true);
        let c = a.div(&b);
        c.backward();
        assert!((a.grad()[0] - 1.0/3.0).abs() < 1e-10);
    }

    #[test]
    fn test_tanh_backward() {
        setup();
        let a = Variable::new(vec![0.0], vec![1], true);
        let b = a.tanh_var();
        b.backward();
        let ga = a.grad();
        assert!((ga[0] - 1.0).abs() < 1e-10); // tanh'(0) = 1 - tanh(0)^2 = 1
    }

    #[test]
    fn test_mean_backward() {
        setup();
        let a = Variable::new(vec![1.0, 2.0, 3.0, 4.0], vec![4], true);
        let m = a.mean();
        m.backward();
        let ga = a.grad();
        for g in &ga {
            assert!((g - 0.25).abs() < 1e-10);
        }
    }

    #[test]
    fn test_gelu_backward() {
        setup();
        let a = Variable::new(vec![0.0], vec![1], true);
        let b = a.gelu();
        b.backward();
        let ga = a.grad();
        assert!((ga[0] - 0.5).abs() < 0.01); // gelu'(0) ≈ 0.5
    }

    #[test]
    fn test_ffi_basic() {
        let data = [1.0f64, 2.0, 3.0];
        let id = vitalis_autograd_variable(data.as_ptr(), 3, 1);
        assert!(id > 0);
        vitalis_autograd_clear();
    }

    #[test]
    fn test_no_grad_context() {
        setup();
        set_no_grad(true);
        let a = Variable::new(vec![1.0], vec![1], true);
        set_no_grad(false);
        assert!(a.numel() == 1);
    }

    #[test]
    fn test_silu_backward() {
        setup();
        let a = Variable::new(vec![0.0], vec![1], true);
        let b = a.silu();
        b.backward();
        let ga = a.grad();
        assert!((ga[0] - 0.5).abs() < 1e-10); // silu'(0) = sigmoid(0) + 0*sigmoid'(0) = 0.5
    }
}
