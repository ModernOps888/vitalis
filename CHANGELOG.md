# Changelog

All notable changes to Vitalis will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [25.0.0] - 2026-06-06

### Added

#### Code Formatter
- **`formatter.rs`** (640+ LOC, 33 tests) — AST-based code formatter/pretty-printer
  - **FormatConfig**: Configurable indent size, max line width, trailing commas, brace style
  - **Complete AST Coverage**: Every node type (functions, structs, enums, modules, imports, extern blocks, impl blocks, traits, type aliases, annotations) has a formatting rule
  - **Expression Formatting**: All 30+ Expr variants formatted (binary ops, calls, method calls, if/else, match, lambdas, pipes, try/catch, parallel, ranges, casts, struct literals)
  - **Statement Formatting**: Let bindings, while/for/loop, expression statements
  - **Pattern Formatting**: Literal, ident, variant, wildcard, struct, or, tuple patterns
  - **Import Sorting**: Optionally sorts imports alphabetically to the top of the file
  - **Idempotency**: Formatting already-formatted code produces identical output
  - **Convenience API**: `format_source()`, `format_source_with_config()`, `check_formatted()`

#### Static Linter
- **`linter.rs`** (650+ LOC, 30 tests) — Configurable static analysis with 17 lint rules
  - **UnusedVariable**: Detects variables declared but never referenced (respects `_` prefix)
  - **UnusedFunction**: Finds functions defined but never called
  - **UnusedImport**: Identifies import statements with no usage
  - **ShadowedVariable**: Warns when a variable shadows an outer binding
  - **DeadCode**: Detects unreachable code after return/break/continue
  - **EmptyBlock**: Flags blocks with no statements or expressions
  - **NamingConvention**: Enforces snake_case for variables/functions, PascalCase for types
  - **MissingReturnType**: Warns on functions without explicit return type
  - **LargeFunction**: Flags functions exceeding configurable line threshold
  - **DeepNesting**: Warns when nesting depth exceeds threshold
  - **MagicNumber**: Detects unnamed numeric literals (excludes 0, 1, 2, -1)
  - **EmptyMatchArm**: Flags match arms with empty bodies
  - **UnusedParameter**: Detects function parameters never used
  - **BoolComparison**: Warns on unnecessary `== true` / `== false` comparisons
  - **RedundantReturn**: Identifies explicit return at tail position
  - **LintConfig**: Enable/disable/suppress individual rules, configure thresholds
  - **Convenience API**: `lint_source()`, `lint_source_with_config()`

#### Refinement Types
- **`refinement_types.rs`** (750+ LOC, 44 tests) — Refinement/dependent type system with constraint solver
  - **RefinedType**: Base type + binder variable + logical predicate (`{ v: i64 | v > 0 }`)
  - **Predicate Language**: True, False, Var, IntConst, FloatConst, BoolConst, Compare, And, Or, Not, Implies, Arith, App
  - **ConstraintSolver**: Bounds-based satisfiability checking and entailment reasoning
  - **Subtype Checking**: Verifies refined subtype relationships (e.g., Positive <: Natural)
  - **Variable Bounds**: Tracks lower/upper bounds and not-equal constraints per variable
  - **Built-in Refinements**: Positive, Natural, NonZero, Percentage, UnitInterval, Byte
  - **RefinementRegistry**: Named refinement types with custom registration
  - **Predicate Operations**: Substitution, negation, Display formatting
  - **Comparison & Arithmetic Ops**: Full CmpOp (==, !=, <, <=, >, >=) and ArithOp (+, -, *, /, %) support

#### AST Enhancement
- **Span derives Copy**: `Span` now derives `Copy` for zero-cost pass-by-value (it only holds two `usize` fields)

### Changed
- Version bumped from 24.0.0 → 25.0.0
- Module count: 61 → 64 (formatter, linter, refinement_types)
- Test count: 1,177 → 1,284 (+107 new tests)
- LOC: ~45,703 → ~47,743 (~2,040 new lines)

