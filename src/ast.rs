//! Vitalis AST — Abstract Syntax Tree types.
//!
//! Every node carries a `Span` for error reporting.
//! The AST is designed to represent self-evolution constructs
//! (modules, evolution pipelines, memory declarations, capabilities)
//! as first-class nodes — not string annotations.

use std::fmt;

/// Byte-offset span in source code.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Provenance — tracks who created this code and its lineage.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    Human,
    Evolved { parent_version: u64 },
    LlmGenerated { model: String },
}

impl Default for Origin {
    fn default() -> Self {
        Origin::Human
    }
}

// ─── Top-Level Program ──────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<TopLevel>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TopLevel {
    Function(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Module(ModuleDef),
    Import(ImportDecl),
    Const(ConstDecl),
    ExternBlock(ExternBlock),
    /// Impl block: impl TypeName { fn methods... }
    Impl(ImplBlock),
    /// Trait definition: trait Name { fn sig; ... }
    Trait(TraitDef),
    /// Type alias: type Name = ExistingType;
    TypeAlias(TypeAliasDef),
    /// An annotation applied to the next item
    Annotated {
        annotations: Vec<Annotation>,
        item: Box<TopLevel>,
        span: Span,
    },
}

// ─── Annotations ────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct Annotation {
    pub name: String,
    pub args: Vec<AnnotationArg>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AnnotationArg {
    Ident(String),
    String(String),
    Int(i64),
    KeyValue { key: String, value: Box<AnnotationArg> },
    List(Vec<AnnotationArg>),
}

// ─── Functions ──────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub is_pub: bool,
    pub is_async: bool,
    pub capabilities: Vec<String>,
    pub origin: Origin,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

// ─── Types ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Primitive: i32, i64, f32, f64, bool, str, void
    Named(String, Span),
    /// Generic: list[T], map[K,V], option[T], result[T,E], future[T]
    Generic {
        name: String,
        args: Vec<TypeExpr>,
        span: Span,
    },
    /// Function type: fn(A, B) -> C
    Function {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
        span: Span,
    },
    /// Array: [T; N]
    Array {
        elem: Box<TypeExpr>,
        size: Option<usize>,
        span: Span,
    },
    /// Reference: &T or &mut T
    Ref {
        inner: Box<TypeExpr>,
        mutable: bool,
        span: Span,
    },
    /// Inferred type (let x = ...)
    Inferred(Span),
}

impl TypeExpr {
    pub fn span(&self) -> &Span {
        match self {
            TypeExpr::Named(_, s) => s,
            TypeExpr::Generic { span, .. } => span,
            TypeExpr::Function { span, .. } => span,
            TypeExpr::Array { span, .. } => span,
            TypeExpr::Ref { span, .. } => span,
            TypeExpr::Inferred(s) => s,
        }
    }

    pub fn is_void(&self) -> bool {
        matches!(self, TypeExpr::Named(n, _) if n == "void")
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, TypeExpr::Named(n, _) if n == "bool")
    }
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeExpr::Named(n, _) => write!(f, "{}", n),
            TypeExpr::Generic { name, args, .. } => {
                write!(f, "{}[", name)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", a)?;
                }
                write!(f, "]")
            }
            TypeExpr::Function { params, ret, .. } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            TypeExpr::Array { elem, size, .. } => {
                if let Some(n) = size {
                    write!(f, "[{}; {}]", elem, n)
                } else {
                    write!(f, "[{}]", elem)
                }
            }
            TypeExpr::Ref { inner, mutable, .. } => {
                if *mutable {
                    write!(f, "&mut {}", inner)
                } else {
                    write!(f, "&{}", inner)
                }
            }
            TypeExpr::Inferred(_) => write!(f, "_"),
        }
    }
}

