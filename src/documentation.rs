//! Documentation generation system for Vitalis.
//!
//! Parses doc comments (/// and /** */), builds a structured API documentation
//! model (modules, functions, structs, enums, traits), generates Markdown and
//! HTML output, resolves cross-references, and extracts code examples.
//!
//! Modeled after Rust's rustdoc, Go's godoc, and Java's javadoc.

use std::collections::HashMap;
use std::fmt;

// ── Doc Comment Parsing ──────────────────────────────────────────────

/// A parsed documentation comment section.
#[derive(Debug, Clone, PartialEq)]
pub enum DocSection {
    /// Free-form description text.
    Description(String),
    /// @param name description.
    Param { name: String, description: String },
    /// @returns description.
    Returns(String),
    /// @example with code block.
    Example { label: Option<String>, code: String },
    /// @see cross-reference.
    SeeAlso(String),
    /// @since version.
    Since(String),
    /// @deprecated reason.
    Deprecated(String),
    /// @throws / @raises error description.
    Throws { error_type: String, description: String },
    /// @note informational note.
    Note(String),
    /// @warning caution text.
    Warning(String),
}

impl fmt::Display for DocSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Description(text) => write!(f, "{text}"),
            Self::Param { name, description } => write!(f, "@param {name} — {description}"),
            Self::Returns(desc) => write!(f, "@returns {desc}"),
            Self::Example { label, code } => {
                if let Some(lbl) = label {
                    write!(f, "@example ({lbl})\n```\n{code}\n```")
                } else {
                    write!(f, "@example\n```\n{code}\n```")
                }
            }
            Self::SeeAlso(ref_name) => write!(f, "@see {ref_name}"),
            Self::Since(version) => write!(f, "@since {version}"),
            Self::Deprecated(reason) => write!(f, "@deprecated {reason}"),
            Self::Throws { error_type, description } => {
                write!(f, "@throws {error_type} — {description}")
            }
            Self::Note(text) => write!(f, "@note {text}"),
            Self::Warning(text) => write!(f, "@warning {text}"),
        }
    }
}

/// A fully parsed doc comment with all sections extracted.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocComment {
    pub sections: Vec<DocSection>,
}

impl DocComment {
    pub fn new() -> Self {
        Self { sections: Vec::new() }
    }

    pub fn add_section(&mut self, section: DocSection) {
        self.sections.push(section);
    }

    /// Get the main description (first Description section if any).
    pub fn description(&self) -> Option<&str> {
        self.sections.iter().find_map(|s| {
            if let DocSection::Description(text) = s {
                Some(text.as_str())
            } else {
                None
            }
        })
    }

    /// Get all @param sections.
    pub fn params(&self) -> Vec<(&str, &str)> {
        self.sections.iter().filter_map(|s| {
            if let DocSection::Param { name, description } = s {
                Some((name.as_str(), description.as_str()))
            } else {
                None
            }
        }).collect()
    }

    /// Get the @returns section.
    pub fn returns(&self) -> Option<&str> {
        self.sections.iter().find_map(|s| {
            if let DocSection::Returns(desc) = s {
                Some(desc.as_str())
            } else {
                None
            }
        })
    }

    /// Get all @example sections.
    pub fn examples(&self) -> Vec<(Option<&str>, &str)> {
        self.sections.iter().filter_map(|s| {
            if let DocSection::Example { label, code } = s {
                Some((label.as_deref(), code.as_str()))
            } else {
                None
            }
        }).collect()
    }

    /// Check if this item is deprecated.
    pub fn is_deprecated(&self) -> bool {
        self.sections.iter().any(|s| matches!(s, DocSection::Deprecated(_)))
    }

    /// Get the deprecation reason if deprecated.
    pub fn deprecation_reason(&self) -> Option<&str> {
        self.sections.iter().find_map(|s| {
            if let DocSection::Deprecated(reason) = s {
                Some(reason.as_str())
            } else {
                None
            }
        })
    }

    /// Get all cross-references.
    pub fn see_also(&self) -> Vec<&str> {
        self.sections.iter().filter_map(|s| {
            if let DocSection::SeeAlso(ref_name) = s {
                Some(ref_name.as_str())
            } else {
                None
            }
        }).collect()
    }

    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }
}

// ── Doc Comment Parser ───────────────────────────────────────────────

