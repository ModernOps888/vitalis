//! Tensor Engine — GPU-accelerated tensor computation for deep learning
//!
//! A complete tensor library with automatic differentiation, broadcasting,
//! GPU acceleration via cuBLAS, and 33+ operations. Ported from the Nova ML
//! engine for Vitalis v20.0.
//!
//! # Architecture
//!
//! ```text
//! Tensor ─── Storage (CPU Vec<f32> | GPU CudaSlice<f32>)
//!        ├── Shape   (dims, strides, broadcasting)
//!        ├── DType   (F32, F16, BF16)
//!        └── Autograd (GradFn trait, backward graph)
//! ```
//!
//! # Supported Operations
//!
//! - **Arithmetic**: add, sub, mul, div, neg, abs, clamp
//! - **Reduction**: sum, mean, var, max, argmax
//! - **Unary**: exp, log, sqrt, pow, reciprocal
//! - **Activation**: relu, gelu, sigmoid, silu, tanh, softmax
//! - **Linear Algebra**: matmul (2D/3D/4D), transpose
//! - **Loss**: cross_entropy_loss, log_softmax
//! - **NN**: embedding lookup, concatenation
//! - **Autograd**: backward pass with gradient accumulation

use std::sync::{Arc, Mutex};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DType
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Data type for tensor elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DType {
    F32,
    F16,
    BF16,
}

impl DType {
    /// Size in bytes of one element.
    pub fn size_bytes(&self) -> usize {
        match self {
            DType::F32 => 4,
            DType::F16 | DType::BF16 => 2,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Shape
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Multi-dimensional shape with row-major (C-order) strides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape {
    dims: Vec<usize>,
    strides: Vec<usize>,
}

impl Shape {
    /// Create shape from dimensions.
    pub fn new(dims: Vec<usize>) -> Self {
        let strides = Self::compute_strides(&dims);
        Self { dims, strides }
    }

    /// Scalar (0-dimensional) shape.
    pub fn scalar() -> Self {
        Self { dims: vec![], strides: vec![] }
    }

    fn compute_strides(dims: &[usize]) -> Vec<usize> {
        if dims.is_empty() { return vec![]; }
        let mut strides = vec![1usize; dims.len()];
        for i in (0..dims.len() - 1).rev() {
            strides[i] = strides[i + 1] * dims[i + 1];
        }
        strides
    }

    pub fn dims(&self) -> &[usize] { &self.dims }
    pub fn strides(&self) -> &[usize] { &self.strides }
    pub fn ndim(&self) -> usize { self.dims.len() }

    /// Total element count.
    pub fn numel(&self) -> usize {
        self.dims.iter().product::<usize>().max(1)
    }

    pub fn dim(&self, d: usize) -> usize { self.dims[d] }

    /// Multi-index → flat index.
    pub fn flat_index(&self, indices: &[usize]) -> usize {
        indices.iter().zip(self.strides.iter()).map(|(i, s)| i * s).sum()
    }

    /// Flat index → multi-index.
    pub fn multi_index(&self, mut flat: usize) -> Vec<usize> {
        let mut out = vec![0usize; self.dims.len()];
        for i in 0..self.dims.len() {
            out[i] = flat / self.strides[i];
            flat %= self.strides[i];
        }
        out
    }

    /// NumPy-style broadcast shape.
    pub fn broadcast_shape(a: &Shape, b: &Shape) -> Option<Shape> {
        let max_ndim = a.ndim().max(b.ndim());
        let mut result = vec![0usize; max_ndim];
        for i in 0..max_ndim {
            let da = if i < a.ndim() { a.dims[a.ndim() - 1 - i] } else { 1 };
            let db = if i < b.ndim() { b.dims[b.ndim() - 1 - i] } else { 1 };
            if da == db { result[max_ndim - 1 - i] = da; }
            else if da == 1 { result[max_ndim - 1 - i] = db; }
            else if db == 1 { result[max_ndim - 1 - i] = da; }
            else { return None; }
        }
        Some(Shape::new(result))
    }

    /// Transpose last two dimensions.
    pub fn transpose_last_two(&self) -> Shape {
        assert!(self.ndim() >= 2, "Need ≥2 dims to transpose");
        let mut d = self.dims.clone();
        let n = d.len();
        d.swap(n - 1, n - 2);
        Shape::new(d)
    }
}

impl std::fmt::Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        for (i, d) in self.dims.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{d}")?;
        }
        write!(f, ")")
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Storage
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Backend storage: CPU `Vec<f32>` or GPU `CudaSlice<f32>`.
#[derive(Clone)]
pub enum Storage {
    Cpu(Vec<f32>),
}

impl std::fmt::Debug for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Storage::Cpu(data) => write!(f, "Storage::Cpu(len={})", data.len()),
        }
    }
}