// ─── Expressions ────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal: 42
    IntLiteral(i64, Span),
    /// Float literal: 3.14
    FloatLiteral(f64, Span),
    /// String literal: "hello"
    StringLiteral(String, Span),
    /// Boolean: true / false
    BoolLiteral(bool, Span),
    /// Identifier: x, foo
    Ident(String, Span),
    /// Binary op: a + b, x == y
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    /// Unary op: !x, -x
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// Function call: foo(a, b)
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Method call: obj.method(args)
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// Field access: obj.field
    Field {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Index: arr[i]
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// If expression: if cond { ... } else { ... }
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Block>,
        span: Span,
    },
    /// Match expression
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// Block expression: { stmt; stmt; expr }
    Block(Block),
    /// List literal: [1, 2, 3]
    List {
        elements: Vec<Expr>,
        span: Span,
    },
    /// Struct literal: Point { x: 1, y: 2 }
    StructLiteral {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    /// Lambda: |x, y| x + y
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
        span: Span,
    },
    /// Pipe chain: a |> b |> c
    Pipe {
        stages: Vec<Expr>,
        span: Span,
    },
    /// Parallel block: parallel { a(), b(), c() }
    Parallel {
        exprs: Vec<Expr>,
        span: Span,
    },
    /// Error propagation: expr?
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    /// Try/catch block: try { ... } catch e { ... }
    TryCatch {
        try_body: Block,
        catch_var: String,
        catch_body: Block,
        span: Span,
    },
    /// Throw an error: throw(code, "message")
    Throw {
        code: Box<Expr>,
        message: Box<Expr>,
        span: Span,
    },
    /// Return
    Return {
        value: Option<Box<Expr>>,
        span: Span,
    },
    /// Break
    Break(Span),
    /// Continue
    Continue(Span),
    /// Assignment: x = expr
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    /// Compound assignment: x += expr
    CompoundAssign {
        op: BinOp,
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    /// Cast: expr as Type
    Cast {
        expr: Box<Expr>,
        ty: TypeExpr,
        span: Span,
    },
    /// Range: start..end  (used in for-range loops)
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Expr::IntLiteral(_, s) => s,
            Expr::FloatLiteral(_, s) => s,
            Expr::StringLiteral(_, s) => s,
            Expr::BoolLiteral(_, s) => s,
            Expr::Ident(_, s) => s,
            Expr::Binary { span, .. } => span,
            Expr::Unary { span, .. } => span,
            Expr::Call { span, .. } => span,
            Expr::MethodCall { span, .. } => span,
            Expr::Field { span, .. } => span,
            Expr::Index { span, .. } => span,
            Expr::If { span, .. } => span,
            Expr::Match { span, .. } => span,
            Expr::Block(b) => &b.span,
            Expr::List { span, .. } => span,
            Expr::StructLiteral { span, .. } => span,
            Expr::Lambda { span, .. } => span,
            Expr::Pipe { span, .. } => span,
            Expr::Parallel { span, .. } => span,
            Expr::Try { span, .. } => span,
            Expr::Return { span, .. } => span,
            Expr::Break(s) => s,
            Expr::Continue(s) => s,
            Expr::Assign { span, .. } => span,
            Expr::CompoundAssign { span, .. } => span,
            Expr::Cast { span, .. } => span,
            Expr::Range { span, .. } => span,
            Expr::TryCatch { span, .. } => span,
            Expr::Throw { span, .. } => span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

impl BinOp {
    pub fn precedence(&self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Eq | BinOp::NotEq => 3,
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 4,
            BinOp::Add | BinOp::Sub => 5,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 6,
        }
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Mod => write!(f, "%"),
            BinOp::Eq => write!(f, "=="),
            BinOp::NotEq => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::LtEq => write!(f, "<="),
            BinOp::GtEq => write!(f, ">="),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

// ─── Statements ─────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Let binding: let x: T = expr;
    Let {
        name: String,
        ty: Option<TypeExpr>,
        value: Option<Expr>,
        mutable: bool,
        span: Span,
    },
    /// Expression statement: foo();
    Expr(Expr),
    /// While loop
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    /// For loop: for x in iter { ... }
    For {
        var: String,
        iter: Expr,
        body: Block,
        span: Span,
    },
    /// Loop (infinite): loop { ... }
    Loop {
        body: Block,
        span: Span,
    },
}

// ─── Block ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// The final expression (no semicolon) — becomes the block's value.
    pub tail_expr: Option<Box<Expr>>,
    pub span: Span,
}

// ─── Match Arms ─────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    /// Literal: 42, "hello", true
    Literal(Expr),
    /// Identifier binding: x
    Ident(String, Span),
    /// Enum variant: ok(value), err(e)
    Variant {
        name: String,
        fields: Vec<Pattern>,
        span: Span,
    },
    /// Wildcard: _
    Wildcard(Span),
    /// Struct pattern: Point { x, y }
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
        span: Span,
    },
    /// Or-pattern: A | B | C
    Or {
        patterns: Vec<Pattern>,
        span: Span,
    },
    /// Tuple pattern: (a, b, c)
    Tuple {
        elements: Vec<Pattern>,
        span: Span,
    },
}

// ─── Struct / Enum Definitions ──────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: TypeExpr,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<TypeExpr>,
    pub span: Span,
}
// ─── Impl Block ─────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub type_name: String,
    pub trait_name: Option<String>,
    pub methods: Vec<Function>,
    pub span: Span,
}
// ─── Trait Definition ────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<TraitMethod>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub has_default: bool,
    pub default_body: Option<Block>,
    pub span: Span,
}

// ─── Type Alias ─────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct TypeAliasDef {
    pub name: String,
    pub ty: TypeExpr,
    pub is_pub: bool,
    pub span: Span,
}

// ─── Module ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ModuleDef {
    pub name: String,
    pub items: Vec<TopLevel>,
    pub trust_tier: Option<u8>,
    pub span: Span,
}

// ─── Import ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub version: Option<String>,
    pub span: Span,
}

// ─── Const ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub is_pub: bool,
    pub span: Span,
}

// ─── Extern Block ───────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ExternBlock {
    pub language: String,
    pub items: Vec<ExternItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExternItem {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}