/// Parse a raw doc comment string into structured sections.
pub fn parse_doc_comment(raw: &str) -> DocComment {
    let mut doc = DocComment::new();
    let mut current_desc = String::new();
    let mut in_example = false;
    let mut example_label: Option<String> = None;
    let mut example_code = String::new();

    for line in raw.lines() {
        let trimmed = line.trim()
            .trim_start_matches("///")
            .trim_start_matches("/**")
            .trim_end_matches("*/")
            .trim_start_matches('*')
            .trim();

        // Check for code block boundaries  
        if trimmed.starts_with("```") {
            if in_example {
                // End of example
                doc.add_section(DocSection::Example {
                    label: example_label.take(),
                    code: example_code.trim().to_string(),
                });
                example_code.clear();
                in_example = false;
            } else {
                // Start of example
                in_example = true;
                example_code.clear();
            }
            continue;
        }

        if in_example {
            if !example_code.is_empty() {
                example_code.push('\n');
            }
            example_code.push_str(trimmed);
            continue;
        }

        if trimmed.is_empty() {
            if !current_desc.is_empty() {
                doc.add_section(DocSection::Description(current_desc.trim().to_string()));
                current_desc.clear();
            }
            continue;
        }

        // Parse @tags
        if let Some(rest) = trimmed.strip_prefix("@param ") {
            if !current_desc.is_empty() {
                doc.add_section(DocSection::Description(current_desc.trim().to_string()));
                current_desc.clear();
            }
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            let name = parts[0].to_string();
            let description = parts.get(1).unwrap_or(&"").to_string();
            doc.add_section(DocSection::Param { name, description });
        } else if let Some(rest) = trimmed.strip_prefix("@returns ").or_else(|| trimmed.strip_prefix("@return ")) {
            if !current_desc.is_empty() {
                doc.add_section(DocSection::Description(current_desc.trim().to_string()));
                current_desc.clear();
            }
            doc.add_section(DocSection::Returns(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("@see ") {
            if !current_desc.is_empty() {
                doc.add_section(DocSection::Description(current_desc.trim().to_string()));
                current_desc.clear();
            }
            doc.add_section(DocSection::SeeAlso(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("@since ") {
            doc.add_section(DocSection::Since(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("@deprecated ") {
            doc.add_section(DocSection::Deprecated(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("@throws ").or_else(|| trimmed.strip_prefix("@raises ")) {
            if !current_desc.is_empty() {
                doc.add_section(DocSection::Description(current_desc.trim().to_string()));
                current_desc.clear();
            }
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            let error_type = parts[0].to_string();
            let description = parts.get(1).unwrap_or(&"").to_string();
            doc.add_section(DocSection::Throws { error_type, description });
        } else if let Some(rest) = trimmed.strip_prefix("@example") {
            if !current_desc.is_empty() {
                doc.add_section(DocSection::Description(current_desc.trim().to_string()));
                current_desc.clear();
            }
            let label_text = rest.trim();
            if !label_text.is_empty() {
                example_label = Some(label_text.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("@note ") {
            doc.add_section(DocSection::Note(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("@warning ") {
            doc.add_section(DocSection::Warning(rest.to_string()));
        } else {
            if !current_desc.is_empty() {
                current_desc.push(' ');
            }
            current_desc.push_str(trimmed);
        }
    }

    if !current_desc.is_empty() {
        doc.add_section(DocSection::Description(current_desc.trim().to_string()));
    }

    doc
}

// ── API Documentation Model ──────────────────────────────────────────

/// Visibility of a documented item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Internal,
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Public => write!(f, "pub"),
            Self::Private => write!(f, "priv"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

/// A documented function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct DocParam {
    pub name: String,
    pub type_name: String,
    pub description: Option<String>,
    pub default_value: Option<String>,
}

/// Documentation for a function.
#[derive(Debug, Clone)]
pub struct DocFunction {
    pub name: String,
    pub visibility: Visibility,
    pub params: Vec<DocParam>,
    pub return_type: Option<String>,
    pub doc: DocComment,
    pub is_async: bool,
    pub is_const: bool,
    pub generic_params: Vec<String>,
}

impl DocFunction {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            visibility: Visibility::Public,
            params: Vec::new(),
            return_type: None,
            doc: DocComment::new(),
            is_async: false,
            is_const: false,
            generic_params: Vec::new(),
        }
    }

    pub fn signature(&self) -> String {
        let async_prefix = if self.is_async { "async " } else { "" };
        let const_prefix = if self.is_const { "const " } else { "" };
        let generics = if self.generic_params.is_empty() {
            String::new()
        } else {
            format!("<{}>", self.generic_params.join(", "))
        };
        let params: Vec<String> = self.params.iter()
            .map(|p| format!("{}: {}", p.name, p.type_name))
            .collect();
        let ret = self.return_type.as_deref().unwrap_or("void");
        format!("{async_prefix}{const_prefix}fn {}{generics}({}) -> {ret}", self.name, params.join(", "))
    }
}

/// Documentation for a struct field.
#[derive(Debug, Clone, PartialEq)]
pub struct DocField {
    pub name: String,
    pub type_name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
}

/// Documentation for a struct.
#[derive(Debug, Clone)]
pub struct DocStruct {
    pub name: String,
    pub visibility: Visibility,
    pub fields: Vec<DocField>,
    pub methods: Vec<DocFunction>,
    pub doc: DocComment,
    pub generic_params: Vec<String>,
}

impl DocStruct {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            visibility: Visibility::Public,
            fields: Vec::new(),
            methods: Vec::new(),
            doc: DocComment::new(),
            generic_params: Vec::new(),
        }
    }
}

/// Documentation for an enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct DocVariant {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub description: Option<String>,
}

/// Documentation for an enum.
#[derive(Debug, Clone)]
pub struct DocEnum {
    pub name: String,
    pub visibility: Visibility,
    pub variants: Vec<DocVariant>,
    pub methods: Vec<DocFunction>,
    pub doc: DocComment,
    pub generic_params: Vec<String>,
}

impl DocEnum {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            visibility: Visibility::Public,
            variants: Vec::new(),
            methods: Vec::new(),
            doc: DocComment::new(),
            generic_params: Vec::new(),
        }
    }
}

/// Documentation for a trait.
#[derive(Debug, Clone)]
pub struct DocTrait {
    pub name: String,
    pub visibility: Visibility,
    pub methods: Vec<DocFunction>,
    pub doc: DocComment,
    pub generic_params: Vec<String>,
    pub super_traits: Vec<String>,
}

impl DocTrait {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            visibility: Visibility::Public,
            methods: Vec::new(),
            doc: DocComment::new(),
            generic_params: Vec::new(),
            super_traits: Vec::new(),
        }
    }
}

/// Documentation for a type alias.
#[derive(Debug, Clone)]
pub struct DocTypeAlias {
    pub name: String,
    pub target: String,
    pub doc: DocComment,
    pub visibility: Visibility,
}

// ── Module Documentation ─────────────────────────────────────────────

/// Full documentation for a module.
#[derive(Debug, Clone)]
pub struct DocModule {
    pub name: String,
    pub path: String,
    pub doc: DocComment,
    pub functions: Vec<DocFunction>,
    pub structs: Vec<DocStruct>,
    pub enums: Vec<DocEnum>,
    pub traits: Vec<DocTrait>,
    pub type_aliases: Vec<DocTypeAlias>,
    pub submodules: Vec<String>,
}

impl DocModule {
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            doc: DocComment::new(),
            functions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            type_aliases: Vec::new(),
            submodules: Vec::new(),
        }
    }

    /// Total number of documented items in this module.
    pub fn item_count(&self) -> usize {
        self.functions.len()
            + self.structs.len()
            + self.enums.len()
            + self.traits.len()
            + self.type_aliases.len()
    }

    /// Find a function by name.
    pub fn find_function(&self, name: &str) -> Option<&DocFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Find a struct by name.
    pub fn find_struct(&self, name: &str) -> Option<&DocStruct> {
        self.structs.iter().find(|s| s.name == name)
    }
}

// ── Cross-Reference Resolution ───────────────────────────────────────

/// A resolved cross-reference target.
#[derive(Debug, Clone, PartialEq)]
pub enum DocRef {
    Function { module: String, name: String },
    Struct { module: String, name: String },
    Enum { module: String, name: String },
    Trait { module: String, name: String },
    Module { name: String },
    External { url: String },
    Unresolved { target: String },
}

impl fmt::Display for DocRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function { module, name } => write!(f, "{module}::{name}()"),
            Self::Struct { module, name } => write!(f, "{module}::{name}"),
            Self::Enum { module, name } => write!(f, "{module}::{name}"),
            Self::Trait { module, name } => write!(f, "{module}::{name}"),
            Self::Module { name } => write!(f, "mod {name}"),
            Self::External { url } => write!(f, "{url}"),
            Self::Unresolved { target } => write!(f, "?{target}"),
        }
    }
}

/// Cross-reference resolver for linking documentation together.
#[derive(Debug, Clone)]
pub struct RefResolver {
    /// All known items: (qualified_name, DocRef).
    known: HashMap<String, DocRef>,
}

impl RefResolver {
    pub fn new() -> Self {
        Self { known: HashMap::new() }
    }

    /// Register a known item.
    pub fn register(&mut self, qualified_name: &str, doc_ref: DocRef) {
        self.known.insert(qualified_name.to_string(), doc_ref);
    }

    /// Index a module, registering all its items.
    pub fn index_module(&mut self, module: &DocModule) {
        let mod_name = &module.name;
        self.register(mod_name, DocRef::Module { name: mod_name.clone() });

        for func in &module.functions {
            let qn = format!("{mod_name}::{}", func.name);
            self.register(&qn, DocRef::Function {
                module: mod_name.clone(),
                name: func.name.clone(),
            });
        }
        for s in &module.structs {
            let qn = format!("{mod_name}::{}", s.name);
            self.register(&qn, DocRef::Struct {
                module: mod_name.clone(),
                name: s.name.clone(),
            });
        }
        for e in &module.enums {
            let qn = format!("{mod_name}::{}", e.name);
            self.register(&qn, DocRef::Enum {
                module: mod_name.clone(),
                name: e.name.clone(),
            });
        }
        for t in &module.traits {
            let qn = format!("{mod_name}::{}", t.name);
            self.register(&qn, DocRef::Trait {
                module: mod_name.clone(),
                name: t.name.clone(),
            });
        }
    }

    /// Resolve a reference string to a DocRef.
    pub fn resolve(&self, reference: &str) -> DocRef {
        if let Some(doc_ref) = self.known.get(reference) {
            doc_ref.clone()
        } else if reference.starts_with("http://") || reference.starts_with("https://") {
            DocRef::External { url: reference.to_string() }
        } else {
            DocRef::Unresolved { target: reference.to_string() }
        }
    }

    pub fn known_count(&self) -> usize {
        self.known.len()
    }
}

impl Default for RefResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ── Output Generation ────────────────────────────────────────────────

/// Output format for generated documentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Markdown,
    Html,
    PlainText,
}

/// Generate documentation for a module in the specified format.
pub fn generate_module_docs(module: &DocModule, format: OutputFormat) -> String {
    match format {
        OutputFormat::Markdown => generate_markdown(module),
        OutputFormat::Html => generate_html(module),
        OutputFormat::PlainText => generate_plaintext(module),
    }
}

fn generate_markdown(module: &DocModule) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Module `{}`\n\n", module.name));

    if let Some(desc) = module.doc.description() {
        out.push_str(desc);
        out.push_str("\n\n");
    }

    if !module.structs.is_empty() {
        out.push_str("## Structs\n\n");
        for s in &module.structs {
            out.push_str(&format!("### `{}`\n\n", s.name));
            if let Some(desc) = s.doc.description() {
                out.push_str(desc);
                out.push_str("\n\n");
            }
            if !s.fields.is_empty() {
                out.push_str("**Fields:**\n\n");
                for f in &s.fields {
                    let desc = f.description.as_deref().unwrap_or("");
                    out.push_str(&format!("- `{}`: `{}` — {desc}\n", f.name, f.type_name));
                }
                out.push('\n');
            }
        }
    }

    if !module.enums.is_empty() {
        out.push_str("## Enums\n\n");
        for e in &module.enums {
            out.push_str(&format!("### `{}`\n\n", e.name));
            if let Some(desc) = e.doc.description() {
                out.push_str(desc);
                out.push_str("\n\n");
            }
            if !e.variants.is_empty() {
                out.push_str("**Variants:**\n\n");
                for v in &e.variants {
                    let desc = v.description.as_deref().unwrap_or("");
                    out.push_str(&format!("- `{}` — {desc}\n", v.name));
                }
                out.push('\n');
            }
        }
    }

    if !module.traits.is_empty() {
        out.push_str("## Traits\n\n");
        for t in &module.traits {
            out.push_str(&format!("### `{}`\n\n", t.name));
            if let Some(desc) = t.doc.description() {
                out.push_str(desc);
                out.push_str("\n\n");
            }
        }
    }

    if !module.functions.is_empty() {
        out.push_str("## Functions\n\n");
        for f in &module.functions {
            out.push_str(&format!("### `{}`\n\n", f.name));
            out.push_str(&format!("```\n{}\n```\n\n", f.signature()));
            if let Some(desc) = f.doc.description() {
                out.push_str(desc);
                out.push_str("\n\n");
            }
            let params = f.doc.params();
            if !params.is_empty() {
                out.push_str("**Parameters:**\n\n");
                for (name, desc) in params {
                    out.push_str(&format!("- `{name}` — {desc}\n"));
                }
                out.push('\n');
            }
            if let Some(ret) = f.doc.returns() {
                out.push_str(&format!("**Returns:** {ret}\n\n"));
            }
        }
    }

    if !module.type_aliases.is_empty() {
        out.push_str("## Type Aliases\n\n");
        for ta in &module.type_aliases {
            out.push_str(&format!("- `type {} = {}`", ta.name, ta.target));
            if let Some(desc) = ta.doc.description() {
                out.push_str(&format!(" — {desc}"));
            }
            out.push('\n');
        }
    }

    out
}

fn generate_html(module: &DocModule) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    out.push_str(&format!("<title>Module {}</title>\n", module.name));
    out.push_str("<style>body{font-family:sans-serif;max-width:800px;margin:auto;padding:20px;}\n");
    out.push_str("code{background:#f4f4f4;padding:2px 4px;border-radius:3px;}\n");
    out.push_str("pre{background:#f4f4f4;padding:12px;overflow-x:auto;}\n");
    out.push_str(".deprecated{text-decoration:line-through;color:#999;}\n");
    out.push_str("</style>\n</head>\n<body>\n");
    out.push_str(&format!("<h1>Module <code>{}</code></h1>\n", module.name));

    if let Some(desc) = module.doc.description() {
        out.push_str(&format!("<p>{desc}</p>\n"));
    }

    if !module.functions.is_empty() {
        out.push_str("<h2>Functions</h2>\n");
        for f in &module.functions {
            let class = if f.doc.is_deprecated() { " class=\"deprecated\"" } else { "" };
            out.push_str(&format!("<h3{class}><code>{}</code></h3>\n", f.name));
            out.push_str(&format!("<pre>{}</pre>\n", f.signature()));
            if let Some(desc) = f.doc.description() {
                out.push_str(&format!("<p>{desc}</p>\n"));
            }
        }
    }

    if !module.structs.is_empty() {
        out.push_str("<h2>Structs</h2>\n");
        for s in &module.structs {
            out.push_str(&format!("<h3><code>{}</code></h3>\n", s.name));
            if let Some(desc) = s.doc.description() {
                out.push_str(&format!("<p>{desc}</p>\n"));
            }
        }
    }

    out.push_str("</body>\n</html>");
    out
}

fn generate_plaintext(module: &DocModule) -> String {
    let mut out = String::new();
    out.push_str(&format!("MODULE: {}\n", module.name));
    out.push_str(&"=".repeat(40));
    out.push('\n');

    if let Some(desc) = module.doc.description() {
        out.push_str(desc);
        out.push_str("\n\n");
    }

    if !module.functions.is_empty() {
        out.push_str("FUNCTIONS:\n");
        for f in &module.functions {
            out.push_str(&format!("  {}\n", f.signature()));
            if let Some(desc) = f.doc.description() {
                out.push_str(&format!("    {desc}\n"));
            }
        }
    }

    if !module.structs.is_empty() {
        out.push_str("\nSTRUCTS:\n");
        for s in &module.structs {
            out.push_str(&format!("  {}\n", s.name));
        }
    }

    out
}

// ── Documentation Index ──────────────────────────────────────────────

/// A full documentation index across all modules.
#[derive(Debug, Clone)]
pub struct DocIndex {
    pub modules: Vec<DocModule>,
    pub resolver: RefResolver,
}

impl DocIndex {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            resolver: RefResolver::new(),
        }
    }

    /// Add a module and index its items.
    pub fn add_module(&mut self, module: DocModule) {
        self.resolver.index_module(&module);
        self.modules.push(module);
    }

    /// Find a module by name.
    pub fn find_module(&self, name: &str) -> Option<&DocModule> {
        self.modules.iter().find(|m| m.name == name)
    }

    /// Total documented items across all modules.
    pub fn total_items(&self) -> usize {
        self.modules.iter().map(|m| m.item_count()).sum()
    }

    /// Generate a table of contents in Markdown.
    pub fn table_of_contents(&self) -> String {
        let mut out = String::from("# API Documentation\n\n");
        out.push_str("## Modules\n\n");
        for module in &self.modules {
            out.push_str(&format!(
                "- [{}]({}.md) — {} items\n",
                module.name,
                module.name,
                module.item_count(),
            ));
        }
        out
    }

    /// Generate all module docs in a given format.
    pub fn generate_all(&self, format: OutputFormat) -> Vec<(String, String)> {
        self.modules.iter()
            .map(|m| (m.name.clone(), generate_module_docs(m, format)))
            .collect()
    }

    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