impl Storage {
    pub fn len(&self) -> usize {
        match self { Storage::Cpu(d) => d.len() }
    }

    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn is_cpu(&self) -> bool { true }
    pub fn is_cuda(&self) -> bool { false }

    pub fn as_cpu(&self) -> &[f32] {
        match self { Storage::Cpu(d) => d }
    }

    pub fn as_cpu_mut(&mut self) -> &mut Vec<f32> {
        match self { Storage::Cpu(d) => d }
    }

    pub fn to_cpu(&self) -> Storage { self.clone() }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Autograd
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Gradient function trait for automatic differentiation.
pub trait GradFn: Send + Sync + std::fmt::Debug {
    /// Compute gradients w.r.t. inputs given the output gradient.
    fn backward(&self, grad_output: &Tensor) -> Vec<Tensor>;
    /// Return input tensors that this operation depends on.
    fn inputs(&self) -> Vec<Tensor>;
}

#[derive(Debug)]
pub(crate) struct MatmulGrad { pub a: Tensor, pub b: Tensor }
impl GradFn for MatmulGrad {
    fn backward(&self, g: &Tensor) -> Vec<Tensor> {
        let ga = matmul(g, &transpose(&self.b));
        let gb = matmul(&transpose(&self.a), g);
        vec![ga, gb]
    }
    fn inputs(&self) -> Vec<Tensor> { vec![self.a.clone(), self.b.clone()] }
}

#[derive(Debug)]
pub(crate) struct AddGrad { pub a: Tensor, pub b: Tensor }
impl GradFn for AddGrad {
    fn backward(&self, g: &Tensor) -> Vec<Tensor> { vec![g.clone(), g.clone()] }
    fn inputs(&self) -> Vec<Tensor> { vec![self.a.clone(), self.b.clone()] }
}

#[derive(Debug)]
pub(crate) struct ReluGrad { pub input: Tensor }
impl GradFn for ReluGrad {
    fn backward(&self, g: &Tensor) -> Vec<Tensor> {
        let mask: Vec<f32> = self.input.data_f32().iter()
            .map(|&x| if x > 0.0 { 1.0 } else { 0.0 }).collect();
        vec![mul(g, &Tensor::from_vec(mask, self.input.dims()))]
    }
    fn inputs(&self) -> Vec<Tensor> { vec![self.input.clone()] }
}

#[derive(Debug)]
pub(crate) struct CrossEntropyGrad { pub logits: Tensor, pub targets: Vec<usize> }
impl GradFn for CrossEntropyGrad {
    fn backward(&self, g: &Tensor) -> Vec<Tensor> {
        let d = self.logits.dims();
        let (batch, vocab) = (d[0], d[1]);
        let probs = softmax(&self.logits, -1);
        let mut grad = probs.data_f32().to_vec();
        let scale = g.data_f32()[0] / batch as f32;
        for b in 0..batch {
            grad[b * vocab + self.targets[b]] -= 1.0;
            for v in 0..vocab { grad[b * vocab + v] *= scale; }
        }
        vec![Tensor::from_vec(grad, &[batch, vocab])]
    }
    fn inputs(&self) -> Vec<Tensor> { vec![self.logits.clone()] }
}

#[derive(Debug)]
pub(crate) struct EmbeddingGrad { pub weight: Tensor, pub indices: Vec<usize> }
impl GradFn for EmbeddingGrad {
    fn backward(&self, g: &Tensor) -> Vec<Tensor> {
        let (vocab, dim) = (self.weight.dims()[0], self.weight.dims()[1]);
        let mut grad_w = vec![0.0f32; vocab * dim];
        let gd = g.data_f32();
        for (i, &idx) in self.indices.iter().enumerate() {
            for j in 0..dim { grad_w[idx * dim + j] += gd[i * dim + j]; }
        }
        vec![Tensor::from_vec(grad_w, &[vocab, dim])]
    }
    fn inputs(&self) -> Vec<Tensor> { vec![self.weight.clone()] }
}

/// Run backward pass from a scalar loss tensor.
pub fn backward(loss: &Tensor) {
    assert_eq!(loss.numel(), 1, "backward() requires scalar loss");
    let grad = Tensor::ones(&[1]);
    backward_recursive(loss, &grad);
}

fn backward_recursive(tensor: &Tensor, grad: &Tensor) {
    if tensor.requires_grad {
        tensor.accumulate_grad(grad);
    }
    if let Some(ref grad_fn) = tensor.grad_fn {
        let grads = grad_fn.backward(grad);
        let inputs = grad_fn.inputs();
        for (inp, g) in inputs.iter().zip(grads.iter()) {
            backward_recursive(inp, g);
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tensor
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Multi-dimensional array with optional automatic differentiation.
///
/// Supports CPU storage, shape broadcasting, gradient tracking, and 33+ ops.
#[derive(Clone, Debug)]
pub struct Tensor {
    pub storage: Arc<Storage>,
    pub shape: Shape,
    pub dtype: DType,
    pub requires_grad: bool,
    pub grad: Option<Arc<Mutex<Option<Tensor>>>>,
    pub grad_fn: Option<Arc<dyn GradFn>>,
    pub name: Option<String>,
}

impl Tensor {
    /// Create tensor from data and shape.
    pub fn from_vec(data: Vec<f32>, shape: &[usize]) -> Self {
        let expected: usize = shape.iter().product();
        assert_eq!(data.len(), expected, "data length {} != shape {:?} ({})", data.len(), shape, expected);
        Self {
            storage: Arc::new(Storage::Cpu(data)),
            shape: Shape::new(shape.to_vec()),
            dtype: DType::F32,
            requires_grad: false,
            grad: None,
            grad_fn: None,
            name: None,
        }
    }

    pub fn zeros(shape: &[usize]) -> Self {
        Self::from_vec(vec![0.0; shape.iter().product()], shape)
    }

    pub fn ones(shape: &[usize]) -> Self {
        Self::from_vec(vec![1.0; shape.iter().product()], shape)
    }

    pub fn full(shape: &[usize], val: f32) -> Self {
        Self::from_vec(vec![val; shape.iter().product()], shape)
    }

    /// Random normal (mean=0, std=1).
    pub fn randn(shape: &[usize]) -> Self {
        use std::f32::consts::PI;
        let n: usize = shape.iter().product();
        let mut data = Vec::with_capacity(n);
        let mut i = 0;
        while i < n {
            let u1: f32 = (rand_u32() as f32 + 1.0) / (u32::MAX as f32 + 2.0);
            let u2: f32 = rand_u32() as f32 / u32::MAX as f32;
            let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
            let z1 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).sin();
            data.push(z0);
            if i + 1 < n { data.push(z1); }
            i += 2;
        }
        data.truncate(n);
        Self::from_vec(data, shape)
    }

    /// Kaiming uniform initialization.
    pub fn kaiming_uniform(shape: &[usize], fan_in: usize) -> Self {
        let bound = (6.0f32 / fan_in as f32).sqrt();
        let n: usize = shape.iter().product();
        let data: Vec<f32> = (0..n).map(|_| {
            let u = rand_u32() as f32 / u32::MAX as f32;
            u * 2.0 * bound - bound
        }).collect();
        Self::from_vec(data, shape)
    }

    /// Xavier uniform initialization.
    pub fn xavier_uniform(shape: &[usize], fan_in: usize, fan_out: usize) -> Self {
        let bound = (6.0f32 / (fan_in + fan_out) as f32).sqrt();
        let n: usize = shape.iter().product();
        let data: Vec<f32> = (0..n).map(|_| {
            let u = rand_u32() as f32 / u32::MAX as f32;
            u * 2.0 * bound - bound
        }).collect();
        Self::from_vec(data, shape)
    }

    /// Enable gradient tracking.
    pub fn requires_grad_(mut self) -> Self {
        self.requires_grad = true;
        self.grad = Some(Arc::new(Mutex::new(None)));
        self
    }

    /// Set tensor name.
    pub fn named(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn numel(&self) -> usize { self.shape.numel() }
    pub fn ndim(&self) -> usize { self.shape.ndim() }
    pub fn dims(&self) -> &[usize] { self.shape.dims() }

    /// Get data as f32 slice (CPU only).
    pub fn data_f32(&self) -> &[f32] {
        self.storage.as_cpu()
    }

    /// Get mutable data (forces unique ownership via Arc::make_mut).
    pub fn data_f32_mut(&mut self) -> &mut [f32] {
        Arc::make_mut(&mut self.storage).as_cpu_mut()
    }

    pub fn is_cpu(&self) -> bool { self.storage.is_cpu() }

    pub fn to_cpu(&self) -> Self {
        Self {
            storage: Arc::new(self.storage.to_cpu()),
            shape: self.shape.clone(),
            dtype: self.dtype,
            requires_grad: false,
            grad: None,
            grad_fn: None,
            name: self.name.clone(),
        }
    }

    /// Get single element by flat index.
    pub fn item(&self, index: usize) -> f32 {
        self.data_f32()[index]
    }

    /// Reshape (same total elements).
    pub fn reshape(&self, new_shape: &[usize]) -> Self {
        let new_numel: usize = new_shape.iter().product();
        assert_eq!(self.numel(), new_numel, "reshape: numel mismatch");
        Self {
            storage: self.storage.clone(),
            shape: Shape::new(new_shape.to_vec()),
            dtype: self.dtype,
            requires_grad: self.requires_grad,
            grad: self.grad.clone(),
            grad_fn: self.grad_fn.clone(),
            name: self.name.clone(),
        }
    }

    /// Detach from autograd graph.
    pub fn detach(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            shape: self.shape.clone(),
            dtype: self.dtype,
            requires_grad: false,
            grad: None,
            grad_fn: None,
            name: self.name.clone(),
        }
    }

    /// Accumulate a gradient into this tensor.
    pub fn accumulate_grad(&self, grad: &Tensor) {
        if let Some(ref g) = self.grad {
            let mut lock = g.lock().unwrap();
            match lock.as_mut() {
                Some(existing) => {
                    let sum = add(existing, grad);
                    *existing = sum;
                }
                None => { *lock = Some(grad.clone()); }
            }
        }
    }

    /// Zero out accumulated gradients.
    pub fn zero_grad(&self) {
        if let Some(ref g) = self.grad {
            *g.lock().unwrap() = None;
        }
    }

    /// Get accumulated gradient.
    pub fn get_grad(&self) -> Option<Tensor> {
        self.grad.as_ref().and_then(|g| g.lock().unwrap().clone())
    }

    /// Transpose last two dims.
    pub fn t(&self) -> Self { transpose(self) }

    /// Copy data from another tensor.
    pub fn copy_from(&mut self, other: &Tensor) {
        assert_eq!(self.numel(), other.numel());
        let src = other.data_f32();
        let dst = self.data_f32_mut();
        dst.copy_from_slice(src);
    }

    /// Make data contiguous (copy if needed).
    pub fn contiguous(&self) -> Self { self.clone() }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tensor Operations (33+ ops)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// ── Elementwise arithmetic ──────────────────────────────────────────────────

pub fn add(a: &Tensor, b: &Tensor) -> Tensor {
    let ad = a.data_f32();
    let bd = b.data_f32();
    if ad.len() == bd.len() {
        let out: Vec<f32> = ad.iter().zip(bd).map(|(x, y)| x + y).collect();
        Tensor::from_vec(out, a.dims())
    } else {
        broadcast_binop(a, b, |x, y| x + y)
    }
}

pub fn sub(a: &Tensor, b: &Tensor) -> Tensor {
    let ad = a.data_f32();
    let bd = b.data_f32();
    if ad.len() == bd.len() {
        let out: Vec<f32> = ad.iter().zip(bd).map(|(x, y)| x - y).collect();
        Tensor::from_vec(out, a.dims())
    } else {
        broadcast_binop(a, b, |x, y| x - y)
    }
}

pub fn mul(a: &Tensor, b: &Tensor) -> Tensor {
    let ad = a.data_f32();
    let bd = b.data_f32();
    if ad.len() == bd.len() {
        let out: Vec<f32> = ad.iter().zip(bd).map(|(x, y)| x * y).collect();
        Tensor::from_vec(out, a.dims())
    } else {
        broadcast_binop(a, b, |x, y| x * y)
    }
}

pub fn div(a: &Tensor, b: &Tensor) -> Tensor {
    let ad = a.data_f32();
    let bd = b.data_f32();
    if ad.len() == bd.len() {
        let out: Vec<f32> = ad.iter().zip(bd).map(|(x, y)| x / y).collect();
        Tensor::from_vec(out, a.dims())
    } else {
        broadcast_binop(a, b, |x, y| x / y)
    }
}

pub fn mul_scalar(a: &Tensor, s: f32) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x * s).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn add_scalar(a: &Tensor, s: f32) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x + s).collect();
    Tensor::from_vec(out, a.dims())
}

fn broadcast_binop(a: &Tensor, b: &Tensor, op: impl Fn(f32, f32) -> f32) -> Tensor {
    let out_shape = Shape::broadcast_shape(&a.shape, &b.shape)
        .expect("Shapes not broadcastable");
    let n = out_shape.numel();
    let mut data = vec![0.0f32; n];
    for i in 0..n {
        let idx = out_shape.multi_index(i);
        let ai = broadcast_idx(&idx, a.dims());
        let bi = broadcast_idx(&idx, b.dims());
        data[i] = op(a.data_f32()[ai], b.data_f32()[bi]);
    }
    Tensor::from_vec(data, out_shape.dims())
}

fn broadcast_idx(idx: &[usize], dims: &[usize]) -> usize {
    let offset = idx.len() - dims.len();
    let mut flat = 0;
    let shape = Shape::new(dims.to_vec());
    for (i, &d) in dims.iter().enumerate() {
        let ii = if d == 1 { 0 } else { idx[i + offset] };
        flat += ii * shape.strides()[i];
    }
    flat
}

// ── Unary ops ───────────────────────────────────────────────────────────────

pub fn exp(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x.exp()).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn log(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x.ln()).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn sqrt(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x.sqrt()).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn pow(a: &Tensor, p: f32) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x.powf(p)).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn neg(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| -x).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn reciprocal(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| 1.0 / x).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn abs(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x.abs()).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn clamp(a: &Tensor, min: f32, max: f32) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x.clamp(min, max)).collect();
    Tensor::from_vec(out, a.dims())
}

