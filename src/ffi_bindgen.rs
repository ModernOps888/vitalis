//! Multi-Language FFI & Binding Generation
//!
//! Provides C ABI layout computation, C header generation from Vitalis types,
//! calling convention support (C, stdcall, fastcall, System V, Win64),
//! type marshaling, dynamic library interfaces, and TypeScript definition
//! generation for WASM exports.

use std::collections::HashMap;

// ── ABI Types ────────────────────────────────────────────────────────

/// Supported calling conventions for FFI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    /// C default (cdecl on x86, System V on x86-64 Unix, Win64 on Windows).
    C,
    /// Windows stdcall (callee cleans stack).
    Stdcall,
    /// fastcall (first 2 args in registers on x86).
    Fastcall,
    /// System V AMD64 ABI (6 integer regs, 8 SSE regs).
    SystemV,
    /// Windows x64 ABI (4 register args, shadow space).
    Win64,
}

impl CallingConvention {
    /// Number of register-passed integer arguments.
    pub fn integer_arg_registers(&self) -> usize {
        match self {
            Self::C | Self::SystemV => 6,
            Self::Win64 => 4,
            Self::Fastcall => 2,
            Self::Stdcall => 0,
        }
    }

    /// Number of register-passed float/SSE arguments.
    pub fn float_arg_registers(&self) -> usize {
        match self {
            Self::SystemV => 8,
            Self::Win64 => 4,
            Self::C => 8,
            _ => 0,
        }
    }

    /// C name for the calling convention.
    pub fn c_attribute(&self) -> &str {
        match self {
            Self::C => "",
            Self::Stdcall => "__attribute__((stdcall))",
            Self::Fastcall => "__attribute__((fastcall))",
            Self::SystemV => "__attribute__((sysv_abi))",
            Self::Win64 => "__attribute__((ms_abi))",
        }
    }
}

// ── C Type System ────────────────────────────────────────────────────

/// Representation of a C type for ABI layout computation.
#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Void,
    Bool,
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
    Float,
    Double,
    /// Fixed-size integer types: i8, u8, i16, u16, i32, u32, i64, u64.
    SizedInt { bits: u32, signed: bool },
    /// Pointer to another type.
    Pointer(Box<CType>),
    /// Fixed-size array.
    Array(Box<CType>, usize),
    /// Struct with named fields.
    Struct(CStruct),
    /// Union with named fields.
    Union(CUnion),
    /// Enum (treated as int).
    Enum(String),
    /// Function pointer.
    FunctionPtr {
        return_type: Box<CType>,
        params: Vec<CType>,
        convention: CallingConvention,
    },
    /// Opaque type (forward declaration).
    Opaque(String),
}

impl CType {
    /// Size in bytes (assuming LP64: long=8, pointer=8).
    pub fn size(&self) -> usize {
        match self {
            Self::Void => 0,
            Self::Bool | Self::Char | Self::UChar => 1,
            Self::Short | Self::UShort => 2,
            Self::Int | Self::UInt | Self::Enum(_) => 4,
            Self::Long | Self::ULong | Self::LongLong | Self::ULongLong => 8,
            Self::Float => 4,
            Self::Double => 8,
            Self::SizedInt { bits, .. } => (*bits as usize + 7) / 8,
            Self::Pointer(_) | Self::FunctionPtr { .. } => 8,
            Self::Array(elem, count) => elem.size() * count,
            Self::Struct(s) => s.layout().size,
            Self::Union(u) => u.layout().size,
            Self::Opaque(_) => 0,
        }
    }

    /// Natural alignment in bytes.
    pub fn alignment(&self) -> usize {
        match self {
            Self::Void | Self::Opaque(_) => 1,
            Self::Bool | Self::Char | Self::UChar => 1,
            Self::Short | Self::UShort => 2,
            Self::Int | Self::UInt | Self::Float | Self::Enum(_) => 4,
            Self::Long | Self::ULong | Self::LongLong | Self::ULongLong | Self::Double => 8,
            Self::SizedInt { bits, .. } => {
                let sz = (*bits as usize + 7) / 8;
                sz.next_power_of_two().min(8)
            }
            Self::Pointer(_) | Self::FunctionPtr { .. } => 8,
            Self::Array(elem, _) => elem.alignment(),
            Self::Struct(s) => s.layout().alignment,
            Self::Union(u) => u.layout().alignment,
        }
    }

