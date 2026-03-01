//! Vitalis Parser — recursive descent with error recovery.
//!
//! Transforms a flat `Vec<SpannedToken>` into a structured `ast::Program`.
//! Designed to produce partial ASTs for malformed (LLM-generated) code
//! instead of aborting on first error.

use crate::ast::*;
use crate::lexer::{SpannedToken, Token};

// ─── Parser Errors ──────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "parse error at {}..{}: {}",
            self.span.start, self.span.end, self.message
        )
    }
}

// ─── Parser State ───────────────────────────────────────────────────────
pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    errors: Vec<ParseError>,
}

pub type ParseResult<T> = Result<T, ParseError>;

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| Span::new(t.span.start, t.span.end))
            .unwrap_or_default()
    }

    /// Returns true if there was at least one newline before the current token.
    /// Used to prevent `expr\n(args)` from being parsed as a function call.
    fn peek_had_leading_newline(&self) -> bool {
        self.tokens
            .get(self.pos)
            .map(|t| t.has_leading_newline)
            .unwrap_or(false)
    }

    fn advance(&mut self) -> Option<&SpannedToken> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn expect(&mut self, expected: &Token) -> ParseResult<Span> {
        if let Some(tok) = self.peek() {
            if std::mem::discriminant(tok) == std::mem::discriminant(expected) {
                let span = self.peek_span();
                self.advance();
                return Ok(span);
            }
            let span = self.peek_span();
            Err(ParseError {
                message: format!("expected '{}', found '{}'", expected, tok),
                span,
            })
        } else {
            Err(ParseError {
                message: format!("expected '{}', found end of input", expected),
                span: self.eof_span(),
            })
        }
    }

    fn expect_ident(&mut self) -> ParseResult<(String, Span)> {
        if let Some(Token::Ident(_)) = self.peek() {
            let span = self.peek_span();
            if let Some(st) = self.advance() {
                if let Token::Ident(name) = &st.token {
                    return Ok((name.clone(), span));
                }
            }
        }
        let span = self.peek_span();
        Err(ParseError {
            message: format!(
                "expected identifier, found '{}'",
                self.peek().map(|t| format!("{}", t)).unwrap_or("EOF".into())
            ),
            span,
        })
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek()
            .map(|t| std::mem::discriminant(t) == std::mem::discriminant(expected))
            .unwrap_or(false)
    }

    fn eat(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn eof_span(&self) -> Span {
        if let Some(last) = self.tokens.last() {
            Span::new(last.span.end, last.span.end)
        } else {
            Span::default()
        }
    }

    /// Synchronize after an error — skip tokens until we find a likely statement start.
    fn synchronize(&mut self) {
        while !self.at_end() {
            if self.eat(&Token::Semicolon) {
                return;
            }
            match self.peek() {
                Some(
                    Token::Fn
                    | Token::Let
                    | Token::If
                    | Token::While
                    | Token::For
                    | Token::Loop
                    | Token::Return
                    | Token::Struct
                    | Token::Enum
                    | Token::Import
                    | Token::Pub
                    | Token::Evolve
                    | Token::Memory,
                ) => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ── Entry Point ─────────────────────────────────────────────────
    pub fn parse(mut self) -> (Program, Vec<ParseError>) {
        let start = self.peek_span();
        let mut items = Vec::new();

        while !self.at_end() {
            match self.parse_top_level() {
                Ok(item) => items.push(item),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
        }

        let end = self.eof_span();
        let span = if items.is_empty() {
            start
        } else {
            start.merge(&end)
        };

        (Program { items, span }, self.errors)
    }

    // ── Top-Level Items ─────────────────────────────────────────────
    fn parse_top_level(&mut self) -> ParseResult<TopLevel> {
        // Collect annotations
        let mut annotations = Vec::new();
        while let Some(Token::Annotation(_)) = self.peek() {
            annotations.push(self.parse_annotation()?);
        }

        let item = self.parse_top_level_inner()?;

        if annotations.is_empty() {
            Ok(item)
        } else {
            let span = annotations[0].span.merge(self.item_span(&item));
            Ok(TopLevel::Annotated {
                annotations,
                item: Box::new(item),
                span,
            })
        }
    }

    fn item_span<'a>(&'a self, item: &'a TopLevel) -> &'a Span {
        match item {
            TopLevel::Function(f) => &f.span,
            TopLevel::Struct(s) => &s.span,
            TopLevel::Enum(e) => &e.span,
            TopLevel::Module(m) => &m.span,
            TopLevel::Import(i) => &i.span,
            TopLevel::Const(c) => &c.span,
            TopLevel::ExternBlock(e) => &e.span,
            TopLevel::Annotated { span, .. } => span,
        }
    }

    fn parse_annotation(&mut self) -> ParseResult<Annotation> {
        let span = self.peek_span();
        let name = if let Some(Token::Annotation(n)) = self.peek().cloned() {
            self.advance();
            n
        } else {
            return Err(ParseError {
                message: "expected annotation".into(),
                span,
            });
        };

        let mut args = Vec::new();
        if self.eat(&Token::LParen) {
            while !self.check(&Token::RParen) && !self.at_end() {
                args.push(self.parse_annotation_arg()?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::RParen)?;
        }

        let end = self.peek_span();
        Ok(Annotation {
            name,
            args,
            span: span.merge(&end),
        })
    }

    fn parse_annotation_arg(&mut self) -> ParseResult<AnnotationArg> {
        match self.peek().cloned() {
            Some(Token::StringLiteral(s)) => {
                self.advance();
                Ok(AnnotationArg::String(s))
            }
            Some(Token::IntLiteral(n)) => {
                self.advance();
                Ok(AnnotationArg::Int(n))
            }
            Some(Token::Ident(name)) => {
                self.advance();
                if self.eat(&Token::Eq) {
                    let val = self.parse_annotation_arg()?;
                    Ok(AnnotationArg::KeyValue {
                        key: name,
                        value: Box::new(val),
                    })
                } else {
                    Ok(AnnotationArg::Ident(name))
                }
            }
            _ => {
                let span = self.peek_span();
                Err(ParseError {
                    message: "expected annotation argument".into(),
                    span,
                })
            }
        }
    }

    fn parse_top_level_inner(&mut self) -> ParseResult<TopLevel> {
        let is_pub = self.eat(&Token::Pub);

        match self.peek() {
            Some(Token::Fn) | Some(Token::Async) => {
                let func = self.parse_function(is_pub)?;
                Ok(TopLevel::Function(func))
            }
            Some(Token::Struct) => {
                let s = self.parse_struct(is_pub)?;
                Ok(TopLevel::Struct(s))
            }
            Some(Token::Enum) => {
                let e = self.parse_enum(is_pub)?;
                Ok(TopLevel::Enum(e))
            }
            Some(Token::Module) => {
                let m = self.parse_module()?;
                Ok(TopLevel::Module(m))
            }
            Some(Token::Import) => {
                let i = self.parse_import()?;
                Ok(TopLevel::Import(i))
            }
            Some(Token::Let) => {
                let c = self.parse_const(is_pub)?;
                Ok(TopLevel::Const(c))
            }
            Some(Token::Extern) => {
                let e = self.parse_extern_block()?;
                Ok(TopLevel::ExternBlock(e))
            }
            _ => {
                let span = self.peek_span();
                Err(ParseError {
                    message: format!(
                        "expected top-level item (fn, struct, enum, ...), found '{}'",
                        self.peek().map(|t| format!("{}", t)).unwrap_or("EOF".into())
                    ),
                    span,
                })
            }
        }
    }

    // ── Function Definition ─────────────────────────────────────────
    fn parse_function(&mut self, is_pub: bool) -> ParseResult<Function> {
        let start = self.peek_span();
        let is_async = self.eat(&Token::Async);
        self.expect(&Token::Fn)?;
        let (name, _) = self.expect_ident()?;

        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;

        let return_type = if self.eat(&Token::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        let end_span = body.span.clone();

        Ok(Function {
            name,
            params,
            return_type,
            body,
            is_pub,
            is_async,
            capabilities: Vec::new(),
            origin: Origin::Human,
            span: start.merge(&end_span),
        })
    }

    fn parse_params(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();
        while !self.check(&Token::RParen) && !self.at_end() {
            params.push(self.parse_param()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> ParseResult<Param> {
        let span = self.peek_span();
        let (name, _) = self.expect_ident()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;

        let default = if self.eat(&Token::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let end = self.peek_span();
        Ok(Param {
            name,
            ty,
            default,
            span: span.merge(&end),
        })
    }

    // ── Type Expressions ────────────────────────────────────────────
    fn parse_type(&mut self) -> ParseResult<TypeExpr> {
        // &T or &mut T
        if self.eat(&Token::Star) {
            // We use & in the language but * as a lexer kludge; revisit
            let mutable = self.eat(&Token::Mut);
            let inner = self.parse_type()?;
            let span = inner.span().clone();
            return Ok(TypeExpr::Ref {
                inner: Box::new(inner),
                mutable,
                span,
            });
        }

        // fn(params) -> ret
        if self.check(&Token::Fn) {
            return self.parse_fn_type();
        }

        // [T; N] or [T]
        if self.check(&Token::LBracket) {
            return self.parse_array_type();
        }

        // Named or generic type
        let span = self.peek_span();
        let name = self.parse_type_name()?;

        // Generic args: name[T, U]
        if self.check(&Token::LBracket) {
            self.advance();
            let mut args = Vec::new();
            while !self.check(&Token::RBracket) && !self.at_end() {
                args.push(self.parse_type()?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            let end = self.peek_span();
            self.expect(&Token::RBracket)?;
            return Ok(TypeExpr::Generic {
                name,
                args,
                span: span.merge(&end),
            });
        }

        Ok(TypeExpr::Named(name, span))
    }

    fn parse_type_name(&mut self) -> ParseResult<String> {
        match self.peek() {
            Some(Token::I32) => { self.advance(); Ok("i32".into()) }
            Some(Token::I64) => { self.advance(); Ok("i64".into()) }
            Some(Token::F32) => { self.advance(); Ok("f32".into()) }
            Some(Token::F64) => { self.advance(); Ok("f64".into()) }
            Some(Token::Bool) => { self.advance(); Ok("bool".into()) }
            Some(Token::Str) => { self.advance(); Ok("str".into()) }
            Some(Token::Void) => { self.advance(); Ok("void".into()) }
            Some(Token::Ident(_)) => {
                let (name, _) = self.expect_ident()?;
                Ok(name)
            }
            _ => {
                let span = self.peek_span();
                Err(ParseError {
                    message: format!(
                        "expected type name, found '{}'",
                        self.peek().map(|t| format!("{}", t)).unwrap_or("EOF".into())
                    ),
                    span,
                })
            }
        }
    }

    fn parse_fn_type(&mut self) -> ParseResult<TypeExpr> {
        let start = self.peek_span();
        self.expect(&Token::Fn)?;
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while !self.check(&Token::RParen) && !self.at_end() {
            params.push(self.parse_type()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::Arrow)?;
        let ret = self.parse_type()?;
        let end = ret.span().clone();
        Ok(TypeExpr::Function {
            params,
            ret: Box::new(ret),
            span: start.merge(&end),
        })
    }

    fn parse_array_type(&mut self) -> ParseResult<TypeExpr> {
        let start = self.peek_span();
        self.expect(&Token::LBracket)?;
        let elem = self.parse_type()?;
        let size = if self.eat(&Token::Semicolon) {
            if let Some(Token::IntLiteral(n)) = self.peek().cloned() {
                self.advance();
                Some(n as usize)
            } else {
                None
            }
        } else {
            None
        };
        let end = self.peek_span();
        self.expect(&Token::RBracket)?;
        Ok(TypeExpr::Array {
            elem: Box::new(elem),
            size,
            span: start.merge(&end),
        })
    }

    // ── Struct Definition ───────────────────────────────────────────
    fn parse_struct(&mut self, is_pub: bool) -> ParseResult<StructDef> {
        let start = self.peek_span();
        self.expect(&Token::Struct)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let fspan = self.peek_span();
            let fpub = self.eat(&Token::Pub);
            let (fname, _) = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let fty = self.parse_type()?;
            let fend = self.peek_span();
            fields.push(StructField {
                name: fname,
                ty: fty,
                is_pub: fpub,
                span: fspan.merge(&fend),
            });
            if !self.eat(&Token::Comma) {
                break;
            }
        }

        let end = self.peek_span();
        self.expect(&Token::RBrace)?;

        Ok(StructDef {
            name,
            fields,
            is_pub,
            span: start.merge(&end),
        })
    }

    // ── Enum Definition ─────────────────────────────────────────────
    fn parse_enum(&mut self, is_pub: bool) -> ParseResult<EnumDef> {
        let start = self.peek_span();
        self.expect(&Token::Enum)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut variants = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let vspan = self.peek_span();
            let (vname, _) = self.expect_ident()?;
            let mut fields = Vec::new();
            if self.eat(&Token::LParen) {
                while !self.check(&Token::RParen) && !self.at_end() {
                    fields.push(self.parse_type()?);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::RParen)?;
            }
            let vend = self.peek_span();
            variants.push(EnumVariant {
                name: vname,
                fields,
                span: vspan.merge(&vend),
            });
            if !self.eat(&Token::Comma) {
                break;
            }
        }

        let end = self.peek_span();
        self.expect(&Token::RBrace)?;

        Ok(EnumDef {
            name,
            variants,
            is_pub,
            span: start.merge(&end),
        })
    }

    // ── Module Definition ───────────────────────────────────────────
    fn parse_module(&mut self) -> ParseResult<ModuleDef> {
        let start = self.peek_span();
        self.expect(&Token::Module)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut items = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            match self.parse_top_level() {
                Ok(item) => items.push(item),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
        }

        let end = self.peek_span();
        self.expect(&Token::RBrace)?;

        Ok(ModuleDef {
            name,
            items,
            trust_tier: None,
            span: start.merge(&end),
        })
    }

    // ── Import Declaration ──────────────────────────────────────────
    fn parse_import(&mut self) -> ParseResult<ImportDecl> {
        let start = self.peek_span();
        self.expect(&Token::Import)?;

        let mut path = Vec::new();
        let (first, _) = self.expect_ident()?;
        path.push(first);

        while self.eat(&Token::ColonColon) {
            let (seg, _) = self.expect_ident()?;
            path.push(seg);
        }

        let alias = if self.eat(&Token::As) {
            let (a, _) = self.expect_ident()?;
            Some(a)
        } else {
            None
        };

        let end = self.peek_span();
        self.eat(&Token::Semicolon);

        Ok(ImportDecl {
            path,
            alias,
            version: None,
            span: start.merge(&end),
        })
    }

    // ── Const Declaration ───────────────────────────────────────────
    fn parse_const(&mut self, is_pub: bool) -> ParseResult<ConstDecl> {
        let start = self.peek_span();
        self.expect(&Token::Let)?;
        let (name, _) = self.expect_ident()?;

        let ty = if self.eat(&Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        let end = self.peek_span();
        self.eat(&Token::Semicolon);

        Ok(ConstDecl {
            name,
            ty,
            value,
            is_pub,
            span: start.merge(&end),
        })
    }

    // ── Extern Block ────────────────────────────────────────────────
    fn parse_extern_block(&mut self) -> ParseResult<ExternBlock> {
        let start = self.peek_span();
        self.expect(&Token::Extern)?;

        let language = if let Some(Token::StringLiteral(s)) = self.peek().cloned() {
            self.advance();
            s
        } else {
            "C".to_string()
        };

        self.expect(&Token::LBrace)?;

        let mut items = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let ispan = self.peek_span();
            self.expect(&Token::Fn)?;
            let (iname, _) = self.expect_ident()?;
            self.expect(&Token::LParen)?;
            let iparams = self.parse_params()?;
            self.expect(&Token::RParen)?;
            let iret = if self.eat(&Token::Arrow) {
                Some(self.parse_type()?)
            } else {
                None
            };
            let iend = self.peek_span();
            self.eat(&Token::Semicolon);
            items.push(ExternItem {
                name: iname,
                params: iparams,
                return_type: iret,
                span: ispan.merge(&iend),
            });
        }

        let end = self.peek_span();
        self.expect(&Token::RBrace)?;

        Ok(ExternBlock {
            language,
            items,
            span: start.merge(&end),
        })
    }

    // ── Block ───────────────────────────────────────────────────────
    fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.peek_span();
        self.expect(&Token::LBrace)?;

        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while !self.check(&Token::RBrace) && !self.at_end() {
            // Try parsing a statement
            match self.parse_stmt() {
                Ok(stmt) => {
                    // Check if this is an expression statement without semicolon (tail expr)
                    stmts.push(stmt);
                }
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
        }

        // The last statement, if it's an Expr without semicolon, is the tail
        // For simplicity, we check if the last stmt is an Expr
        if let Some(Stmt::Expr(_)) = stmts.last() {
            if let Some(Stmt::Expr(e)) = stmts.pop() {
                tail_expr = Some(Box::new(e));
            }
        }

        let end = self.peek_span();
        self.expect(&Token::RBrace)?;

        Ok(Block {
            stmts,
            tail_expr,
            span: start.merge(&end),
        })
    }

    // ── Statements ──────────────────────────────────────────────────
    fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        match self.peek() {
            Some(Token::Let) => self.parse_let_stmt(),
            Some(Token::While) => self.parse_while_stmt(),
            Some(Token::For) => self.parse_for_stmt(),
            Some(Token::Loop) => self.parse_loop_stmt(),
            _ => {
                let expr = self.parse_expr()?;
                // If followed by semicolon, it's a statement; otherwise might be tail expr
                if self.eat(&Token::Semicolon) {
                    Ok(Stmt::Expr(expr))
                } else {
                    // Might be a tail expression — caller decides
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_let_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.peek_span();
        self.expect(&Token::Let)?;
        let mutable = self.eat(&Token::Mut);
        let (name, _) = self.expect_ident()?;

        let ty = if self.eat(&Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let value = if self.eat(&Token::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let end = self.peek_span();
        self.eat(&Token::Semicolon);

        Ok(Stmt::Let {
            name,
            ty,
            value,
            mutable,
            span: start.merge(&end),
        })
    }

    fn parse_while_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.peek_span();
        self.expect(&Token::While)?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        let end = body.span.clone();

        Ok(Stmt::While {
            condition,
            body,
            span: start.merge(&end),
        })
    }

    fn parse_for_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.peek_span();
        self.expect(&Token::For)?;
        let (var, _) = self.expect_ident()?;
        self.expect(&Token::In)?;
        let iter = self.parse_expr()?;
        let body = self.parse_block()?;
        let end = body.span.clone();

        Ok(Stmt::For {
            var,
            iter,
            body,
            span: start.merge(&end),
        })
    }

    fn parse_loop_stmt(&mut self) -> ParseResult<Stmt> {
        let start = self.peek_span();
        self.expect(&Token::Loop)?;
        let body = self.parse_block()?;
        let end = body.span.clone();

        Ok(Stmt::Loop {
            body,
            span: start.merge(&end),
        })
    }

    // ── Expressions — Pratt Parser with precedence climbing ─────────
    fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> ParseResult<Expr> {
        let expr = self.parse_pipe()?;

        if self.eat(&Token::Eq) {
            let value = self.parse_expr()?;
            let span = expr.span().merge(value.span());
            return Ok(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
                span,
            });
        }

        // Compound assignment
        let compound_op = match self.peek() {
            Some(Token::PlusEq) => Some(BinOp::Add),
            Some(Token::MinusEq) => Some(BinOp::Sub),
            Some(Token::StarEq) => Some(BinOp::Mul),
            Some(Token::SlashEq) => Some(BinOp::Div),
            _ => None,
        };

        if let Some(op) = compound_op {
            self.advance();
            let value = self.parse_expr()?;
            let span = expr.span().merge(value.span());
            return Ok(Expr::CompoundAssign {
                op,
                target: Box::new(expr),
                value: Box::new(value),
                span,
            });
        }

        Ok(expr)
    }

    fn parse_pipe(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_or()?;

        if self.check(&Token::PipeArrow) {
            let start = expr.span().clone();
            let mut stages = vec![expr];
            while self.eat(&Token::PipeArrow) {
                stages.push(self.parse_or()?);
            }
            let end = stages.last().unwrap().span().clone();
            expr = Expr::Pipe {
                stages,
                span: start.merge(&end),
            };
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_and()?;
        while self.eat(&Token::OrOr) {
            let right = self.parse_and()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_range()?;
        while self.eat(&Token::AndAnd) {
            let right = self.parse_range()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_range(&mut self) -> ParseResult<Expr> {
        let left = self.parse_equality()?;
        if self.eat(&Token::DotDot) {
            let right = self.parse_equality()?;
            let span = left.span().merge(right.span());
            return Ok(Expr::Range {
                start: Box::new(left),
                end: Box::new(right),
                span,
            });
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Some(Token::EqEq) => BinOp::Eq,
                Some(Token::NotEq) => BinOp::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Some(Token::Lt) => BinOp::Lt,
                Some(Token::Gt) => BinOp::Gt,
                Some(Token::LtEq) => BinOp::LtEq,
                Some(Token::GtEq) => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::Percent) => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> ParseResult<Expr> {
        match self.peek() {
            Some(Token::Minus) => {
                let start = self.peek_span();
                self.advance();
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span());
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            Some(Token::Bang) => {
                let start = self.peek_span();
                self.advance();
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span());
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                // Function call: expr(args) — only if ( is on the same line (no leading newline)
                Some(Token::LParen) if !self.peek_had_leading_newline() => {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&Token::RParen) && !self.at_end() {
                        args.push(self.parse_expr()?);
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    let end = self.peek_span();
                    self.expect(&Token::RParen)?;
                    let span = expr.span().merge(&end);
                    expr = Expr::Call {
                        func: Box::new(expr),
                        args,
                        span,
                    };
                }
                // Field access or method call: expr.field or expr.method(args)
                Some(Token::Dot) => {
                    self.advance();
                    let (field, _) = self.expect_ident()?;
                    if self.check(&Token::LParen) {
                        self.advance();
                        let mut args = Vec::new();
                        while !self.check(&Token::RParen) && !self.at_end() {
                            args.push(self.parse_expr()?);
                            if !self.eat(&Token::Comma) {
                                break;
                            }
                        }
                        let end = self.peek_span();
                        self.expect(&Token::RParen)?;
                        let span = expr.span().merge(&end);
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span,
                        };
                    } else {
                        let end = self.peek_span();
                        let span = expr.span().merge(&end);
                        expr = Expr::Field {
                            object: Box::new(expr),
                            field,
                            span,
                        };
                    }
                }
                // Index: expr[index]
                Some(Token::LBracket) => {
                    self.advance();
                    let index = self.parse_expr()?;
                    let end = self.peek_span();
                    self.expect(&Token::RBracket)?;
                    let span = expr.span().merge(&end);
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                // Try operator: expr?
                Some(Token::Question) => {
                    let end = self.peek_span();
                    self.advance();
                    let span = expr.span().merge(&end);
                    expr = Expr::Try {
                        expr: Box::new(expr),
                        span,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> ParseResult<Expr> {
        match self.peek().cloned() {
            // Integer literal
            Some(Token::IntLiteral(n)) => {
                let span = self.peek_span();
                self.advance();
                Ok(Expr::IntLiteral(n, span))
            }
            // Float literal
            Some(Token::FloatLiteral(n)) => {
                let span = self.peek_span();
                self.advance();
                Ok(Expr::FloatLiteral(n, span))
            }
            // String literal
            Some(Token::StringLiteral(s)) => {
                let span = self.peek_span();
                self.advance();
                Ok(Expr::StringLiteral(s, span))
            }
            // Boolean
            Some(Token::True) => {
                let span = self.peek_span();
                self.advance();
                Ok(Expr::BoolLiteral(true, span))
            }
            Some(Token::False) => {
                let span = self.peek_span();
                self.advance();
                Ok(Expr::BoolLiteral(false, span))
            }
            // Identifier (could be struct literal: Name { ... })
            Some(Token::Ident(name)) => {
                let span = self.peek_span();
                self.advance();

                // Check for struct literal: Name { field: val, ... }
                if self.check(&Token::LBrace) {
                    // Lookahead: if it's `ident { ident :` then struct literal,
                    // otherwise treat as block expression
                    if self.is_struct_literal_ahead() {
                        self.advance(); // eat {
                        let mut fields = Vec::new();
                        while !self.check(&Token::RBrace) && !self.at_end() {
                            let (fname, _) = self.expect_ident()?;
                            self.expect(&Token::Colon)?;
                            let fval = self.parse_expr()?;
                            fields.push((fname, fval));
                            if !self.eat(&Token::Comma) {
                                break;
                            }
                        }
                        let end = self.peek_span();
                        self.expect(&Token::RBrace)?;
                        return Ok(Expr::StructLiteral {
                            name,
                            fields,
                            span: span.merge(&end),
                        });
                    }
                }

                Ok(Expr::Ident(name, span))
            }
            // Parenthesized expression or lambda
            Some(Token::LParen) => {
                let start = self.peek_span();
                self.advance();
                let expr = self.parse_expr()?;
                let end = self.peek_span();
                self.expect(&Token::RParen)?;
                // Wrapped expression inherits inner span but we could merge
                let _ = start.merge(&end);
                Ok(expr)
            }
            // Block expression
            Some(Token::LBrace) => {
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }
            // If expression
            Some(Token::If) => self.parse_if_expr(),
            // Match expression
            Some(Token::Match) => self.parse_match_expr(),
            // Return
            Some(Token::Return) => {
                let start = self.peek_span();
                self.advance();
                let value = if self.check(&Token::Semicolon)
                    || self.check(&Token::RBrace)
                    || self.at_end()
                {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                let end = value
                    .as_ref()
                    .map(|v| v.span().clone())
                    .unwrap_or(start.clone());
                Ok(Expr::Return {
                    value,
                    span: start.merge(&end),
                })
            }
            // Break
            Some(Token::Break) => {
                let span = self.peek_span();
                self.advance();
                Ok(Expr::Break(span))
            }
            // Continue
            Some(Token::Continue) => {
                let span = self.peek_span();
                self.advance();
                Ok(Expr::Continue(span))
            }
            // Parallel block
            Some(Token::Parallel) => self.parse_parallel_expr(),
            // List literal: [a, b, c]
            Some(Token::LBracket) => self.parse_list_expr(),
            // Lambda / closure: |params| expr  or  |params| -> Type { block }
            // Phase 5 readiness: `Pipe` is the single `|` token.
            Some(Token::Pipe) => self.parse_lambda_expr(),
            _ => {
                let span = self.peek_span();
                Err(ParseError {
                    message: format!(
                        "expected expression, found '{}'",
                        self.peek().map(|t| format!("{}", t)).unwrap_or("EOF".into())
                    ),
                    span,
                })
            }
        }
    }

    /// Lookahead to determine if `{` starts a struct literal or a block.
    /// We check if the pattern is `ident : expr` after `{`.
    fn is_struct_literal_ahead(&self) -> bool {
        // pos is at `{` — look ahead for `ident` then `:`
        let after_brace = self.pos + 1;
        if let Some(t1) = self.tokens.get(after_brace) {
            if matches!(t1.token, Token::Ident(_)) {
                if let Some(t2) = self.tokens.get(after_brace + 1) {
                    return matches!(t2.token, Token::Colon);
                }
            }
        }
        false
    }

    fn parse_if_expr(&mut self) -> ParseResult<Expr> {
        let start = self.peek_span();
        self.expect(&Token::If)?;
        let condition = self.parse_expr()?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.eat(&Token::Else) {
            if self.check(&Token::If) {
                // else if — wrap in a block with a single if expr
                let inner = self.parse_if_expr()?;
                let span = inner.span().clone();
                Some(Block {
                    stmts: Vec::new(),
                    tail_expr: Some(Box::new(inner)),
                    span,
                })
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        let end = else_branch
            .as_ref()
            .map(|b| b.span.clone())
            .unwrap_or(then_branch.span.clone());

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
            span: start.merge(&end),
        })
    }

    fn parse_match_expr(&mut self) -> ParseResult<Expr> {
        let start = self.peek_span();
        self.expect(&Token::Match)?;
        let subject = self.parse_expr()?;
        self.expect(&Token::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let aspan = self.peek_span();
            let pattern = self.parse_pattern()?;

            let guard = if self.eat(&Token::If) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            self.expect(&Token::FatArrow)?;
            let body = self.parse_expr()?;
            let aend = body.span().clone();
            self.eat(&Token::Comma);

            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: aspan.merge(&aend),
            });
        }

        let end = self.peek_span();
        self.expect(&Token::RBrace)?;

        Ok(Expr::Match {
            subject: Box::new(subject),
            arms,
            span: start.merge(&end),
        })
    }

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        match self.peek().cloned() {
            // Wildcard: _
            Some(Token::Ident(ref s)) if s == "_" => {
                let span = self.peek_span();
                self.advance();
                Ok(Pattern::Wildcard(span))
            }
            // Ident pattern (possibly variant with fields)
            Some(Token::Ident(name)) => {
                let span = self.peek_span();
                self.advance();
                if self.eat(&Token::LParen) {
                    let mut fields = Vec::new();
                    while !self.check(&Token::RParen) && !self.at_end() {
                        fields.push(self.parse_pattern()?);
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    let end = self.peek_span();
                    self.expect(&Token::RParen)?;
                    Ok(Pattern::Variant {
                        name,
                        fields,
                        span: span.merge(&end),
                    })
                } else {
                    Ok(Pattern::Ident(name, span))
                }
            }
            // Literal patterns
            Some(Token::IntLiteral(n)) => {
                let span = self.peek_span();
                self.advance();
                Ok(Pattern::Literal(Expr::IntLiteral(n, span)))
            }
            Some(Token::StringLiteral(s)) => {
                let span = self.peek_span();
                self.advance();
                Ok(Pattern::Literal(Expr::StringLiteral(s, span)))
            }
            Some(Token::True) => {
                let span = self.peek_span();
                self.advance();
                Ok(Pattern::Literal(Expr::BoolLiteral(true, span)))
            }
            Some(Token::False) => {
                let span = self.peek_span();
                self.advance();
                Ok(Pattern::Literal(Expr::BoolLiteral(false, span)))
            }
            _ => {
                let span = self.peek_span();
                Err(ParseError {
                    message: "expected pattern".into(),
                    span,
                })
            }
        }
    }

    fn parse_parallel_expr(&mut self) -> ParseResult<Expr> {
        let start = self.peek_span();
        self.expect(&Token::Parallel)?;
        self.expect(&Token::LBrace)?;

        let mut exprs = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            exprs.push(self.parse_expr()?);
            if !self.eat(&Token::Comma) && !self.eat(&Token::Semicolon) {
                // Allow either comma or semicolon as separator
                if !self.check(&Token::RBrace) {
                    break;
                }
            }
        }

        let end = self.peek_span();
        self.expect(&Token::RBrace)?;

        Ok(Expr::Parallel {
            exprs,
            span: start.merge(&end),
        })
    }

    fn parse_list_expr(&mut self) -> ParseResult<Expr> {
        let start = self.peek_span();
        self.expect(&Token::LBracket)?;

        let mut elements = Vec::new();
        while !self.check(&Token::RBracket) && !self.at_end() {
            elements.push(self.parse_expr()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }

        let end = self.peek_span();
        self.expect(&Token::RBracket)?;

        Ok(Expr::List {
            elements,
            span: start.merge(&end),
        })
    }

    /// Parse a lambda expression: `|param, param| -> RetTy body_expr`
    ///
    /// Supports three syntactic forms:
    /// - `|x: i64| x * 2`                     (single expr body)
    /// - `|x: i64, y: i64| -> i64 { x + y }`  (block body with return type)
    /// - `|| 42`                               (no-argument lambda)
    fn parse_lambda_expr(&mut self) -> ParseResult<Expr> {
        let start = self.peek_span();
        // Consume opening `|`
        self.expect(&Token::Pipe)?;

        // Parse parameter list terminated by another `|`
        let mut params: Vec<Param> = Vec::new();
        while !self.check(&Token::Pipe) && !self.at_end() {
            let pspan = self.peek_span();
            let (pname, _) = self.expect_ident()?;
            let pty = if self.eat(&Token::Colon) {
                self.parse_type()?
            } else {
                // Inferred type
                TypeExpr::Inferred(pspan.clone())
            };
            let pend = self.peek_span();
            params.push(Param {
                name: pname,
                ty: pty,
                default: None,
                span: pspan.merge(&pend),
            });
            if !self.eat(&Token::Comma) {
                break;
            }
        }

        // Consume closing `|`
        self.expect(&Token::Pipe)?;

        // Optional return type annotation: `-> Type`
        let _ret_ty = if self.eat(&Token::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Body: either a block `{ ... }` or a single expression
        let body: Expr = if self.check(&Token::LBrace) {
            let block = self.parse_block()?;
            Expr::Block(block)
        } else {
            self.parse_expr()?
        };

        let end = body.span().clone();
        Ok(Expr::Lambda {
            params,
            body: Box::new(body),
            span: start.merge(&end),
        })
    }
}

// ─── Public API ─────────────────────────────────────────────────────────
/// Parse a source string into an AST Program.
pub fn parse(source: &str) -> (Program, Vec<ParseError>) {
    let (tokens, lex_errors) = crate::lexer::lex(source);

    let parser = Parser::new(tokens);
    let (program, mut parse_errors) = parser.parse();

    // Prepend lex errors as parse errors
    let lex_as_parse: Vec<ParseError> = lex_errors
        .into_iter()
        .map(|e| ParseError {
            message: format!("unexpected character(s): '{}'", e.text),
            span: Span::new(e.span.start, e.span.end),
        })
        .collect();

    let mut all_errors = lex_as_parse;
    all_errors.append(&mut parse_errors);

    (program, all_errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let (prog, errors) = parse("");
        assert!(errors.is_empty());
        assert!(prog.items.is_empty());
    }

    #[test]
    fn test_parse_simple_function() {
        let (prog, errors) = parse("fn main() -> i64 { return 42; }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            TopLevel::Function(f) => {
                assert_eq!(f.name, "main");
                assert!(f.params.is_empty());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_function_with_params() {
        let (prog, errors) = parse("fn add(a: i64, b: i64) -> i64 { a + b }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        match &prog.items[0] {
            TopLevel::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.params[0].name, "a");
                assert_eq!(f.params[1].name, "b");
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_struct() {
        let (prog, errors) = parse("struct Point { x: f64, y: f64 }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        match &prog.items[0] {
            TopLevel::Struct(s) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 2);
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn test_parse_enum() {
        let (prog, errors) = parse("enum Option { Some(i64), None }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        match &prog.items[0] {
            TopLevel::Enum(e) => {
                assert_eq!(e.name, "Option");
                assert_eq!(e.variants.len(), 2);
                assert_eq!(e.variants[0].name, "Some");
                assert_eq!(e.variants[1].name, "None");
            }
            _ => panic!("expected enum"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let (_prog, errors) = parse("fn test() { if x > 0 { 1 } else { 2 } }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_pipe() {
        let (_prog, errors) = parse("fn test() { data |> transform |> output; }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_annotation() {
        let (prog, errors) = parse("@evolvable fn evolve_me() { }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        match &prog.items[0] {
            TopLevel::Annotated { annotations, .. } => {
                assert_eq!(annotations[0].name, "evolvable");
            }
            _ => panic!("expected annotated item"),
        }
    }

    #[test]
    fn test_parse_import() {
        let (prog, errors) = parse("import std::io;");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        match &prog.items[0] {
            TopLevel::Import(i) => {
                assert_eq!(i.path, vec!["std", "io"]);
            }
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_error_recovery() {
        let (prog, errors) = parse("fn good() { 1 } $$$$ fn also_good() { 2 }");
        // Should have errors for $$$$ but still parse both functions
        assert!(!errors.is_empty());
        // At least one function should be parsed
        assert!(!prog.items.is_empty());
    }

    #[test]
    fn test_parse_match() {
        let src = r#"
fn test() {
    match x {
        0 => "zero",
        1 => "one",
        _ => "other",
    }
}
"#;
        let (_prog, errors) = parse(src);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_let_mut() {
        let (_prog, errors) = parse("fn test() { let mut x: i64 = 0; x = 1; }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_parallel() {
        let (_prog, errors) = parse("fn test() { parallel { task_a(), task_b() } }");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }
}
