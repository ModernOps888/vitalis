//! Vitalis Type System — structural typing with capability annotations.
//!
//! Types carry safety metadata (trust tiers, mutability permissions,
//! evolvability flags) that is enforced at compile time. This prevents
//! evolved code from violating safety invariants.

use crate::ast::*;
use std::collections::HashMap;
use std::fmt;

// ─── Internal Type Representation ───────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Str,
    Void,
    /// Named user type (struct / enum)
    Named(String),
    /// List[T]
    List(Box<Type>),
    /// Map[K, V]
    Map(Box<Type>, Box<Type>),
    /// Option[T]
    Option(Box<Type>),
    /// Result[T, E]
    Result(Box<Type>, Box<Type>),
    /// Future[T]
    Future(Box<Type>),
    /// Function type: (params...) -> ret
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    /// Array: [T; N]
    Array(Box<Type>, Option<usize>),
    /// Reference: &T or &mut T
    Ref {
        inner: Box<Type>,
        mutable: bool,
    },
    /// Type variable (for generics, inference)
    Var(u32),
    /// Error sentinel — used during recovery
    Error,
    /// Not yet resolved
    Unknown,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "str"),
            Type::Void => write!(f, "void"),
            Type::Named(n) => write!(f, "{}", n),
            Type::List(t) => write!(f, "list[{}]", t),
            Type::Map(k, v) => write!(f, "map[{}, {}]", k, v),
            Type::Option(t) => write!(f, "option[{}]", t),
            Type::Result(t, e) => write!(f, "result[{}, {}]", t, e),
            Type::Future(t) => write!(f, "future[{}]", t),
            Type::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Array(t, Some(n)) => write!(f, "[{}; {}]", t, n),
            Type::Array(t, None) => write!(f, "[{}]", t),
            Type::Ref { inner, mutable } => {
                if *mutable { write!(f, "&mut {}", inner) }
                else { write!(f, "&{}", inner) }
            }
            Type::Var(id) => write!(f, "?T{}", id),
            Type::Error => write!(f, "<error>"),
            Type::Unknown => write!(f, "<unknown>"),
        }
    }
}

// ─── Type Errors ────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type error at {}..{}: {}", self.span.start, self.span.end, self.message)
    }
}

// ─── Symbol Table ───────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub ty: Type,
    pub mutable: bool,
    pub evolvable: bool,
    pub trust_tier: Option<u8>,
}

#[derive(Debug)]
pub struct Scope {
    symbols: HashMap<String, SymbolInfo>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, name: String, info: SymbolInfo) {
        self.symbols.insert(name, info);
    }

    pub fn lookup(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    pub fn into_parent(self) -> Option<Scope> {
        self.parent.map(|b| *b)
    }
}

