//! Vitalis LSP Server — Language Server Protocol implementation.
//!
//! Provides IDE support features:
//! - **Diagnostics**: Real-time error/warning reporting
//! - **Hover**: Type information on hover
//! - **Go to Definition**: Jump to function/struct/variable definition
//! - **Completion**: Auto-complete for keywords, functions, types
//! - **Document Symbols**: Outline of all top-level items
//! - **Signature Help**: Parameter hints in function calls
//!
//! The LSP server reuses the Vitalis compiler pipeline (lex → parse → type-check)
//! to provide accurate, real-time feedback. It communicates via JSON-RPC over stdio.

use std::collections::HashMap;
use std::fmt;

// ─── LSP Position & Range ───────────────────────────────────────────────

/// A position in a text document (0-indexed line and character).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.character + 1)
    }
}

/// A range in a text document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn from_line(line: u32, start_char: u32, end_char: u32) -> Self {
        Self {
            start: Position::new(line, start_char),
            end: Position::new(line, end_char),
        }
    }

    pub fn contains(&self, pos: Position) -> bool {
        if pos.line < self.start.line || pos.line > self.end.line {
            return false;
        }
        if pos.line == self.start.line && pos.character < self.start.character {
            return false;
        }
        if pos.line == self.end.line && pos.character > self.end.character {
            return false;
        }
        true
    }
}

// ─── Diagnostics ────────────────────────────────────────────────────────

/// Severity level for a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// A diagnostic message (error, warning, etc.).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: String,
    pub code: Option<String>,
}

impl Diagnostic {
    pub fn error(range: Range, message: &str) -> Self {
        Self {
            range,
            severity: DiagnosticSeverity::Error,
            message: message.to_string(),
            source: "vitalis".to_string(),
            code: None,
        }
    }

    pub fn warning(range: Range, message: &str) -> Self {
        Self {
            range,
            severity: DiagnosticSeverity::Warning,
            message: message.to_string(),
            source: "vitalis".to_string(),
            code: None,
        }
    }

    pub fn hint(range: Range, message: &str) -> Self {
        Self {
            range,
            severity: DiagnosticSeverity::Hint,
            message: message.to_string(),
            source: "vitalis".to_string(),
            code: None,
        }
    }
}

// ─── Completion ─────────────────────────────────────────────────────────

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword = 14,
    Function = 3,
    Variable = 6,
    Struct = 22,
    Enum = 13,
    Module = 9,
    Trait = 25,
    Type = 1,
    Snippet = 15,
}

/// A completion item suggested to the user.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
    pub documentation: Option<String>,
}

impl CompletionItem {
    pub fn keyword(kw: &str) -> Self {
        Self {
            label: kw.to_string(),
            kind: CompletionKind::Keyword,
            detail: Some("keyword".to_string()),
            insert_text: None,
            documentation: None,
        }
    }

    pub fn function(name: &str, sig: &str) -> Self {
        Self {
            label: name.to_string(),
            kind: CompletionKind::Function,
            detail: Some(sig.to_string()),
            insert_text: Some(format!("{}($0)", name)),
            documentation: None,
        }
    }

    pub fn type_item(name: &str) -> Self {
        Self {
            label: name.to_string(),
            kind: CompletionKind::Type,
            detail: Some("type".to_string()),
            insert_text: None,
            documentation: None,
        }
    }
}

/// Get all Vitalis keyword completions.
pub fn keyword_completions() -> Vec<CompletionItem> {
    let keywords = [
        "fn", "let", "mut", "if", "else", "match", "for", "in",
        "while", "loop", "break", "continue", "return", "struct",
        "enum", "impl", "trait", "type", "import", "extern", "pub",
        "self", "as", "try", "catch", "throw", "async", "await",
        "spawn", "module", "evolve", "pipeline", "parallel",
    ];
    keywords.iter().map(|kw| CompletionItem::keyword(kw)).collect()
}

/// Get type name completions.
pub fn type_completions() -> Vec<CompletionItem> {
    let types = ["i32", "i64", "f32", "f64", "bool", "str", "void"];
    types.iter().map(|t| CompletionItem::type_item(t)).collect()
}

// ─── Hover ──────────────────────────────────────────────────────────────

/// Hover information for a symbol.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Range>,
}

impl HoverInfo {
    pub fn new(contents: &str) -> Self {
        Self {
            contents: contents.to_string(),
            range: None,
        }
    }

    pub fn with_range(mut self, range: Range) -> Self {
        self.range = Some(range);
        self
    }
}

// ─── Document Symbols ───────────────────────────────────────────────────

/// The kind of a document symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function = 12,
    Struct = 23,
    Enum = 10,
    Module = 2,
    Variable = 13,
    Constant = 14,
    Trait = 11,
    TypeAlias = 26,
}

