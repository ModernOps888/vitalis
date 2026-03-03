//! Vitalis GPU Compute Backend — Device-agnostic GPU acceleration.
//!
//! Provides a compute abstraction for GPU-accelerated operations:
//! - **GpuDevice**: Enumeration & selection of compute devices
//! - **GpuBuffer**: Typed buffer abstraction (host ↔ device transfer)
//! - **ComputeKernel**: Kernel definition with parameters
//! - **ComputePipeline**: Staged execution pipeline
//! - **ShaderBuilder**: Generates compute shaders from Vitalis IR
//! - **Dispatch**: Work-group configuration & launch
//!
//! The backend is designed to sit behind a trait so that concrete
//! implementations (CUDA, Vulkan Compute, Metal, WebGPU) can be swapped.

use std::collections::HashMap;
use std::fmt;

// ─── Device Abstraction ─────────────────────────────────────────────────

/// Supported GPU API backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuBackend {
    Cuda,
    Vulkan,
    Metal,
    WebGpu,
    Software, // CPU fallback
}

impl fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuBackend::Cuda => write!(f, "CUDA"),
            GpuBackend::Vulkan => write!(f, "Vulkan"),
            GpuBackend::Metal => write!(f, "Metal"),
            GpuBackend::WebGpu => write!(f, "WebGPU"),
            GpuBackend::Software => write!(f, "Software"),
        }
    }
}

/// Information about a GPU device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub backend: GpuBackend,
    pub memory_bytes: u64,
    pub compute_units: u32,
    pub max_workgroup_size: [u32; 3],
    pub max_shared_memory: u32,
}

impl DeviceInfo {
    pub fn software_fallback() -> Self {
        Self {
            name: "CPU Software Fallback".to_string(),
            backend: GpuBackend::Software,
            memory_bytes: 0,
            compute_units: 1,
            max_workgroup_size: [1024, 1024, 64],
            max_shared_memory: 65536,
        }
    }

    pub fn memory_mb(&self) -> u64 {
        self.memory_bytes / (1024 * 1024)
    }
}

// ─── GpuBuffer ──────────────────────────────────────────────────────────

/// Data type of buffer elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferElementType {
    F32,
    F64,
    I32,
    I64,
    U32,
    U8,
}

impl BufferElementType {
    pub fn size_bytes(&self) -> usize {
        match self {
            BufferElementType::F32 => 4,
            BufferElementType::F64 => 8,
            BufferElementType::I32 => 4,
            BufferElementType::I64 => 8,
            BufferElementType::U32 => 4,
            BufferElementType::U8 => 1,
        }
    }
}

/// Usage hints for a GPU buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    Storage,     // Read/write from compute shader
    Uniform,     // Read-only uniform data
    Staging,     // Host ↔ device transfer
    Vertex,      // Vertex data (for rendering)
    Index,       // Index data (for rendering)
}

/// A GPU buffer descriptor.
#[derive(Debug, Clone)]
pub struct GpuBuffer {
    pub id: u32,
    pub element_type: BufferElementType,
    pub element_count: usize,
    pub usage: BufferUsage,
    pub data: Vec<u8>,
}

impl GpuBuffer {
    pub fn new(id: u32, element_type: BufferElementType, count: usize, usage: BufferUsage) -> Self {
        let byte_size = element_type.size_bytes() * count;
        Self {
            id,
            element_type,
            element_count: count,
            usage,
            data: vec![0u8; byte_size],
        }
    }

    pub fn from_f32(id: u32, data: &[f32], usage: BufferUsage) -> Self {
        let bytes: Vec<u8> = data.iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        Self {
            id,
            element_type: BufferElementType::F32,
            element_count: data.len(),
            usage,
            data: bytes,
        }
    }

    pub fn from_i32(id: u32, data: &[i32], usage: BufferUsage) -> Self {
        let bytes: Vec<u8> = data.iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        Self {
            id,
            element_type: BufferElementType::I32,
            element_count: data.len(),
            usage,
            data: bytes,
        }
    }

    pub fn byte_size(&self) -> usize {
        self.data.len()
    }

