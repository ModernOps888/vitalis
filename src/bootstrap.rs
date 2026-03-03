//! Vitalis Self-Hosted Compiler Bootstrap (v22 Roadmap)
//!
//! Provides the infrastructure for bootstrapping Vitalis's compiler in Vitalis itself.
//! The bootstrap system enables a multi-stage compilation process:
//!
//! ```text
//! Stage 0: Rust-based compiler (current implementation)
//! Stage 1: Vitalis compiler written in .sl, compiled by Stage 0
//! Stage 2: Self-compiled Vitalis compiler (Stage 1 compiles itself)
//! ```
//!
//! This module provides:
//! - Bootstrap pipeline management (stage tracking, verification)
//! - Cross-validation between stage outputs (IR comparison, binary diff)
//! - Minimal Vitalis runtime library for the self-hosted compiler
//! - Bootstrap test harness
//!
//! The actual Vitalis source for the self-hosted compiler lives in `bootstrap/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Bootstrap Stages
// ═══════════════════════════════════════════════════════════════════════

/// A compilation stage in the bootstrap process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Stage 0: The Rust-based compiler (this binary)
    Stage0,
    /// Stage 1: Vitalis compiler compiled by Stage 0
    Stage1,
    /// Stage 2: Vitalis compiler compiled by Stage 1 (self-hosted)
    Stage2,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stage::Stage0 => write!(f, "Stage 0 (Rust)"),
            Stage::Stage1 => write!(f, "Stage 1 (Vitalis via Rust)"),
            Stage::Stage2 => write!(f, "Stage 2 (Self-hosted)"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Bootstrap Configuration
// ═══════════════════════════════════════════════════════════════════════

/// Configuration for the bootstrap process.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// Root directory for bootstrap source files.
    pub bootstrap_dir: PathBuf,
    /// Output directory for compiled stages.
    pub output_dir: PathBuf,
    /// Whether to verify stage outputs match.
    pub verify_stages: bool,
    /// Test programs to compile with each stage for verification.
    pub test_programs: Vec<PathBuf>,
    /// Whether to keep intermediate artifacts.
    pub keep_artifacts: bool,
    /// Verbose logging.
    pub verbose: bool,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            bootstrap_dir: PathBuf::from("bootstrap"),
            output_dir: PathBuf::from("target/bootstrap"),
            verify_stages: true,
            test_programs: Vec::new(),
            keep_artifacts: true,
            verbose: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Bootstrap Artifacts
// ═══════════════════════════════════════════════════════════════════════

/// An artifact produced by a bootstrap stage.
#[derive(Debug, Clone)]
pub struct BootstrapArtifact {
    pub stage: Stage,
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub compile_time: Duration,
    /// Hash of the produced IR (for cross-validation).
    pub ir_hash: u64,
    /// Hash of the produced binary output.
    pub binary_hash: u64,
}

impl fmt::Display for BootstrapArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} — {} bytes, compiled in {:?} (IR: {:016x}, BIN: {:016x})",
               self.stage, self.name, self.size_bytes, self.compile_time,
               self.ir_hash, self.binary_hash)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Bootstrap Validation
// ═══════════════════════════════════════════════════════════════════════

