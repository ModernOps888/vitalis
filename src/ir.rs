//! Vitalis IR — SSA-based Intermediate Representation.
//!
//! Lowers the AST into a flat, linear IR suitable for Cranelift codegen.
//! Each function becomes a list of basic blocks, each block a list of
//! instructions operating on typed virtual registers.

use crate::ast::{self, BinOp, UnaryOp};
use crate::types::Type;
use std::collections::{HashMap, HashSet};
use std::fmt;

// ─── IR Values ──────────────────────────────────────────────────────────
/// A virtual register / SSA value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub u32);

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// A basic block label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

// ─── IR Types ───────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum IrType {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Ptr,    // Generic pointer (for strings, structs, etc.)
    Void,
}

impl IrType {
    pub fn from_type(ty: &Type) -> Self {
        match ty {
            Type::I32 => IrType::I32,
            Type::I64 => IrType::I64,
            Type::F32 => IrType::F32,
            Type::F64 => IrType::F64,
            Type::Bool => IrType::Bool,
            Type::Str => IrType::Ptr,
            Type::Void => IrType::Void,
            Type::Named(_) => IrType::Ptr,
            Type::List(_) => IrType::Ptr,
            Type::Map(_, _) => IrType::Ptr,
            Type::Option(_) => IrType::Ptr,
            Type::Result(_, _) => IrType::Ptr,
            Type::Future(_) => IrType::Ptr,
            Type::Function { .. } => IrType::Ptr,
            Type::Array(_, _) => IrType::Ptr,
            Type::Ref { .. } => IrType::Ptr,
            _ => IrType::I64, // Fallback
        }
    }

    /// Size in bytes on the target.
    pub fn byte_size(&self) -> u32 {
        match self {
            IrType::I32 | IrType::F32 | IrType::Bool => 4,
            IrType::I64 | IrType::F64 | IrType::Ptr => 8,
            IrType::Void => 0,
        }
    }
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrType::I32 => write!(f, "i32"),
            IrType::I64 => write!(f, "i64"),
            IrType::F32 => write!(f, "f32"),
            IrType::F64 => write!(f, "f64"),
            IrType::Bool => write!(f, "bool"),
            IrType::Ptr => write!(f, "ptr"),
            IrType::Void => write!(f, "void"),
        }
    }
}

// ─── Instructions ───────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum Inst {
    /// result = iconst value
    IConst { result: Value, value: i64, ty: IrType },
    /// result = fconst value
    FConst { result: Value, value: f64, ty: IrType },
    /// result = bconst true/false
    BConst { result: Value, value: bool },
    /// result = string_const "..."
    StrConst { result: Value, value: String },

    /// result = binop lhs, rhs
    BinOp { result: Value, op: IrBinOp, lhs: Value, rhs: Value, ty: IrType },
    /// result = unop operand
    UnOp { result: Value, op: IrUnOp, operand: Value, ty: IrType },
    /// result = icmp cond lhs, rhs
    ICmp { result: Value, cond: IrCmp, lhs: Value, rhs: Value },
    /// result = fcmp cond lhs, rhs
    FCmp { result: Value, cond: IrCmp, lhs: Value, rhs: Value },

    /// result = call func(args...)
    Call { result: Value, func: String, args: Vec<Value>, ret_ty: IrType },
    /// return value
    Return { value: Option<Value> },

    /// Unconditional branch
    Jump { target: BlockId },
    /// Conditional branch
    Branch { cond: Value, then_bb: BlockId, else_bb: BlockId },

    /// result = phi [(val, block), ...]
    Phi { result: Value, incoming: Vec<(Value, BlockId)>, ty: IrType },

    /// result = alloca size
    Alloca { result: Value, size: u32 },
    /// result = load ptr
    Load { result: Value, ptr: Value, ty: IrType },
    /// store value, ptr
    Store { value: Value, ptr: Value },

    /// Copy: result = source  (used during lowering)
    Copy { result: Value, source: Value },

    /// Nop — placeholder
    Nop,

    // ── Phase 4: Arrays ────────────────────────────────────────────────────
    /// Heap-allocate an array: layout = [i64 length][elem0..elemN].
    /// `count` is the element count; returns pointer to the data region.
    ArrayAlloc { result: Value, elem_ty: IrType, count: Value },
    /// Bounds-checked element load: result = array[index]
    ArrayGet { result: Value, array: Value, index: Value, elem_ty: IrType },
    /// Bounds-checked element store: array[index] = value
    ArraySet { array: Value, index: Value, value: Value, elem_ty: IrType },
    /// Read length header: result = *(array_ptr - 8) as i64
    ArrayLen { result: Value, array: Value },

    // ── Phase 5: Closures (scaffolding) ────────────────────────────────────
    /// Capture environment into a boxed closure record.
    ClosureAlloc { result: Value, func: String, captures: Vec<Value> },

    // ── Phase 6: Structs (scaffolding) ─────────────────────────────────────
    /// Heap-allocate a struct; `fields` are the initial field values in order.
    StructAlloc { result: Value, type_name: String, fields: Vec<Value> },
    /// Load a struct field at `field_index * 8` byte offset.
    FieldGet { result: Value, object: Value, field_index: u32, ty: IrType },
    /// Store a value to a struct field at `field_index * 8` byte offset.
    FieldSet { object: Value, field_index: u32, value: Value },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IrBinOp {
    Add, Sub, Mul, Div, Mod,
    FAdd, FSub, FMul, FDiv,
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IrUnOp {
    Neg, FNeg, Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IrCmp {
    Eq, Ne, Lt, Gt, Le, Ge,
}

// ─── Basic Block ────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub insts: Vec<Inst>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self { id, insts: Vec::new() }
    }
}

// ─── Function IR ────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub ret_type: IrType,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
}

impl fmt::Display for IrFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.name)?;
        for (i, (name, ty)) in self.params.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}: {}", name, ty)?;
        }
        writeln!(f, ") -> {} {{", self.ret_type)?;
        for block in &self.blocks {
            writeln!(f, "  {}:", block.id)?;
            for inst in &block.insts {
                writeln!(f, "    {:?}", inst)?;
            }
        }
        writeln!(f, "}}")
    }
}

// ─── Module IR ──────────────────────────────────────────────────────────
#[derive(Debug)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
    pub string_constants: Vec<String>,
}

impl IrModule {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            string_constants: Vec::new(),
        }
    }
}

// ─── IR Builder ─────────────────────────────────────────────────────────
pub struct IrBuilder {
    module: IrModule,
    // Current function state
    current_blocks: Vec<BasicBlock>,
    current_block: usize,
    next_value: u32,
    next_block: u32,
    /// Variable name → current Value (immutable: the value itself, mutable: the alloca ptr)
    locals: HashMap<String, Value>,
    /// Set of mutable variable names (use alloca/store/load)
    mutables: HashSet<String>,
    /// Function signatures for calls
    fn_sigs: HashMap<String, (Vec<IrType>, IrType)>,
    /// Type of each SSA value — enables float-aware binary op dispatch
    value_types: HashMap<Value, IrType>,
    /// Type of each mutable variable (for correct Load type on Ident)
    mutable_var_types: HashMap<String, IrType>,
    /// v18: Struct field layouts: struct_name → [(field_name, index)]
    struct_defs: HashMap<String, Vec<String>>,
    /// Method registry: type_name → { method_name → mangled_name }
    method_registry: HashMap<String, HashMap<String, String>>,
    /// Enum definitions: enum_name → [(variant_name, field_count)]
    enum_defs: HashMap<String, Vec<(String, usize)>>,
    /// Type aliases: alias_name → resolved IrType
    type_aliases: HashMap<String, IrType>,
    /// v18: Tracks which struct type each variable holds (for method dispatch)
    var_struct_types: HashMap<String, String>,
    /// v18: Loop context stack: (continue_bb, break_bb) for break/continue
    loop_stack: Vec<(BlockId, BlockId)>,
    /// Module prefix stack for name mangling (e.g. ["math"] → "math_")
    module_prefix: Vec<String>,
}

