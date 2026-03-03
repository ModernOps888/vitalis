//! Vitalis v24.0 — A JIT/AOT-compiled language with algebraic effect handlers,
//! pattern exhaustiveness checking, or-patterns, non-lexical lifetimes (NLL),
//! async/await, generics, WASM target, LSP IDE support, GPU compute, package management,
//! impl blocks, try/catch, closures with capture, stdlib functions, built-in code
//! evolution, multi-domain algorithm libraries, lifetime annotations, region analysis,
//! effect system, capability types, incremental codegen, hot-reload, self-hosted compiler
//! bootstrap, native AOT compilation, cross-compilation (x86_64, AArch64, RISC-V),
//! and native Cranelift JIT performance.
//!
//! Enterprise-grade release with 61 modules spanning algebraic effect handlers, pattern
//! exhaustiveness analysis, NLL borrow analysis, async runtimes,
//! type-system generics, WebAssembly compilation, GPU compute shaders, language server
//! protocol, package management, quantum computing, bioinformatics, neuromorphic
//! computation, advanced evolutionary algorithms, physical sciences, lifetime/region
//! analysis, effect systems, AOT compilation, and cross-compilation targets.
//! This library provides the compiler pipeline (lex → parse → type-check → IR → JIT)
//! and a C FFI bridge so Python code (via ctypes) and other languages can compile
//! and execute `.sl` code natively.
//!
//! # Architecture
//!
//! ```text
//! Source (.sl) → Lexer → Parser → AST → TypeChecker → IR → Cranelift JIT → native
//!                                                      ↕           ↕
//!                                                  WASM Target  GPU Compute
//!                                                      ↕
//!                                             C FFI bridge (bridge.rs)
//!                                                      ↕
//!                                             Python (vitalis.py)
//! ```
//!
//! # Module Domains (v23.0 — 59 modules)
//! - **Core Compiler**: lexer, ast, parser, types, ir, codegen, stdlib
//! - **Async Runtime**: async_runtime (executor, tasks, channels, futures)
//! - **Generics**: generics (type params, monomorphization, type inference, bounds)
//! - **Package Manager**: package_manager (SemVer, registry, dependency resolution)
//! - **LSP Server**: lsp (diagnostics, completion, hover, go-to-def, symbols)
//! - **WebAssembly**: wasm_target (module builder, sections, LEB128, validation)
//! - **GPU Compute**: gpu_compute (buffers, kernels, pipelines, shader builder)
//! - **Evolution**: evolution, engine, meta_evolution, optimizer
//! - **Advanced Evolution**: evolution_advanced (DE, PSO, CMA-ES, NSGA-II, MAP-Elites)
//! - **Memory**: memory (engram store)
//! - **Performance**: hotpath, simd_ops
//! - **Signal Processing**: signal_processing (FFT, DSP, filtering)
//! - **Cryptography**: crypto (SHA-256, HMAC, Base64, CRC32)
//! - **Graph Theory**: graph (BFS, DFS, Dijkstra, MST, SCC, PageRank)
//! - **String Algorithms**: string_algorithms (KMP, Levenshtein, Jaro-Winkler)
//! - **Numerical Methods**: numerical (linear algebra, calculus, interpolation)
//! - **Compression**: compression (RLE, Huffman, LZ77, BWT, delta)
//! - **Probability & Statistics**: probability (distributions, regression, tests)
//! - **Quantum Computing**: quantum, quantum_math, quantum_algorithms (Shor, VQE, QAOA, QPE)
//! - **Advanced Mathematics**: advanced_math (number theory, tensors, Galois fields)
//! - **Science & Physics**: science, chemistry_advanced (stat-mech, relativity, QM)
//! - **Bioinformatics**: bioinformatics (DNA/RNA, alignment, epidemiology, kinetics)
//! - **Neuromorphic Computing**: neuromorphic (LIF, Izhikevich, STDP, ESN, NEAT)
//! - **Analytics & Reporting**: analytics (time-series, anomaly detection, forecasting)
//! - **Security Guardrails**: security (validation, injection detection, sandboxing)
//! - **Scoring & Fitness**: scoring (code quality, ELO, Pareto, A/B testing)
//! - **Machine Learning**: ml (k-means, KNN, Naive Bayes, PCA, DBSCAN, LDA)
//! - **Computational Geometry**: geometry (convex hull, Voronoi, Welzl, triangulation)
//! - **Sorting & Searching**: sorting (quicksort, mergesort, radixsort, binary search)
//! - **Automata & Patterns**: automata (Aho-Corasick, Bloom filter, tries, regex)
//! - **Combinatorial Optimization**: combinatorial (knapsack, TSP, simplex, genetic)
//! - **Lifetime Annotations**: lifetimes (region variables, borrow scoping, outlives constraints)
//! - **Effect System**: effects (IO, Net, FS, Async, GPU effects, capability tokens)
//! - **Hot-Reload**: hot_reload (file watcher, incremental compile, live function swap)
//! - **Self-Hosted Bootstrap**: bootstrap (Stage 0/1/2 pipeline, cross-validation)
//! - **Native AOT Compilation**: aot (ObjectModule backend, static linking, standalone executables)
//! - **Cross-Compilation**: cross_compile (x86_64, AArch64, RISC-V targets, ABI configs)
//! - **Non-Lexical Lifetimes**: nll (CFG builder, liveness analysis, NLL regions, conflict detection)
//! - **Effect Handlers**: effect_handlers (algebraic effects, continuations, handler stacks)
//! - **Pattern Analysis**: pattern_exhaustiveness (exhaustiveness, redundancy, or-patterns, witnesses)