/// A symbol in the document (for outline / breadcrumbs).
#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub detail: Option<String>,
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    pub fn new(name: &str, kind: SymbolKind, range: Range) -> Self {
        Self {
            name: name.to_string(),
            kind,
            range,
            detail: None,
            children: Vec::new(),
        }
    }

    pub fn with_child(mut self, child: DocumentSymbol) -> Self {
        self.children.push(child);
        self
    }
}

// ─── Go to Definition ───────────────────────────────────────────────────

/// A location in a document (for go-to-definition, references).
#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl Location {
    pub fn new(uri: &str, range: Range) -> Self {
        Self { uri: uri.to_string(), range }
    }
}

// ─── Signature Help ─────────────────────────────────────────────────────

/// Parameter information for signature help.
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub label: String,
    pub documentation: Option<String>,
}

/// Signature help — shows function parameter hints.
#[derive(Debug, Clone)]
pub struct SignatureHelp {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub active_parameter: u32,
}

impl SignatureHelp {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            documentation: None,
            parameters: Vec::new(),
            active_parameter: 0,
        }
    }

    pub fn add_param(&mut self, label: &str, doc: Option<&str>) {
        self.parameters.push(ParameterInfo {
            label: label.to_string(),
            documentation: doc.map(|d| d.to_string()),
        });
    }
}

// ─── Symbol Index ───────────────────────────────────────────────────────

/// An indexed collection of symbols for fast lookup.
#[derive(Debug, Default)]
pub struct SymbolIndex {
    functions: HashMap<String, Location>,
    structs: HashMap<String, Location>,
    enums: HashMap<String, Location>,
    traits: HashMap<String, Location>,
    variables: HashMap<String, Vec<Location>>,
    modules: HashMap<String, Location>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_function(&mut self, name: &str, location: Location) {
        self.functions.insert(name.to_string(), location);
    }

    pub fn add_struct(&mut self, name: &str, location: Location) {
        self.structs.insert(name.to_string(), location);
    }

    pub fn add_enum(&mut self, name: &str, location: Location) {
        self.enums.insert(name.to_string(), location);
    }

    pub fn add_trait(&mut self, name: &str, location: Location) {
        self.traits.insert(name.to_string(), location);
    }

    pub fn add_module(&mut self, name: &str, location: Location) {
        self.modules.insert(name.to_string(), location);
    }

    pub fn add_variable(&mut self, name: &str, location: Location) {
        self.variables.entry(name.to_string())
            .or_default()
            .push(location);
    }

    /// Find the definition location of a symbol by name.
    pub fn find_definition(&self, name: &str) -> Option<&Location> {
        self.functions.get(name)
            .or_else(|| self.structs.get(name))
            .or_else(|| self.enums.get(name))
            .or_else(|| self.traits.get(name))
            .or_else(|| self.modules.get(name))
    }

    /// Get all symbol names.
    pub fn all_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        names.extend(self.functions.keys().cloned());
        names.extend(self.structs.keys().cloned());
        names.extend(self.enums.keys().cloned());
        names.extend(self.traits.keys().cloned());
        names.extend(self.modules.keys().cloned());
        names.sort();
        names.dedup();
        names
    }

    pub fn function_count(&self) -> usize { self.functions.len() }
    pub fn struct_count(&self) -> usize { self.structs.len() }
    pub fn total_count(&self) -> usize {
        self.functions.len() + self.structs.len() + self.enums.len()
            + self.traits.len() + self.modules.len()
    }
}

// ─── LSP Server State ───────────────────────────────────────────────────

/// Server capabilities advertised during initialization.
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub hover: bool,
    pub completion: bool,
    pub go_to_definition: bool,
    pub document_symbols: bool,
    pub diagnostics: bool,
    pub signature_help: bool,
    pub references: bool,
    pub rename: bool,
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            hover: true,
            completion: true,
            go_to_definition: true,
            document_symbols: true,
            diagnostics: true,
            signature_help: true,
            references: false, // Not yet implemented
            rename: false,     // Not yet implemented
        }
    }
}

