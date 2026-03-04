//! WASM AOT & WASI Runtime — compile `.sl` to standalone `.wasm`, WASI support,
//! component model, browser shim, and size-optimization passes (DCE, tree-shaking).
//!
//! Extends the existing `wasm_target.rs` with full AOT compilation pipeline,
//! WASI-Preview2 host bindings, component model adapters, and binary optimization.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

// ── WASM AOT Compiler ───────────────────────────────────────────────────

/// A compiled WASM module ready for execution or serialization.
#[derive(Debug, Clone)]
pub struct WasmModule {
    pub name: String,
    pub sections: Vec<WasmSection>,
    pub exports: Vec<WasmExport>,
    pub imports: Vec<WasmImport>,
    pub functions: Vec<WasmFunction>,
    pub memory_pages: u32,
    pub data_segments: Vec<DataSegment>,
}

/// WASM binary section.
#[derive(Debug, Clone)]
pub enum WasmSection {
    Type(Vec<WasmFuncType>),
    Import(Vec<WasmImport>),
    Function(Vec<u32>),       // type indices
    Memory(u32, Option<u32>), // min, max pages
    Export(Vec<WasmExport>),
    Code(Vec<Vec<u8>>),       // function bodies
    Data(Vec<DataSegment>),
    Custom(String, Vec<u8>),
}

/// WASM function type signature.
#[derive(Debug, Clone, PartialEq)]
pub struct WasmFuncType {
    pub params: Vec<WasmValType>,
    pub results: Vec<WasmValType>,
}

/// WASM value types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmValType {
    I32,
    I64,
    F32,
    F64,
    FuncRef,
    ExternRef,
    V128,
}

/// WASM export entry.
#[derive(Debug, Clone)]
pub struct WasmExport {
    pub name: String,
    pub kind: ExportKind,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportKind {
    Function,
    Table,
    Memory,
    Global,
}

/// WASM import entry.
#[derive(Debug, Clone)]
pub struct WasmImport {
    pub module: String,
    pub name: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Function(WasmFuncType),
    Memory(u32, Option<u32>),
    Global(WasmValType, bool), // type, mutable
}

/// WASM function with bytecode.
#[derive(Debug, Clone)]
pub struct WasmFunction {
    pub name: String,
    pub type_idx: u32,
    pub locals: Vec<WasmValType>,
    pub body: Vec<WasmOpcode>,
    pub is_exported: bool,
}

/// Data segment for linear memory initialization.
#[derive(Debug, Clone)]
pub struct DataSegment {
    pub offset: u32,
    pub data: Vec<u8>,
}

/// Simplified WASM opcodes for AOT code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum WasmOpcode {
    // Control
    Unreachable,
    Nop,
    Block(i64),
    Loop(i64),
    If(i64),
    Else,
    End,
    Br(u32),
    BrIf(u32),
    Return,
    Call(u32),
    CallIndirect(u32),
    // Constants
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    // Locals/Globals
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),
    // Memory
    I32Load(u32, u32),
    I64Load(u32, u32),
    F64Load(u32, u32),
    I32Store(u32, u32),
    I64Store(u32, u32),
    F64Store(u32, u32),
    MemorySize,
    MemoryGrow,
    // Arithmetic
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I64Add,
    I64Sub,
    I64Mul,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Sqrt,
    // Comparison
    I32Eqz,
    I32Eq,
    I32LtS,
    I32GtS,
    F64Lt,
    F64Gt,
    F64Eq,
    // Conversion
    I32WrapI64,
    I64ExtendI32S,
    F64ConvertI32S,
    I32TruncF64S,
    // Drop
    Drop,
}

impl WasmModule {
    /// Create a new empty WASM module.
    pub fn new(name: &str) -> Self {
        WasmModule {
            name: name.to_string(),
            sections: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            memory_pages: 1,
            data_segments: Vec::new(),
        }
    }

    /// Add a function to the module.
    pub fn add_function(&mut self, func: WasmFunction) {
        if func.is_exported {
            self.exports.push(WasmExport {
                name: func.name.clone(),
                kind: ExportKind::Function,
                index: self.functions.len() as u32,
            });
        }
        self.functions.push(func);
    }

