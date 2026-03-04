//! LLM-assisted compiler features for Vitalis.
//!
//! - **Natural-language errors**: Translate compiler errors to plain English
//! - **Fix suggestions**: LLM-powered auto-fix proposals
//! - **Code completion**: Context-aware completion
//! - **Docstring generation**: Auto-generate documentation
//! - **Commit message generation**: From staged diff

use std::collections::HashMap;

// ── Error Explanation ───────────────────────────────────────────────

/// Compiler error severity.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

/// A compiler diagnostic.
#[derive(Debug, Clone)]
pub struct CompilerDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: Severity,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub span_text: String,
}

/// An error explanation with suggested fixes.
#[derive(Debug, Clone)]
pub struct ErrorExplanation {
    pub original: CompilerDiagnostic,
    pub plain_english: String,
    pub why_it_happens: String,
    pub suggested_fixes: Vec<SuggestedFix>,
    pub related_docs: Vec<String>,
    pub confidence: f64,
}

/// A suggested fix.
#[derive(Debug, Clone)]
pub struct SuggestedFix {
    pub description: String,
    pub replacement_text: String,
    pub line: u32,
    pub column: u32,
    pub end_column: u32,
    pub confidence: f64,
    pub is_safe: bool,
}

/// Pattern-based error explainer (no actual LLM needed, pattern matching).
pub struct ErrorExplainer {
    patterns: Vec<ErrorPattern>,
}

/// An error pattern for template matching.
#[derive(Debug, Clone)]
pub struct ErrorPattern {
    pub code_prefix: String,
    pub message_pattern: String,
    pub explanation_template: String,
    pub fix_template: Option<String>,
}

impl ErrorExplainer {
    pub fn new() -> Self {
        let mut explainer = Self { patterns: Vec::new() };
        explainer.register_default_patterns();
        explainer
    }

    fn register_default_patterns(&mut self) {
        self.patterns.push(ErrorPattern {
            code_prefix: "E001".into(),
            message_pattern: "type mismatch".into(),
            explanation_template: "The compiler found a value of one type where it expected another. This often happens when passing arguments to functions or assigning values to variables.".into(),
            fix_template: Some("Cast the value or change the variable type.".into()),
        });
        self.patterns.push(ErrorPattern {
            code_prefix: "E002".into(),
            message_pattern: "undefined variable".into(),
            explanation_template: "You're using a variable name that hasn't been declared yet in this scope. Check for typos in the variable name or make sure you've declared it before using it.".into(),
            fix_template: Some("Declare the variable with 'let' before using it, or fix the spelling.".into()),
        });
        self.patterns.push(ErrorPattern {
            code_prefix: "E003".into(),
            message_pattern: "borrow".into(),
            explanation_template: "Vitalis's ownership system prevents using a value that has been moved or borrowed mutably. Only one mutable borrow or multiple immutable borrows can exist at a time.".into(),
            fix_template: Some("Clone the value, restructure borrows, or use a reference instead of moving.".into()),
        });
        self.patterns.push(ErrorPattern {
            code_prefix: "E004".into(),
            message_pattern: "unused".into(),
            explanation_template: "A variable was declared but never used. This might indicate a bug or leftover code from refactoring.".into(),
            fix_template: Some("Prefix the variable with an underscore (_) to suppress the warning, or remove it.".into()),
        });
        self.patterns.push(ErrorPattern {
            code_prefix: "E005".into(),
            message_pattern: "missing return".into(),
            explanation_template: "The function is expected to return a value, but not all code paths produce a return. Add a return statement to the end of the function or to branches that are missing one.".into(),
            fix_template: Some("Add a return expression at the end of the function.".into()),
        });
    }

    pub fn add_pattern(&mut self, pattern: ErrorPattern) {
        self.patterns.push(pattern);
    }