## [24.0.0] - 2026-03-03

### Added

#### Algebraic Effect Handlers
- **`effect_handlers.rs`** (1,245+ LOC, 39 tests) — Full algebraic effect handler system
  - **Effect Declarations**: Define effects with named operation signatures (args + return types)
  - **Handler Blocks**: `handle { body } with { Effect::op(args) => resume(val) }` syntax representation
  - **Continuations**: `Resume(value)` and `Abort(value)` for controlling suspended computations
  - **Handler Stack**: Nested handler frames with LIFO dispatch — inner handlers shadow outer
  - **Effect Dispatcher**: Resolves `perform Effect::op(args)` through the handler chain
  - **Handler Composition**: Combine multiple handlers, layer handlers with fallback chains
  - **Validation**: Duplicate-effect detection, unhandled-effect checking, arity verification
  - **Convenience API**: `validate_handler()`, `check_unhandled_effects()`, `compose_handlers()`

#### Pattern Exhaustiveness Checking
- **`pattern_exhaustiveness.rs`** (1,345+ LOC, 51 tests) — Matrix-based Maranget usefulness algorithm
  - **Exhaustiveness Analysis**: Detects non-exhaustive match expressions with missing pattern suggestions
  - **Redundancy Detection**: Identifies unreachable/redundant match arms
  - **Or-Patterns**: `A | B` disjunctive patterns with automatic expansion in the pattern matrix
  - **Guard Clauses**: Guarded arms treated as potentially non-matching for soundness
  - **Nested Destructuring**: Struct, enum, and tuple patterns with field-level analysis
  - **Type Descriptors**: Bool, enum, integer, string, struct, tuple, option, result type shapes
  - **Constructor Coverage**: Tracks which constructors are matched, suggests missing ones
  - **Diagnostics**: `ExhaustivenessWarning` and `RedundancyWarning` with source spans and descriptions

#### AST Extensions
- **Pattern::Or**: Or-pattern `A | B` — matches if any sub-pattern matches
- **Pattern::Tuple**: Tuple destructuring `(a, b, c)` in match arms
- **Expr::Handle**: Handle expression node for `handle { body } with { handlers }`

### Changed
- Version bumped from 23.0.0 → 24.0.0
- `lib.rs` expanded from 59 → 61 public modules
- `ast.rs` extended with Or, Tuple pattern variants and Handle expression
- `ir.rs` updated to handle new pattern variants in IR lowering

### Metrics
| Metric | v23.0.0 | v24.0.0 | Delta |
|--------|---------|---------|-------|
| Rust source files | 59 | 61 | +2 |
| Rust LOC (total) | ~43,095 | ~45,703 | +2,608 |
| Test cases | 1,087 | 1,177 | +90 |

---

## [23.0.0] - 2025-07-26

### Added

#### Non-Lexical Lifetimes (NLL)
- **`nll.rs`** (750+ LOC, 44 tests) — Full NLL borrow analysis engine
  - **CFG Builder**: Constructs control-flow graphs from AST functions with Entry/Exit nodes, Branch/Join for conditionals, LoopHeader/LoopBack for loops, Call nodes, Assignment tracking
  - **Liveness Analysis**: Iterative backward dataflow (live_in/live_out per CFG node) computing variable liveness at every program point
  - **NLL Regions**: Borrow regions represented as sets of CFG points (not lexical scopes) — borrows end at last use, not at scope exit
  - **Conflict Detection**: Overlapping mutable/shared region detection, modify-while-borrowed checks
  - **Convenience API**: `analyze_nll()`, `build_cfg_from_source()`, `compute_liveness_from_source()` for tooling integration

### Changed
- Version bumped from 22.0.0 → 23.0.0
- `lib.rs` expanded from 58 → 59 public modules
- `lib.rs` doc comment updated to v23.0 with NLL module domain
- `bridge.rs` version string now returns "23.0.0"