    /// C type name as a string.
    pub fn c_name(&self) -> String {
        match self {
            Self::Void => "void".into(),
            Self::Bool => "_Bool".into(),
            Self::Char => "char".into(),
            Self::UChar => "unsigned char".into(),
            Self::Short => "short".into(),
            Self::UShort => "unsigned short".into(),
            Self::Int => "int".into(),
            Self::UInt => "unsigned int".into(),
            Self::Long => "long".into(),
            Self::ULong => "unsigned long".into(),
            Self::LongLong => "long long".into(),
            Self::ULongLong => "unsigned long long".into(),
            Self::Float => "float".into(),
            Self::Double => "double".into(),
            Self::SizedInt { bits, signed } => {
                if *signed {
                    format!("int{}_t", bits)
                } else {
                    format!("uint{}_t", bits)
                }
            }
            Self::Pointer(inner) => format!("{}*", inner.c_name()),
            Self::Array(elem, count) => format!("{}[{}]", elem.c_name(), count),
            Self::Struct(s) => format!("struct {}", s.name),
            Self::Union(u) => format!("union {}", u.name),
            Self::Enum(name) => format!("enum {}", name),
            Self::FunctionPtr { return_type, params, .. } => {
                let params_str: Vec<_> = params.iter().map(|p| p.c_name()).collect();
                format!("{}(*)({})", return_type.c_name(), params_str.join(", "))
            }
            Self::Opaque(name) => format!("struct {}", name),
        }
    }

    /// Convert to TypeScript type name (for WASM interop).
    pub fn ts_name(&self) -> String {
        match self {
            Self::Void => "void".into(),
            Self::Bool => "boolean".into(),
            Self::Char | Self::UChar | Self::Short | Self::UShort
            | Self::Int | Self::UInt | Self::Long | Self::ULong
            | Self::LongLong | Self::ULongLong | Self::Float | Self::Double
            | Self::SizedInt { .. } | Self::Enum(_) => "number".into(),
            Self::Pointer(inner) if matches!(inner.as_ref(), CType::Char) => "string".into(),
            Self::Pointer(_) => "number".into(), // Pointer as i32/i64 in WASM
            Self::Array(elem, _) => format!("{}[]", elem.ts_name()),
            Self::Struct(s) => s.name.clone(),
            Self::Union(u) => u.name.clone(),
            Self::FunctionPtr { return_type, params, .. } => {
                let ps: Vec<_> = params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| format!("arg{}: {}", i, p.ts_name()))
                    .collect();
                format!("({}) => {}", ps.join(", "), return_type.ts_name())
            }
            Self::Opaque(_) => "number".into(),
        }
    }
}

// ── Struct Layout ────────────────────────────────────────────────────

/// C struct with field names and types.
#[derive(Debug, Clone, PartialEq)]
pub struct CStruct {
    pub name: String,
    pub fields: Vec<(String, CType)>,
    pub packed: bool,
}

/// Computed layout for a struct/union.
#[derive(Debug, Clone)]
pub struct Layout {
    pub size: usize,
    pub alignment: usize,
    /// (field_name, offset, size) for each field.
    pub field_offsets: Vec<(String, usize, usize)>,
    pub padding_bytes: usize,
}

impl CStruct {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
            packed: false,
        }
    }

    pub fn add_field(&mut self, name: &str, ty: CType) {
        self.fields.push((name.to_string(), ty));
    }

    /// Compute struct layout with proper C ABI padding.
    pub fn layout(&self) -> Layout {
        let mut offset = 0usize;
        let mut max_align = 1usize;
        let mut field_offsets = Vec::new();
        let mut padding = 0usize;

        for (name, ty) in &self.fields {
            let field_align = if self.packed { 1 } else { ty.alignment() };
            let field_size = ty.size();

            // Align offset
            let aligned_offset = (offset + field_align - 1) & !(field_align - 1);
            padding += aligned_offset - offset;
            offset = aligned_offset;

            field_offsets.push((name.clone(), offset, field_size));
            offset += field_size;

            if field_align > max_align {
                max_align = field_align;
            }
        }

        // Final padding for struct alignment
        let struct_align = if self.packed { 1 } else { max_align };
        let final_size = (offset + struct_align - 1) & !(struct_align - 1);
        padding += final_size - offset;

        Layout {
            size: final_size,
            alignment: struct_align,
            field_offsets,
            padding_bytes: padding,
        }
    }
}