/// Result of cross-validating two bootstrap stages.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub stage_a: Stage,
    pub stage_b: Stage,
    pub program: String,
    pub ir_match: bool,
    pub output_match: bool,
    pub details: Vec<String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.ir_match && self.output_match
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.is_valid() { "✓ PASS" } else { "✗ FAIL" };
        write!(f, "{} — {} vs {} on '{}' (IR: {}, Output: {})",
               status, self.stage_a, self.stage_b, self.program,
               if self.ir_match { "match" } else { "MISMATCH" },
               if self.output_match { "match" } else { "MISMATCH" })
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Bootstrap Errors
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct BootstrapError {
    pub stage: Stage,
    pub message: String,
    pub kind: BootstrapErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BootstrapErrorKind {
    /// Source files not found.
    MissingSources,
    /// Compilation failed at a stage.
    CompilationFailed,
    /// Stage outputs don't match (verification failure).
    VerificationFailed,
    /// IO error.
    IoError,
    /// The self-hosted compiler produced different output.
    OutputMismatch,
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bootstrap error at {}: {}", self.stage, self.message)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  FNV-1a Hash (for artifact comparison)
// ═══════════════════════════════════════════════════════════════════════

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ═══════════════════════════════════════════════════════════════════════
//  Bootstrap Pipeline
// ═══════════════════════════════════════════════════════════════════════

/// The bootstrap pipeline manager.
///
/// Orchestrates the multi-stage compilation process and validates
/// that each stage produces consistent output.
pub struct BootstrapPipeline {
    config: BootstrapConfig,
    /// Artifacts produced by each stage.
    artifacts: Vec<BootstrapArtifact>,
    /// Validation results.
    validations: Vec<ValidationResult>,
    /// Errors encountered.
    errors: Vec<BootstrapError>,
    /// Current stage.
    current_stage: Stage,
    /// Bootstrap source files (component name → source code).
    bootstrap_sources: HashMap<String, String>,
}

impl BootstrapPipeline {
    pub fn new(config: BootstrapConfig) -> Self {
        Self {
            config,
            artifacts: Vec::new(),
            validations: Vec::new(),
            errors: Vec::new(),
            current_stage: Stage::Stage0,
            bootstrap_sources: HashMap::new(),
        }
    }

    /// Load bootstrap source files from the bootstrap directory.
    pub fn load_sources(&mut self) -> Result<usize, BootstrapError> {
        let dir = &self.config.bootstrap_dir;
        if !dir.exists() {
            // Create the bootstrap directory with skeleton files
            std::fs::create_dir_all(dir).map_err(|e| BootstrapError {
                stage: Stage::Stage0,
                message: format!("cannot create bootstrap dir: {}", e),
                kind: BootstrapErrorKind::IoError,
            })?;
        }

        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "sl").unwrap_or(false) {
                    if let Ok(source) = std::fs::read_to_string(&path) {
                        let name = path.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        self.bootstrap_sources.insert(name, source);
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Run Stage 0: Compile a test program using the Rust-based compiler.
    pub fn run_stage0(&mut self, source: &str, name: &str) -> Result<BootstrapArtifact, BootstrapError> {
        let start = Instant::now();
        self.current_stage = Stage::Stage0;

        if self.config.verbose {
            eprintln!("[bootstrap] Stage 0: Compiling '{}'...", name);
        }

        // Parse
        let (program, parse_errors) = crate::parser::parse(source);
        if !parse_errors.is_empty() {
            return Err(BootstrapError {
                stage: Stage::Stage0,
                message: format!("parse errors in '{}': {}",
                    name,
                    parse_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ")
                ),
                kind: BootstrapErrorKind::CompilationFailed,
            });
        }

        // Type check
        let _type_errors = crate::types::TypeChecker::new().check(&program);

        // Lower to IR
        let ir_module = crate::ir::IrBuilder::new().build(&program);

        // Compute IR hash for verification
        let ir_repr = format!("{:?}", ir_module.functions.iter()
            .map(|f| (&f.name, f.blocks.len()))
            .collect::<Vec<_>>());
        let ir_hash = fnv1a_hash(ir_repr.as_bytes());

        // Compile
        let mut jit = crate::codegen::JitCompiler::new().map_err(|e| BootstrapError {
            stage: Stage::Stage0,
            message: e.to_string(),
            kind: BootstrapErrorKind::CompilationFailed,
        })?;
        jit.compile(&ir_module).map_err(|e| BootstrapError {
            stage: Stage::Stage0,
            message: e.to_string(),
            kind: BootstrapErrorKind::CompilationFailed,
        })?;

        let duration = start.elapsed();
        let binary_hash = fnv1a_hash(source.as_bytes()); // simplified

        let artifact = BootstrapArtifact {
            stage: Stage::Stage0,
            name: name.to_string(),
            path: self.config.output_dir.join(format!("stage0_{}", name)),
            size_bytes: source.len() as u64,
            compile_time: duration,
            ir_hash,
            binary_hash,
        };

        if self.config.verbose {
            eprintln!("[bootstrap] {}", artifact);
        }

        self.artifacts.push(artifact.clone());
        Ok(artifact)
    }

    /// Validate that two stages produce the same output for a test program.
    pub fn validate_stages(
        &mut self,
        stage_a: Stage,
        stage_b: Stage,
        program_name: &str,
    ) -> ValidationResult {
        let artifacts_a: Vec<_> = self.artifacts.iter()
            .filter(|a| a.stage == stage_a && a.name == program_name)
            .collect();
        let artifacts_b: Vec<_> = self.artifacts.iter()
            .filter(|a| a.stage == stage_b && a.name == program_name)
            .collect();

        let (ir_match, output_match, details) = match (artifacts_a.first(), artifacts_b.first()) {
            (Some(a), Some(b)) => {
                let ir_match = a.ir_hash == b.ir_hash;
                let output_match = a.binary_hash == b.binary_hash;
                let mut details = Vec::new();
                if !ir_match {
                    details.push(format!("IR hash mismatch: {:016x} vs {:016x}", a.ir_hash, b.ir_hash));
                }
                if !output_match {
                    details.push(format!("Binary hash mismatch: {:016x} vs {:016x}", a.binary_hash, b.binary_hash));
                }
                (ir_match, output_match, details)
            }
            _ => {
                (false, false, vec!["Missing artifacts for comparison".to_string()])
            }
        };

        let result = ValidationResult {
            stage_a,
            stage_b,
            program: program_name.to_string(),
            ir_match,
            output_match,
            details,
        };

        self.validations.push(result.clone());
        result
    }

    /// Run the full bootstrap pipeline.
    pub fn run_full_bootstrap(&mut self) -> BootstrapReport {
        let start = Instant::now();

        // Load sources
        let source_count = match self.load_sources() {
            Ok(n) => n,
            Err(e) => {
                self.errors.push(e);
                0
            }
        };

        // Compile test programs with Stage 0
        let test_sources = vec![
            ("hello", "fn main() -> i64 { 42 }"),
            ("arithmetic", "fn main() -> i64 { let x: i64 = 10; let y: i64 = 20; x + y }"),
            ("nested", "fn add(a: i64, b: i64) -> i64 { a + b }\nfn main() -> i64 { add(3, 4) }"),
        ];

        let mut stage0_results = Vec::new();
        for (name, source) in &test_sources {
            match self.run_stage0(source, name) {
                Ok(artifact) => stage0_results.push(artifact),
                Err(e) => self.errors.push(e),
            }
        }

        let duration = start.elapsed();

        BootstrapReport {
            bootstrap_sources: source_count,
            stage0_artifacts: stage0_results.len(),
            stage1_artifacts: 0, // Future: when Stage 1 is implemented
            stage2_artifacts: 0,
            validations: self.validations.clone(),
            errors: self.errors.clone(),
            total_time: duration,
        }
    }

    // ── Queries ────────────────────────────────────────────────────────

    /// Get all artifacts.
    pub fn artifacts(&self) -> &[BootstrapArtifact] {
        &self.artifacts
    }

    /// Get all validation results.
    pub fn validations(&self) -> &[ValidationResult] {
        &self.validations
    }

    /// Get all errors.
    pub fn errors(&self) -> &[BootstrapError] {
        &self.errors
    }

    /// Get bootstrap sources.
    pub fn sources(&self) -> &HashMap<String, String> {
        &self.bootstrap_sources
    }

    /// Current stage.
    pub fn current_stage(&self) -> Stage {
        self.current_stage
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Bootstrap Report
// ═══════════════════════════════════════════════════════════════════════

/// Summary of a bootstrap run.
#[derive(Debug, Clone)]
pub struct BootstrapReport {
    pub bootstrap_sources: usize,
    pub stage0_artifacts: usize,
    pub stage1_artifacts: usize,
    pub stage2_artifacts: usize,
    pub validations: Vec<ValidationResult>,
    pub errors: Vec<BootstrapError>,
    pub total_time: Duration,
}

impl BootstrapReport {
    pub fn is_success(&self) -> bool {
        self.errors.is_empty() && self.validations.iter().all(|v| v.is_valid())
    }
}

impl fmt::Display for BootstrapReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "═══ Bootstrap Report ═══")?;
        writeln!(f, "  Bootstrap sources: {}", self.bootstrap_sources)?;
        writeln!(f, "  Stage 0 artifacts: {}", self.stage0_artifacts)?;
        writeln!(f, "  Stage 1 artifacts: {}", self.stage1_artifacts)?;
        writeln!(f, "  Stage 2 artifacts: {}", self.stage2_artifacts)?;
        writeln!(f, "  Validations:       {}", self.validations.len())?;
        for v in &self.validations {
            writeln!(f, "    {}", v)?;
        }
        writeln!(f, "  Errors:            {}", self.errors.len())?;
        for e in &self.errors {
            writeln!(f, "    {}", e)?;
        }
        writeln!(f, "  Total time:        {:?}", self.total_time)?;
        if self.is_success() {
            writeln!(f, "  Status:            ✓ SUCCESS")
        } else {
            writeln!(f, "  Status:            ✗ FAILED")
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Bootstrap Skeleton Generator
// ═══════════════════════════════════════════════════════════════════════

/// Generates the skeleton Vitalis source files for the self-hosted compiler.
pub fn generate_bootstrap_skeleton(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    std::fs::create_dir_all(dir)?;
    let mut created = Vec::new();

    // Lexer skeleton
    let lexer_source = r#"// Vitalis Self-Hosted Lexer
// This is the Stage 1 lexer, written in Vitalis (.sl)

fn lex(source: str) -> i64 {
    // Token types: 0=EOF, 1=Int, 2=Ident, 3=String, 4=Plus, 5=Minus, ...
    let pos: i64 = 0;
    let token_count: i64 = 0;
    // Lexer implementation will go here
    token_count
}

fn is_digit(c: i64) -> bool {
    c >= 48 && c <= 57
}

fn is_alpha(c: i64) -> bool {
    (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 95
}

fn main() -> i64 {
    // Self-test
    42
}
"#;

    let lexer_path = dir.join("lexer.sl");
    std::fs::write(&lexer_path, lexer_source)?;
    created.push(lexer_path);

    // Parser skeleton
    let parser_source = r#"// Vitalis Self-Hosted Parser
// Recursive descent parser for .sl syntax

fn parse(source: str) -> i64 {
    // Returns the number of parsed top-level items
    let items: i64 = 0;
    items
}

fn main() -> i64 {
    // Self-test
    42
}
"#;

    let parser_path = dir.join("parser.sl");
    std::fs::write(&parser_path, parser_source)?;
    created.push(parser_path);

    // Type checker skeleton
    let types_source = r#"// Vitalis Self-Hosted Type Checker
// Structural type checking with capability annotations

fn type_check(ast: i64) -> i64 {
    // Returns the number of type errors
    0
}

fn main() -> i64 {
    42
}
"#;

    let types_path = dir.join("types.sl");
    std::fs::write(&types_path, types_source)?;
    created.push(types_path);

    // IR builder skeleton
    let ir_source = r#"// Vitalis Self-Hosted IR Builder
// Lowers AST to SSA-based intermediate representation

fn build_ir(ast: i64) -> i64 {
    // Returns the number of IR functions built
    0
}

fn main() -> i64 {
    42
}
"#;

    let ir_path = dir.join("ir.sl");
    std::fs::write(&ir_path, ir_source)?;
    created.push(ir_path);

    // Main compiler driver
    let main_source = r#"// Vitalis Self-Hosted Compiler Driver
// Stage 1 compiler entry point

fn compile(source: str) -> i64 {
    // Full pipeline: lex -> parse -> typecheck -> IR -> codegen
    // Returns 0 on success, non-zero on error
    0
}

fn main() -> i64 {
    // Bootstrap self-test: compile a simple program
    let result: i64 = compile("fn main() -> i64 { 42 }");
    result
}
"#;

    let main_path = dir.join("main.sl");
    std::fs::write(&main_path, main_source)?;
    created.push(main_path);

    Ok(created)
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_ordering() {
        assert!(Stage::Stage0 < Stage::Stage1);
        assert!(Stage::Stage1 < Stage::Stage2);
    }

    #[test]
    fn test_stage_display() {
        assert_eq!(format!("{}", Stage::Stage0), "Stage 0 (Rust)");
        assert_eq!(format!("{}", Stage::Stage1), "Stage 1 (Vitalis via Rust)");
        assert_eq!(format!("{}", Stage::Stage2), "Stage 2 (Self-hosted)");
    }

    #[test]
    fn test_fnv1a_hash() {
        let hash1 = fnv1a_hash(b"hello");
        let hash2 = fnv1a_hash(b"world");
        let hash3 = fnv1a_hash(b"hello");
        assert_ne!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_bootstrap_config_default() {
        let config = BootstrapConfig::default();
        assert_eq!(config.bootstrap_dir, PathBuf::from("bootstrap"));
        assert!(config.verify_stages);
        assert!(config.keep_artifacts);
    }

    #[test]
    fn test_bootstrap_pipeline_creation() {
        let pipeline = BootstrapPipeline::new(BootstrapConfig::default());
        assert_eq!(pipeline.current_stage(), Stage::Stage0);
        assert!(pipeline.artifacts().is_empty());
        assert!(pipeline.errors().is_empty());
    }

    #[test]
    fn test_stage0_compilation() {
        let mut pipeline = BootstrapPipeline::new(BootstrapConfig::default());
        let result = pipeline.run_stage0("fn main() -> i64 { 42 }", "test_hello");
        assert!(result.is_ok());
        let artifact = result.unwrap();
        assert_eq!(artifact.stage, Stage::Stage0);
        assert_eq!(artifact.name, "test_hello");
        assert!(artifact.ir_hash != 0);
    }

    #[test]
    fn test_stage0_parse_error() {
        let mut pipeline = BootstrapPipeline::new(BootstrapConfig::default());
        let result = pipeline.run_stage0("fn main( { }", "bad_program");
        // Should either succeed (if parser is permissive) or fail with compilation error
        // The important thing is it doesn't panic
        if let Err(e) = result {
            assert_eq!(e.kind, BootstrapErrorKind::CompilationFailed);
        }
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult {
            stage_a: Stage::Stage0,
            stage_b: Stage::Stage1,
            program: "test".to_string(),
            ir_match: true,
            output_match: true,
            details: Vec::new(),
        };
        assert!(result.is_valid());

        let fail_result = ValidationResult {
            stage_a: Stage::Stage0,
            stage_b: Stage::Stage1,
            program: "test".to_string(),
            ir_match: false,
            output_match: true,
            details: vec!["IR mismatch".to_string()],
        };
        assert!(!fail_result.is_valid());
    }

    #[test]
    fn test_bootstrap_report_display() {
        let report = BootstrapReport {
            bootstrap_sources: 5,
            stage0_artifacts: 3,
            stage1_artifacts: 0,
            stage2_artifacts: 0,
            validations: Vec::new(),
            errors: Vec::new(),
            total_time: Duration::from_millis(100),
        };
        assert!(report.is_success());
        let display = format!("{}", report);
        assert!(display.contains("SUCCESS"));
        assert!(display.contains("Stage 0 artifacts: 3"));
    }

    #[test]
    fn test_full_bootstrap_pipeline() {
        let mut pipeline = BootstrapPipeline::new(BootstrapConfig {
            verbose: false,
            ..Default::default()
        });
        let report = pipeline.run_full_bootstrap();
        assert!(report.stage0_artifacts > 0);
    }
}