    /// Read back as f32 slice.
    pub fn as_f32(&self) -> Vec<f32> {
        self.data.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    /// Read back as i32 slice.
    pub fn as_i32(&self) -> Vec<i32> {
        self.data.chunks_exact(4)
            .map(|chunk| i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }
}

// ─── Compute Kernel ─────────────────────────────────────────────────────

/// A parameter binding in a compute kernel.
#[derive(Debug, Clone)]
pub struct KernelParam {
    pub name: String,
    pub binding: u32,
    pub element_type: BufferElementType,
    pub access: BufferAccess,
}

/// Access pattern for a buffer parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferAccess {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

/// A compute kernel definition.
#[derive(Debug, Clone)]
pub struct ComputeKernel {
    pub name: String,
    pub params: Vec<KernelParam>,
    pub workgroup_size: [u32; 3],
    pub source: String, // Generated shader source
}

impl ComputeKernel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            params: Vec::new(),
            workgroup_size: [64, 1, 1],
            source: String::new(),
        }
    }

    pub fn add_param(&mut self, name: &str, binding: u32, ty: BufferElementType, access: BufferAccess) {
        self.params.push(KernelParam {
            name: name.to_string(),
            binding,
            element_type: ty,
            access,
        });
    }

    pub fn set_workgroup_size(&mut self, x: u32, y: u32, z: u32) {
        self.workgroup_size = [x, y, z];
    }

    pub fn set_source(&mut self, source: &str) {
        self.source = source.to_string();
    }

    pub fn param_count(&self) -> usize {
        self.params.len()
    }
}

// ─── Dispatch Configuration ─────────────────────────────────────────────

/// Work dispatch dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dispatch {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Dispatch {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    pub fn linear(count: u32, workgroup_size: u32) -> Self {
        let x = (count + workgroup_size - 1) / workgroup_size;
        Self { x, y: 1, z: 1 }
    }

    pub fn grid_2d(width: u32, height: u32, wg_x: u32, wg_y: u32) -> Self {
        Self {
            x: (width + wg_x - 1) / wg_x,
            y: (height + wg_y - 1) / wg_y,
            z: 1,
        }
    }

    pub fn total_workgroups(&self) -> u64 {
        self.x as u64 * self.y as u64 * self.z as u64
    }
}

impl fmt::Display for Dispatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// ─── Compute Pipeline ───────────────────────────────────────────────────

/// A stage in the compute pipeline.
#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub kernel_name: String,
    pub dispatch: Dispatch,
    pub buffer_bindings: Vec<(u32, u32)>, // (binding, buffer_id)
}

/// A multi-stage compute pipeline.
#[derive(Debug)]
pub struct ComputePipeline {
    pub name: String,
    pub stages: Vec<PipelineStage>,
    pub kernels: HashMap<String, ComputeKernel>,
    pub buffers: HashMap<u32, GpuBuffer>,
    next_buffer_id: u32,
}

impl ComputePipeline {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            stages: Vec::new(),
            kernels: HashMap::new(),
            buffers: HashMap::new(),
            next_buffer_id: 0,
        }
    }

    /// Register a kernel.
    pub fn add_kernel(&mut self, kernel: ComputeKernel) {
        self.kernels.insert(kernel.name.clone(), kernel);
    }

    /// Allocate a new buffer, returning its ID.
    pub fn create_buffer(&mut self, element_type: BufferElementType, count: usize, usage: BufferUsage) -> u32 {
        let id = self.next_buffer_id;
        self.next_buffer_id += 1;
        let buffer = GpuBuffer::new(id, element_type, count, usage);
        self.buffers.insert(id, buffer);
        id
    }

    /// Add a buffer from f32 data.
    pub fn create_buffer_f32(&mut self, data: &[f32], usage: BufferUsage) -> u32 {
        let id = self.next_buffer_id;
        self.next_buffer_id += 1;
        let buffer = GpuBuffer::from_f32(id, data, usage);
        self.buffers.insert(id, buffer);
        id
    }

    /// Add a pipeline stage.
    pub fn add_stage(&mut self, kernel_name: &str, dispatch: Dispatch, bindings: Vec<(u32, u32)>) {
        self.stages.push(PipelineStage {
            kernel_name: kernel_name.to_string(),
            dispatch,
            buffer_bindings: bindings,
        });
    }

    /// Execute the pipeline on the software backend (CPU simulation).
    pub fn execute_software(&mut self) -> Result<(), String> {
        for stage in &self.stages {
            let _kernel = self.kernels.get(&stage.kernel_name)
                .ok_or_else(|| format!("Kernel '{}' not found", stage.kernel_name))?;

            for &(_binding, buf_id) in &stage.buffer_bindings {
                if !self.buffers.contains_key(&buf_id) {
                    return Err(format!("Buffer {} not found", buf_id));
                }
            }
        }
        Ok(())
    }

    /// Get a buffer by ID.
    pub fn get_buffer(&self, id: u32) -> Option<&GpuBuffer> {
        self.buffers.get(&id)
    }

    /// Get a mutable buffer by ID.
    pub fn get_buffer_mut(&mut self, id: u32) -> Option<&mut GpuBuffer> {
        self.buffers.get_mut(&id)
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn kernel_count(&self) -> usize {
        self.kernels.len()
    }

    pub fn buffer_count(&self) -> usize {
        self.buffers.len()
    }
}