// ─── Type Checker ───────────────────────────────────────────────────────
pub struct TypeChecker {
    scope: Scope,
    errors: Vec<TypeError>,
    next_var: u32,
    /// Struct definitions: name -> fields
    struct_defs: HashMap<String, Vec<(String, Type)>>,
    /// Enum definitions: name -> variants
    enum_defs: HashMap<String, Vec<(String, Vec<Type>)>>,
    /// Function signatures for forward references
    fn_sigs: HashMap<String, (Vec<Type>, Type)>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            scope: Scope::new(),
            errors: Vec::new(),
            next_var: 0,
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            fn_sigs: HashMap::new(),
        };
        // Register built-in functions
        checker.register_builtin("print", vec![Type::Str], Type::Void);
        checker.register_builtin("println", vec![Type::Str], Type::Void);
        checker.register_builtin("assert", vec![Type::Bool], Type::Void);
        checker.register_builtin("panic", vec![Type::Str], Type::Void);
        checker.register_builtin("to_string", vec![Type::I64], Type::Str);
        checker.register_builtin("len", vec![Type::Str], Type::I64);

        // ── Phase 1 stdlib ──────────────────────────────────────────
        // I/O overloads
        checker.register_builtin("print_f64",    vec![Type::F64],  Type::Void);
        checker.register_builtin("println_f64",  vec![Type::F64],  Type::Void);
        checker.register_builtin("print_bool",   vec![Type::Bool], Type::Void);
        checker.register_builtin("println_bool", vec![Type::Bool], Type::Void);
        checker.register_builtin("print_str",    vec![Type::Str],  Type::Void);
        checker.register_builtin("println_str",  vec![Type::Str],  Type::Void);
        // Math f64 → f64
        for name in &["sqrt", "ln", "log2", "log10", "sin", "cos", "exp",
                       "floor", "ceil", "round", "abs_f64"] {
            checker.register_builtin(name, vec![Type::F64], Type::F64);
        }
        checker.register_builtin("pow",     vec![Type::F64, Type::F64], Type::F64);
        checker.register_builtin("min_f64", vec![Type::F64, Type::F64], Type::F64);
        checker.register_builtin("max_f64", vec![Type::F64, Type::F64], Type::F64);
        // Math i64
        checker.register_builtin("abs", vec![Type::I64], Type::I64);
        checker.register_builtin("min", vec![Type::I64, Type::I64], Type::I64);
        checker.register_builtin("max", vec![Type::I64, Type::I64], Type::I64);
        // Type conversions
        checker.register_builtin("to_f64",     vec![Type::I64], Type::F64);
        checker.register_builtin("to_i64",     vec![Type::F64], Type::I64);
        checker.register_builtin("i64_to_f64", vec![Type::I64], Type::F64);
        checker.register_builtin("f64_to_i64", vec![Type::F64], Type::I64);
        // String operations
        checker.register_builtin("str_len", vec![Type::Str], Type::I64);
        checker.register_builtin("str_eq",  vec![Type::Str, Type::Str], Type::Bool);
        checker.register_builtin("str_cat", vec![Type::Str, Type::Str], Type::Str);
        // Extended math
        checker.register_builtin("atan2",     vec![Type::F64, Type::F64], Type::F64);
        checker.register_builtin("hypot",     vec![Type::F64, Type::F64], Type::F64);
        checker.register_builtin("clamp",     vec![Type::F64, Type::F64, Type::F64], Type::F64);
        checker.register_builtin("clamp_f64", vec![Type::F64, Type::F64, Type::F64], Type::F64);
        checker.register_builtin("clamp_i64", vec![Type::I64, Type::I64, Type::I64], Type::I64);
        checker.register_builtin("rand_f64",  vec![], Type::F64);
        checker.register_builtin("rand_i64",  vec![], Type::I64);
        checker
    }

    fn register_builtin(&mut self, name: &str, params: Vec<Type>, ret: Type) {
        self.fn_sigs.insert(name.to_string(), (params.clone(), ret.clone()));
        self.scope.define(
            name.to_string(),
            SymbolInfo {
                ty: Type::Function { params, ret: Box::new(ret) },
                mutable: false,
                evolvable: false,
                trust_tier: None,
            },
        );
    }

    fn fresh_var(&mut self) -> Type {
        let v = self.next_var;
        self.next_var += 1;
        Type::Var(v)
    }

    fn error(&mut self, message: impl Into<String>, span: &Span) {
        self.errors.push(TypeError {
            message: message.into(),
            span: span.clone(),
        });
    }

    // ── Public Entry Point ──────────────────────────────────────────
    pub fn check(mut self, program: &Program) -> Vec<TypeError> {
        // First pass: collect signatures
        for item in &program.items {
            self.collect_signatures(item);
        }

        // Second pass: type-check bodies
        for item in &program.items {
            self.check_top_level(item);
        }

        self.errors
    }

    fn collect_signatures(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Function(f) => {
                let params: Vec<Type> = f.params.iter().map(|p| self.resolve_type_expr(&p.ty)).collect();
                let ret = f.return_type.as_ref().map(|t| self.resolve_type_expr(t)).unwrap_or(Type::Void);
                self.fn_sigs.insert(f.name.clone(), (params.clone(), ret.clone()));
                self.scope.define(
                    f.name.clone(),
                    SymbolInfo {
                        ty: Type::Function { params, ret: Box::new(ret) },
                        mutable: false,
                        evolvable: false,
                        trust_tier: None,
                    },
                );
            }
            TopLevel::Struct(s) => {
                let fields: Vec<(String, Type)> = s.fields.iter()
                    .map(|f| (f.name.clone(), self.resolve_type_expr(&f.ty)))
                    .collect();
                self.struct_defs.insert(s.name.clone(), fields);
            }
            TopLevel::Enum(e) => {
                let variants: Vec<(String, Vec<Type>)> = e.variants.iter()
                    .map(|v| (v.name.clone(), v.fields.iter().map(|t| self.resolve_type_expr(t)).collect()))
                    .collect();
                self.enum_defs.insert(e.name.clone(), variants);
            }
            TopLevel::Annotated { item, .. } => {
                self.collect_signatures(item);
            }
            TopLevel::Module(m) => {
                for sub in &m.items {
                    self.collect_signatures(sub);
                }
            }
            TopLevel::Impl(imp) => {
                // Collect impl method signatures as TypeName_method
                for method in &imp.methods {
                    let mangled = format!("{}_{}", imp.type_name, method.name);
                    let params: Vec<Type> = method.params.iter().map(|p| self.resolve_type_expr(&p.ty)).collect();
                    let ret = method.return_type.as_ref().map(|t| self.resolve_type_expr(t)).unwrap_or(Type::Void);
                    self.fn_sigs.insert(mangled.clone(), (params.clone(), ret.clone()));
                }
            }
            _ => {}
        }
    }

    // ── Resolve AST TypeExpr → internal Type ────────────────────────
    pub fn resolve_type_expr(&mut self, texpr: &TypeExpr) -> Type {
        match texpr {
            TypeExpr::Named(name, _) => match name.as_str() {
                "i32" => Type::I32,
                "i64" => Type::I64,
                "f32" => Type::F32,
                "f64" => Type::F64,
                "bool" => Type::Bool,
                "str" => Type::Str,
                "void" => Type::Void,
                _ => Type::Named(name.clone()),
            },
            TypeExpr::Generic { name, args, span } => {
                let resolved: Vec<Type> = args.iter().map(|a| self.resolve_type_expr(a)).collect();
                match name.as_str() {
                    "list" if resolved.len() == 1 => Type::List(Box::new(resolved[0].clone())),
                    "map" if resolved.len() == 2 => Type::Map(Box::new(resolved[0].clone()), Box::new(resolved[1].clone())),
                    "option" if resolved.len() == 1 => Type::Option(Box::new(resolved[0].clone())),
                    "result" if resolved.len() == 2 => Type::Result(Box::new(resolved[0].clone()), Box::new(resolved[1].clone())),
                    "future" if resolved.len() == 1 => Type::Future(Box::new(resolved[0].clone())),
                    _ => {
                        self.error(format!("unknown generic type '{}'", name), span);
                        Type::Error
                    }
                }
            }
            TypeExpr::Function { params, ret, .. } => {
                let ps: Vec<Type> = params.iter().map(|p| self.resolve_type_expr(p)).collect();
                let r = self.resolve_type_expr(ret);
                Type::Function { params: ps, ret: Box::new(r) }
            }
            TypeExpr::Array { elem, size, .. } => {
                let e = self.resolve_type_expr(elem);
                Type::Array(Box::new(e), *size)
            }
            TypeExpr::Ref { inner, mutable, .. } => {
                let i = self.resolve_type_expr(inner);
                Type::Ref { inner: Box::new(i), mutable: *mutable }
            }
            TypeExpr::Inferred(_) => self.fresh_var(),
        }
    }

    // ── Check Top-Level Items ───────────────────────────────────────
    fn check_top_level(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Function(f) => self.check_function(f),
            TopLevel::Struct(_) => {} // Already collected in first pass
            TopLevel::Enum(_) => {}
            TopLevel::Annotated { item, .. } => self.check_top_level(item),
            TopLevel::Module(m) => {
                for sub in &m.items {
                    self.check_top_level(sub);
                }
            }
            TopLevel::Const(c) => {
                let val_ty = self.check_expr(&c.value);
                if let Some(ref declared) = c.ty {
                    let expected = self.resolve_type_expr(declared);
                    self.unify(&expected, &val_ty, &c.span);
                }
                self.scope.define(
                    c.name.clone(),
                    SymbolInfo {
                        ty: val_ty,
                        mutable: false,
                        evolvable: false,
                        trust_tier: None,
                    },
                );
            }
            _ => {}
        }
    }

    fn check_function(&mut self, f: &Function) {
        // Push a new scope for the function body
        let old_scope = std::mem::replace(&mut self.scope, Scope::new());
        self.scope = Scope::child(old_scope);

        // Bind parameters
        for param in &f.params {
            let ty = self.resolve_type_expr(&param.ty);
            self.scope.define(
                param.name.clone(),
                SymbolInfo {
                    ty,
                    mutable: false,
                    evolvable: false,
                    trust_tier: None,
                },
            );
        }

        // Check body
        let body_ty = self.check_block(&f.body);

        // Verify return type
        if let Some(ref ret_texpr) = f.return_type {
            let expected = self.resolve_type_expr(ret_texpr);
            self.unify(&expected, &body_ty, &f.span);
        }

        // Pop scope
        let old_scope = self.take_scope();
        if let Some(parent) = old_scope.into_parent() {
            self.scope = parent;
        }
    }

    fn take_scope(&mut self) -> Scope {
        std::mem::replace(&mut self.scope, Scope::new())
    }

    // ── Check Block ─────────────────────────────────────────────────
    fn check_block(&mut self, block: &Block) -> Type {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        if let Some(ref tail) = block.tail_expr {
            self.check_expr(tail)
        } else {
            Type::Void
        }
    }

    // ── Check Statements ────────────────────────────────────────────
    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value, mutable, span } => {
                let val_ty = if let Some(v) = value {
                    self.check_expr(v)
                } else {
                    Type::Unknown
                };

                let declared = if let Some(texpr) = ty {
                    let expected = self.resolve_type_expr(texpr);
                    if !matches!(val_ty, Type::Unknown) {
                        self.unify(&expected, &val_ty, span);
                    }
                    expected
                } else {
                    val_ty
                };

                self.scope.define(
                    name.clone(),
                    SymbolInfo {
                        ty: declared,
                        mutable: *mutable,
                        evolvable: false,
                        trust_tier: None,
                    },
                );
            }
            Stmt::Expr(e) => {
                self.check_expr(e);
            }
            Stmt::While { condition, body, span } => {
                let cond_ty = self.check_expr(condition);
                self.unify(&Type::Bool, &cond_ty, span);
                self.check_block(body);
            }
            Stmt::For { var, iter, body, span } => {
                // Special-case: for x in start..end  → loop variable is i64
                let elem_ty = if matches!(iter, Expr::Range { .. }) {
                    self.check_expr(iter); // still type-check start/end
                    Type::I64
                } else {
                    let iter_ty = self.check_expr(iter);
                    match &iter_ty {
                        Type::List(inner) => *inner.clone(),
                        Type::Array(inner, _) => *inner.clone(),
                        _ => {
                            self.error(format!("cannot iterate over type '{}'", iter_ty), span);
                            Type::Error
                        }
                    }
                };

                let old_scope = self.take_scope();
                self.scope = Scope::child(old_scope);
                self.scope.define(
                    var.clone(),
                    SymbolInfo {
                        ty: elem_ty,
                        mutable: false,
                        evolvable: false,
                        trust_tier: None,
                    },
                );
                self.check_block(body);
                let old_scope = self.take_scope();
                if let Some(parent) = old_scope.into_parent() {
                    self.scope = parent;
                }
            }
            Stmt::Loop { body, .. } => {
                self.check_block(body);
            }
        }
    }

    // ── Check Expressions ───────────────────────────────────────────
    fn check_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::IntLiteral(_, _) => Type::I64,
            Expr::FloatLiteral(_, _) => Type::F64,
            Expr::StringLiteral(_, _) => Type::Str,
            Expr::BoolLiteral(_, _) => Type::Bool,

            Expr::Ident(name, span) => {
                if let Some(info) = self.scope.lookup(name) {
                    info.ty.clone()
                } else {
                    self.error(format!("undefined variable '{}'", name), span);
                    Type::Error
                }
            }

            Expr::Binary { op, left, right, span } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                self.check_binary_op(*op, &lt, &rt, span)
            }

            Expr::Unary { op, operand, span } => {
                let t = self.check_expr(operand);
                match op {
                    UnaryOp::Neg => {
                        if !self.is_numeric(&t) {
                            self.error(format!("cannot negate type '{}'", t), span);
                        }
                        t
                    }
                    UnaryOp::Not => {
                        self.unify(&Type::Bool, &t, span);
                        Type::Bool
                    }
                }
            }

            Expr::Call { func, args, span } => {
                let func_ty = self.check_expr(func);
                match func_ty {
                    Type::Function { params, ret } => {
                        if args.len() != params.len() {
                            self.error(
                                format!("expected {} arguments, got {}", params.len(), args.len()),
                                span,
                            );
                        }
                        for (arg, expected) in args.iter().zip(params.iter()) {
                            let arg_ty = self.check_expr(arg);
                            self.unify(expected, &arg_ty, arg.span());
                        }
                        *ret
                    }
                    Type::Error => Type::Error,
                    _ => {
                        self.error(format!("type '{}' is not callable", func_ty), span);
                        Type::Error
                    }
                }
            }

            Expr::MethodCall { object, method, args, span } => {
                let _obj_ty = self.check_expr(object);
                // For Phase 0 — method calls are loosely typed
                for arg in args {
                    self.check_expr(arg);
                }
                let _ = method;
                let _ = span;
                Type::Unknown
            }

            Expr::Field { object, field, span } => {
                let obj_ty = self.check_expr(object);
                match &obj_ty {
                    Type::Named(name) => {
                        if let Some(fields) = self.struct_defs.get(name).cloned() {
                            if let Some((_, fty)) = fields.iter().find(|(n, _)| n == field) {
                                fty.clone()
                            } else {
                                self.error(format!("struct '{}' has no field '{}'", name, field), span);
                                Type::Error
                            }
                        } else {
                            Type::Unknown
                        }
                    }
                    _ => {
                        // Allow field access on unknown types for flexibility
                        Type::Unknown
                    }
                }
            }

            Expr::Index { object, index, span } => {
                let obj_ty = self.check_expr(object);
                let idx_ty = self.check_expr(index);
                match &obj_ty {
                    Type::List(inner) => {
                        self.unify(&Type::I64, &idx_ty, span);
                        *inner.clone()
                    }
                    Type::Array(inner, _) => {
                        self.unify(&Type::I64, &idx_ty, span);
                        *inner.clone()
                    }
                    Type::Map(k, v) => {
                        self.unify(k, &idx_ty, span);
                        *v.clone()
                    }
                    _ => {
                        self.error(format!("type '{}' is not indexable", obj_ty), span);
                        Type::Error
                    }
                }
            }

            Expr::If { condition, then_branch, else_branch, span } => {
                let cond_ty = self.check_expr(condition);
                self.unify(&Type::Bool, &cond_ty, span);
                let then_ty = self.check_block(then_branch);
                if let Some(else_b) = else_branch {
                    let else_ty = self.check_block(else_b);
                    self.unify(&then_ty, &else_ty, span);
                    then_ty
                } else {
                    Type::Void
                }
            }

            Expr::Match { subject, arms, span } => {
                let _subj_ty = self.check_expr(subject);
                let mut result_ty = Type::Unknown;
                for arm in arms {
                    let arm_ty = self.check_expr(&arm.body);
                    if matches!(result_ty, Type::Unknown) {
                        result_ty = arm_ty;
                    } else {
                        self.unify(&result_ty, &arm_ty, span);
                    }
                }
                result_ty
            }

            Expr::Block(block) => self.check_block(block),

            Expr::List { elements, .. } => {
                if elements.is_empty() {
                    Type::List(Box::new(self.fresh_var()))
                } else {
                    let first = self.check_expr(&elements[0]);
                    for elem in &elements[1..] {
                        let t = self.check_expr(elem);
                        self.unify(&first, &t, elem.span());
                    }
                    Type::List(Box::new(first))
                }
            }

            Expr::StructLiteral { name, fields, span } => {
                if let Some(def_fields) = self.struct_defs.get(name).cloned() {
                    for (fname, fexpr) in fields {
                        let fty = self.check_expr(fexpr);
                        if let Some((_, expected)) = def_fields.iter().find(|(n, _)| n == fname) {
                            self.unify(expected, &fty, fexpr.span());
                        } else {
                            self.error(format!("struct '{}' has no field '{}'", name, fname), span);
                        }
                    }
                    Type::Named(name.clone())
                } else {
                    self.error(format!("unknown struct '{}'", name), span);
                    Type::Error
                }
            }

            Expr::Lambda { params, body, .. } => {
                let old_scope = self.take_scope();
                self.scope = Scope::child(old_scope);
                let param_types: Vec<Type> = params.iter().map(|p| {
                    let ty = self.resolve_type_expr(&p.ty);
                    self.scope.define(p.name.clone(), SymbolInfo {
                        ty: ty.clone(),
                        mutable: false,
                        evolvable: false,
                        trust_tier: None,
                    });
                    ty
                }).collect();
                let ret = self.check_expr(body);
                let old_scope = self.take_scope();
                if let Some(parent) = old_scope.into_parent() {
                    self.scope = parent;
                }
                Type::Function {
                    params: param_types,
                    ret: Box::new(ret),
                }
            }

            Expr::Pipe { stages, .. } => {
                // The last stage's return type is the pipe's type
                // (simplified — full version would thread types through)
                let mut last = Type::Unknown;
                for stage in stages {
                    last = self.check_expr(stage);
                }
                last
            }

            Expr::Parallel { exprs, .. } => {
                // Returns a tuple/list of all results — simplified as list
                let mut types = Vec::new();
                for e in exprs {
                    types.push(self.check_expr(e));
                }
                if types.is_empty() {
                    Type::Void
                } else {
                    // Simplified: return the type of the first for now
                    types.remove(0)
                }
            }

            Expr::Try { expr, .. } => {
                let t = self.check_expr(expr);
                match t {
                    Type::Result(ok, _) => *ok,
                    Type::Option(inner) => *inner,
                    _ => t, // Permissive for Phase 0
                }
            }

            Expr::TryCatch { try_body, catch_body, .. } => {
                let try_ty = self.check_block(try_body);
                let catch_ty = self.check_block(catch_body);
                // Both branches should have compatible types
                self.unify(&try_ty, &catch_ty, &try_body.span);
                try_ty
            }

            Expr::Throw { code, message, .. } => {
                self.check_expr(code);
                self.check_expr(message);
                Type::Void // throw never produces a value
            }

            Expr::Return { value, .. } => {
                if let Some(v) = value {
                    self.check_expr(v)
                } else {
                    Type::Void
                }
            }

            Expr::Break(_) | Expr::Continue(_) => Type::Void,

            Expr::Assign { target, value, span } => {
                let target_ty = self.check_expr(target);
                let value_ty = self.check_expr(value);
                // Check mutability
                if let Expr::Ident(name, _) = target.as_ref() {
                    if let Some(info) = self.scope.lookup(name) {
                        if !info.mutable {
                            self.error(format!("cannot assign to immutable variable '{}'", name), span);
                        }
                    }
                }
                self.unify(&target_ty, &value_ty, span);
                Type::Void
            }

            Expr::CompoundAssign { target, value, span, .. } => {
                let target_ty = self.check_expr(target);
                let value_ty = self.check_expr(value);
                self.unify(&target_ty, &value_ty, span);
                Type::Void
            }

            Expr::Cast { ty, .. } => {
                self.resolve_type_expr(ty)
            }

            Expr::Range { start, end, .. } => {
                // Range is only valid inside a for-loop iterator; check both sides
                self.check_expr(&**start);
                self.check_expr(&**end);
                Type::I64 // ranges are integer ranges
            }
        }
    }

    // ── Binary Op Type Rules ────────────────────────────────────────
    fn check_binary_op(&mut self, op: BinOp, left: &Type, right: &Type, span: &Span) -> Type {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if self.is_numeric(left) && self.is_numeric(right) {
                    self.unify(left, right, span);
                    left.clone()
                } else if op == BinOp::Add && matches!((left, right), (Type::Str, Type::Str)) {
                    Type::Str
                } else {
                    self.error(format!("cannot apply '{}' to '{}' and '{}'", op, left, right), span);
                    Type::Error
                }
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                self.unify(left, right, span);
                Type::Bool
            }
            BinOp::And | BinOp::Or => {
                self.unify(&Type::Bool, left, span);
                self.unify(&Type::Bool, right, span);
                Type::Bool
            }
        }
    }

    fn is_numeric(&self, ty: &Type) -> bool {
        matches!(ty, Type::I32 | Type::I64 | Type::F32 | Type::F64 | Type::Var(_) | Type::Unknown)
    }

    // ── Unification ─────────────────────────────────────────────────
    fn unify(&mut self, expected: &Type, actual: &Type, span: &Span) {
        // Errors / Unknown / Var always unify (permissive for Phase 0)
        if matches!(expected, Type::Error | Type::Unknown | Type::Var(_)) {
            return;
        }
        if matches!(actual, Type::Error | Type::Unknown | Type::Var(_)) {
            return;
        }
        if expected != actual {
            self.error(
                format!("type mismatch: expected '{}', found '{}'", expected, actual),
                span,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn check_src(source: &str) -> Vec<TypeError> {
        let (program, parse_errors) = parser::parse(source);
        assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);
        let checker = TypeChecker::new();
        checker.check(&program)
    }

    #[test]
    fn test_simple_function() {
        let errors = check_src("fn main() -> i64 { 42 }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_type_mismatch() {
        let errors = check_src("fn main() -> i64 { true }");
        assert!(!errors.is_empty(), "Expected a type error");
    }

    #[test]
    fn test_undefined_variable() {
        let errors = check_src("fn main() { let x: i64 = y; }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("undefined"));
    }

    #[test]
    fn test_let_binding() {
        let errors = check_src("fn main() { let x: i64 = 42; }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_binary_ops() {
        let errors = check_src("fn test() -> i64 { 1 + 2 }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_immutable_assign() {
        let errors = check_src("fn test() { let x: i64 = 1; x = 2; }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("immutable"));
    }

    #[test]
    fn test_mutable_assign() {
        let errors = check_src("fn test() { let mut x: i64 = 1; x = 2; }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_struct_field_check() {
        let errors = check_src(r#"
            struct Point { x: f64, y: f64 }
            fn test() -> f64 {
                let p: Point = Point { x: 1.0, y: 2.0 };
                p.x
            }
        "#);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_condition_must_be_bool() {
        let errors = check_src("fn test() { if 42 { } }");
        assert!(!errors.is_empty());
    }
}