### Metrics
| Metric | v22.0.0 | v23.0.0 | Delta |
|--------|---------|---------|-------|
| Rust source files | 58 | 59 | +1 |
| Rust LOC (total) | ~41,772 | ~43,095 | +1,323 |
| Test cases | 1,043 | 1,087 | +44 |

---

## [22.0.0] - 2025-07-19

### Added

#### Borrow Checker & Ownership Analysis
- **`ownership.rs`** (422 LOC, 20 tests) — Phase-1 borrow checker with lexical scope analysis
  - Ownership states: Owned, Moved, BorrowedShared, BorrowedMut, Dropped, Undefined
  - Use-after-move detection, double-drop prevention, mutable aliasing checks
  - Full AST walk with scope push/pop, variable declaration/lookup

#### Incremental Compilation & Caching
- **`incremental.rs`** — Hash-based incremental compilation with dependency graph
  - Content hashing, cache state management, topological sort invalidation

#### Full Trait Dispatch with VTables
- **`trait_dispatch.rs`** — Dynamic dispatch via vtable construction and method resolution

#### Debug Adapter Protocol (DAP)
- **`dap.rs`** — Full DAP implementation: breakpoints, stepping, variable inspection, stack frames

#### Interactive REPL
- **`repl.rs`** — REPL with `:help`, `:ast`, `:ir`, `:type`, `:quit` commands and history

#### Lifetime Annotations & Region Analysis
- **`lifetimes.rs`** (955 LOC, 10 tests) — Region-based lifetime analysis
  - Region variables, scope-depth tracking, outlives constraints, equality constraints
  - Constraint solving with fixed-point iteration, cycle detection in outlives graph
  - Borrow record tracking, mutable aliasing detection, scope-based cleanup
  - Program-level LifetimeChecker operating on AST functions and impl blocks

#### Effect System & Capability Types
- **`effects.rs`** (734 LOC, 15+ tests) — Static effect system
  - 12 effect types: IO, Net, FileSystem, Async, Unsafe, GPU, Evolve, System, NonDet, Alloc, Exception, Custom
  - Capability tokens with attenuation, effect propagation through call chains
  - Pure function enforcement, effect checker with compliance verification

#### Hot-Reload Engine
- **`hot_reload.rs`** — File watching with incremental recompilation and live function swap

#### Self-Hosted Compiler Bootstrap
- **`bootstrap.rs`** — Stage 0 (Rust) → Stage 1 (.sl) → Stage 2 (self-compiled) pipeline with cross-validation

#### Native AOT Compilation
- **`aot.rs`** — Cranelift ObjectModule backend for ahead-of-time compilation to standalone executables

#### Cross-Compilation Targets
- **`cross_compile.rs`** — x86-64, AArch64, RISC-V targets with ISA features and ABI lowering

### Metrics
| Metric | v21.0.0 | v22.0.0 | Delta |
|--------|---------|---------|-------|
| Rust source files | 47 | 58 | +11 |
| Rust LOC (total) | ~35,856 | ~41,772 | +5,916 |
| Test cases | 870 | 1,043 | +173 |

---

## [21.0.0] - 2025-07-05

### Added

#### Async/Await Runtime
- **`async_runtime.rs`** — Cooperative async runtime with TaskId, round-robin executor, channels, futures

#### Generics & Type Parameters
- **`generics.rs`** — Generic functions/structs, monomorphization, type inference, bounds checking

#### Package Manager & Registry
- **`package_manager.rs`** — SemVer resolution, lockfiles, registry client, dependency resolver

#### LSP Server & IDE Support
- **`lsp.rs`** — Language Server Protocol: diagnostics, hover, go-to-definition, completion, signature help, workspace symbols

#### WebAssembly Target
- **`wasm_target.rs`** — WASM module builder with LEB128 encoding, section generation, validation

#### GPU Compute Backend
- **`gpu_compute.rs`** — GPU compute buffers, kernels, pipelines, shader builder

