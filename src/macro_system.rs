//! Vitalis Macro System (v26)
//!
//! A hygienic macro expansion engine supporting declarative pattern-matching macros,
//! derive macros, and procedural transforms. Macros are expanded before type checking,
//! operating on token trees and producing AST fragments.
//!
//! # Architecture
//!
//! - **MacroDef**: A named macro with rules (pattern → template pairs)
//! - **MacroRule**: A single `(pattern) => { template }` arm
//! - **HygieneContext**: Scope‐aware renaming to prevent accidental capture
//! - **MacroExpander**: Drives expansion by matching token trees against rules
//! - **DeriveRegistry**: Built-in derive macros (Debug, Clone, PartialEq, etc.)
//!
//! # Examples
//!
//! ```text
//! // Vitalis source (future syntax):
//! macro vec!($($elem:expr),*) => { let v = []; $(v = array_push(v, $elem);)* v }
//! let xs = vec![1, 2, 3];
//!
//! @derive(Debug, Clone)
//! struct Point { x: i64, y: i64 }
//! ```

use std::collections::HashMap;
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Token Trees
// ═══════════════════════════════════════════════════════════════════════

/// A token tree — the basic unit macros operate on.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenTree {
    /// An identifier token.
    Ident(String),
    /// An integer literal.
    IntLit(i64),
    /// A float literal.
    FloatLit(f64),
    /// A string literal.
    StringLit(String),
    /// A boolean literal.
    BoolLit(bool),
    /// A punctuation/operator token.
    Punct(String),
    /// A delimited group: `(...)`, `[...]`, or `{...}`.
    Group {
        delimiter: Delimiter,
        tokens: Vec<TokenTree>,
    },
}

impl fmt::Display for TokenTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenTree::Ident(s) => write!(f, "{}", s),
            TokenTree::IntLit(n) => write!(f, "{}", n),
            TokenTree::FloatLit(v) => write!(f, "{}", v),
            TokenTree::StringLit(s) => write!(f, "\"{}\"", s),
            TokenTree::BoolLit(b) => write!(f, "{}", b),
            TokenTree::Punct(p) => write!(f, "{}", p),
            TokenTree::Group { delimiter, tokens } => {
                let (open, close) = delimiter.chars();
                write!(f, "{}", open)?;
                for (i, t) in tokens.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, "{}", close)
            }
        }
    }
}

/// Delimiter kinds for grouped token trees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Delimiter {
    Paren,
    Bracket,
    Brace,
}

impl Delimiter {
    pub fn chars(self) -> (char, char) {
        match self {
            Delimiter::Paren => ('(', ')'),
            Delimiter::Bracket => ('[', ']'),
            Delimiter::Brace => ('{', '}'),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Fragment Specs & Patterns
// ═══════════════════════════════════════════════════════════════════════

/// Fragment specifier — what a metavariable can capture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FragSpec {
    /// An expression fragment.
    Expr,
    /// A type fragment.
    Type,
    /// An identifier fragment.
    Ident,
    /// A block fragment `{ ... }`.
    Block,
    /// A statement fragment.
    Stmt,
    /// A pattern fragment.
    Pat,
    /// A token tree fragment (anything).
    Tt,
    /// A literal (int, float, string, bool).
    Literal,
}

impl fmt::Display for FragSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FragSpec::Expr => write!(f, "expr"),
            FragSpec::Type => write!(f, "ty"),
            FragSpec::Ident => write!(f, "ident"),
            FragSpec::Block => write!(f, "block"),
            FragSpec::Stmt => write!(f, "stmt"),
            FragSpec::Pat => write!(f, "pat"),
            FragSpec::Tt => write!(f, "tt"),
            FragSpec::Literal => write!(f, "literal"),
        }
    }
}

impl FragSpec {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "expr" => Some(FragSpec::Expr),
            "ty" | "type" => Some(FragSpec::Type),
            "ident" => Some(FragSpec::Ident),
            "block" => Some(FragSpec::Block),
            "stmt" => Some(FragSpec::Stmt),
            "pat" => Some(FragSpec::Pat),
            "tt" => Some(FragSpec::Tt),
            "literal" | "lit" => Some(FragSpec::Literal),
            _ => None,
        }
    }
}

/// A macro pattern element.
#[derive(Debug, Clone, PartialEq)]
pub enum MacroPatternElem {
    /// A literal token to match exactly.
    Token(TokenTree),
    /// A metavariable capture: `$name:frag`.
    Capture {
        name: String,
        frag: FragSpec,
    },
    /// A repetition: `$(...)*` or `$(...)+` or `$(...)?`.
    Repetition {
        elements: Vec<MacroPatternElem>,
        separator: Option<String>,
        kind: RepKind,
    },
}

/// Repetition quantifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepKind {
    /// Zero or more: `*`
    ZeroOrMore,
    /// One or more: `+`
    OneOrMore,
    /// Zero or one: `?`
    Optional,
}

impl fmt::Display for RepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepKind::ZeroOrMore => write!(f, "*"),
            RepKind::OneOrMore => write!(f, "+"),
            RepKind::Optional => write!(f, "?"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Template (Transcription)
// ═══════════════════════════════════════════════════════════════════════

/// A macro template element — used to produce output.
#[derive(Debug, Clone, PartialEq)]
pub enum MacroTemplateElem {
    /// Emit a literal token.
    Token(TokenTree),
    /// Substitute a captured metavariable.
    Substitution(String),
    /// Repeated substitution: `$(...)*`.
    Repetition {
        elements: Vec<MacroTemplateElem>,
        separator: Option<String>,
        kind: RepKind,
    },
}

// ═══════════════════════════════════════════════════════════════════════
//  Macro Definitions & Rules
// ═══════════════════════════════════════════════════════════════════════

/// A single macro rule: pattern → template.
#[derive(Debug, Clone)]
pub struct MacroRule {
    pub pattern: Vec<MacroPatternElem>,
    pub template: Vec<MacroTemplateElem>,
}

/// The kind of macro.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MacroKind {
    /// Declarative pattern-matching macro (like Rust's `macro_rules!`).
    Declarative,
    /// Derive macro that generates impls for a struct/enum.
    Derive,
    /// Attribute macro that transforms the annotated item.
    Attribute,
}

impl fmt::Display for MacroKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacroKind::Declarative => write!(f, "declarative"),
            MacroKind::Derive => write!(f, "derive"),
            MacroKind::Attribute => write!(f, "attribute"),
        }
    }
}

/// A macro definition.
#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name: String,
    pub kind: MacroKind,
    pub rules: Vec<MacroRule>,
    pub is_exported: bool,
}

// ═══════════════════════════════════════════════════════════════════════
//  Hygiene Context
// ═══════════════════════════════════════════════════════════════════════