/// C union (overlapping fields, size = max field size).
#[derive(Debug, Clone, PartialEq)]
pub struct CUnion {
    pub name: String,
    pub fields: Vec<(String, CType)>,
}

impl CUnion {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
        }
    }

    pub fn add_field(&mut self, name: &str, ty: CType) {
        self.fields.push((name.to_string(), ty));
    }

    pub fn layout(&self) -> Layout {
        let mut max_size = 0usize;
        let mut max_align = 1usize;
        let mut field_offsets = Vec::new();

        for (name, ty) in &self.fields {
            let sz = ty.size();
            let al = ty.alignment();
            field_offsets.push((name.clone(), 0, sz));
            max_size = max_size.max(sz);
            max_align = max_align.max(al);
        }

        let final_size = (max_size + max_align - 1) & !(max_align - 1);

        Layout {
            size: final_size,
            alignment: max_align,
            field_offsets,
            padding_bytes: final_size - max_size,
        }
    }
}

// ── Type Marshaling ──────────────────────────────────────────────────

/// Vitalis ↔ C type mapping.
#[derive(Debug, Clone)]
pub struct TypeMarshal {
    pub vitalis_type: String,
    pub c_type: CType,
    pub conversion: MarshalConversion,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarshalConversion {
    /// Direct bitcast, no conversion needed.
    Identity,
    /// Integer widening (e.g., i32 → i64).
    Widen,
    /// Integer narrowing (e.g., i64 → i32).
    Narrow,
    /// String to/from C string (null-terminated).
    StringConvert,
    /// Boolean to/from C _Bool.
    BoolConvert,
    /// Struct layout conversion.
    StructConvert,
    /// Pointer wrapping.
    PointerWrap,
}

/// Registry of type marshaling rules.
#[derive(Debug, Clone, Default)]
pub struct MarshalRegistry {
    pub rules: HashMap<String, TypeMarshal>,
}

impl MarshalRegistry {
    pub fn new() -> Self {
        let mut reg = Self::default();
        // Built-in mappings
        reg.register("i32", CType::Int, MarshalConversion::Identity);
        reg.register("i64", CType::LongLong, MarshalConversion::Identity);
        reg.register("f32", CType::Float, MarshalConversion::Identity);
        reg.register("f64", CType::Double, MarshalConversion::Identity);
        reg.register("bool", CType::Bool, MarshalConversion::BoolConvert);
        reg.register(
            "str",
            CType::Pointer(Box::new(CType::Char)),
            MarshalConversion::StringConvert,
        );
        reg
    }

    pub fn register(&mut self, vitalis_type: &str, c_type: CType, conversion: MarshalConversion) {
        self.rules.insert(
            vitalis_type.to_string(),
            TypeMarshal {
                vitalis_type: vitalis_type.to_string(),
                c_type,
                conversion,
            },
        );
    }

    pub fn lookup(&self, vitalis_type: &str) -> Option<&TypeMarshal> {
        self.rules.get(vitalis_type)
    }

    /// Check if a Vitalis type can be marshaled to C.
    pub fn can_marshal(&self, vitalis_type: &str) -> bool {
        self.rules.contains_key(vitalis_type)
    }
}

// ── C Header Generation ──────────────────────────────────────────────

/// FFI function declaration.
#[derive(Debug, Clone)]
pub struct FfiFunction {
    pub name: String,
    pub return_type: CType,
    pub params: Vec<(String, CType)>,
    pub convention: CallingConvention,
    pub doc: Option<String>,
}

/// C header generator — produces `.h` files from Vitalis type declarations.
#[derive(Debug, Clone)]
pub struct HeaderGenerator {
    pub guard: String,
    pub includes: Vec<String>,
    pub structs: Vec<CStruct>,
    pub unions: Vec<CUnion>,
    pub functions: Vec<FfiFunction>,
    pub typedefs: Vec<(String, CType)>,
    pub enums: Vec<CEnum>,
}

/// C enum definition.
#[derive(Debug, Clone)]
pub struct CEnum {
    pub name: String,
    pub variants: Vec<(String, i64)>,
}

impl HeaderGenerator {
    pub fn new(guard: &str) -> Self {
        Self {
            guard: guard.to_string(),
            includes: vec!["<stdint.h>".into(), "<stdbool.h>".into()],
            structs: Vec::new(),
            unions: Vec::new(),
            functions: Vec::new(),
            typedefs: Vec::new(),
            enums: Vec::new(),
        }
    }