### Metrics
| Metric | v20.0.0 | v21.0.0 | Delta |
|--------|---------|---------|-------|
| Rust source files | 41 | 47 | +6 |
| Rust LOC (total) | ~32,500 | ~35,856 | +3,356 |
| Test cases | 741 | 870 | +129 |

---

## [20.0.0] - 2025-06-20

### Added
- Trait definitions with method signatures
- Type aliases (`type Name = Type`)
- Cast expressions (`expr as Type`)
- Enum definitions with variant indexing
- Method registry for impl dispatch
- Bare `self` parameter sugar

### Metrics
| Metric | v19 | v20.0.0 |
|--------|-----|---------|
| Test cases | ~650 | 741 |

---

## [19.0.0] - 2025-06-10

### Added
- Structs with impl blocks and method dispatch
- Try/catch/throw error handling
- Sets, tuples, and regex support
- Module system with namespaces and imports
- HTTP networking and async stubs
- Iterator protocol and comprehensions

---

## [15.0.0] - 2025-05-20

### Added
- Closures and lambda expressions with capture
- File I/O operations, maps, and JSON support
- Full error handling system
- Evolution engine with `@evolvable` annotation
- 46 new stdlib functions

---

## [13.0.0] - 2025-05-01

### Added
- **`quantum_algorithms.rs`** — Grover's search, Shor's algorithm, QFT, VQE, QAOA, QPE
- **`bioinformatics.rs`** — DNA/RNA analysis, sequence alignment, epidemiology, kinetics
- **`chemistry_advanced.rs`** — Molecular dynamics, statistical mechanics
- **`neuromorphic.rs`** — Spiking neural networks (LIF, Izhikevich), STDP, ESN, NEAT
- **`evolution_advanced.rs`** — DE, PSO, CMA-ES, NSGA-II, Novelty Search, MAP-Elites, Island Model

---

## [10.0.0] - 2025-04-15

### Added
- **`ml.rs`** — K-means, KNN, Naive Bayes, PCA, DBSCAN, LDA, neural networks
- **`geometry.rs`** — Convex hull, Voronoi diagrams, Welzl algorithm, triangulation
- **`sorting.rs`** — Parallel quicksort, mergesort, radixsort, binary search
- **`automata.rs`** — Aho-Corasick, Bloom filters, tries, regex engines
- **`combinatorial.rs`** — Knapsack, TSP, simplex, genetic algorithms, graph coloring

## [0.1.0] - 2025-03-01

### Added

#### Compiler Pipeline
- Zero-copy Logos-based lexer with ~70 token variants
- Recursive-descent + Pratt parser with operator precedence
- 27 AST expression variants with origin tracking
- Two-pass type checker with scope chains
- SSA-form intermediate representation
- Cranelift 0.116 JIT backend (compiles to native x86-64)
- CLI binary (`vtc`) with subcommands: `run`, `eval`, `check`, `dump-ast`, `dump-ir`, `lex`

#### Language Features
- Static typing with type inference (`i64`, `f64`, `bool`, `string`)
- Functions with parameters and return types
- Structs with field access and construction
- Enums with variant constructors
- Pattern matching
- `if/else` expressions
- `while` and `for` loops
- Pipe operator (`|>`) for function chaining
- `let` bindings with optional type annotations
- String literals and string operations
- Block expressions with implicit returns
- `@evolvable` annotation for runtime code evolution