/// Unique stamp for a macro expansion, used for hygienic renaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxContext(pub u32);

/// Hygiene context tracks scope stamps so that macro-introduced bindings
/// do not collide with user code.
#[derive(Debug, Clone)]
pub struct HygieneContext {
    next_context: u32,
    /// Maps (original_name, context) → renamed identifier.
    renames: HashMap<(String, SyntaxContext), String>,
}

impl HygieneContext {
    pub fn new() -> Self {
        Self {
            next_context: 1,
            renames: HashMap::new(),
        }
    }

    /// Allocate a fresh syntax context for a new expansion.
    pub fn fresh_context(&mut self) -> SyntaxContext {
        let ctx = SyntaxContext(self.next_context);
        self.next_context += 1;
        ctx
    }

    /// Rename an identifier introduced by a macro expansion.
    pub fn rename(&mut self, name: &str, ctx: SyntaxContext) -> String {
        let key = (name.to_string(), ctx);
        if let Some(existing) = self.renames.get(&key) {
            return existing.clone();
        }
        let renamed = format!("__{}_ctx{}", name, ctx.0);
        self.renames.insert(key, renamed.clone());
        renamed
    }

    /// Look up a renamed identifier, returning the original if not found.
    pub fn resolve(&self, name: &str, ctx: SyntaxContext) -> String {
        let key = (name.to_string(), ctx);
        self.renames
            .get(&key)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Return total number of renames tracked.
    pub fn rename_count(&self) -> usize {
        self.renames.len()
    }
}

impl Default for HygieneContext {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Capture Bindings
// ═══════════════════════════════════════════════════════════════════════

/// Captured values from a successful pattern match.
#[derive(Debug, Clone)]
pub enum CapturedValue {
    /// A single captured token tree.
    Single(TokenTree),
    /// Multiple captured fragments (from repetitions).
    Repeated(Vec<Vec<CapturedValue>>),
}

/// A set of bindings from a macro pattern match.
pub type CaptureBindings = HashMap<String, CapturedValue>;

// ═══════════════════════════════════════════════════════════════════════
//  Pattern Matching Engine
// ═══════════════════════════════════════════════════════════════════════

/// Match a sequence of pattern elements against a token tree stream.
/// Returns `Some(bindings)` if the match is successful.
pub fn match_pattern(
    pattern: &[MacroPatternElem],
    tokens: &[TokenTree],
) -> Option<CaptureBindings> {
    let mut bindings = CaptureBindings::new();
    let consumed = match_pattern_inner(pattern, tokens, &mut bindings)?;
    if consumed == tokens.len() {
        Some(bindings)
    } else {
        None
    }
}

fn match_pattern_inner(
    pattern: &[MacroPatternElem],
    tokens: &[TokenTree],
    bindings: &mut CaptureBindings,
) -> Option<usize> {
    let mut tok_idx = 0;
    for elem in pattern {
        match elem {
            MacroPatternElem::Token(expected) => {
                if tok_idx >= tokens.len() {
                    return None;
                }
                if !token_eq(expected, &tokens[tok_idx]) {
                    return None;
                }
                tok_idx += 1;
            }
            MacroPatternElem::Capture { name, frag } => {
                if tok_idx >= tokens.len() {
                    return None;
                }
                if matches_frag(*frag, &tokens[tok_idx]) {
                    bindings.insert(
                        name.clone(),
                        CapturedValue::Single(tokens[tok_idx].clone()),
                    );
                    tok_idx += 1;
                } else {
                    return None;
                }
            }
            MacroPatternElem::Repetition {
                elements,
                separator,
                kind,
            } => {
                let mut all_iterations: Vec<Vec<CapturedValue>> = Vec::new();
                let mut iter_count = 0usize;
                loop {
                    // Try separator before second+ iteration
                    let sep_offset = if iter_count > 0 {
                        if let Some(sep) = separator {
                            if tok_idx < tokens.len()
                                && token_eq_str(sep, &tokens[tok_idx])
                            {
                                1
                            } else {
                                break;
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    let mut iter_bindings = CaptureBindings::new();
                    if let Some(consumed) = match_pattern_inner(
                        elements,
                        &tokens[tok_idx + sep_offset..],
                        &mut iter_bindings,
                    ) {
                        tok_idx += sep_offset + consumed;
                        // Collect captured values for this iteration
                        let iter_vals: Vec<CapturedValue> = elements
                            .iter()
                            .filter_map(|e| {
                                if let MacroPatternElem::Capture { name, .. } = e {
                                    iter_bindings.get(name).cloned()
                                } else {
                                    None
                                }
                            })
                            .collect();
                        all_iterations.push(iter_vals);
                        iter_count += 1;
                        if *kind == RepKind::Optional && iter_count >= 1 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                // Check minimum count
                match kind {
                    RepKind::OneOrMore if iter_count == 0 => return None,
                    _ => {}
                }
                // Bind repeated captures
                for elem_pat in elements {
                    if let MacroPatternElem::Capture { name, .. } = elem_pat {
                        bindings.insert(
                            name.clone(),
                            CapturedValue::Repeated(all_iterations.clone()),
                        );
                    }
                }
            }
        }
    }
    Some(tok_idx)
}

fn token_eq(a: &TokenTree, b: &TokenTree) -> bool {
    match (a, b) {
        (TokenTree::Ident(x), TokenTree::Ident(y)) => x == y,
        (TokenTree::IntLit(x), TokenTree::IntLit(y)) => x == y,
        (TokenTree::StringLit(x), TokenTree::StringLit(y)) => x == y,
        (TokenTree::BoolLit(x), TokenTree::BoolLit(y)) => x == y,
        (TokenTree::Punct(x), TokenTree::Punct(y)) => x == y,
        _ => false,
    }
}

fn token_eq_str(s: &str, tok: &TokenTree) -> bool {
    match tok {
        TokenTree::Punct(p) => p == s,
        TokenTree::Ident(i) => i == s,
        _ => false,
    }
}

fn matches_frag(frag: FragSpec, tok: &TokenTree) -> bool {
    match frag {
        FragSpec::Expr | FragSpec::Tt => true,
        FragSpec::Ident => matches!(tok, TokenTree::Ident(_)),
        FragSpec::Literal => matches!(
            tok,
            TokenTree::IntLit(_)
                | TokenTree::FloatLit(_)
                | TokenTree::StringLit(_)
                | TokenTree::BoolLit(_)
        ),
        FragSpec::Block => matches!(
            tok,
            TokenTree::Group {
                delimiter: Delimiter::Brace,
                ..
            }
        ),
        FragSpec::Type => matches!(tok, TokenTree::Ident(_)),
        FragSpec::Stmt => true,
        FragSpec::Pat => matches!(tok, TokenTree::Ident(_) | TokenTree::IntLit(_)),
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Template Expansion (Transcription)
// ═══════════════════════════════════════════════════════════════════════

/// Expand a template using captured bindings, producing new tokens.
pub fn expand_template(
    template: &[MacroTemplateElem],
    bindings: &CaptureBindings,
    hygiene: &mut HygieneContext,
    ctx: SyntaxContext,
) -> Result<Vec<TokenTree>, MacroError> {
    let mut output = Vec::new();
    for elem in template {
        match elem {
            MacroTemplateElem::Token(tok) => {
                // Apply hygiene to identifiers introduced by the macro
                let tok = match tok {
                    TokenTree::Ident(name) if !bindings.contains_key(name) => {
                        TokenTree::Ident(hygiene.rename(name, ctx))
                    }
                    other => other.clone(),
                };
                output.push(tok);
            }
            MacroTemplateElem::Substitution(name) => {
                match bindings.get(name) {
                    Some(CapturedValue::Single(tok)) => output.push(tok.clone()),
                    Some(CapturedValue::Repeated(iters)) => {
                        // Flatten all iterations for a simple substitution
                        for iter_vals in iters {
                            for val in iter_vals {
                                if let CapturedValue::Single(tok) = val {
                                    output.push(tok.clone());
                                }
                            }
                        }
                    }
                    None => {
                        return Err(MacroError::UnboundVariable(name.clone()));
                    }
                }
            }
            MacroTemplateElem::Repetition {
                elements,
                separator,
                ..
            } => {
                // Find the repetition-bound variable to determine iteration count
                let rep_var = find_rep_var(elements);
                if let Some(var_name) = rep_var {
                    if let Some(CapturedValue::Repeated(iters)) = bindings.get(&var_name) {
                        for (i, iter_vals) in iters.iter().enumerate() {
                            if i > 0 {
                                if let Some(sep) = separator {
                                    output.push(TokenTree::Punct(sep.clone()));
                                }
                            }
                            // Build per-iteration bindings
                            let mut iter_bindings = bindings.clone();
                            if let Some(val) = iter_vals.first() {
                                iter_bindings
                                    .insert(var_name.clone(), val.clone());
                            }
                            let expanded =
                                expand_template(elements, &iter_bindings, hygiene, ctx)?;
                            output.extend(expanded);
                        }
                    }
                }
            }
        }
    }
    Ok(output)
}

fn find_rep_var(elements: &[MacroTemplateElem]) -> Option<String> {
    for elem in elements {
        if let MacroTemplateElem::Substitution(name) = elem {
            return Some(name.clone());
        }
        if let MacroTemplateElem::Repetition { elements: inner, .. } = elem {
            if let Some(v) = find_rep_var(inner) {
                return Some(v);
            }
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════
//  Macro Expander
// ═══════════════════════════════════════════════════════════════════════

/// Errors that can occur during macro expansion.
#[derive(Debug, Clone, PartialEq)]
pub enum MacroError {
    /// No macro rule matched the input.
    NoMatchingRule { macro_name: String },
    /// An unbound metavariable in the template.
    UnboundVariable(String),
    /// Recursion depth exceeded.
    RecursionLimit { depth: usize },
    /// A derive macro was invoked on a non-struct/enum.
    InvalidDeriveTarget { derive_name: String },
    /// Unknown macro name.
    UnknownMacro(String),
    /// Unknown derive macro name.
    UnknownDerive(String),
    /// General expansion error.
    ExpansionError(String),
}

impl fmt::Display for MacroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacroError::NoMatchingRule { macro_name } => {
                write!(f, "no matching rule for macro `{}`", macro_name)
            }
            MacroError::UnboundVariable(v) => {
                write!(f, "unbound metavariable `${}`", v)
            }
            MacroError::RecursionLimit { depth } => {
                write!(f, "macro recursion limit exceeded (depth {})", depth)
            }
            MacroError::InvalidDeriveTarget { derive_name } => {
                write!(f, "derive `{}` requires a struct or enum", derive_name)
            }
            MacroError::UnknownMacro(name) => {
                write!(f, "unknown macro `{}`", name)
            }
            MacroError::UnknownDerive(name) => {
                write!(f, "unknown derive macro `{}`", name)
            }
            MacroError::ExpansionError(msg) => {
                write!(f, "macro expansion error: {}", msg)
            }
        }
    }
}

/// The macro expander drives expansion for all registered macros.
pub struct MacroExpander {
    macros: HashMap<String, MacroDef>,
    derives: DeriveRegistry,
    hygiene: HygieneContext,
    max_recursion: usize,
    expansion_count: usize,
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            derives: DeriveRegistry::with_builtins(),
            hygiene: HygieneContext::new(),
            max_recursion: 128,
            expansion_count: 0,
        }
    }

    /// Set the maximum recursion depth for macro expansion.
    pub fn set_max_recursion(&mut self, depth: usize) {
        self.max_recursion = depth;
    }

    /// Register a declarative macro.
    pub fn register_macro(&mut self, def: MacroDef) {
        self.macros.insert(def.name.clone(), def);
    }

    /// Check if a macro is registered.
    pub fn has_macro(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// Return count of registered macros.
    pub fn macro_count(&self) -> usize {
        self.macros.len()
    }

    /// Return total expansions performed.
    pub fn expansion_count(&self) -> usize {
        self.expansion_count
    }

    /// Get a reference to the hygiene context.
    pub fn hygiene(&self) -> &HygieneContext {
        &self.hygiene
    }

    /// Expand a macro invocation.
    pub fn expand(
        &mut self,
        name: &str,
        tokens: &[TokenTree],
    ) -> Result<Vec<TokenTree>, MacroError> {
        self.expand_depth(name, tokens, 0)
    }

    fn expand_depth(
        &mut self,
        name: &str,
        tokens: &[TokenTree],
        depth: usize,
    ) -> Result<Vec<TokenTree>, MacroError> {
        if depth >= self.max_recursion {
            return Err(MacroError::RecursionLimit { depth });
        }
        let def = self
            .macros
            .get(name)
            .ok_or_else(|| MacroError::UnknownMacro(name.to_string()))?
            .clone();

        for rule in &def.rules {
            if let Some(bindings) = match_pattern(&rule.pattern, tokens) {
                let ctx = self.hygiene.fresh_context();
                let result =
                    expand_template(&rule.template, &bindings, &mut self.hygiene, ctx)?;
                self.expansion_count += 1;
                return Ok(result);
            }
        }
        Err(MacroError::NoMatchingRule {
            macro_name: name.to_string(),
        })
    }

    /// Expand a derive macro for a struct.
    pub fn expand_derive(
        &mut self,
        derive_name: &str,
        struct_name: &str,
        field_names: &[&str],
    ) -> Result<Vec<TokenTree>, MacroError> {
        self.derives
            .expand(derive_name, struct_name, field_names)
    }

    /// List all registered macro names.
    pub fn list_macros(&self) -> Vec<&str> {
        self.macros.keys().map(|s| s.as_str()).collect()
    }

    /// List all available derive macros.
    pub fn list_derives(&self) -> Vec<&str> {
        self.derives.list()
    }
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Derive Registry
// ═══════════════════════════════════════════════════════════════════════

/// A derive macro handler function.
type DeriveFn = fn(&str, &[&str]) -> Result<Vec<TokenTree>, MacroError>;

/// Registry of built-in derive macros.
pub struct DeriveRegistry {
    derives: HashMap<String, DeriveFn>,
}

impl DeriveRegistry {
    pub fn new() -> Self {
        Self {
            derives: HashMap::new(),
        }
    }

    /// Create a registry with the built-in derives pre-registered.
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.register("Debug", derive_debug);
        reg.register("Clone", derive_clone);
        reg.register("PartialEq", derive_partial_eq);
        reg.register("Default", derive_default);
        reg.register("Display", derive_display);
        reg.register("Hash", derive_hash);
        reg
    }

    /// Register a derive macro handler.
    pub fn register(&mut self, name: &str, handler: DeriveFn) {
        self.derives.insert(name.to_string(), handler);
    }

    /// Expand a derive macro.
    pub fn expand(
        &self,
        name: &str,
        struct_name: &str,
        fields: &[&str],
    ) -> Result<Vec<TokenTree>, MacroError> {
        let handler = self
            .derives
            .get(name)
            .ok_or_else(|| MacroError::UnknownDerive(name.to_string()))?;
        handler(struct_name, fields)
    }

    /// Check if a derive is registered.
    pub fn has(&self, name: &str) -> bool {
        self.derives.contains_key(name)
    }

    /// List all derive names.
    pub fn list(&self) -> Vec<&str> {
        self.derives.keys().map(|s| s.as_str()).collect()
    }

    /// Number of registered derives.
    pub fn count(&self) -> usize {
        self.derives.len()
    }
}

impl Default for DeriveRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Built-in Derive Implementations
// ═══════════════════════════════════════════════════════════════════════

/// Generate a Debug impl: produces tokens for `fn debug(self) -> str`.
fn derive_debug(struct_name: &str, fields: &[&str]) -> Result<Vec<TokenTree>, MacroError> {
    let mut tokens = Vec::new();
    // impl StructName { fn debug(self) -> str { ... } }
    tokens.push(TokenTree::Ident("impl".to_string()));
    tokens.push(TokenTree::Ident(struct_name.to_string()));
    let mut body = Vec::new();
    body.push(TokenTree::Ident("fn".to_string()));
    body.push(TokenTree::Ident("debug".to_string()));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Paren,
        tokens: vec![TokenTree::Ident("self".to_string())],
    });
    body.push(TokenTree::Punct("->".to_string()));
    body.push(TokenTree::Ident("str".to_string()));
    // Build format string like "StructName { field1: {}, field2: {} }"
    let field_fmt: Vec<String> = fields.iter().map(|f| format!("{}: {{}}", f)).collect();
    let fmt_str = format!("{} {{ {} }}", struct_name, field_fmt.join(", "));
    let mut fn_body = Vec::new();
    fn_body.push(TokenTree::StringLit(fmt_str));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: fn_body,
    });
    tokens.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: body,
    });
    Ok(tokens)
}

/// Generate a Clone impl.
fn derive_clone(struct_name: &str, fields: &[&str]) -> Result<Vec<TokenTree>, MacroError> {
    let mut tokens = Vec::new();
    tokens.push(TokenTree::Ident("impl".to_string()));
    tokens.push(TokenTree::Ident(struct_name.to_string()));
    let mut body = Vec::new();
    body.push(TokenTree::Ident("fn".to_string()));
    body.push(TokenTree::Ident("clone".to_string()));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Paren,
        tokens: vec![TokenTree::Ident("self".to_string())],
    });
    body.push(TokenTree::Punct("->".to_string()));
    body.push(TokenTree::Ident(struct_name.to_string()));
    let mut fn_body = Vec::new();
    fn_body.push(TokenTree::Ident(struct_name.to_string()));
    let mut field_tokens = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            field_tokens.push(TokenTree::Punct(",".to_string()));
        }
        field_tokens.push(TokenTree::Ident(field.to_string()));
        field_tokens.push(TokenTree::Punct(":".to_string()));
        field_tokens.push(TokenTree::Ident("self".to_string()));
        field_tokens.push(TokenTree::Punct(".".to_string()));
        field_tokens.push(TokenTree::Ident(field.to_string()));
    }
    fn_body.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: field_tokens,
    });
    body.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: fn_body,
    });
    tokens.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: body,
    });
    Ok(tokens)
}

