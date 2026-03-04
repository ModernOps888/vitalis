//! IDE Features — LSP v4, profiler integration, refactoring engine,
//! code coverage reporting, code actions, and workspace analysis.
//!
//! Extends the existing `lsp.rs` with advanced IDE capabilities for
//! the Vitalis language tooling.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Refactoring Engine ──────────────────────────────────────────────────

/// A refactoring operation that transforms source code.
#[derive(Debug, Clone)]
pub struct Refactoring {
    pub kind: RefactoringKind,
    pub edits: Vec<TextEdit>,
    pub description: String,
}

/// Kinds of refactoring operations.
#[derive(Debug, Clone, PartialEq)]
pub enum RefactoringKind {
    Rename,
    ExtractFunction,
    ExtractVariable,
    InlineFunction,
    InlineVariable,
    MoveToModule,
    ChangeSignature,
    IntroduceParameter,
}

/// A text edit (range-based replacement).
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub file: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub new_text: String,
}

/// Refactoring engine that generates code transformations.
#[derive(Debug)]
pub struct RefactoringEngine {
    pub history: Vec<Refactoring>,
}

impl RefactoringEngine {
    pub fn new() -> Self {
        RefactoringEngine { history: Vec::new() }
    }

    /// Generate rename refactoring.
    pub fn rename(&mut self, file: &str, line: u32, col: u32, old_name: &str, new_name: &str, occurrences: &[(u32, u32, u32, u32)]) -> Refactoring {
        let edits: Vec<TextEdit> = occurrences.iter().map(|&(sl, sc, el, ec)| {
            TextEdit {
                file: file.to_string(),
                start_line: sl, start_col: sc,
                end_line: el, end_col: ec,
                new_text: new_name.to_string(),
            }
        }).collect();

        let refactoring = Refactoring {
            kind: RefactoringKind::Rename,
            edits,
            description: format!("Rename '{}' to '{}' at {}:{}:{}", old_name, new_name, file, line, col),
        };
        self.history.push(refactoring.clone());
        refactoring
    }

    /// Generate extract-function refactoring.
    pub fn extract_function(&mut self, file: &str, start_line: u32, start_col: u32, end_line: u32, end_col: u32, fn_name: &str, extracted_code: &str, params: &[(&str, &str)]) -> Refactoring {
        let param_list: String = params.iter().map(|(n, t)| format!("{}: {}", n, t)).collect::<Vec<_>>().join(", ");
        let call_args: String = params.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ");

        let new_fn = format!("fn {}({}) {{\n    {}\n}}", fn_name, param_list, extracted_code);
        let call_site = format!("{}({})", fn_name, call_args);

        let edits = vec![
            // Replace extracted code with function call
            TextEdit {
                file: file.to_string(),
                start_line, start_col,
                end_line, end_col,
                new_text: call_site,
            },
            // Insert function definition (after current function)
            TextEdit {
                file: file.to_string(),
                start_line: end_line + 2, start_col: 0,
                end_line: end_line + 2, end_col: 0,
                new_text: new_fn,
            },
        ];

        let refactoring = Refactoring {
            kind: RefactoringKind::ExtractFunction,
            edits,
            description: format!("Extract function '{}'", fn_name),
        };
        self.history.push(refactoring.clone());
        refactoring
    }

    /// Generate extract-variable refactoring.
    pub fn extract_variable(&mut self, file: &str, line: u32, start_col: u32, end_col: u32, var_name: &str, expression: &str) -> Refactoring {
        let edits = vec![
            // Insert variable declaration before the line
            TextEdit {
                file: file.to_string(),
                start_line: line, start_col: 0,
                end_line: line, end_col: 0,
                new_text: format!("let {} = {};\n    ", var_name, expression),
            },
            // Replace expression with variable name
            TextEdit {
                file: file.to_string(),
                start_line: line, start_col,
                end_line: line, end_col,
                new_text: var_name.to_string(),
            },
        ];

        let refactoring = Refactoring {
            kind: RefactoringKind::ExtractVariable,
            edits,
            description: format!("Extract variable '{}'", var_name),
        };
        self.history.push(refactoring.clone());
        refactoring
    }

    /// Undo last refactoring.
    pub fn undo(&mut self) -> Option<Refactoring> {
        self.history.pop()
    }
}

// ── Code Coverage ───────────────────────────────────────────────────────

/// Code coverage tracking.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub file_coverages: HashMap<String, FileCoverage>,
    pub total_lines: usize,
    pub covered_lines: usize,
    pub total_branches: usize,
    pub covered_branches: usize,
}

