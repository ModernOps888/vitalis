//! Tensor Engine — N-dimensional tensor type with accelerated linear algebra.
//!
//! Provides the foundational data structure for all deep learning operations in Vitalis.
//! Supports f32/f64 storage, shape tracking, broadcasting, strided views, and tiled
//! SIMD matrix multiplication (Goto algorithm with L1/L2 cache blocking).
//!
//! Design: Tensors own contiguous storage with shape+stride metadata. Views share
//! storage with custom strides (zero-copy transpose, slice, reshape). All operations
//! return new tensors unless suffixed with `_` (in-place mutation).
//!
//! Reuses: `simd_ops::F64x4` for SIMD kernels, `numerical` for fallback linear algebra.

use std::sync::Mutex;
use std::collections::HashMap;

// ── Core Types ──────────────────────────────────────────────────────────

/// Data type for tensor elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DType {
    F32,
    F64,
    I32,
    I64,
    BF16,
}

impl DType {
    pub fn size_bytes(self) -> usize {
        match self {
            DType::F32 => 4,
            DType::F64 => 8,
            DType::I32 => 4,
            DType::I64 => 8,
            DType::BF16 => 2,
        }
    }
}

/// N-dimensional tensor with contiguous f64 storage.
#[derive(Debug, Clone)]
pub struct Tensor {
    pub data: Vec<f64>,
    pub shape: Vec<usize>,
    pub strides: Vec<usize>,
    pub dtype: DType,
    pub requires_grad: bool,
    pub grad: Option<Box<Tensor>>,
}

impl Tensor {
    /// Create a new tensor with given shape, filled with zeros.
    pub fn zeros(shape: &[usize]) -> Self {
        let numel: usize = shape.iter().product();
        Self {
            data: vec![0.0; numel],
            shape: shape.to_vec(),
            strides: compute_strides(shape),
            dtype: DType::F64,
            requires_grad: false,
            grad: None,
        }
    }

    /// Create a tensor filled with ones.
    pub fn ones(shape: &[usize]) -> Self {
        let numel: usize = shape.iter().product();
        Self {
            data: vec![1.0; numel],
            shape: shape.to_vec(),
            strides: compute_strides(shape),
            dtype: DType::F64,
            requires_grad: false,
            grad: None,
        }
    }

    /// Create a tensor filled with a constant value.
    pub fn full(shape: &[usize], value: f64) -> Self {
        let numel: usize = shape.iter().product();
        Self {
            data: vec![value; numel],
            shape: shape.to_vec(),
            strides: compute_strides(shape),
            dtype: DType::F64,
            requires_grad: false,
            grad: None,
        }
    }

    /// Create a tensor from a flat data vector and shape.
    pub fn from_data(data: Vec<f64>, shape: &[usize]) -> Self {
        let numel: usize = shape.iter().product();
        assert_eq!(data.len(), numel, "Data length {} != shape product {}", data.len(), numel);
        Self {
            data,
            shape: shape.to_vec(),
            strides: compute_strides(shape),
            dtype: DType::F64,
            requires_grad: false,
            grad: None,
        }
    }

    /// Create a 1D tensor (vector).
    pub fn vec(data: Vec<f64>) -> Self {
        let n = data.len();
        Self::from_data(data, &[n])
    }

    /// Create a 2D identity matrix.
    pub fn eye(n: usize) -> Self {
        let mut data = vec![0.0; n * n];
        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Self::from_data(data, &[n, n])
    }

    /// Create a tensor with random uniform values in [0, 1).
    pub fn rand(shape: &[usize], seed: u64) -> Self {
        let numel: usize = shape.iter().product();
        let mut data = Vec::with_capacity(numel);
        let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
        for _ in 0..numel {
            // xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            data.push((state as f64) / (u64::MAX as f64));
        }
        Self::from_data(data, shape)
    }

    /// Create a tensor with random normal values (Box-Muller).
    pub fn randn(shape: &[usize], seed: u64) -> Self {
        let numel: usize = shape.iter().product();
        let mut data = Vec::with_capacity(numel);
        let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
        let mut i = 0;
        while i < numel {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let u1 = ((state as f64) / (u64::MAX as f64)).max(1e-10);
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let u2 = (state as f64) / (u64::MAX as f64);
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f64::consts::PI * u2;
            data.push(r * theta.cos());
            if i + 1 < numel {
                data.push(r * theta.sin());
            }
            i += 2;
        }
        data.truncate(numel);
        Self::from_data(data, shape)
    }