    pub fn add_struct(&mut self, s: CStruct) {
        self.structs.push(s);
    }

    pub fn add_union(&mut self, u: CUnion) {
        self.unions.push(u);
    }

    pub fn add_function(&mut self, f: FfiFunction) {
        self.functions.push(f);
    }

    pub fn add_typedef(&mut self, alias: &str, ty: CType) {
        self.typedefs.push((alias.to_string(), ty));
    }

    pub fn add_enum(&mut self, e: CEnum) {
        self.enums.push(e);
    }

    /// Generate the complete C header.
    pub fn generate(&self) -> String {
        let mut out = String::new();

        // Header guard
        out.push_str(&format!("#ifndef {}\n#define {}\n\n", self.guard, self.guard));

        // Includes
        for inc in &self.includes {
            out.push_str(&format!("#include {}\n", inc));
        }
        out.push('\n');

        // Extern C
        out.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");

        // Typedefs
        for (alias, ty) in &self.typedefs {
            out.push_str(&format!("typedef {} {};\n", ty.c_name(), alias));
        }
        if !self.typedefs.is_empty() {
            out.push('\n');
        }

        // Enums
        for e in &self.enums {
            out.push_str(&format!("typedef enum {{\n"));
            for (i, (name, val)) in e.variants.iter().enumerate() {
                out.push_str(&format!("    {} = {}", name, val));
                if i < e.variants.len() - 1 {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&format!("}} {};\n\n", e.name));
        }

        // Structs
        for s in &self.structs {
            if s.packed {
                out.push_str("#pragma pack(push, 1)\n");
            }
            out.push_str(&format!("typedef struct {{\n"));
            for (fname, ftype) in &s.fields {
                out.push_str(&format!("    {} {};\n", ftype.c_name(), fname));
            }
            out.push_str(&format!("}} {};\n", s.name));
            if s.packed {
                out.push_str("#pragma pack(pop)\n");
            }
            out.push('\n');
        }

        // Unions
        for u in &self.unions {
            out.push_str(&format!("typedef union {{\n"));
            for (fname, ftype) in &u.fields {
                out.push_str(&format!("    {} {};\n", ftype.c_name(), fname));
            }
            out.push_str(&format!("}} {};\n\n", u.name));
        }

        // Functions
        for f in &self.functions {
            if let Some(doc) = &f.doc {
                out.push_str(&format!("/* {} */\n", doc));
            }
            let params: Vec<String> = if f.params.is_empty() {
                vec!["void".to_string()]
            } else {
                f.params
                    .iter()
                    .map(|(name, ty)| format!("{} {}", ty.c_name(), name))
                    .collect()
            };
            let cc = f.convention.c_attribute();
            if cc.is_empty() {
                out.push_str(&format!(
                    "{} {}({});\n",
                    f.return_type.c_name(),
                    f.name,
                    params.join(", ")
                ));
            } else {
                out.push_str(&format!(
                    "{} {} {}({});\n",
                    cc,
                    f.return_type.c_name(),
                    f.name,
                    params.join(", ")
                ));
            }
        }

        out.push_str("\n#ifdef __cplusplus\n}\n#endif\n\n");
        out.push_str(&format!("#endif /* {} */\n", self.guard));
        out
    }
}

// ── TypeScript Definition Generator ──────────────────────────────────

/// Generates TypeScript `.d.ts` definitions for WASM exports.
#[derive(Debug, Clone, Default)]
pub struct TsDefinitionGenerator {
    pub module_name: String,
    pub functions: Vec<FfiFunction>,
    pub interfaces: Vec<TsInterface>,
}

/// TypeScript interface (maps from C struct).
#[derive(Debug, Clone)]
pub struct TsInterface {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

impl TsDefinitionGenerator {
    pub fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            functions: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    pub fn add_function(&mut self, f: FfiFunction) {
        self.functions.push(f);
    }