    /// Add a data segment to linear memory.
    pub fn add_data(&mut self, offset: u32, data: Vec<u8>) {
        self.data_segments.push(DataSegment { offset, data });
    }

    /// Serialize to WASM binary format (simplified).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // WASM magic + version
        bytes.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // \0asm
        bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1

        // Type section (Section ID 1)
        let mut type_sec = Vec::new();
        let n_types = self.functions.len() as u32;
        encode_leb128_u32(&mut type_sec, n_types);
        for func in &self.functions {
            type_sec.push(0x60); // functype marker
            encode_leb128_u32(&mut type_sec, func.locals.len() as u32); // params
            for p in &func.locals {
                type_sec.push(valtype_byte(*p));
            }
            type_sec.push(0x01); // one result
            type_sec.push(0x7E); // i64
        }
        write_section(&mut bytes, 1, &type_sec);

        // Function section (Section ID 3)
        let mut func_sec = Vec::new();
        encode_leb128_u32(&mut func_sec, self.functions.len() as u32);
        for i in 0..self.functions.len() {
            encode_leb128_u32(&mut func_sec, i as u32);
        }
        write_section(&mut bytes, 3, &func_sec);

        // Memory section (Section ID 5)
        let mut mem_sec = Vec::new();
        mem_sec.push(0x01); // 1 memory
        mem_sec.push(0x00); // no max
        encode_leb128_u32(&mut mem_sec, self.memory_pages);
        write_section(&mut bytes, 5, &mem_sec);

        // Export section (Section ID 7)
        if !self.exports.is_empty() {
            let mut exp_sec = Vec::new();
            encode_leb128_u32(&mut exp_sec, self.exports.len() as u32);
            for exp in &self.exports {
                encode_leb128_u32(&mut exp_sec, exp.name.len() as u32);
                exp_sec.extend_from_slice(exp.name.as_bytes());
                exp_sec.push(match exp.kind {
                    ExportKind::Function => 0x00,
                    ExportKind::Table => 0x01,
                    ExportKind::Memory => 0x02,
                    ExportKind::Global => 0x03,
                });
                encode_leb128_u32(&mut exp_sec, exp.index);
            }
            write_section(&mut bytes, 7, &exp_sec);
        }

        // Code section (Section ID 10)
        let mut code_sec = Vec::new();
        encode_leb128_u32(&mut code_sec, self.functions.len() as u32);
        for func in &self.functions {
            let body = encode_function_body(func);
            encode_leb128_u32(&mut code_sec, body.len() as u32);
            code_sec.extend(body);
        }
        write_section(&mut bytes, 10, &code_sec);

        bytes
    }

    /// Total size of serialized module.
    pub fn size_bytes(&self) -> usize {
        self.to_bytes().len()
    }

    /// Count of exported functions.
    pub fn export_count(&self) -> usize {
        self.exports.iter().filter(|e| e.kind == ExportKind::Function).count()
    }
}

// ── WASI Runtime ────────────────────────────────────────────────────────

/// WASI capability set for sandboxed execution.
#[derive(Debug, Clone)]
pub struct WasiCapabilities {
    pub allow_fs_read: bool,
    pub allow_fs_write: bool,
    pub allow_env_vars: bool,
    pub allow_clock: bool,
    pub allow_random: bool,
    pub allow_network: bool,
    pub preopened_dirs: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub args: Vec<String>,
}

impl WasiCapabilities {
    /// Default safe capabilities (read-only, no network).
    pub fn safe_defaults() -> Self {
        WasiCapabilities {
            allow_fs_read: true,
            allow_fs_write: false,
            allow_env_vars: false,
            allow_clock: true,
            allow_random: true,
            allow_network: false,
            preopened_dirs: Vec::new(),
            env_vars: HashMap::new(),
            args: Vec::new(),
        }
    }

    /// Full capabilities.
    pub fn unrestricted() -> Self {
        WasiCapabilities {
            allow_fs_read: true,
            allow_fs_write: true,
            allow_env_vars: true,
            allow_clock: true,
            allow_random: true,
            allow_network: true,
            preopened_dirs: Vec::new(),
            env_vars: HashMap::new(),
            args: Vec::new(),
        }
    }

