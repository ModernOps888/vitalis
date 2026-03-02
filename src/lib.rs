//! Vitalis v20.0 — A JIT-compiled language with impl blocks, try/catch, closures with capture,
//! stdlib functions (strings, file I/O, maps, error handling, JSON, system), built-in
//! code evolution, multi-domain algorithm libraries, native Cranelift JIT performance,
//! and a full deep learning engine (tensors, transformers, GPU compute, BPE, training).
//!
//! Enterprise-grade release with 47 modules spanning quantum computing, bioinformatics,
//! neuromorphic computation, advanced evolutionary algorithms, physical sciences,
//! and a production-quality ML training pipeline (Nova ML Engine).
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
//!
//! # Module Domains (v20.0 — 47 modules)
//! - **Core Compiler**: lexer, ast, parser, types, ir, codegen, stdlib
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
//! - **Nova ML Engine**: tensor_engine, deep_learning, gpu_compute, ml_training, bpe_tokenizer, model_inference

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

// ── Nova ML Engine (v20.0) ────────────────────────────────────────────
pub mod tensor_engine;
pub mod deep_learning;
pub mod gpu_compute;
pub mod ml_training;
pub mod bpe_tokenizer;
pub mod model_inference;

// ── FFI Bridge ───────────────────────────────────────────────────────
pub mod bridge;
