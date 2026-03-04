//! Error recovery for Vitalis parser and type checker.
//!
//! - **Parser recovery**: Synchronize after syntax errors
//! - **Type error repair**: Suggest type coercions and fixes
//! - **Cascading suppression**: Prevent error floods from one root cause
//! - **Edit distance suggestions**: Did-you-mean for identifiers
//! - **Error budget**: Limit total errors reported per compilation

use std::collections::HashMap;

// ── Edit Distance ───────────────────────────────────────────────────

/// Compute Levenshtein edit distance between two strings.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost)
                .min(prev[j + 1] + 1)
                .min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

/// Damerau-Levenshtein distance (includes transpositions).
pub fn damerau_levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];
    for i in 0..=a_len { matrix[i][0] = i; }
    for j in 0..=b_len { matrix[0][j] = j; }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);

            if i > 1 && j > 1
                && a_chars[i - 1] == b_chars[j - 2]
                && a_chars[i - 2] == b_chars[j - 1]
            {
                matrix[i][j] = matrix[i][j].min(matrix[i - 2][j - 2] + cost);
            }
        }
    }

    matrix[a_len][b_len]
}

/// Find best matches from a list of candidates.
pub fn find_similar(target: &str, candidates: &[&str], max_distance: usize) -> Vec<(String, usize)> {
    let mut matches: Vec<(String, usize)> = candidates.iter()
        .map(|c| (c.to_string(), damerau_levenshtein(target, c)))
        .filter(|(_, d)| *d <= max_distance && *d > 0)
        .collect();
    matches.sort_by_key(|(_, d)| *d);
    matches
}

// ── Parser Recovery ─────────────────────────────────────────────────

/// Recovery strategy.
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    /// Skip tokens until a synchronization point.
    SkipUntilSync,
    /// Insert a missing token.
    InsertToken(String),
    /// Delete the unexpected token.
    DeleteToken,
    /// Replace the token with the expected one.
    ReplaceToken(String),
    /// Wrap in a construct (e.g., missing braces).
    WrapConstruct(String, String),
}

/// Synchronization tokens for parser recovery.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncPoint {
    Semicolon,
    CloseBrace,
    CloseParen,
    Keyword(String),
    EndOfFile,
}

/// A recovery action taken by the parser.
#[derive(Debug, Clone)]
pub struct RecoveryAction {
    pub strategy: RecoveryStrategy,
    pub position: usize,
    pub message: String,
    pub tokens_skipped: u32,
}

/// Parser recovery engine.
pub struct ParserRecovery {
    sync_points: Vec<SyncPoint>,
    max_skip: u32,
    recovery_log: Vec<RecoveryAction>,
}

impl ParserRecovery {
    pub fn new() -> Self {
        Self {
            sync_points: vec![
                SyncPoint::Semicolon,
                SyncPoint::CloseBrace,
                SyncPoint::Keyword("fn".into()),
                SyncPoint::Keyword("struct".into()),
                SyncPoint::Keyword("enum".into()),
                SyncPoint::Keyword("let".into()),
                SyncPoint::EndOfFile,
            ],
            max_skip: 50,
            recovery_log: Vec::new(),
        }
    }

    /// Choose a recovery strategy for a given error.
    pub fn choose_strategy(&self, expected: &str, found: &str) -> RecoveryStrategy {
        // Missing semicolon → insert.
        if expected == ";" {
            return RecoveryStrategy::InsertToken(";".into());
        }
        // Missing closing delimiters → insert.
        if expected == "}" || expected == ")" || expected == "]" {
            return RecoveryStrategy::InsertToken(expected.into());
        }
        // Unexpected token that's close to expected → replace.
        let dist = levenshtein(expected, found);
        if dist <= 2 && !expected.is_empty() {
            return RecoveryStrategy::ReplaceToken(expected.into());
        }
        // Default: skip until sync.
        RecoveryStrategy::SkipUntilSync
    }