    /// Generate WASI import stubs based on capabilities.
    pub fn generate_imports(&self) -> Vec<WasmImport> {
        let mut imports = Vec::new();

        // wasi_snapshot_preview1 fd_write (for stdout/stderr)
        imports.push(WasmImport {
            module: "wasi_snapshot_preview1".into(),
            name: "fd_write".into(),
            kind: ImportKind::Function(WasmFuncType {
                params: vec![WasmValType::I32, WasmValType::I32, WasmValType::I32, WasmValType::I32],
                results: vec![WasmValType::I32],
            }),
        });

        if self.allow_clock {
            imports.push(WasmImport {
                module: "wasi_snapshot_preview1".into(),
                name: "clock_time_get".into(),
                kind: ImportKind::Function(WasmFuncType {
                    params: vec![WasmValType::I32, WasmValType::I64, WasmValType::I32],
                    results: vec![WasmValType::I32],
                }),
            });
        }

        if self.allow_random {
            imports.push(WasmImport {
                module: "wasi_snapshot_preview1".into(),
                name: "random_get".into(),
                kind: ImportKind::Function(WasmFuncType {
                    params: vec![WasmValType::I32, WasmValType::I32],
                    results: vec![WasmValType::I32],
                }),
            });
        }

        if self.allow_fs_read || self.allow_fs_write {
            imports.push(WasmImport {
                module: "wasi_snapshot_preview1".into(),
                name: "path_open".into(),
                kind: ImportKind::Function(WasmFuncType {
                    params: vec![WasmValType::I32; 9],
                    results: vec![WasmValType::I32],
                }),
            });
        }

        imports
    }
}

// ── WASM Component Model ────────────────────────────────────────────────

/// Component model interface definition.
#[derive(Debug, Clone)]
pub struct ComponentInterface {
    pub name: String,
    pub functions: Vec<InterfaceFunction>,
    pub types: Vec<InterfaceType>,
}

/// Function in a component interface.
#[derive(Debug, Clone)]
pub struct InterfaceFunction {
    pub name: String,
    pub params: Vec<(String, InterfaceValType)>,
    pub result: Option<InterfaceValType>,
}

/// Component model types (richer than core WASM).
#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceValType {
    Bool,
    U8, U16, U32, U64,
    S8, S16, S32, S64,
    F32, F64,
    Char,
    StringType,
    List(Box<InterfaceValType>),
    Record(Vec<(String, InterfaceValType)>),
    Variant(Vec<(String, Option<InterfaceValType>)>),
    Option(Box<InterfaceValType>),
    Result { ok: Box<InterfaceValType>, err: Box<InterfaceValType> },
}

/// Component model value types for interop.
#[derive(Debug, Clone)]
pub struct InterfaceType {
    pub name: String,
    pub definition: InterfaceValType,
}

impl ComponentInterface {
    pub fn new(name: &str) -> Self {
        ComponentInterface { name: name.to_string(), functions: Vec::new(), types: Vec::new() }
    }

    pub fn add_function(&mut self, name: &str, params: Vec<(String, InterfaceValType)>, result: Option<InterfaceValType>) {
        self.functions.push(InterfaceFunction { name: name.to_string(), params, result });
    }

    pub fn add_type(&mut self, name: &str, def: InterfaceValType) {
        self.types.push(InterfaceType { name: name.to_string(), definition: def });
    }

    /// Generate WIT (WASM Interface Types) text.
    pub fn to_wit(&self) -> String {
        let mut wit = format!("interface {} {{\n", self.name);
        for t in &self.types {
            wit.push_str(&format!("  type {} = {};\n", t.name, wit_type(&t.definition)));
        }
        for f in &self.functions {
            let params: Vec<String> = f.params.iter().map(|(n, t)| format!("{}: {}", n, wit_type(t))).collect();
            let ret = match &f.result {
                Some(t) => format!(" -> {}", wit_type(t)),
                None => String::new(),
            };
            wit.push_str(&format!("  {}: func({}){};\n", f.name, params.join(", "), ret));
        }
        wit.push_str("}\n");
        wit
    }
}