/// Generate a PartialEq impl.
fn derive_partial_eq(struct_name: &str, fields: &[&str]) -> Result<Vec<TokenTree>, MacroError> {
    let mut tokens = Vec::new();
    tokens.push(TokenTree::Ident("impl".to_string()));
    tokens.push(TokenTree::Ident(struct_name.to_string()));
    let mut body = Vec::new();
    body.push(TokenTree::Ident("fn".to_string()));
    body.push(TokenTree::Ident("eq".to_string()));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Paren,
        tokens: vec![
            TokenTree::Ident("self".to_string()),
            TokenTree::Punct(",".to_string()),
            TokenTree::Ident("other".to_string()),
        ],
    });
    body.push(TokenTree::Punct("->".to_string()));
    body.push(TokenTree::Ident("bool".to_string()));
    let mut fn_body = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            fn_body.push(TokenTree::Punct("&&".to_string()));
        }
        fn_body.push(TokenTree::Ident("self".to_string()));
        fn_body.push(TokenTree::Punct(".".to_string()));
        fn_body.push(TokenTree::Ident(field.to_string()));
        fn_body.push(TokenTree::Punct("==".to_string()));
        fn_body.push(TokenTree::Ident("other".to_string()));
        fn_body.push(TokenTree::Punct(".".to_string()));
        fn_body.push(TokenTree::Ident(field.to_string()));
    }
    body.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: fn_body,
    });
    tokens.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: body,
    });
    Ok(tokens)
}

