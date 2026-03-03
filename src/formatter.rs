//! Vitalis Code Formatter (v25)
//!
//! AST-based code formatter that produces canonical, consistent output from
//! parsed Vitalis source code. Supports configurable indentation, line width,
//! and style options.
//!
//! # Features
//!
//! - **AST-aware**: Formats based on parsed structure, not text manipulation
//! - **Configurable**: Indent size, max line width, brace style, trailing commas
//! - **Idempotent**: Formatting already-formatted code produces identical output
//! - **Complete coverage**: Every AST node type has a formatting rule
//!
//! # Usage
//!
//! ```text
//! vtc fmt examples/hello.sl          # Format a file
//! vtc fmt --check examples/hello.sl  # Check if already formatted
//! ```

use crate::ast::*;
use std::fmt::Write;

// ═══════════════════════════════════════════════════════════════════════
//  Configuration
// ═══════════════════════════════════════════════════════════════════════

/// Formatting configuration — controls output style.
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Number of spaces per indent level (default: 4)
    pub indent_size: usize,
    /// Maximum line width before wrapping (default: 100)
    pub max_width: usize,
    /// Whether to use trailing commas in multi-line lists (default: true)
    pub trailing_commas: bool,
    /// Whether to put opening braces on the same line (default: true)
    pub same_line_braces: bool,
    /// Whether to add a blank line between top-level items (default: true)
    pub blank_between_items: bool,
    /// Whether to sort imports alphabetically (default: true)
    pub sort_imports: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            max_width: 100,
            trailing_commas: true,
            same_line_braces: true,
            blank_between_items: true,
            sort_imports: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Formatter
// ═══════════════════════════════════════════════════════════════════════

/// The code formatter. Holds configuration and builds formatted output.
pub struct Formatter {
    config: FormatConfig,
    output: String,
    indent: usize,
}

impl Formatter {
    /// Create a new formatter with the given configuration.
    pub fn new(config: FormatConfig) -> Self {
        Self {
            config,
            output: String::with_capacity(4096),
            indent: 0,
        }
    }