// ── Core Compiler Pipeline ───────────────────────────────────────────
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod types;
pub mod ir;
pub mod codegen;
pub mod stdlib;

// ── Async Runtime (v21.0) ────────────────────────────────────────────
pub mod async_runtime;

// ── Generics & Type Parameters (v21.0) ───────────────────────────────
pub mod generics;

// ── Package Manager & Registry (v21.0) ───────────────────────────────
pub mod package_manager;

// ── LSP Server & IDE Support (v21.0) ─────────────────────────────────
pub mod lsp;

// ── WebAssembly Target (v21.0) ───────────────────────────────────────
pub mod wasm_target;

// ── GPU Compute Backend (v21.0) ──────────────────────────────────────
pub mod gpu_compute;

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

// ── Machine Learning, Geometry, Sorting, Automata, Optimization (v10.0)
pub mod ml;
pub mod geometry;
pub mod sorting;
pub mod automata;
pub mod combinatorial;

// ── Quantum Algorithms (v13.0) ────────────────────────────────────────
pub mod quantum_algorithms;

// ── Bioinformatics (v13.0) ────────────────────────────────────────────
pub mod bioinformatics;

// ── Advanced Chemistry & Physics (v13.0) ──────────────────────────────
pub mod chemistry_advanced;

// ── Neuromorphic Computing (v13.0) ────────────────────────────────────
pub mod neuromorphic;

// ── Advanced Evolutionary Computation (v13.0) ─────────────────────────
pub mod evolution_advanced;

// ── FFI Bridge ───────────────────────────────────────────────────────
pub mod bridge;

// ── v22: Borrow Checker & Ownership Analysis ─────────────────────────
pub mod ownership;

// ── v22: Incremental Compilation & Caching ───────────────────────────
pub mod incremental;

// ── v22: Full Trait Dispatch with VTables ────────────────────────────
pub mod trait_dispatch;

// ── v22: Debug Adapter Protocol (DAP) ────────────────────────────────
pub mod dap;

// ── v22: Interactive REPL ────────────────────────────────────────────
pub mod repl;

// ── v22: Lifetime Annotations & Region Analysis ─────────────────────
pub mod lifetimes;

// ── v22: Effect System & Capability Types ───────────────────────────
pub mod effects;

// ── v22: Hot-Reload Engine ──────────────────────────────────────────
pub mod hot_reload;

// ── v22: Self-Hosted Compiler Bootstrap ─────────────────────────────
pub mod bootstrap;

// ── v22: Native AOT Compilation ─────────────────────────────────────
pub mod aot;

// ── v22: Cross-Compilation Targets (ARM, RISC-V) ────────────────────
pub mod cross_compile;

// ── v23: Non-Lexical Lifetimes (NLL) ────────────────────────────────
pub mod nll;

// ── v24: Effect Handlers & Pattern Exhaustiveness ───────────────────
pub mod effect_handlers;
pub mod pattern_exhaustiveness;