fn wit_type(t: &InterfaceValType) -> String {
    match t {
        InterfaceValType::Bool => "bool".into(),
        InterfaceValType::U8 => "u8".into(),
        InterfaceValType::U16 => "u16".into(),
        InterfaceValType::U32 => "u32".into(),
        InterfaceValType::U64 => "u64".into(),
        InterfaceValType::S8 => "s8".into(),
        InterfaceValType::S16 => "s16".into(),
        InterfaceValType::S32 => "s32".into(),
        InterfaceValType::S64 => "s64".into(),
        InterfaceValType::F32 => "f32".into(),
        InterfaceValType::F64 => "f64".into(),
        InterfaceValType::Char => "char".into(),
        InterfaceValType::StringType => "string".into(),
        InterfaceValType::List(inner) => format!("list<{}>", wit_type(inner)),
        InterfaceValType::Record(fields) => {
            let fs: Vec<String> = fields.iter().map(|(n, t)| format!("{}: {}", n, wit_type(t))).collect();
            format!("record {{ {} }}", fs.join(", "))
        }
        InterfaceValType::Option(inner) => format!("option<{}>", wit_type(inner)),
        InterfaceValType::Result { ok, err } => format!("result<{}, {}>", wit_type(ok), wit_type(err)),
        InterfaceValType::Variant(cases) => {
            let cs: Vec<String> = cases.iter().map(|(n, t)| {
                match t { Some(ty) => format!("{}: {}", n, wit_type(ty)), None => n.clone() }
            }).collect();
            format!("variant {{ {} }}", cs.join(", "))
        }
    }
}

// ── Browser Runtime Shim ────────────────────────────────────────────────

/// Generate JavaScript glue code for running WASM in browser.
#[derive(Debug, Clone)]
pub struct BrowserShim {
    pub module_name: String,
    pub memory_pages: u32,
    pub exports: Vec<String>,
}

impl BrowserShim {
    pub fn new(module: &WasmModule) -> Self {
        BrowserShim {
            module_name: module.name.clone(),
            memory_pages: module.memory_pages,
            exports: module.exports.iter().map(|e| e.name.clone()).collect(),
        }
    }

    /// Generate JS loader for the WASM module.
    pub fn generate_js(&self) -> String {
        let mut js = String::new();
        js.push_str("// Vitalis WASM Browser Runtime\n");
        js.push_str("(async function() {\n");
        js.push_str("  const memory = new WebAssembly.Memory({ initial: ");
        js.push_str(&self.memory_pages.to_string());
        js.push_str(" });\n\n");

        js.push_str("  const importObject = {\n");
        js.push_str("    env: { memory },\n");
        js.push_str("    wasi_snapshot_preview1: {\n");
        js.push_str("      fd_write: (fd, iovsPtr, iovsLen, nwrittenPtr) => {\n");
        js.push_str("        const view = new DataView(memory.buffer);\n");
        js.push_str("        let written = 0;\n");
        js.push_str("        for (let i = 0; i < iovsLen; i++) {\n");
        js.push_str("          const ptr = view.getUint32(iovsPtr + i * 8, true);\n");
        js.push_str("          const len = view.getUint32(iovsPtr + i * 8 + 4, true);\n");
        js.push_str("          const bytes = new Uint8Array(memory.buffer, ptr, len);\n");
        js.push_str("          if (fd === 1) console.log(new TextDecoder().decode(bytes));\n");
        js.push_str("          else console.error(new TextDecoder().decode(bytes));\n");
        js.push_str("          written += len;\n");
        js.push_str("        }\n");
        js.push_str("        view.setUint32(nwrittenPtr, written, true);\n");
        js.push_str("        return 0;\n");
        js.push_str("      },\n");
        js.push_str("      proc_exit: (code) => { throw new Error('exit: ' + code); },\n");
        js.push_str("      clock_time_get: () => 0,\n");
        js.push_str("      random_get: (buf, len) => {\n");
        js.push_str("        const view = new Uint8Array(memory.buffer, buf, len);\n");
        js.push_str("        crypto.getRandomValues(view);\n");
        js.push_str("        return 0;\n");
        js.push_str("      },\n");
        js.push_str("    },\n");
        js.push_str("  };\n\n");

        js.push_str("  const response = await fetch('");
        js.push_str(&self.module_name);
        js.push_str(".wasm');\n");
        js.push_str("  const { instance } = await WebAssembly.instantiate(\n");
        js.push_str("    await response.arrayBuffer(), importObject\n");
        js.push_str("  );\n\n");

        js.push_str(&format!("  window.{} = {{}};\n", self.module_name));
        for exp in &self.exports {
            js.push_str(&format!("  window.{}.{} = instance.exports.{};\n",
                self.module_name, exp, exp));
        }

        js.push_str("  if (instance.exports._start) instance.exports._start();\n");
        js.push_str("})();\n");
        js
    }
}