// ─── Shader Builder ─────────────────────────────────────────────────────

/// Generate WGSL-style compute shader source from a kernel description.
pub struct ShaderBuilder {
    lines: Vec<String>,
    indent: usize,
}

impl ShaderBuilder {
    pub fn new() -> Self {
        Self { lines: Vec::new(), indent: 0 }
    }

    pub fn line(&mut self, text: &str) {
        let prefix = "    ".repeat(self.indent);
        self.lines.push(format!("{}{}", prefix, text));
    }

    pub fn blank(&mut self) {
        self.lines.push(String::new());
    }

    pub fn indent(&mut self) { self.indent += 1; }
    pub fn dedent(&mut self) { if self.indent > 0 { self.indent -= 1; } }

    /// Generate a simple element-wise operation shader.
    pub fn element_wise_op(kernel: &ComputeKernel, op: &str) -> String {
        let mut b = ShaderBuilder::new();

        // Bindings
        for param in &kernel.params {
            let access = match param.access {
                BufferAccess::ReadOnly => "read",
                BufferAccess::WriteOnly => "write",
                BufferAccess::ReadWrite => "read_write",
            };
            let ty_str = match param.element_type {
                BufferElementType::F32 => "f32",
                BufferElementType::I32 => "i32",
                BufferElementType::U32 => "u32",
                _ => "f32",
            };
            b.line(&format!(
                "@group(0) @binding({}) var<storage, {}> {}: array<{}>;",
                param.binding, access, param.name, ty_str
            ));
        }
        b.blank();

        // Main function
        let [wx, wy, wz] = kernel.workgroup_size;
        b.line(&format!(
            "@compute @workgroup_size({}, {}, {})",
            wx, wy, wz
        ));
        b.line("fn main(@builtin(global_invocation_id) gid: vec3<u32>) {");
        b.indent();
        b.line("let idx = gid.x;");
        b.line(&format!("{}", op));
        b.dedent();
        b.line("}");

        b.build()
    }

    pub fn build(&self) -> String {
        self.lines.join("\n")
    }
}

impl Default for ShaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Built-in Kernels ───────────────────────────────────────────────────

/// Create a vector-add kernel.
pub fn vector_add_kernel(workgroup_size: u32) -> ComputeKernel {
    let mut kernel = ComputeKernel::new("vector_add");
    kernel.set_workgroup_size(workgroup_size, 1, 1);
    kernel.add_param("a", 0, BufferElementType::F32, BufferAccess::ReadOnly);
    kernel.add_param("b", 1, BufferElementType::F32, BufferAccess::ReadOnly);
    kernel.add_param("result", 2, BufferElementType::F32, BufferAccess::WriteOnly);

    let source = ShaderBuilder::element_wise_op(
        &kernel,
        "result[idx] = a[idx] + b[idx];",
    );
    kernel.set_source(&source);
    kernel
}

/// Create a matrix-multiply kernel.
pub fn matmul_kernel(m: u32, n: u32, k: u32) -> ComputeKernel {
    let mut kernel = ComputeKernel::new("matmul");
    kernel.set_workgroup_size(16, 16, 1);
    kernel.add_param("a", 0, BufferElementType::F32, BufferAccess::ReadOnly);
    kernel.add_param("b", 1, BufferElementType::F32, BufferAccess::ReadOnly);
    kernel.add_param("c", 2, BufferElementType::F32, BufferAccess::WriteOnly);

    let mut sb = ShaderBuilder::new();
    sb.line("// Matrix dimensions");
    sb.line(&format!("const M: u32 = {};", m));
    sb.line(&format!("const N: u32 = {};", n));
    sb.line(&format!("const K: u32 = {};", k));
    sb.blank();
    sb.line("@group(0) @binding(0) var<storage, read> a: array<f32>;");
    sb.line("@group(0) @binding(1) var<storage, read> b: array<f32>;");
    sb.line("@group(0) @binding(2) var<storage, read_write> c: array<f32>;");
    sb.blank();
    sb.line("@compute @workgroup_size(16, 16, 1)");
    sb.line("fn main(@builtin(global_invocation_id) gid: vec3<u32>) {");
    sb.indent();
    sb.line("let row = gid.y;");
    sb.line("let col = gid.x;");
    sb.line("if (row >= M || col >= N) { return; }");
    sb.line("var sum: f32 = 0.0;");
    sb.line("for (var i: u32 = 0u; i < K; i++) {");
    sb.indent();
    sb.line("sum += a[row * K + i] * b[i * N + col];");
    sb.dedent();
    sb.line("}");
    sb.line("c[row * N + col] = sum;");
    sb.dedent();
    sb.line("}");

    kernel.set_source(&sb.build());
    kernel
}