impl IrBuilder {
    pub fn new() -> Self {
        let mut fn_sigs: HashMap<String, (Vec<IrType>, IrType)> = HashMap::new();
        // Register stdlib builtin signatures so call return types are correct.
        // I/O (void return)
        for name in &["print", "println", "print_f64", "println_f64",
                       "print_bool", "println_bool", "print_str", "println_str"] {
            fn_sigs.insert(name.to_string(), (vec![IrType::I64], IrType::Void));
        }
        // Math f64 → f64
        for name in &["sqrt", "ln", "log2", "log10", "sin", "cos", "exp",
                       "floor", "ceil", "round", "abs_f64"] {
            fn_sigs.insert(name.to_string(), (vec![IrType::F64], IrType::F64));
        }
        fn_sigs.insert("pow".into(),     (vec![IrType::F64, IrType::F64], IrType::F64));
        fn_sigs.insert("min_f64".into(), (vec![IrType::F64, IrType::F64], IrType::F64));
        fn_sigs.insert("max_f64".into(), (vec![IrType::F64, IrType::F64], IrType::F64));
        // Math i64
        fn_sigs.insert("abs".into(),  (vec![IrType::I64], IrType::I64));
        fn_sigs.insert("min".into(),  (vec![IrType::I64, IrType::I64], IrType::I64));
        fn_sigs.insert("max".into(),  (vec![IrType::I64, IrType::I64], IrType::I64));
        // Conversions
        fn_sigs.insert("to_f64".into(),     (vec![IrType::I64], IrType::F64));
        fn_sigs.insert("to_i64".into(),     (vec![IrType::F64], IrType::I64));
        fn_sigs.insert("i64_to_f64".into(), (vec![IrType::I64], IrType::F64));
        fn_sigs.insert("f64_to_i64".into(), (vec![IrType::F64], IrType::I64));
        // Strings
        fn_sigs.insert("str_len".into(), (vec![IrType::Ptr], IrType::I64));
        fn_sigs.insert("str_eq".into(),  (vec![IrType::Ptr, IrType::Ptr], IrType::Bool));
        fn_sigs.insert("str_cat".into(), (vec![IrType::Ptr, IrType::Ptr], IrType::Ptr));
        // Extended math
        fn_sigs.insert("clamp_f64".into(), (vec![IrType::F64, IrType::F64, IrType::F64], IrType::F64));
        fn_sigs.insert("clamp_i64".into(), (vec![IrType::I64, IrType::I64, IrType::I64], IrType::I64));
        fn_sigs.insert("atan2".into(),     (vec![IrType::F64, IrType::F64], IrType::F64));
        fn_sigs.insert("hypot".into(),     (vec![IrType::F64, IrType::F64], IrType::F64));
        fn_sigs.insert("rand_f64".into(),  (vec![], IrType::F64));
        fn_sigs.insert("rand_i64".into(),  (vec![], IrType::I64));
        // Phase 4: Array builtins
        fn_sigs.insert("slang_array_alloc".into(),    (vec![IrType::I64, IrType::I64], IrType::Ptr));
        fn_sigs.insert("slang_array_get_i64".into(),  (vec![IrType::Ptr, IrType::I64], IrType::I64));
        fn_sigs.insert("slang_array_set_i64".into(),  (vec![IrType::Ptr, IrType::I64, IrType::I64], IrType::Void));
        fn_sigs.insert("slang_array_get_f64".into(),  (vec![IrType::Ptr, IrType::I64], IrType::F64));
        fn_sigs.insert("slang_array_set_f64".into(),  (vec![IrType::Ptr, IrType::I64, IrType::F64], IrType::Void));
        fn_sigs.insert("slang_array_len".into(),      (vec![IrType::Ptr], IrType::I64));

        // ── v15: String operations ────────────────────────────────────
        for name in &["str_upper", "str_lower", "str_trim", "str_reverse"] {
            fn_sigs.insert(name.to_string(), (vec![IrType::Ptr], IrType::Ptr));
        }
        for name in &["str_contains", "str_starts_with", "str_ends_with"] {
            fn_sigs.insert(name.to_string(), (vec![IrType::Ptr, IrType::Ptr], IrType::Bool));
        }
        fn_sigs.insert("str_char_at".into(),      (vec![IrType::Ptr, IrType::I64], IrType::Ptr));
        fn_sigs.insert("str_substr".into(),        (vec![IrType::Ptr, IrType::I64, IrType::I64], IrType::Ptr));
        fn_sigs.insert("str_index_of".into(),      (vec![IrType::Ptr, IrType::Ptr], IrType::I64));
        fn_sigs.insert("str_replace".into(),       (vec![IrType::Ptr, IrType::Ptr, IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("str_repeat".into(),        (vec![IrType::Ptr, IrType::I64], IrType::Ptr));
        fn_sigs.insert("str_split_count".into(),   (vec![IrType::Ptr, IrType::Ptr], IrType::I64));
        fn_sigs.insert("str_split_get".into(),     (vec![IrType::Ptr, IrType::Ptr, IrType::I64], IrType::Ptr));
        fn_sigs.insert("to_string_i64".into(),     (vec![IrType::I64], IrType::Ptr));
        fn_sigs.insert("to_string_f64".into(),     (vec![IrType::F64], IrType::Ptr));
        fn_sigs.insert("to_string_bool".into(),    (vec![IrType::Bool], IrType::Ptr));
        fn_sigs.insert("parse_int".into(),         (vec![IrType::Ptr], IrType::I64));
        fn_sigs.insert("parse_float".into(),       (vec![IrType::Ptr], IrType::F64));

        // ── v15: File I/O ─────────────────────────────────────────────
        fn_sigs.insert("file_read".into(),         (vec![IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("file_write".into(),        (vec![IrType::Ptr, IrType::Ptr], IrType::Bool));
        fn_sigs.insert("file_append".into(),       (vec![IrType::Ptr, IrType::Ptr], IrType::Bool));
        fn_sigs.insert("file_exists".into(),       (vec![IrType::Ptr], IrType::Bool));
        fn_sigs.insert("file_delete".into(),       (vec![IrType::Ptr], IrType::Bool));
        fn_sigs.insert("file_size".into(),         (vec![IrType::Ptr], IrType::I64));

        // ── v15: Map operations ───────────────────────────────────────
        fn_sigs.insert("map_new".into(),           (vec![], IrType::I64));
        fn_sigs.insert("map_set".into(),           (vec![IrType::I64, IrType::Ptr, IrType::I64], IrType::Void));
        fn_sigs.insert("map_get".into(),           (vec![IrType::I64, IrType::Ptr], IrType::I64));
        fn_sigs.insert("map_has".into(),            (vec![IrType::I64, IrType::Ptr], IrType::Bool));
        fn_sigs.insert("map_remove".into(),        (vec![IrType::I64, IrType::Ptr], IrType::Void));
        fn_sigs.insert("map_len".into(),           (vec![IrType::I64], IrType::I64));
        fn_sigs.insert("map_keys".into(),          (vec![IrType::I64], IrType::Ptr));

        // ── v15: Error handling ───────────────────────────────────────
        fn_sigs.insert("error_set".into(),         (vec![IrType::I64, IrType::Ptr], IrType::Void));
        fn_sigs.insert("error_check".into(),       (vec![], IrType::I64));
        fn_sigs.insert("error_msg".into(),         (vec![], IrType::Ptr));
        fn_sigs.insert("error_clear".into(),       (vec![], IrType::Void));

        // ── v15: Environment & System ─────────────────────────────────
        fn_sigs.insert("env_get".into(),           (vec![IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("sleep_ms".into(),          (vec![IrType::I64], IrType::Void));
        fn_sigs.insert("eprint".into(),            (vec![IrType::Ptr], IrType::Void));
        fn_sigs.insert("eprintln".into(),          (vec![IrType::Ptr], IrType::Void));
        fn_sigs.insert("pid".into(),               (vec![], IrType::I64));
        fn_sigs.insert("format_int".into(),        (vec![IrType::Ptr, IrType::I64], IrType::Ptr));
        fn_sigs.insert("format_float".into(),      (vec![IrType::Ptr, IrType::F64], IrType::Ptr));

        // ── v15: JSON ─────────────────────────────────────────────────
        fn_sigs.insert("json_encode".into(),       (vec![IrType::I64], IrType::Ptr));
        fn_sigs.insert("json_decode".into(),       (vec![IrType::Ptr], IrType::I64));

        // ── v18: Collection methods (array_push, etc.) ────────────────
        fn_sigs.insert("array_push".into(),     (vec![IrType::Ptr, IrType::I64], IrType::Ptr));
        fn_sigs.insert("array_pop".into(),      (vec![IrType::Ptr], IrType::I64));
        fn_sigs.insert("array_contains".into(), (vec![IrType::Ptr, IrType::I64], IrType::Bool));
        fn_sigs.insert("array_reverse".into(),  (vec![IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("array_sort".into(),     (vec![IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("array_join".into(),     (vec![IrType::Ptr, IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("array_slice".into(),    (vec![IrType::Ptr, IrType::I64, IrType::I64], IrType::Ptr));
        fn_sigs.insert("array_find".into(),     (vec![IrType::Ptr, IrType::I64], IrType::I64));
        fn_sigs.insert("array_map".into(),      (vec![IrType::Ptr, IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("array_filter".into(),   (vec![IrType::Ptr, IrType::Ptr], IrType::Ptr));

        // ── v18: Error handling extensions ────────────────────────────
        fn_sigs.insert("error_message".into(),  (vec![], IrType::Ptr));

        // ── v18: Format function ──────────────────────────────────────
        fn_sigs.insert("format".into(),         (vec![IrType::Ptr, IrType::I64], IrType::Ptr));
        fn_sigs.insert("format2".into(),        (vec![IrType::Ptr, IrType::I64, IrType::I64], IrType::Ptr));

        // ── v18: String method wrappers ───────────────────────────────
        fn_sigs.insert("str_to_upper".into(),   (vec![IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("str_to_lower".into(),   (vec![IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("str_split".into(),      (vec![IrType::Ptr, IrType::Ptr], IrType::Ptr));
        fn_sigs.insert("str_substring".into(),  (vec![IrType::Ptr, IrType::I64, IrType::I64], IrType::Ptr));

        Self {
            module: IrModule::new(),
            current_blocks: Vec::new(),
            current_block: 0,
            next_value: 0,
            next_block: 0,
            locals: HashMap::new(),
            mutables: HashSet::new(),
            fn_sigs,
            value_types: HashMap::new(),
            mutable_var_types: HashMap::new(),
            struct_defs: HashMap::new(),
            method_registry: HashMap::new(),
            enum_defs: HashMap::new(),
            type_aliases: HashMap::new(),
            var_struct_types: HashMap::new(),
            loop_stack: Vec::new(),
            module_prefix: Vec::new(),
        }
    }

    fn fresh_value(&mut self) -> Value {
        let v = Value(self.next_value);
        self.next_value += 1;
        v
    }

    fn fresh_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    /// Record the semantic type of an SSA value for type-aware dispatch.
    fn record_type(&mut self, v: Value, ty: IrType) {
        self.value_types.insert(v, ty);
    }

    /// Get the inferred type of an SSA value (default I64).
    fn infer_type(&self, v: Value) -> IrType {
        self.value_types.get(&v).cloned().unwrap_or(IrType::I64)
    }

    /// True if the value is a float (F64 or F32).
    fn is_float(&self, v: Value) -> bool {
        matches!(self.infer_type(v), IrType::F64 | IrType::F32)
    }

    /// Look up the return type of a declared function. Falls back to I64.
    fn lookup_fn_ret_ty(&self, name: &str) -> IrType {
        self.fn_sigs.get(name).map(|(_, ret)| ret.clone()).unwrap_or(IrType::I64)
    }

    fn emit(&mut self, inst: Inst) {
        if let Some(block) = self.current_blocks.get_mut(self.current_block) {
            block.insts.push(inst);
        }
    }

    fn switch_block(&mut self, id: BlockId) {
        // Find or create the block
        if let Some(idx) = self.current_blocks.iter().position(|b| b.id == id) {
            self.current_block = idx;
        } else {
            let bb = BasicBlock::new(id);
            self.current_blocks.push(bb);
            self.current_block = self.current_blocks.len() - 1;
        }
    }

    // ── Public Entry Point ──────────────────────────────────────────
    pub fn build(mut self, program: &ast::Program) -> IrModule {
        // First pass: collect function signatures + struct defs
        for item in &program.items {
            self.collect_fn_sig(item);
        }

        // Second pass: lower functions
        for item in &program.items {
            self.lower_top_level(item);
        }

        self.module
    }

    /// Compute the mangled function name with module prefix.
    fn mangled_fn_name(&self, name: &str) -> String {
        if self.module_prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}_{}", self.module_prefix.join("_"), name)
        }
    }

    fn collect_fn_sig(&mut self, item: &ast::TopLevel) {
        match item {
            ast::TopLevel::Function(f) => {
                let params: Vec<IrType> = f.params.iter()
                    .map(|p| self.type_expr_to_ir(&p.ty))
                    .collect();
                let ret = f.return_type.as_ref()
                    .map(|t| self.type_expr_to_ir(t))
                    .unwrap_or(IrType::Void);
                let mangled = self.mangled_fn_name(&f.name);
                self.fn_sigs.insert(mangled, (params, ret));
            }
            ast::TopLevel::Struct(s) => {
                // Record struct field layout: name → [field_names in order]
                let field_names: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
                self.struct_defs.insert(s.name.clone(), field_names);
            }
            ast::TopLevel::Impl(imp) => {
                // Collect impl method signatures as TypeName_method
                for method in &imp.methods {
                    let mangled = format!("{}_{}", imp.type_name, method.name);
                    // Register in method_registry for dispatch
                    self.method_registry
                        .entry(imp.type_name.clone())
                        .or_insert_with(HashMap::new)
                        .insert(method.name.clone(), mangled.clone());
                    // Add implicit self param (Ptr) if first param is "self"
                    let mut params: Vec<IrType> = Vec::new();
                    for p in &method.params {
                        if p.name == "self" {
                            params.push(IrType::Ptr);
                        } else {
                            params.push(self.type_expr_to_ir(&p.ty));
                        }
                    }
                    let ret = method.return_type.as_ref()
                        .map(|t| self.type_expr_to_ir(t))
                        .unwrap_or(IrType::Void);
                    self.fn_sigs.insert(mangled, (params, ret));
                }
            }
            ast::TopLevel::Annotated { item, .. } => self.collect_fn_sig(item),
            ast::TopLevel::Trait(_) | ast::TopLevel::TypeAlias(_) => {}
            ast::TopLevel::Module(m) => {
                self.module_prefix.push(m.name.clone());
                for sub in &m.items {
                    self.collect_fn_sig(sub);
                }
                self.module_prefix.pop();
            }
            _ => {}
        }
    }

    fn type_expr_to_ir(&self, texpr: &ast::TypeExpr) -> IrType {
        match texpr {
            ast::TypeExpr::Named(name, _) => match name.as_str() {
                "i32" => IrType::I32,
                "i64" => IrType::I64,
                "f32" => IrType::F32,
                "f64" => IrType::F64,
                "bool" => IrType::Bool,
                "str" => IrType::Ptr,
                "void" => IrType::Void,
                _ => IrType::Ptr,
            },
            ast::TypeExpr::Function { .. } => IrType::Ptr,
            ast::TypeExpr::Array { .. } => IrType::Ptr,
            ast::TypeExpr::Generic { .. } => IrType::Ptr,
            ast::TypeExpr::Ref { .. } => IrType::Ptr,
            ast::TypeExpr::Inferred(_) => IrType::I64,
        }
    }

    fn lower_top_level(&mut self, item: &ast::TopLevel) {
        match item {
            ast::TopLevel::Function(f) => self.lower_function(f),
            ast::TopLevel::Annotated { item, .. } => self.lower_top_level(item),
            ast::TopLevel::Module(m) => {
                self.module_prefix.push(m.name.clone());
                for sub in &m.items {
                    self.lower_top_level(sub);
                }
                self.module_prefix.pop();
            }
            ast::TopLevel::Impl(imp) => {
                // Lower each method as TypeName_method with self prepended
                for method in &imp.methods {
                    let mangled = format!("{}_{}", imp.type_name, method.name);
                    let mut mangled_fn = method.clone();
                    mangled_fn.name = mangled;
                    self.lower_function(&mangled_fn);
                }
            }
            ast::TopLevel::Const(_c) => {
                // Lower const as a global-scope computed value
                // We create a tiny init function if needed; for simple literals,
                // just record in locals when encountered
                // For v18, consts are handled at lower_expr/Ident time
            }
            ast::TopLevel::Struct(s) => {
                // Register struct field layout
                let field_names: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
                self.struct_defs.insert(s.name.clone(), field_names);
            }
            ast::TopLevel::Enum(e) => {
                // Register enum variant layout
                let variants: Vec<(String, usize)> = e.variants.iter()
                    .map(|v| (v.name.clone(), v.fields.len()))
                    .collect();
                self.enum_defs.insert(e.name.clone(), variants);
            }
            ast::TopLevel::TypeAlias(ta) => {
                let ir_ty = self.type_expr_to_ir(&ta.ty);
                self.type_aliases.insert(ta.name.clone(), ir_ty);
            }
            ast::TopLevel::Trait(_) => {
                // Trait defs are type-level only; methods come via impl blocks
            }
            _ => {} // Imports don't produce IR directly
        }
    }

    // ── Lower Function ──────────────────────────────────────────────
    fn lower_function(&mut self, f: &ast::Function) {
        // Reset per-function state
        self.current_blocks = Vec::new();
        self.next_value = 0;
        self.next_block = 0;
        self.locals = HashMap::new();
        self.mutables = HashSet::new();
        self.value_types = HashMap::new();
        self.mutable_var_types = HashMap::new();
        self.var_struct_types = HashMap::new();
        self.loop_stack = Vec::new();

        let entry = self.fresh_block();
        let bb = BasicBlock::new(entry);
        self.current_blocks.push(bb);
        self.current_block = 0;

        let params: Vec<(String, IrType)> = f.params.iter()
            .map(|p| (p.name.clone(), self.type_expr_to_ir(&p.ty)))
            .collect();

        let ret_type = f.return_type.as_ref()
            .map(|t| self.type_expr_to_ir(t))
            .unwrap_or(IrType::Void);

        // Bind parameters as values and record their types
        for (name, ty) in &params {
            let v = self.fresh_value();
            self.locals.insert(name.clone(), v);
            self.record_type(v, ty.clone());
        }

        // v18: Track struct types for parameters (for impl methods)
        for p in &f.params {
            if let ast::TypeExpr::Named(type_name, _) = &p.ty {
                if self.struct_defs.contains_key(type_name) {
                    self.var_struct_types.insert(p.name.clone(), type_name.clone());
                }
            }
        }

        // Lower body
        let body_val = self.lower_block(&f.body);

        // Emit return
        self.emit(Inst::Return { value: body_val });

        // Apply module prefix to function name
        let fn_name = self.mangled_fn_name(&f.name);

        let ir_func = IrFunction {
            name: fn_name,
            params,
            ret_type,
            blocks: std::mem::take(&mut self.current_blocks),
            entry,
        };

        self.module.functions.push(ir_func);
    }

    fn lower_block(&mut self, block: &ast::Block) -> Option<Value> {
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
        if let Some(ref tail) = block.tail_expr {
            Some(self.lower_expr(tail))
        } else {
            None
        }
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Let { name, value, mutable, ty: ty_annot, .. } => {
                let val = if let Some(expr) = value {
                    // v18: Track struct type if RHS is a struct literal
                    if let ast::Expr::StructLiteral { name: sname, .. } = expr {
                        self.var_struct_types.insert(name.clone(), sname.clone());
                    }
                    self.lower_expr(expr)
                } else {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::I64 });
                    self.record_type(v, IrType::I64);
                    v
                };
                // Determine var type: prefer annotation, then infer from value
                let var_ty = if let Some(ta) = ty_annot {
                    self.type_expr_to_ir(ta)
                } else {
                    self.infer_type(val)
                };
                if *mutable {
                    // Mutable: alloca a stack slot, store initial value, track ptr
                    let ptr = self.fresh_value();
                    self.emit(Inst::Alloca { result: ptr, size: 8 });
                    self.emit(Inst::Store { value: val, ptr });
                    self.locals.insert(name.clone(), ptr);
                    self.mutables.insert(name.clone());
                    self.mutable_var_types.insert(name.clone(), var_ty);
                } else {
                    self.locals.insert(name.clone(), val);
                }
            }
            ast::Stmt::Expr(e) => {
                self.lower_expr(e);
            }
            ast::Stmt::While { condition, body, .. } => {
                let cond_bb = self.fresh_block();
                let body_bb = self.fresh_block();
                let exit_bb = self.fresh_block();

                // v18: Push loop context for break/continue
                self.loop_stack.push((cond_bb, exit_bb));

                self.emit(Inst::Jump { target: cond_bb });
                self.switch_block(cond_bb);

                let cond_val = self.lower_expr(condition);
                self.emit(Inst::Branch {
                    cond: cond_val,
                    then_bb: body_bb,
                    else_bb: exit_bb,
                });

                self.switch_block(body_bb);
                self.lower_block(body);
                self.emit(Inst::Jump { target: cond_bb });

                self.loop_stack.pop();
                self.switch_block(exit_bb);
            }
            ast::Stmt::For { var, iter, body, .. } => {
                if let ast::Expr::Range { start, end, .. } = &iter {
                    // Proper counted for-range loop: for var in start..end
                    let start_val = self.lower_expr(&**start);
                    let end_val   = self.lower_expr(&**end);

                    // Alloca mutable loop counter
                    let counter_ptr = self.fresh_value();
                    self.emit(Inst::Alloca { result: counter_ptr, size: 8 });
                    self.emit(Inst::Store { value: start_val, ptr: counter_ptr });

                    let cond_bb = self.fresh_block();
                    let body_bb = self.fresh_block();
                    let exit_bb = self.fresh_block();

                    // v18: Push loop context for break/continue
                    self.loop_stack.push((cond_bb, exit_bb));

                    self.emit(Inst::Jump { target: cond_bb });
                    self.switch_block(cond_bb);

                    // Load counter and check < end
                    let cnt = self.fresh_value();
                    self.emit(Inst::Load { result: cnt, ptr: counter_ptr, ty: IrType::I64 });
                    self.record_type(cnt, IrType::I64);

                    // Compare: cnt < end_val
                    let cmp = self.fresh_value();
                    self.emit(Inst::ICmp { result: cmp, cond: IrCmp::Lt, lhs: cnt, rhs: end_val });
                    self.record_type(cmp, IrType::Bool);
                    self.emit(Inst::Branch {
                        cond: cmp,
                        then_bb: body_bb,
                        else_bb: exit_bb,
                    });

                    // Body: bind loop variable, run body, increment counter
                    self.switch_block(body_bb);
                    self.locals.insert(var.clone(), cnt); // loop var = current counter value
                    self.lower_block(body);

                    // Increment: counter++
                    let cnt2 = self.fresh_value();
                    self.emit(Inst::Load { result: cnt2, ptr: counter_ptr, ty: IrType::I64 });
                    self.record_type(cnt2, IrType::I64);
                    let one = self.fresh_value();
                    self.emit(Inst::IConst { result: one, value: 1, ty: IrType::I64 });
                    self.record_type(one, IrType::I64);
                    let cnt3 = self.fresh_value();
                    self.emit(Inst::BinOp { result: cnt3, op: IrBinOp::Add, lhs: cnt2, rhs: one, ty: IrType::I64 });
                    self.record_type(cnt3, IrType::I64);
                    self.emit(Inst::Store { value: cnt3, ptr: counter_ptr });
                    self.emit(Inst::Jump { target: cond_bb });

                    self.loop_stack.pop();
                    self.switch_block(exit_bb);
                } else {
                    // v18: For-each over arrays: for x in arr { body }
                    // Lower as: len = array_len(arr); i = 0; while i < len { x = arr[i]; body; i++ }
                    let arr_val = self.lower_expr(&iter);

                    // Get array length
                    let len_val = self.fresh_value();
                    self.emit(Inst::ArrayLen { result: len_val, array: arr_val });
                    self.record_type(len_val, IrType::I64);

                    // Alloca mutable index counter
                    let idx_ptr = self.fresh_value();
                    let zero = self.fresh_value();
                    self.emit(Inst::IConst { result: zero, value: 0, ty: IrType::I64 });
                    self.record_type(zero, IrType::I64);
                    self.emit(Inst::Alloca { result: idx_ptr, size: 8 });
                    self.emit(Inst::Store { value: zero, ptr: idx_ptr });

                    let cond_bb = self.fresh_block();
                    let body_bb = self.fresh_block();
                    let exit_bb = self.fresh_block();

                    // v18: Push loop context for break/continue
                    self.loop_stack.push((cond_bb, exit_bb));

                    self.emit(Inst::Jump { target: cond_bb });
                    self.switch_block(cond_bb);

                    // Load index, compare < len
                    let idx = self.fresh_value();
                    self.emit(Inst::Load { result: idx, ptr: idx_ptr, ty: IrType::I64 });
                    self.record_type(idx, IrType::I64);
                    let cmp = self.fresh_value();
                    self.emit(Inst::ICmp { result: cmp, cond: IrCmp::Lt, lhs: idx, rhs: len_val });
                    self.record_type(cmp, IrType::Bool);
                    self.emit(Inst::Branch { cond: cmp, then_bb: body_bb, else_bb: exit_bb });

                    // Body: arr[idx] → loop var → execute body → idx++
                    self.switch_block(body_bb);
                    let elem = self.fresh_value();
                    self.emit(Inst::ArrayGet { result: elem, array: arr_val, index: idx, elem_ty: IrType::I64 });
                    self.record_type(elem, IrType::I64);
                    self.locals.insert(var.clone(), elem);
                    self.lower_block(body);

                    // Increment index
                    let idx2 = self.fresh_value();
                    self.emit(Inst::Load { result: idx2, ptr: idx_ptr, ty: IrType::I64 });
                    self.record_type(idx2, IrType::I64);
                    let one = self.fresh_value();
                    self.emit(Inst::IConst { result: one, value: 1, ty: IrType::I64 });
                    self.record_type(one, IrType::I64);
                    let idx3 = self.fresh_value();
                    self.emit(Inst::BinOp { result: idx3, op: IrBinOp::Add, lhs: idx2, rhs: one, ty: IrType::I64 });
                    self.record_type(idx3, IrType::I64);
                    self.emit(Inst::Store { value: idx3, ptr: idx_ptr });
                    self.emit(Inst::Jump { target: cond_bb });

                    self.loop_stack.pop();
                    self.switch_block(exit_bb);
                }
            }
            ast::Stmt::Loop { body, .. } => {
                let body_bb = self.fresh_block();
                let exit_bb = self.fresh_block();

                // v18: Push loop context for break/continue
                self.loop_stack.push((body_bb, exit_bb));

                self.emit(Inst::Jump { target: body_bb });
                self.switch_block(body_bb);
                self.lower_block(body);
                self.emit(Inst::Jump { target: body_bb });

                self.loop_stack.pop();
                self.switch_block(exit_bb);
            }
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr) -> Value {
        match expr {
            ast::Expr::IntLiteral(n, _) => {
                let v = self.fresh_value();
                self.emit(Inst::IConst { result: v, value: *n, ty: IrType::I64 });
                self.record_type(v, IrType::I64);
                v
            }
            ast::Expr::FloatLiteral(n, _) => {
                let v = self.fresh_value();
                self.emit(Inst::FConst { result: v, value: *n, ty: IrType::F64 });
                self.record_type(v, IrType::F64);
                v
            }
            ast::Expr::StringLiteral(s, _) => {
                let v = self.fresh_value();
                let idx = self.module.string_constants.len();
                self.module.string_constants.push(s.clone());
                self.emit(Inst::StrConst { result: v, value: s.clone() });
                self.record_type(v, IrType::Ptr);
                let _ = idx;
                v
            }
            ast::Expr::BoolLiteral(b, _) => {
                let v = self.fresh_value();
                self.emit(Inst::BConst { result: v, value: *b });
                self.record_type(v, IrType::Bool);
                v
            }
            ast::Expr::Ident(name, _) => {
                if let Some(val) = self.locals.get(name).copied() {
                    if self.mutables.contains(name) {
                        // Mutable variable: val is the alloca ptr, emit Load with correct type
                        let var_ty = self.mutable_var_types.get(name).cloned().unwrap_or(IrType::I64);
                        let loaded = self.fresh_value();
                        self.emit(Inst::Load { result: loaded, ptr: val, ty: var_ty.clone() });
                        self.record_type(loaded, var_ty);
                        loaded
                    } else {
                        val
                    }
                } else {
                    // Undefined — produce a zero value
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::I64 });
                    self.record_type(v, IrType::I64);
                    v
                }
            }
            ast::Expr::Binary { op, left, right, .. } => {
                let lhs = self.lower_expr(left);
                let rhs = self.lower_expr(right);
                let v = self.fresh_value();
                let float_op = self.is_float(lhs) || self.is_float(rhs);
                let arith_ty = if float_op { IrType::F64 } else { IrType::I64 };

                match op {
                    BinOp::Add => {
                        if float_op {
                            self.emit(Inst::BinOp { result: v, op: IrBinOp::FAdd, lhs, rhs, ty: IrType::F64 });
                        } else {
                            self.emit(Inst::BinOp { result: v, op: IrBinOp::Add, lhs, rhs, ty: IrType::I64 });
                        }
                        self.record_type(v, arith_ty);
                    }
                    BinOp::Sub => {
                        if float_op {
                            self.emit(Inst::BinOp { result: v, op: IrBinOp::FSub, lhs, rhs, ty: IrType::F64 });
                        } else {
                            self.emit(Inst::BinOp { result: v, op: IrBinOp::Sub, lhs, rhs, ty: IrType::I64 });
                        }
                        self.record_type(v, arith_ty);
                    }
                    BinOp::Mul => {
                        if float_op {
                            self.emit(Inst::BinOp { result: v, op: IrBinOp::FMul, lhs, rhs, ty: IrType::F64 });
                        } else {
                            self.emit(Inst::BinOp { result: v, op: IrBinOp::Mul, lhs, rhs, ty: IrType::I64 });
                        }
                        self.record_type(v, arith_ty);
                    }
                    BinOp::Div => {
                        if float_op {
                            self.emit(Inst::BinOp { result: v, op: IrBinOp::FDiv, lhs, rhs, ty: IrType::F64 });
                        } else {
                            self.emit(Inst::BinOp { result: v, op: IrBinOp::Div, lhs, rhs, ty: IrType::I64 });
                        }
                        self.record_type(v, arith_ty);
                    }
                    BinOp::Mod => {
                        self.emit(Inst::BinOp { result: v, op: IrBinOp::Mod, lhs, rhs, ty: IrType::I64 });
                        self.record_type(v, IrType::I64);
                    }
                    BinOp::Eq => {
                        if float_op {
                            self.emit(Inst::FCmp { result: v, cond: IrCmp::Eq, lhs, rhs });
                        } else {
                            self.emit(Inst::ICmp { result: v, cond: IrCmp::Eq, lhs, rhs });
                        }
                        self.record_type(v, IrType::Bool);
                    }
                    BinOp::NotEq => {
                        if float_op {
                            self.emit(Inst::FCmp { result: v, cond: IrCmp::Ne, lhs, rhs });
                        } else {
                            self.emit(Inst::ICmp { result: v, cond: IrCmp::Ne, lhs, rhs });
                        }
                        self.record_type(v, IrType::Bool);
                    }
                    BinOp::Lt => {
                        if float_op {
                            self.emit(Inst::FCmp { result: v, cond: IrCmp::Lt, lhs, rhs });
                        } else {
                            self.emit(Inst::ICmp { result: v, cond: IrCmp::Lt, lhs, rhs });
                        }
                        self.record_type(v, IrType::Bool);
                    }
                    BinOp::Gt => {
                        if float_op {
                            self.emit(Inst::FCmp { result: v, cond: IrCmp::Gt, lhs, rhs });
                        } else {
                            self.emit(Inst::ICmp { result: v, cond: IrCmp::Gt, lhs, rhs });
                        }
                        self.record_type(v, IrType::Bool);
                    }
                    BinOp::LtEq => {
                        if float_op {
                            self.emit(Inst::FCmp { result: v, cond: IrCmp::Le, lhs, rhs });
                        } else {
                            self.emit(Inst::ICmp { result: v, cond: IrCmp::Le, lhs, rhs });
                        }
                        self.record_type(v, IrType::Bool);
                    }
                    BinOp::GtEq => {
                        if float_op {
                            self.emit(Inst::FCmp { result: v, cond: IrCmp::Ge, lhs, rhs });
                        } else {
                            self.emit(Inst::ICmp { result: v, cond: IrCmp::Ge, lhs, rhs });
                        }
                        self.record_type(v, IrType::Bool);
                    }
                    BinOp::And => {
                        self.emit(Inst::BinOp { result: v, op: IrBinOp::And, lhs, rhs, ty: IrType::Bool });
                        self.record_type(v, IrType::Bool);
                    }
                    BinOp::Or => {
                        self.emit(Inst::BinOp { result: v, op: IrBinOp::Or, lhs, rhs, ty: IrType::Bool });
                        self.record_type(v, IrType::Bool);
                    }
                }
                v
            }
            ast::Expr::Unary { op, operand, .. } => {
                let inner = self.lower_expr(operand);
                let v = self.fresh_value();
                let is_float_inner = self.is_float(inner);
                match op {
                    UnaryOp::Neg => {
                        if is_float_inner {
                            self.emit(Inst::UnOp { result: v, op: IrUnOp::FNeg, operand: inner, ty: IrType::F64 });
                            self.record_type(v, IrType::F64);
                        } else {
                            self.emit(Inst::UnOp { result: v, op: IrUnOp::Neg, operand: inner, ty: IrType::I64 });
                            self.record_type(v, IrType::I64);
                        }
                    }
                    UnaryOp::Not => {
                        self.emit(Inst::UnOp { result: v, op: IrUnOp::Not, operand: inner, ty: IrType::Bool });
                        self.record_type(v, IrType::Bool);
                    }
                }
                v
            }
            ast::Expr::Call { func, args, .. } => {
                let func_name = match func.as_ref() {
                    ast::Expr::Ident(name, _) => name.clone(),
                    _ => "<indirect>".to_string(),
                };
                let arg_vals: Vec<Value> = args.iter().map(|a| self.lower_expr(a)).collect();
                let ret_ty = self.fn_sigs.get(&func_name)
                    .map(|(_, r)| r.clone())
                    .unwrap_or(IrType::I64);
                let v = self.fresh_value();
                self.record_type(v, ret_ty.clone());
                self.emit(Inst::Call {
                    result: v,
                    func: func_name,
                    args: arg_vals,
                    ret_ty,
                });
                v
            }
            ast::Expr::If { condition, then_branch, else_branch, .. } => {
                let cond_val = self.lower_expr(condition);
                let then_bb = self.fresh_block();
                let else_bb = self.fresh_block();
                let merge_bb = self.fresh_block();

                self.emit(Inst::Branch {
                    cond: cond_val,
                    then_bb,
                    else_bb,
                });

                // Then
                self.switch_block(then_bb);
                let then_val = self.lower_block(then_branch).unwrap_or_else(|| {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::I64 });
                    self.record_type(v, IrType::I64);
                    v
                });
                let then_ty = self.infer_type(then_val);
                // Capture actual current block (nested if/else may change it)
                let then_pred_bb = self.current_blocks[self.current_block].id;
                self.emit(Inst::Jump { target: merge_bb });

                // Else
                self.switch_block(else_bb);
                let else_val = if let Some(eb) = else_branch {
                    self.lower_block(eb).unwrap_or_else(|| {
                        let v = self.fresh_value();
                        self.emit(Inst::IConst { result: v, value: 0, ty: IrType::I64 });
                        self.record_type(v, IrType::I64);
                        v
                    })
                } else {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::I64 });
                    self.record_type(v, IrType::I64);
                    v
                };
                // Capture actual current block (nested if/else may change it)
                let else_pred_bb = self.current_blocks[self.current_block].id;
                self.emit(Inst::Jump { target: merge_bb });

                // Merge with correct phi type
                self.switch_block(merge_bb);
                let phi = self.fresh_value();
                let phi_ty = then_ty;
                self.record_type(phi, phi_ty.clone());
                self.emit(Inst::Phi {
                    result: phi,
                    incoming: vec![(then_val, then_pred_bb), (else_val, else_pred_bb)],
                    ty: phi_ty,
                });
                phi
            }
            ast::Expr::Return { value, .. } => {
                let val = value.as_ref().map(|v| self.lower_expr(v));
                self.emit(Inst::Return { value: val });
                // Return a dummy value (this instruction is terminal)
                let v = self.fresh_value();
                self.emit(Inst::IConst { result: v, value: 0, ty: IrType::Void });
                v
            }
            ast::Expr::Assign { target, value, .. } => {
                let val = self.lower_expr(value);
                if let ast::Expr::Ident(name, _) = target.as_ref() {
                    if self.mutables.contains(name) {
                        // Mutable: store to alloca ptr
                        if let Some(ptr) = self.locals.get(name).copied() {
                            self.emit(Inst::Store { value: val, ptr });
                        }
                    } else {
                        self.locals.insert(name.clone(), val);
                    }
                }
                val
            }
            ast::Expr::CompoundAssign { op, target, value, .. } => {
                let old = self.lower_expr(target);
                let rhs = self.lower_expr(value);
                let v = self.fresh_value();
                let float_op = self.is_float(old) || self.is_float(rhs);
                let arith_ty = if float_op { IrType::F64 } else { IrType::I64 };
                let ir_op = match op {
                    BinOp::Add => if float_op { IrBinOp::FAdd } else { IrBinOp::Add },
                    BinOp::Sub => if float_op { IrBinOp::FSub } else { IrBinOp::Sub },
                    BinOp::Mul => if float_op { IrBinOp::FMul } else { IrBinOp::Mul },
                    BinOp::Div => if float_op { IrBinOp::FDiv } else { IrBinOp::Div },
                    _ => if float_op { IrBinOp::FAdd } else { IrBinOp::Add },
                };
                self.emit(Inst::BinOp { result: v, op: ir_op, lhs: old, rhs, ty: arith_ty.clone() });
                self.record_type(v, arith_ty);
                if let ast::Expr::Ident(name, _) = target.as_ref() {
                    if self.mutables.contains(name) {
                        // Mutable: store result to alloca ptr
                        if let Some(ptr) = self.locals.get(name).copied() {
                            self.emit(Inst::Store { value: v, ptr });
                        }
                    } else {
                        self.locals.insert(name.clone(), v);
                    }
                }
                v
            }
            ast::Expr::Block(block) => {
                self.lower_block(block).unwrap_or_else(|| {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::Void });
                    v
                })
            }
            ast::Expr::Pipe { stages, .. } => {
                // Thread values through pipeline: `a |> f |> g` → `g(f(a))`
                // First stage evaluates normally; subsequent stages must be
                // calls that receive the previous result as their first arg.
                if stages.is_empty() {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::Void });
                    return v;
                }

                let mut current = self.lower_expr(&stages[0]);

                for stage in &stages[1..] {
                    match stage {
                        // `value |> func(extra_args)` → `func(value, extra_args)`
                        ast::Expr::Call { func, args, .. } => {
                            let func_name = match func.as_ref() {
                                ast::Expr::Ident(n, _) => n.clone(),
                                _ => "<indirect>".to_string(),
                            };
                            let mut all_args = vec![current];
                            for a in args {
                                all_args.push(self.lower_expr(a));
                            }
                            let result = self.fresh_value();
                            let ret_ty = self.lookup_fn_ret_ty(&func_name);
                            self.record_type(result, ret_ty.clone());
                            self.emit(Inst::Call {
                                result, func: func_name,
                                args: all_args, ret_ty,
                            });
                            current = result;
                        }
                        // `value |> ident` — call ident(value)
                        ast::Expr::Ident(name, _) => {
                            let result = self.fresh_value();
                            let ret_ty = self.lookup_fn_ret_ty(name);
                            self.record_type(result, ret_ty.clone());
                            self.emit(Inst::Call {
                                result, func: name.clone(),
                                args: vec![current], ret_ty,
                            });
                            current = result;
                        }
                        // Anything else: just evaluate (backwards compat)
                        other => {
                            current = self.lower_expr(other);
                        }
                    }
                }
                current
            }
            ast::Expr::Parallel { exprs, .. } => {
                // Lower all exprs (sequential in Phase 0)
                let mut last = None;
                for e in exprs {
                    last = Some(self.lower_expr(e));
                }
                last.unwrap_or_else(|| {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::Void });
                    v
                })
            }
            // ── Phase 4: List / Array literal ─────────────────────────────────
            ast::Expr::List { elements, .. } => {
                let count_v = self.fresh_value();
                let count = elements.len() as i64;
                self.emit(Inst::IConst { result: count_v, value: count, ty: IrType::I64 });
                self.record_type(count_v, IrType::I64);

                // Determine element type from first element (homogeneous arrays).
                // Emit a dummy allocation first to get the result register.
                let arr_result = self.fresh_value();
                // Pick element type by speculatively lowering first element
                let elem_ty = if elements.is_empty() {
                    IrType::I64
                } else {
                    let probe = self.fresh_value();
                    self.emit(Inst::IConst { result: probe, value: 0, ty: IrType::I64 });
                    // Lower first element to discover its type, then discard the temp
                    let fv = self.lower_expr(&elements[0]);
                    self.infer_type(fv)
                };
                let stride_v = self.fresh_value();
                let stride = elem_ty.byte_size() as i64;
                self.emit(Inst::IConst { result: stride_v, value: stride, ty: IrType::I64 });
                self.record_type(stride_v, IrType::I64);
                self.emit(Inst::ArrayAlloc { result: arr_result, elem_ty: elem_ty.clone(), count: count_v });
                self.record_type(arr_result, IrType::Ptr);

                // Populate elements (re-lower each, including index 0).
                for (i, elem_expr) in elements.iter().enumerate() {
                    let elem_val = self.lower_expr(elem_expr);
                    let idx_v = self.fresh_value();
                    self.emit(Inst::IConst { result: idx_v, value: i as i64, ty: IrType::I64 });
                    self.record_type(idx_v, IrType::I64);
                    let ev_ty = self.infer_type(elem_val);
                    self.emit(Inst::ArraySet {
                        array: arr_result,
                        index: idx_v,
                        value: elem_val,
                        elem_ty: ev_ty,
                    });
                }
                arr_result
            }

            // ── Phase 4: Index expression arr[i] ─────────────────────────────────
            ast::Expr::Index { object, index, .. } => {
                let arr = self.lower_expr(object);
                let idx = self.lower_expr(index);
                let result = self.fresh_value();
                // Infer element type: if object's type is Ptr, default to I64.
                // A proper type table would refine this in Phase 4B.
                let elem_ty = IrType::I64;
                self.emit(Inst::ArrayGet { result, array: arr, index: idx, elem_ty: elem_ty.clone() });
                self.record_type(result, elem_ty);
                result
            }

            // ── Cast: expr as Type ──────────────────────────────────────────────
            ast::Expr::Cast { expr, ty, .. } => {
                let inner = self.lower_expr(expr);
                let src_ty = self.infer_type(inner);
                let dst_ty = self.type_expr_to_ir(ty);
                let result = self.fresh_value();
                match (&src_ty, &dst_ty) {
                    (IrType::I64, IrType::F64) | (IrType::I32, IrType::F64) => {
                        self.record_type(result, IrType::F64);
                        self.emit(Inst::Call {
                            result, func: "i64_to_f64".to_string(),
                            args: vec![inner], ret_ty: IrType::F64,
                        });
                    }
                    (IrType::F64, IrType::I64) | (IrType::F32, IrType::I64) => {
                        self.record_type(result, IrType::I64);
                        self.emit(Inst::Call {
                            result, func: "f64_to_i64".to_string(),
                            args: vec![inner], ret_ty: IrType::I64,
                        });
                    }
                    _ => {
                        // Same-type or unrecognised cast — identity copy.
                        self.emit(Inst::Copy { result, source: inner });
                        self.record_type(result, dst_ty.clone());
                    }
                }
                result
            }

            // ── Method calls: obj.method(args) ──────────────────────────────────
            ast::Expr::MethodCall { object, method, args, .. } => {
                let obj = self.lower_expr(object);
                match method.as_str() {
                    "len" => {
                        let result = self.fresh_value();
                        self.emit(Inst::ArrayLen { result, array: obj });
                        self.record_type(result, IrType::I64);
                        result
                    }
                    "push" | "pop" | "contains" | "reverse" | "sort" | "join" | "slice" | "find" | "map" | "filter" => {
                        // v18: built-in collection methods → runtime calls
                        let builtin_name = format!("array_{}", method);
                        let mut call_args = vec![obj];
                        call_args.extend(args.iter().map(|a| self.lower_expr(a)));
                        let result = self.fresh_value();
                        let ret_ty = match method.as_str() {
                            "push" | "reverse" | "sort" => IrType::Ptr,
                            "pop" | "find" => IrType::I64,
                            "contains" => IrType::Bool,
                            "join" => IrType::Ptr,
                            "slice" | "map" | "filter" => IrType::Ptr,
                            _ => IrType::I64,
                        };
                        self.record_type(result, ret_ty.clone());
                        self.emit(Inst::Call { result, func: builtin_name, args: call_args, ret_ty });
                        result
                    }
                    // String methods
                    "to_upper" | "to_lower" | "trim" | "split" | "starts_with" | "ends_with"
                    | "replace" | "substring" | "char_at" | "index_of" => {
                        let builtin_name = format!("str_{}", method);
                        let mut call_args = vec![obj];
                        call_args.extend(args.iter().map(|a| self.lower_expr(a)));
                        let result = self.fresh_value();
                        let ret_ty = match method.as_str() {
                            "starts_with" | "ends_with" => IrType::Bool,
                            "index_of" | "char_at" => IrType::I64,
                            _ => IrType::Ptr,
                        };
                        self.record_type(result, ret_ty.clone());
                        self.emit(Inst::Call { result, func: builtin_name, args: call_args, ret_ty });
                        result
                    }
                    _ => {
                        // v18: Try impl-based method dispatch first
                        // Check if object is a known struct type and look for TypeName_method
                        let obj_type_name = if let ast::Expr::Ident(name, _) = object.as_ref() {
                            self.var_struct_types.get(name).cloned()
                        } else {
                            None
                        };

                        if let Some(type_name) = obj_type_name {
                            let mangled = format!("{}_{}", type_name, method);
                            if self.fn_sigs.contains_key(&mangled) {
                                let mut call_args = vec![obj];
                                call_args.extend(args.iter().map(|a| self.lower_expr(a)));
                                let result = self.fresh_value();
                                let ret_ty = self.lookup_fn_ret_ty(&mangled);
                                self.record_type(result, ret_ty.clone());
                                self.emit(Inst::Call { result, func: mangled, args: call_args, ret_ty });
                                return result;
                            }
                        }

                        // Generic fallback: lower as __vtbl_obj_method
                        let mut call_args = vec![obj];
                        call_args.extend(args.iter().map(|a| self.lower_expr(a)));
                        let result = self.fresh_value();
                        let callee = format!("__vtbl_{}_{}", "obj", method);
                        let ret_ty = IrType::I64;
                        self.record_type(result, ret_ty.clone());
                        self.emit(Inst::Call { result, func: callee, args: call_args, ret_ty });
                        result
                    }
                }
            }

            // ── Field access: obj.field ─────────────────────────────────────────
            ast::Expr::Field { object, field, .. } => {
                let obj = self.lower_expr(object);

                // v18: Resolve field name → index using struct_defs layout table
                let field_index: u32 = if let Ok(idx) = field.parse::<u32>() {
                    idx // If it's a numeric index, use directly
                } else {
                    // Look up struct type from variable name
                    let struct_type = if let ast::Expr::Ident(name, _) = object.as_ref() {
                        self.var_struct_types.get(name).cloned()
                    } else {
                        None
                    };

                    if let Some(type_name) = struct_type {
                        if let Some(fields) = self.struct_defs.get(&type_name) {
                            fields.iter().position(|f| f == field).map(|i| i as u32).unwrap_or(0)
                        } else {
                            0
                        }
                    } else {
                        // Try all struct_defs to find one with this field
                        let mut found = 0u32;
                        for (_sname, fields) in &self.struct_defs {
                            if let Some(idx) = fields.iter().position(|f| f == field) {
                                found = idx as u32;
                                break;
                            }
                        }
                        found
                    }
                };

                let result = self.fresh_value();
                let ty = IrType::I64;
                self.emit(Inst::FieldGet { result, object: obj, field_index, ty: ty.clone() });
                self.record_type(result, ty);
                result
            }

            // ── Phase 5 → v18: Lambda / closure with capture ──────────────────
            ast::Expr::Lambda { params, body, .. } => {
                let anon_name = format!("__lambda_{}", self.next_value);
                let lambda_params: Vec<(String, IrType)> = params.iter()
                    .map(|p| (p.name.clone(), self.type_expr_to_ir(&p.ty)))
                    .collect();
                let lambda_ret = IrType::I64;

                // v18: Collect free variables from body that exist in outer scope
                let param_names: HashSet<String> = lambda_params.iter().map(|(n,_)| n.clone()).collect();
                let free_vars = self.collect_free_vars(body, &param_names);
                let captured: Vec<(String, Value)> = free_vars.iter()
                    .filter_map(|name| {
                        self.locals.get(name).map(|v| (name.clone(), *v))
                    })
                    .collect();

                // Save outer function state
                let saved_blocks  = std::mem::take(&mut self.current_blocks);
                let saved_block   = self.current_block;
                let saved_nv      = self.next_value;
                let saved_nb      = self.next_block;
                let saved_locals  = std::mem::take(&mut self.locals);
                let saved_muts    = std::mem::take(&mut self.mutables);
                let saved_vtypes  = std::mem::take(&mut self.value_types);
                let saved_mvtypes = std::mem::take(&mut self.mutable_var_types);

                // Initialize lambda function state
                self.next_value = 0;
                self.next_block = 0;
                let entry = self.fresh_block();
                let bb = BasicBlock::new(entry);
                self.current_blocks = vec![bb];
                self.current_block = 0;

                // v18: Build params list with captures prepended as extra params
                let mut full_params = Vec::new();
                for (cap_name, _) in &captured {
                    let cap_ty = saved_locals.get(cap_name)
                        .and_then(|v| self.value_types.get(v).or_else(|| saved_vtypes.get(v)))
                        .cloned()
                        .unwrap_or(IrType::I64);
                    full_params.push((cap_name.clone(), cap_ty));
                }
                full_params.extend(lambda_params.iter().cloned());

                // Bind all params (captures + lambda params)
                for (name, ty) in &full_params {
                    let v = self.fresh_value();
                    self.locals.insert(name.clone(), v);
                    self.record_type(v, ty.clone());
                }

                // Register lambda in fn_sigs
                self.fn_sigs.insert(anon_name.clone(),
                    (full_params.iter().map(|(_, t)| t.clone()).collect(), lambda_ret.clone()));

                // Lower body — it's an Expr, not a Block
                let body_val = self.lower_expr(body);
                self.emit(Inst::Return { value: Some(body_val) });

                let ir_func = IrFunction {
                    name: anon_name.clone(),
                    params: full_params,
                    ret_type: lambda_ret,
                    blocks: std::mem::take(&mut self.current_blocks),
                    entry,
                };
                self.module.functions.push(ir_func);

                // Restore outer function state
                self.current_blocks  = saved_blocks;
                self.current_block   = saved_block;
                self.next_value      = saved_nv;
                self.next_block      = saved_nb;
                self.locals          = saved_locals;
                self.mutables        = saved_muts;
                self.value_types     = saved_vtypes;
                self.mutable_var_types = saved_mvtypes;

                let capture_vals: Vec<Value> = captured.iter().map(|(_, v)| *v).collect();
                let result = self.fresh_value();
                self.emit(Inst::ClosureAlloc {
                    result,
                    func: anon_name,
                    captures: capture_vals,
                });
                self.record_type(result, IrType::Ptr);
                result
            }

            // ── Match expression ─────────────────────────────────────────────────
            ast::Expr::Match { subject, arms, .. } => {
                let subj = self.lower_expr(subject);
                let merge_bb = self.fresh_block();

                // Collect (value, predecessor_block_id) for the Phi.
                let mut phi_incoming: Vec<(Value, BlockId)> = Vec::new();
                let mut first_arm_ty: Option<IrType> = None;

                // Build one test+body pair per arm.
                // next_test_bb is where we jump if this arm's pattern doesn't match.
                let arm_count = arms.len();
                let mut next_test_bb = self.fresh_block();

                for (i, arm) in arms.iter().enumerate() {
                    let is_last = i + 1 == arm_count;
                    let body_bb = self.fresh_block();
                    // The block where the pattern test happens:
                    let test_bb = if i == 0 {
                        // For the first arm, we're still in the current block.
                        // Emit the branch from the current block.
                        self.current_blocks[self.current_block].id
                    } else {
                        next_test_bb
                    };

                    if i > 0 {
                        self.switch_block(test_bb);
                    }

                    // Prepare the fallthrough target for failed match.
                    let fail_bb = if is_last {
                        // Last arm: if it fails, jump to merge (default 0)
                        merge_bb
                    } else {
                        let fb = self.fresh_block();
                        next_test_bb = fb;
                        fb
                    };

                    // Emit pattern test
                    match &arm.pattern {
                        ast::Pattern::Literal(lit_expr) => {
                            let pat_val = self.lower_expr(lit_expr);
                            let cmp_result = self.fresh_value();
                            if self.is_float(subj) || self.is_float(pat_val) {
                                self.emit(Inst::FCmp {
                                    result: cmp_result,
                                    cond: IrCmp::Eq,
                                    lhs: subj,
                                    rhs: pat_val,
                                });
                            } else {
                                self.emit(Inst::ICmp {
                                    result: cmp_result,
                                    cond: IrCmp::Eq,
                                    lhs: subj,
                                    rhs: pat_val,
                                });
                            }
                            self.record_type(cmp_result, IrType::Bool);

                            // If guard present, AND it with the pattern match
                            let final_cond = if let Some(guard) = &arm.guard {
                                let guard_val = self.lower_expr(guard);
                                let and_result = self.fresh_value();
                                self.emit(Inst::BinOp {
                                    result: and_result,
                                    op: IrBinOp::And,
                                    lhs: cmp_result,
                                    rhs: guard_val,
                                    ty: IrType::Bool,
                                });
                                self.record_type(and_result, IrType::Bool);
                                and_result
                            } else {
                                cmp_result
                            };

                            self.emit(Inst::Branch {
                                cond: final_cond,
                                then_bb: body_bb,
                                else_bb: fail_bb,
                            });
                        }
                        ast::Pattern::Ident(name, _) => {
                            // Ident pattern: always matches, binds subject to name.
                            self.locals.insert(name.clone(), subj);
                            self.emit(Inst::Jump { target: body_bb });
                        }
                        ast::Pattern::Wildcard(_) => {
                            // Wildcard: always matches.
                            self.emit(Inst::Jump { target: body_bb });
                        }
                        ast::Pattern::Variant { .. } | ast::Pattern::Struct { .. } => {
                            // Phase 7 scaffolding: variant/struct patterns
                            // treated as wildcard for now.
                            self.emit(Inst::Jump { target: body_bb });
                        }
                    }

                    // Body block
                    self.switch_block(body_bb);
                    let body_val = self.lower_expr(&arm.body);
                    let body_ty = self.infer_type(body_val);
                    if first_arm_ty.is_none() {
                        first_arm_ty = Some(body_ty);
                    }
                    let pred_bb = self.current_blocks[self.current_block].id;
                    self.emit(Inst::Jump { target: merge_bb });
                    phi_incoming.push((body_val, pred_bb));
                }

                // If the last arm was not a wildcard/ident, we need a default value
                // at the merge block from the last fail_bb path.
                // We handle this by adding a zero default for the merge phi.
                let result_ty = first_arm_ty.unwrap_or(IrType::I64);

                // Merge block
                self.switch_block(merge_bb);
                if phi_incoming.is_empty() {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: result_ty.clone() });
                    self.record_type(v, result_ty);
                    v
                } else {
                    let phi = self.fresh_value();
                    self.record_type(phi, result_ty.clone());
                    self.emit(Inst::Phi {
                        result: phi,
                        incoming: phi_incoming,
                        ty: result_ty,
                    });
                    phi
                }
            }

            // ── Phase 6: Struct literal ─────────────────────────────────────────
            ast::Expr::StructLiteral { name, fields, .. } => {
                let field_vals: Vec<Value> = fields.iter()
                    .map(|(_, v)| self.lower_expr(v))
                    .collect();
                let result = self.fresh_value();
                self.emit(Inst::StructAlloc {
                    result,
                    type_name: name.clone(),
                    fields: field_vals,
                });
                self.record_type(result, IrType::Ptr);
                // v18: Track result as having this struct type (for field access + method dispatch)
                // The binding happens in the Let handler below
                result
            }

            // Remaining unimplemented AST nodes — zero constant fallback.
            // ── v18: Try/Catch expression ─────────────────────────────────────
            ast::Expr::TryCatch { try_body, catch_var, catch_body, .. } => {
                // Clear error state → execute try body → check error → branch
                let clear_result = self.fresh_value();
                self.emit(Inst::Call {
                    result: clear_result,
                    func: "error_clear".to_string(),
                    args: vec![],
                    ret_ty: IrType::Void,
                });

                // Lower try body
                let try_val = self.lower_block(try_body).unwrap_or_else(|| {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::I64 });
                    self.record_type(v, IrType::I64);
                    v
                });
                let try_ty = self.infer_type(try_val);
                let try_pred_bb = self.current_blocks[self.current_block].id;

                // Check if error occurred (error_check returns i64, 0 = no error)
                let err_code = self.fresh_value();
                self.emit(Inst::Call {
                    result: err_code,
                    func: "error_check".to_string(),
                    args: vec![],
                    ret_ty: IrType::I64,
                });
                self.record_type(err_code, IrType::I64);

                // Compare: err_code != 0 → has error
                let zero = self.fresh_value();
                self.emit(Inst::IConst { result: zero, value: 0, ty: IrType::I64 });
                self.record_type(zero, IrType::I64);
                let err_check = self.fresh_value();
                self.emit(Inst::ICmp { result: err_check, cond: IrCmp::Ne, lhs: err_code, rhs: zero });
                self.record_type(err_check, IrType::Bool);

                let catch_bb = self.fresh_block();
                let merge_bb = self.fresh_block();

                self.emit(Inst::Branch {
                    cond: err_check,
                    then_bb: catch_bb,
                    else_bb: merge_bb,
                });

                // Catch block: bind error message to catch_var
                self.switch_block(catch_bb);
                let err_msg = self.fresh_value();
                self.emit(Inst::Call {
                    result: err_msg,
                    func: "error_message".to_string(),
                    args: vec![],
                    ret_ty: IrType::Ptr,
                });
                self.record_type(err_msg, IrType::Ptr);
                self.locals.insert(catch_var.clone(), err_msg);

                let catch_val = self.lower_block(catch_body).unwrap_or_else(|| {
                    let v = self.fresh_value();
                    self.emit(Inst::IConst { result: v, value: 0, ty: IrType::I64 });
                    self.record_type(v, IrType::I64);
                    v
                });
                let catch_pred_bb = self.current_blocks[self.current_block].id;
                self.emit(Inst::Jump { target: merge_bb });

                // Re-point the try success path: need to fix predecessors
                // The try_pred_bb already branches to catch_bb or merge_bb

                // Merge block
                self.switch_block(merge_bb);
                let phi = self.fresh_value();
                self.record_type(phi, try_ty.clone());
                self.emit(Inst::Phi {
                    result: phi,
                    incoming: vec![(try_val, try_pred_bb), (catch_val, catch_pred_bb)],
                    ty: try_ty,
                });
                phi
            }

            // ── v18: Throw expression ────────────────────────────────────────
            ast::Expr::Throw { code, message, .. } => {
                let code_val = self.lower_expr(code);
                let msg_val = self.lower_expr(message);

                let result = self.fresh_value();
                self.emit(Inst::Call {
                    result,
                    func: "error_set".to_string(),
                    args: vec![code_val, msg_val],
                    ret_ty: IrType::Void,
                });
                self.record_type(result, IrType::Void);
                result
            }

            // ── v18: Break / Continue ────────────────────────────────────────
            ast::Expr::Break(_) => {
                if let Some((_cont_bb, break_bb)) = self.loop_stack.last().copied() {
                    self.emit(Inst::Jump { target: break_bb });
                    // Switch to a new unreachable block so subsequent code doesn't
                    // cause Cranelift verifier errors
                    let dead_bb = self.fresh_block();
                    self.switch_block(dead_bb);
                }
                let v = self.fresh_value();
                self.emit(Inst::IConst { result: v, value: 0, ty: IrType::Void });
                v
            }
            ast::Expr::Continue(_) => {
                if let Some((cont_bb, _break_bb)) = self.loop_stack.last().copied() {
                    self.emit(Inst::Jump { target: cont_bb });
                    let dead_bb = self.fresh_block();
                    self.switch_block(dead_bb);
                }
                let v = self.fresh_value();
                self.emit(Inst::IConst { result: v, value: 0, ty: IrType::Void });
                v
            }

            _ => {
                let v = self.fresh_value();
                self.emit(Inst::IConst { result: v, value: 0, ty: IrType::I64 });
                v
            }
        }
    }

    /// v18: Collect free variable names referenced in an expression that are
    /// not in the given bound set (lambda params). Used for closure capture.
    fn collect_free_vars(&self, expr: &ast::Expr, bound: &HashSet<String>) -> Vec<String> {
        let mut free = Vec::new();
        self.collect_free_vars_inner(expr, bound, &mut free);
        free.sort();
        free.dedup();
        free
    }

    fn collect_free_vars_inner(&self, expr: &ast::Expr, bound: &HashSet<String>, out: &mut Vec<String>) {
        match expr {
            ast::Expr::Ident(name, _) => {
                if !bound.contains(name) && self.locals.contains_key(name) {
                    out.push(name.clone());
                }
            }
            ast::Expr::Binary { left, right, .. } => {
                self.collect_free_vars_inner(left, bound, out);
                self.collect_free_vars_inner(right, bound, out);
            }
            ast::Expr::Unary { operand, .. } => {
                self.collect_free_vars_inner(operand, bound, out);
            }
            ast::Expr::Call { func, args, .. } => {
                self.collect_free_vars_inner(func, bound, out);
                for a in args { self.collect_free_vars_inner(a, bound, out); }
            }
            ast::Expr::If { condition, then_branch, else_branch, .. } => {
                self.collect_free_vars_inner(condition, bound, out);
                for s in &then_branch.stmts {
                    if let ast::Stmt::Expr(e) = s { self.collect_free_vars_inner(e, bound, out); }
                }
                if let Some(t) = &then_branch.tail_expr { self.collect_free_vars_inner(t, bound, out); }
                if let Some(eb) = else_branch {
                    for s in &eb.stmts {
                        if let ast::Stmt::Expr(e) = s { self.collect_free_vars_inner(e, bound, out); }
                    }
                    if let Some(t) = &eb.tail_expr { self.collect_free_vars_inner(t, bound, out); }
                }
            }
            ast::Expr::Block(block) => {
                for s in &block.stmts {
                    if let ast::Stmt::Expr(e) = s { self.collect_free_vars_inner(e, bound, out); }
                }
                if let Some(t) = &block.tail_expr { self.collect_free_vars_inner(t, bound, out); }
            }
            _ => {
                // For other expr variants, we don't recurse deeply (conservative)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn lower_src(source: &str) -> IrModule {
        let (program, errors) = parser::parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        let builder = IrBuilder::new();
        builder.build(&program)
    }

    #[test]
    fn test_lower_simple() {
        let module = lower_src("fn main() -> i64 { 42 }");
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "main");
        assert!(!module.functions[0].blocks.is_empty());
    }

    #[test]
    fn test_lower_binop() {
        let module = lower_src("fn add() -> i64 { 1 + 2 }");
        assert_eq!(module.functions.len(), 1);
        let block = &module.functions[0].blocks[0];
        // Should have: iconst 1, iconst 2, add, return
        assert!(block.insts.len() >= 3);
    }

    #[test]
    fn test_lower_if() {
        let module = lower_src("fn test() -> i64 { if true { 1 } else { 2 } }");
        assert_eq!(module.functions.len(), 1);
        // Should have multiple blocks: entry, then, else, merge
        assert!(module.functions[0].blocks.len() >= 3);
    }

    #[test]
    fn test_lower_let_and_use() {
        let module = lower_src("fn test() -> i64 { let x: i64 = 10; x }");
        assert_eq!(module.functions.len(), 1);
    }

    #[test]
    fn test_lower_while() {
        let module = lower_src("fn test() { let mut i: i64 = 0; while i < 10 { i += 1; } }");
        assert_eq!(module.functions.len(), 1);
        // Should have blocks for condition, body, exit
        assert!(module.functions[0].blocks.len() >= 3);
    }

    #[test]
    fn test_lower_call() {
        let module = lower_src("fn foo() -> i64 { 1 } fn bar() -> i64 { foo() }");
        assert_eq!(module.functions.len(), 2);
    }

    #[test]
    fn test_ir_display() {
        let module = lower_src("fn main() -> i64 { 42 }");
        let output = format!("{}", module.functions[0]);
        assert!(output.contains("fn main"));
    }

    #[test]
    fn test_lower_match_literal() {
        let module = lower_src(r#"
            fn test() -> i64 {
                let x: i64 = 2;
                match x {
                    1 => 10,
                    2 => 20,
                    3 => 30,
                    _ => 0,
                }
            }
        "#);
        assert_eq!(module.functions.len(), 1);
        // Should have blocks for: entry, arm1-test, arm1-body, arm2-test, arm2-body, ...
        assert!(module.functions[0].blocks.len() >= 4);
    }

    #[test]
    fn test_lower_match_wildcard() {
        let module = lower_src(r#"
            fn test() -> i64 {
                match 42 {
                    _ => 99,
                }
            }
        "#);
        assert_eq!(module.functions.len(), 1);
        // Wildcard arm should always match
        assert!(module.functions[0].blocks.len() >= 2);
    }

    #[test]
    fn test_lower_match_ident_binding() {
        let module = lower_src(r#"
            fn test() -> i64 {
                match 5 {
                    1 => 10,
                    n => n,
                }
            }
        "#);
        assert_eq!(module.functions.len(), 1);
        assert!(module.functions[0].blocks.len() >= 3);
    }

    #[test]
    fn test_lower_pipe() {
        let module = lower_src(r#"
            fn double(x: i64) -> i64 { x * 2 }
            fn main() -> i64 { 5 |> double }
        "#);
        assert_eq!(module.functions.len(), 2);
        // The main function should contain a Call to double
        let main_fn = &module.functions[1];
        let has_call = main_fn.blocks.iter().any(|b| {
            b.insts.iter().any(|i| matches!(i, Inst::Call { func, .. } if func == "double"))
        });
        assert!(has_call, "Pipe should lower to Call instruction");
    }

    #[test]
    fn test_v18_module_basic() {
        let module = lower_src(r#"
module math {
    fn add(a: i64, b: i64) -> i64 {
        a + b
    }
}

fn main() -> i64 {
    math::add(10, 20)
}
        "#);
        // Should have 2 functions: math_add and main
        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.functions[0].name, "math_add");
        assert_eq!(module.functions[1].name, "main");
        // main should call math_add
        let main_fn = &module.functions[1];
        let has_call = main_fn.blocks.iter().any(|b| {
            b.insts.iter().any(|i| matches!(i, Inst::Call { func, .. } if func == "math_add"))
        });
        assert!(has_call, "main should call math_add");
    }

    #[test]
    fn test_v18_module_nested_fn() {
        let module = lower_src(r#"
module utils {
    fn double(x: i64) -> i64 { x * 2 }
    fn triple(x: i64) -> i64 { x * 3 }
}

fn main() -> i64 {
    utils::double(5) + utils::triple(3)
}
        "#);
        // Should have 3 functions: utils_double, utils_triple, main
        assert_eq!(module.functions.len(), 3);
        assert_eq!(module.functions[0].name, "utils_double");
        assert_eq!(module.functions[1].name, "utils_triple");
        assert_eq!(module.functions[2].name, "main");
    }

    // ── v20: Trait/TypeAlias/Enum/Method registry tests ─────────

    #[test]
    fn test_lower_trait_def_no_crash() {
        // Trait defs should be accepted without generating IR
        let module = lower_src("trait Drawable { fn draw(self); } fn main() -> i64 { 0 }");
        assert!(!module.functions.is_empty());
    }

    #[test]
    fn test_lower_type_alias_no_crash() {
        let module = lower_src("type Meters = f64; fn main() -> i64 { 0 }");
        assert!(!module.functions.is_empty());
    }

    #[test]
    fn test_lower_enum_definition() {
        let module = lower_src("enum Color { Red, Green, Blue } fn main() -> i64 { 0 }");
        assert!(!module.functions.is_empty());
    }

    #[test]
    fn test_lower_impl_method() {
        let module = lower_src("struct Point { x: i64, y: i64 } impl Point { fn get_x(self: Point) -> i64 { self.x } } fn main() -> i64 { 0 }");
        let has_mangled = module.functions.iter().any(|f| f.name == "Point_get_x");
        assert!(has_mangled, "Expected mangled method Point_get_x");
    }

    #[test]
    fn test_lower_impl_multiple_methods() {
        let module = lower_src("struct Vec2 { x: i64, y: i64 } impl Vec2 { fn get_x(self: Vec2) -> i64 { self.x } fn get_y(self: Vec2) -> i64 { self.y } } fn main() -> i64 { 0 }");
        let has_x = module.functions.iter().any(|f| f.name == "Vec2_get_x");
        let has_y = module.functions.iter().any(|f| f.name == "Vec2_get_y");
        assert!(has_x && has_y, "Expected both Vec2_get_x and Vec2_get_y");
    }

    #[test]
    fn test_lower_match_literal_multi_arm() {
        let module = lower_src("fn main() -> i64 { let x = 2; match x { 1 => 10, 2 => 20, _ => 0 } }");
        let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
        // Should have multiple blocks for match arms
        assert!(main_fn.blocks.len() > 1, "Match should produce multiple blocks");
    }

    #[test]
    fn test_lower_match_wildcard_only() {
        let module = lower_src("fn main() -> i64 { let x = 5; match x { _ => 42 } }");
        assert!(!module.functions.is_empty());
    }

    #[test]
    fn test_lower_lambda_captures() {
        let module = lower_src("fn main() -> i64 { let a = 10; let f = |x: i64| a + x; 0 }");
        let has_lambda = module.functions.iter().any(|f| f.name.starts_with("__lambda"));
        assert!(has_lambda, "Expected a lambda function");
    }

    #[test]
    fn test_lower_lambda_no_captures() {
        let module = lower_src("fn main() -> i64 { let f = |x: i64| x * 2; 0 }");
        let has_lambda = module.functions.iter().any(|f| f.name.starts_with("__lambda"));
        assert!(has_lambda, "Expected a lambda function");
    }

}