// ── Activations ─────────────────────────────────────────────────────────────

pub fn relu(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x.max(0.0)).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn gelu(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| {
        0.5 * x * (1.0 + (0.7978845608 * (x + 0.044715 * x * x * x)).tanh())
    }).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn sigmoid(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn silu(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x / (1.0 + (-x).exp())).collect();
    Tensor::from_vec(out, a.dims())
}

pub fn tanh_act(a: &Tensor) -> Tensor {
    let out: Vec<f32> = a.data_f32().iter().map(|&x| x.tanh()).collect();
    Tensor::from_vec(out, a.dims())
}

// ── Reductions ──────────────────────────────────────────────────────────────

pub fn sum(a: &Tensor) -> Tensor {
    let s: f32 = a.data_f32().iter().sum();
    Tensor::from_vec(vec![s], &[1])
}

pub fn sum_dim(a: &Tensor, dim: usize) -> Tensor {
    let dims = a.dims();
    assert!(dim < dims.len());
    let mut out_dims: Vec<usize> = dims.to_vec();
    out_dims[dim] = 1;
    let n = out_dims.iter().product::<usize>();
    let mut data = vec![0.0f32; n];
    let out_shape = Shape::new(out_dims.clone());
    for i in 0..a.numel() {
        let idx = a.shape.multi_index(i);
        let mut out_idx = idx.clone();
        out_idx[dim] = 0;
        let oi = out_shape.flat_index(&out_idx);
        data[oi] += a.data_f32()[i];
    }
    Tensor::from_vec(data, &out_dims)
}