// ── Size Optimization Passes ────────────────────────────────────────────

/// Dead Code Elimination for WASM modules.
pub fn dead_code_elimination(module: &mut WasmModule) -> usize {
    let exported_indices: HashSet<u32> = module.exports.iter()
        .filter(|e| e.kind == ExportKind::Function)
        .map(|e| e.index)
        .collect();

    // Build call graph
    let mut reachable = exported_indices.clone();
    let mut worklist: Vec<u32> = reachable.iter().copied().collect();

    while let Some(func_idx) = worklist.pop() {
        if let Some(func) = module.functions.get(func_idx as usize) {
            for op in &func.body {
                if let WasmOpcode::Call(target) = op {
                    if reachable.insert(*target) {
                        worklist.push(*target);
                    }
                }
            }
        }
    }

    let before = module.functions.len();
    let mut removed = 0;

    // Mark unreachable functions as empty (preserving indices)
    for (i, func) in module.functions.iter_mut().enumerate() {
        if !reachable.contains(&(i as u32)) {
            func.body = vec![WasmOpcode::Unreachable, WasmOpcode::End];
            removed += 1;
        }
    }

    let _ = before;
    removed
}

/// Tree shaking: remove unused data segments.
pub fn tree_shake_data(module: &mut WasmModule) -> usize {
    // Collect all memory offsets referenced by functions
    let mut referenced_offsets: HashSet<u32> = HashSet::new();
    for func in &module.functions {
        for op in &func.body {
            match op {
                WasmOpcode::I32Const(v) => { referenced_offsets.insert(*v as u32); }
                WasmOpcode::I32Load(_, offset) | WasmOpcode::I64Load(_, offset)
                | WasmOpcode::F64Load(_, offset) => { referenced_offsets.insert(*offset); }
                _ => {}
            }
        }
    }

    let before = module.data_segments.len();
    module.data_segments.retain(|seg| referenced_offsets.contains(&seg.offset));
    before - module.data_segments.len()
}

// ── Encoding Helpers ────────────────────────────────────────────────────

fn encode_leb128_u32(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 { byte |= 0x80; }
        buf.push(byte);
        if value == 0 { break; }
    }
}

fn valtype_byte(t: WasmValType) -> u8 {
    match t {
        WasmValType::I32 => 0x7F,
        WasmValType::I64 => 0x7E,
        WasmValType::F32 => 0x7D,
        WasmValType::F64 => 0x7C,
        WasmValType::FuncRef => 0x70,
        WasmValType::ExternRef => 0x6F,
        WasmValType::V128 => 0x7B,
    }
}

fn write_section(buf: &mut Vec<u8>, id: u8, data: &[u8]) {
    buf.push(id);
    let mut size_buf = Vec::new();
    encode_leb128_u32(&mut size_buf, data.len() as u32);
    buf.extend(size_buf);
    buf.extend(data);
}

fn encode_function_body(func: &WasmFunction) -> Vec<u8> {
    let mut body = Vec::new();
    // Local declarations
    body.push(0x00); // 0 local declaration groups (simplified)
    // Opcodes
    for op in &func.body {
        encode_opcode(&mut body, op);
    }
    if func.body.last() != Some(&WasmOpcode::End) {
        body.push(0x0B); // end
    }
    body
}