    /// Explain a diagnostic using pattern matching.
    pub fn explain(&self, diag: &CompilerDiagnostic) -> ErrorExplanation {
        let msg_lower = diag.message.to_lowercase();

        for pattern in &self.patterns {
            if diag.code.starts_with(&pattern.code_prefix) || msg_lower.contains(&pattern.message_pattern.to_lowercase()) {
                let mut fixes = Vec::new();
                if let Some(fix) = &pattern.fix_template {
                    fixes.push(SuggestedFix {
                        description: fix.clone(),
                        replacement_text: String::new(),
                        line: diag.line,
                        column: diag.column,
                        end_column: diag.column + diag.span_text.len() as u32,
                        confidence: 0.7,
                        is_safe: false,
                    });
                }

                return ErrorExplanation {
                    original: diag.clone(),
                    plain_english: pattern.explanation_template.clone(),
                    why_it_happens: format!("This error (code {}) occurs because: {}", diag.code, pattern.explanation_template),
                    suggested_fixes: fixes,
                    related_docs: vec![format!("https://vitalis-lang.org/errors/{}", diag.code)],
                    confidence: 0.8,
                };
            }
        }

        // No matching pattern.
        ErrorExplanation {
            original: diag.clone(),
            plain_english: format!("Compiler error: {}", diag.message),
            why_it_happens: "See the error message for details.".into(),
            suggested_fixes: vec![],
            related_docs: vec![],
            confidence: 0.3,
        }
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

// ── Code Completion ─────────────────────────────────────────────────

/// A completion item.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: String,
    pub documentation: Option<String>,
    pub insert_text: String,
    pub relevance_score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Function,
    Variable,
    Type,
    Keyword,
    Snippet,
    Field,
    Method,
    Module,
}

/// Context-aware completion provider.
pub struct CompletionProvider {
    keywords: Vec<String>,
    builtins: Vec<(String, String)>, // (name, signature)
    recent_symbols: Vec<String>,
}

impl CompletionProvider {
    pub fn new() -> Self {
        Self {
            keywords: vec![
                "fn", "let", "mut", "if", "else", "while", "for", "return",
                "struct", "enum", "impl", "trait", "match", "true", "false",
                "pub", "mod", "use", "async", "await", "spawn",
            ].into_iter().map(String::from).collect(),
            builtins: vec![
                ("print".into(), "fn print(value: any) -> void".into()),
                ("len".into(), "fn len(collection: any) -> i64".into()),
                ("push".into(), "fn push(list: List, value: any) -> void".into()),
                ("map".into(), "fn map(list: List, f: fn) -> List".into()),
                ("filter".into(), "fn filter(list: List, f: fn) -> List".into()),
            ],
            recent_symbols: Vec::new(),
        }
    }

    pub fn complete(&self, prefix: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let p = prefix.to_lowercase();

        // Recent symbols (higher priority).
        for sym in &self.recent_symbols {
            if sym.to_lowercase().starts_with(&p) {
                items.push(CompletionItem {
                    label: sym.clone(), kind: CompletionKind::Variable,
                    detail: "recent".into(), documentation: None,
                    insert_text: sym.clone(), relevance_score: 0.9,
                });
            }
        }

        // Keywords.
        for kw in &self.keywords {
            if kw.starts_with(&p) {
                items.push(CompletionItem {
                    label: kw.clone(), kind: CompletionKind::Keyword,
                    detail: "keyword".into(), documentation: None,
                    insert_text: kw.clone(), relevance_score: 0.5,
                });
            }
        }

        // Builtins.
        for (name, sig) in &self.builtins {
            if name.to_lowercase().starts_with(&p) {
                items.push(CompletionItem {
                    label: name.clone(), kind: CompletionKind::Function,
                    detail: sig.clone(), documentation: Some(format!("Built-in function: {}", name)),
                    insert_text: format!("{}()", name), relevance_score: 0.7,
                });
            }
        }

        items.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        items
    }