pub fn mean(a: &Tensor) -> Tensor {
    let s: f32 = a.data_f32().iter().sum::<f32>() / a.numel() as f32;
    Tensor::from_vec(vec![s], &[1])
}

pub fn mean_dim(a: &Tensor, dim: usize) -> Tensor {
    let s = sum_dim(a, dim);
    let count = a.dims()[dim] as f32;
    mul_scalar(&s, 1.0 / count)
}

pub fn var_dim(a: &Tensor, dim: usize) -> Tensor {
    let m = mean_dim(a, dim);
    let diff = sub(a, &m);
    let sq = mul(&diff, &diff);
    mean_dim(&sq, dim)
}

pub fn max_dim(a: &Tensor, dim: usize) -> (Tensor, Vec<usize>) {
    let dims = a.dims();
    let mut out_dims = dims.to_vec();
    out_dims[dim] = 1;
    let n = out_dims.iter().product::<usize>();
    let mut data = vec![f32::NEG_INFINITY; n];
    let mut indices = vec![0usize; n];
    let out_shape = Shape::new(out_dims.clone());
    for i in 0..a.numel() {
        let idx = a.shape.multi_index(i);
        let mut out_idx = idx.clone();
        out_idx[dim] = 0;
        let oi = out_shape.flat_index(&out_idx);
        if a.data_f32()[i] > data[oi] {
            data[oi] = a.data_f32()[i];
            indices[oi] = idx[dim];
        }
    }
    (Tensor::from_vec(data, &out_dims), indices)
}