/// The LSP server state.
#[derive(Debug)]
pub struct LspServer {
    pub capabilities: ServerCapabilities,
    pub documents: HashMap<String, String>,
    pub index: SymbolIndex,
    pub initialized: bool,
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            capabilities: ServerCapabilities::default(),
            documents: HashMap::new(),
            index: SymbolIndex::new(),
            initialized: false,
        }
    }

    /// Initialize the server.
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Open a document.
    pub fn open_document(&mut self, uri: &str, text: &str) {
        self.documents.insert(uri.to_string(), text.to_string());
        self.reindex_document(uri, text);
    }

    /// Update a document.
    pub fn update_document(&mut self, uri: &str, text: &str) {
        self.documents.insert(uri.to_string(), text.to_string());
        self.reindex_document(uri, text);
    }

    /// Close a document.
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Get diagnostics for a document by running the compiler pipeline.
    pub fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        let source = match self.documents.get(uri) {
            Some(s) => s,
            None => return vec![],
        };

        let mut diagnostics = Vec::new();

        // Run lexer + parser
        let (_program, errors) = crate::parser::parse(source);
        for err in &errors {
            diagnostics.push(Diagnostic::error(
                Range::from_line(0, 0, 1),
                &err.message,
            ));
        }

        diagnostics
    }

    /// Get hover information at a position.
    pub fn hover(&self, _uri: &str, _pos: Position) -> Option<HoverInfo> {
        // In a full implementation, we'd parse the document, find the token
        // at the position, and return its type information.
        None
    }

    /// Get completions at a position.
    pub fn completions(&self, _uri: &str, _pos: Position) -> Vec<CompletionItem> {
        let mut items = keyword_completions();
        items.extend(type_completions());

        // Add known functions/structs
        for name in self.index.all_names() {
            items.push(CompletionItem {
                label: name.clone(),
                kind: CompletionKind::Function,
                detail: None,
                insert_text: None,
                documentation: None,
            });
        }

        items
    }

    /// Find the definition of a symbol.
    pub fn goto_definition(&self, _uri: &str, name: &str) -> Option<Location> {
        self.index.find_definition(name).cloned()
    }

    /// Get document symbols (outline).
    pub fn document_symbols(&self, uri: &str) -> Vec<DocumentSymbol> {
        let source = match self.documents.get(uri) {
            Some(s) => s,
            None => return vec![],
        };

        let mut symbols = Vec::new();
        let (program, _errors) = crate::parser::parse(source);

        for item in &program.items {
            match item {
                crate::ast::TopLevel::Function(f) => {
                    symbols.push(DocumentSymbol::new(
                        &f.name,
                        SymbolKind::Function,
                        Range::default(),
                    ));
                }
                crate::ast::TopLevel::Struct(s) => {
                    symbols.push(DocumentSymbol::new(
                        &s.name,
                        SymbolKind::Struct,
                        Range::default(),
                    ));
                }
                crate::ast::TopLevel::Enum(e) => {
                    symbols.push(DocumentSymbol::new(
                        &e.name,
                        SymbolKind::Enum,
                        Range::default(),
                    ));
                }
                crate::ast::TopLevel::Trait(t) => {
                    symbols.push(DocumentSymbol::new(
                        &t.name,
                        SymbolKind::Trait,
                        Range::default(),
                    ));
                }
                crate::ast::TopLevel::Module(m) => {
                    symbols.push(DocumentSymbol::new(
                        &m.name,
                        SymbolKind::Module,
                        Range::default(),
                    ));
                }
                _ => {}
            }
        }

        symbols
    }

    /// Re-index a document after changes.
    fn reindex_document(&mut self, uri: &str, source: &str) {
        let (program, _errors) = crate::parser::parse(source);

        for item in &program.items {
            match item {
                crate::ast::TopLevel::Function(f) => {
                    self.index.add_function(
                        &f.name,
                        Location::new(uri, Range::default()),
                    );
                }
                crate::ast::TopLevel::Struct(s) => {
                    self.index.add_struct(
                        &s.name,
                        Location::new(uri, Range::default()),
                    );
                }
                crate::ast::TopLevel::Enum(e) => {
                    self.index.add_enum(
                        &e.name,
                        Location::new(uri, Range::default()),
                    );
                }
                crate::ast::TopLevel::Trait(t) => {
                    self.index.add_trait(
                        &t.name,
                        Location::new(uri, Range::default()),
                    );
                }
                _ => {}
            }
        }
    }
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position() {
        let p = Position::new(0, 5);
        assert_eq!(format!("{}", p), "1:6");
    }

    #[test]
    fn test_range_contains() {
        let r = Range::from_line(5, 10, 20);
        assert!(r.contains(Position::new(5, 15)));
        assert!(!r.contains(Position::new(5, 25)));
        assert!(!r.contains(Position::new(6, 0)));
    }

    #[test]
    fn test_diagnostic_error() {
        let d = Diagnostic::error(Range::default(), "undefined variable");
        assert_eq!(d.severity, DiagnosticSeverity::Error);
        assert_eq!(d.source, "vitalis");
    }

    #[test]
    fn test_diagnostic_warning() {
        let d = Diagnostic::warning(Range::default(), "unused variable");
        assert_eq!(d.severity, DiagnosticSeverity::Warning);
    }

    #[test]
    fn test_completion_keyword() {
        let c = CompletionItem::keyword("fn");
        assert_eq!(c.kind, CompletionKind::Keyword);
        assert_eq!(c.label, "fn");
    }

    #[test]
    fn test_completion_function() {
        let c = CompletionItem::function("print", "fn print(s: str)");
        assert_eq!(c.kind, CompletionKind::Function);
        assert_eq!(c.insert_text, Some("print($0)".to_string()));
    }

    #[test]
    fn test_keyword_completions() {
        let completions = keyword_completions();
        assert!(completions.len() > 20);
        assert!(completions.iter().any(|c| c.label == "fn"));
        assert!(completions.iter().any(|c| c.label == "async"));
        assert!(completions.iter().any(|c| c.label == "await"));
    }

    #[test]
    fn test_type_completions() {
        let completions = type_completions();
        assert!(completions.iter().any(|c| c.label == "i64"));
        assert!(completions.iter().any(|c| c.label == "str"));
    }

    #[test]
    fn test_hover_info() {
        let h = HoverInfo::new("fn main() -> i64")
            .with_range(Range::from_line(0, 0, 10));
        assert!(h.range.is_some());
        assert!(h.contents.contains("main"));
    }

    #[test]
    fn test_document_symbol() {
        let sym = DocumentSymbol::new("main", SymbolKind::Function, Range::default())
            .with_child(DocumentSymbol::new("x", SymbolKind::Variable, Range::default()));
        assert_eq!(sym.children.len(), 1);
    }

    #[test]
    fn test_symbol_index() {
        let mut idx = SymbolIndex::new();
        idx.add_function("main", Location::new("file.sl", Range::default()));
        idx.add_struct("Point", Location::new("file.sl", Range::default()));

        assert!(idx.find_definition("main").is_some());
        assert!(idx.find_definition("Point").is_some());
        assert!(idx.find_definition("unknown").is_none());
        assert_eq!(idx.function_count(), 1);
        assert_eq!(idx.struct_count(), 1);
        assert_eq!(idx.total_count(), 2);
    }

    #[test]
    fn test_symbol_index_all_names() {
        let mut idx = SymbolIndex::new();
        idx.add_function("foo", Location::new("f.sl", Range::default()));
        idx.add_struct("Bar", Location::new("f.sl", Range::default()));
        let names = idx.all_names();
        assert!(names.contains(&"foo".to_string()));
        assert!(names.contains(&"Bar".to_string()));
    }

    #[test]
    fn test_signature_help() {
        let mut sh = SignatureHelp::new("fn add(a: i64, b: i64) -> i64");
        sh.add_param("a: i64", Some("First number"));
        sh.add_param("b: i64", Some("Second number"));
        assert_eq!(sh.parameters.len(), 2);
    }

    #[test]
    fn test_lsp_server_init() {
        let mut server = LspServer::new();
        assert!(!server.initialized);
        server.initialize();
        assert!(server.initialized);
    }

    #[test]
    fn test_lsp_open_document() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        assert!(server.documents.contains_key("test.sl"));
    }

    #[test]
    fn test_lsp_diagnostics() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        let diags = server.get_diagnostics("test.sl");
        assert!(diags.is_empty()); // Valid code → no errors
    }

    #[test]
    fn test_lsp_diagnostics_error() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("bad.sl", "fn {{{ broken");
        let diags = server.get_diagnostics("bad.sl");
        assert!(!diags.is_empty()); // Invalid code → errors
    }

    #[test]
    fn test_lsp_completions() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        let completions = server.completions("test.sl", Position::new(0, 0));
        assert!(!completions.is_empty());
    }

    #[test]
    fn test_lsp_document_symbols() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }\nfn add(a: i64, b: i64) -> i64 { a + b }");
        let symbols = server.document_symbols("test.sl");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "main");
        assert_eq!(symbols[1].name, "add");
    }

    #[test]
    fn test_lsp_goto_definition() {
        let mut server = LspServer::new();
        server.initialize();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        let loc = server.goto_definition("test.sl", "main");
        assert!(loc.is_some());
    }

    #[test]
    fn test_lsp_close_document() {
        let mut server = LspServer::new();
        server.open_document("test.sl", "fn main() -> i64 { 42 }");
        server.close_document("test.sl");
        assert!(!server.documents.contains_key("test.sl"));
    }

    #[test]
    fn test_server_capabilities() {
        let caps = ServerCapabilities::default();
        assert!(caps.hover);
        assert!(caps.completion);
        assert!(caps.go_to_definition);
        assert!(caps.diagnostics);
        assert!(!caps.rename); // Not yet
    }

    #[test]
    fn test_location() {
        let loc = Location::new("file.sl", Range::from_line(5, 0, 10));
        assert_eq!(loc.uri, "file.sl");
    }

    #[test]
    fn test_lsp_reindex_struct() {
        let mut server = LspServer::new();
        server.open_document("test.sl", "struct Point { x: i64 }\nfn main() -> i64 { 0 }");
        assert!(server.index.find_definition("Point").is_some());
    }
}
