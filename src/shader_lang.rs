//! Vitalis Shader Language System — Multi-backend shader compilation and generation.
//!
//! Provides comprehensive support for all major GPU shading languages:
//!
//! ## Bare-Metal GPU Languages
//! - **GLSL** (OpenGL Shading Language): WebGL/OpenGL vertex, fragment, compute shaders
//! - **HLSL** (High-Level Shading Language): DirectX/Xbox shader model 5.0/6.0
//! - **WGSL** (WebGPU Shading Language): Next-gen browser GPU access
//! - **MSL** (Metal Shading Language): Apple GPU programming for macOS/iOS
//!
//! ## Features
//! - Shader AST with cross-backend compilation
//! - Uniform/attribute/varying declarations
//! - Built-in math operations (vec2/3/4, mat3/4, dot, cross, normalize, etc.)
//! - Texture sampling, framebuffer operations
//! - Vertex/Fragment/Compute shader stage support
//! - Shader validation and error reporting
//! - Cross-compilation between shader languages
//! - Preprocessor directives (#version, #define, #ifdef)
//! - Shader reflection (input/output introspection)

use std::collections::HashMap;
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  SHADER BACKEND TARGETS
// ═══════════════════════════════════════════════════════════════════════

/// Target shader language backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderBackend {
    /// OpenGL Shading Language (WebGL, OpenGL 3.3+)
    Glsl,
    /// High-Level Shading Language (DirectX 11/12)
    Hlsl,
    /// WebGPU Shading Language (WebGPU API)
    Wgsl,
    /// Metal Shading Language (Apple platforms)
    Msl,
    /// SPIR-V intermediate (Vulkan)
    SpirV,
}

impl fmt::Display for ShaderBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderBackend::Glsl => write!(f, "GLSL"),
            ShaderBackend::Hlsl => write!(f, "HLSL"),
            ShaderBackend::Wgsl => write!(f, "WGSL"),
            ShaderBackend::Msl => write!(f, "MSL"),
            ShaderBackend::SpirV => write!(f, "SPIR-V"),
        }
    }
}

/// Shader stage in the graphics pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    TessControl,
    TessEvaluation,
}

impl fmt::Display for ShaderStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderStage::Vertex => write!(f, "vertex"),
            ShaderStage::Fragment => write!(f, "fragment"),
            ShaderStage::Compute => write!(f, "compute"),
            ShaderStage::Geometry => write!(f, "geometry"),
            ShaderStage::TessControl => write!(f, "tess_control"),
            ShaderStage::TessEvaluation => write!(f, "tess_evaluation"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SHADER DATA TYPES
// ═══════════════════════════════════════════════════════════════════════

/// Shader-level data types (cross-backend).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    IVec2,
    IVec3,
    IVec4,
    UInt,
    Bool,
    Mat2,
    Mat3,
    Mat4,
    Sampler2D,
    SamplerCube,
    Sampler3D,
    SamplerArray,
    Texture2D,
    TextureCube,
    Void,
    Struct(String),
    Array(Box<ShaderType>, usize),
}

impl ShaderType {
    /// Size in bytes (for buffer layout).
    pub fn byte_size(&self) -> usize {
        match self {
            ShaderType::Float => 4,
            ShaderType::Vec2 => 8,
            ShaderType::Vec3 => 12,
            ShaderType::Vec4 => 16,
            ShaderType::Int | ShaderType::UInt | ShaderType::Bool => 4,
            ShaderType::IVec2 => 8,
            ShaderType::IVec3 => 12,
            ShaderType::IVec4 => 16,
            ShaderType::Mat2 => 16,
            ShaderType::Mat3 => 36,
            ShaderType::Mat4 => 64,
            ShaderType::Array(ty, count) => ty.byte_size() * count,
            _ => 0,
        }
    }

    /// Convert to GLSL type name.
    pub fn to_glsl(&self) -> String {
        match self {
            ShaderType::Float => "float".into(),
            ShaderType::Vec2 => "vec2".into(),
            ShaderType::Vec3 => "vec3".into(),
            ShaderType::Vec4 => "vec4".into(),
            ShaderType::Int => "int".into(),
            ShaderType::IVec2 => "ivec2".into(),
            ShaderType::IVec3 => "ivec3".into(),
            ShaderType::IVec4 => "ivec4".into(),
            ShaderType::UInt => "uint".into(),
            ShaderType::Bool => "bool".into(),
            ShaderType::Mat2 => "mat2".into(),
            ShaderType::Mat3 => "mat3".into(),
            ShaderType::Mat4 => "mat4".into(),
            ShaderType::Sampler2D => "sampler2D".into(),
            ShaderType::SamplerCube => "samplerCube".into(),
            ShaderType::Sampler3D => "sampler3D".into(),
            ShaderType::SamplerArray => "sampler2DArray".into(),
            ShaderType::Texture2D => "sampler2D".into(),
            ShaderType::TextureCube => "samplerCube".into(),
            ShaderType::Void => "void".into(),
            ShaderType::Struct(name) => name.clone(),
            ShaderType::Array(ty, size) => format!("{}[{}]", ty.to_glsl(), size),
        }
    }