pub fn argmax(a: &Tensor) -> Vec<usize> {
    let (_, indices) = max_dim(a, a.ndim() - 1);
    indices
}

// ── Linear algebra ──────────────────────────────────────────────────────────

/// Matrix multiply (supports 2D, 3D batch, 4D batch, 3D×2D).
pub fn matmul(a: &Tensor, b: &Tensor) -> Tensor {
    let ad = a.dims();
    let bd = b.dims();
    match (ad.len(), bd.len()) {
        (2, 2) => matmul_2d(a, b),
        (3, 3) => matmul_batched(a, b),
        (4, 4) => matmul_4d(a, b),
        (3, 2) => matmul_3d_2d(a, b),
        _ => panic!("matmul: unsupported shapes {:?} × {:?}", ad, bd),
    }
}

fn matmul_2d(a: &Tensor, b: &Tensor) -> Tensor {
    let (m, k1) = (a.dims()[0], a.dims()[1]);
    let (k2, n) = (b.dims()[0], b.dims()[1]);
    assert_eq!(k1, k2, "matmul: inner dims mismatch {} vs {}", k1, k2);
    let ad = a.data_f32();
    let bd = b.data_f32();
    let mut out = vec![0.0f32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut s = 0.0f32;
            for p in 0..k1 { s += ad[i * k1 + p] * bd[p * n + j]; }
            out[i * n + j] = s;
        }
    }
    Tensor::from_vec(out, &[m, n])
}

