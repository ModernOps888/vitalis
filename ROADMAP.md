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

### v23.0 — Non-Lexical Lifetimes
- ✅ NLL borrow analysis with CFG-based liveness
- ✅ Control-flow graph builder from AST
- ✅ Backward dataflow liveness analysis (live_in/live_out)
- ✅ NLL regions as sets of CFG points (not lexical scopes)
- ✅ Borrow conflict detection via overlapping live ranges
- ✅ Modify-while-borrowed checks
- ✅ 1,087 tests · 59 modules · 43,095 LOC

### v24.0 — Effect Handlers & Pattern Exhaustiveness
- ✅ Algebraic effect handler system with `handle { } with { }` blocks
- ✅ First-class continuations (resume/abort) within effect handlers
- ✅ Handler stack with LIFO dispatch and nested handler frames
- ✅ Handler composition — combine/layer multiple handlers
- ✅ Effect dispatcher resolving `perform` through handler chain
- ✅ Handler validation (duplicate effects, unhandled effects, arity checks)
- ✅ Pattern matching exhaustiveness checker (Maranget usefulness algorithm)
- ✅ Or-patterns (`A | B`), guard clauses, nested destructuring
- ✅ Redundant/unreachable arm detection with diagnostics
- ✅ AST extensions: Or/Tuple patterns, Handle expression
- ✅ 1,177 tests · 61 modules · 45,703 LOC

### v25.0 — Code Formatter, Linter & Refinement Types
- ✅ AST-based code formatter with configurable style
- ✅ Static linter with 17 rules and configurable severity
- ✅ Refinement/dependent types with constraint solver and subtype checking
- ✅ 1,284 tests · 64 modules · 47,743 LOC

### v26.0 — Macro System, Compile-Time Eval & Iterators
- ✅ Hygienic macro system with token trees and derive macros
- ✅ Compile-time evaluation (const fns, static assertions, constant folding)
- ✅ Lazy iterator protocol with 13 adapters and generator→state-machine lowering
- ✅ 1,458 tests · 67 modules · 53,359 LOC

### v27.0 — Structured Concurrency, Type Inference & Documentation
- ✅ Structured concurrency (Mutex, RwLock, channels, Select, WaitGroup, atomics)
- ✅ Hindley-Milner Algorithm W type inference with union/intersection types
- ✅ Documentation generation (doc-comment parser, API model, Markdown/HTML output)
- ✅ 1,586 tests · 70 modules · 57,196 LOC

### v28.0 — Graphics Engine, Shaders, GUI & Creative Coding
- ✅ Software rasterizer with 2D/3D primitives and transformation pipeline
- ✅ Shader language compiler (GLSL/HLSL/Metal/WGSL/SPIR-V backends)
- ✅ Retained-mode GUI framework with layout engine and theming
- ✅ Creative coding toolkit (Perlin noise, particle systems, L-systems)
- ✅ Visual node graph editor for data-flow programming
- ✅ Chart rendering (bar, line, pie, scatter, histogram, heatmap)
- ✅ 1,765 tests · 76 modules · 62,700 LOC

### v29.0 — Profiler, Memory Pools, FFI Bindgen, Type Classes, Build System & Benchmarks
- ✅ Execution profiler with call graphs, flame graphs, PGO hints, hot-path detection
- ✅ Advanced memory allocators (arena, pool, slab, buddy) with RC heap and cycle detection
- ✅ Multi-language FFI bindgen — C headers, TypeScript .d.ts, calling conventions, type marshaling
- ✅ Higher-kinded types, type classes, GADTs, type families, type-level naturals, kind checker
- ✅ Build graph DAG with content-addressed cache (SHA-256), work-stealing scheduler, critical path
- ✅ Micro-benchmarking framework with outlier detection, confidence intervals, regression testing
- ✅ 1,931 tests · 82 modules · ~68,200 LOC