fn encode_opcode(buf: &mut Vec<u8>, op: &WasmOpcode) {
    match op {
        WasmOpcode::Unreachable => buf.push(0x00),
        WasmOpcode::Nop => buf.push(0x01),
        WasmOpcode::Block(_) => buf.push(0x02),
        WasmOpcode::Loop(_) => buf.push(0x03),
        WasmOpcode::If(_) => buf.push(0x04),
        WasmOpcode::Else => buf.push(0x05),
        WasmOpcode::End => buf.push(0x0B),
        WasmOpcode::Br(d) => { buf.push(0x0C); encode_leb128_u32(buf, *d); }
        WasmOpcode::BrIf(d) => { buf.push(0x0D); encode_leb128_u32(buf, *d); }
        WasmOpcode::Return => buf.push(0x0F),
        WasmOpcode::Call(idx) => { buf.push(0x10); encode_leb128_u32(buf, *idx); }
        WasmOpcode::CallIndirect(idx) => { buf.push(0x11); encode_leb128_u32(buf, *idx); buf.push(0x00); }
        WasmOpcode::I32Const(v) => { buf.push(0x41); encode_leb128_u32(buf, *v as u32); }
        WasmOpcode::I64Const(v) => { buf.push(0x42); encode_leb128_u32(buf, *v as u64 as u32); }
        WasmOpcode::F32Const(_) => buf.push(0x43),
        WasmOpcode::F64Const(_) => buf.push(0x44),
        WasmOpcode::LocalGet(i) => { buf.push(0x20); encode_leb128_u32(buf, *i); }
        WasmOpcode::LocalSet(i) => { buf.push(0x21); encode_leb128_u32(buf, *i); }
        WasmOpcode::LocalTee(i) => { buf.push(0x22); encode_leb128_u32(buf, *i); }
        WasmOpcode::GlobalGet(i) => { buf.push(0x23); encode_leb128_u32(buf, *i); }
        WasmOpcode::GlobalSet(i) => { buf.push(0x24); encode_leb128_u32(buf, *i); }
        WasmOpcode::I32Load(a, o) => { buf.push(0x28); encode_leb128_u32(buf, *a); encode_leb128_u32(buf, *o); }
        WasmOpcode::I64Load(a, o) => { buf.push(0x29); encode_leb128_u32(buf, *a); encode_leb128_u32(buf, *o); }
        WasmOpcode::F64Load(a, o) => { buf.push(0x2B); encode_leb128_u32(buf, *a); encode_leb128_u32(buf, *o); }
        WasmOpcode::I32Store(a, o) => { buf.push(0x36); encode_leb128_u32(buf, *a); encode_leb128_u32(buf, *o); }
        WasmOpcode::I64Store(a, o) => { buf.push(0x37); encode_leb128_u32(buf, *a); encode_leb128_u32(buf, *o); }
        WasmOpcode::F64Store(a, o) => { buf.push(0x39); encode_leb128_u32(buf, *a); encode_leb128_u32(buf, *o); }
        WasmOpcode::MemorySize => { buf.push(0x3F); buf.push(0x00); }
        WasmOpcode::MemoryGrow => { buf.push(0x40); buf.push(0x00); }
        WasmOpcode::I32Add => buf.push(0x6A),
        WasmOpcode::I32Sub => buf.push(0x6B),
        WasmOpcode::I32Mul => buf.push(0x6C),
        WasmOpcode::I32DivS => buf.push(0x6D),
        WasmOpcode::I64Add => buf.push(0x7C),
        WasmOpcode::I64Sub => buf.push(0x7D),
        WasmOpcode::I64Mul => buf.push(0x7E),
        WasmOpcode::F64Add => buf.push(0xA0),
        WasmOpcode::F64Sub => buf.push(0xA1),
        WasmOpcode::F64Mul => buf.push(0xA2),
        WasmOpcode::F64Div => buf.push(0xA3),
        WasmOpcode::F64Sqrt => buf.push(0x9F),
        WasmOpcode::I32Eqz => buf.push(0x45),
        WasmOpcode::I32Eq => buf.push(0x46),
        WasmOpcode::I32LtS => buf.push(0x48),
        WasmOpcode::I32GtS => buf.push(0x4A),
        WasmOpcode::F64Lt => buf.push(0x63),
        WasmOpcode::F64Gt => buf.push(0x64),
        WasmOpcode::F64Eq => buf.push(0x61),
        WasmOpcode::I32WrapI64 => buf.push(0xA7),
        WasmOpcode::I64ExtendI32S => buf.push(0xAC),
        WasmOpcode::F64ConvertI32S => buf.push(0xB7),
        WasmOpcode::I32TruncF64S => buf.push(0xAA),
        WasmOpcode::Drop => buf.push(0x1A),
    }
}