    pub fn add_symbol(&mut self, symbol: &str) {
        if !self.recent_symbols.contains(&symbol.to_string()) {
            self.recent_symbols.push(symbol.to_string());
            if self.recent_symbols.len() > 100 {
                self.recent_symbols.remove(0);
            }
        }
    }
}

// ── Docstring Generation ────────────────────────────────────────────

/// Documentation template for a function.
#[derive(Debug, Clone)]
pub struct DocTemplate {
    pub summary: String,
    pub params: Vec<(String, String)>,
    pub returns: Option<String>,
    pub examples: Vec<String>,
    pub throws: Vec<String>,
}

impl DocTemplate {
    pub fn to_doc_comment(&self) -> String {
        let mut doc = String::new();
        doc.push_str(&format!("/// {}\n", self.summary));
        if !self.params.is_empty() {
            doc.push_str("///\n");
            doc.push_str("/// # Parameters\n");
            for (name, desc) in &self.params {
                doc.push_str(&format!("/// - `{}`: {}\n", name, desc));
            }
        }
        if let Some(ret) = &self.returns {
            doc.push_str("///\n");
            doc.push_str(&format!("/// # Returns\n/// {}\n", ret));
        }
        if !self.throws.is_empty() {
            doc.push_str("///\n");
            doc.push_str("/// # Errors\n");
            for err in &self.throws {
                doc.push_str(&format!("/// - {}\n", err));
            }
        }
        if !self.examples.is_empty() {
            doc.push_str("///\n");
            doc.push_str("/// # Examples\n///\n");
            for example in &self.examples {
                doc.push_str(&format!("/// ```\n/// {}\n/// ```\n", example));
            }
        }
        doc
    }
}

/// Generate a doc template from function signature.
pub fn generate_doc_template(name: &str, params: &[(String, String)], return_type: Option<&str>) -> DocTemplate {
    let summary = format!("{} performs its operation.", name_to_summary(name));
    let doc_params = params.iter().map(|(n, t)| {
        (n.clone(), format!("A {} value", t))
    }).collect();
    let returns = return_type.map(|t| format!("A {} result", t));

    DocTemplate {
        summary,
        params: doc_params,
        returns,
        examples: vec![],
        throws: vec![],
    }
}

fn name_to_summary(name: &str) -> String {
    // Convert snake_case to human-readable.
    name.split('_')
        .enumerate()
        .map(|(i, word)| {
            if i == 0 {
                let mut c = word.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().chain(c).collect(),
                }
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── Commit Message Generation ───────────────────────────────────────

/// A file change in a diff.
#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub file: String,
    pub change_type: ChangeType,
    pub additions: u32,
    pub deletions: u32,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Generate a commit message from a diff.
pub fn generate_commit_message(entries: &[DiffEntry]) -> String {
    if entries.is_empty() {
        return "chore: empty commit".to_string();
    }

    let total_adds: u32 = entries.iter().map(|e| e.additions).sum();
    let total_dels: u32 = entries.iter().map(|e| e.deletions).sum();
    let file_count = entries.len();

    // Determine prefix.
    let all_added = entries.iter().all(|e| e.change_type == ChangeType::Added);
    let all_deleted = entries.iter().all(|e| e.change_type == ChangeType::Deleted);
    let has_test = entries.iter().any(|e| e.file.contains("test"));

    let prefix = if all_added { "feat" }
    else if all_deleted { "refactor" }
    else if has_test { "test" }
    else if total_adds > total_dels * 3 { "feat" }
    else if total_dels > total_adds * 3 { "refactor" }
    else { "chore" };

    let scope = if file_count == 1 {
        let file = &entries[0].file;
        file.rsplit('/').next().unwrap_or(file)
            .split('.').next().unwrap_or("unknown").to_string()
    } else {
        format!("{} files", file_count)
    };

    let summary = if all_added {
        format!("add {}", scope)
    } else if all_deleted {
        format!("remove {}", scope)
    } else {
        format!("update {} (+{} -{} lines)", scope, total_adds, total_dels)
    };

    format!("{}({}): {}", prefix, scope, summary)
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_explainer_type_mismatch() {
        let explainer = ErrorExplainer::new();
        let diag = CompilerDiagnostic {
            code: "E001".into(), message: "type mismatch: expected i32, got str".into(),
            severity: Severity::Error, file: "main.sl".into(),
            line: 10, column: 5, span_text: "x".into(),
        };
        let explanation = explainer.explain(&diag);
        assert!(explanation.plain_english.contains("type"));
        assert!(explanation.confidence > 0.5);
    }

    #[test]
    fn test_error_explainer_undefined() {
        let explainer = ErrorExplainer::new();
        let diag = CompilerDiagnostic {
            code: "E002".into(), message: "undefined variable 'foo'".into(),
            severity: Severity::Error, file: "main.sl".into(),
            line: 5, column: 1, span_text: "foo".into(),
        };
        let explanation = explainer.explain(&diag);
        assert!(!explanation.suggested_fixes.is_empty());
    }

    #[test]
    fn test_error_explainer_unknown() {
        let explainer = ErrorExplainer::new();
        let diag = CompilerDiagnostic {
            code: "E999".into(), message: "something unusual".into(),
            severity: Severity::Error, file: "main.sl".into(),
            line: 1, column: 1, span_text: "?".into(),
        };
        let explanation = explainer.explain(&diag);
        assert!(explanation.confidence < 0.5);
    }

    #[test]
    fn test_completion_keywords() {
        let provider = CompletionProvider::new();
        let items = provider.complete("fn");
        assert!(items.iter().any(|i| i.label == "fn"));
    }

    #[test]
    fn test_completion_builtins() {
        let provider = CompletionProvider::new();
        let items = provider.complete("pr");
        assert!(items.iter().any(|i| i.label == "print"));
    }

    #[test]
    fn test_completion_recent() {
        let mut provider = CompletionProvider::new();
        provider.add_symbol("my_variable");
        let items = provider.complete("my");
        assert!(items.iter().any(|i| i.label == "my_variable"));
        // Recent should have higher relevance.
        assert!(items[0].relevance_score >= 0.9);
    }

    #[test]
    fn test_doc_template() {
        let template = generate_doc_template(
            "calculate_area",
            &[("width".into(), "f64".into()), ("height".into(), "f64".into())],
            Some("f64"),
        );
        let doc = template.to_doc_comment();
        assert!(doc.contains("Calculate area"));
        assert!(doc.contains("width"));
        assert!(doc.contains("Returns"));
    }

    #[test]
    fn test_name_to_summary() {
        assert_eq!(name_to_summary("hello_world"), "Hello world");
        assert_eq!(name_to_summary("calculate"), "Calculate");
    }

    #[test]
    fn test_commit_message_added() {
        let entries = vec![DiffEntry {
            file: "src/new_feature.rs".into(),
            change_type: ChangeType::Added,
            additions: 100, deletions: 0, summary: "new module".into(),
        }];
        let msg = generate_commit_message(&entries);
        assert!(msg.contains("feat"));
    }

    #[test]
    fn test_commit_message_multi_file() {
        let entries = vec![
            DiffEntry { file: "src/a.rs".into(), change_type: ChangeType::Modified, additions: 10, deletions: 5, summary: "".into() },
            DiffEntry { file: "src/b.rs".into(), change_type: ChangeType::Modified, additions: 20, deletions: 3, summary: "".into() },
        ];
        let msg = generate_commit_message(&entries);
        assert!(msg.contains("2 files"));
    }

    #[test]
    fn test_completion_empty_prefix() {
        let provider = CompletionProvider::new();
        let items = provider.complete("");
        // Should return all keywords and builtins.
        assert!(items.len() > 10);
    }

    #[test]
    fn test_explainer_add_pattern() {
        let mut explainer = ErrorExplainer::new();
        let initial = explainer.pattern_count();
        explainer.add_pattern(ErrorPattern {
            code_prefix: "E100".into(),
            message_pattern: "custom error".into(),
            explanation_template: "Custom!".into(),
            fix_template: None,
        });
        assert_eq!(explainer.pattern_count(), initial + 1);
    }

    #[test]
    fn test_suggested_fix() {
        let fix = SuggestedFix {
            description: "add semicolon".into(),
            replacement_text: ";".into(),
            line: 5, column: 20, end_column: 20,
            confidence: 0.95, is_safe: true,
        };
        assert!(fix.is_safe);
        assert!(fix.confidence > 0.9);
    }

    #[test]
    fn test_severity_variants() {
        assert_ne!(Severity::Error, Severity::Warning);
        assert_ne!(Severity::Note, Severity::Help);
    }

    #[test]
    fn test_commit_empty() {
        let msg = generate_commit_message(&[]);
        assert_eq!(msg, "chore: empty commit");
    }

    #[test]
    fn test_doc_template_no_return() {
        let template = generate_doc_template("do_something", &[], None);
        let doc = template.to_doc_comment();
        assert!(!doc.contains("Returns"));
    }

    #[test]
    fn test_completion_kind() {
        assert_ne!(CompletionKind::Function, CompletionKind::Variable);
        assert_ne!(CompletionKind::Keyword, CompletionKind::Snippet);
    }

    #[test]
    fn test_change_type() {
        assert_ne!(ChangeType::Added, ChangeType::Deleted);
        assert_ne!(ChangeType::Modified, ChangeType::Renamed);
    }
}
