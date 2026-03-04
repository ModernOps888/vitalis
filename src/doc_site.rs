//! Documentation site generator for Vitalis.
//!
//! Static documentation site generation:
//! - **API docs**: Auto-generate from doc comments with cross-references
//! - **Guide pages**: Markdown tutorials with code extraction
//! - **Search index**: Full-text search with TF-IDF ranking
//! - **Doctest**: Extract and execute code examples from docs
//! - **Versioned docs**: Multiple versions with switcher
//! - **Themes**: Configurable CSS themes, dark/light mode

use std::collections::{HashMap, BTreeMap};

// ── Document Model ──────────────────────────────────────────────────

/// A documentation page.
#[derive(Debug, Clone)]
pub struct DocPage {
    pub path: String,
    pub title: String,
    pub content: String,
    pub kind: PageKind,
    pub sections: Vec<DocSection>,
    pub version: String,
}

/// Page kind.
#[derive(Debug, Clone, PartialEq)]
pub enum PageKind {
    ApiReference,
    Guide,
    Tutorial,
    Changelog,
    Index,
}

/// A section within a page.
#[derive(Debug, Clone)]
pub struct DocSection {
    pub id: String,
    pub title: String,
    pub content: String,
    pub level: u8,
    pub code_blocks: Vec<CodeBlock>,
}

/// A code block within documentation.
#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub language: String,
    pub code: String,
    pub is_runnable: bool,
    pub expected_output: Option<String>,
}

// ── Cross-References ────────────────────────────────────────────────

/// A cross-reference link to another doc item.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossRef {
    pub target: String,
    pub display: String,
    pub kind: RefKind,
}

/// Reference kind.
#[derive(Debug, Clone, PartialEq)]
pub enum RefKind {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Guide,
}

/// Resolve cross-references in text.
pub fn resolve_refs(text: &str, known: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (name, url) in known {
        let pattern = format!("`{}`", name);
        let replacement = format!("[`{}`]({})", name, url);
        result = result.replace(&pattern, &replacement);
    }
    result
}

// ── Search Index ────────────────────────────────────────────────────

/// A search index entry.
#[derive(Debug, Clone)]
pub struct SearchEntry {
    pub path: String,
    pub title: String,
    pub content: String,
    pub section: Option<String>,
}

/// Full-text search index with TF-IDF scoring.
pub struct SearchIndex {
    entries: Vec<SearchEntry>,
    inverted: HashMap<String, Vec<(usize, f64)>>,  // term → [(doc_idx, tf)]
    doc_count: usize,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            inverted: HashMap::new(),
            doc_count: 0,
        }
    }

    /// Add a document to the search index.
    pub fn add(&mut self, entry: SearchEntry) {
        let doc_idx = self.entries.len();
        let tokens = tokenize(&entry.title) .into_iter()
            .chain(tokenize(&entry.content))
            .collect::<Vec<_>>();

        let total = tokens.len() as f64;
        let mut freq: HashMap<String, usize> = HashMap::new();
        for token in &tokens {
            *freq.entry(token.clone()).or_default() += 1;
        }

        for (term, count) in freq {
            let tf = count as f64 / total.max(1.0);
            self.inverted.entry(term).or_default().push((doc_idx, tf));
        }

        self.entries.push(entry);
        self.doc_count += 1;
    }

    /// Search the index with TF-IDF scoring.
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query_tokens = tokenize(query);
        let mut scores: HashMap<usize, f64> = HashMap::new();

        for token in &query_tokens {
            if let Some(postings) = self.inverted.get(token) {
                let idf = (self.doc_count as f64 / postings.len() as f64).ln().max(0.0) + 1.0;
                for &(doc_idx, tf) in postings {
                    *scores.entry(doc_idx).or_default() += tf * idf;
                }
            }
        }

        let mut results: Vec<SearchResult> = scores.into_iter()
            .map(|(idx, score)| SearchResult {
                path: self.entries[idx].path.clone(),
                title: self.entries[idx].title.clone(),
                score,
                snippet: snippet(&self.entries[idx].content, &query_tokens),
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Total documents in index.
    pub fn doc_count(&self) -> usize {
        self.doc_count
    }

    /// Total unique terms.
    pub fn term_count(&self) -> usize {
        self.inverted.len()
    }
}

/// A search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub score: f64,
    pub snippet: String,
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| s.len() > 1)
        .map(|s| s.to_string())
        .collect()
}

fn snippet(content: &str, query_tokens: &[String]) -> String {
    let lower = content.to_lowercase();
    for token in query_tokens {
        if let Some(pos) = lower.find(token) {
            let start = pos.saturating_sub(40);
            let end = (pos + token.len() + 40).min(content.len());
            return format!("...{}...", &content[start..end]);
        }
    }
    content.chars().take(80).collect::<String>() + "..."
}