    pub fn add_interface(&mut self, name: &str, fields: Vec<(String, CType)>) {
        let ts_fields = fields
            .iter()
            .map(|(n, t)| (n.clone(), t.ts_name()))
            .collect();
        self.interfaces.push(TsInterface {
            name: name.to_string(),
            fields: ts_fields,
        });
    }

    /// Generate TypeScript declarations.
    pub fn generate(&self) -> String {
        let mut out = format!(
            "// Auto-generated TypeScript definitions for {}\n\n",
            self.module_name
        );

        // Interfaces
        for iface in &self.interfaces {
            out.push_str(&format!("export interface {} {{\n", iface.name));
            for (name, ty) in &iface.fields {
                out.push_str(&format!("  {}: {};\n", name, ty));
            }
            out.push_str("}\n\n");
        }

        // Functions
        out.push_str(&format!("export interface {} {{\n", self.module_name));
        for f in &self.functions {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|(name, ty)| format!("{}: {}", name, ty.ts_name()))
                .collect();
            out.push_str(&format!(
                "  {}({}): {};\n",
                f.name,
                params.join(", "),
                f.return_type.ts_name()
            ));
        }
        out.push_str("}\n");

        out
    }
}

// ── Dynamic Library Interface ────────────────────────────────────────

/// Represents a dynamically-loaded foreign library.
#[derive(Debug, Clone)]
pub struct DynLibInterface {
    pub name: String,
    pub path: String,
    pub symbols: Vec<FfiFunction>,
}

impl DynLibInterface {
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            symbols: Vec::new(),
        }
    }

    pub fn add_symbol(&mut self, f: FfiFunction) {
        self.symbols.push(f);
    }

    /// Generate Vitalis-side extern block declaration.
    pub fn to_extern_block(&self) -> String {
        let mut out = format!("// Bindings for {}\n", self.name);
        out.push_str(&format!("extern \"{}\" {{\n", self.path));
        for f in &self.symbols {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|(name, ty)| format!("{}: {}", name, ty.c_name()))
                .collect();
            out.push_str(&format!(
                "    fn {}({}) -> {};\n",
                f.name,
                params.join(", "),
                f.return_type.c_name()
            ));
        }
        out.push_str("}\n");
        out
    }

    /// Generate ctypes-based Python loader.
    pub fn to_python_loader(&self) -> String {
        let mut out = format!(
            "import ctypes\nimport os\n\n# Auto-generated bindings for {}\n\n",
            self.name
        );
        out.push_str(&format!(
            "lib = ctypes.CDLL(os.path.join(os.path.dirname(__file__), \"{}\"))\n\n",
            self.path
        ));

        for f in &self.symbols {
            // argtypes
            let argtypes: Vec<String> = f.params.iter().map(|(_, ty)| c_to_ctypes(ty)).collect();
            out.push_str(&format!(
                "lib.{}.argtypes = [{}]\n",
                f.name,
                argtypes.join(", ")
            ));
            out.push_str(&format!(
                "lib.{}.restype = {}\n\n",
                f.name,
                c_to_ctypes(&f.return_type)
            ));
        }
        out
    }
}

/// Map CType to Python ctypes type name.
fn c_to_ctypes(ty: &CType) -> String {
    match ty {
        CType::Void => "None".into(),
        CType::Bool => "ctypes.c_bool".into(),
        CType::Char => "ctypes.c_char".into(),
        CType::Int => "ctypes.c_int".into(),
        CType::UInt => "ctypes.c_uint".into(),
        CType::Long => "ctypes.c_long".into(),
        CType::LongLong => "ctypes.c_longlong".into(),
        CType::Float => "ctypes.c_float".into(),
        CType::Double => "ctypes.c_double".into(),
        CType::Pointer(inner) if matches!(inner.as_ref(), CType::Char) => {
            "ctypes.c_void_p".into() // Never c_char_p for returned strings
        }
        CType::Pointer(_) => "ctypes.c_void_p".into(),
        CType::SizedInt { bits: 32, signed: true } => "ctypes.c_int32".into(),
        CType::SizedInt { bits: 64, signed: true } => "ctypes.c_int64".into(),
        CType::SizedInt { bits: 32, signed: false } => "ctypes.c_uint32".into(),
        CType::SizedInt { bits: 64, signed: false } => "ctypes.c_uint64".into(),
        _ => "ctypes.c_void_p".into(),
    }
}