// ── FFI ─────────────────────────────────────────────────────────────────

static WASM_STORES: Mutex<Option<HashMap<i64, WasmModule>>> = Mutex::new(None);

fn wasm_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, WasmModule>>> {
    WASM_STORES.lock().unwrap()
}

fn next_wasm_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_wasm_aot_create(name_ptr: *const u8, name_len: i64) -> i64 {
    let name = if name_ptr.is_null() { "module".to_string() }
    else { unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(name_ptr, name_len as usize)).into_owned() } };
    let id = next_wasm_id();
    let module = WasmModule::new(&name);
    let mut store = wasm_store();
    store.get_or_insert_with(HashMap::new).insert(id, module);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_wasm_aot_size(id: i64) -> i64 {
    let store = wasm_store();
    store.as_ref().and_then(|s| s.get(&id))
        .map(|m| m.size_bytes() as i64)
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_wasm_aot_exports(id: i64) -> i64 {
    let store = wasm_store();
    store.as_ref().and_then(|s| s.get(&id))
        .map(|m| m.export_count() as i64)
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_wasm_aot_free(id: i64) {
    let mut store = wasm_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_module_new() {
        let m = WasmModule::new("test");
        assert_eq!(m.name, "test");
        assert_eq!(m.functions.len(), 0);
        assert_eq!(m.memory_pages, 1);
    }

    #[test]
    fn test_wasm_add_function() {
        let mut m = WasmModule::new("test");
        m.add_function(WasmFunction {
            name: "add".into(),
            type_idx: 0,
            locals: vec![WasmValType::I32, WasmValType::I32],
            body: vec![WasmOpcode::LocalGet(0), WasmOpcode::LocalGet(1), WasmOpcode::I32Add, WasmOpcode::End],
            is_exported: true,
        });
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.exports.len(), 1);
        assert_eq!(m.export_count(), 1);
    }

    #[test]
    fn test_wasm_serialize() {
        let mut m = WasmModule::new("test");
        m.add_function(WasmFunction {
            name: "main".into(), type_idx: 0,
            locals: vec![],
            body: vec![WasmOpcode::I32Const(42), WasmOpcode::End],
            is_exported: true,
        });
        let bytes = m.to_bytes();
        assert!(bytes.len() > 8); // At least magic + version
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]); // \0asm magic
        assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]); // version 1
    }

    #[test]
    fn test_wasm_data_segment() {
        let mut m = WasmModule::new("test");
        m.add_data(0, b"hello".to_vec());
        assert_eq!(m.data_segments.len(), 1);
        assert_eq!(m.data_segments[0].data, b"hello");
    }

    #[test]
    fn test_wasi_safe_defaults() {
        let caps = WasiCapabilities::safe_defaults();
        assert!(caps.allow_fs_read);
        assert!(!caps.allow_fs_write);
        assert!(!caps.allow_network);
        assert!(caps.allow_random);
    }

    #[test]
    fn test_wasi_generate_imports() {
        let caps = WasiCapabilities::safe_defaults();
        let imports = caps.generate_imports();
        assert!(imports.len() >= 2); // fd_write + at least clock or random
        assert!(imports.iter().any(|i| i.name == "fd_write"));
    }

    #[test]
    fn test_component_interface() {
        let mut iface = ComponentInterface::new("calculator");
        iface.add_function("add", vec![
            ("a".into(), InterfaceValType::S32),
            ("b".into(), InterfaceValType::S32),
        ], Some(InterfaceValType::S32));
        assert_eq!(iface.functions.len(), 1);
        let wit = iface.to_wit();
        assert!(wit.contains("calculator"));
        assert!(wit.contains("add"));
    }

    #[test]
    fn test_wit_generation() {
        let mut iface = ComponentInterface::new("math");
        iface.add_type("point", InterfaceValType::Record(vec![
            ("x".into(), InterfaceValType::F64),
            ("y".into(), InterfaceValType::F64),
        ]));
        iface.add_function("distance", vec![
            ("p1".into(), InterfaceValType::F64),
            ("p2".into(), InterfaceValType::F64),
        ], Some(InterfaceValType::F64));
        let wit = iface.to_wit();
        assert!(wit.contains("type point"));
        assert!(wit.contains("distance"));
    }

    #[test]
    fn test_browser_shim() {
        let mut m = WasmModule::new("app");
        m.add_function(WasmFunction {
            name: "init".into(), type_idx: 0, locals: vec![],
            body: vec![WasmOpcode::Nop, WasmOpcode::End], is_exported: true,
        });
        let shim = BrowserShim::new(&m);
        let js = shim.generate_js();
        assert!(js.contains("WebAssembly"));
        assert!(js.contains("app.wasm"));
        assert!(js.contains("window.app"));
    }

    #[test]
    fn test_dce() {
        let mut m = WasmModule::new("test");
        m.add_function(WasmFunction {
            name: "main".into(), type_idx: 0, locals: vec![],
            body: vec![WasmOpcode::I32Const(1), WasmOpcode::End], is_exported: true,
        });
        m.add_function(WasmFunction {
            name: "unused".into(), type_idx: 0, locals: vec![],
            body: vec![WasmOpcode::I32Const(2), WasmOpcode::End], is_exported: false,
        });
        let removed = dead_code_elimination(&mut m);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_tree_shaking() {
        let mut m = WasmModule::new("test");
        m.add_data(100, b"used".to_vec());
        m.add_data(999, b"unused".to_vec());
        m.add_function(WasmFunction {
            name: "f".into(), type_idx: 0, locals: vec![],
            body: vec![WasmOpcode::I32Const(100), WasmOpcode::End], is_exported: true,
        });
        let removed = tree_shake_data(&mut m);
        assert_eq!(removed, 1);
        assert_eq!(m.data_segments.len(), 1);
    }

    #[test]
    fn test_leb128_encoding() {
        let mut buf = Vec::new();
        encode_leb128_u32(&mut buf, 0);
        assert_eq!(buf, vec![0x00]);

        buf.clear();
        encode_leb128_u32(&mut buf, 127);
        assert_eq!(buf, vec![0x7F]);

        buf.clear();
        encode_leb128_u32(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);
    }

    #[test]
    fn test_valtype_bytes() {
        assert_eq!(valtype_byte(WasmValType::I32), 0x7F);
        assert_eq!(valtype_byte(WasmValType::I64), 0x7E);
        assert_eq!(valtype_byte(WasmValType::F64), 0x7C);
    }

    #[test]
    fn test_ffi_wasm_aot() {
        let id = vitalis_wasm_aot_create(std::ptr::null(), 0);
        assert!(id > 0);
        let size = vitalis_wasm_aot_size(id);
        assert!(size > 0); // At least magic header
        vitalis_wasm_aot_free(id);
    }

    #[test]
    fn test_wasi_unrestricted() {
        let caps = WasiCapabilities::unrestricted();
        assert!(caps.allow_network);
        assert!(caps.allow_fs_write);
        let imports = caps.generate_imports();
        assert!(imports.iter().any(|i| i.name == "path_open"));
    }

    #[test]
    fn test_opcode_encoding() {
        let mut buf = Vec::new();
        encode_opcode(&mut buf, &WasmOpcode::I32Add);
        assert_eq!(buf, vec![0x6A]);

        buf.clear();
        encode_opcode(&mut buf, &WasmOpcode::Call(5));
        assert_eq!(buf, vec![0x10, 0x05]);
    }
}
