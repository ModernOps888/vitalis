//! Vitalis WebAssembly Target — WASM module builder.
//!
//! Compiles Vitalis programs to WebAssembly binary format:
//! - **Module Builder**: Constructs valid .wasm modules
//! - **Type Section**: Function type signatures
//! - **Function Section**: Function body encodings
//! - **Export Section**: Exported functions, memories, tables
//! - **Code Section**: WASM instruction encoding
//! - **Memory Section**: Linear memory configuration
//! - **Validation**: Module integrity checks
//!
//! Produces spec-compliant WebAssembly 1.0 binaries that can run
//! in browsers, Node.js, Wasmtime, Wasmer, etc.

use std::fmt;

// ─── WASM Types ─────────────────────────────────────────────────────────

/// WebAssembly value types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
}

impl WasmType {
    /// Binary encoding of the value type.
    pub fn encode(&self) -> u8 {
        match self {
            WasmType::I32 => 0x7F,
            WasmType::I64 => 0x7E,
            WasmType::F32 => 0x7D,
            WasmType::F64 => 0x7C,
        }
    }

    pub fn from_vitalis(ty: &str) -> Option<Self> {
        match ty {
            "i32" => Some(WasmType::I32),
            "i64" => Some(WasmType::I64),
            "f32" => Some(WasmType::F32),
            "f64" => Some(WasmType::F64),
            "bool" => Some(WasmType::I32),
            _ => None,
        }
    }
}

impl fmt::Display for WasmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmType::I32 => write!(f, "i32"),
            WasmType::I64 => write!(f, "i64"),
            WasmType::F32 => write!(f, "f32"),
            WasmType::F64 => write!(f, "f64"),
        }
    }
}

/// A WebAssembly function type (params → results).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncType {
    pub params: Vec<WasmType>,
    pub results: Vec<WasmType>,
}

impl FuncType {
    pub fn new(params: Vec<WasmType>, results: Vec<WasmType>) -> Self {
        Self { params, results }
    }

    /// Encode as WASM binary.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = vec![0x60]; // functype marker
        // params
        encode_vec_types(&self.params, &mut bytes);
        // results
        encode_vec_types(&self.results, &mut bytes);
        bytes
    }
}

fn encode_vec_types(types: &[WasmType], out: &mut Vec<u8>) {
    encode_leb128_u32(types.len() as u32, out);
    for ty in types {
        out.push(ty.encode());
    }
}

// ─── WASM Instructions ──────────────────────────────────────────────────

/// WebAssembly instructions for code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum WasmInst {
    // Control
    Unreachable,
    Nop,
    Block(Option<WasmType>),
    Loop(Option<WasmType>),
    If(Option<WasmType>),
    Else,
    End,
    Br(u32),
    BrIf(u32),
    Return,
    Call(u32),

    // Constants
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),

    // Local/Global
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),

    // Memory
    I32Load(u32, u32),
    I64Load(u32, u32),
    I32Store(u32, u32),
    I64Store(u32, u32),

    // Arithmetic i32
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32RemS,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32GtS,
    I32LeS,
    I32GeS,

    // Arithmetic i64
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64Eq,
    I64Ne,
    I64LtS,
    I64GtS,

    // Arithmetic f64
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,

    // Conversions
    I32WrapI64,
    I64ExtendI32S,
    F64ConvertI64S,
    I64TruncF64S,

    // Misc
    Drop,
    Select,
}

impl WasmInst {
    /// Encode a single instruction to binary.
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            WasmInst::Unreachable => out.push(0x00),
            WasmInst::Nop => out.push(0x01),
            WasmInst::Block(ty) => { out.push(0x02); encode_block_type(ty, out); },
            WasmInst::Loop(ty) => { out.push(0x03); encode_block_type(ty, out); },
            WasmInst::If(ty) => { out.push(0x04); encode_block_type(ty, out); },
            WasmInst::Else => out.push(0x05),
            WasmInst::End => out.push(0x0B),
            WasmInst::Br(l) => { out.push(0x0C); encode_leb128_u32(*l, out); },
            WasmInst::BrIf(l) => { out.push(0x0D); encode_leb128_u32(*l, out); },
            WasmInst::Return => out.push(0x0F),
            WasmInst::Call(idx) => { out.push(0x10); encode_leb128_u32(*idx, out); },

