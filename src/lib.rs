//! Vitalis v9.0 — A JIT-compiled programming language with built-in code evolution,
//! multi-domain algorithm libraries, and native performance via Cranelift.
//!
//! Final community release. This library provides the compiler pipeline
//! (lex → parse → type-check → IR → JIT) and a C FFI bridge so Python code
//! (via ctypes) and other languages can compile and execute `.sl` code natively.
//!
//! # Architecture
//!
//! ```text
//! Source (.sl) → Lexer → Parser → AST → TypeChecker → IR → Cranelift JIT → native
//!                                                            ↕
//!                                                   C FFI bridge (bridge.rs)
//!                                                            ↕
//!                                                   Python (vitalis.py)
//! ```
//!
//! # Module Domains (v9.0 — 28 modules)
//! - **Core Compiler**: lexer, ast, parser, types, ir, codegen, stdlib
//! - **Evolution**: evolution, engine, meta_evolution, optimizer
//! - **Memory**: memory (engram store)
//! - **Performance**: hotpath, simd_ops
//! - **Signal Processing**: signal_processing (FFT, DSP, filtering)
//! - **Cryptography**: crypto (SHA-256, HMAC, Base64, CRC32)
//! - **Graph Theory**: graph (BFS, DFS, Dijkstra, MST, SCC, PageRank)
//! - **String Algorithms**: string_algorithms (KMP, Levenshtein, Jaro-Winkler)
//! - **Numerical Methods**: numerical (linear algebra, calculus, interpolation)
//! - **Compression**: compression (RLE, Huffman, LZ77, BWT, delta)
//! - **Probability & Statistics**: probability (distributions, regression, tests)
//! - **Quantum & Advanced Math**: quantum, quantum_math, advanced_math
//! - **Science & Physics**: science (mechanics, thermo, EM, nuclear, chemistry)
//! - **Analytics & Reporting**: analytics (time-series, anomaly detection, forecasting)
//! - **Security Guardrails**: security (validation, injection detection, sandboxing)
//! - **Scoring & Fitness**: scoring (code quality, ELO, Pareto, A/B testing)

// ── Core Compiler Pipeline ───────────────────────────────────────────
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod types;
pub mod ir;
pub mod codegen;
pub mod stdlib;

// ── Evolution & Self-Modification ────────────────────────────────────
pub mod evolution;
pub mod engine;
pub mod meta_evolution;
pub mod optimizer;

// ── Memory ───────────────────────────────────────────────────────────
pub mod memory;

// ── Performance ──────────────────────────────────────────────────────
pub mod hotpath;
pub mod simd_ops;

// ── Multi-Domain Algorithm Libraries (v7.0) ──────────────────────────
pub mod signal_processing;
pub mod crypto;
pub mod graph;
pub mod string_algorithms;
pub mod numerical;
pub mod compression;
pub mod probability;

// ── Quantum & Advanced Mathematics (v9.0) ────────────────────────────
pub mod quantum;
pub mod quantum_math;
pub mod advanced_math;

// ── Science & Physics (v9.0) ─────────────────────────────────────────
pub mod science;

// ── Analytics & Reporting (v9.0) ─────────────────────────────────────
pub mod analytics;

// ── Security Guardrails (v9.0) ───────────────────────────────────────
pub mod security;

// ── Scoring & Fitness Evaluation (v9.0) ──────────────────────────────
pub mod scoring;

// ── FFI Bridge ───────────────────────────────────────────────────────
pub mod bridge;