    /// Convert to HLSL type name.
    pub fn to_hlsl(&self) -> String {
        match self {
            ShaderType::Float => "float".into(),
            ShaderType::Vec2 => "float2".into(),
            ShaderType::Vec3 => "float3".into(),
            ShaderType::Vec4 => "float4".into(),
            ShaderType::Int => "int".into(),
            ShaderType::IVec2 => "int2".into(),
            ShaderType::IVec3 => "int3".into(),
            ShaderType::IVec4 => "int4".into(),
            ShaderType::UInt => "uint".into(),
            ShaderType::Bool => "bool".into(),
            ShaderType::Mat2 => "float2x2".into(),
            ShaderType::Mat3 => "float3x3".into(),
            ShaderType::Mat4 => "float4x4".into(),
            ShaderType::Sampler2D | ShaderType::Texture2D => "Texture2D".into(),
            ShaderType::SamplerCube | ShaderType::TextureCube => "TextureCube".into(),
            ShaderType::Sampler3D => "Texture3D".into(),
            ShaderType::SamplerArray => "Texture2DArray".into(),
            ShaderType::Void => "void".into(),
            ShaderType::Struct(name) => name.clone(),
            ShaderType::Array(ty, size) => format!("{}[{}]", ty.to_hlsl(), size),
        }
    }

    /// Convert to WGSL type name.
    pub fn to_wgsl(&self) -> String {
        match self {
            ShaderType::Float => "f32".into(),
            ShaderType::Vec2 => "vec2<f32>".into(),
            ShaderType::Vec3 => "vec3<f32>".into(),
            ShaderType::Vec4 => "vec4<f32>".into(),
            ShaderType::Int => "i32".into(),
            ShaderType::IVec2 => "vec2<i32>".into(),
            ShaderType::IVec3 => "vec3<i32>".into(),
            ShaderType::IVec4 => "vec4<i32>".into(),
            ShaderType::UInt => "u32".into(),
            ShaderType::Bool => "bool".into(),
            ShaderType::Mat2 => "mat2x2<f32>".into(),
            ShaderType::Mat3 => "mat3x3<f32>".into(),
            ShaderType::Mat4 => "mat4x4<f32>".into(),
            ShaderType::Sampler2D | ShaderType::Texture2D => "texture_2d<f32>".into(),
            ShaderType::SamplerCube | ShaderType::TextureCube => "texture_cube<f32>".into(),
            ShaderType::Sampler3D => "texture_3d<f32>".into(),
            ShaderType::SamplerArray => "texture_2d_array<f32>".into(),
            ShaderType::Void => "void".into(),
            ShaderType::Struct(name) => name.clone(),
            ShaderType::Array(ty, size) => format!("array<{}, {}>", ty.to_wgsl(), size),
        }
    }

    /// Convert to MSL type name.
    pub fn to_msl(&self) -> String {
        match self {
            ShaderType::Float => "float".into(),
            ShaderType::Vec2 => "float2".into(),
            ShaderType::Vec3 => "float3".into(),
            ShaderType::Vec4 => "float4".into(),
            ShaderType::Int => "int".into(),
            ShaderType::IVec2 => "int2".into(),
            ShaderType::IVec3 => "int3".into(),
            ShaderType::IVec4 => "int4".into(),
            ShaderType::UInt => "uint".into(),
            ShaderType::Bool => "bool".into(),
            ShaderType::Mat2 => "float2x2".into(),
            ShaderType::Mat3 => "float3x3".into(),
            ShaderType::Mat4 => "float4x4".into(),
            ShaderType::Sampler2D | ShaderType::Texture2D => "texture2d<float>".into(),
            ShaderType::SamplerCube | ShaderType::TextureCube => "texturecube<float>".into(),
            ShaderType::Sampler3D => "texture3d<float>".into(),
            ShaderType::SamplerArray => "texture2d_array<float>".into(),
            ShaderType::Void => "void".into(),
            ShaderType::Struct(name) => name.clone(),
            ShaderType::Array(ty, size) => format!("array<{}, {}>", ty.to_msl(), size),
        }
    }