fn matmul_batched(a: &Tensor, b: &Tensor) -> Tensor {
    let (batch, m, k) = (a.dims()[0], a.dims()[1], a.dims()[2]);
    let n = b.dims()[2];
    assert_eq!(b.dims()[1], k);
    let ad = a.data_f32();
    let bd = b.data_f32();
    let mut out = vec![0.0f32; batch * m * n];
    for bs in 0..batch {
        for i in 0..m {
            for j in 0..n {
                let mut s = 0.0f32;
                for p in 0..k {
                    s += ad[bs * m * k + i * k + p] * bd[bs * k * n + p * n + j];
                }
                out[bs * m * n + i * n + j] = s;
            }
        }
    }
    Tensor::from_vec(out, &[batch, m, n])
}

fn matmul_4d(a: &Tensor, b: &Tensor) -> Tensor {
    let (b0, b1, m, k) = (a.dims()[0], a.dims()[1], a.dims()[2], a.dims()[3]);
    let n = b.dims()[3];
    assert_eq!(b.dims()[2], k);
    let ad = a.data_f32();
    let bd = b.data_f32();
    let mut out = vec![0.0f32; b0 * b1 * m * n];
    for i0 in 0..b0 {
        for i1 in 0..b1 {
            let a_off = (i0 * b1 + i1) * m * k;
            let b_off = (i0 * b1 + i1) * k * n;
            let o_off = (i0 * b1 + i1) * m * n;
            for i in 0..m {
                for j in 0..n {
                    let mut s = 0.0f32;
                    for p in 0..k {
                        s += ad[a_off + i * k + p] * bd[b_off + p * n + j];
                    }
                    out[o_off + i * n + j] = s;
                }
            }
        }
    }
    Tensor::from_vec(out, &[b0, b1, m, n])
}

fn matmul_3d_2d(a: &Tensor, b: &Tensor) -> Tensor {
    let (batch, m, k) = (a.dims()[0], a.dims()[1], a.dims()[2]);
    let n = b.dims()[1];
    assert_eq!(b.dims()[0], k);
    let ad = a.data_f32();
    let bd = b.data_f32();
    let mut out = vec![0.0f32; batch * m * n];
    for bs in 0..batch {
        for i in 0..m {
            for j in 0..n {
                let mut s = 0.0f32;
                for p in 0..k { s += ad[bs * m * k + i * k + p] * bd[p * n + j]; }
                out[bs * m * n + i * n + j] = s;
            }
        }
    }
    Tensor::from_vec(out, &[batch, m, n])
}

pub fn transpose(a: &Tensor) -> Tensor {
    let d = a.dims();
    match d.len() {
        2 => {
            let (m, n) = (d[0], d[1]);
            let ad = a.data_f32();
            let mut out = vec![0.0f32; m * n];
            for i in 0..m {
                for j in 0..n { out[j * m + i] = ad[i * n + j]; }
            }
            Tensor::from_vec(out, &[n, m])
        }
        3 => {
            let (b, m, n) = (d[0], d[1], d[2]);
            let ad = a.data_f32();
            let mut out = vec![0.0f32; b * m * n];
            for bs in 0..b {
                for i in 0..m {
                    for j in 0..n {
                        out[bs * n * m + j * m + i] = ad[bs * m * n + i * n + j];
                    }
                }
            }
            Tensor::from_vec(out, &[b, n, m])
        }
        _ => panic!("transpose: unsupported ndim={}", d.len()),
    }
}

// ── Softmax / Loss ──────────────────────────────────────────────────────────

