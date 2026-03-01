//! Vitalis — a JIT-compiled programming language with built-in code evolution.
//!
//! This library provides the compiler pipeline (lex → parse → type-check → IR → JIT)
//! and a C FFI bridge so Python code (via ctypes) and other languages can compile
//! and execute `.sl` code natively.
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

pub mod lexer;
pub mod ast;
pub mod parser;
pub mod types;
pub mod ir;
pub mod codegen;
pub mod stdlib;
pub mod evolution;
pub mod engine;
pub mod bridge;
pub mod hotpath;
pub mod memory;
pub mod meta_evolution;
pub mod optimizer;
pub mod simd_ops;