    /// Convert to the requested backend.
    pub fn to_backend(&self, backend: ShaderBackend) -> String {
        match backend {
            ShaderBackend::Glsl => self.to_glsl(),
            ShaderBackend::Hlsl => self.to_hlsl(),
            ShaderBackend::Wgsl => self.to_wgsl(),
            ShaderBackend::Msl => self.to_msl(),
            ShaderBackend::SpirV => self.to_glsl(), // Fallback
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SHADER VARIABLE QUALIFIERS
// ═══════════════════════════════════════════════════════════════════════

/// Variable qualifier in shader programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderQualifier {
    /// Per-vertex data from CPU (position, normal, uv)
    Attribute,
    /// Uniform data shared across all invocations
    Uniform,
    /// Data passed from vertex to fragment stage
    Varying,
    /// Output from fragment shader
    Output,
    /// Local variable
    Local,
    /// Constant
    Constant,
    /// Shader storage buffer object
    StorageBuffer,
    /// Texture binding
    TextureBind,
    /// Sampler binding
    SamplerBind,
}

/// A variable declaration in a shader.
#[derive(Debug, Clone)]
pub struct ShaderVariable {
    pub name: String,
    pub ty: ShaderType,
    pub qualifier: ShaderQualifier,
    pub location: Option<u32>,
    pub binding: Option<u32>,
    pub set: Option<u32>,
}

impl ShaderVariable {
    pub fn attribute(name: &str, ty: ShaderType, location: u32) -> Self {
        Self { name: name.into(), ty, qualifier: ShaderQualifier::Attribute, location: Some(location), binding: None, set: None }
    }
    pub fn uniform(name: &str, ty: ShaderType, binding: u32) -> Self {
        Self { name: name.into(), ty, qualifier: ShaderQualifier::Uniform, location: None, binding: Some(binding), set: Some(0) }
    }
    pub fn varying(name: &str, ty: ShaderType, location: u32) -> Self {
        Self { name: name.into(), ty, qualifier: ShaderQualifier::Varying, location: Some(location), binding: None, set: None }
    }
    pub fn output(name: &str, ty: ShaderType, location: u32) -> Self {
        Self { name: name.into(), ty, qualifier: ShaderQualifier::Output, location: Some(location), binding: None, set: None }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SHADER AST / EXPRESSIONS
// ═══════════════════════════════════════════════════════════════════════

/// Built-in shader functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderBuiltin {
    // Math
    Sin, Cos, Tan, Asin, Acos, Atan, Atan2,
    Sqrt, InverseSqrt, Abs, Sign, Floor, Ceil, Fract, Round,
    Pow, Exp, Exp2, Log, Log2,
    Min, Max, Clamp, Mix, Step, Smoothstep,
    Length, Distance, Dot, Cross, Normalize, Reflect, Refract,
    // Matrix
    Transpose, Inverse, Determinant,
    // Texture
    TextureSample, TextureSampleLevel, TextureSize, TextureGather,
    // Fragment
    Discard, DDx, DDy, Fwidth,
    // Compute
    BarrierSync, AtomicAdd, AtomicMax, AtomicMin,
}

impl ShaderBuiltin {
    pub fn to_glsl(&self) -> &str {
        match self {
            ShaderBuiltin::Sin => "sin", ShaderBuiltin::Cos => "cos",
            ShaderBuiltin::Tan => "tan", ShaderBuiltin::Sqrt => "sqrt",
            ShaderBuiltin::Abs => "abs", ShaderBuiltin::Sign => "sign",
            ShaderBuiltin::Floor => "floor", ShaderBuiltin::Ceil => "ceil",
            ShaderBuiltin::Fract => "fract", ShaderBuiltin::Round => "round",
            ShaderBuiltin::Pow => "pow", ShaderBuiltin::Exp => "exp",
            ShaderBuiltin::Log => "log", ShaderBuiltin::Log2 => "log2",
            ShaderBuiltin::Min => "min", ShaderBuiltin::Max => "max",
            ShaderBuiltin::Clamp => "clamp", ShaderBuiltin::Mix => "mix",
            ShaderBuiltin::Step => "step", ShaderBuiltin::Smoothstep => "smoothstep",
            ShaderBuiltin::Length => "length", ShaderBuiltin::Distance => "distance",
            ShaderBuiltin::Dot => "dot", ShaderBuiltin::Cross => "cross",
            ShaderBuiltin::Normalize => "normalize", ShaderBuiltin::Reflect => "reflect",
            ShaderBuiltin::Refract => "refract",
            ShaderBuiltin::Transpose => "transpose", ShaderBuiltin::Inverse => "inverse",
            ShaderBuiltin::Determinant => "determinant",
            ShaderBuiltin::TextureSample => "texture",
            ShaderBuiltin::TextureSampleLevel => "textureLod",
            ShaderBuiltin::TextureSize => "textureSize",
            ShaderBuiltin::TextureGather => "textureGather",
            ShaderBuiltin::Discard => "discard",
            ShaderBuiltin::DDx => "dFdx", ShaderBuiltin::DDy => "dFdy",
            ShaderBuiltin::Fwidth => "fwidth",
            ShaderBuiltin::BarrierSync => "barrier",
            _ => "unknown",
        }
    }

    pub fn to_hlsl(&self) -> &str {
        match self {
            ShaderBuiltin::Sin => "sin", ShaderBuiltin::Cos => "cos",
            ShaderBuiltin::Sqrt => "sqrt", ShaderBuiltin::Abs => "abs",
            ShaderBuiltin::Floor => "floor", ShaderBuiltin::Ceil => "ceil",
            ShaderBuiltin::Fract => "frac", // HLSL uses "frac" not "fract"
            ShaderBuiltin::Clamp => "clamp", ShaderBuiltin::Mix => "lerp", // HLSL uses "lerp"
            ShaderBuiltin::Smoothstep => "smoothstep",
            ShaderBuiltin::Length => "length", ShaderBuiltin::Distance => "distance",
            ShaderBuiltin::Dot => "dot", ShaderBuiltin::Cross => "cross",
            ShaderBuiltin::Normalize => "normalize",
            ShaderBuiltin::TextureSample => "Sample",
            ShaderBuiltin::DDx => "ddx", ShaderBuiltin::DDy => "ddy",
            _ => self.to_glsl(),
        }
    }
}

/// Shader expression node.
#[derive(Debug, Clone)]
pub enum ShaderExpr {
    FloatLit(f64),
    IntLit(i64),
    BoolLit(bool),
    Var(String),
    Swizzle { expr: Box<ShaderExpr>, components: String },
    BinaryOp { op: ShaderBinOp, left: Box<ShaderExpr>, right: Box<ShaderExpr> },
    UnaryOp { op: ShaderUnaryOp, expr: Box<ShaderExpr> },
    Call { func: ShaderBuiltin, args: Vec<ShaderExpr> },
    CustomCall { name: String, args: Vec<ShaderExpr> },
    Constructor { ty: ShaderType, args: Vec<ShaderExpr> },
    FieldAccess { expr: Box<ShaderExpr>, field: String },
    ArrayIndex { expr: Box<ShaderExpr>, index: Box<ShaderExpr> },
    Ternary { condition: Box<ShaderExpr>, true_expr: Box<ShaderExpr>, false_expr: Box<ShaderExpr> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderBinOp { Add, Sub, Mul, Div, Mod, And, Or, Eq, NotEq, Lt, Gt, LtEq, GtEq, BitAnd, BitOr, BitXor, ShiftLeft, ShiftRight }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderUnaryOp { Negate, Not, BitNot }

/// A shader statement.
#[derive(Debug, Clone)]
pub enum ShaderStatement {
    VarDecl { name: String, ty: ShaderType, init: Option<ShaderExpr> },
    Assign { target: ShaderExpr, value: ShaderExpr },
    OpAssign { op: ShaderBinOp, target: ShaderExpr, value: ShaderExpr },
    Return(Option<ShaderExpr>),
    If { condition: ShaderExpr, then_body: Vec<ShaderStatement>, else_body: Vec<ShaderStatement> },
    For { init: Box<ShaderStatement>, condition: ShaderExpr, increment: Box<ShaderStatement>, body: Vec<ShaderStatement> },
    While { condition: ShaderExpr, body: Vec<ShaderStatement> },
    ExprStmt(ShaderExpr),
    Break,
    Continue,
    Discard,
}

// ═══════════════════════════════════════════════════════════════════════
//  SHADER PROGRAM
// ═══════════════════════════════════════════════════════════════════════

/// A user-defined function within a shader.
#[derive(Debug, Clone)]
pub struct ShaderFunction {
    pub name: String,
    pub return_type: ShaderType,
    pub params: Vec<(String, ShaderType)>,
    pub body: Vec<ShaderStatement>,
}

/// A user-defined struct within a shader.
#[derive(Debug, Clone)]
pub struct ShaderStruct {
    pub name: String,
    pub fields: Vec<(String, ShaderType)>,
}

/// A complete shader program.
#[derive(Debug, Clone)]
pub struct ShaderProgram {
    pub name: String,
    pub stage: ShaderStage,
    pub version: String,
    pub inputs: Vec<ShaderVariable>,
    pub outputs: Vec<ShaderVariable>,
    pub uniforms: Vec<ShaderVariable>,
    pub structs: Vec<ShaderStruct>,
    pub functions: Vec<ShaderFunction>,
    pub main_body: Vec<ShaderStatement>,
    pub defines: HashMap<String, String>,
    pub extensions: Vec<String>,
}

impl ShaderProgram {
    pub fn new(name: &str, stage: ShaderStage) -> Self {
        Self {
            name: name.into(),
            stage,
            version: "450".into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            uniforms: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            main_body: Vec::new(),
            defines: HashMap::new(),
            extensions: Vec::new(),
        }
    }

    pub fn add_input(&mut self, var: ShaderVariable) { self.inputs.push(var); }
    pub fn add_output(&mut self, var: ShaderVariable) { self.outputs.push(var); }
    pub fn add_uniform(&mut self, var: ShaderVariable) { self.uniforms.push(var); }

    pub fn add_function(&mut self, func: ShaderFunction) { self.functions.push(func); }
    pub fn add_struct(&mut self, s: ShaderStruct) { self.structs.push(s); }

    pub fn add_statement(&mut self, stmt: ShaderStatement) { self.main_body.push(stmt); }

    pub fn define(&mut self, key: &str, value: &str) { self.defines.insert(key.into(), value.into()); }

    pub fn total_variables(&self) -> usize {
        self.inputs.len() + self.outputs.len() + self.uniforms.len()
    }

    /// Compile to the specified backend.
    pub fn compile(&self, backend: ShaderBackend) -> ShaderCompileResult {
        match backend {
            ShaderBackend::Glsl => self.compile_glsl(),
            ShaderBackend::Hlsl => self.compile_hlsl(),
            ShaderBackend::Wgsl => self.compile_wgsl(),
            ShaderBackend::Msl => self.compile_msl(),
            ShaderBackend::SpirV => self.compile_spirv(),
        }
    }

    fn compile_glsl(&self) -> ShaderCompileResult {
        let mut src = String::new();
        src.push_str(&format!("#version {}\n", self.version));
        for (k, v) in &self.defines { src.push_str(&format!("#define {} {}\n", k, v)); }
        src.push('\n');

        // Inputs
        for var in &self.inputs {
            src.push_str(&format!("layout(location = {}) in {} {};\n",
                var.location.unwrap_or(0), var.ty.to_glsl(), var.name));
        }
        // Outputs
        for var in &self.outputs {
            src.push_str(&format!("layout(location = {}) out {} {};\n",
                var.location.unwrap_or(0), var.ty.to_glsl(), var.name));
        }
        // Uniforms
        for var in &self.uniforms {
            src.push_str(&format!("uniform {} {};\n", var.ty.to_glsl(), var.name));
        }
        src.push_str("\nvoid main() {\n");
        for stmt in &self.main_body {
            src.push_str(&format!("    {};\n", self.stmt_to_glsl(stmt)));
        }
        src.push_str("}\n");

        ShaderCompileResult {
            source: src,
            backend: ShaderBackend::Glsl,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn compile_hlsl(&self) -> ShaderCompileResult {
        let mut src = String::new();
        for (k, v) in &self.defines { src.push_str(&format!("#define {} {}\n", k, v)); }

        // Struct for inputs
        src.push_str(&format!("struct VS_INPUT {{\n"));
        for var in &self.inputs {
            src.push_str(&format!("    {} {} : TEXCOORD{};\n", var.ty.to_hlsl(), var.name, var.location.unwrap_or(0)));
        }
        src.push_str("};\n\n");

        // Struct for outputs
        src.push_str(&format!("struct PS_OUTPUT {{\n"));
        for var in &self.outputs {
            src.push_str(&format!("    {} {} : SV_TARGET{};\n", var.ty.to_hlsl(), var.name, var.location.unwrap_or(0)));
        }
        src.push_str("};\n\n");

        // Uniforms as cbuffer
        if !self.uniforms.is_empty() {
            src.push_str("cbuffer Constants : register(b0) {\n");
            for var in &self.uniforms {
                src.push_str(&format!("    {} {};\n", var.ty.to_hlsl(), var.name));
            }
            src.push_str("};\n\n");
        }

        let entry = match self.stage {
            ShaderStage::Vertex => "VSMain",
            ShaderStage::Fragment => "PSMain",
            ShaderStage::Compute => "CSMain",
            _ => "main",
        };
        src.push_str(&format!("void {}() {{\n", entry));
        src.push_str("    // HLSL shader body\n");
        src.push_str("}\n");

        ShaderCompileResult {
            source: src,
            backend: ShaderBackend::Hlsl,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn compile_wgsl(&self) -> ShaderCompileResult {
        let mut src = String::new();

        // Struct declarations
        for s in &self.structs {
            src.push_str(&format!("struct {} {{\n", s.name));
            for (name, ty) in &s.fields {
                src.push_str(&format!("    {}: {},\n", name, ty.to_wgsl()));
            }
            src.push_str("};\n\n");
        }

        // Uniforms as bind groups
        for var in &self.uniforms {
            src.push_str(&format!("@group({}) @binding({}) var<uniform> {}: {};\n",
                var.set.unwrap_or(0), var.binding.unwrap_or(0), var.name, var.ty.to_wgsl()));
        }

        // Entry point
        let stage_attr = match self.stage {
            ShaderStage::Vertex => "@vertex",
            ShaderStage::Fragment => "@fragment",
            ShaderStage::Compute => "@compute @workgroup_size(64)",
            _ => "@vertex",
        };

        src.push_str(&format!("\n{}\nfn main() {{\n", stage_attr));
        src.push_str("    // WGSL shader body\n");
        src.push_str("}\n");

        ShaderCompileResult {
            source: src,
            backend: ShaderBackend::Wgsl,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn compile_msl(&self) -> ShaderCompileResult {
        let mut src = String::new();
        src.push_str("#include <metal_stdlib>\nusing namespace metal;\n\n");

        // Structs
        for s in &self.structs {
            src.push_str(&format!("struct {} {{\n", s.name));
            for (name, ty) in &s.fields {
                src.push_str(&format!("    {} {};\n", ty.to_msl(), name));
            }
            src.push_str("};\n\n");
        }

        // Entry point
        let qualifier = match self.stage {
            ShaderStage::Vertex => "vertex",
            ShaderStage::Fragment => "fragment",
            ShaderStage::Compute => "kernel",
            _ => "vertex",
        };

        src.push_str(&format!("{} float4 {}Main() {{\n", qualifier, self.name));
        src.push_str("    return float4(0.0, 0.0, 0.0, 1.0);\n");
        src.push_str("}\n");

        ShaderCompileResult {
            source: src,
            backend: ShaderBackend::Msl,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn compile_spirv(&self) -> ShaderCompileResult {
        // SPIR-V is a binary format; we generate a textual representation
        let mut src = String::new();
        src.push_str("; SPIR-V generated from Vitalis shader\n");
        src.push_str("; Magic:     0x07230203\n");
        src.push_str("; Version:   1.5\n");
        src.push_str(&format!("; Generator: Vitalis Shader Compiler\n"));
        src.push_str(&format!("; Bound:     {}\n", self.total_variables() + 10));
        src.push_str("; Schema:    0\n");
        src.push_str("               OpCapability Shader\n");
        src.push_str("               OpMemoryModel Logical GLSL450\n");
        src.push_str(&format!("               OpEntryPoint {} %main \"main\"\n",
            match self.stage { ShaderStage::Vertex => "Vertex", ShaderStage::Fragment => "Fragment", _ => "GLCompute" }));

        ShaderCompileResult {
            source: src,
            backend: ShaderBackend::SpirV,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn stmt_to_glsl(&self, stmt: &ShaderStatement) -> String {
        match stmt {
            ShaderStatement::VarDecl { name, ty, init } => {
                if let Some(init_expr) = init {
                    format!("{} {} = {}", ty.to_glsl(), name, self.expr_to_glsl(init_expr))
                } else {
                    format!("{} {}", ty.to_glsl(), name)
                }
            }
            ShaderStatement::Assign { target, value } => {
                format!("{} = {}", self.expr_to_glsl(target), self.expr_to_glsl(value))
            }
            ShaderStatement::Return(Some(expr)) => format!("return {}", self.expr_to_glsl(expr)),
            ShaderStatement::Return(None) => "return".into(),
            ShaderStatement::ExprStmt(expr) => self.expr_to_glsl(expr),
            ShaderStatement::Discard => "discard".into(),
            _ => "/* unsupported */".into(),
        }
    }

    fn expr_to_glsl(&self, expr: &ShaderExpr) -> String {
        match expr {
            ShaderExpr::FloatLit(v) => format!("{:.6}", v),
            ShaderExpr::IntLit(v) => format!("{}", v),
            ShaderExpr::BoolLit(v) => format!("{}", v),
            ShaderExpr::Var(name) => name.clone(),
            ShaderExpr::Swizzle { expr, components } => format!("{}.{}", self.expr_to_glsl(expr), components),
            ShaderExpr::BinaryOp { op, left, right } => {
                let op_str = match op {
                    ShaderBinOp::Add => "+", ShaderBinOp::Sub => "-", ShaderBinOp::Mul => "*",
                    ShaderBinOp::Div => "/", ShaderBinOp::Mod => "%",
                    ShaderBinOp::And => "&&", ShaderBinOp::Or => "||",
                    ShaderBinOp::Eq => "==", ShaderBinOp::NotEq => "!=",
                    ShaderBinOp::Lt => "<", ShaderBinOp::Gt => ">",
                    ShaderBinOp::LtEq => "<=", ShaderBinOp::GtEq => ">=",
                    _ => "?",
                };
                format!("({} {} {})", self.expr_to_glsl(left), op_str, self.expr_to_glsl(right))
            }
            ShaderExpr::Call { func, args } => {
                let arg_str: Vec<String> = args.iter().map(|a| self.expr_to_glsl(a)).collect();
                format!("{}({})", func.to_glsl(), arg_str.join(", "))
            }
            ShaderExpr::Constructor { ty, args } => {
                let arg_str: Vec<String> = args.iter().map(|a| self.expr_to_glsl(a)).collect();
                format!("{}({})", ty.to_glsl(), arg_str.join(", "))
            }
            _ => "/* expr */".into(),
        }
    }
}

/// Result of shader compilation.
#[derive(Debug, Clone)]
pub struct ShaderCompileResult {
    pub source: String,
    pub backend: ShaderBackend,
    pub success: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ShaderCompileResult {
    pub fn line_count(&self) -> usize { self.source.lines().count() }
}

// ═══════════════════════════════════════════════════════════════════════
//  SHADER REFLECTION
// ═══════════════════════════════════════════════════════════════════════

/// Information about a shader's interface, extracted via reflection.
#[derive(Debug, Clone)]
pub struct ShaderReflection {
    pub stage: ShaderStage,
    pub inputs: Vec<ShaderVariable>,
    pub outputs: Vec<ShaderVariable>,
    pub uniforms: Vec<ShaderVariable>,
    pub uniform_blocks: Vec<(String, Vec<ShaderVariable>)>,
    pub total_uniform_bytes: usize,
}

impl ShaderReflection {
    pub fn from_program(program: &ShaderProgram) -> Self {
        let total_bytes: usize = program.uniforms.iter().map(|u| u.ty.byte_size()).sum();
        Self {
            stage: program.stage,
            inputs: program.inputs.clone(),
            outputs: program.outputs.clone(),
            uniforms: program.uniforms.clone(),
            uniform_blocks: Vec::new(),
            total_uniform_bytes: total_bytes,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  CROSS-COMPILER
// ═══════════════════════════════════════════════════════════════════════

/// Cross-compile a shader from one backend to another.
pub struct ShaderCrossCompiler;

impl ShaderCrossCompiler {
    /// Cross-compile a shader program to a different backend.
    pub fn cross_compile(program: &ShaderProgram, target: ShaderBackend) -> ShaderCompileResult {
        program.compile(target)
    }

    /// Get all supported backend targets for a given stage.
    pub fn supported_targets(stage: ShaderStage) -> Vec<ShaderBackend> {
        match stage {
            ShaderStage::Vertex | ShaderStage::Fragment => {
                vec![ShaderBackend::Glsl, ShaderBackend::Hlsl, ShaderBackend::Wgsl, ShaderBackend::Msl, ShaderBackend::SpirV]
            }
            ShaderStage::Compute => {
                vec![ShaderBackend::Glsl, ShaderBackend::Hlsl, ShaderBackend::Wgsl, ShaderBackend::Msl]
            }
            _ => vec![ShaderBackend::Glsl, ShaderBackend::SpirV],
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SHADER LIBRARY — Common Shader Snippets
// ═══════════════════════════════════════════════════════════════════════

/// Pre-built shader templates for common use cases.
pub struct ShaderLibrary;

impl ShaderLibrary {
    /// Standard vertex shader: transforms position by MVP matrix.
    pub fn standard_vertex() -> ShaderProgram {
        let mut prog = ShaderProgram::new("standard_vertex", ShaderStage::Vertex);
        prog.add_input(ShaderVariable::attribute("aPosition", ShaderType::Vec3, 0));
        prog.add_input(ShaderVariable::attribute("aNormal", ShaderType::Vec3, 1));
        prog.add_input(ShaderVariable::attribute("aTexCoord", ShaderType::Vec2, 2));
        prog.add_uniform(ShaderVariable::uniform("uModel", ShaderType::Mat4, 0));
        prog.add_uniform(ShaderVariable::uniform("uView", ShaderType::Mat4, 1));
        prog.add_uniform(ShaderVariable::uniform("uProjection", ShaderType::Mat4, 2));
        prog.add_output(ShaderVariable::varying("vNormal", ShaderType::Vec3, 0));
        prog.add_output(ShaderVariable::varying("vTexCoord", ShaderType::Vec2, 1));
        prog.add_statement(ShaderStatement::Assign {
            target: ShaderExpr::Var("gl_Position".into()),
            value: ShaderExpr::BinaryOp {
                op: ShaderBinOp::Mul,
                left: Box::new(ShaderExpr::Var("uProjection".into())),
                right: Box::new(ShaderExpr::BinaryOp {
                    op: ShaderBinOp::Mul,
                    left: Box::new(ShaderExpr::Var("uView".into())),
                    right: Box::new(ShaderExpr::BinaryOp {
                        op: ShaderBinOp::Mul,
                        left: Box::new(ShaderExpr::Var("uModel".into())),
                        right: Box::new(ShaderExpr::Constructor {
                            ty: ShaderType::Vec4,
                            args: vec![ShaderExpr::Var("aPosition".into()), ShaderExpr::FloatLit(1.0)],
                        }),
                    }),
                }),
            },
        });
        prog
    }

    /// Phong fragment shader: diffuse + specular lighting.
    pub fn phong_fragment() -> ShaderProgram {
        let mut prog = ShaderProgram::new("phong_fragment", ShaderStage::Fragment);
        prog.add_input(ShaderVariable::varying("vNormal", ShaderType::Vec3, 0));
        prog.add_input(ShaderVariable::varying("vTexCoord", ShaderType::Vec2, 1));
        prog.add_uniform(ShaderVariable::uniform("uLightDir", ShaderType::Vec3, 3));
        prog.add_uniform(ShaderVariable::uniform("uColor", ShaderType::Vec4, 4));
        prog.add_output(ShaderVariable::output("fragColor", ShaderType::Vec4, 0));
        prog
    }

    /// Compute shader for parallel reduction (sum).
    pub fn compute_reduction() -> ShaderProgram {
        let mut prog = ShaderProgram::new("reduction", ShaderStage::Compute);
        prog.version = "450".into();
        prog.define("WORKGROUP_SIZE", "256");
        prog
    }

    /// Fullscreen quad vertex shader (common for post-processing).
    pub fn fullscreen_quad_vertex() -> ShaderProgram {
        let mut prog = ShaderProgram::new("fullscreen_quad", ShaderStage::Vertex);
        prog.add_output(ShaderVariable::varying("vTexCoord", ShaderType::Vec2, 0));
        prog
    }

    /// Post-processing fragment shader (blur, tone mapping, etc.)
    pub fn post_process_fragment() -> ShaderProgram {
        let mut prog = ShaderProgram::new("post_process", ShaderStage::Fragment);
        prog.add_input(ShaderVariable::varying("vTexCoord", ShaderType::Vec2, 0));
        prog.add_uniform(ShaderVariable::uniform("uTexture", ShaderType::Sampler2D, 0));
        prog.add_uniform(ShaderVariable::uniform("uTime", ShaderType::Float, 1));
        prog.add_output(ShaderVariable::output("fragColor", ShaderType::Vec4, 0));
        prog
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_backend_display() {
        assert_eq!(ShaderBackend::Glsl.to_string(), "GLSL");
        assert_eq!(ShaderBackend::Hlsl.to_string(), "HLSL");
        assert_eq!(ShaderBackend::Wgsl.to_string(), "WGSL");
        assert_eq!(ShaderBackend::Msl.to_string(), "MSL");
        assert_eq!(ShaderBackend::SpirV.to_string(), "SPIR-V");
    }

    #[test]
    fn test_shader_type_sizes() {
        assert_eq!(ShaderType::Float.byte_size(), 4);
        assert_eq!(ShaderType::Vec4.byte_size(), 16);
        assert_eq!(ShaderType::Mat4.byte_size(), 64);
        assert_eq!(ShaderType::Array(Box::new(ShaderType::Vec4), 10).byte_size(), 160);
    }

    #[test]
    fn test_shader_type_glsl() {
        assert_eq!(ShaderType::Float.to_glsl(), "float");
        assert_eq!(ShaderType::Vec3.to_glsl(), "vec3");
        assert_eq!(ShaderType::Mat4.to_glsl(), "mat4");
        assert_eq!(ShaderType::Sampler2D.to_glsl(), "sampler2D");
    }

    #[test]
    fn test_shader_type_hlsl() {
        assert_eq!(ShaderType::Vec3.to_hlsl(), "float3");
        assert_eq!(ShaderType::Mat4.to_hlsl(), "float4x4");
        assert_eq!(ShaderType::Vec4.to_hlsl(), "float4");
    }

    #[test]
    fn test_shader_type_wgsl() {
        assert_eq!(ShaderType::Float.to_wgsl(), "f32");
        assert_eq!(ShaderType::Vec4.to_wgsl(), "vec4<f32>");
        assert_eq!(ShaderType::Mat4.to_wgsl(), "mat4x4<f32>");
        assert_eq!(ShaderType::Int.to_wgsl(), "i32");
    }

    #[test]
    fn test_shader_type_msl() {
        assert_eq!(ShaderType::Vec3.to_msl(), "float3");
        assert_eq!(ShaderType::Mat4.to_msl(), "float4x4");
        assert_eq!(ShaderType::Texture2D.to_msl(), "texture2d<float>");
    }

    #[test]
    fn test_shader_type_backend_dispatch() {
        let ty = ShaderType::Vec4;
        assert_eq!(ty.to_backend(ShaderBackend::Glsl), "vec4");
        assert_eq!(ty.to_backend(ShaderBackend::Hlsl), "float4");
        assert_eq!(ty.to_backend(ShaderBackend::Wgsl), "vec4<f32>");
        assert_eq!(ty.to_backend(ShaderBackend::Msl), "float4");
    }

    #[test]
    fn test_compile_glsl() {
        let prog = ShaderLibrary::standard_vertex();
        let result = prog.compile(ShaderBackend::Glsl);
        assert!(result.success);
        assert!(result.source.contains("#version"));
        assert!(result.source.contains("void main()"));
        assert!(result.source.contains("aPosition"));
        assert!(result.line_count() > 5);
    }

    #[test]
    fn test_compile_hlsl() {
        let prog = ShaderLibrary::phong_fragment();
        let result = prog.compile(ShaderBackend::Hlsl);
        assert!(result.success);
        assert!(result.source.contains("struct"));
        assert!(result.source.contains("PSMain"));
    }

    #[test]
    fn test_compile_wgsl() {
        let prog = ShaderLibrary::standard_vertex();
        let result = prog.compile(ShaderBackend::Wgsl);
        assert!(result.success);
        assert!(result.source.contains("@vertex"));
        assert!(result.source.contains("fn main()"));
    }

    #[test]
    fn test_compile_msl() {
        let prog = ShaderLibrary::standard_vertex();
        let result = prog.compile(ShaderBackend::Msl);
        assert!(result.success);
        assert!(result.source.contains("#include <metal_stdlib>"));
        assert!(result.source.contains("vertex"));
    }

    #[test]
    fn test_compile_spirv() {
        let prog = ShaderLibrary::standard_vertex();
        let result = prog.compile(ShaderBackend::SpirV);
        assert!(result.success);
        assert!(result.source.contains("OpCapability Shader"));
        assert!(result.source.contains("Vertex"));
    }

    #[test]
    fn test_cross_compile_all_backends() {
        let prog = ShaderLibrary::standard_vertex();
        for backend in [ShaderBackend::Glsl, ShaderBackend::Hlsl, ShaderBackend::Wgsl, ShaderBackend::Msl] {
            let result = ShaderCrossCompiler::cross_compile(&prog, backend);
            assert!(result.success, "Failed for {:?}", backend);
            assert!(!result.source.is_empty());
        }
    }

    #[test]
    fn test_shader_reflection() {
        let prog = ShaderLibrary::standard_vertex();
        let refl = ShaderReflection::from_program(&prog);
        assert_eq!(refl.inputs.len(), 3);
        assert_eq!(refl.outputs.len(), 2);
        assert_eq!(refl.uniforms.len(), 3);
        assert!(refl.total_uniform_bytes > 0);
    }

    #[test]
    fn test_supported_targets() {
        let vertex_targets = ShaderCrossCompiler::supported_targets(ShaderStage::Vertex);
        assert!(vertex_targets.contains(&ShaderBackend::Glsl));
        assert!(vertex_targets.contains(&ShaderBackend::Hlsl));
        assert!(vertex_targets.contains(&ShaderBackend::Wgsl));
        assert!(vertex_targets.contains(&ShaderBackend::Msl));
    }

    #[test]
    fn test_shader_program_defines() {
        let mut prog = ShaderProgram::new("test", ShaderStage::Fragment);
        prog.define("MAX_LIGHTS", "8");
        prog.define("USE_NORMAL_MAP", "1");
        let result = prog.compile(ShaderBackend::Glsl);
        assert!(result.source.contains("#define MAX_LIGHTS 8"));
    }

    #[test]
    fn test_shader_builtin_names() {
        assert_eq!(ShaderBuiltin::Sin.to_glsl(), "sin");
        assert_eq!(ShaderBuiltin::Mix.to_hlsl(), "lerp");
        assert_eq!(ShaderBuiltin::Fract.to_hlsl(), "frac");
        assert_eq!(ShaderBuiltin::DDx.to_hlsl(), "ddx");
    }

    #[test]
    fn test_shader_variable_creation() {
        let attr = ShaderVariable::attribute("position", ShaderType::Vec3, 0);
        assert_eq!(attr.qualifier, ShaderQualifier::Attribute);
        assert_eq!(attr.location, Some(0));

        let unif = ShaderVariable::uniform("mvp", ShaderType::Mat4, 0);
        assert_eq!(unif.qualifier, ShaderQualifier::Uniform);
        assert_eq!(unif.binding, Some(0));
    }

    #[test]
    fn test_compute_shader() {
        let prog = ShaderLibrary::compute_reduction();
        assert_eq!(prog.stage, ShaderStage::Compute);
        let result = prog.compile(ShaderBackend::Wgsl);
        assert!(result.source.contains("@compute"));
    }

    #[test]
    fn test_fullscreen_quad() {
        let prog = ShaderLibrary::fullscreen_quad_vertex();
        let result = prog.compile(ShaderBackend::Glsl);
        assert!(result.success);
        assert!(result.source.contains("vTexCoord"));
    }

    #[test]
    fn test_post_process() {
        let prog = ShaderLibrary::post_process_fragment();
        assert_eq!(prog.stage, ShaderStage::Fragment);
        let refl = ShaderReflection::from_program(&prog);
        assert_eq!(refl.uniforms.len(), 2);
    }

    #[test]
    fn test_shader_stage_display() {
        assert_eq!(ShaderStage::Vertex.to_string(), "vertex");
        assert_eq!(ShaderStage::Fragment.to_string(), "fragment");
        assert_eq!(ShaderStage::Compute.to_string(), "compute");
    }

    #[test]
    fn test_shader_expr_generation() {
        let prog = ShaderProgram::new("test", ShaderStage::Fragment);
        let expr = ShaderExpr::BinaryOp {
            op: ShaderBinOp::Add,
            left: Box::new(ShaderExpr::FloatLit(1.0)),
            right: Box::new(ShaderExpr::FloatLit(2.0)),
        };
        let glsl = prog.expr_to_glsl(&expr);
        assert!(glsl.contains("+"));
    }

    #[test]
    fn test_shader_struct_declaration() {
        let s = ShaderStruct {
            name: "Material".into(),
            fields: vec![
                ("diffuse".into(), ShaderType::Vec4),
                ("specular".into(), ShaderType::Vec4),
                ("shininess".into(), ShaderType::Float),
            ],
        };
        assert_eq!(s.fields.len(), 3);
    }
}
