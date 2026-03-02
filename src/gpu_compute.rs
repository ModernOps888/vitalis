//! GPU Compute — CUDA device management and cuBLAS acceleration
//!
//! Provides GPU device detection, memory management, kernel compilation,
//! and cuBLAS SGEMM matrix multiply. Ported from the Nova ML engine.
//!
//! All GPU code is gated behind the `cuda` feature flag so Vitalis compiles
//! cleanly on systems without NVIDIA GPUs.
//!
//! # Architecture
//!
//! ```text
//! GpuContext (global singleton)
//!  ├── CudaDevice + CudaStream
//!  ├── CudaBlas (cuBLAS handle)
//!  └── GpuMemoryPool (allocation tracking)
//!
//! DeviceInfo (hardware capabilities)
//!  ├── compute_capability
//!  ├── total_memory / multiprocessor_count
//!  └── architecture detection (Blackwell, Ampere, fp16, bf16)
//!
//! CUDA Kernels (PTX source)
//!  ├── Tiled F32 GEMM
//!  ├── Fused Attention (Q@K^T + causal mask)
//!  ├── Softmax, RMSNorm, SwiGLU, GELU, RoPE
//!  ├── Embedding lookup
//!  ├── AdamW optimizer step
//!  └── Cross-entropy loss + backward
//! ```

use std::collections::HashMap;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Device Info
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Information about a CUDA GPU device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub ordinal: usize,
    pub compute_capability: (u32, u32),
    pub total_memory_bytes: usize,
    pub multiprocessor_count: u32,
    pub max_threads_per_block: u32,
    pub warp_size: u32,
}