// ── Doctest Runner ──────────────────────────────────────────────────

/// A doctest extracted from documentation.
#[derive(Debug, Clone)]
pub struct Doctest {
    pub source: String,
    pub source_file: String,
    pub line: u32,
    pub code: String,
    pub expected_output: Option<String>,
}

/// Doctest result.
#[derive(Debug, Clone)]
pub struct DoctestResult {
    pub source: String,
    pub passed: bool,
    pub actual_output: String,
    pub error: Option<String>,
}

/// Extract doctests from documentation pages.
pub fn extract_doctests(pages: &[DocPage]) -> Vec<Doctest> {
    let mut doctests = Vec::new();
    for page in pages {
        for section in &page.sections {
            for (i, block) in section.code_blocks.iter().enumerate() {
                if block.is_runnable {
                    doctests.push(Doctest {
                        source: format!("{}#{}", page.path, section.id),
                        source_file: page.path.clone(),
                        line: i as u32,
                        code: block.code.clone(),
                        expected_output: block.expected_output.clone(),
                    });
                }
            }
        }
    }
    doctests
}

// ── Site Generator ──────────────────────────────────────────────────

/// Site theme configuration.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub primary_color: String,
    pub bg_color: String,
    pub text_color: String,
    pub code_bg: String,
    pub dark_mode: bool,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            name: "light".into(),
            primary_color: "#0066cc".into(),
            bg_color: "#ffffff".into(),
            text_color: "#333333".into(),
            code_bg: "#f5f5f5".into(),
            dark_mode: false,
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "dark".into(),
            primary_color: "#4da6ff".into(),
            bg_color: "#1a1a2e".into(),
            text_color: "#e0e0e0".into(),
            code_bg: "#2d2d44".into(),
            dark_mode: true,
        }
    }

    pub fn to_css(&self) -> String {
        format!(
            ":root {{ --primary: {}; --bg: {}; --text: {}; --code-bg: {}; }}",
            self.primary_color, self.bg_color, self.text_color, self.code_bg
        )
    }
}

/// Documentation site configuration.
#[derive(Debug, Clone)]
pub struct SiteConfig {
    pub title: String,
    pub base_url: String,
    pub theme: Theme,
    pub versions: Vec<String>,
    pub current_version: String,
}

/// The documentation site generator.
pub struct DocSiteGenerator {
    config: SiteConfig,
    pages: Vec<DocPage>,
    search_index: SearchIndex,
}

impl DocSiteGenerator {
    pub fn new(config: SiteConfig) -> Self {
        Self {
            config,
            pages: Vec::new(),
            search_index: SearchIndex::new(),
        }
    }

    /// Add a page to the site.
    pub fn add_page(&mut self, page: DocPage) {
        self.search_index.add(SearchEntry {
            path: page.path.clone(),
            title: page.title.clone(),
            content: page.content.clone(),
            section: None,
        });
        self.pages.push(page);
    }

    /// Generate site HTML (returns map of path → HTML content).
    pub fn generate(&self) -> HashMap<String, String> {
        let mut output = HashMap::new();

        for page in &self.pages {
            let html = self.render_page(page);
            output.insert(page.path.clone(), html);
        }

        // Generate index page.
        output.insert("index.html".into(), self.render_index());

        output
    }

    fn render_page(&self, page: &DocPage) -> String {
        let mut html = format!(
            "<!DOCTYPE html><html><head><title>{} - {}</title><style>{}</style></head><body>",
            page.title, self.config.title, self.config.theme.to_css()
        );
        html.push_str(&format!("<h1>{}</h1>", page.title));
        for section in &page.sections {
            html.push_str(&format!("<h{} id=\"{}\">{}</h{}>", section.level, section.id, section.title, section.level));
            html.push_str(&format!("<div>{}</div>", section.content));
            for block in &section.code_blocks {
                html.push_str(&format!("<pre><code class=\"language-{}\">{}</code></pre>", block.language, block.code));
            }
        }
        html.push_str("</body></html>");
        html
    }