    /// Record a recovery action.
    pub fn record_recovery(&mut self, action: RecoveryAction) {
        self.recovery_log.push(action);
    }

    pub fn recovery_count(&self) -> usize {
        self.recovery_log.len()
    }

    pub fn log(&self) -> &[RecoveryAction] {
        &self.recovery_log
    }

    pub fn clear_log(&mut self) {
        self.recovery_log.clear();
    }
}

// ── Type Error Repair ───────────────────────────────────────────────

/// Type coercion suggestion.
#[derive(Debug, Clone)]
pub struct TypeCoercion {
    pub from_type: String,
    pub to_type: String,
    pub method: CoercionMethod,
    pub is_lossless: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoercionMethod {
    /// Implicit widening (e.g., i32 → i64).
    Widening,
    /// Explicit cast (e.g., f64 → i32).
    ExplicitCast,
    /// Parse from string.
    Parse,
    /// ToString conversion.
    Display,
    /// Dereference.
    Deref,
    /// Borrow.
    Borrow,
}

/// Type repair engine.
pub struct TypeRepair {
    coercion_rules: Vec<TypeCoercion>,
}

impl TypeRepair {
    pub fn new() -> Self {
        let mut repair = Self { coercion_rules: Vec::new() };
        repair.register_defaults();
        repair
    }

    fn register_defaults(&mut self) {
        self.coercion_rules.push(TypeCoercion {
            from_type: "i32".into(), to_type: "i64".into(),
            method: CoercionMethod::Widening, is_lossless: true,
        });
        self.coercion_rules.push(TypeCoercion {
            from_type: "i32".into(), to_type: "f64".into(),
            method: CoercionMethod::Widening, is_lossless: true,
        });
        self.coercion_rules.push(TypeCoercion {
            from_type: "f64".into(), to_type: "i32".into(),
            method: CoercionMethod::ExplicitCast, is_lossless: false,
        });
        self.coercion_rules.push(TypeCoercion {
            from_type: "i64".into(), to_type: "i32".into(),
            method: CoercionMethod::ExplicitCast, is_lossless: false,
        });
        self.coercion_rules.push(TypeCoercion {
            from_type: "str".into(), to_type: "i32".into(),
            method: CoercionMethod::Parse, is_lossless: false,
        });
        self.coercion_rules.push(TypeCoercion {
            from_type: "i32".into(), to_type: "str".into(),
            method: CoercionMethod::Display, is_lossless: true,
        });
    }

    /// Suggest a coercion from one type to another.
    pub fn suggest_coercion(&self, from: &str, to: &str) -> Option<&TypeCoercion> {
        self.coercion_rules.iter().find(|c| c.from_type == from && c.to_type == to)
    }

    pub fn add_rule(&mut self, rule: TypeCoercion) {
        self.coercion_rules.push(rule);
    }

    pub fn lossless_coercions(&self) -> Vec<&TypeCoercion> {
        self.coercion_rules.iter().filter(|c| c.is_lossless).collect()
    }
}

// ── Cascading Error Suppression ─────────────────────────────────────

/// An error with suppression tracking.
#[derive(Debug, Clone)]
pub struct TrackedError {
    pub id: u32,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub is_root_cause: bool,
    pub caused_by: Option<u32>, // ID of root cause
    pub suppressed: bool,
}

/// Error cascade suppressor.
pub struct CascadeSuppressor {
    errors: Vec<TrackedError>,
    next_id: u32,
    suppression_radius: u32, // Lines around root cause to suppress
    max_cascade_depth: u32,
}

impl CascadeSuppressor {
    pub fn new(suppression_radius: u32, max_cascade_depth: u32) -> Self {
        Self {
            errors: Vec::new(), next_id: 0,
            suppression_radius, max_cascade_depth,
        }
    }