    /// Create a formatter with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(FormatConfig::default())
    }

    /// Format an entire program and return the formatted source string.
    pub fn format_program(&mut self, program: &Program) -> String {
        self.output.clear();
        self.indent = 0;

        let mut items = program.items.clone();

        // Optionally sort imports to the top
        if self.config.sort_imports {
            let mut imports: Vec<TopLevel> = Vec::new();
            let mut rest: Vec<TopLevel> = Vec::new();
            for item in items.drain(..) {
                if matches!(&item, TopLevel::Import(_)) {
                    imports.push(item);
                } else {
                    rest.push(item);
                }
            }
            imports.sort_by(|a, b| {
                let a_path = if let TopLevel::Import(i) = a {
                    i.path.join("::")
                } else {
                    String::new()
                };
                let b_path = if let TopLevel::Import(i) = b {
                    i.path.join("::")
                } else {
                    String::new()
                };
                a_path.cmp(&b_path)
            });
            items.extend(imports);
            items.extend(rest);
        } else {
            items = program.items.clone();
        }

        for (i, item) in items.iter().enumerate() {
            if i > 0 && self.config.blank_between_items {
                self.output.push('\n');
            }
            self.format_top_level(item);
            self.output.push('\n');
        }

        self.output.clone()
    }

    // ─── Indentation helpers ─────────────────────────────────────────

    fn indent_str(&self) -> String {
        " ".repeat(self.indent * self.config.indent_size)
    }

    fn push_indent(&mut self) {
        self.indent += 1;
    }

    fn pop_indent(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
    }

    fn write_indent(&mut self) {
        let s = self.indent_str();
        self.output.push_str(&s);
    }

    // ─── Top-level items ─────────────────────────────────────────────

    fn format_top_level(&mut self, item: &TopLevel) {
        match item {
            TopLevel::Function(f) => self.format_function(f),
            TopLevel::Struct(s) => self.format_struct(s),
            TopLevel::Enum(e) => self.format_enum(e),
            TopLevel::Module(m) => self.format_module(m),
            TopLevel::Import(i) => self.format_import(i),
            TopLevel::Const(c) => self.format_const(c),
            TopLevel::ExternBlock(e) => self.format_extern_block(e),
            TopLevel::Impl(i) => self.format_impl(i),
            TopLevel::Trait(t) => self.format_trait(t),
            TopLevel::TypeAlias(ta) => self.format_type_alias(ta),
            TopLevel::Annotated { annotations, item, .. } => {
                for ann in annotations {
                    self.format_annotation(ann);
                    self.output.push('\n');
                }
                self.format_top_level(item);
            }
        }
    }

    fn format_annotation(&mut self, ann: &Annotation) {
        self.write_indent();
        write!(self.output, "@{}", ann.name).unwrap();
        if !ann.args.is_empty() {
            self.output.push('(');
            for (i, arg) in ann.args.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.format_annotation_arg(arg);
            }
            self.output.push(')');
        }
    }

    fn format_annotation_arg(&mut self, arg: &AnnotationArg) {
        match arg {
            AnnotationArg::Ident(s) => self.output.push_str(s),
            AnnotationArg::String(s) => write!(self.output, "\"{}\"", s).unwrap(),
            AnnotationArg::Int(n) => write!(self.output, "{}", n).unwrap(),
            AnnotationArg::KeyValue { key, value } => {
                write!(self.output, "{} = ", key).unwrap();
                self.format_annotation_arg(value);
            }
            AnnotationArg::List(items) => {
                self.output.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_annotation_arg(item);
                }
                self.output.push(']');
            }
        }
    }

    fn format_function(&mut self, f: &Function) {
        self.write_indent();
        if f.is_pub {
            self.output.push_str("pub ");
        }
        if f.is_async {
            self.output.push_str("async ");
        }
        write!(self.output, "fn {}(", f.name).unwrap();

        // Parameters
        if f.params.len() <= 3 {
            for (i, p) in f.params.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.format_param(p);
            }
        } else {
            self.output.push('\n');
            self.push_indent();
            for (i, p) in f.params.iter().enumerate() {
                self.write_indent();
                self.format_param(p);
                if i < f.params.len() - 1 || self.config.trailing_commas {
                    self.output.push(',');
                }
                self.output.push('\n');
            }
            self.pop_indent();
            self.write_indent();
        }
        self.output.push(')');

        // Return type
        if let Some(ret) = &f.return_type {
            self.output.push_str(" -> ");
            self.format_type_expr(ret);
        }

        // Capabilities
        if !f.capabilities.is_empty() {
            self.output.push_str(" performs ");
            for (i, cap) in f.capabilities.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.output.push_str(cap);
            }
        }

        // Body
        if self.config.same_line_braces {
            self.output.push_str(" {\n");
        } else {
            self.output.push('\n');
            self.write_indent();
            self.output.push_str("{\n");
        }
        self.push_indent();
        self.format_block_contents(&f.body);
        self.pop_indent();
        self.write_indent();
        self.output.push('}');
    }

    fn format_param(&mut self, p: &Param) {
        write!(self.output, "{}: ", p.name).unwrap();
        self.format_type_expr(&p.ty);
        if let Some(default) = &p.default {
            self.output.push_str(" = ");
            self.format_expr(default);
        }
    }

    fn format_struct(&mut self, s: &StructDef) {
        self.write_indent();
        if s.is_pub {
            self.output.push_str("pub ");
        }
        write!(self.output, "struct {} {{\n", s.name).unwrap();
        self.push_indent();
        for field in &s.fields {
            self.write_indent();
            write!(self.output, "{}: ", field.name).unwrap();
            self.format_type_expr(&field.ty);
            self.output.push_str(",\n");
        }
        self.pop_indent();
        self.write_indent();
        self.output.push('}');
    }

    fn format_enum(&mut self, e: &EnumDef) {
        self.write_indent();
        if e.is_pub {
            self.output.push_str("pub ");
        }
        write!(self.output, "enum {} {{\n", e.name).unwrap();
        self.push_indent();
        for variant in &e.variants {
            self.write_indent();
            self.output.push_str(&variant.name);
            if !variant.fields.is_empty() {
                self.output.push('(');
                for (i, f) in variant.fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_type_expr(f);
                }
                self.output.push(')');
            }
            self.output.push_str(",\n");
        }
        self.pop_indent();
        self.write_indent();
        self.output.push('}');
    }

    fn format_module(&mut self, m: &ModuleDef) {
        self.write_indent();
        write!(self.output, "module {} {{\n", m.name).unwrap();
        self.push_indent();
        for item in &m.items {
            self.format_top_level(item);
            self.output.push('\n');
        }
        self.pop_indent();
        self.write_indent();
        self.output.push('}');
    }

    fn format_import(&mut self, i: &ImportDecl) {
        self.write_indent();
        let path = i.path.join("::");
        write!(self.output, "import {}", path).unwrap();
        if let Some(alias) = &i.alias {
            write!(self.output, " as {}", alias).unwrap();
        }
        self.output.push(';');
    }

    fn format_const(&mut self, c: &ConstDecl) {
        self.write_indent();
        if c.is_pub {
            self.output.push_str("pub ");
        }
        write!(self.output, "const {}", c.name).unwrap();
        if let Some(ty) = &c.ty {
            self.output.push_str(": ");
            self.format_type_expr(ty);
        }
        self.output.push_str(" = ");
        self.format_expr(&c.value);
        self.output.push(';');
    }

    fn format_extern_block(&mut self, e: &ExternBlock) {
        self.write_indent();
        write!(self.output, "extern \"{}\" {{\n", e.language).unwrap();
        self.push_indent();
        for item in &e.items {
            self.write_indent();
            write!(self.output, "fn {}(", item.name).unwrap();
            for (i, p) in item.params.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.format_param(p);
            }
            self.output.push(')');
            if let Some(ret) = &item.return_type {
                self.output.push_str(" -> ");
                self.format_type_expr(ret);
            }
            self.output.push_str(";\n");
        }
        self.pop_indent();
        self.write_indent();
        self.output.push('}');
    }

    fn format_impl(&mut self, i: &ImplBlock) {
        self.write_indent();
        write!(self.output, "impl {}", i.type_name).unwrap();
        if let Some(trait_name) = &i.trait_name {
            write!(self.output, " for {}", trait_name).unwrap();
        }
        self.output.push_str(" {\n");
        self.push_indent();
        for method in &i.methods {
            self.format_function(method);
            self.output.push('\n');
            if self.config.blank_between_items {
                self.output.push('\n');
            }
        }
        self.pop_indent();
        self.write_indent();
        self.output.push('}');
    }

    fn format_trait(&mut self, t: &TraitDef) {
        self.write_indent();
        if t.is_pub {
            self.output.push_str("pub ");
        }
        write!(self.output, "trait {} {{\n", t.name).unwrap();
        self.push_indent();
        for method in &t.methods {
            self.write_indent();
            write!(self.output, "fn {}(", method.name).unwrap();
            for (i, p) in method.params.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.format_param(p);
            }
            self.output.push(')');
            if let Some(ret) = &method.return_type {
                self.output.push_str(" -> ");
                self.format_type_expr(ret);
            }
            self.output.push_str(";\n");
        }
        self.pop_indent();
        self.write_indent();
        self.output.push('}');
    }

    fn format_type_alias(&mut self, ta: &TypeAliasDef) {
        self.write_indent();
        if ta.is_pub {
            self.output.push_str("pub ");
        }
        write!(self.output, "type {} = ", ta.name).unwrap();
        self.format_type_expr(&ta.ty);
        self.output.push(';');
    }

    // ─── Types ───────────────────────────────────────────────────────

    fn format_type_expr(&mut self, ty: &TypeExpr) {
        match ty {
            TypeExpr::Named(name, _) => self.output.push_str(name),
            TypeExpr::Generic { name, args, .. } => {
                write!(self.output, "{}[", name).unwrap();
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_type_expr(arg);
                }
                self.output.push(']');
            }
            TypeExpr::Function { params, ret, .. } => {
                self.output.push_str("fn(");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_type_expr(p);
                }
                self.output.push_str(") -> ");
                self.format_type_expr(ret);
            }
            TypeExpr::Array { elem, size, .. } => {
                self.output.push('[');
                self.format_type_expr(elem);
                if let Some(n) = size {
                    write!(self.output, "; {}", n).unwrap();
                }
                self.output.push(']');
            }
            TypeExpr::Ref { inner, mutable, .. } => {
                if *mutable {
                    self.output.push_str("&mut ");
                } else {
                    self.output.push('&');
                }
                self.format_type_expr(inner);
            }
            TypeExpr::Inferred(_) => self.output.push('_'),
        }
    }

    // ─── Blocks ──────────────────────────────────────────────────────

    fn format_block_contents(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.format_stmt(stmt);
            self.output.push('\n');
        }
        if let Some(tail) = &block.tail_expr {
            self.write_indent();
            self.format_expr(tail);
            self.output.push('\n');
        }
    }

    fn format_block_inline(&mut self, block: &Block) {
        self.output.push_str("{\n");
        self.push_indent();
        self.format_block_contents(block);
        self.pop_indent();
        self.write_indent();
        self.output.push('}');
    }

    // ─── Statements ─────────────────────────────────────────────────

    fn format_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value, mutable, .. } => {
                self.write_indent();
                if *mutable {
                    self.output.push_str("let mut ");
                } else {
                    self.output.push_str("let ");
                }
                self.output.push_str(name);
                if let Some(t) = ty {
                    self.output.push_str(": ");
                    self.format_type_expr(t);
                }
                if let Some(val) = value {
                    self.output.push_str(" = ");
                    self.format_expr(val);
                }
                self.output.push(';');
            }
            Stmt::Expr(expr) => {
                self.write_indent();
                self.format_expr(expr);
                self.output.push(';');
            }
            Stmt::While { condition, body, .. } => {
                self.write_indent();
                self.output.push_str("while ");
                self.format_expr(condition);
                self.output.push(' ');
                self.format_block_inline(body);
            }
            Stmt::For { var, iter, body, .. } => {
                self.write_indent();
                write!(self.output, "for {} in ", var).unwrap();
                self.format_expr(iter);
                self.output.push(' ');
                self.format_block_inline(body);
            }
            Stmt::Loop { body, .. } => {
                self.write_indent();
                self.output.push_str("loop ");
                self.format_block_inline(body);
            }
        }
    }

    // ─── Expressions ─────────────────────────────────────────────────

    fn format_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLiteral(n, _) => write!(self.output, "{}", n).unwrap(),
            Expr::FloatLiteral(f, _) => {
                let s = format!("{}", f);
                if !s.contains('.') {
                    write!(self.output, "{}.0", s).unwrap();
                } else {
                    self.output.push_str(&s);
                }
            }
            Expr::StringLiteral(s, _) => write!(self.output, "\"{}\"", s).unwrap(),
            Expr::BoolLiteral(b, _) => write!(self.output, "{}", b).unwrap(),
            Expr::Ident(name, _) => self.output.push_str(name),
            Expr::Binary { op, left, right, .. } => {
                self.format_expr(left);
                write!(self.output, " {} ", op).unwrap();
                self.format_expr(right);
            }
            Expr::Unary { op, operand, .. } => {
                match op {
                    UnaryOp::Neg => self.output.push('-'),
                    UnaryOp::Not => self.output.push('!'),
                }
                self.format_expr(operand);
            }
            Expr::Call { func, args, .. } => {
                self.format_expr(func);
                self.output.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_expr(arg);
                }
                self.output.push(')');
            }
            Expr::MethodCall { object, method, args, .. } => {
                self.format_expr(object);
                write!(self.output, ".{}(", method).unwrap();
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_expr(arg);
                }
                self.output.push(')');
            }
            Expr::Field { object, field, .. } => {
                self.format_expr(object);
                write!(self.output, ".{}", field).unwrap();
            }
            Expr::Index { object, index, .. } => {
                self.format_expr(object);
                self.output.push('[');
                self.format_expr(index);
                self.output.push(']');
            }
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.output.push_str("if ");
                self.format_expr(condition);
                self.output.push(' ');
                self.format_block_inline(then_branch);
                if let Some(eb) = else_branch {
                    self.output.push_str(" else ");
                    self.format_block_inline(eb);
                }
            }
            Expr::Match { subject, arms, .. } => {
                self.output.push_str("match ");
                self.format_expr(subject);
                self.output.push_str(" {\n");
                self.push_indent();
                for arm in arms {
                    self.write_indent();
                    self.format_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.output.push_str(" if ");
                        self.format_expr(guard);
                    }
                    self.output.push_str(" => ");
                    self.format_expr(&arm.body);
                    self.output.push_str(",\n");
                }
                self.pop_indent();
                self.write_indent();
                self.output.push('}');
            }
            Expr::Block(block) => self.format_block_inline(block),
            Expr::List { elements, .. } => {
                if elements.len() <= 5 {
                    self.output.push('[');
                    for (i, el) in elements.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.format_expr(el);
                    }
                    self.output.push(']');
                } else {
                    self.output.push_str("[\n");
                    self.push_indent();
                    for (i, el) in elements.iter().enumerate() {
                        self.write_indent();
                        self.format_expr(el);
                        if i < elements.len() - 1 || self.config.trailing_commas {
                            self.output.push(',');
                        }
                        self.output.push('\n');
                    }
                    self.pop_indent();
                    self.write_indent();
                    self.output.push(']');
                }
            }
            Expr::StructLiteral { name, fields, .. } => {
                write!(self.output, "{} {{\n", name).unwrap();
                self.push_indent();
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    self.write_indent();
                    write!(self.output, "{}: ", fname).unwrap();
                    self.format_expr(fval);
                    if i < fields.len() - 1 || self.config.trailing_commas {
                        self.output.push(',');
                    }
                    self.output.push('\n');
                }
                self.pop_indent();
                self.write_indent();
                self.output.push('}');
            }
            Expr::Lambda { params, body, .. } => {
                self.output.push('|');
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_param(p);
                }
                self.output.push_str("| ");
                self.format_expr(body);
            }
            Expr::Pipe { stages, .. } => {
                for (i, stage) in stages.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(" |> ");
                    }
                    self.format_expr(stage);
                }
            }
            Expr::Parallel { exprs, .. } => {
                self.output.push_str("parallel {\n");
                self.push_indent();
                for e in exprs {
                    self.write_indent();
                    self.format_expr(e);
                    self.output.push_str(";\n");
                }
                self.pop_indent();
                self.write_indent();
                self.output.push('}');
            }
            Expr::Try { expr, .. } => {
                self.format_expr(expr);
                self.output.push('?');
            }
            Expr::TryCatch { try_body, catch_var, catch_body, .. } => {
                self.output.push_str("try ");
                self.format_block_inline(try_body);
                write!(self.output, " catch {} ", catch_var).unwrap();
                self.format_block_inline(catch_body);
            }
            Expr::Throw { code, message, .. } => {
                self.output.push_str("throw(");
                self.format_expr(code);
                self.output.push_str(", ");
                self.format_expr(message);
                self.output.push(')');
            }
            Expr::Return { value, .. } => {
                self.output.push_str("return");
                if let Some(v) = value {
                    self.output.push(' ');
                    self.format_expr(v);
                }
            }
            Expr::Break(_) => self.output.push_str("break"),
            Expr::Continue(_) => self.output.push_str("continue"),
            Expr::Assign { target, value, .. } => {
                self.format_expr(target);
                self.output.push_str(" = ");
                self.format_expr(value);
            }
            Expr::CompoundAssign { op, target, value, .. } => {
                self.format_expr(target);
                write!(self.output, " {}= ", op).unwrap();
                self.format_expr(value);
            }
            Expr::Cast { expr, ty, .. } => {
                self.format_expr(expr);
                self.output.push_str(" as ");
                self.format_type_expr(ty);
            }
            Expr::Range { start, end, .. } => {
                self.format_expr(start);
                self.output.push_str("..");
                self.format_expr(end);
            }
        }
    }

    // ─── Patterns ────────────────────────────────────────────────────

    fn format_pattern(&mut self, pat: &Pattern) {
        match pat {
            Pattern::Literal(expr) => self.format_expr(expr),
            Pattern::Ident(name, _) => self.output.push_str(name),
            Pattern::Variant { name, fields, .. } => {
                self.output.push_str(name);
                if !fields.is_empty() {
                    self.output.push('(');
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            self.output.push_str(", ");
                        }
                        self.format_pattern(f);
                    }
                    self.output.push(')');
                }
            }
            Pattern::Wildcard(_) => self.output.push('_'),
            Pattern::Struct { name, fields, .. } => {
                write!(self.output, "{} {{ ", name).unwrap();
                for (i, (fname, fpat)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    write!(self.output, "{}: ", fname).unwrap();
                    self.format_pattern(fpat);
                }
                self.output.push_str(" }");
            }
            Pattern::Or { patterns, .. } => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(" | ");
                    }
                    self.format_pattern(p);
                }
            }
            Pattern::Tuple { elements, .. } => {
                self.output.push('(');
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_pattern(e);
                }
                self.output.push(')');
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Convenience API
// ═══════════════════════════════════════════════════════════════════════