// ── C++ Name Mangling ────────────────────────────────────────────────

/// Itanium C++ name mangling (used on Linux/macOS).
pub struct CppMangler;

impl CppMangler {
    /// Mangle a simple function name (no namespace, no overloads).
    pub fn mangle_function(name: &str, params: &[CType]) -> String {
        let mut mangled = format!("_Z{}{}", name.len(), name);
        if params.is_empty() {
            mangled.push('v');
        } else {
            for p in params {
                mangled.push_str(&Self::mangle_type(p));
            }
        }
        mangled
    }

    /// Mangle a type to Itanium encoding.
    pub fn mangle_type(ty: &CType) -> String {
        match ty {
            CType::Void => "v".into(),
            CType::Bool => "b".into(),
            CType::Char => "c".into(),
            CType::UChar => "h".into(),
            CType::Short => "s".into(),
            CType::UShort => "t".into(),
            CType::Int => "i".into(),
            CType::UInt => "j".into(),
            CType::Long => "l".into(),
            CType::ULong => "m".into(),
            CType::LongLong => "x".into(),
            CType::ULongLong => "y".into(),
            CType::Float => "f".into(),
            CType::Double => "d".into(),
            CType::Pointer(inner) => format!("P{}", Self::mangle_type(inner)),
            _ => "v".into(),
        }
    }

    /// Demangle an Itanium mangled name (simple cases).
    pub fn demangle(mangled: &str) -> Option<String> {
        if !mangled.starts_with("_Z") {
            return None;
        }
        let rest = &mangled[2..];
        // Extract name length
        let len_end = rest.find(|c: char| !c.is_ascii_digit())?;
        let name_len: usize = rest[..len_end].parse().ok()?;
        let name = &rest[len_end..len_end + name_len];
        Some(name.to_string())
    }
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── CType ────────────────────────────────────────────────────────

    #[test]
    fn test_ctype_sizes() {
        assert_eq!(CType::Void.size(), 0);
        assert_eq!(CType::Bool.size(), 1);
        assert_eq!(CType::Char.size(), 1);
        assert_eq!(CType::Short.size(), 2);
        assert_eq!(CType::Int.size(), 4);
        assert_eq!(CType::LongLong.size(), 8);
        assert_eq!(CType::Float.size(), 4);
        assert_eq!(CType::Double.size(), 8);
        assert_eq!(CType::Pointer(Box::new(CType::Int)).size(), 8);
    }

    #[test]
    fn test_ctype_alignment() {
        assert_eq!(CType::Char.alignment(), 1);
        assert_eq!(CType::Short.alignment(), 2);
        assert_eq!(CType::Int.alignment(), 4);
        assert_eq!(CType::Double.alignment(), 8);
    }

    #[test]
    fn test_ctype_c_name() {
        assert_eq!(CType::Int.c_name(), "int");
        assert_eq!(CType::Double.c_name(), "double");
        assert_eq!(
            CType::Pointer(Box::new(CType::Char)).c_name(),
            "char*"
        );
        assert_eq!(
            CType::SizedInt { bits: 32, signed: true }.c_name(),
            "int32_t"
        );
    }

    #[test]
    fn test_ctype_ts_name() {
        assert_eq!(CType::Int.ts_name(), "number");
        assert_eq!(CType::Bool.ts_name(), "boolean");
        assert_eq!(CType::Void.ts_name(), "void");
        assert_eq!(
            CType::Pointer(Box::new(CType::Char)).ts_name(),
            "string"
        );
    }

    #[test]
    fn test_ctype_array() {
        let arr = CType::Array(Box::new(CType::Int), 10);
        assert_eq!(arr.size(), 40);
        assert_eq!(arr.alignment(), 4);
    }

    // ── Struct Layout ────────────────────────────────────────────────