/// Generate a Default impl (all fields get default values).
fn derive_default(struct_name: &str, fields: &[&str]) -> Result<Vec<TokenTree>, MacroError> {
    let mut tokens = Vec::new();
    tokens.push(TokenTree::Ident("impl".to_string()));
    tokens.push(TokenTree::Ident(struct_name.to_string()));
    let mut body = Vec::new();
    body.push(TokenTree::Ident("fn".to_string()));
    body.push(TokenTree::Ident("default".to_string()));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Paren,
        tokens: vec![],
    });
    body.push(TokenTree::Punct("->".to_string()));
    body.push(TokenTree::Ident(struct_name.to_string()));
    let mut fn_body = Vec::new();
    fn_body.push(TokenTree::Ident(struct_name.to_string()));
    let mut field_tokens = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            field_tokens.push(TokenTree::Punct(",".to_string()));
        }
        field_tokens.push(TokenTree::Ident(field.to_string()));
        field_tokens.push(TokenTree::Punct(":".to_string()));
        field_tokens.push(TokenTree::IntLit(0));
    }
    fn_body.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: field_tokens,
    });
    body.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: fn_body,
    });
    tokens.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: body,
    });
    Ok(tokens)
}

/// Generate a Display impl.
fn derive_display(struct_name: &str, fields: &[&str]) -> Result<Vec<TokenTree>, MacroError> {
    let mut tokens = Vec::new();
    tokens.push(TokenTree::Ident("impl".to_string()));
    tokens.push(TokenTree::Ident(struct_name.to_string()));
    let mut body = Vec::new();
    body.push(TokenTree::Ident("fn".to_string()));
    body.push(TokenTree::Ident("to_string".to_string()));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Paren,
        tokens: vec![TokenTree::Ident("self".to_string())],
    });
    body.push(TokenTree::Punct("->".to_string()));
    body.push(TokenTree::Ident("str".to_string()));
    let field_parts: Vec<String> = fields.iter().map(|f| format!("{}={{}}", f)).collect();
    let fmt_str = format!("{}({})", struct_name, field_parts.join(", "));
    let mut fn_body = Vec::new();
    fn_body.push(TokenTree::StringLit(fmt_str));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: fn_body,
    });
    tokens.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: body,
    });
    Ok(tokens)
}