            WasmInst::I32Const(v) => { out.push(0x41); encode_leb128_i32(*v, out); },
            WasmInst::I64Const(v) => { out.push(0x42); encode_leb128_i64(*v, out); },
            WasmInst::F32Const(v) => { out.push(0x43); out.extend_from_slice(&v.to_le_bytes()); },
            WasmInst::F64Const(v) => { out.push(0x44); out.extend_from_slice(&v.to_le_bytes()); },

            WasmInst::LocalGet(i) => { out.push(0x20); encode_leb128_u32(*i, out); },
            WasmInst::LocalSet(i) => { out.push(0x21); encode_leb128_u32(*i, out); },
            WasmInst::LocalTee(i) => { out.push(0x22); encode_leb128_u32(*i, out); },
            WasmInst::GlobalGet(i) => { out.push(0x23); encode_leb128_u32(*i, out); },
            WasmInst::GlobalSet(i) => { out.push(0x24); encode_leb128_u32(*i, out); },

            WasmInst::I32Load(align, offset) => { out.push(0x28); encode_leb128_u32(*align, out); encode_leb128_u32(*offset, out); },
            WasmInst::I64Load(align, offset) => { out.push(0x29); encode_leb128_u32(*align, out); encode_leb128_u32(*offset, out); },
            WasmInst::I32Store(align, offset) => { out.push(0x36); encode_leb128_u32(*align, out); encode_leb128_u32(*offset, out); },
            WasmInst::I64Store(align, offset) => { out.push(0x37); encode_leb128_u32(*align, out); encode_leb128_u32(*offset, out); },

            WasmInst::I32Add => out.push(0x6A),
            WasmInst::I32Sub => out.push(0x6B),
            WasmInst::I32Mul => out.push(0x6C),
            WasmInst::I32DivS => out.push(0x6D),
            WasmInst::I32RemS => out.push(0x6F),
            WasmInst::I32And => out.push(0x71),
            WasmInst::I32Or => out.push(0x72),
            WasmInst::I32Xor => out.push(0x73),
            WasmInst::I32Shl => out.push(0x74),
            WasmInst::I32ShrS => out.push(0x75),
            WasmInst::I32Eqz => out.push(0x45),
            WasmInst::I32Eq => out.push(0x46),
            WasmInst::I32Ne => out.push(0x47),
            WasmInst::I32LtS => out.push(0x48),
            WasmInst::I32GtS => out.push(0x4A),
            WasmInst::I32LeS => out.push(0x4C),
            WasmInst::I32GeS => out.push(0x4E),

            WasmInst::I64Add => out.push(0x7C),
            WasmInst::I64Sub => out.push(0x7D),
            WasmInst::I64Mul => out.push(0x7E),
            WasmInst::I64DivS => out.push(0x7F),
            WasmInst::I64Eq => out.push(0x51),
            WasmInst::I64Ne => out.push(0x52),
            WasmInst::I64LtS => out.push(0x53),
            WasmInst::I64GtS => out.push(0x55),

            WasmInst::F64Add => out.push(0xA0),
            WasmInst::F64Sub => out.push(0xA1),
            WasmInst::F64Mul => out.push(0xA2),
            WasmInst::F64Div => out.push(0xA3),
            WasmInst::F64Eq => out.push(0x61),
            WasmInst::F64Ne => out.push(0x62),
            WasmInst::F64Lt => out.push(0x63),
            WasmInst::F64Gt => out.push(0x64),

            WasmInst::I32WrapI64 => out.push(0xA7),
            WasmInst::I64ExtendI32S => out.push(0xAC),
            WasmInst::F64ConvertI64S => out.push(0xB9),
            WasmInst::I64TruncF64S => out.push(0xB0),

            WasmInst::Drop => out.push(0x1A),
            WasmInst::Select => out.push(0x1B),
        }
    }
}

fn encode_block_type(ty: &Option<WasmType>, out: &mut Vec<u8>) {
    match ty {
        Some(t) => out.push(t.encode()),
        None => out.push(0x40), // empty block type
    }
}

// ─── LEB128 Encoding ────────────────────────────────────────────────────

pub fn encode_leb128_u32(mut value: u32, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

pub fn encode_leb128_i32(mut value: i32, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        let more = !((value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0));
        if more {
            out.push(byte | 0x80);
        } else {
            out.push(byte);
            break;
        }
    }
}