    #[test]
    fn test_struct_layout_basic() {
        let mut s = CStruct::new("Point");
        s.add_field("x", CType::Float);
        s.add_field("y", CType::Float);
        let layout = s.layout();
        assert_eq!(layout.size, 8);
        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.field_offsets[0], ("x".into(), 0, 4));
        assert_eq!(layout.field_offsets[1], ("y".into(), 4, 4));
    }

    #[test]
    fn test_struct_layout_padding() {
        let mut s = CStruct::new("Padded");
        s.add_field("a", CType::Char);    // offset 0, size 1
        s.add_field("b", CType::Int);     // offset 4 (3 bytes padding), size 4
        s.add_field("c", CType::Char);    // offset 8, size 1
        let layout = s.layout();
        assert_eq!(layout.field_offsets[1].1, 4); // b at offset 4
        assert_eq!(layout.size, 12); // 8+1 → padded to 12
        assert!(layout.padding_bytes > 0);
    }

    #[test]
    fn test_struct_packed() {
        let mut s = CStruct::new("Packed");
        s.packed = true;
        s.add_field("a", CType::Char);
        s.add_field("b", CType::Int);
        let layout = s.layout();
        assert_eq!(layout.field_offsets[1].1, 1); // No padding
        assert_eq!(layout.size, 5);
    }

    // ── Union Layout ─────────────────────────────────────────────────

    #[test]
    fn test_union_layout() {
        let mut u = CUnion::new("Value");
        u.add_field("i", CType::Int);
        u.add_field("d", CType::Double);
        let layout = u.layout();
        assert_eq!(layout.size, 8); // max(4, 8) = 8
        assert_eq!(layout.alignment, 8);
        // All fields at offset 0
        assert_eq!(layout.field_offsets[0].1, 0);
        assert_eq!(layout.field_offsets[1].1, 0);
    }

    // ── Calling Convention ───────────────────────────────────────────

    #[test]
    fn test_calling_conventions() {
        assert_eq!(CallingConvention::SystemV.integer_arg_registers(), 6);
        assert_eq!(CallingConvention::Win64.integer_arg_registers(), 4);
        assert_eq!(CallingConvention::Fastcall.integer_arg_registers(), 2);
        assert_eq!(CallingConvention::SystemV.float_arg_registers(), 8);
    }

    #[test]
    fn test_cc_attribute() {
        assert_eq!(CallingConvention::C.c_attribute(), "");
        assert!(CallingConvention::Stdcall.c_attribute().contains("stdcall"));
    }

    // ── Type Marshaling ──────────────────────────────────────────────

    #[test]
    fn test_marshal_registry() {
        let reg = MarshalRegistry::new();
        assert!(reg.can_marshal("i32"));
        assert!(reg.can_marshal("f64"));
        assert!(reg.can_marshal("str"));
        assert!(!reg.can_marshal("Widget"));
    }

    #[test]
    fn test_marshal_lookup() {
        let reg = MarshalRegistry::new();
        let m = reg.lookup("i32").unwrap();
        assert_eq!(m.c_type, CType::Int);
        assert_eq!(m.conversion, MarshalConversion::Identity);
    }

    #[test]
    fn test_marshal_custom() {
        let mut reg = MarshalRegistry::new();
        reg.register("MyStruct", CType::Opaque("MyStruct".into()), MarshalConversion::StructConvert);
        assert!(reg.can_marshal("MyStruct"));
    }

    // ── Header Generation ────────────────────────────────────────────

    #[test]
    fn test_header_generation() {
        let mut hgen = HeaderGenerator::new("VITALIS_FFI_H");
        let mut s = CStruct::new("Point2D");
        s.add_field("x", CType::Double);
        s.add_field("y", CType::Double);
        hgen.add_struct(s);

        hgen.add_function(FfiFunction {
            name: "point_distance".into(),
            return_type: CType::Double,
            params: vec![
                ("a".into(), CType::Struct(CStruct::new("Point2D"))),
                ("b".into(), CType::Struct(CStruct::new("Point2D"))),
            ],
            convention: CallingConvention::C,
            doc: Some("Euclidean distance between two points".into()),
        });

        let header = hgen.generate();
        assert!(header.contains("#ifndef VITALIS_FFI_H"));
        assert!(header.contains("typedef struct"));
        assert!(header.contains("double x"));
        assert!(header.contains("point_distance"));
        assert!(header.contains("#endif"));
    }

    #[test]
    fn test_header_enum() {
        let mut hgen = HeaderGenerator::new("TEST_H");
        hgen.add_enum(CEnum {
            name: "Color".into(),
            variants: vec![
                ("RED".into(), 0),
                ("GREEN".into(), 1),
                ("BLUE".into(), 2),
            ],
        });
        let header = hgen.generate();
        assert!(header.contains("RED = 0"));
        assert!(header.contains("Color"));
    }

    #[test]
    fn test_header_typedef() {
        let mut hgen = HeaderGenerator::new("TEST_H");
        hgen.add_typedef("size_t", CType::ULongLong);
        let header = hgen.generate();
        assert!(header.contains("typedef unsigned long long size_t"));
    }

    // ── TypeScript Generation ────────────────────────────────────────

    #[test]
    fn test_ts_generation() {
        let mut ts = TsDefinitionGenerator::new("VitalisModule");
        ts.add_function(FfiFunction {
            name: "add".into(),
            return_type: CType::Int,
            params: vec![("a".into(), CType::Int), ("b".into(), CType::Int)],
            convention: CallingConvention::C,
            doc: None,
        });
        ts.add_interface("Result", vec![
            ("value".into(), CType::Int),
            ("error".into(), CType::Pointer(Box::new(CType::Char))),
        ]);
        let dts = ts.generate();
        assert!(dts.contains("export interface Result"));
        assert!(dts.contains("value: number"));
        assert!(dts.contains("error: string"));
        assert!(dts.contains("add(a: number, b: number): number"));
    }

    // ── Dynamic Library ──────────────────────────────────────────────

    #[test]
    fn test_dynlib_extern_block() {
        let mut lib = DynLibInterface::new("mathlib", "libmath.so");
        lib.add_symbol(FfiFunction {
            name: "fast_sqrt".into(),
            return_type: CType::Double,
            params: vec![("x".into(), CType::Double)],
            convention: CallingConvention::C,
            doc: None,
        });
        let block = lib.to_extern_block();
        assert!(block.contains("extern \"libmath.so\""));
        assert!(block.contains("fn fast_sqrt"));
    }

    #[test]
    fn test_dynlib_python_loader() {
        let mut lib = DynLibInterface::new("vitalis", "vitalis.dll");
        lib.add_symbol(FfiFunction {
            name: "compile_and_run".into(),
            return_type: CType::LongLong,
            params: vec![("src".into(), CType::Pointer(Box::new(CType::Char)))],
            convention: CallingConvention::C,
            doc: None,
        });
        let py = lib.to_python_loader();
        assert!(py.contains("ctypes.CDLL"));
        assert!(py.contains("ctypes.c_void_p")); // Not c_char_p!
        assert!(py.contains("ctypes.c_longlong"));
    }

    // ── C++ Name Mangling ────────────────────────────────────────────

    #[test]
    fn test_cpp_mangle() {
        let mangled = CppMangler::mangle_function("foo", &[CType::Int, CType::Double]);
        assert_eq!(mangled, "_Z3fooid");
    }

    #[test]
    fn test_cpp_mangle_void() {
        let mangled = CppMangler::mangle_function("bar", &[]);
        assert_eq!(mangled, "_Z3barv");
    }

    #[test]
    fn test_cpp_demangle() {
        assert_eq!(CppMangler::demangle("_Z3fooid"), Some("foo".into()));
        assert_eq!(CppMangler::demangle("_Z6myFuncv"), Some("myFunc".into()));
        assert_eq!(CppMangler::demangle("not_mangled"), None);
    }

    #[test]
    fn test_cpp_mangle_pointer() {
        let mangled = CppMangler::mangle_type(&CType::Pointer(Box::new(CType::Int)));
        assert_eq!(mangled, "Pi");
    }

    // ── ctypes mapping ───────────────────────────────────────────────

    #[test]
    fn test_c_to_ctypes() {
        assert_eq!(c_to_ctypes(&CType::Int), "ctypes.c_int");
        assert_eq!(c_to_ctypes(&CType::Double), "ctypes.c_double");
        assert_eq!(
            c_to_ctypes(&CType::Pointer(Box::new(CType::Char))),
            "ctypes.c_void_p"
        );
    }
}