/// Generate a Hash impl.
fn derive_hash(struct_name: &str, fields: &[&str]) -> Result<Vec<TokenTree>, MacroError> {
    let mut tokens = Vec::new();
    tokens.push(TokenTree::Ident("impl".to_string()));
    tokens.push(TokenTree::Ident(struct_name.to_string()));
    let mut body = Vec::new();
    body.push(TokenTree::Ident("fn".to_string()));
    body.push(TokenTree::Ident("hash".to_string()));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Paren,
        tokens: vec![TokenTree::Ident("self".to_string())],
    });
    body.push(TokenTree::Punct("->".to_string()));
    body.push(TokenTree::Ident("i64".to_string()));
    let mut fn_body = Vec::new();
    fn_body.push(TokenTree::Ident("let".to_string()));
    fn_body.push(TokenTree::Ident("h".to_string()));
    fn_body.push(TokenTree::Punct("=".to_string()));
    fn_body.push(TokenTree::IntLit(17));
    for field in fields {
        fn_body.push(TokenTree::Punct(";".to_string()));
        fn_body.push(TokenTree::Ident("h".to_string()));
        fn_body.push(TokenTree::Punct("=".to_string()));
        fn_body.push(TokenTree::Ident("h".to_string()));
        fn_body.push(TokenTree::Punct("*".to_string()));
        fn_body.push(TokenTree::IntLit(31));
        fn_body.push(TokenTree::Punct("+".to_string()));
        fn_body.push(TokenTree::Ident("self".to_string()));
        fn_body.push(TokenTree::Punct(".".to_string()));
        fn_body.push(TokenTree::Ident(field.to_string()));
    }
    fn_body.push(TokenTree::Punct(";".to_string()));
    fn_body.push(TokenTree::Ident("h".to_string()));
    body.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: fn_body,
    });
    tokens.push(TokenTree::Group {
        delimiter: Delimiter::Brace,
        tokens: body,
    });
    Ok(tokens)
}

// ═══════════════════════════════════════════════════════════════════════
//  Macro Utilities
// ═══════════════════════════════════════════════════════════════════════

/// Parse a fragment specifier from a string like "expr", "ident", etc.
pub fn parse_frag_spec(s: &str) -> Option<FragSpec> {
    FragSpec::from_str(s)
}

/// Count the number of token trees (recursively).
pub fn count_tokens(tokens: &[TokenTree]) -> usize {
    let mut count = 0;
    for tok in tokens {
        count += 1;
        if let TokenTree::Group { tokens: inner, .. } = tok {
            count += count_tokens(inner);
        }
    }
    count
}

/// Flatten a token tree into a flat list (removing group delimiters).
pub fn flatten_tokens(tokens: &[TokenTree]) -> Vec<TokenTree> {
    let mut result = Vec::new();
    for tok in tokens {
        match tok {
            TokenTree::Group { tokens: inner, .. } => {
                result.extend(flatten_tokens(inner));
            }
            other => result.push(other.clone()),
        }
    }
    result
}