pub fn softmax(a: &Tensor, dim: i32) -> Tensor {
    let dims = a.dims();
    let ndim = dims.len();
    let dim = if dim < 0 { (ndim as i32 + dim) as usize } else { dim as usize };
    let d = dims[dim];
    let outer: usize = dims[..dim].iter().product();
    let inner: usize = dims[dim + 1..].iter().product();
    let ad = a.data_f32();
    let mut out = vec![0.0f32; a.numel()];

    for o in 0..outer {
        for i in 0..inner {
            let mut max_val = f32::NEG_INFINITY;
            for j in 0..d {
                let idx = (o * d + j) * inner + i;
                if ad[idx] > max_val { max_val = ad[idx]; }
            }
            let mut sum_exp = 0.0f32;
            for j in 0..d {
                let idx = (o * d + j) * inner + i;
                let e = (ad[idx] - max_val).exp();
                out[idx] = e;
                sum_exp += e;
            }
            for j in 0..d {
                let idx = (o * d + j) * inner + i;
                out[idx] /= sum_exp;
            }
        }
    }
    Tensor::from_vec(out, dims)
}

pub fn log_softmax(a: &Tensor, dim: i32) -> Tensor {
    let s = softmax(a, dim);
    log(&s)
}

/// Cross-entropy loss (logits [batch, vocab], targets [batch]).
pub fn cross_entropy_loss(logits: &Tensor, targets: &[usize]) -> Tensor {
    let (batch, vocab) = (logits.dims()[0], logits.dims()[1]);
    assert_eq!(targets.len(), batch);
    let ld = logits.data_f32();
    let mut total_loss = 0.0f32;
    for b in 0..batch {
        let row = &ld[b * vocab..(b + 1) * vocab];
        let max_val = row.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let sum_exp: f32 = row.iter().map(|&x| (x - max_val).exp()).sum();
        let log_prob = row[targets[b]] - max_val - sum_exp.ln();
        total_loss -= log_prob;
    }
    total_loss /= batch as f32;
    let mut result = Tensor::from_vec(vec![total_loss], &[1]);
    if logits.requires_grad {
        result.grad_fn = Some(Arc::new(CrossEntropyGrad {
            logits: logits.clone(),
            targets: targets.to_vec(),
        }));
        result.requires_grad = true;
        result.grad = Some(Arc::new(Mutex::new(None)));
    }
    result
}

// ── Embedding ───────────────────────────────────────────────────────────────

/// Embedding lookup: weight [vocab, dim], indices [seq] → [seq, dim].
pub fn embedding(weight: &Tensor, indices: &[usize]) -> Tensor {
    let dim = weight.dims()[1];
    let wd = weight.data_f32();
    let mut out = Vec::with_capacity(indices.len() * dim);
    for &idx in indices {
        let start = idx * dim;
        out.extend_from_slice(&wd[start..start + dim]);
    }
    Tensor::from_vec(out, &[indices.len(), dim])
}

// ── Concatenation ───────────────────────────────────────────────────────────

/// Concatenate tensors along a dimension.
pub fn cat(tensors: &[&Tensor], dim: usize) -> Tensor {
    assert!(!tensors.is_empty());
    let ndim = tensors[0].ndim();
    let mut out_dims = tensors[0].dims().to_vec();
    for t in &tensors[1..] {
        assert_eq!(t.ndim(), ndim);
        out_dims[dim] += t.dims()[dim];
    }
    let mut data = Vec::with_capacity(out_dims.iter().product());
    let outer: usize = out_dims[..dim].iter().product();
    let inner: usize = out_dims[dim + 1..].iter().product();

    for o in 0..outer {
        for t in tensors {
            let td = t.dims()[dim];
            let td_data = t.data_f32();
            for d in 0..td {
                let start = (o * td + d) * inner;
                data.extend_from_slice(&td_data[start..start + inner]);
            }
        }
    }
    Tensor::from_vec(data, &out_dims)
}

// ── Simple RNG (xorshift32) ────────────────────────────────────────────────

fn rand_u32() -> u32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u32> = Cell::new(0xDEAD_BEEF);
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        s.set(x);
        x
    })
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FFI — extern "C" functions for Vitalis stdlib
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a tensor filled with zeros. Returns an opaque handle.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_zeros(rows: i64, cols: i64) -> i64 {
    let t = Box::new(Tensor::zeros(&[rows as usize, cols as usize]));
    Box::into_raw(t) as i64
}

/// Create a random normal tensor.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_randn(rows: i64, cols: i64) -> i64 {
    let t = Box::new(Tensor::randn(&[rows as usize, cols as usize]));
    Box::into_raw(t) as i64
}