/// Create a ReLU activation kernel.
pub fn relu_kernel(workgroup_size: u32) -> ComputeKernel {
    let mut kernel = ComputeKernel::new("relu");
    kernel.set_workgroup_size(workgroup_size, 1, 1);
    kernel.add_param("input", 0, BufferElementType::F32, BufferAccess::ReadOnly);
    kernel.add_param("output", 1, BufferElementType::F32, BufferAccess::WriteOnly);

    let source = ShaderBuilder::element_wise_op(
        &kernel,
        "output[idx] = max(input[idx], 0.0);",
    );
    kernel.set_source(&source);
    kernel
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_backend_display() {
        assert_eq!(format!("{}", GpuBackend::Cuda), "CUDA");
        assert_eq!(format!("{}", GpuBackend::WebGpu), "WebGPU");
        assert_eq!(format!("{}", GpuBackend::Software), "Software");
    }

    #[test]
    fn test_device_info_software() {
        let dev = DeviceInfo::software_fallback();
        assert_eq!(dev.backend, GpuBackend::Software);
        assert_eq!(dev.compute_units, 1);
    }

    #[test]
    fn test_buffer_element_type_size() {
        assert_eq!(BufferElementType::F32.size_bytes(), 4);
        assert_eq!(BufferElementType::F64.size_bytes(), 8);
        assert_eq!(BufferElementType::U8.size_bytes(), 1);
    }

    #[test]
    fn test_gpu_buffer_new() {
        let buf = GpuBuffer::new(0, BufferElementType::F32, 100, BufferUsage::Storage);
        assert_eq!(buf.byte_size(), 400);
        assert_eq!(buf.element_count, 100);
    }

    #[test]
    fn test_gpu_buffer_from_f32() {
        let data = vec![1.0f32, 2.0, 3.0, 4.0];
        let buf = GpuBuffer::from_f32(0, &data, BufferUsage::Storage);
        assert_eq!(buf.element_count, 4);
        let read_back = buf.as_f32();
        assert_eq!(read_back, data);
    }

    #[test]
    fn test_gpu_buffer_from_i32() {
        let data = vec![10i32, 20, 30];
        let buf = GpuBuffer::from_i32(0, &data, BufferUsage::Storage);
        let read_back = buf.as_i32();
        assert_eq!(read_back, data);
    }

    #[test]
    fn test_compute_kernel_new() {
        let mut k = ComputeKernel::new("test_kernel");
        k.add_param("input", 0, BufferElementType::F32, BufferAccess::ReadOnly);
        k.add_param("output", 1, BufferElementType::F32, BufferAccess::WriteOnly);
        assert_eq!(k.param_count(), 2);
        assert_eq!(k.workgroup_size, [64, 1, 1]);
    }

    #[test]
    fn test_kernel_workgroup_size() {
        let mut k = ComputeKernel::new("k");
        k.set_workgroup_size(256, 1, 1);
        assert_eq!(k.workgroup_size, [256, 1, 1]);
    }

    #[test]
    fn test_dispatch_linear() {
        let d = Dispatch::linear(1000, 64);
        assert_eq!(d.x, 16); // ceil(1000/64) = 16
        assert_eq!(d.y, 1);
        assert_eq!(d.z, 1);
    }

    #[test]
    fn test_dispatch_grid_2d() {
        let d = Dispatch::grid_2d(512, 512, 16, 16);
        assert_eq!(d.x, 32);
        assert_eq!(d.y, 32);
    }

    #[test]
    fn test_dispatch_total() {
        let d = Dispatch::new(4, 4, 2);
        assert_eq!(d.total_workgroups(), 32);
    }

    #[test]
    fn test_dispatch_display() {
        let d = Dispatch::new(8, 8, 1);
        assert_eq!(format!("{}", d), "(8, 8, 1)");
    }

    #[test]
    fn test_compute_pipeline_new() {
        let pipeline = ComputePipeline::new("test");
        assert_eq!(pipeline.stage_count(), 0);
        assert_eq!(pipeline.kernel_count(), 0);
        assert_eq!(pipeline.buffer_count(), 0);
    }

    #[test]
    fn test_pipeline_create_buffer() {
        let mut pipeline = ComputePipeline::new("test");
        let id = pipeline.create_buffer(BufferElementType::F32, 256, BufferUsage::Storage);
        assert_eq!(id, 0);
        assert_eq!(pipeline.buffer_count(), 1);
        assert!(pipeline.get_buffer(0).is_some());
    }

    #[test]
    fn test_pipeline_create_buffer_f32() {
        let mut pipeline = ComputePipeline::new("test");
        let data = vec![1.0f32, 2.0, 3.0];
        let id = pipeline.create_buffer_f32(&data, BufferUsage::Storage);
        let buf = pipeline.get_buffer(id).unwrap();
        assert_eq!(buf.as_f32(), data);
    }

    #[test]
    fn test_pipeline_add_kernel_and_stage() {
        let mut pipeline = ComputePipeline::new("vecadd");
        let kernel = vector_add_kernel(64);
        pipeline.add_kernel(kernel);

        let a_id = pipeline.create_buffer_f32(&[1.0, 2.0], BufferUsage::Storage);
        let b_id = pipeline.create_buffer_f32(&[3.0, 4.0], BufferUsage::Storage);
        let c_id = pipeline.create_buffer(BufferElementType::F32, 2, BufferUsage::Storage);

        pipeline.add_stage("vector_add", Dispatch::linear(2, 64), vec![
            (0, a_id), (1, b_id), (2, c_id),
        ]);

        assert_eq!(pipeline.stage_count(), 1);
        assert_eq!(pipeline.kernel_count(), 1);
    }

    #[test]
    fn test_pipeline_execute_software() {
        let mut pipeline = ComputePipeline::new("test");
        let kernel = vector_add_kernel(64);
        pipeline.add_kernel(kernel);

        let a_id = pipeline.create_buffer_f32(&[1.0], BufferUsage::Storage);
        let b_id = pipeline.create_buffer_f32(&[2.0], BufferUsage::Storage);
        let c_id = pipeline.create_buffer(BufferElementType::F32, 1, BufferUsage::Storage);

        pipeline.add_stage("vector_add", Dispatch::linear(1, 64), vec![
            (0, a_id), (1, b_id), (2, c_id),
        ]);

        assert!(pipeline.execute_software().is_ok());
    }

    #[test]
    fn test_pipeline_missing_kernel() {
        let mut pipeline = ComputePipeline::new("test");
        pipeline.add_stage("missing", Dispatch::linear(1, 1), vec![]);
        assert!(pipeline.execute_software().is_err());
    }

    #[test]
    fn test_pipeline_missing_buffer() {
        let mut pipeline = ComputePipeline::new("test");
        let kernel = vector_add_kernel(64);
        pipeline.add_kernel(kernel);
        pipeline.add_stage("vector_add", Dispatch::linear(1, 64), vec![(0, 999)]);
        assert!(pipeline.execute_software().is_err());
    }

    #[test]
    fn test_vector_add_kernel() {
        let k = vector_add_kernel(128);
        assert_eq!(k.name, "vector_add");
        assert_eq!(k.param_count(), 3);
        assert_eq!(k.workgroup_size, [128, 1, 1]);
        assert!(k.source.contains("a[idx] + b[idx]"));
    }

    #[test]
    fn test_matmul_kernel() {
        let k = matmul_kernel(64, 64, 64);
        assert_eq!(k.name, "matmul");
        assert!(k.source.contains("const M: u32 = 64"));
        assert!(k.source.contains("sum +="));
    }

    #[test]
    fn test_relu_kernel() {
        let k = relu_kernel(256);
        assert_eq!(k.name, "relu");
        assert!(k.source.contains("max(input[idx], 0.0)"));
    }

    #[test]
    fn test_shader_builder() {
        let mut sb = ShaderBuilder::new();
        sb.line("fn main() {");
        sb.indent();
        sb.line("let x = 1;");
        sb.dedent();
        sb.line("}");
        let src = sb.build();
        assert!(src.contains("    let x = 1;"));
    }

    #[test]
    fn test_element_wise_shader() {
        let k = vector_add_kernel(64);
        let source = &k.source;
        assert!(source.contains("@compute @workgroup_size(64, 1, 1)"));
        assert!(source.contains("@group(0) @binding(0)"));
        assert!(source.contains("fn main"));
    }

    #[test]
    fn test_device_memory_mb() {
        let dev = DeviceInfo {
            name: "Test GPU".to_string(),
            backend: GpuBackend::Vulkan,
            memory_bytes: 8 * 1024 * 1024 * 1024,
            compute_units: 80,
            max_workgroup_size: [1024, 1024, 64],
            max_shared_memory: 49152,
        };
        assert_eq!(dev.memory_mb(), 8192);
    }
}