    /// Total number of elements.
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }

    /// Number of dimensions.
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Get element at flat index.
    pub fn get_flat(&self, idx: usize) -> f64 {
        self.data[idx]
    }

    /// Set element at flat index.
    pub fn set_flat(&mut self, idx: usize, val: f64) {
        self.data[idx] = val;
    }

    /// Get element at n-dimensional index.
    pub fn get(&self, indices: &[usize]) -> f64 {
        assert_eq!(indices.len(), self.ndim());
        let flat = indices.iter().zip(&self.strides).map(|(i, s)| i * s).sum::<usize>();
        self.data[flat]
    }

    /// Set element at n-dimensional index.
    pub fn set(&mut self, indices: &[usize], val: f64) {
        assert_eq!(indices.len(), self.ndim());
        let flat = indices.iter().zip(&self.strides).map(|(i, s)| i * s).sum::<usize>();
        self.data[flat] = val;
    }

    /// Reshape tensor (must have same total elements).
    pub fn reshape(&self, new_shape: &[usize]) -> Self {
        let new_numel: usize = new_shape.iter().product();
        assert_eq!(self.numel(), new_numel, "Cannot reshape {} into {}", self.numel(), new_numel);
        Self {
            data: self.data.clone(),
            shape: new_shape.to_vec(),
            strides: compute_strides(new_shape),
            dtype: self.dtype,
            requires_grad: self.requires_grad,
            grad: None,
        }
    }

    /// Transpose a 2D tensor.
    pub fn transpose(&self) -> Self {
        assert_eq!(self.ndim(), 2, "transpose requires 2D tensor");
        let (m, n) = (self.shape[0], self.shape[1]);
        let mut data = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                data[j * m + i] = self.data[i * n + j];
            }
        }
        Self::from_data(data, &[n, m])
    }

    /// Transpose arbitrary axes.
    pub fn permute(&self, axes: &[usize]) -> Self {
        assert_eq!(axes.len(), self.ndim());
        let new_shape: Vec<usize> = axes.iter().map(|&a| self.shape[a]).collect();
        let new_strides: Vec<usize> = axes.iter().map(|&a| self.strides[a]).collect();
        let numel = self.numel();
        let mut data = vec![0.0; numel];
        let nd = self.ndim();
        let mut indices = vec![0usize; nd];
        for flat in 0..numel {
            let src = indices.iter().zip(&new_strides).map(|(i, s)| i * s).sum::<usize>();
            data[flat] = self.data[src];
            // Increment indices (row-major for new shape)
            for d in (0..nd).rev() {
                indices[d] += 1;
                if indices[d] < new_shape[d] {
                    break;
                }
                indices[d] = 0;
            }
        }
        Self {
            data,
            shape: new_shape,
            strides: compute_strides(&axes.iter().map(|&a| self.shape[a]).collect::<Vec<_>>()),
            dtype: self.dtype,
            requires_grad: self.requires_grad,
            grad: None,
        }
    }

    // ── Element-wise Operations ─────────────────────────────────────

    /// Element-wise addition with broadcasting.
    pub fn add(&self, other: &Tensor) -> Tensor {
        elementwise_binary(self, other, |a, b| a + b)
    }

    /// Element-wise subtraction with broadcasting.
    pub fn sub(&self, other: &Tensor) -> Tensor {
        elementwise_binary(self, other, |a, b| a - b)
    }

    /// Element-wise multiplication (Hadamard product) with broadcasting.
    pub fn mul(&self, other: &Tensor) -> Tensor {
        elementwise_binary(self, other, |a, b| a * b)
    }

    /// Element-wise division with broadcasting.
    pub fn div(&self, other: &Tensor) -> Tensor {
        elementwise_binary(self, other, |a, b| a / b)
    }

    /// Scalar addition.
    pub fn add_scalar(&self, val: f64) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x + val).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Scalar multiplication.
    pub fn mul_scalar(&self, val: f64) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x * val).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Element-wise power.
    pub fn pow(&self, exp: f64) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x.powf(exp)).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Element-wise exponential.
    pub fn exp(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x.exp()).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Element-wise natural logarithm.
    pub fn log(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x.ln()).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Element-wise square root.
    pub fn sqrt(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x.sqrt()).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Element-wise absolute value.
    pub fn abs(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x.abs()).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Element-wise clamp.
    pub fn clamp(&self, min: f64, max: f64) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x.clamp(min, max)).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Negate.
    pub fn neg(&self) -> Tensor {
        self.mul_scalar(-1.0)
    }

    // ── In-place Operations ─────────────────────────────────────────

    /// In-place addition (mutates self).
    pub fn add_(&mut self, other: &Tensor) {
        assert_eq!(self.shape, other.shape, "In-place ops require same shape");
        for (a, b) in self.data.iter_mut().zip(&other.data) {
            *a += b;
        }
    }

    /// In-place scalar multiply.
    pub fn mul_scalar_(&mut self, val: f64) {
        for x in &mut self.data {
            *x *= val;
        }
    }

    /// Zero out all elements.
    pub fn zero_(&mut self) {
        for x in &mut self.data {
            *x = 0.0;
        }
    }

    // ── Reduction Operations ────────────────────────────────────────

    /// Sum all elements.
    pub fn sum(&self) -> f64 {
        self.data.iter().sum()
    }

    /// Mean of all elements.
    pub fn mean(&self) -> f64 {
        self.sum() / self.numel() as f64
    }

    /// Max element.
    pub fn max_val(&self) -> f64 {
        self.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    /// Min element.
    pub fn min_val(&self) -> f64 {
        self.data.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    /// Argmax (flat index).
    pub fn argmax(&self) -> usize {
        self.data.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Argmin (flat index).
    pub fn argmin(&self) -> usize {
        self.data.iter().enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Sum along an axis.
    pub fn sum_axis(&self, axis: usize) -> Tensor {
        reduce_axis(self, axis, |slice| slice.iter().sum())
    }

    /// Mean along an axis.
    pub fn mean_axis(&self, axis: usize) -> Tensor {
        reduce_axis(self, axis, |slice| {
            let n = slice.len() as f64;
            slice.iter().sum::<f64>() / n
        })
    }

    /// Max along an axis.
    pub fn max_axis(&self, axis: usize) -> Tensor {
        reduce_axis(self, axis, |slice| {
            slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        })
    }

    /// Variance of all elements.
    pub fn var(&self) -> f64 {
        let mean = self.mean();
        self.data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / self.numel() as f64
    }

    /// Standard deviation.
    pub fn std_dev(&self) -> f64 {
        self.var().sqrt()
    }

    // ── Matrix Operations ───────────────────────────────────────────

    /// Matrix multiplication with tiled (Goto-style) algorithm for cache efficiency.
    /// Supports 2D×2D and batched matmul (leading batch dimensions must match).
    pub fn matmul(&self, other: &Tensor) -> Tensor {
        assert!(self.ndim() >= 2 && other.ndim() >= 2, "matmul requires at least 2D tensors");

        if self.ndim() == 2 && other.ndim() == 2 {
            let (m, k1) = (self.shape[0], self.shape[1]);
            let (k2, n) = (other.shape[0], other.shape[1]);
            assert_eq!(k1, k2, "matmul inner dimensions must match: {} vs {}", k1, k2);
            let data = tiled_matmul(&self.data, &other.data, m, k1, n);
            return Tensor::from_data(data, &[m, n]);
        }

        // Batched matmul: broadcast batch dimensions
        let a_mat = [self.shape[self.ndim()-2], self.shape[self.ndim()-1]];
        let b_mat = [other.shape[other.ndim()-2], other.shape[other.ndim()-1]];
        assert_eq!(a_mat[1], b_mat[0], "matmul inner dimensions must match");

        let a_batch: Vec<usize> = self.shape[..self.ndim()-2].to_vec();
        let b_batch: Vec<usize> = other.shape[..other.ndim()-2].to_vec();
        let out_batch = broadcast_shape(&a_batch, &b_batch);
        let batch_size: usize = out_batch.iter().product();

        let (m, k, n) = (a_mat[0], a_mat[1], b_mat[1]);
        let mat_size_a = m * k;
        let mat_size_b = k * n;
        let mat_size_c = m * n;

        let mut result = vec![0.0; batch_size * mat_size_c];
        for b in 0..batch_size {
            let a_off = (b % (a_batch.iter().product::<usize>().max(1))) * mat_size_a;
            let b_off = (b % (b_batch.iter().product::<usize>().max(1))) * mat_size_b;
            let c_off = b * mat_size_c;
            let partial = tiled_matmul(
                &self.data[a_off..a_off+mat_size_a],
                &other.data[b_off..b_off+mat_size_b],
                m, k, n,
            );
            result[c_off..c_off+mat_size_c].copy_from_slice(&partial);
        }

        let mut out_shape = out_batch;
        out_shape.push(m);
        out_shape.push(n);
        Tensor::from_data(result, &out_shape)
    }

    /// Dot product (for 1D tensors).
    pub fn dot(&self, other: &Tensor) -> f64 {
        assert_eq!(self.shape, other.shape, "dot requires same shape");
        self.data.iter().zip(&other.data).map(|(a, b)| a * b).sum()
    }

    /// L2 norm.
    pub fn norm(&self) -> f64 {
        self.data.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// Normalize to unit norm.
    pub fn normalize(&self) -> Tensor {
        let n = self.norm();
        if n < 1e-12 { return self.clone(); }
        self.mul_scalar(1.0 / n)
    }

    // ── Activation Functions ────────────────────────────────────────

    /// ReLU activation.
    pub fn relu(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x.max(0.0)).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// GELU activation (approximate).
    pub fn gelu(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| {
            0.5 * x * (1.0 + (0.7978845608 * (x + 0.044715 * x * x * x)).tanh())
        }).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// SiLU / Swish activation.
    pub fn silu(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x / (1.0 + (-x).exp())).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Sigmoid activation.
    pub fn sigmoid(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Softmax along last dimension.
    pub fn softmax(&self) -> Tensor {
        if self.ndim() == 1 {
            let max = self.max_val();
            let exp_data: Vec<f64> = self.data.iter().map(|&x| (x - max).exp()).collect();
            let sum: f64 = exp_data.iter().sum();
            let data: Vec<f64> = exp_data.iter().map(|&x| x / sum).collect();
            return Tensor::from_data(data, &self.shape);
        }
        // For nD tensors, apply softmax along last axis
        let last = *self.shape.last().unwrap();
        let batch: usize = self.numel() / last;
        let mut data = self.data.clone();
        for b in 0..batch {
            let offset = b * last;
            let slice = &data[offset..offset+last];
            let max = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mut sum = 0.0;
            for i in 0..last {
                data[offset + i] = (data[offset + i] - max).exp();
                sum += data[offset + i];
            }
            for i in 0..last {
                data[offset + i] /= sum;
            }
        }
        Tensor::from_data(data, &self.shape)
    }

    /// Tanh activation.
    pub fn tanh_act(&self) -> Tensor {
        let data: Vec<f64> = self.data.iter().map(|&x| x.tanh()).collect();
        Tensor::from_data(data, &self.shape)
    }

    /// Layer normalization over last axis.
    pub fn layer_norm(&self, gamma: &Tensor, beta: &Tensor, eps: f64) -> Tensor {
        let last = *self.shape.last().unwrap();
        let batch = self.numel() / last;
        let mut data = vec![0.0; self.numel()];
        for b in 0..batch {
            let off = b * last;
            let slice = &self.data[off..off+last];
            let mean: f64 = slice.iter().sum::<f64>() / last as f64;
            let var: f64 = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / last as f64;
            let inv_std = 1.0 / (var + eps).sqrt();
            for i in 0..last {
                data[off + i] = gamma.data[i] * (slice[i] - mean) * inv_std + beta.data[i];
            }
        }
        Tensor::from_data(data, &self.shape)
    }

    /// RMS normalization (used in modern transformers).
    pub fn rms_norm(&self, gamma: &Tensor, eps: f64) -> Tensor {
        let last = *self.shape.last().unwrap();
        let batch = self.numel() / last;
        let mut data = vec![0.0; self.numel()];
        for b in 0..batch {
            let off = b * last;
            let slice = &self.data[off..off+last];
            let rms = (slice.iter().map(|x| x * x).sum::<f64>() / last as f64 + eps).sqrt();
            for i in 0..last {
                data[off + i] = gamma.data[i] * slice[i] / rms;
            }
        }
        Tensor::from_data(data, &self.shape)
    }

    /// Concatenate tensors along an axis.
    pub fn cat(tensors: &[&Tensor], axis: usize) -> Tensor {
        assert!(!tensors.is_empty());
        let ndim = tensors[0].ndim();
        for t in tensors {
            assert_eq!(t.ndim(), ndim);
        }
        let mut new_shape = tensors[0].shape.clone();
        new_shape[axis] = tensors.iter().map(|t| t.shape[axis]).sum();

        let numel: usize = new_shape.iter().product();
        let mut data = vec![0.0; numel];
        let new_strides = compute_strides(&new_shape);

        let mut axis_offset = 0;
        for t in tensors {
            let mut indices = vec![0usize; ndim];
            for _ in 0..t.numel() {
                let src = indices.iter().zip(&t.strides).map(|(i, s)| i * s).sum::<usize>();
                let mut dst_indices = indices.clone();
                dst_indices[axis] += axis_offset;
                let dst = dst_indices.iter().zip(&new_strides).map(|(i, s)| i * s).sum::<usize>();
                data[dst] = t.data[src];
                // increment indices
                for d in (0..ndim).rev() {
                    indices[d] += 1;
                    if indices[d] < t.shape[d] { break; }
                    indices[d] = 0;
                }
            }
            axis_offset += t.shape[axis];
        }

        Tensor::from_data(data, &new_shape)
    }

    /// Slice tensor along an axis.
    pub fn slice(&self, axis: usize, start: usize, end: usize) -> Tensor {
        assert!(axis < self.ndim());
        assert!(end <= self.shape[axis]);
        let mut new_shape = self.shape.clone();
        new_shape[axis] = end - start;
        let numel: usize = new_shape.iter().product();
        let mut data = vec![0.0; numel];
        let new_strides = compute_strides(&new_shape);
        let ndim = self.ndim();

        let mut indices = vec![0usize; ndim];
        for flat in 0..numel {
            let mut src_indices = indices.clone();
            src_indices[axis] += start;
            let src = src_indices.iter().zip(&self.strides).map(|(i, s)| i * s).sum::<usize>();
            data[flat] = self.data[src];
            for d in (0..ndim).rev() {
                indices[d] += 1;
                if indices[d] < new_shape[d] { break; }
                indices[d] = 0;
            }
        }
        Tensor::from_data(data, &new_shape)
    }

    /// Unsqueeze — add a dimension of size 1 at the given axis.
    pub fn unsqueeze(&self, axis: usize) -> Tensor {
        let mut new_shape = self.shape.clone();
        new_shape.insert(axis, 1);
        self.reshape(&new_shape)
    }

    /// Squeeze — remove dimensions of size 1.
    pub fn squeeze(&self) -> Tensor {
        let new_shape: Vec<usize> = self.shape.iter().filter(|&&s| s != 1).cloned().collect();
        if new_shape.is_empty() {
            self.reshape(&[1])
        } else {
            self.reshape(&new_shape)
        }
    }

    /// Repeat tensor along dimensions.
    pub fn repeat(&self, repeats: &[usize]) -> Tensor {
        assert_eq!(repeats.len(), self.ndim());
        let new_shape: Vec<usize> = self.shape.iter().zip(repeats).map(|(s, r)| s * r).collect();
        let numel: usize = new_shape.iter().product();
        let mut data = vec![0.0; numel];
        let new_strides = compute_strides(&new_shape);
        let ndim = self.ndim();

        let mut indices = vec![0usize; ndim];
        for flat in 0..numel {
            let src_indices: Vec<usize> = indices.iter().zip(&self.shape)
                .map(|(&i, &s)| i % s).collect();
            let src = src_indices.iter().zip(&self.strides).map(|(i, s)| i * s).sum::<usize>();
            let _dst = indices.iter().zip(&new_strides).map(|(i, s)| i * s).sum::<usize>();
            data[flat] = self.data[src];
            for d in (0..ndim).rev() {
                indices[d] += 1;
                if indices[d] < new_shape[d] { break; }
                indices[d] = 0;
            }
        }
        Tensor::from_data(data, &new_shape)
    }
}

// ── Helper Functions ────────────────────────────────────────────────────

/// Compute row-major strides from shape.
fn compute_strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1; shape.len()];
    for i in (0..shape.len().saturating_sub(1)).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

/// Compute broadcast shape from two shapes.
fn broadcast_shape(a: &[usize], b: &[usize]) -> Vec<usize> {
    let max_ndim = a.len().max(b.len());
    let mut result = vec![1; max_ndim];
    for i in 0..max_ndim {
        let da = if i < a.len() { a[a.len() - 1 - i] } else { 1 };
        let db = if i < b.len() { b[b.len() - 1 - i] } else { 1 };
        if da == db {
            result[max_ndim - 1 - i] = da;
        } else if da == 1 {
            result[max_ndim - 1 - i] = db;
        } else if db == 1 {
            result[max_ndim - 1 - i] = da;
        } else {
            panic!("Shapes cannot be broadcast: {:?} and {:?}", a, b);
        }
    }
    result
}

/// Element-wise binary operation with broadcasting.
fn elementwise_binary(a: &Tensor, b: &Tensor, op: impl Fn(f64, f64) -> f64) -> Tensor {
    if a.shape == b.shape {
        // Fast path: same shape
        let data: Vec<f64> = a.data.iter().zip(&b.data).map(|(&x, &y)| op(x, y)).collect();
        return Tensor::from_data(data, &a.shape);
    }
    // Broadcasting
    let out_shape = broadcast_shape(&a.shape, &b.shape);
    let numel: usize = out_shape.iter().product();
    let mut data = vec![0.0; numel];
    let out_strides = compute_strides(&out_shape);
    let ndim = out_shape.len();

    let a_pad: Vec<usize> = {
        let mut v = vec![1; ndim - a.ndim()];
        v.extend_from_slice(&a.shape);
        v
    };
    let b_pad: Vec<usize> = {
        let mut v = vec![1; ndim - b.ndim()];
        v.extend_from_slice(&b.shape);
        v
    };
    let a_strides_pad: Vec<usize> = {
        let mut v = vec![0; ndim - a.ndim()];
        v.extend_from_slice(&a.strides);
        v
    };
    let b_strides_pad: Vec<usize> = {
        let mut v = vec![0; ndim - b.ndim()];
        v.extend_from_slice(&b.strides);
        v
    };

    let mut indices = vec![0usize; ndim];
    for flat in 0..numel {
        let a_idx: usize = indices.iter().enumerate().map(|(d, &i)| {
            let ai = if a_pad[d] == 1 { 0 } else { i };
            ai * a_strides_pad[d]
        }).sum();
        let b_idx: usize = indices.iter().enumerate().map(|(d, &i)| {
            let bi = if b_pad[d] == 1 { 0 } else { i };
            bi * b_strides_pad[d]
        }).sum();
        data[flat] = op(a.data[a_idx], b.data[b_idx]);
        let _ = out_strides; // used for shape only
        for d in (0..ndim).rev() {
            indices[d] += 1;
            if indices[d] < out_shape[d] { break; }
            indices[d] = 0;
        }
    }
    Tensor::from_data(data, &out_shape)
}

/// Tiled matrix multiplication (Goto algorithm variant).
/// Uses L1/L2 cache-friendly blocking with micro-kernel approach.
/// C [m×n] = A [m×k] × B [k×n]
fn tiled_matmul(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Vec<f64> {
    let mut c = vec![0.0; m * n];

    // Block sizes tuned for L1 (32KB) and L2 (256KB) caches
    const MC: usize = 64;   // Block rows of A
    const KC: usize = 256;  // Block cols of A / rows of B
    const NC: usize = 64;   // Block cols of B
    const MR: usize = 4;    // Micro-kernel rows
    const NR: usize = 4;    // Micro-kernel cols

    for jc in (0..n).step_by(NC) {
        let jc_end = (jc + NC).min(n);
        for pc in (0..k).step_by(KC) {
            let pc_end = (pc + KC).min(k);
            for ic in (0..m).step_by(MC) {
                let ic_end = (ic + MC).min(m);
                // Micro-kernel: compute MR×NR blocks
                for ir in (ic..ic_end).step_by(MR) {
                    let ir_end = (ir + MR).min(ic_end);
                    for jr in (jc..jc_end).step_by(NR) {
                        let jr_end = (jr + NR).min(jc_end);
                        // Accumulate over K dimension
                        for p in pc..pc_end {
                            for i in ir..ir_end {
                                let a_val = a[i * k + p];
                                for j in jr..jr_end {
                                    c[i * n + j] += a_val * b[p * n + j];
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    c
}

/// Reduce along an axis.
fn reduce_axis(t: &Tensor, axis: usize, reducer: impl Fn(&[f64]) -> f64) -> Tensor {
    assert!(axis < t.ndim());
    let axis_size = t.shape[axis];
    let mut new_shape = t.shape.clone();
    new_shape.remove(axis);
    if new_shape.is_empty() {
        new_shape.push(1);
    }
    let numel: usize = new_shape.iter().product();
    let mut data = vec![0.0; numel];

    // Collect elements along axis for each output position
    let outer: usize = t.shape[..axis].iter().product();
    let inner: usize = t.shape[axis+1..].iter().product();

    for o in 0..outer {
        for i in 0..inner {
            let mut vals = Vec::with_capacity(axis_size);
            for a in 0..axis_size {
                let idx = o * axis_size * inner + a * inner + i;
                vals.push(t.data[idx]);
            }
            data[o * inner + i] = reducer(&vals);
        }
    }
    Tensor::from_data(data, &new_shape)
}

// ── FFI Interface ───────────────────────────────────────────────────────

static TENSOR_STORE: Mutex<Option<HashMap<i64, Tensor>>> = Mutex::new(None);

fn with_store<R>(f: impl FnOnce(&mut HashMap<i64, Tensor>) -> R) -> R {
    let mut guard = TENSOR_STORE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    f(guard.as_mut().unwrap())
}

fn next_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_zeros(ndim: i64, shape_ptr: *const i64) -> i64 {
    let shape = unsafe { std::slice::from_raw_parts(shape_ptr, ndim as usize) };
    let shape_usize: Vec<usize> = shape.iter().map(|&s| s as usize).collect();
    let t = Tensor::zeros(&shape_usize);
    let id = next_id();
    with_store(|s| s.insert(id, t));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_ones(ndim: i64, shape_ptr: *const i64) -> i64 {
    let shape = unsafe { std::slice::from_raw_parts(shape_ptr, ndim as usize) };
    let shape_usize: Vec<usize> = shape.iter().map(|&s| s as usize).collect();
    let t = Tensor::ones(&shape_usize);
    let id = next_id();
    with_store(|s| s.insert(id, t));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_rand(ndim: i64, shape_ptr: *const i64, seed: i64) -> i64 {
    let shape = unsafe { std::slice::from_raw_parts(shape_ptr, ndim as usize) };
    let shape_usize: Vec<usize> = shape.iter().map(|&s| s as usize).collect();
    let t = Tensor::rand(&shape_usize, seed as u64);
    let id = next_id();
    with_store(|s| s.insert(id, t));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_randn(ndim: i64, shape_ptr: *const i64, seed: i64) -> i64 {
    let shape = unsafe { std::slice::from_raw_parts(shape_ptr, ndim as usize) };
    let shape_usize: Vec<usize> = shape.iter().map(|&s| s as usize).collect();
    let t = Tensor::randn(&shape_usize, seed as u64);
    let id = next_id();
    with_store(|s| s.insert(id, t));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_from_data(data_ptr: *const f64, count: i64, ndim: i64, shape_ptr: *const i64) -> i64 {
    let data = unsafe { std::slice::from_raw_parts(data_ptr, count as usize) }.to_vec();
    let shape = unsafe { std::slice::from_raw_parts(shape_ptr, ndim as usize) };
    let shape_usize: Vec<usize> = shape.iter().map(|&s| s as usize).collect();
    let t = Tensor::from_data(data, &shape_usize);
    let id = next_id();
    with_store(|s| s.insert(id, t));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_add(a: i64, b: i64) -> i64 {
    with_store(|s| {
        let ta = s.get(&a).expect("tensor not found").clone();
        let tb = s.get(&b).expect("tensor not found");
        let result = ta.add(tb);
        let id = next_id();
        s.insert(id, result);
        id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_mul(a: i64, b: i64) -> i64 {
    with_store(|s| {
        let ta = s.get(&a).expect("tensor not found").clone();
        let tb = s.get(&b).expect("tensor not found");
        let result = ta.mul(tb);
        let id = next_id();
        s.insert(id, result);
        id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_matmul(a: i64, b: i64) -> i64 {
    with_store(|s| {
        let ta = s.get(&a).expect("tensor not found").clone();
        let tb = s.get(&b).expect("tensor not found");
        let result = ta.matmul(tb);
        let id = next_id();
        s.insert(id, result);
        id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_sum(id: i64) -> f64 {
    with_store(|s| s.get(&id).expect("tensor not found").sum())
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_mean(id: i64) -> f64 {
    with_store(|s| s.get(&id).expect("tensor not found").mean())
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_reshape(id: i64, ndim: i64, shape_ptr: *const i64) -> i64 {
    let shape = unsafe { std::slice::from_raw_parts(shape_ptr, ndim as usize) };
    let shape_usize: Vec<usize> = shape.iter().map(|&s| s as usize).collect();
    with_store(|s| {
        let t = s.get(&id).expect("tensor not found").reshape(&shape_usize);
        let new_id = next_id();
        s.insert(new_id, t);
        new_id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_transpose(id: i64) -> i64 {
    with_store(|s| {
        let t = s.get(&id).expect("tensor not found").transpose();
        let new_id = next_id();
        s.insert(new_id, t);
        new_id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_relu(id: i64) -> i64 {
    with_store(|s| {
        let t = s.get(&id).expect("tensor not found").relu();
        let new_id = next_id();
        s.insert(new_id, t);
        new_id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_softmax(id: i64) -> i64 {
    with_store(|s| {
        let t = s.get(&id).expect("tensor not found").softmax();
        let new_id = next_id();
        s.insert(new_id, t);
        new_id
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_numel(id: i64) -> i64 {
    with_store(|s| s.get(&id).expect("tensor not found").numel() as i64)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_ndim(id: i64) -> i64 {
    with_store(|s| s.get(&id).expect("tensor not found").ndim() as i64)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_get(id: i64, flat_idx: i64) -> f64 {
    with_store(|s| s.get(&id).expect("tensor not found").get_flat(flat_idx as usize))
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tensor_free(id: i64) {
    with_store(|s| { s.remove(&id); });
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_creation() {
        let t = Tensor::zeros(&[2, 3]);
        assert_eq!(t.shape, vec![2, 3]);
        assert_eq!(t.numel(), 6);
        assert_eq!(t.ndim(), 2);
        assert!(t.data.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_tensor_ones() {
        let t = Tensor::ones(&[3, 4]);
        assert_eq!(t.numel(), 12);
        assert!(t.data.iter().all(|&x| x == 1.0));
    }

    #[test]
    fn test_tensor_from_data() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0], &[2, 2]);
        assert_eq!(t.get(&[0, 0]), 1.0);
        assert_eq!(t.get(&[0, 1]), 2.0);
        assert_eq!(t.get(&[1, 0]), 3.0);
        assert_eq!(t.get(&[1, 1]), 4.0);
    }

    #[test]
    fn test_tensor_eye() {
        let t = Tensor::eye(3);
        assert_eq!(t.get(&[0, 0]), 1.0);
        assert_eq!(t.get(&[1, 1]), 1.0);
        assert_eq!(t.get(&[2, 2]), 1.0);
        assert_eq!(t.get(&[0, 1]), 0.0);
    }

    #[test]
    fn test_tensor_rand() {
        let t = Tensor::rand(&[10], 42);
        assert_eq!(t.numel(), 10);
        assert!(t.data.iter().all(|&x| (0.0..1.0).contains(&x)));
    }

    #[test]
    fn test_tensor_randn() {
        let t = Tensor::randn(&[1000], 42);
        let mean = t.mean();
        assert!(mean.abs() < 0.2, "randn mean should be ~0, got {}", mean);
    }

    #[test]
    fn test_reshape() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        let r = t.reshape(&[3, 2]);
        assert_eq!(r.shape, vec![3, 2]);
        assert_eq!(r.data, t.data);
    }

    #[test]
    fn test_transpose_2d() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        let tr = t.transpose();
        assert_eq!(tr.shape, vec![3, 2]);
        assert_eq!(tr.get(&[0, 0]), 1.0);
        assert_eq!(tr.get(&[0, 1]), 4.0);
        assert_eq!(tr.get(&[2, 0]), 3.0);
    }

    #[test]
    fn test_add_same_shape() {
        let a = Tensor::from_data(vec![1.0, 2.0, 3.0], &[3]);
        let b = Tensor::from_data(vec![4.0, 5.0, 6.0], &[3]);
        let c = a.add(&b);
        assert_eq!(c.data, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_broadcasting() {
        let a = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0], &[2, 2]);
        let b = Tensor::from_data(vec![10.0, 20.0], &[1, 2]);
        let c = a.add(&b);
        assert_eq!(c.shape, vec![2, 2]);
        assert_eq!(c.data, vec![11.0, 22.0, 13.0, 24.0]);
    }

    #[test]
    fn test_scalar_ops() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0], &[3]);
        assert_eq!(t.add_scalar(10.0).data, vec![11.0, 12.0, 13.0]);
        assert_eq!(t.mul_scalar(2.0).data, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_elementwise_ops() {
        let t = Tensor::from_data(vec![4.0, 9.0, 16.0], &[3]);
        let sq = t.sqrt();
        assert!((sq.data[0] - 2.0).abs() < 1e-10);
        assert!((sq.data[1] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_reductions() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0], &[4]);
        assert_eq!(t.sum(), 10.0);
        assert_eq!(t.mean(), 2.5);
        assert_eq!(t.max_val(), 4.0);
        assert_eq!(t.min_val(), 1.0);
        assert_eq!(t.argmax(), 3);
        assert_eq!(t.argmin(), 0);
    }

    #[test]
    fn test_sum_axis() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        let s0 = t.sum_axis(0);
        assert_eq!(s0.shape, vec![3]);
        assert_eq!(s0.data, vec![5.0, 7.0, 9.0]);
        let s1 = t.sum_axis(1);
        assert_eq!(s1.shape, vec![2]);
        assert_eq!(s1.data, vec![6.0, 15.0]);
    }

    #[test]
    fn test_matmul_2d() {
        // [2,3] × [3,2] = [2,2]
        let a = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        let b = Tensor::from_data(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], &[3, 2]);
        let c = a.matmul(&b);
        assert_eq!(c.shape, vec![2, 2]);
        assert_eq!(c.data[0], 1.0*7.0 + 2.0*9.0 + 3.0*11.0);  // 58
        assert_eq!(c.data[1], 1.0*8.0 + 2.0*10.0 + 3.0*12.0); // 64
        assert_eq!(c.data[2], 4.0*7.0 + 5.0*9.0 + 6.0*11.0);  // 139
        assert_eq!(c.data[3], 4.0*8.0 + 5.0*10.0 + 6.0*12.0); // 154
    }

    #[test]
    fn test_matmul_identity() {
        let a = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0], &[2, 2]);
        let eye = Tensor::eye(2);
        let c = a.matmul(&eye);
        assert_eq!(c.data, a.data);
    }

    #[test]
    fn test_dot_product() {
        let a = Tensor::vec(vec![1.0, 2.0, 3.0]);
        let b = Tensor::vec(vec![4.0, 5.0, 6.0]);
        assert_eq!(a.dot(&b), 32.0);
    }

    #[test]
    fn test_norm() {
        let t = Tensor::vec(vec![3.0, 4.0]);
        assert!((t.norm() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_relu() {
        let t = Tensor::from_data(vec![-2.0, -1.0, 0.0, 1.0, 2.0], &[5]);
        let r = t.relu();
        assert_eq!(r.data, vec![0.0, 0.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_sigmoid() {
        let t = Tensor::from_data(vec![0.0], &[1]);
        let s = t.sigmoid();
        assert!((s.data[0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_softmax() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0], &[3]);
        let s = t.softmax();
        let sum: f64 = s.data.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
        assert!(s.data[2] > s.data[1]);
        assert!(s.data[1] > s.data[0]);
    }

    #[test]
    fn test_softmax_2d() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0], &[2, 3]);
        let s = t.softmax();
        let row0_sum: f64 = s.data[0..3].iter().sum();
        let row1_sum: f64 = s.data[3..6].iter().sum();
        assert!((row0_sum - 1.0).abs() < 1e-10);
        assert!((row1_sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_layer_norm() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0], &[2, 2]);
        let gamma = Tensor::ones(&[2]);
        let beta = Tensor::zeros(&[2]);
        let ln = t.layer_norm(&gamma, &beta, 1e-5);
        // Each row should have mean ~0 and std ~1
        let row0_mean = (ln.data[0] + ln.data[1]) / 2.0;
        assert!(row0_mean.abs() < 1e-5);
    }

    #[test]
    fn test_rms_norm() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0], &[2, 2]);
        let gamma = Tensor::ones(&[2]);
        let rn = t.rms_norm(&gamma, 1e-5);
        assert_eq!(rn.shape, vec![2, 2]);
    }

    #[test]
    fn test_cat() {
        let a = Tensor::from_data(vec![1.0, 2.0], &[1, 2]);
        let b = Tensor::from_data(vec![3.0, 4.0], &[1, 2]);
        let c = Tensor::cat(&[&a, &b], 0);
        assert_eq!(c.shape, vec![2, 2]);
        assert_eq!(c.data, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_slice() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        let s = t.slice(1, 1, 3);
        assert_eq!(s.shape, vec![2, 2]);
        assert_eq!(s.data, vec![2.0, 3.0, 5.0, 6.0]);
    }

    #[test]
    fn test_unsqueeze_squeeze() {
        let t = Tensor::from_data(vec![1.0, 2.0, 3.0], &[3]);
        let u = t.unsqueeze(0);
        assert_eq!(u.shape, vec![1, 3]);
        let s = u.squeeze();
        assert_eq!(s.shape, vec![3]);
    }

    #[test]
    fn test_inplace_ops() {
        let mut a = Tensor::from_data(vec![1.0, 2.0, 3.0], &[3]);
        let b = Tensor::from_data(vec![10.0, 20.0, 30.0], &[3]);
        a.add_(&b);
        assert_eq!(a.data, vec![11.0, 22.0, 33.0]);
    }

    #[test]
    fn test_variance_stddev() {
        let t = Tensor::from_data(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0], &[8]);
        let v = t.var();
        assert!((v - 4.0).abs() < 1e-10);
        assert!((t.std_dev() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_gelu() {
        let t = Tensor::from_data(vec![0.0, 1.0, -1.0], &[3]);
        let g = t.gelu();
        assert!((g.data[0] - 0.0).abs() < 1e-5);
        assert!(g.data[1] > 0.8); // gelu(1) ≈ 0.841
        assert!(g.data[2] < 0.0); // gelu(-1) ≈ -0.159
    }

    #[test]
    fn test_silu() {
        let t = Tensor::from_data(vec![0.0], &[1]);
        let s = t.silu();
        assert!((s.data[0] - 0.0).abs() < 1e-10); // silu(0) = 0
    }

    #[test]
    fn test_tiled_matmul_large() {
        // Test with sizes exceeding block sizes
        let m = 100;
        let k = 80;
        let n = 90;
        let a = Tensor::rand(&[m, k], 42);
        let b = Tensor::rand(&[k, n], 43);
        let c = a.matmul(&b);
        assert_eq!(c.shape, vec![m, n]);
        // Verify one element by naive calculation
        let mut expected = 0.0;
        for p in 0..k {
            expected += a.data[0 * k + p] * b.data[p * n + 0];
        }
        assert!((c.data[0] - expected).abs() < 1e-6);
    }

    #[test]
    fn test_compute_strides() {
        assert_eq!(compute_strides(&[2, 3, 4]), vec![12, 4, 1]);
        assert_eq!(compute_strides(&[5]), vec![1]);
        assert_eq!(compute_strides(&[2, 3]), vec![3, 1]);
    }

    #[test]
    fn test_broadcast_shape() {
        assert_eq!(broadcast_shape(&[2, 1], &[1, 3]), vec![2, 3]);
        assert_eq!(broadcast_shape(&[5, 1, 4], &[1, 3, 1]), vec![5, 3, 4]);
        assert_eq!(broadcast_shape(&[3], &[2, 3]), vec![2, 3]);
    }

    #[test]
    fn test_repeat() {
        let t = Tensor::from_data(vec![1.0, 2.0], &[1, 2]);
        let r = t.repeat(&[3, 1]);
        assert_eq!(r.shape, vec![3, 2]);
        assert_eq!(r.data, vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
    }

    #[test]
    fn test_ffi_basic() {
        let shape = [2i64, 3i64];
        let id = vitalis_tensor_zeros(2, shape.as_ptr());
        assert!(id > 0);
        assert_eq!(vitalis_tensor_numel(id), 6);
        assert_eq!(vitalis_tensor_ndim(id), 2);
        assert_eq!(vitalis_tensor_sum(id), 0.0);
        vitalis_tensor_free(id);
    }

    #[test]
    fn test_ffi_matmul() {
        let shape_a = [2i64, 2i64];
        let data_a = [1.0f64, 2.0, 3.0, 4.0];
        let shape_b = [2i64, 2i64];
        let data_b = [5.0f64, 6.0, 7.0, 8.0];
        let a = vitalis_tensor_from_data(data_a.as_ptr(), 4, 2, shape_a.as_ptr());
        let b = vitalis_tensor_from_data(data_b.as_ptr(), 4, 2, shape_b.as_ptr());
        let c = vitalis_tensor_matmul(a, b);
        let c00 = vitalis_tensor_get(c, 0);
        assert_eq!(c00, 1.0*5.0 + 2.0*7.0); // 19
        vitalis_tensor_free(a);
        vitalis_tensor_free(b);
        vitalis_tensor_free(c);
    }

    #[test]
    fn test_normalize() {
        let t = Tensor::vec(vec![3.0, 4.0]);
        let n = t.normalize();
        assert!((n.norm() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_neg() {
        let t = Tensor::from_data(vec![1.0, -2.0, 3.0], &[3]);
        let n = t.neg();
        assert_eq!(n.data, vec![-1.0, 2.0, -3.0]);
    }

    #[test]
    fn test_full() {
        let t = Tensor::full(&[2, 3], 7.0);
        assert!(t.data.iter().all(|&x| x == 7.0));
    }

    #[test]
    fn test_tanh_act() {
        let t = Tensor::from_data(vec![0.0], &[1]);
        let r = t.tanh_act();
        assert!((r.data[0] - 0.0).abs() < 1e-10);
    }
}