/// Check if a token tree list contains a specific identifier.
pub fn contains_ident(tokens: &[TokenTree], name: &str) -> bool {
    for tok in tokens {
        match tok {
            TokenTree::Ident(n) if n == name => return true,
            TokenTree::Group { tokens: inner, .. } => {
                if contains_ident(inner, name) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── TokenTree basics ────────────────────────────────────────────

    #[test]
    fn test_token_tree_display_ident() {
        let tok = TokenTree::Ident("foo".to_string());
        assert_eq!(tok.to_string(), "foo");
    }

    #[test]
    fn test_token_tree_display_int() {
        let tok = TokenTree::IntLit(42);
        assert_eq!(tok.to_string(), "42");
    }

    #[test]
    fn test_token_tree_display_string() {
        let tok = TokenTree::StringLit("hello".to_string());
        assert_eq!(tok.to_string(), "\"hello\"");
    }

    #[test]
    fn test_token_tree_display_group() {
        let tok = TokenTree::Group {
            delimiter: Delimiter::Paren,
            tokens: vec![TokenTree::IntLit(1), TokenTree::Punct("+".to_string()), TokenTree::IntLit(2)],
        };
        assert_eq!(tok.to_string(), "(1 + 2)");
    }

    #[test]
    fn test_delimiter_chars() {
        assert_eq!(Delimiter::Paren.chars(), ('(', ')'));
        assert_eq!(Delimiter::Bracket.chars(), ('[', ']'));
        assert_eq!(Delimiter::Brace.chars(), ('{', '}'));
    }

    // ── FragSpec ────────────────────────────────────────────────────

    #[test]
    fn test_frag_spec_from_str() {
        assert_eq!(FragSpec::from_str("expr"), Some(FragSpec::Expr));
        assert_eq!(FragSpec::from_str("ident"), Some(FragSpec::Ident));
        assert_eq!(FragSpec::from_str("tt"), Some(FragSpec::Tt));
        assert_eq!(FragSpec::from_str("ty"), Some(FragSpec::Type));
        assert_eq!(FragSpec::from_str("type"), Some(FragSpec::Type));
        assert_eq!(FragSpec::from_str("literal"), Some(FragSpec::Literal));
        assert_eq!(FragSpec::from_str("lit"), Some(FragSpec::Literal));
        assert_eq!(FragSpec::from_str("bogus"), None);
    }

    #[test]
    fn test_frag_spec_display() {
        assert_eq!(format!("{}", FragSpec::Expr), "expr");
        assert_eq!(format!("{}", FragSpec::Block), "block");
    }

    // ── RepKind ────────────────────────────────────────────────────

    #[test]
    fn test_rep_kind_display() {
        assert_eq!(format!("{}", RepKind::ZeroOrMore), "*");
        assert_eq!(format!("{}", RepKind::OneOrMore), "+");
        assert_eq!(format!("{}", RepKind::Optional), "?");
    }

    // ── MacroKind ──────────────────────────────────────────────────

    #[test]
    fn test_macro_kind_display() {
        assert_eq!(format!("{}", MacroKind::Declarative), "declarative");
        assert_eq!(format!("{}", MacroKind::Derive), "derive");
        assert_eq!(format!("{}", MacroKind::Attribute), "attribute");
    }

    // ── HygieneContext ─────────────────────────────────────────────

    #[test]
    fn test_hygiene_fresh_context() {
        let mut hygiene = HygieneContext::new();
        let c1 = hygiene.fresh_context();
        let c2 = hygiene.fresh_context();
        assert_eq!(c1, SyntaxContext(1));
        assert_eq!(c2, SyntaxContext(2));
    }

    #[test]
    fn test_hygiene_rename() {
        let mut hygiene = HygieneContext::new();
        let ctx = hygiene.fresh_context();
        let renamed = hygiene.rename("x", ctx);
        assert_eq!(renamed, "__x_ctx1");
        // Same name+ctx should return same rename
        let again = hygiene.rename("x", ctx);
        assert_eq!(again, "__x_ctx1");
    }

    #[test]
    fn test_hygiene_resolve_known() {
        let mut hygiene = HygieneContext::new();
        let ctx = hygiene.fresh_context();
        hygiene.rename("x", ctx);
        assert_eq!(hygiene.resolve("x", ctx), "__x_ctx1");
    }

    #[test]
    fn test_hygiene_resolve_unknown() {
        let hygiene = HygieneContext::new();
        assert_eq!(hygiene.resolve("y", SyntaxContext(99)), "y");
    }

    #[test]
    fn test_hygiene_rename_count() {
        let mut hygiene = HygieneContext::new();
        assert_eq!(hygiene.rename_count(), 0);
        let ctx = hygiene.fresh_context();
        hygiene.rename("a", ctx);
        hygiene.rename("b", ctx);
        assert_eq!(hygiene.rename_count(), 2);
    }

    #[test]
    fn test_hygiene_different_contexts() {
        let mut hygiene = HygieneContext::new();
        let c1 = hygiene.fresh_context();
        let c2 = hygiene.fresh_context();
        let r1 = hygiene.rename("x", c1);
        let r2 = hygiene.rename("x", c2);
        assert_ne!(r1, r2);
        assert_eq!(r1, "__x_ctx1");
        assert_eq!(r2, "__x_ctx2");
    }

    // ── Pattern Matching ───────────────────────────────────────────

    #[test]
    fn test_match_literal_token() {
        let pattern = vec![MacroPatternElem::Token(TokenTree::Ident("hello".to_string()))];
        let tokens = vec![TokenTree::Ident("hello".to_string())];
        assert!(match_pattern(&pattern, &tokens).is_some());
    }

    #[test]
    fn test_match_literal_token_fail() {
        let pattern = vec![MacroPatternElem::Token(TokenTree::Ident("hello".to_string()))];
        let tokens = vec![TokenTree::Ident("world".to_string())];
        assert!(match_pattern(&pattern, &tokens).is_none());
    }

    #[test]
    fn test_match_capture_expr() {
        let pattern = vec![MacroPatternElem::Capture {
            name: "x".to_string(),
            frag: FragSpec::Expr,
        }];
        let tokens = vec![TokenTree::IntLit(42)];
        let bindings = match_pattern(&pattern, &tokens).unwrap();
        assert!(matches!(bindings.get("x"), Some(CapturedValue::Single(TokenTree::IntLit(42)))));
    }

    #[test]
    fn test_match_capture_ident() {
        let pattern = vec![MacroPatternElem::Capture {
            name: "name".to_string(),
            frag: FragSpec::Ident,
        }];
        let tokens = vec![TokenTree::Ident("foo".to_string())];
        let bindings = match_pattern(&pattern, &tokens).unwrap();
        assert!(matches!(bindings.get("name"), Some(CapturedValue::Single(TokenTree::Ident(_)))));
    }

    #[test]
    fn test_match_capture_ident_fails_on_int() {
        let pattern = vec![MacroPatternElem::Capture {
            name: "name".to_string(),
            frag: FragSpec::Ident,
        }];
        let tokens = vec![TokenTree::IntLit(42)];
        assert!(match_pattern(&pattern, &tokens).is_none());
    }

    #[test]
    fn test_match_multiple_tokens() {
        let pattern = vec![
            MacroPatternElem::Token(TokenTree::Ident("let".to_string())),
            MacroPatternElem::Capture { name: "name".to_string(), frag: FragSpec::Ident },
            MacroPatternElem::Token(TokenTree::Punct("=".to_string())),
            MacroPatternElem::Capture { name: "val".to_string(), frag: FragSpec::Expr },
        ];
        let tokens = vec![
            TokenTree::Ident("let".to_string()),
            TokenTree::Ident("x".to_string()),
            TokenTree::Punct("=".to_string()),
            TokenTree::IntLit(10),
        ];
        let bindings = match_pattern(&pattern, &tokens).unwrap();
        assert!(matches!(bindings.get("name"), Some(CapturedValue::Single(TokenTree::Ident(_)))));
        assert!(matches!(bindings.get("val"), Some(CapturedValue::Single(TokenTree::IntLit(10)))));
    }

    #[test]
    fn test_match_repetition_zero_or_more() {
        let pattern = vec![MacroPatternElem::Repetition {
            elements: vec![MacroPatternElem::Capture {
                name: "x".to_string(),
                frag: FragSpec::Expr,
            }],
            separator: Some(",".to_string()),
            kind: RepKind::ZeroOrMore,
        }];
        // 3 elements separated by commas
        let tokens = vec![
            TokenTree::IntLit(1),
            TokenTree::Punct(",".to_string()),
            TokenTree::IntLit(2),
            TokenTree::Punct(",".to_string()),
            TokenTree::IntLit(3),
        ];
        let bindings = match_pattern(&pattern, &tokens).unwrap();
        if let Some(CapturedValue::Repeated(iters)) = bindings.get("x") {
            assert_eq!(iters.len(), 3);
        } else {
            panic!("expected repeated capture");
        }
    }

    #[test]
    fn test_match_repetition_zero_items() {
        let pattern = vec![MacroPatternElem::Repetition {
            elements: vec![MacroPatternElem::Capture {
                name: "x".to_string(),
                frag: FragSpec::Ident,
            }],
            separator: None,
            kind: RepKind::ZeroOrMore,
        }];
        let tokens: Vec<TokenTree> = vec![];
        let bindings = match_pattern(&pattern, &tokens).unwrap();
        if let Some(CapturedValue::Repeated(iters)) = bindings.get("x") {
            assert_eq!(iters.len(), 0);
        }
    }

    #[test]
    fn test_match_repetition_one_or_more_fails_empty() {
        let pattern = vec![MacroPatternElem::Repetition {
            elements: vec![MacroPatternElem::Capture {
                name: "x".to_string(),
                frag: FragSpec::Ident,
            }],
            separator: None,
            kind: RepKind::OneOrMore,
        }];
        let tokens: Vec<TokenTree> = vec![];
        assert!(match_pattern(&pattern, &tokens).is_none());
    }

    #[test]
    fn test_match_extra_tokens_fail() {
        let pattern = vec![MacroPatternElem::Token(TokenTree::Ident("a".to_string()))];
        let tokens = vec![
            TokenTree::Ident("a".to_string()),
            TokenTree::Ident("b".to_string()),
        ];
        // Should fail because there are unconsumed tokens
        assert!(match_pattern(&pattern, &tokens).is_none());
    }

    // ── Template Expansion ─────────────────────────────────────────

    #[test]
    fn test_expand_literal_token() {
        let template = vec![MacroTemplateElem::Token(TokenTree::IntLit(99))];
        let bindings = CaptureBindings::new();
        let mut hygiene = HygieneContext::new();
        let ctx = hygiene.fresh_context();
        let result = expand_template(&template, &bindings, &mut hygiene, ctx).unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], TokenTree::IntLit(99)));
    }

    #[test]
    fn test_expand_substitution() {
        let template = vec![MacroTemplateElem::Substitution("x".to_string())];
        let mut bindings = CaptureBindings::new();
        bindings.insert("x".to_string(), CapturedValue::Single(TokenTree::IntLit(42)));
        let mut hygiene = HygieneContext::new();
        let ctx = hygiene.fresh_context();
        let result = expand_template(&template, &bindings, &mut hygiene, ctx).unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], TokenTree::IntLit(42)));
    }

    #[test]
    fn test_expand_unbound_variable_error() {
        let template = vec![MacroTemplateElem::Substitution("missing".to_string())];
        let bindings = CaptureBindings::new();
        let mut hygiene = HygieneContext::new();
        let ctx = hygiene.fresh_context();
        let result = expand_template(&template, &bindings, &mut hygiene, ctx);
        assert!(matches!(result, Err(MacroError::UnboundVariable(_))));
    }

    // ── MacroExpander ──────────────────────────────────────────────

    #[test]
    fn test_expander_register_and_expand() {
        let mut expander = MacroExpander::new();
        let def = MacroDef {
            name: "double".to_string(),
            kind: MacroKind::Declarative,
            rules: vec![MacroRule {
                pattern: vec![MacroPatternElem::Capture {
                    name: "x".to_string(),
                    frag: FragSpec::Expr,
                }],
                template: vec![
                    MacroTemplateElem::Substitution("x".to_string()),
                    MacroTemplateElem::Token(TokenTree::Punct("+".to_string())),
                    MacroTemplateElem::Substitution("x".to_string()),
                ],
            }],
            is_exported: false,
        };
        expander.register_macro(def);
        assert!(expander.has_macro("double"));
        assert_eq!(expander.macro_count(), 1);

        let result = expander.expand("double", &[TokenTree::IntLit(5)]).unwrap();
        assert_eq!(result.len(), 3);
        assert!(matches!(result[0], TokenTree::IntLit(5)));
        assert!(matches!(result[2], TokenTree::IntLit(5)));
        assert_eq!(expander.expansion_count(), 1);
    }

    #[test]
    fn test_expander_unknown_macro() {
        let mut expander = MacroExpander::new();
        let result = expander.expand("nope", &[]);
        assert!(matches!(result, Err(MacroError::UnknownMacro(_))));
    }

    #[test]
    fn test_expander_no_matching_rule() {
        let mut expander = MacroExpander::new();
        let def = MacroDef {
            name: "test_mac".to_string(),
            kind: MacroKind::Declarative,
            rules: vec![MacroRule {
                pattern: vec![MacroPatternElem::Capture {
                    name: "x".to_string(),
                    frag: FragSpec::Ident,
                }],
                template: vec![MacroTemplateElem::Substitution("x".to_string())],
            }],
            is_exported: false,
        };
        expander.register_macro(def);
        // Pass an int instead of an ident
        let result = expander.expand("test_mac", &[TokenTree::IntLit(1)]);
        assert!(matches!(result, Err(MacroError::NoMatchingRule { .. })));
    }

    #[test]
    fn test_expander_recursion_limit() {
        let mut expander = MacroExpander::new();
        expander.set_max_recursion(2);
        // This tests the limit is set
        assert_eq!(expander.max_recursion, 2);
    }

    #[test]
    fn test_expander_list_macros() {
        let mut expander = MacroExpander::new();
        expander.register_macro(MacroDef {
            name: "a".to_string(),
            kind: MacroKind::Declarative,
            rules: vec![],
            is_exported: false,
        });
        expander.register_macro(MacroDef {
            name: "b".to_string(),
            kind: MacroKind::Declarative,
            rules: vec![],
            is_exported: false,
        });
        let names = expander.list_macros();
        assert_eq!(names.len(), 2);
    }

    // ── Derive Registry ────────────────────────────────────────────

    #[test]
    fn test_derive_registry_builtins() {
        let reg = DeriveRegistry::with_builtins();
        assert!(reg.has("Debug"));
        assert!(reg.has("Clone"));
        assert!(reg.has("PartialEq"));
        assert!(reg.has("Default"));
        assert!(reg.has("Display"));
        assert!(reg.has("Hash"));
        assert_eq!(reg.count(), 6);
    }

    #[test]
    fn test_derive_debug() {
        let reg = DeriveRegistry::with_builtins();
        let tokens = reg.expand("Debug", "Point", &["x", "y"]).unwrap();
        assert!(!tokens.is_empty());
        // Should contain "impl" and "Point"
        assert!(contains_ident(&tokens, "impl"));
        assert!(contains_ident(&tokens, "Point"));
        assert!(contains_ident(&tokens, "debug"));
    }

    #[test]
    fn test_derive_clone() {
        let reg = DeriveRegistry::with_builtins();
        let tokens = reg.expand("Clone", "Vec2", &["x", "y"]).unwrap();
        assert!(contains_ident(&tokens, "impl"));
        assert!(contains_ident(&tokens, "Vec2"));
        assert!(contains_ident(&tokens, "clone"));
    }

    #[test]
    fn test_derive_partial_eq() {
        let reg = DeriveRegistry::with_builtins();
        let tokens = reg.expand("PartialEq", "Color", &["r", "g", "b"]).unwrap();
        assert!(contains_ident(&tokens, "impl"));
        assert!(contains_ident(&tokens, "eq"));
    }

    #[test]
    fn test_derive_default() {
        let reg = DeriveRegistry::with_builtins();
        let tokens = reg.expand("Default", "Config", &["width", "height"]).unwrap();
        assert!(contains_ident(&tokens, "default"));
    }

    #[test]
    fn test_derive_display() {
        let reg = DeriveRegistry::with_builtins();
        let tokens = reg.expand("Display", "Item", &["name"]).unwrap();
        assert!(contains_ident(&tokens, "to_string"));
    }

    #[test]
    fn test_derive_hash() {
        let reg = DeriveRegistry::with_builtins();
        let tokens = reg.expand("Hash", "Key", &["id", "name"]).unwrap();
        assert!(contains_ident(&tokens, "hash"));
    }

    #[test]
    fn test_derive_unknown() {
        let reg = DeriveRegistry::with_builtins();
        let result = reg.expand("Serialize", "Foo", &["bar"]);
        assert!(matches!(result, Err(MacroError::UnknownDerive(_))));
    }

    #[test]
    fn test_expander_expand_derive() {
        let mut expander = MacroExpander::new();
        let tokens = expander.expand_derive("Debug", "MyStruct", &["a", "b"]).unwrap();
        assert!(!tokens.is_empty());
    }

    // ── Utility Functions ──────────────────────────────────────────

    #[test]
    fn test_count_tokens_flat() {
        let tokens = vec![TokenTree::IntLit(1), TokenTree::IntLit(2)];
        assert_eq!(count_tokens(&tokens), 2);
    }

    #[test]
    fn test_count_tokens_nested() {
        let tokens = vec![
            TokenTree::IntLit(1),
            TokenTree::Group {
                delimiter: Delimiter::Paren,
                tokens: vec![TokenTree::IntLit(2), TokenTree::IntLit(3)],
            },
        ];
        assert_eq!(count_tokens(&tokens), 4); // 1 + group(1) + 2 inner
    }

    #[test]
    fn test_flatten_tokens() {
        let tokens = vec![
            TokenTree::Ident("a".to_string()),
            TokenTree::Group {
                delimiter: Delimiter::Paren,
                tokens: vec![TokenTree::Ident("b".to_string())],
            },
        ];
        let flat = flatten_tokens(&tokens);
        assert_eq!(flat.len(), 2);
        assert!(matches!(&flat[0], TokenTree::Ident(s) if s == "a"));
        assert!(matches!(&flat[1], TokenTree::Ident(s) if s == "b"));
    }

    #[test]
    fn test_contains_ident_found() {
        let tokens = vec![
            TokenTree::IntLit(1),
            TokenTree::Ident("target".to_string()),
        ];
        assert!(contains_ident(&tokens, "target"));
    }

    #[test]
    fn test_contains_ident_not_found() {
        let tokens = vec![TokenTree::IntLit(1)];
        assert!(!contains_ident(&tokens, "target"));
    }

    #[test]
    fn test_contains_ident_nested() {
        let tokens = vec![TokenTree::Group {
            delimiter: Delimiter::Brace,
            tokens: vec![TokenTree::Ident("inner".to_string())],
        }];
        assert!(contains_ident(&tokens, "inner"));
    }

    // ── MacroError Display ─────────────────────────────────────────

    #[test]
    fn test_macro_error_display() {
        let e = MacroError::NoMatchingRule { macro_name: "foo".to_string() };
        assert!(e.to_string().contains("foo"));

        let e = MacroError::UnboundVariable("x".to_string());
        assert!(e.to_string().contains("$x"));

        let e = MacroError::RecursionLimit { depth: 50 };
        assert!(e.to_string().contains("50"));

        let e = MacroError::UnknownMacro("bar".to_string());
        assert!(e.to_string().contains("bar"));

        let e = MacroError::UnknownDerive("Serialize".to_string());
        assert!(e.to_string().contains("Serialize"));

        let e = MacroError::InvalidDeriveTarget { derive_name: "X".to_string() };
        assert!(e.to_string().contains("X"));

        let e = MacroError::ExpansionError("oops".to_string());
        assert!(e.to_string().contains("oops"));
    }

    // ── Matches Frag ───────────────────────────────────────────────

    #[test]
    fn test_matches_frag_literal() {
        assert!(matches_frag(FragSpec::Literal, &TokenTree::IntLit(1)));
        assert!(matches_frag(FragSpec::Literal, &TokenTree::FloatLit(1.0)));
        assert!(matches_frag(FragSpec::Literal, &TokenTree::StringLit("x".to_string())));
        assert!(matches_frag(FragSpec::Literal, &TokenTree::BoolLit(true)));
        assert!(!matches_frag(FragSpec::Literal, &TokenTree::Ident("x".to_string())));
    }

    #[test]
    fn test_matches_frag_block() {
        let block = TokenTree::Group {
            delimiter: Delimiter::Brace,
            tokens: vec![],
        };
        assert!(matches_frag(FragSpec::Block, &block));
        let paren = TokenTree::Group {
            delimiter: Delimiter::Paren,
            tokens: vec![],
        };
        assert!(!matches_frag(FragSpec::Block, &paren));
    }

    #[test]
    fn test_token_tree_bool_display() {
        assert_eq!(TokenTree::BoolLit(true).to_string(), "true");
        assert_eq!(TokenTree::BoolLit(false).to_string(), "false");
    }

    #[test]
    fn test_token_tree_float_display() {
        let tok = TokenTree::FloatLit(3.14);
        assert!(tok.to_string().starts_with("3.14"));
    }

    #[test]
    fn test_token_tree_punct_display() {
        let tok = TokenTree::Punct("->".to_string());
        assert_eq!(tok.to_string(), "->");
    }
}