    pub fn add_error(&mut self, message: &str, file: &str, line: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        // Check if this might be caused by an existing root error.
        let caused_by = self.errors.iter()
            .filter(|e| e.is_root_cause && e.file == file && !e.suppressed)
            .find(|e| line.abs_diff(e.line) <= self.suppression_radius)
            .map(|e| e.id);

        let is_root = caused_by.is_none();
        let suppressed = caused_by.is_some();

        self.errors.push(TrackedError {
            id, message: message.to_string(), file: file.to_string(),
            line, is_root_cause: is_root, caused_by, suppressed,
        });

        id
    }

    pub fn visible_errors(&self) -> Vec<&TrackedError> {
        self.errors.iter().filter(|e| !e.suppressed).collect()
    }

    pub fn all_errors(&self) -> &[TrackedError] {
        &self.errors
    }

    pub fn root_cause_count(&self) -> usize {
        self.errors.iter().filter(|e| e.is_root_cause).count()
    }

    pub fn suppressed_count(&self) -> usize {
        self.errors.iter().filter(|e| e.suppressed).count()
    }

    pub fn total_count(&self) -> usize {
        self.errors.len()
    }
}

// ── Error Budget ────────────────────────────────────────────────────

/// Error budget controller.
pub struct ErrorBudget {
    max_errors: u32,
    max_warnings: u32,
    error_count: u32,
    warning_count: u32,
    treat_warnings_as_errors: bool,
}

impl ErrorBudget {
    pub fn new(max_errors: u32, max_warnings: u32) -> Self {
        Self {
            max_errors, max_warnings,
            error_count: 0, warning_count: 0,
            treat_warnings_as_errors: false,
        }
    }

    pub fn strict(max: u32) -> Self {
        Self {
            max_errors: max, max_warnings: max,
            error_count: 0, warning_count: 0,
            treat_warnings_as_errors: true,
        }
    }

    pub fn record_error(&mut self) -> bool {
        self.error_count += 1;
        self.error_count <= self.max_errors
    }

    pub fn record_warning(&mut self) -> bool {
        if self.treat_warnings_as_errors {
            self.error_count += 1;
            return self.error_count <= self.max_errors;
        }
        self.warning_count += 1;
        self.warning_count <= self.max_warnings
    }

    pub fn is_exhausted(&self) -> bool {
        self.error_count > self.max_errors
    }

    pub fn remaining_errors(&self) -> u32 {
        self.max_errors.saturating_sub(self.error_count)
    }

    pub fn errors(&self) -> u32 {
        self.error_count
    }