/// Coverage for a single file.
#[derive(Debug, Clone)]
pub struct FileCoverage {
    pub file: String,
    pub line_hits: HashMap<u32, u64>,    // line → hit count
    pub branch_hits: HashMap<u32, (u64, u64)>, // branch → (true_count, false_count)
    pub function_hits: HashMap<String, u64>, // function name → call count
}

impl CoverageReport {
    pub fn new() -> Self {
        CoverageReport {
            file_coverages: HashMap::new(),
            total_lines: 0,
            covered_lines: 0,
            total_branches: 0,
            covered_branches: 0,
        }
    }

    /// Record a line hit.
    pub fn record_line(&mut self, file: &str, line: u32) {
        let fc = self.file_coverages.entry(file.to_string())
            .or_insert_with(|| FileCoverage {
                file: file.to_string(),
                line_hits: HashMap::new(),
                branch_hits: HashMap::new(),
                function_hits: HashMap::new(),
            });
        *fc.line_hits.entry(line).or_insert(0) += 1;
    }

    /// Record a branch hit.
    pub fn record_branch(&mut self, file: &str, branch_id: u32, taken: bool) {
        let fc = self.file_coverages.entry(file.to_string())
            .or_insert_with(|| FileCoverage {
                file: file.to_string(),
                line_hits: HashMap::new(),
                branch_hits: HashMap::new(),
                function_hits: HashMap::new(),
            });
        let entry = fc.branch_hits.entry(branch_id).or_insert((0, 0));
        if taken { entry.0 += 1; } else { entry.1 += 1; }
    }

    /// Record a function call.
    pub fn record_function(&mut self, file: &str, function_name: &str) {
        let fc = self.file_coverages.entry(file.to_string())
            .or_insert_with(|| FileCoverage {
                file: file.to_string(),
                line_hits: HashMap::new(),
                branch_hits: HashMap::new(),
                function_hits: HashMap::new(),
            });
        *fc.function_hits.entry(function_name.to_string()).or_insert(0) += 1;
    }

    /// Compute coverage statistics.
    pub fn compute(&mut self, total_lines: usize, total_branches: usize) {
        self.total_lines = total_lines;
        self.total_branches = total_branches;
        self.covered_lines = self.file_coverages.values()
            .flat_map(|fc| fc.line_hits.values())
            .filter(|&&count| count > 0)
            .count();
        self.covered_branches = self.file_coverages.values()
            .flat_map(|fc| fc.branch_hits.values())
            .filter(|&&(t, f)| t > 0 && f > 0)  // Both branches taken
            .count();
    }

    /// Line coverage percentage.
    pub fn line_coverage_pct(&self) -> f64 {
        if self.total_lines == 0 { 100.0 }
        else { self.covered_lines as f64 / self.total_lines as f64 * 100.0 }
    }

    /// Branch coverage percentage.
    pub fn branch_coverage_pct(&self) -> f64 {
        if self.total_branches == 0 { 100.0 }
        else { self.covered_branches as f64 / self.total_branches as f64 * 100.0 }
    }

    /// Generate LCOV-format output.
    pub fn to_lcov(&self) -> String {
        let mut lcov = String::new();
        for (file, fc) in &self.file_coverages {
            lcov.push_str(&format!("SF:{}\n", file));
            for (func, &count) in &fc.function_hits {
                lcov.push_str(&format!("FN:0,{}\nFNDA:{},{}\n", func, count, func));
            }
            for (&line, &count) in &fc.line_hits {
                lcov.push_str(&format!("DA:{},{}\n", line, count));
            }
            for (&branch, &(t, f)) in &fc.branch_hits {
                lcov.push_str(&format!("BRDA:{},0,0,{}\nBRDA:{},0,1,{}\n", branch, t, branch, f));
            }
            lcov.push_str("end_of_record\n");
        }
        lcov
    }
}

// ── Code Actions ────────────────────────────────────────────────────────