pub fn encode_leb128_i64(mut value: i64, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        let more = !((value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0));
        if more {
            out.push(byte | 0x80);
        } else {
            out.push(byte);
            break;
        }
    }
}

// ─── WASM Sections ──────────────────────────────────────────────────────

/// WASM section identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionId {
    Type = 1,
    Import = 2,
    Function = 3,
    Table = 4,
    Memory = 5,
    Global = 6,
    Export = 7,
    Start = 8,
    Element = 9,
    Code = 10,
    Data = 11,
}

/// Export kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Function = 0,
    Table = 1,
    Memory = 2,
    Global = 3,
}

/// An export entry.
#[derive(Debug, Clone)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
    pub index: u32,
}

/// Memory limits.
#[derive(Debug, Clone, Copy)]
pub struct MemoryLimits {
    pub initial: u32,
    pub maximum: Option<u32>,
}

impl MemoryLimits {
    pub fn new(initial: u32, maximum: Option<u32>) -> Self {
        Self { initial, maximum }
    }

    /// Encode memory limits.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self.maximum {
            Some(max) => {
                bytes.push(0x01); // has max
                encode_leb128_u32(self.initial, &mut bytes);
                encode_leb128_u32(max, &mut bytes);
            }
            None => {
                bytes.push(0x00); // no max
                encode_leb128_u32(self.initial, &mut bytes);
            }
        }
        bytes
    }
}

/// A function body.
#[derive(Debug, Clone)]
pub struct FunctionBody {
    pub locals: Vec<(u32, WasmType)>,
    pub instructions: Vec<WasmInst>,
}

impl FunctionBody {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            instructions: Vec::new(),
        }
    }

    pub fn add_local(&mut self, count: u32, ty: WasmType) {
        self.locals.push((count, ty));
    }

    pub fn emit(&mut self, inst: WasmInst) {
        self.instructions.push(inst);
    }

    /// Encode as WASM code body.
    pub fn encode(&self) -> Vec<u8> {
        let mut body = Vec::new();

        // locals
        encode_leb128_u32(self.locals.len() as u32, &mut body);
        for (count, ty) in &self.locals {
            encode_leb128_u32(*count, &mut body);
            body.push(ty.encode());
        }

        // instructions
        for inst in &self.instructions {
            inst.encode(&mut body);
        }
        body.push(0x0B); // end

        // size-prefixed
        let mut out = Vec::new();
        encode_leb128_u32(body.len() as u32, &mut out);
        out.extend(body);
        out
    }
}

impl Default for FunctionBody {
    fn default() -> Self {
        Self::new()
    }
}

// ─── WASM Module Builder ────────────────────────────────────────────────

/// Builds a complete WASM module.
#[derive(Debug)]
pub struct WasmModule {
    pub types: Vec<FuncType>,
    pub functions: Vec<u32>,       // index into types
    pub exports: Vec<Export>,
    pub memory: Option<MemoryLimits>,
    pub bodies: Vec<FunctionBody>,
}