    pub fn warnings(&self) -> u32 {
        self.warning_count
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_identical() {
        assert_eq!(levenshtein("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_one_edit() {
        assert_eq!(levenshtein("cat", "car"), 1);
        assert_eq!(levenshtein("cat", "cats"), 1);
        assert_eq!(levenshtein("cat", "at"), 1);
    }

    #[test]
    fn test_levenshtein_empty() {
        assert_eq!(levenshtein("", "hello"), 5);
        assert_eq!(levenshtein("hello", ""), 5);
    }

    #[test]
    fn test_damerau_transposition() {
        // "ab" → "ba" = 1 transposition.
        assert_eq!(damerau_levenshtein("ab", "ba"), 1);
    }

    #[test]
    fn test_find_similar() {
        let candidates = vec!["print", "println", "printf", "sprint", "paint"];
        let results = find_similar("prnt", &candidates, 2);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "print");
    }

    #[test]
    fn test_recovery_insert_semicolon() {
        let recovery = ParserRecovery::new();
        let strategy = recovery.choose_strategy(";", "fn");
        assert_eq!(strategy, RecoveryStrategy::InsertToken(";".into()));
    }

    #[test]
    fn test_recovery_insert_brace() {
        let recovery = ParserRecovery::new();
        let strategy = recovery.choose_strategy("}", "let");
        assert_eq!(strategy, RecoveryStrategy::InsertToken("}".into()));
    }

    #[test]
    fn test_recovery_skip() {
        let recovery = ParserRecovery::new();
        let strategy = recovery.choose_strategy("expr", "@@@@");
        assert_eq!(strategy, RecoveryStrategy::SkipUntilSync);
    }

    #[test]
    fn test_recovery_log() {
        let mut recovery = ParserRecovery::new();
        recovery.record_recovery(RecoveryAction {
            strategy: RecoveryStrategy::InsertToken(";".into()),
            position: 42, message: "inserted semicolon".into(),
            tokens_skipped: 0,
        });
        assert_eq!(recovery.recovery_count(), 1);
    }

    #[test]
    fn test_type_coercion_widening() {
        let repair = TypeRepair::new();
        let coercion = repair.suggest_coercion("i32", "i64").unwrap();
        assert_eq!(coercion.method, CoercionMethod::Widening);
        assert!(coercion.is_lossless);
    }

    #[test]
    fn test_type_coercion_lossy() {
        let repair = TypeRepair::new();
        let coercion = repair.suggest_coercion("f64", "i32").unwrap();
        assert!(!coercion.is_lossless);
    }

    #[test]
    fn test_type_coercion_not_found() {
        let repair = TypeRepair::new();
        assert!(repair.suggest_coercion("bool", "Map").is_none());
    }

    #[test]
    fn test_cascade_suppression() {
        let mut sup = CascadeSuppressor::new(5, 3);
        sup.add_error("root error", "main.sl", 10);
        sup.add_error("cascade error", "main.sl", 12);
        assert_eq!(sup.visible_errors().len(), 1);
        assert_eq!(sup.suppressed_count(), 1);
    }

    #[test]
    fn test_cascade_different_files() {
        let mut sup = CascadeSuppressor::new(5, 3);
        sup.add_error("error in a", "a.sl", 10);
        sup.add_error("error in b", "b.sl", 10);
        // Different files — both visible.
        assert_eq!(sup.visible_errors().len(), 2);
    }

    #[test]
    fn test_cascade_out_of_range() {
        let mut sup = CascadeSuppressor::new(3, 3);
        sup.add_error("root", "main.sl", 10);
        sup.add_error("far away", "main.sl", 100);
        // Too far — not suppressed.
        assert_eq!(sup.visible_errors().len(), 2);
    }

    #[test]
    fn test_error_budget() {
        let mut budget = ErrorBudget::new(3, 10);
        assert!(budget.record_error());
        assert!(budget.record_error());
        assert!(budget.record_error());
        assert!(!budget.record_error()); // Over budget.
        assert!(budget.is_exhausted());
    }

    #[test]
    fn test_error_budget_warnings() {
        let mut budget = ErrorBudget::new(100, 2);
        assert!(budget.record_warning());
        assert!(budget.record_warning());
        assert!(!budget.record_warning());
    }

    #[test]
    fn test_error_budget_strict() {
        let mut budget = ErrorBudget::strict(1);
        assert!(budget.record_warning()); // Treated as error.
        assert!(!budget.record_warning()); // Over.
        assert!(budget.is_exhausted());
    }

    #[test]
    fn test_remaining_errors() {
        let mut budget = ErrorBudget::new(5, 10);
        budget.record_error();
        budget.record_error();
        assert_eq!(budget.remaining_errors(), 3);
    }

    #[test]
    fn test_lossless_coercions() {
        let repair = TypeRepair::new();
        let lossless = repair.lossless_coercions();
        assert!(lossless.len() >= 2);
        assert!(lossless.iter().all(|c| c.is_lossless));
    }

    #[test]
    fn test_recovery_replace() {
        let recovery = ParserRecovery::new();
        let strategy = recovery.choose_strategy("fn", "gn");
        assert_eq!(strategy, RecoveryStrategy::ReplaceToken("fn".into()));
    }
}