/// A code action suggestion from the IDE.
#[derive(Debug, Clone)]
pub struct CodeAction {
    pub title: String,
    pub kind: CodeActionKind,
    pub edits: Vec<TextEdit>,
    pub diagnostics: Vec<String>,
    pub is_preferred: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CodeActionKind {
    QuickFix,
    Refactor,
    RefactorExtract,
    RefactorInline,
    RefactorRewrite,
    Source,
    SourceOrganizeImports,
    SourceFixAll,
}

/// Generate quick-fix code actions from diagnostics.
pub fn generate_quickfixes(file: &str, diagnostics: &[(u32, String)]) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    for (line, msg) in diagnostics {
        if msg.contains("unused variable") {
            if let Some(var_name) = msg.split('`').nth(1) {
                actions.push(CodeAction {
                    title: format!("Prefix with underscore: _{}", var_name),
                    kind: CodeActionKind::QuickFix,
                    edits: vec![TextEdit {
                        file: file.to_string(),
                        start_line: *line, start_col: 0,
                        end_line: *line, end_col: 0,
                        new_text: format!("_{}", var_name),
                    }],
                    diagnostics: vec![msg.clone()],
                    is_preferred: true,
                });
            }
        }

        if msg.contains("missing import") {
            if let Some(module_name) = msg.split('`').nth(1) {
                actions.push(CodeAction {
                    title: format!("Import {}", module_name),
                    kind: CodeActionKind::QuickFix,
                    edits: vec![TextEdit {
                        file: file.to_string(),
                        start_line: 1, start_col: 0,
                        end_line: 1, end_col: 0,
                        new_text: format!("import {};\n", module_name),
                    }],
                    diagnostics: vec![msg.clone()],
                    is_preferred: true,
                });
            }
        }
    }

    actions
}

// ── Workspace Analysis ──────────────────────────────────────────────────

/// Workspace-wide analysis results.
#[derive(Debug, Clone)]
pub struct WorkspaceAnalysis {
    pub file_count: usize,
    pub total_lines: usize,
    pub function_count: usize,
    pub struct_count: usize,
    pub complexity_scores: HashMap<String, f64>,
    pub dependency_graph: HashMap<String, Vec<String>>,
}

impl WorkspaceAnalysis {
    pub fn new() -> Self {
        WorkspaceAnalysis {
            file_count: 0, total_lines: 0,
            function_count: 0, struct_count: 0,
            complexity_scores: HashMap::new(),
            dependency_graph: HashMap::new(),
        }
    }

    /// Add a file's analysis.
    pub fn add_file(&mut self, file: &str, lines: usize, functions: usize, structs: usize, complexity: f64) {
        self.file_count += 1;
        self.total_lines += lines;
        self.function_count += functions;
        self.struct_count += structs;
        self.complexity_scores.insert(file.to_string(), complexity);
    }

    /// Add a module dependency.
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.dependency_graph.entry(from.to_string()).or_insert_with(Vec::new).push(to.to_string());
    }

    /// Average cyclomatic complexity.
    pub fn avg_complexity(&self) -> f64 {
        if self.complexity_scores.is_empty() { 0.0 }
        else { self.complexity_scores.values().sum::<f64>() / self.complexity_scores.len() as f64 }
    }

    /// Find most complex files.
    pub fn hotspots(&self, threshold: f64) -> Vec<(String, f64)> {
        let mut hot: Vec<_> = self.complexity_scores.iter()
            .filter(|&(_, &c)| c > threshold)
            .map(|(f, &c)| (f.clone(), c))
            .collect();
        hot.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        hot
    }

    /// Check for circular dependencies.
    pub fn has_circular_dependencies(&self) -> bool {
        for start in self.dependency_graph.keys() {
            let mut visited = std::collections::HashSet::new();
            let mut stack = vec![start.clone()];
            while let Some(node) = stack.pop() {
                if !visited.insert(node.clone()) {
                    if node == *start { return true; }
                    continue;
                }
                if let Some(deps) = self.dependency_graph.get(&node) {
                    for dep in deps {
                        stack.push(dep.clone());
                    }
                }
            }
        }
        false
    }
}

// ── FFI ─────────────────────────────────────────────────────────────────

static IDE_STORES: Mutex<Option<HashMap<i64, RefactoringEngine>>> = Mutex::new(None);

fn ide_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, RefactoringEngine>>> {
    IDE_STORES.lock().unwrap()
}