impl Default for DocIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ── Example Extraction ───────────────────────────────────────────────

/// An extracted code example from documentation.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedExample {
    pub source_module: String,
    pub source_item: String,
    pub label: Option<String>,
    pub code: String,
}

/// Extract all code examples from a doc index.
pub fn extract_examples(index: &DocIndex) -> Vec<ExtractedExample> {
    let mut examples = Vec::new();

    for module in &index.modules {
        for func in &module.functions {
            for (label, code) in func.doc.examples() {
                examples.push(ExtractedExample {
                    source_module: module.name.clone(),
                    source_item: func.name.clone(),
                    label: label.map(|s| s.to_string()),
                    code: code.to_string(),
                });
            }
        }
        for s in &module.structs {
            for (label, code) in s.doc.examples() {
                examples.push(ExtractedExample {
                    source_module: module.name.clone(),
                    source_item: s.name.clone(),
                    label: label.map(|s| s.to_string()),
                    code: code.to_string(),
                });
            }
        }
    }

    examples
}

// ── Dependency Graph ─────────────────────────────────────────────────

/// A dependency edge between modules.
#[derive(Debug, Clone, PartialEq)]
pub struct DepEdge {
    pub from: String,
    pub to: String,
}

/// Build a module dependency graph from import information.
pub fn build_dep_graph(modules: &[(String, Vec<String>)]) -> Vec<DepEdge> {
    let mut edges = Vec::new();
    for (module_name, imports) in modules {
        for import in imports {
            edges.push(DepEdge {
                from: module_name.clone(),
                to: import.clone(),
            });
        }
    }
    edges
}