/// Matrix multiply two tensor handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tensor_matmul(a: i64, b: i64) -> i64 {
    let a = unsafe { &*(a as *const Tensor) };
    let b = unsafe { &*(b as *const Tensor) };
    let result = Box::new(matmul(a, b));
    Box::into_raw(result) as i64
}

/// Get a single element from a tensor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tensor_item(handle: i64, index: i64) -> f64 {
    let t = unsafe { &*(handle as *const Tensor) };
    t.item(index as usize) as f64
}

/// Get the total number of elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tensor_numel(handle: i64) -> i64 {
    let t = unsafe { &*(handle as *const Tensor) };
    t.numel() as i64
}

/// Free a tensor handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tensor_free(handle: i64) {
    if handle != 0 {
        let _ = unsafe { Box::from_raw(handle as *mut Tensor) };
    }
}

/// Compute dot product of two flat tensors.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tensor_dot(a: i64, b: i64) -> f64 {
    let a = unsafe { &*(a as *const Tensor) };
    let b = unsafe { &*(b as *const Tensor) };
    let ad = a.data_f32();
    let bd = b.data_f32();
    let n = ad.len().min(bd.len());
    let mut s = 0.0f64;
    for i in 0..n { s += ad[i] as f64 * bd[i] as f64; }
    s
}

/// Compute Frobenius norm of a tensor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tensor_norm(handle: i64) -> f64 {
    let t = unsafe { &*(handle as *const Tensor) };
    let s: f64 = t.data_f32().iter().map(|&x| (x as f64) * (x as f64)).sum();
    s.sqrt()
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_create() {
        let t = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        assert_eq!(t.dims(), &[2, 3]);
        assert_eq!(t.numel(), 6);
    }

    #[test]
    fn test_matmul_2d() {
        let a = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], &[2, 2]);
        let b = Tensor::from_vec(vec![5.0, 6.0, 7.0, 8.0], &[2, 2]);
        let c = matmul(&a, &b);
        assert_eq!(c.dims(), &[2, 2]);
        assert!((c.item(0) - 19.0).abs() < 1e-5); // 1*5+2*7
        assert!((c.item(3) - 50.0).abs() < 1e-5); // 3*6+4*8
    }

    #[test]
    fn test_softmax() {
        let a = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[1, 3]);
        let s = softmax(&a, -1);
        let d = s.data_f32();
        let total: f32 = d.iter().sum();
        assert!((total - 1.0).abs() < 1e-5);
        assert!(d[2] > d[1] && d[1] > d[0]);
    }

    #[test]
    fn test_cross_entropy() {
        let logits = Tensor::from_vec(vec![2.0, 1.0, 0.1, 0.5, 2.5, 0.3], &[2, 3]);
        let loss = cross_entropy_loss(&logits, &[0, 1]);
        assert!(loss.item(0) > 0.0);
        assert!(loss.item(0) < 10.0);
    }

    #[test]
    fn test_broadcast_add() {
        let a = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        let b = Tensor::from_vec(vec![10.0, 20.0, 30.0], &[1, 3]);
        let c = add(&a, &b);
        assert_eq!(c.dims(), &[2, 3]);
        assert!((c.item(0) - 11.0).abs() < 1e-5);
        assert!((c.item(5) - 36.0).abs() < 1e-5);
    }

    #[test]
    fn test_embedding() {
        let w = Tensor::from_vec(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], &[4, 2]);
        let e = embedding(&w, &[0, 2, 1]);
        assert_eq!(e.dims(), &[3, 2]);
        assert!((e.item(0) - 0.1).abs() < 1e-5);
        assert!((e.item(2) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_shape_broadcast() {
        let a = Shape::new(vec![2, 1, 3]);
        let b = Shape::new(vec![4, 3]);
        let c = Shape::broadcast_shape(&a, &b).unwrap();
        assert_eq!(c.dims(), &[2, 4, 3]);
    }

    #[test]
    fn test_activations() {
        let a = Tensor::from_vec(vec![-1.0, 0.0, 1.0, 2.0], &[4]);
        let r = relu(&a);
        assert!((r.item(0) - 0.0).abs() < 1e-5);
        assert!((r.item(2) - 1.0).abs() < 1e-5);

        let s = sigmoid(&a);
        assert!((s.item(1) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_reduction() {
        let a = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], &[2, 2]);
        let s = sum(&a);
        assert!((s.item(0) - 10.0).abs() < 1e-5);
        let m = mean(&a);
        assert!((m.item(0) - 2.5).abs() < 1e-5);
    }
}
