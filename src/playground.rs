//! Web-based playground for Vitalis.
//!
//! In-browser coding environment:
//! - **WASM compilation**: Compile user code in-browser
//! - **Share-by-URL**: LZ-compressed source in URL fragment
//! - **Example gallery**: Curated examples
//! - **Editor integration**: Syntax highlighting, basic autocomplete
//! - **Output panels**: Console, AST viewer, IR viewer

use std::collections::HashMap;

// ── Playground Model ────────────────────────────────────────────────

/// A playground session state.
#[derive(Debug, Clone)]
pub struct PlaygroundSession {
    pub source: String,
    pub output: PlaygroundOutput,
    pub settings: PlaygroundSettings,
    pub active_tab: OutputTab,
}

/// Playground output.
#[derive(Debug, Clone)]
pub struct PlaygroundOutput {
    pub console: String,
    pub ast: Option<String>,
    pub ir: Option<String>,
    pub type_info: Option<String>,
    pub execution_time_ms: f64,
    pub compile_time_ms: f64,
    pub errors: Vec<PlaygroundError>,
}

/// A playground error.
#[derive(Debug, Clone)]
pub struct PlaygroundError {
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Output tab selection.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputTab {
    Console,
    Ast,
    Ir,
    TypeInfo,
    Assembly,
}

/// Playground settings.
#[derive(Debug, Clone)]
pub struct PlaygroundSettings {
    pub optimization_level: OptLevel,
    pub show_ast: bool,
    pub show_ir: bool,
    pub show_types: bool,
    pub max_execution_ms: u64,
    pub font_size: u32,
    pub theme: PlaygroundTheme,
}

impl Default for PlaygroundSettings {
    fn default() -> Self {
        Self {
            optimization_level: OptLevel::Release,
            show_ast: false,
            show_ir: false,
            show_types: false,
            max_execution_ms: 5000,
            font_size: 14,
            theme: PlaygroundTheme::Dark,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptLevel {
    Debug,
    Release,
    Size,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaygroundTheme {
    Light,
    Dark,
    HighContrast,
}

// ── URL Sharing ─────────────────────────────────────────────────────

/// Encode source code for URL sharing (simplified LZ + base64).
pub fn encode_for_url(source: &str) -> String {
    // Simple RLE compression + base64.
    let compressed = rle_compress(source.as_bytes());
    base64_encode(&compressed)
}

/// Decode source code from URL fragment.
pub fn decode_from_url(encoded: &str) -> Option<String> {
    let decoded = base64_decode(encoded)?;
    let decompressed = rle_decompress(&decoded)?;
    String::from_utf8(decompressed).ok()
}

fn rle_compress(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut count = 1u8;
        while i + (count as usize) < data.len()
            && data[i + (count as usize)] == byte
            && count < 255
        {
            count += 1;
        }
        if count > 3 {
            output.push(0xFF);
            output.push(count);
            output.push(byte);
        } else {
            for _ in 0..count {
                if byte == 0xFF {
                    output.push(0xFF);
                    output.push(1);
                    output.push(0xFF);
                } else {
                    output.push(byte);
                }
            }
        }
        i += count as usize;
    }
    output
}

fn rle_decompress(data: &[u8]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0xFF {
            if i + 2 >= data.len() { return None; }
            let count = data[i + 1] as usize;
            let byte = data[i + 2];
            for _ in 0..count {
                output.push(byte);
            }
            i += 3;
        } else {
            output.push(data[i]);
            i += 1;
        }
    }
    Some(output)
}

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64_encode(data: &[u8]) -> String {
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b = match chunk.len() {
            1 => [chunk[0], 0, 0],
            2 => [chunk[0], chunk[1], 0],
            _ => [chunk[0], chunk[1], chunk[2]],
        };
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        result.push(BASE64_CHARS[((n >> 18) & 63) as usize] as char);
        result.push(BASE64_CHARS[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            result.push(BASE64_CHARS[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            result.push(BASE64_CHARS[(n & 63) as usize] as char);
        }
    }
    result
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut result = Vec::new();
    let chars: Vec<u8> = s.bytes().collect();
    for chunk in chars.chunks(4) {
        let vals: Vec<u32> = chunk.iter().filter_map(|&c| {
            BASE64_CHARS.iter().position(|&b| b == c).map(|p| p as u32)
        }).collect();
        if vals.len() < 2 { continue; }
        let n = match vals.len() {
            2 => (vals[0] << 18) | (vals[1] << 12),
            3 => (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6),
            _ => (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6) | vals[3],
        };
        result.push(((n >> 16) & 0xFF) as u8);
        if vals.len() > 2 { result.push(((n >> 8) & 0xFF) as u8); }
        if vals.len() > 3 { result.push((n & 0xFF) as u8); }
    }
    Some(result)
}

// ── Example Gallery ─────────────────────────────────────────────────

/// A gallery example entry.
#[derive(Debug, Clone)]
pub struct Example {
    pub name: String,
    pub description: String,
    pub source: String,
    pub category: ExampleCategory,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExampleCategory {
    GettingStarted,
    DataStructures,
    Algorithms,
    Concurrency,
    WebDev,
    MachineLearning,
    Systems,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}

/// Example gallery.
pub struct ExampleGallery {
    examples: Vec<Example>,
}

impl ExampleGallery {
    pub fn new() -> Self {
        Self { examples: Vec::new() }
    }

    pub fn add(&mut self, example: Example) {
        self.examples.push(example);
    }

    pub fn by_category(&self, cat: &ExampleCategory) -> Vec<&Example> {
        self.examples.iter().filter(|e| e.category == *cat).collect()
    }

    pub fn by_difficulty(&self, diff: &Difficulty) -> Vec<&Example> {
        self.examples.iter().filter(|e| e.difficulty == *diff).collect()
    }

    pub fn search(&self, query: &str) -> Vec<&Example> {
        let q = query.to_lowercase();
        self.examples.iter()
            .filter(|e| e.name.to_lowercase().contains(&q) || e.description.to_lowercase().contains(&q))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.examples.len()
    }
}

// ── Syntax Highlighting ─────────────────────────────────────────────

/// Token kind for syntax highlighting.
#[derive(Debug, Clone, PartialEq)]
pub enum HighlightToken {
    Keyword,
    Identifier,
    Number,
    StringLiteral,
    Comment,
    Operator,
    Punctuation,
    Type,
    Function,
    Whitespace,
}

/// A highlighted region.
#[derive(Debug, Clone)]
pub struct HighlightRegion {
    pub start: usize,
    pub end: usize,
    pub kind: HighlightToken,
}

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "if", "else", "while", "for", "return", "struct",
    "enum", "impl", "trait", "match", "true", "false", "pub", "mod",
    "use", "async", "await", "spawn", "try", "catch",
];

/// Simple lexer-based syntax highlighting.
pub fn highlight(source: &str) -> Vec<HighlightRegion> {
    let mut regions = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Skip whitespace.
        if bytes[i].is_ascii_whitespace() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }
            regions.push(HighlightRegion { start, end: i, kind: HighlightToken::Whitespace });
            continue;
        }

        // Comments.
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' { i += 1; }
            regions.push(HighlightRegion { start, end: i, kind: HighlightToken::Comment });
            continue;
        }

        // Strings.
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' { i += 1; }
                i += 1;
            }
            if i < bytes.len() { i += 1; }
            regions.push(HighlightRegion { start, end: i, kind: HighlightToken::StringLiteral });
            continue;
        }