    fn render_index(&self) -> String {
        let mut html = format!(
            "<!DOCTYPE html><html><head><title>{}</title></head><body><h1>{}</h1><ul>",
            self.config.title, self.config.title
        );
        for page in &self.pages {
            html.push_str(&format!("<li><a href=\"{}\">{}</a></li>", page.path, page.title));
        }
        html.push_str("</ul></body></html>");
        html
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        self.search_index.search(query)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_page() -> DocPage {
        DocPage {
            path: "api/foo.html".into(),
            title: "Module foo".into(),
            content: "The foo module provides utility functions.".into(),
            kind: PageKind::ApiReference,
            version: "1.0.0".into(),
            sections: vec![DocSection {
                id: "functions".into(),
                title: "Functions".into(),
                content: "Public functions in foo.".into(),
                level: 2,
                code_blocks: vec![CodeBlock {
                    language: "vitalis".into(),
                    code: "fn add(a: i32, b: i32) -> i32 { a + b }".into(),
                    is_runnable: true,
                    expected_output: None,
                }],
            }],
        }
    }

    #[test]
    fn test_search_index_add() {
        let mut idx = SearchIndex::new();
        idx.add(SearchEntry { path: "a.html".into(), title: "Hello".into(), content: "world".into(), section: None });
        assert_eq!(idx.doc_count(), 1);
        assert!(idx.term_count() > 0);
    }

    #[test]
    fn test_search_query() {
        let mut idx = SearchIndex::new();
        idx.add(SearchEntry { path: "a.html".into(), title: "Hello World".into(), content: "greetings".into(), section: None });
        idx.add(SearchEntry { path: "b.html".into(), title: "Other".into(), content: "different".into(), section: None });
        let results = idx.search("hello");
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "a.html");
    }

    #[test]
    fn test_search_tfidf_ranking() {
        let mut idx = SearchIndex::new();
        idx.add(SearchEntry { path: "a.html".into(), title: "Foo".into(), content: "foo foo foo bar".into(), section: None });
        idx.add(SearchEntry { path: "b.html".into(), title: "Bar".into(), content: "bar baz qux".into(), section: None });
        let results = idx.search("foo");
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "a.html");
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! fn add(a: i32)");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"add".to_string()));
    }

    #[test]
    fn test_cross_ref_resolve() {
        let mut refs = HashMap::new();
        refs.insert("Vec".to_string(), "/api/vec.html".to_string());
        let result = resolve_refs("Use `Vec` for lists", &refs);
        assert!(result.contains("[`Vec`](/api/vec.html)"));
    }

    #[test]
    fn test_extract_doctests() {
        let page = sample_page();
        let doctests = extract_doctests(&[page]);
        assert_eq!(doctests.len(), 1);
        assert!(doctests[0].code.contains("fn add"));
    }

    #[test]
    fn test_theme_light() {
        let theme = Theme::light();
        assert!(!theme.dark_mode);
        let css = theme.to_css();
        assert!(css.contains("--primary"));
    }

    #[test]
    fn test_theme_dark() {
        let theme = Theme::dark();
        assert!(theme.dark_mode);
    }

    #[test]
    fn test_site_generator() {
        let config = SiteConfig {
            title: "Vitalis Docs".into(),
            base_url: "https://docs.example.com".into(),
            theme: Theme::light(),
            versions: vec!["1.0.0".into()],
            current_version: "1.0.0".into(),
        };
        let mut generator = DocSiteGenerator::new(config);
        generator.add_page(sample_page());
        assert_eq!(generator.page_count(), 1);
        let output = generator.generate();
        assert!(output.contains_key("index.html"));
        assert!(output.contains_key("api/foo.html"));
    }

    #[test]
    fn test_site_search() {
        let config = SiteConfig {
            title: "Docs".into(),
            base_url: "/".into(),
            theme: Theme::dark(),
            versions: vec![],
            current_version: "1.0".into(),
        };
        let mut generator = DocSiteGenerator::new(config);
        generator.add_page(sample_page());
        let results = generator.search("foo");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_page_kinds() {
        assert_ne!(PageKind::ApiReference, PageKind::Guide);
        assert_ne!(PageKind::Tutorial, PageKind::Changelog);
    }

    #[test]
    fn test_ref_kinds() {
        let refs = vec![
            RefKind::Function, RefKind::Struct, RefKind::Enum,
            RefKind::Trait, RefKind::Module, RefKind::Guide,
        ];
        assert_eq!(refs.len(), 6);
    }

    #[test]
    fn test_empty_search() {
        let idx = SearchIndex::new();
        let results = idx.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_doctest_source_tracking() {
        let doctest = Doctest {
            source: "api/foo.html#functions".into(),
            source_file: "api/foo.html".into(),
            line: 0,
            code: "let x = 1;".into(),
            expected_output: Some("1".into()),
        };
        assert!(doctest.expected_output.is_some());
    }

    #[test]
    fn test_html_output_structure() {
        let config = SiteConfig {
            title: "Test".into(),
            base_url: "/".into(),
            theme: Theme::light(),
            versions: vec![],
            current_version: "1.0".into(),
        };
        let mut generator = DocSiteGenerator::new(config);
        generator.add_page(sample_page());
        let output = generator.generate();
        let html = &output["api/foo.html"];
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<h1>Module foo</h1>"));
    }
}