### v30.0 — Regex Engine, Serialization, Property Testing, Data Structures, Networking & ECS (Current Release)
- ✅ Thompson NFA + Pike VM regex engine with O(n·m) guaranteed matching (no backtracking)
- ✅ Character classes, quantifiers (greedy/lazy), anchors, alternation, capturing groups
- ✅ JSON parser/stringify with full spec compliance; Base64, Hex, URL encoding, Varint/LEB128, MessagePack
- ✅ JSON path queries for nested data extraction
- ✅ QuickCheck-style property-based testing with automatic shrinking (Xorshift128+ PRNG, binary search shrink)
- ✅ B-Tree, Skip List, Ring Buffer, Union-Find (path compression + union by rank), Interval Tree, LRU Cache
- ✅ URL parser (RFC 3986), HTTP/1.1 request/response builder & parser, HTTP/2 frame codec
- ✅ WebSocket frame codec (RFC 6455), DNS packet builder/parser (RFC 1035), TCP state machine (RFC 793)
- ✅ IP address validation (IPv4/IPv6)
- ✅ Entity-Component-System with generational entity IDs and sparse set storage (O(1) CRUD)
- ✅ Component queries with With/Without filters, system scheduling with dependency ordering
- ✅ 2,108 tests · 88 modules · ~72,000 LOC

---

## 📋 Planned

### v31.0 — WASM AOT & WASI Runtime
- 📋 WASM AOT target — compile `.sl` → standalone `.wasm` files
- 📋 WASM-WASI support for file I/O and environment access in WebAssembly
- 📋 WASM component model integration for language interop
- 📋 Browser runtime shim for running `.wasm` output in web environments
- 📋 Size optimization passes for WASM output (dead code elimination, tree shaking)

### v32.0 — Package Registry & Ecosystem
- 📋 Package registry server (`vitalis install <package>`)
- 📋 Online package search, publishing, and version management
- 📋 Dependency vulnerability scanning and advisory database
- 📋 Lockfile pinning with reproducible builds
- 📋 Package templating and scaffolding (`vtc new`)

### v33.0 — Distributed Compilation & Remote Build
- 📋 Distributed compilation across networked nodes
- 📋 Build server protocol for remote compilation offloading
- 📋 Shared compilation cache across machines (content-addressed)
- 📋 Build graph visualization and profiling (`vtc build --profile`)
- 📋 Hermetic builds with sandboxed build environments

### v34.0 — Formal Verification & Safety
- 📋 Verified compilation passes (proof-carrying code)
- 📋 Formal verification integration for safety-critical code
- 📋 Contract-based programming (pre/postconditions, invariants)
- 📋 Symbolic execution engine for property checking
- 📋 Certified compiler pass — provably correct optimizations

### v35.0 — Advanced IDE & Tooling
- 📋 LSP v4 features (inlay hints, semantic tokens, call hierarchy)
- 📋 IDE-native debugger integration with watch expressions
- 📋 Profiler integration in IDE (flame graph visualization, hotspot highlighting)
- 📋 Refactoring engine (rename, extract function, inline, move)
- 📋 Code coverage reporting and visualization

### v36.0+ — Research Frontier
- 📋 Self-evolving optimizer passes via evolution engine
- 📋 Auto-vectorization via SIMD intrinsics detection
- 📋 Incremental type checking with demand-driven analysis
- 📋 Effect polymorphism and row-polymorphic effects
- 📋 Algebraic subtyping with polar types
- 📋 Capability-secure module system with object-capability model

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
| v23.0.0 | 2025-07-26 | 59 | 1,087 | ~43,095 | Non-Lexical Lifetimes |
| v24.0.0 | 2026-03-03 | 61 | 1,177 | ~45,703 | Effect handlers, pattern exhaustiveness |
| v25.0.0 | 2026-03-10 | 64 | 1,284 | ~47,743 | Formatter, linter, refinement types |
| v26.0.0 | 2026-03-17 | 67 | 1,458 | ~53,359 | Macros, const eval, iterators |
| v27.0.0 | 2026-03-24 | 70 | 1,586 | ~57,196 | Concurrency, type inference, documentation |
| v28.0.0 | 2026-03-31 | 76 | 1,765 | ~62,700 | Graphics engine, shaders, GUI, creative coding, visual nodes, charts |
| v29.0.0 | 2026-04-07 | 82 | 1,931 | ~68,200 | Profiler, memory pools, FFI bindgen, type classes, build system, benchmarks |
| v30.0.0 | 2026-04-14 | 88 | 2,108 | ~72,000 | Regex engine, serialization, property testing, data structures, networking, ECS |