/// Generate a Mermaid diagram from dependency edges.
pub fn dep_graph_mermaid(edges: &[DepEdge]) -> String {
    let mut out = String::from("graph TD\n");
    for edge in edges {
        out.push_str(&format!("    {} --> {}\n", edge.from, edge.to));
    }
    out
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Doc Section Display ──────────────────────────────────────────

    #[test]
    fn test_doc_section_display() {
        assert_eq!(
            format!("{}", DocSection::Description("Hello world".into())),
            "Hello world"
        );
        assert_eq!(
            format!("{}", DocSection::Param { name: "x".into(), description: "the value".into() }),
            "@param x — the value"
        );
        assert_eq!(
            format!("{}", DocSection::Returns("the result".into())),
            "@returns the result"
        );
        assert_eq!(
            format!("{}", DocSection::SeeAlso("foo::bar".into())),
            "@see foo::bar"
        );
    }

    // ── Doc Comment Parsing ──────────────────────────────────────────

    #[test]
    fn test_parse_simple_description() {
        let doc = parse_doc_comment("/// This is a simple function.");
        assert_eq!(doc.description(), Some("This is a simple function."));
    }

    #[test]
    fn test_parse_params_and_returns() {
        let raw = "/// Adds two numbers.\n/// @param a the first number\n/// @param b the second number\n/// @returns the sum";
        let doc = parse_doc_comment(raw);
        assert_eq!(doc.description(), Some("Adds two numbers."));
        let params = doc.params();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], ("a", "the first number"));
        assert_eq!(params[1], ("b", "the second number"));
        assert_eq!(doc.returns(), Some("the sum"));
    }

    #[test]
    fn test_parse_example() {
        let raw = "/// A function.\n/// @example\n/// ```\n/// let x = 42\n/// ```";
        let doc = parse_doc_comment(raw);
        let examples = doc.examples();
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].1, "let x = 42");
    }

    #[test]
    fn test_parse_deprecated() {
        let raw = "/// Old function.\n/// @deprecated Use new_function instead.";
        let doc = parse_doc_comment(raw);
        assert!(doc.is_deprecated());
        assert_eq!(doc.deprecation_reason(), Some("Use new_function instead."));
    }

    #[test]
    fn test_parse_see_also() {
        let raw = "/// Some function.\n/// @see foo::bar\n/// @see baz::qux";
        let doc = parse_doc_comment(raw);
        let refs = doc.see_also();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0], "foo::bar");
    }

    #[test]
    fn test_parse_throws() {
        let raw = "/// Dangerous function.\n/// @throws IOError if file not found";
        let doc = parse_doc_comment(raw);
        assert_eq!(doc.section_count(), 2);
    }

    #[test]
    fn test_parse_note_and_warning() {
        let raw = "/// Function.\n/// @note This is important.\n/// @warning Be careful!";
        let doc = parse_doc_comment(raw);
        assert_eq!(doc.section_count(), 3);
    }

    #[test]
    fn test_parse_since() {
        let raw = "/// @since v2.0.0";
        let doc = parse_doc_comment(raw);
        assert_eq!(doc.section_count(), 1);
        assert!(matches!(&doc.sections[0], DocSection::Since(v) if v == "v2.0.0"));
    }

    #[test]
    fn test_empty_doc_comment() {
        let doc = parse_doc_comment("");
        assert!(doc.is_empty());
    }

    // ── DocComment Methods ───────────────────────────────────────────

    #[test]
    fn test_doc_comment_methods() {
        let mut doc = DocComment::new();
        assert!(doc.is_empty());
        doc.add_section(DocSection::Description("hello".into()));
        assert_eq!(doc.section_count(), 1);
        assert!(!doc.is_empty());
    }

    // ── DocFunction ──────────────────────────────────────────────────

    #[test]
    fn test_doc_function_signature() {
        let mut f = DocFunction::new("add");
        f.params = vec![
            DocParam { name: "a".into(), type_name: "i64".into(), description: None, default_value: None },
            DocParam { name: "b".into(), type_name: "i64".into(), description: None, default_value: None },
        ];
        f.return_type = Some("i64".into());
        assert_eq!(f.signature(), "fn add(a: i64, b: i64) -> i64");
    }

    #[test]
    fn test_doc_function_signature_async() {
        let mut f = DocFunction::new("fetch");
        f.is_async = true;
        f.return_type = Some("str".into());
        assert_eq!(f.signature(), "async fn fetch() -> str");
    }

    #[test]
    fn test_doc_function_signature_generic() {
        let mut f = DocFunction::new("identity");
        f.generic_params = vec!["T".into()];
        f.params = vec![
            DocParam { name: "x".into(), type_name: "T".into(), description: None, default_value: None },
        ];
        f.return_type = Some("T".into());
        assert_eq!(f.signature(), "fn identity<T>(x: T) -> T");
    }

    // ── DocModule ────────────────────────────────────────────────────

    #[test]
    fn test_doc_module_item_count() {
        let mut module = DocModule::new("math", "src/math.sl");
        assert_eq!(module.item_count(), 0);
        module.functions.push(DocFunction::new("add"));
        module.structs.push(DocStruct::new("Vector"));
        assert_eq!(module.item_count(), 2);
    }

    #[test]
    fn test_doc_module_find() {
        let mut module = DocModule::new("math", "src/math.sl");
        module.functions.push(DocFunction::new("add"));
        module.structs.push(DocStruct::new("Vector"));
        assert!(module.find_function("add").is_some());
        assert!(module.find_function("sub").is_none());
        assert!(module.find_struct("Vector").is_some());
    }

    // ── Cross-Reference Resolution ───────────────────────────────────

    #[test]
    fn test_ref_resolver_index_module() {
        let mut resolver = RefResolver::new();
        let mut module = DocModule::new("math", "src/math.sl");
        module.functions.push(DocFunction::new("add"));
        module.structs.push(DocStruct::new("Vector"));
        resolver.index_module(&module);

        assert_eq!(resolver.known_count(), 3); // module + function + struct
        assert!(matches!(resolver.resolve("math"), DocRef::Module { .. }));
        assert!(matches!(resolver.resolve("math::add"), DocRef::Function { .. }));
        assert!(matches!(resolver.resolve("math::Vector"), DocRef::Struct { .. }));
    }

    #[test]
    fn test_ref_resolver_unresolved() {
        let resolver = RefResolver::new();
        assert!(matches!(resolver.resolve("foo::bar"), DocRef::Unresolved { .. }));
    }

    #[test]
    fn test_ref_resolver_external() {
        let resolver = RefResolver::new();
        let result = resolver.resolve("https://example.com");
        assert!(matches!(result, DocRef::External { .. }));
    }

    // ── Output Generation ────────────────────────────────────────────

    #[test]
    fn test_generate_markdown() {
        let mut module = DocModule::new("math", "src/math.sl");
        module.doc.add_section(DocSection::Description("Math utilities.".into()));
        let mut f = DocFunction::new("add");
        f.params = vec![
            DocParam { name: "a".into(), type_name: "i64".into(), description: None, default_value: None },
        ];
        f.return_type = Some("i64".into());
        f.doc.add_section(DocSection::Description("Adds one.".into()));
        module.functions.push(f);

        let md = generate_module_docs(&module, OutputFormat::Markdown);
        assert!(md.contains("# Module `math`"));
        assert!(md.contains("Math utilities."));
        assert!(md.contains("## Functions"));
        assert!(md.contains("### `add`"));
    }

    #[test]
    fn test_generate_html() {
        let mut module = DocModule::new("core", "src/core.sl");
        module.functions.push(DocFunction::new("main"));
        let html = generate_module_docs(&module, OutputFormat::Html);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<h1>Module <code>core</code></h1>"));
        assert!(html.contains("<code>main</code>"));
    }

    #[test]
    fn test_generate_plaintext() {
        let mut module = DocModule::new("utils", "src/utils.sl");
        module.functions.push(DocFunction::new("helper"));
        let txt = generate_module_docs(&module, OutputFormat::PlainText);
        assert!(txt.contains("MODULE: utils"));
        assert!(txt.contains("FUNCTIONS:"));
    }

    // ── DocIndex ─────────────────────────────────────────────────────

    #[test]
    fn test_doc_index() {
        let mut index = DocIndex::new();
        let mut m1 = DocModule::new("math", "src/math.sl");
        m1.functions.push(DocFunction::new("add"));
        m1.functions.push(DocFunction::new("sub"));
        let mut m2 = DocModule::new("io", "src/io.sl");
        m2.functions.push(DocFunction::new("read"));

        index.add_module(m1);
        index.add_module(m2);

        assert_eq!(index.module_count(), 2);
        assert_eq!(index.total_items(), 3);
        assert!(index.find_module("math").is_some());
        assert!(index.find_module("io").is_some());
    }

    #[test]
    fn test_doc_index_toc() {
        let mut index = DocIndex::new();
        let mut module = DocModule::new("core", "src/core.sl");
        module.functions.push(DocFunction::new("main"));
        index.add_module(module);
        let toc = index.table_of_contents();
        assert!(toc.contains("# API Documentation"));
        assert!(toc.contains("[core]"));
    }

    #[test]
    fn test_doc_index_generate_all() {
        let mut index = DocIndex::new();
        index.add_module(DocModule::new("a", "a.sl"));
        index.add_module(DocModule::new("b", "b.sl"));
        let results = index.generate_all(OutputFormat::Markdown);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "a");
        assert_eq!(results[1].0, "b");
    }

    // ── Example Extraction ───────────────────────────────────────────

    #[test]
    fn test_extract_examples() {
        let mut index = DocIndex::new();
        let mut module = DocModule::new("demo", "demo.sl");
        let mut f = DocFunction::new("greet");
        f.doc.add_section(DocSection::Example {
            label: Some("basic usage".into()),
            code: "greet(\"world\")".into(),
        });
        module.functions.push(f);
        index.add_module(module);

        let examples = extract_examples(&index);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].source_item, "greet");
        assert_eq!(examples[0].code, "greet(\"world\")");
    }

    // ── Dependency Graph ─────────────────────────────────────────────

    #[test]
    fn test_build_dep_graph() {
        let modules = vec![
            ("parser".to_string(), vec!["lexer".to_string(), "ast".to_string()]),
            ("types".to_string(), vec!["ast".to_string()]),
        ];
        let edges = build_dep_graph(&modules);
        assert_eq!(edges.len(), 3);
    }

    #[test]
    fn test_dep_graph_mermaid() {
        let edges = vec![
            DepEdge { from: "parser".into(), to: "lexer".into() },
            DepEdge { from: "parser".into(), to: "ast".into() },
        ];
        let mermaid = dep_graph_mermaid(&edges);
        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("parser --> lexer"));
        assert!(mermaid.contains("parser --> ast"));
    }

    // ── Visibility Display ───────────────────────────────────────────

    #[test]
    fn test_visibility_display() {
        assert_eq!(format!("{}", Visibility::Public), "pub");
        assert_eq!(format!("{}", Visibility::Private), "priv");
        assert_eq!(format!("{}", Visibility::Internal), "internal");
    }

    // ── DocRef Display ───────────────────────────────────────────────

    #[test]
    fn test_doc_ref_display() {
        assert_eq!(
            format!("{}", DocRef::Function { module: "math".into(), name: "add".into() }),
            "math::add()"
        );
        assert_eq!(
            format!("{}", DocRef::Module { name: "core".into() }),
            "mod core"
        );
        assert_eq!(
            format!("{}", DocRef::Unresolved { target: "foo".into() }),
            "?foo"
        );
    }

    // ── Integration ──────────────────────────────────────────────────

    #[test]
    fn test_integration_full_pipeline() {
        // Parse doc comment
        let raw = "/// Compute the factorial.\n/// @param n the input number\n/// @returns n! as i64\n/// @example\n/// ```\n/// factorial(5)\n/// ```\n/// @since v1.0.0";
        let doc = parse_doc_comment(raw);

        // Build function doc
        let mut f = DocFunction::new("factorial");
        f.params = vec![
            DocParam { name: "n".into(), type_name: "i64".into(), description: Some("the input number".into()), default_value: None },
        ];
        f.return_type = Some("i64".into());
        f.doc = doc;

        // Build module
        let mut module = DocModule::new("math", "src/math.sl");
        module.functions.push(f);

        // Generate docs
        let md = generate_module_docs(&module, OutputFormat::Markdown);
        assert!(md.contains("factorial"));
        assert!(md.contains("n: i64"));

        // Index and resolve
        let mut index = DocIndex::new();
        index.add_module(module);
        let resolved = index.resolver.resolve("math::factorial");
        assert!(matches!(resolved, DocRef::Function { .. }));

        // Extract examples
        let examples = extract_examples(&index);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].code, "factorial(5)");
    }
}