        // Numbers.
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') { i += 1; }
            regions.push(HighlightRegion { start, end: i, kind: HighlightToken::Number });
            continue;
        }

        // Identifiers / keywords.
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
            let word = &source[start..i];
            let kind = if KEYWORDS.contains(&word) {
                HighlightToken::Keyword
            } else if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                HighlightToken::Type
            } else {
                HighlightToken::Identifier
            };
            regions.push(HighlightRegion { start, end: i, kind });
            continue;
        }

        // Operators / punctuation.
        let start = i;
        i += 1;
        let kind = match bytes[start] {
            b'+' | b'-' | b'*' | b'/' | b'=' | b'<' | b'>' | b'!' | b'&' | b'|' => HighlightToken::Operator,
            _ => HighlightToken::Punctuation,
        };
        regions.push(HighlightRegion { start, end: i, kind });
    }

    regions
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode_decode() {
        let source = "fn main() { print(42); }";
        let encoded = encode_for_url(source);
        let decoded = decode_from_url(&encoded).unwrap();
        assert_eq!(decoded, source);
    }

    #[test]
    fn test_url_encode_repeated() {
        let source = "aaaaaaaaaa"; // 10 a's — should compress well
        let encoded = encode_for_url(source);
        let decoded = decode_from_url(&encoded).unwrap();
        assert_eq!(decoded, source);
    }

    #[test]
    fn test_highlight_keywords() {
        let regions = highlight("fn main");
        assert!(regions.iter().any(|r| r.kind == HighlightToken::Keyword));
        assert!(regions.iter().any(|r| r.kind == HighlightToken::Identifier));
    }

    #[test]
    fn test_highlight_string() {
        let regions = highlight("\"hello world\"");
        assert!(regions.iter().any(|r| r.kind == HighlightToken::StringLiteral));
    }

    #[test]
    fn test_highlight_number() {
        let regions = highlight("42 3.14");
        let nums: Vec<_> = regions.iter().filter(|r| r.kind == HighlightToken::Number).collect();
        assert_eq!(nums.len(), 2);
    }

    #[test]
    fn test_highlight_comment() {
        let regions = highlight("// this is a comment\nlet x = 1");
        assert!(regions.iter().any(|r| r.kind == HighlightToken::Comment));
    }

    #[test]
    fn test_highlight_type() {
        let regions = highlight("Vec String MyStruct");
        let types: Vec<_> = regions.iter().filter(|r| r.kind == HighlightToken::Type).collect();
        assert_eq!(types.len(), 3);
    }

    #[test]
    fn test_example_gallery() {
        let mut gallery = ExampleGallery::new();
        gallery.add(Example {
            name: "Hello World".into(),
            description: "Print hello".into(),
            source: "print(\"hello\")".into(),
            category: ExampleCategory::GettingStarted,
            difficulty: Difficulty::Beginner,
        });
        gallery.add(Example {
            name: "Sort".into(),
            description: "Quicksort algorithm".into(),
            source: "fn sort(arr) { ... }".into(),
            category: ExampleCategory::Algorithms,
            difficulty: Difficulty::Intermediate,
        });
        assert_eq!(gallery.count(), 2);
        assert_eq!(gallery.by_category(&ExampleCategory::GettingStarted).len(), 1);
        assert_eq!(gallery.by_difficulty(&Difficulty::Beginner).len(), 1);
    }

    #[test]
    fn test_example_search() {
        let mut gallery = ExampleGallery::new();
        gallery.add(Example {
            name: "Hello".into(),
            description: "world".into(),
            source: "".into(),
            category: ExampleCategory::GettingStarted,
            difficulty: Difficulty::Beginner,
        });
        assert_eq!(gallery.search("hello").len(), 1);
        assert_eq!(gallery.search("nonexistent").len(), 0);
    }

    #[test]
    fn test_playground_settings_default() {
        let settings = PlaygroundSettings::default();
        assert_eq!(settings.optimization_level, OptLevel::Release);
        assert_eq!(settings.font_size, 14);
        assert_eq!(settings.theme, PlaygroundTheme::Dark);
    }

    #[test]
    fn test_output_tabs() {
        let tabs = vec![
            OutputTab::Console, OutputTab::Ast, OutputTab::Ir,
            OutputTab::TypeInfo, OutputTab::Assembly,
        ];
        assert_eq!(tabs.len(), 5);
    }

    #[test]
    fn test_error_severity() {
        assert_ne!(ErrorSeverity::Error, ErrorSeverity::Warning);
        assert_ne!(ErrorSeverity::Info, ErrorSeverity::Hint);
    }

    #[test]
    fn test_difficulty_ordering() {
        assert!(Difficulty::Beginner < Difficulty::Intermediate);
        assert!(Difficulty::Intermediate < Difficulty::Advanced);
    }

    #[test]
    fn test_rle_roundtrip_empty() {
        let compressed = rle_compress(b"");
        let decompressed = rle_decompress(&compressed).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_playground_output() {
        let output = PlaygroundOutput {
            console: "42".into(),
            ast: None,
            ir: None,
            type_info: None,
            execution_time_ms: 1.5,
            compile_time_ms: 0.8,
            errors: vec![],
        };
        assert!(output.errors.is_empty());
    }

    #[test]
    fn test_playground_error() {
        let err = PlaygroundError {
            line: 10,
            column: 5,
            message: "type mismatch".into(),
            severity: ErrorSeverity::Error,
        };
        assert_eq!(err.severity, ErrorSeverity::Error);
    }

    #[test]
    fn test_highlight_operators() {
        let regions = highlight("a + b - c");
        let ops: Vec<_> = regions.iter().filter(|r| r.kind == HighlightToken::Operator).collect();
        assert_eq!(ops.len(), 2);
    }
}