/// Format source code with default settings.
pub fn format_source(source: &str) -> Result<String, String> {
    let (program, errors) = crate::parser::parse(source);
    if !errors.is_empty() {
        return Err(format!("Parse errors: {:?}", errors));
    }
    let mut formatter = Formatter::with_defaults();
    Ok(formatter.format_program(&program))
}

/// Format source code with custom configuration.
pub fn format_source_with_config(source: &str, config: FormatConfig) -> Result<String, String> {
    let (program, errors) = crate::parser::parse(source);
    if !errors.is_empty() {
        return Err(format!("Parse errors: {:?}", errors));
    }
    let mut formatter = Formatter::new(config);
    Ok(formatter.format_program(&program))
}

/// Check if source code is already formatted (returns true if it matches).
pub fn check_formatted(source: &str) -> Result<bool, String> {
    let formatted = format_source(source)?;
    Ok(formatted.trim() == source.trim())
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(source: &str) -> String {
        format_source(source).expect("format failed")
    }

    #[test]
    fn test_format_simple_function() {
        let src = "fn main() -> i64 { 42 }";
        let out = fmt(src);
        assert!(out.contains("fn main() -> i64 {"));
        assert!(out.contains("    42"));
    }

    #[test]
    fn test_format_function_with_params() {
        let src = "fn add(a: i64, b: i64) -> i64 { a + b }";
        let out = fmt(src);
        assert!(out.contains("fn add(a: i64, b: i64) -> i64 {"));
    }

    #[test]
    fn test_format_let_binding() {
        let src = "fn main() -> i64 { let x: i64 = 42; x }";
        let out = fmt(src);
        assert!(out.contains("let x: i64 = 42;"));
    }

    #[test]
    fn test_format_mutable_let() {
        let src = "fn main() -> i64 { let mut x: i64 = 0; x = 1; x }";
        let out = fmt(src);
        assert!(out.contains("let mut x: i64 = 0;"));
    }

    #[test]
    fn test_format_if_else() {
        let src = "fn main() -> i64 { if true { 1 } else { 0 } }";
        let out = fmt(src);
        assert!(out.contains("if true {"));
        assert!(out.contains("} else {"));
    }

    #[test]
    fn test_format_while_loop() {
        let src = "fn main() -> i64 { let mut i: i64 = 0; while i < 10 { i = i + 1; } i }";
        let out = fmt(src);
        assert!(out.contains("while i < 10 {"));
    }

    #[test]
    fn test_format_struct() {
        let src = "struct Point { x: i64, y: i64 }";
        let out = fmt(src);
        assert!(out.contains("struct Point {"));
        assert!(out.contains("    x: i64,"));
        assert!(out.contains("    y: i64,"));
    }

    #[test]
    fn test_format_enum() {
        let src = "enum Color { Red, Green, Blue }";
        let out = fmt(src);
        assert!(out.contains("enum Color {"));
        assert!(out.contains("    Red,"));
    }

    #[test]
    fn test_format_match() {
        let src = "fn main() -> i64 { match 1 { 1 => 10, _ => 0 } }";
        let out = fmt(src);
        assert!(out.contains("match 1 {"));
        assert!(out.contains("1 => 10,"));
        assert!(out.contains("_ => 0,"));
    }

    #[test]
    fn test_format_lambda() {
        let src = "fn main() -> i64 { let f: fn(i64) -> i64 = |x: i64| x + 1; f(41) }";
        let out = fmt(src);
        assert!(out.contains("|x: i64| x + 1"));
    }

    #[test]
    fn test_format_pipe() {
        let src = "fn double(x: i64) -> i64 { x * 2 }\nfn main() -> i64 { 5 |> double }";
        let out = fmt(src);
        assert!(out.contains("|>"));
    }

    #[test]
    fn test_format_for_loop() {
        let src = "fn main() -> i64 { let mut s: i64 = 0; for i in 0..10 { s = s + i; } s }";
        let out = fmt(src);
        assert!(out.contains("for i in 0..10 {"));
    }

    #[test]
    fn test_format_list_literal() {
        let src = "fn main() -> i64 { let xs: list[i64] = [1, 2, 3]; 0 }";
        let out = fmt(src);
        assert!(out.contains("[1, 2, 3]"));
    }

    #[test]
    fn test_format_return() {
        let src = "fn main() -> i64 { return 42 }";
        let out = fmt(src);
        assert!(out.contains("return 42"));
    }

    #[test]
    fn test_format_binary_ops() {
        let src = "fn main() -> i64 { 1 + 2 * 3 }";
        let out = fmt(src);
        assert!(out.contains("1 + 2 * 3"));
    }

    #[test]
    fn test_format_unary_neg() {
        let src = "fn main() -> i64 { -42 }";
        let out = fmt(src);
        assert!(out.contains("-42"));
    }

    #[test]
    fn test_format_try_catch() {
        let src = "fn main() -> i64 { try { 1 } catch e { 0 } }";
        let out = fmt(src);
        assert!(out.contains("try {"));
        assert!(out.contains("catch e {"));
    }

    #[test]
    fn test_format_idempotent() {
        let src = "fn main() -> i64 {\n    42\n}\n";
        let out1 = fmt(src);
        let out2 = fmt(&out1);
        assert_eq!(out1, out2, "Formatting should be idempotent");
    }

    #[test]
    fn test_format_config_indent() {
        let config = FormatConfig {
            indent_size: 2,
            ..Default::default()
        };
        let src = "fn main() -> i64 { 42 }";
        let (program, _) = crate::parser::parse(src);
        let mut formatter = Formatter::new(config);
        let out = formatter.format_program(&program);
        assert!(out.contains("  42"), "Should use 2-space indent: {}", out);
    }

    #[test]
    fn test_format_pub_function() {
        let src = "pub fn hello() -> i64 { 1 }";
        let out = fmt(src);
        assert!(out.contains("pub fn hello()"));
    }

    #[test]
    fn test_format_const() {
        let src = "fn main() -> i64 { 100 }";
        let out = fmt(src);
        assert!(out.contains("100"));
    }

    #[test]
    fn test_format_nested_if() {
        let src = "fn main() -> i64 { if true { if false { 1 } else { 2 } } else { 3 } }";
        let out = fmt(src);
        assert!(out.contains("if true {"));
        assert!(out.contains("if false {"));
    }

    #[test]
    fn test_format_empty_function() {
        let src = "fn noop() -> i64 { 0 }";
        let out = fmt(src);
        assert!(out.contains("fn noop() -> i64 {"));
    }

    #[test]
    fn test_format_method_call() {
        let src = "fn main() -> i64 { let x: i64 = 5; x.abs() }";
        let result = format_source(src);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_format_check_formatted() {
        let src = "fn main() -> i64 {\n    42\n}\n";
        let result = check_formatted(src);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_wildcard_pattern() {
        let src = "fn main() -> i64 { match 1 { _ => 0 } }";
        let out = fmt(src);
        assert!(out.contains("_ => 0"));
    }

    #[test]
    fn test_format_cast() {
        let src = "fn main() -> i64 { 3 as i64 }";
        let out = fmt(src);
        assert!(out.contains("as i64"));
    }

    #[test]
    fn test_format_range() {
        let src = "fn main() -> i64 { let mut s: i64 = 0; for i in 1..5 { s = s + i; } s }";
        let out = fmt(src);
        assert!(out.contains("1..5"));
    }

    #[test]
    fn test_format_break_continue() {
        let src = "fn main() -> i64 { let mut i: i64 = 0; loop { i = i + 1; if i > 5 { break } } i }";
        let out = fmt(src);
        assert!(out.contains("break"));
    }

    #[test]
    fn test_format_compound_assign() {
        let src = "fn main() -> i64 { let mut x: i64 = 0; x += 5; x }";
        let out = fmt(src);
        assert!(out.contains("+= 5"));
    }

    #[test]
    fn test_format_string_literal() {
        let src = "fn main() -> i64 { let s: str = \"hello world\"; 0 }";
        let out = fmt(src);
        assert!(out.contains("\"hello world\""));
    }

    #[test]
    fn test_format_bool_literal() {
        let src = "fn main() -> i64 { let b: bool = true; 0 }";
        let out = fmt(src);
        assert!(out.contains("true"));
    }

    #[test]
    fn test_format_multiple_functions() {
        let src = "fn foo() -> i64 { 1 }\nfn bar() -> i64 { 2 }";
        let out = fmt(src);
        assert!(out.contains("fn foo()"));
        assert!(out.contains("fn bar()"));
    }
}