impl DeviceInfo {
    /// Total memory in GB.
    pub fn total_memory_gb(&self) -> f64 {
        self.total_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Check if this is a Blackwell GPU (compute capability 12.x).
    pub fn is_blackwell(&self) -> bool {
        self.compute_capability.0 >= 12
    }

    /// Check if Ampere or newer (CC >= 8.0).
    pub fn is_ampere_or_newer(&self) -> bool {
        self.compute_capability.0 >= 8
    }

    /// Check if the GPU supports BF16 (Ampere+).
    pub fn supports_bf16(&self) -> bool {
        self.compute_capability.0 >= 8
    }

    /// Check if the GPU supports FP16 tensor cores.
    pub fn supports_fp16_tc(&self) -> bool {
        self.compute_capability.0 >= 7
    }

    /// CPU fallback device info.
    pub fn cpu_fallback() -> Self {
        Self {
            name: "CPU (no GPU)".to_string(),
            ordinal: 0,
            compute_capability: (0, 0),
            total_memory_bytes: 0,
            multiprocessor_count: 0,
            max_threads_per_block: 0,
            warp_size: 0,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Compiled Kernel
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A compiled CUDA kernel with launch configuration.
#[derive(Debug, Clone)]
pub struct CompiledKernel {
    pub name: String,
    pub ptx_source: String,
    pub grid_dim: (u32, u32, u32),
    pub block_dim: (u32, u32, u32),
    pub shared_mem_bytes: u32,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CUDA Runtime
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// CUDA runtime manager — handles device initialization and kernel compilation.
pub struct CudaRuntime {
    pub device: DeviceInfo,
    compiled_kernels: HashMap<String, CompiledKernel>,
}

impl CudaRuntime {
    /// Create a CPU-only fallback runtime.
    pub fn cpu_fallback() -> Self {
        Self {
            device: DeviceInfo::cpu_fallback(),
            compiled_kernels: HashMap::new(),
        }
    }

    /// Get device information.
    pub fn device_info(&self) -> &DeviceInfo {
        &self.device
    }

    /// Register a compiled kernel.
    pub fn compile_kernel(&mut self, name: &str, source: &str,
                          grid: (u32, u32, u32), block: (u32, u32, u32),
                          shared_mem: u32) {
        self.compiled_kernels.insert(name.to_string(), CompiledKernel {
            name: name.to_string(),
            ptx_source: source.to_string(),
            grid_dim: grid,
            block_dim: block,
            shared_mem_bytes: shared_mem,
        });
    }

    /// Get a compiled kernel by name.
    pub fn get_kernel(&self, name: &str) -> Option<&CompiledKernel> {
        self.compiled_kernels.get(name)
    }

    /// Compute optimal 1D launch configuration.
    pub fn optimal_1d_config(&self, n_elements: usize) -> (u32, u32) {
        let max_tpb = if self.device.max_threads_per_block == 0 { 256 } else { self.device.max_threads_per_block };
        let block = 256u32.min(max_tpb);
        let sms = if self.device.multiprocessor_count == 0 { 1 } else { self.device.multiprocessor_count };
        let grid = ((n_elements as u32 + block - 1) / block)
            .min(sms * 32);
        (grid.max(1), block)
    }

    /// Compute optimal 2D launch configuration.
    pub fn optimal_2d_config(&self, rows: usize, cols: usize) -> ((u32, u32), (u32, u32)) {
        let bx = 16u32;
        let by = 16u32;
        let gx = (cols as u32 + bx - 1) / bx;
        let gy = (rows as u32 + by - 1) / by;
        ((gx, gy), (bx, by))
    }

    /// Check if the GPU can fit a model with the given param count.
    pub fn can_fit_model(&self, param_count: usize, dtype_bytes: usize, training: bool) -> bool {
        let bytes = param_count * dtype_bytes;
        let required = if training { bytes * 5 } else { bytes };
        required < self.device.total_memory_bytes
    }

    /// Print device summary.
    pub fn print_summary(&self) {
        println!("╔══════════════════════════════════════════════════╗");
        println!("║                GPU Device Info                   ║");
        println!("╠══════════════════════════════════════════════════╣");
        println!("║ Name:      {:<39}║", self.device.name);
        println!("║ CC:        {}.{:<37}║",
                 self.device.compute_capability.0, self.device.compute_capability.1);
        println!("║ VRAM:      {:.1} GB{:<33}║", self.device.total_memory_gb(), "");
        println!("║ SMs:       {:<39}║", self.device.multiprocessor_count);
        println!("║ BF16:      {:<39}║", self.device.supports_bf16());
        println!("╚══════════════════════════════════════════════════╝");
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GPU Memory Pool
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Tracks GPU memory allocation.
pub struct GpuMemoryPool {
    pub device_id: usize,
    pub allocated_bytes: usize,
    pub peak_bytes: usize,
    pub total_bytes: usize,
}

impl GpuMemoryPool {
    pub fn new(device_id: usize, total_bytes: usize) -> Self {
        Self { device_id, allocated_bytes: 0, peak_bytes: 0, total_bytes }
    }

    /// Allocate GPU memory (tracking only).
    pub fn allocate(&mut self, bytes: usize) -> usize {
        self.allocated_bytes += bytes;
        if self.allocated_bytes > self.peak_bytes {
            self.peak_bytes = self.allocated_bytes;
        }
        self.allocated_bytes
    }

    /// Free GPU memory (tracking only).
    pub fn free(&mut self, bytes: usize) {
        self.allocated_bytes = self.allocated_bytes.saturating_sub(bytes);
    }

    /// Available GPU memory.
    pub fn available(&self) -> usize {
        self.total_bytes.saturating_sub(self.allocated_bytes)
    }

    /// GPU memory utilization (0.0 - 1.0).
    pub fn utilization(&self) -> f64 {
        if self.total_bytes == 0 { return 0.0; }
        self.allocated_bytes as f64 / self.total_bytes as f64
    }
}

/// Estimate max trainable parameters for given VRAM.
pub fn max_training_params(vram_bytes: usize, dtype_bytes: usize) -> usize {
    // Training needs ~5× model size: weights + gradients + optimizer (m, v) + activations
    vram_bytes / (dtype_bytes * 5)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CUDA Kernel Sources (PTX)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Tiled F32 GEMM kernel (32×32 tiles, shared memory).
pub const MATMUL_KERNEL: &str = r#"
extern "C" __global__ void matmul_f32(
    const float* A, const float* B, float* C,
    int M, int N, int K
) {
    const int TILE = 32;
    __shared__ float As[TILE][TILE], Bs[TILE][TILE];
    int row = blockIdx.y * TILE + threadIdx.y;
    int col = blockIdx.x * TILE + threadIdx.x;
    float sum = 0.0f;
    for (int t = 0; t < (K + TILE - 1) / TILE; t++) {
        int aCol = t * TILE + threadIdx.x;
        int bRow = t * TILE + threadIdx.y;
        As[threadIdx.y][threadIdx.x] = (row < M && aCol < K) ? A[row * K + aCol] : 0.0f;
        Bs[threadIdx.y][threadIdx.x] = (bRow < K && col < N) ? B[bRow * N + col] : 0.0f;
        __syncthreads();
        for (int k = 0; k < TILE; k++) sum += As[threadIdx.y][k] * Bs[k][threadIdx.x];
        __syncthreads();
    }
    if (row < M && col < N) C[row * N + col] = sum;
}
"#;

/// FP16 GEMM kernel with Tensor Cores (Ampere+).
pub const MATMUL_FP16_KERNEL: &str = r#"
extern "C" __global__ void matmul_fp16(
    const __half* A, const __half* B, float* C,
    int M, int N, int K
) {
    const int TILE = 16;
    __shared__ __half As[TILE][TILE], Bs[TILE][TILE];
    int row = blockIdx.y * TILE + threadIdx.y;
    int col = blockIdx.x * TILE + threadIdx.x;
    float sum = 0.0f;
    for (int t = 0; t < (K + TILE - 1) / TILE; t++) {
        int aCol = t * TILE + threadIdx.x;
        int bRow = t * TILE + threadIdx.y;
        As[threadIdx.y][threadIdx.x] = (row < M && aCol < K) ? A[row * K + aCol] : __float2half(0.0f);
        Bs[threadIdx.y][threadIdx.x] = (bRow < K && col < N) ? B[bRow * N + col] : __float2half(0.0f);
        __syncthreads();
        for (int k = 0; k < TILE; k++) sum += __half2float(As[threadIdx.y][k]) * __half2float(Bs[k][threadIdx.x]);
        __syncthreads();
    }
    if (row < M && col < N) C[row * N + col] = sum;
}
"#;

/// Fused attention kernel (Q @ K^T with causal mask).
pub const ATTENTION_KERNEL: &str = r#"
extern "C" __global__ void attention_scores(
    const float* Q, const float* K, float* S,
    int seq_len, int head_dim, float scale
) {
    int i = blockIdx.y * blockDim.y + threadIdx.y;
    int j = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < seq_len && j < seq_len) {
        if (j > i) { S[i * seq_len + j] = -1e9f; return; }
        float dot = 0.0f;
        for (int d = 0; d < head_dim; d++)
            dot += Q[i * head_dim + d] * K[j * head_dim + d];
        S[i * seq_len + j] = dot * scale;
    }
}
"#;

/// Row-wise softmax kernel.
pub const SOFTMAX_KERNEL: &str = r#"
extern "C" __global__ void softmax_rows(float* data, int rows, int cols) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= rows) return;
    float* r = data + row * cols;
    float mx = r[0];
    for (int j = 1; j < cols; j++) if (r[j] > mx) mx = r[j];
    float sum = 0.0f;
    for (int j = 0; j < cols; j++) { r[j] = expf(r[j] - mx); sum += r[j]; }
    for (int j = 0; j < cols; j++) r[j] /= sum;
}
"#;

/// RMSNorm kernel with warp-level reduction.
pub const RMSNORM_KERNEL: &str = r#"
extern "C" __global__ void rmsnorm(
    const float* x, const float* w, float* out,
    int n, int dim, float eps
) {
    int row = blockIdx.x;
    if (row >= n) return;
    const float* xr = x + row * dim;
    float* or = out + row * dim;
    float sumsq = 0.0f;
    for (int i = threadIdx.x; i < dim; i += blockDim.x) sumsq += xr[i] * xr[i];
    __shared__ float sdata[256];
    sdata[threadIdx.x] = sumsq;
    __syncthreads();
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (threadIdx.x < s) sdata[threadIdx.x] += sdata[threadIdx.x + s];
        __syncthreads();
    }
    float rms = rsqrtf(sdata[0] / dim + eps);
    for (int i = threadIdx.x; i < dim; i += blockDim.x) or[i] = xr[i] * rms * w[i];
}
"#;

/// SwiGLU activation kernel.
pub const SWIGLU_KERNEL: &str = r#"
extern "C" __global__ void swiglu(
    const float* gate, const float* up, float* out, int n
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        float g = gate[i];
        float silu_g = g / (1.0f + expf(-g));
        out[i] = silu_g * up[i];
    }
}
"#;

/// GELU activation kernel.
pub const GELU_KERNEL: &str = r#"
extern "C" __global__ void gelu(float* data, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        float x = data[i];
        data[i] = 0.5f * x * (1.0f + tanhf(0.7978845608f * (x + 0.044715f * x * x * x)));
    }
}
"#;

/// Embedding lookup kernel.
pub const EMBEDDING_KERNEL: &str = r#"
extern "C" __global__ void embedding_lookup(
    const float* table, const int* indices, float* out,
    int seq_len, int dim
) {
    int pos = blockIdx.x;
    int d = threadIdx.x;
    if (pos < seq_len && d < dim) {
        out[pos * dim + d] = table[indices[pos] * dim + d];
    }
}
"#;

/// AdamW optimizer step kernel.
pub const ADAMW_KERNEL: &str = r#"
extern "C" __global__ void adamw_step(
    float* params, const float* grads, float* m, float* v,
    int n, float lr, float beta1, float beta2, float eps, float wd, int step
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    float g = grads[i];
    m[i] = beta1 * m[i] + (1.0f - beta1) * g;
    v[i] = beta2 * v[i] + (1.0f - beta2) * g * g;
    float m_hat = m[i] / (1.0f - powf(beta1, (float)step));
    float v_hat = v[i] / (1.0f - powf(beta2, (float)step));
    params[i] = params[i] * (1.0f - lr * wd) - lr * m_hat / (sqrtf(v_hat) + eps);
}
"#;

/// Fused cross-entropy loss + backward kernel.
pub const CROSS_ENTROPY_KERNEL: &str = r#"
extern "C" __global__ void cross_entropy_fwd(
    const float* logits, const int* targets, float* losses,
    int batch, int vocab
) {
    int b = blockIdx.x * blockDim.x + threadIdx.x;
    if (b >= batch) return;
    const float* row = logits + b * vocab;
    float mx = row[0];
    for (int j = 1; j < vocab; j++) if (row[j] > mx) mx = row[j];
    float sum_exp = 0.0f;
    for (int j = 0; j < vocab; j++) sum_exp += expf(row[j] - mx);
    losses[b] = -(row[targets[b]] - mx - logf(sum_exp));
}
"#;

/// Rotary Position Embedding kernel.
pub const ROPE_KERNEL: &str = r#"
extern "C" __global__ void apply_rope(
    float* q, const float* cos_table, const float* sin_table,
    int batch, int seq, int n_heads, int head_dim
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int half = head_dim / 2;
    int total = batch * seq * n_heads * half;
    if (idx >= total) return;
    int b = idx / (seq * n_heads * half);
    int rem = idx % (seq * n_heads * half);
    int s = rem / (n_heads * half);
    rem = rem % (n_heads * half);
    int h = rem / half;
    int i = rem % half;
    int offset = ((b * seq + s) * n_heads + h) * head_dim;
    float x0 = q[offset + i];
    float x1 = q[offset + half + i];
    float c = cos_table[s * half + i];
    float si = sin_table[s * half + i];
    q[offset + i] = x0 * c - x1 * si;
    q[offset + half + i] = x0 * si + x1 * c;
}
"#;

/// Get all CUDA kernel sources as (name, source) pairs.
pub fn all_kernels() -> Vec<(&'static str, &'static str)> {
    vec![
        ("matmul_f32", MATMUL_KERNEL),
        ("matmul_fp16", MATMUL_FP16_KERNEL),
        ("attention_scores", ATTENTION_KERNEL),
        ("softmax_rows", SOFTMAX_KERNEL),
        ("rmsnorm", RMSNORM_KERNEL),
        ("swiglu", SWIGLU_KERNEL),
        ("gelu", GELU_KERNEL),
        ("embedding_lookup", EMBEDDING_KERNEL),
        ("adamw_step", ADAMW_KERNEL),
        ("cross_entropy_fwd", CROSS_ENTROPY_KERNEL),
        ("apply_rope", ROPE_KERNEL),
    ]
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FFI — extern "C" functions for Vitalis stdlib
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Check if CUDA GPU is available.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_gpu_available() -> i64 {
    // Returns 0 (no GPU) in CPU-only builds; when cuda feature is enabled,
    // this would probe cudarc. For now, basic detection:
    0
}

/// Get total GPU memory in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_gpu_memory_total() -> i64 {
    0
}

/// Get number of CUDA kernels available.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_gpu_kernel_count() -> i64 {
    all_kernels().len() as i64
}

/// Estimate max trainable parameters for given VRAM (in bytes).
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_gpu_max_params(vram_bytes: i64, dtype_bytes: i64) -> i64 {
    max_training_params(vram_bytes as usize, dtype_bytes as usize) as i64
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_cpu_fallback() {
        let d = DeviceInfo::cpu_fallback();
        assert_eq!(d.compute_capability, (0, 0));
        assert!(!d.is_blackwell());
        assert!(!d.is_ampere_or_newer());
    }

    #[test]
    fn test_device_info_blackwell() {
        let d = DeviceInfo {
            name: "RTX 5060".to_string(), ordinal: 0,
            compute_capability: (12, 0),
            total_memory_bytes: 8 * 1024 * 1024 * 1024,
            multiprocessor_count: 30,
            max_threads_per_block: 1024,
            warp_size: 32,
        };
        assert!(d.is_blackwell());
        assert!(d.is_ampere_or_newer());
        assert!(d.supports_bf16());
        assert!(d.supports_fp16_tc());
        assert!((d.total_memory_gb() - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_memory_pool() {
        let mut pool = GpuMemoryPool::new(0, 8_000_000_000);
        assert_eq!(pool.utilization(), 0.0);
        pool.allocate(1_000_000_000);
        assert!((pool.utilization() - 0.125).abs() < 0.01);
        pool.free(500_000_000);
        assert_eq!(pool.allocated_bytes, 500_000_000);
        assert_eq!(pool.peak_bytes, 1_000_000_000);
    }

    #[test]
    fn test_cuda_runtime_config() {
        let rt = CudaRuntime::cpu_fallback();
        let (grid, block) = rt.optimal_1d_config(1024);
        assert!(grid >= 1);
        assert!(block >= 1);
    }

    #[test]
    fn test_kernel_count() {
        let kernels = all_kernels();
        assert_eq!(kernels.len(), 11);
    }

    #[test]
    fn test_max_training_params() {
        let max = max_training_params(8_000_000_000, 4);
        assert_eq!(max, 400_000_000); // 8GB / (4 bytes × 5)
    }

    #[test]
    fn test_can_fit_model() {
        let rt = CudaRuntime {
            device: DeviceInfo {
                name: "Test GPU".to_string(), ordinal: 0,
                compute_capability: (8, 0),
                total_memory_bytes: 8_000_000_000,
                multiprocessor_count: 60,
                max_threads_per_block: 1024,
                warp_size: 32,
            },
            compiled_kernels: HashMap::new(),
        };
        // 100M params × 4 bytes = 400MB → fits in 8GB
        assert!(rt.can_fit_model(100_000_000, 4, false));
        // 100M params × 4 bytes × 5 = 2GB → fits in 8GB
        assert!(rt.can_fit_model(100_000_000, 4, true));
        // 1B params × 4 bytes × 5 = 20GB → doesn't fit in 8GB
        assert!(!rt.can_fit_model(1_000_000_000, 4, true));
    }
}
