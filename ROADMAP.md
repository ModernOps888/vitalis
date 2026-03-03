# Vitalis Roadmap

This document tracks the development roadmap for the Vitalis programming language.
Completed milestones are marked with ✅, in-progress with 🔄, and planned with 📋.

---

## ✅ Completed

### v1.0 — Foundation
- ✅ Lexer with Logos tokenizer (~70 token variants)
- ✅ Recursive-descent + Pratt parser → AST (30+ expression types)
- ✅ Two-pass type checker with scope chains
- ✅ SSA-form intermediate representation
- ✅ Cranelift 0.116 JIT backend
- ✅ CLI binary (`vtc`) with subcommands
- ✅ 97 stdlib functions

### v5.0 — Type System
- ✅ i64, f64, bool, str type support
- ✅ Heap-allocated arrays
- ✅ SSA IR builder with ~30 instruction variants

### v7.0–v9.0 — Algorithm Libraries
- ✅ Signal processing, cryptography, graph algorithms
- ✅ String algorithms, numerical methods, compression
- ✅ Probability & statistics, quantum simulator
- ✅ Advanced math, science, analytics, security, scoring

### v10.0 — Machine Learning & Optimization
- ✅ ML (k-means, KNN, PCA, DBSCAN)
- ✅ Computational geometry (convex hull, Voronoi)
- ✅ Sorting algorithms, automata & tries
- ✅ Combinatorial optimization (knapsack, TSP, simplex)

### v13.0 — Quantum, Bio & Neuromorphic
- ✅ Quantum algorithms (Grover, Shor, QFT, VQE)
- ✅ Bioinformatics (DNA/RNA, alignment, epidemiology)
- ✅ Neuromorphic computing (LIF, STDP, ESN, NEAT)
- ✅ Advanced chemistry & molecular dynamics
- ✅ Advanced evolutionary computation (DE, PSO, CMA-ES, NSGA-II)

### v15.0 — Language Power
- ✅ Closures & lambda expressions with capture
- ✅ File I/O, maps, JSON support
- ✅ Error handling system
- ✅ Evolution engine with `@evolvable`

### v19.0 — General Purpose
- ✅ Structs + impl blocks + method dispatch
- ✅ Try/catch/throw error handling
- ✅ Sets, tuples, regex
- ✅ Module system with namespaces
- ✅ HTTP networking + async stubs
- ✅ Iterator protocol + comprehensions

### v20.0 — Trait System & Type Power
- ✅ Trait definitions + trait methods
- ✅ Type aliases, cast expressions
- ✅ Enum definitions with variant indexing
- ✅ Method registry for impl dispatch
- ✅ 741 tests passing

### v21.0 — Async, Generics, WASM & GPU
- ✅ Full async/await runtime (executor, channels, futures)
- ✅ Generics + type parameters + monomorphization
- ✅ Package manager + registry + dependency resolver
- ✅ LSP server + IDE support (diagnostics, completion, hover)
- ✅ WebAssembly target (module builder, LEB128, sections)
- ✅ GPU compute backend (buffers, kernels, pipelines, shaders)
- ✅ 870 tests · 47 modules · 35,856 LOC

### v22.0 — Borrow Checker, DAP, REPL & AOT
- ✅ Ownership & borrow checker (move tracking, scope analysis)
- ✅ Incremental compilation (hash caching, dep graph, topo sort)
- ✅ Full trait dispatch with vtables + method resolution
- ✅ Debug Adapter Protocol (breakpoints, stack, variables, stepping)
- ✅ Interactive REPL (eval, commands, history)
- ✅ Lifetime annotations + region-based memory analysis
- ✅ Effect system + capability types + algebraic effects
- ✅ Incremental codegen + hot-reload with file watching
- ✅ Self-hosted compiler bootstrap (Stage 0/1/2 pipeline)
- ✅ Native AOT compilation (standalone executables)
- ✅ Cross-compilation targets (x86-64, AArch64, RISC-V)
- ✅ 1,043 tests · 58 modules · 41,772 LOC

### v23.0 — Non-Lexical Lifetimes (Current Release)
- ✅ NLL borrow analysis with CFG-based liveness
- ✅ Control-flow graph builder from AST
- ✅ Backward dataflow liveness analysis (live_in/live_out)
- ✅ NLL regions as sets of CFG points (not lexical scopes)
- ✅ Borrow conflict detection via overlapping live ranges
- ✅ Modify-while-borrowed checks
- ✅ 1,087 tests · 59 modules · 42,500+ LOC

---

## 📋 Planned

### v24.0 — Effect Handlers & Pattern Exhaustiveness
- 📋 Effect handlers with resumptions (algebraic effect handler blocks)
- 📋 Pattern matching exhaustiveness checking
- 📋 `handle` blocks for capturing and resuming effects
- 📋 Exhaustiveness warnings for non-total match expressions
- 📋 Wildcard patterns and or-patterns support

### v25.0 — WASM AOT & Code Formatter
- 📋 WASM AOT target (compile `.sl` → standalone `.wasm` files)
- 📋 Code formatter for `.sl` files (`vtc fmt`)
- 📋 WASM-WASI support for file I/O in WebAssembly
- 📋 Formatter configuration (indent style, line width)

### v26.0 — Distributed Compilation
- 📋 Distributed compilation across nodes
- 📋 Build server protocol for remote compilation
- 📋 Parallel translation units
- 📋 Shared compilation cache across machines

### v27.0 — Package Registry & Ecosystem
- 📋 Package registry server (`vitalis install`)
- 📋 Online package search and publishing
- 📋 Dependency vulnerability scanning
- 📋 Documentation generator for `.sl` files (`vtc doc`)

### v28.0 — Hardware Validation & Optimization
- 📋 ARM/RISC-V hardware validation on real devices
- 📋 Profile-guided optimization (PGO) for JIT
- 📋 Auto-vectorization via SIMD intrinsics
- 📋 Memory pool allocator for reduced GC pressure

### v29.0+ — Research Frontier
- 📋 Gradual typing with refinement types
- 📋 Dependent type support (lightweight)
- 📋 Verified compilation passes (proof-carrying code)
- 📋 Multi-language FFI (C, C++, Python, JS)
- 📋 GPU shader language subset (compute shaders from `.sl`)
- 📋 Self-evolving optimizer passes via evolution engine

---

## Version History

| Version | Date | Modules | Tests | LOC | Key Feature |
|---------|------|---------|-------|-----|-------------|
| v0.1.0 | 2025-03-01 | 17 | 234 | ~13,500 | Initial compiler pipeline |
| v9.0.0 | 2025-03-01 | 31 | 470 | ~24,769 | 14 algorithm libraries |
| v10.0.0 | 2025-04-15 | 36 | ~550 | ~27,000 | ML, geometry, automata |
| v13.0.0 | 2025-05-01 | 41 | ~650 | ~30,000 | Quantum, bio, neuromorphic |
| v15.0.0 | 2025-05-20 | 41 | ~650 | ~31,000 | Closures, error handling |
| v19.0.0 | 2025-06-10 | 41 | ~650 | ~32,000 | Structs, modules, HTTP |
| v20.0.0 | 2025-06-20 | 41 | 741 | ~32,500 | Traits, type aliases, enums |
| v21.0.0 | 2025-07-05 | 47 | 870 | ~35,856 | Async, generics, WASM, GPU |
| v22.0.0 | 2025-07-19 | 58 | 1,043 | ~41,772 | Borrow checker, DAP, AOT |
| v23.0.0 | 2025-07-26 | 59 | 1,087 | ~42,500 | Non-Lexical Lifetimes |