#### Standard Library (97 built-in functions)
- **I/O**: `print`, `println`, `print_f64`, `println_f64`, `print_bool`, `println_bool`, `print_str`, `println_str`
- **Math (f64)**: `sqrt`, `ln`, `log2`, `log10`, `sin`, `cos`, `exp`, `floor`, `ceil`, `round`, `abs_f64`, `pow`, `min_f64`, `max_f64`
- **Math (i64)**: `abs`, `min`, `max`, `sign`, `gcd`, `lcm`, `factorial`, `fibonacci`, `is_prime`, `ipow`
- **Trigonometry**: `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`
- **Extended math**: `hypot`, `cbrt`, `fma`, `log`, `log1p`, `exp2`, `expm1`, `copysign`, `fract`, `trunc`, `recip`, `rsqrt`, `sinc`, `inv_sqrt_approx`, `logit`
- **AI activations**: `sigmoid`, `relu`, `leaky_relu`, `elu`, `selu`, `celu`, `gelu`, `swish`, `mish`, `softplus`, `softsign`, `hard_sigmoid`, `hard_swish`, `log_sigmoid`, `gaussian`
- **Conversions**: `to_f64`, `to_i64`, `i64_to_f64`, `f64_to_i64`, `deg_to_rad`, `rad_to_deg`
- **String ops**: `str_len`, `str_eq`, `str_cat`
- **Numeric utils**: `lerp`, `smoothstep`, `clamp`, `clamp_f64`, `clamp_i64`, `wrap`, `map_range`, `step`
- **Bitwise**: `popcount`, `leading_zeros`, `trailing_zeros`
- **Random**: `rand_f64`, `rand_i64`
- **Time**: `clock_ns`, `clock_ms`, `epoch_secs`
- **Assert**: `assert_eq`, `assert_true`
- **Hash**: `hash`

#### Hot-Path Native Operations (44 Rust-native ops via C FFI)
- **Rate limiting**: sliding window count/compact, token bucket
- **Statistics**: P95, mean, median, standard deviation, percentile, variance, entropy
- **ML activations**: softmax, sigmoid, ReLU, GELU, batch norm, layer norm
- **Loss functions**: cross-entropy, MSE, Huber loss, KL divergence
- **Vector ops**: cosine similarity, cosine distance, L2 normalize, hamming distance, dot product
- **Optimization**: Bayesian UCB, simulated annealing, Boltzmann selection, CMA-ES step
- **Analysis**: code quality scoring, cognitive complexity, vote tallying (numeric + string)
- **Scoring**: weighted score, fitness scoring

#### Code Evolution System
- `@evolvable` annotation and function registry
- Multi-generation variant tracking with rollback
- Fitness scoring and selection
- Meta-evolution strategies
- Engram-based memory store

#### Python Integration
- `vitalis.py` — full ctypes wrapper for `vitalis.dll` / `libvitalis.so`
- Compile-and-run, type checking, lexing, AST dump, IR dump
- Evolution API (register, evolve, rollback, fitness)
- All 44 hot-path operations callable from Python
- Benchmarked at 7.6x avg / 29.7x peak faster than Python equivalents

#### Infrastructure
- Dual MIT / Apache-2.0 license
- CI pipeline (GitHub Actions) for Linux, Windows, macOS
- 8 example `.sl` programs
- 234 test cases

[0.1.0]: https://github.com/ModernOps888/vitalis/releases/tag/v0.1.0
[9.0.0]: https://github.com/ModernOps888/vitalis/compare/v0.1.0...v9.0.0
[10.0.0]: https://github.com/ModernOps888/vitalis/compare/v9.0.0...v10.0.0
[13.0.0]: https://github.com/ModernOps888/vitalis/compare/v10.0.0...v13.0.0
[15.0.0]: https://github.com/ModernOps888/vitalis/compare/v13.0.0...v15.0.0
[19.0.0]: https://github.com/ModernOps888/vitalis/compare/v15.0.0...v19.0.0
[20.0.0]: https://github.com/ModernOps888/vitalis/compare/v19.0.0...v20.0.0
[21.0.0]: https://github.com/ModernOps888/vitalis/compare/v20.0.0...v21.0.0
[22.0.0]: https://github.com/ModernOps888/vitalis/compare/v21.0.0...v22.0.0
[23.0.0]: https://github.com/ModernOps888/vitalis/compare/v22.0.0...HEAD