fn next_ide_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ide_create() -> i64 {
    let id = next_ide_id();
    let engine = RefactoringEngine::new();
    let mut store = ide_store();
    store.get_or_insert_with(HashMap::new).insert(id, engine);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ide_history_len(id: i64) -> i64 {
    let store = ide_store();
    store.as_ref().and_then(|s| s.get(&id))
        .map(|e| e.history.len() as i64)
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ide_free(id: i64) {
    let mut store = ide_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rename_refactoring() {
        let mut engine = RefactoringEngine::new();
        let r = engine.rename("main.sl", 5, 4, "foo", "bar", &[(5, 4, 5, 7), (10, 8, 10, 11)]);
        assert_eq!(r.kind, RefactoringKind::Rename);
        assert_eq!(r.edits.len(), 2);
        assert_eq!(r.edits[0].new_text, "bar");
    }

    #[test]
    fn test_extract_function() {
        let mut engine = RefactoringEngine::new();
        let r = engine.extract_function("main.sl", 5, 0, 8, 20, "helper", "x + y",
            &[("x", "i32"), ("y", "i32")]);
        assert_eq!(r.kind, RefactoringKind::ExtractFunction);
        assert_eq!(r.edits.len(), 2);
        assert!(r.edits[0].new_text.contains("helper(x, y)"));
    }

    #[test]
    fn test_extract_variable() {
        let mut engine = RefactoringEngine::new();
        let r = engine.extract_variable("main.sl", 10, 8, 25, "result", "complex_expr()");
        assert_eq!(r.kind, RefactoringKind::ExtractVariable);
        assert!(r.edits[0].new_text.contains("let result"));
    }

    #[test]
    fn test_undo() {
        let mut engine = RefactoringEngine::new();
        engine.rename("f.sl", 1, 0, "a", "b", &[(1, 0, 1, 1)]);
        assert_eq!(engine.history.len(), 1);
        let undone = engine.undo();
        assert!(undone.is_some());
        assert_eq!(engine.history.len(), 0);
    }

    #[test]
    fn test_coverage_report() {
        let mut report = CoverageReport::new();
        report.record_line("test.sl", 1);
        report.record_line("test.sl", 2);
        report.record_line("test.sl", 1); // hit twice
        report.compute(10, 0);
        assert_eq!(report.covered_lines, 2);
        assert!((report.line_coverage_pct() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_branch_coverage() {
        let mut report = CoverageReport::new();
        report.record_branch("test.sl", 1, true);
        report.record_branch("test.sl", 1, false);
        report.record_branch("test.sl", 2, true); // only true branch
        report.compute(10, 2);
        assert_eq!(report.covered_branches, 1); // Only branch 1 has both sides
        assert!((report.branch_coverage_pct() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_function_coverage() {
        let mut report = CoverageReport::new();
        report.record_function("test.sl", "main");
        report.record_function("test.sl", "main");
        report.record_function("test.sl", "helper");
        let fc = report.file_coverages.get("test.sl").unwrap();
        assert_eq!(*fc.function_hits.get("main").unwrap(), 2);
        assert_eq!(*fc.function_hits.get("helper").unwrap(), 1);
    }

    #[test]
    fn test_lcov_output() {
        let mut report = CoverageReport::new();
        report.record_line("test.sl", 5);
        report.record_function("test.sl", "main");
        let lcov = report.to_lcov();
        assert!(lcov.contains("SF:test.sl"));
        assert!(lcov.contains("DA:5,1"));
        assert!(lcov.contains("end_of_record"));
    }

    #[test]
    fn test_quickfix_unused_var() {
        let actions = generate_quickfixes("test.sl", &[(5, "unused variable `x`".into())]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].kind, CodeActionKind::QuickFix);
        assert!(actions[0].title.contains("_x"));
    }

    #[test]
    fn test_workspace_analysis() {
        let mut ws = WorkspaceAnalysis::new();
        ws.add_file("a.sl", 100, 5, 2, 3.5);
        ws.add_file("b.sl", 200, 10, 3, 7.2);
        assert_eq!(ws.file_count, 2);
        assert_eq!(ws.total_lines, 300);
        assert!((ws.avg_complexity() - 5.35).abs() < 0.01);
    }

    #[test]
    fn test_hotspots() {
        let mut ws = WorkspaceAnalysis::new();
        ws.add_file("simple.sl", 50, 2, 0, 1.5);
        ws.add_file("complex.sl", 500, 20, 5, 15.0);
        ws.add_file("medium.sl", 200, 8, 2, 5.0);
        let hot = ws.hotspots(10.0);
        assert_eq!(hot.len(), 1);
        assert_eq!(hot[0].0, "complex.sl");
    }

    #[test]
    fn test_circular_deps() {
        let mut ws = WorkspaceAnalysis::new();
        ws.add_dependency("a", "b");
        ws.add_dependency("b", "c");
        assert!(!ws.has_circular_dependencies());

        ws.add_dependency("c", "a");
        assert!(ws.has_circular_dependencies());
    }

    #[test]
    fn test_ffi_ide() {
        let id = vitalis_ide_create();
        assert!(id > 0);
        assert_eq!(vitalis_ide_history_len(id), 0);
        vitalis_ide_free(id);
    }
}