impl WasmModule {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            functions: Vec::new(),
            exports: Vec::new(),
            memory: None,
            bodies: Vec::new(),
        }
    }

    /// Add a function type and return its type index.
    pub fn add_type(&mut self, func_type: FuncType) -> u32 {
        // Re-use existing if identical
        for (i, existing) in self.types.iter().enumerate() {
            if existing == &func_type {
                return i as u32;
            }
        }
        let idx = self.types.len() as u32;
        self.types.push(func_type);
        idx
    }

    /// Add a function with the given type index. Returns function index.
    pub fn add_function(&mut self, type_idx: u32, body: FunctionBody) -> u32 {
        let func_idx = self.functions.len() as u32;
        self.functions.push(type_idx);
        self.bodies.push(body);
        func_idx
    }

    /// Add an export.
    pub fn add_export(&mut self, name: &str, kind: ExportKind, index: u32) {
        self.exports.push(Export {
            name: name.to_string(),
            kind,
            index,
        });
    }

    /// Set memory limits.
    pub fn set_memory(&mut self, initial: u32, maximum: Option<u32>) {
        self.memory = Some(MemoryLimits::new(initial, maximum));
    }

    /// Encode the complete module to WASM binary.
    pub fn encode(&self) -> Vec<u8> {
        let mut wasm = Vec::new();

        // Magic number + version
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // \0asm
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1

        // Type section
        if !self.types.is_empty() {
            let mut payload = Vec::new();
            encode_leb128_u32(self.types.len() as u32, &mut payload);
            for ft in &self.types {
                payload.extend(ft.encode());
            }
            self.emit_section(SectionId::Type, &payload, &mut wasm);
        }

        // Function section
        if !self.functions.is_empty() {
            let mut payload = Vec::new();
            encode_leb128_u32(self.functions.len() as u32, &mut payload);
            for &type_idx in &self.functions {
                encode_leb128_u32(type_idx, &mut payload);
            }
            self.emit_section(SectionId::Function, &payload, &mut wasm);
        }

        // Memory section
        if let Some(mem) = &self.memory {
            let mut payload = Vec::new();
            encode_leb128_u32(1, &mut payload);
            payload.extend(mem.encode());
            self.emit_section(SectionId::Memory, &payload, &mut wasm);
        }

        // Export section
        if !self.exports.is_empty() {
            let mut payload = Vec::new();
            encode_leb128_u32(self.exports.len() as u32, &mut payload);
            for exp in &self.exports {
                encode_leb128_u32(exp.name.len() as u32, &mut payload);
                payload.extend_from_slice(exp.name.as_bytes());
                payload.push(exp.kind as u8);
                encode_leb128_u32(exp.index, &mut payload);
            }
            self.emit_section(SectionId::Export, &payload, &mut wasm);
        }

        // Code section
        if !self.bodies.is_empty() {
            let mut payload = Vec::new();
            encode_leb128_u32(self.bodies.len() as u32, &mut payload);
            for body in &self.bodies {
                payload.extend(body.encode());
            }
            self.emit_section(SectionId::Code, &payload, &mut wasm);
        }

        wasm
    }

    fn emit_section(&self, id: SectionId, payload: &[u8], out: &mut Vec<u8>) {
        out.push(id as u8);
        encode_leb128_u32(payload.len() as u32, out);
        out.extend_from_slice(payload);
    }

    /// Validate the module structure.
    pub fn validate(&self) -> Result<(), String> {
        if self.functions.len() != self.bodies.len() {
            return Err(format!(
                "Function/body count mismatch: {} functions, {} bodies",
                self.functions.len(),
                self.bodies.len()
            ));
        }

        for (i, &type_idx) in self.functions.iter().enumerate() {
            if type_idx as usize >= self.types.len() {
                return Err(format!(
                    "Function {} references invalid type index {}",
                    i, type_idx
                ));
            }
        }

        for exp in &self.exports {
            match exp.kind {
                ExportKind::Function => {
                    if exp.index as usize >= self.functions.len() {
                        return Err(format!(
                            "Export '{}' references invalid function index {}",
                            exp.name, exp.index
                        ));
                    }
                }
                ExportKind::Memory => {
                    if self.memory.is_none() {
                        return Err(format!(
                            "Export '{}' references memory but none defined",
                            exp.name
                        ));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl Default for WasmModule {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helper: Build a simple function ────────────────────────────────────

/// Quick helper to build a WASM module with a single exported function
/// that returns a constant i64.
pub fn build_const_i64_module(name: &str, value: i64) -> Vec<u8> {
    let mut module = WasmModule::new();

    let type_idx = module.add_type(FuncType::new(vec![], vec![WasmType::I64]));
    let mut body = FunctionBody::new();
    body.emit(WasmInst::I64Const(value));
    let func_idx = module.add_function(type_idx, body);
    module.add_export(name, ExportKind::Function, func_idx);

    module.encode()
}

/// Quick helper to build a WASM "add" function module.
pub fn build_add_i64_module() -> Vec<u8> {
    let mut module = WasmModule::new();

    let type_idx = module.add_type(FuncType::new(
        vec![WasmType::I64, WasmType::I64],
        vec![WasmType::I64],
    ));
    let mut body = FunctionBody::new();
    body.emit(WasmInst::LocalGet(0));
    body.emit(WasmInst::LocalGet(1));
    body.emit(WasmInst::I64Add);
    let func_idx = module.add_function(type_idx, body);
    module.add_export("add", ExportKind::Function, func_idx);

    module.set_memory(1, Some(256));

    module.encode()
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_type_encode() {
        assert_eq!(WasmType::I32.encode(), 0x7F);
        assert_eq!(WasmType::I64.encode(), 0x7E);
        assert_eq!(WasmType::F32.encode(), 0x7D);
        assert_eq!(WasmType::F64.encode(), 0x7C);
    }

    #[test]
    fn test_wasm_type_display() {
        assert_eq!(format!("{}", WasmType::I64), "i64");
        assert_eq!(format!("{}", WasmType::F32), "f32");
    }

    #[test]
    fn test_wasm_type_from_vitalis() {
        assert_eq!(WasmType::from_vitalis("i64"), Some(WasmType::I64));
        assert_eq!(WasmType::from_vitalis("bool"), Some(WasmType::I32));
        assert_eq!(WasmType::from_vitalis("str"), None);
    }

    #[test]
    fn test_func_type_encode() {
        let ft = FuncType::new(vec![WasmType::I64], vec![WasmType::I64]);
        let encoded = ft.encode();
        assert_eq!(encoded[0], 0x60);
    }

    #[test]
    fn test_leb128_u32() {
        let mut buf = Vec::new();
        encode_leb128_u32(0, &mut buf);
        assert_eq!(buf, vec![0]);

        buf.clear();
        encode_leb128_u32(127, &mut buf);
        assert_eq!(buf, vec![127]);

        buf.clear();
        encode_leb128_u32(128, &mut buf);
        assert_eq!(buf, vec![0x80, 0x01]);

        buf.clear();
        encode_leb128_u32(624485, &mut buf);
        assert_eq!(buf, vec![0xE5, 0x8E, 0x26]);
    }

    #[test]
    fn test_leb128_i32() {
        let mut buf = Vec::new();
        encode_leb128_i32(0, &mut buf);
        assert_eq!(buf, vec![0]);

        buf.clear();
        encode_leb128_i32(-1, &mut buf);
        assert_eq!(buf, vec![0x7F]);

        buf.clear();
        encode_leb128_i32(42, &mut buf);
        assert_eq!(buf, vec![42]);
    }

    #[test]
    fn test_leb128_i64() {
        let mut buf = Vec::new();
        encode_leb128_i64(0, &mut buf);
        assert_eq!(buf, vec![0]);

        buf.clear();
        encode_leb128_i64(-1, &mut buf);
        assert_eq!(buf, vec![0x7F]);
    }

    #[test]
    fn test_wasm_module_magic() {
        let module = WasmModule::new();
        let binary = module.encode();
        assert_eq!(&binary[0..4], &[0x00, 0x61, 0x73, 0x6D]);
        assert_eq!(&binary[4..8], &[0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_empty_module_valid() {
        let module = WasmModule::new();
        assert!(module.validate().is_ok());
    }

    #[test]
    fn test_add_type_dedup() {
        let mut module = WasmModule::new();
        let idx1 = module.add_type(FuncType::new(vec![WasmType::I64], vec![WasmType::I64]));
        let idx2 = module.add_type(FuncType::new(vec![WasmType::I64], vec![WasmType::I64]));
        assert_eq!(idx1, idx2);
        assert_eq!(module.types.len(), 1);
    }

    #[test]
    fn test_add_function() {
        let mut module = WasmModule::new();
        let type_idx = module.add_type(FuncType::new(vec![], vec![WasmType::I64]));
        let mut body = FunctionBody::new();
        body.emit(WasmInst::I64Const(42));
        let func_idx = module.add_function(type_idx, body);
        assert_eq!(func_idx, 0);
        assert_eq!(module.functions.len(), 1);
    }

    #[test]
    fn test_add_export() {
        let mut module = WasmModule::new();
        module.add_export("main", ExportKind::Function, 0);
        assert_eq!(module.exports.len(), 1);
        assert_eq!(module.exports[0].name, "main");
    }

    #[test]
    fn test_memory_limits() {
        let mem = MemoryLimits::new(1, Some(256));
        let encoded = mem.encode();
        assert_eq!(encoded[0], 0x01); // has max

        let mem_no_max = MemoryLimits::new(1, None);
        let encoded2 = mem_no_max.encode();
        assert_eq!(encoded2[0], 0x00); // no max
    }

    #[test]
    fn test_build_const_module() {
        let wasm = build_const_i64_module("answer", 42);
        assert!(wasm.len() > 8);
        assert_eq!(&wasm[0..4], &[0x00, 0x61, 0x73, 0x6D]);
    }

    #[test]
    fn test_build_add_module() {
        let wasm = build_add_i64_module();
        assert!(wasm.len() > 8);
        assert_eq!(&wasm[0..4], &[0x00, 0x61, 0x73, 0x6D]);
    }

    #[test]
    fn test_function_body_locals() {
        let mut body = FunctionBody::new();
        body.add_local(1, WasmType::I64);
        body.add_local(2, WasmType::I32);
        assert_eq!(body.locals.len(), 2);
    }

    #[test]
    fn test_function_body_encode() {
        let mut body = FunctionBody::new();
        body.emit(WasmInst::I64Const(99));
        let encoded = body.encode();
        assert!(!encoded.is_empty());
        // Must end with "end" (0x0B)
        assert_eq!(*encoded.last().unwrap(), 0x0B);
    }

    #[test]
    fn test_validate_mismatch() {
        let mut module = WasmModule::new();
        module.functions.push(0); // no matching type
        module.bodies.push(FunctionBody::new());
        assert!(module.validate().is_err());
    }

    #[test]
    fn test_validate_bad_export() {
        let mut module = WasmModule::new();
        module.add_export("missing", ExportKind::Function, 0);
        assert!(module.validate().is_err());
    }

    #[test]
    fn test_validate_memory_export() {
        let mut module = WasmModule::new();
        module.add_export("mem", ExportKind::Memory, 0);
        assert!(module.validate().is_err()); // no memory defined

        module.set_memory(1, None);
        assert!(module.validate().is_ok());
    }

    #[test]
    fn test_instruction_encoding_i32_add() {
        let mut out = Vec::new();
        WasmInst::I32Add.encode(&mut out);
        assert_eq!(out, vec![0x6A]);
    }

    #[test]
    fn test_instruction_encoding_call() {
        let mut out = Vec::new();
        WasmInst::Call(5).encode(&mut out);
        assert_eq!(out[0], 0x10);
    }

    #[test]
    fn test_instruction_encoding_f64_const() {
        let mut out = Vec::new();
        WasmInst::F64Const(3.14).encode(&mut out);
        assert_eq!(out[0], 0x44);
        assert_eq!(out.len(), 9);
    }

    #[test]
    fn test_full_module_encode() {
        let mut module = WasmModule::new();
        let type_idx = module.add_type(FuncType::new(
            vec![WasmType::I64, WasmType::I64],
            vec![WasmType::I64],
        ));
        let mut body = FunctionBody::new();
        body.emit(WasmInst::LocalGet(0));
        body.emit(WasmInst::LocalGet(1));
        body.emit(WasmInst::I64Add);
        let func_idx = module.add_function(type_idx, body);
        module.add_export("add", ExportKind::Function, func_idx);
        module.set_memory(1, Some(16));

        assert!(module.validate().is_ok());
        let binary = module.encode();
        assert!(binary.len() > 20);
        assert_eq!(&binary[0..4], &[0x00, 0x61, 0x73, 0x6D]);
    }

    #[test]
    fn test_section_ordering() {
        let mut module = WasmModule::new();
        let type_idx = module.add_type(FuncType::new(vec![], vec![WasmType::I32]));
        let mut body = FunctionBody::new();
        body.emit(WasmInst::I32Const(1));
        module.add_function(type_idx, body);
        module.set_memory(1, None);
        module.add_export("main", ExportKind::Function, 0);

        let binary = module.encode();
        // After magic+version (8 bytes), sections should appear in order:
        // Type(1), Function(3), Memory(5), Export(7), Code(10)
        let mut pos = 8;
        let mut prev_id = 0u8;
        while pos < binary.len() {
            let section_id = binary[pos];
            assert!(section_id > prev_id, "Sections not in ascending order");
            prev_id = section_id;
            pos += 1;
            // Read size (LEB128)
            let mut size = 0u32;
            let mut shift = 0;
            loop {
                let byte = binary[pos];
                pos += 1;
                size |= ((byte & 0x7F) as u32) << shift;
                shift += 7;
                if byte & 0x80 == 0 { break; }
            }
            pos += size as usize;
        }
    }
}
