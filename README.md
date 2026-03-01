<div align="center">

# ⚡ Vitalis

**A JIT-compiled programming language with built-in code evolution**

[![Rust](https://img.shields.io/badge/Rust-Edition%202024-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue)](LICENSE-MIT)
[![Cranelift](https://img.shields.io/badge/Backend-Cranelift%200.116-green)](https://cranelift.dev/)
[![Tests](https://img.shields.io/badge/Tests-470%20passing-brightgreen)](.)
[![LOC](https://img.shields.io/badge/Rust%20LOC-24%2C769-informational)](.)
[![CI](https://github.com/ModernOps888/vitalis/actions/workflows/ci.yml/badge.svg)](https://github.com/ModernOps888/vitalis/actions/workflows/ci.yml)

*A language where functions can evolve themselves at runtime.*

**v9.0** — 31 source files · 24,769 LOC · 470 tests · 14 algorithm libraries · 304 Python exports

</div>

---

## What is Vitalis?

Vitalis is a compiled programming language built in Rust that compiles to native machine code via Cranelift JIT. What makes it unique is the **`@evolvable` annotation** — functions marked with it can be mutated, fitness-scored, and rolled back at runtime, enabling autonomous code evolution.

```rust
// hello.sl
fn main() -> i64 {
    println("Hello from Vitalis!");
    42
}
```

```rust
// evolution.sl
@evolvable
fn score(x: i64) -> i64 {
    x * 2 + 1
}

fn main() -> i64 {
    score(21)
}
```

## Features

### Compiler Pipeline
```
Source (.sl) → Lexer → Parser → AST → TypeChecker → IR → Cranelift JIT → Native
```

- **Zero-copy lexer** (Logos) with ~70 token variants
- **Recursive-descent + Pratt parser** with operator precedence
- **Two-pass type checker** with scope chains
- **SSA-form IR** lowering
- **Cranelift 0.116 JIT** backend — compiles to native x86-64

### Language Features
- Static typing with type inference
- Structs, enums, pattern matching
- Pipe operator (`|>`)
- `@evolvable` annotations for runtime code evolution
- 97 built-in stdlib functions (math, ML activations, string ops)
- Evolution keywords: `evolve`, `fitness`, `mutation`, `rollback`, `recall`, `memorize`

### Native Performance — 7.5x Faster Than Python (29.1x Peak)

Rust-implemented operations exposed via C FFI. Benchmarked with **100K elements, 500 iterations, pre-allocated arrays (zero marshalling overhead)**:

| Operation | Python | Vitalis | Speedup |
|-----------|--------|---------|---------|
| Cosine Distance (100K) | 13,921ms | 479ms | **29.1x** |
| Batch ReLU (100K) | 6,308ms | 613ms | **10.3x** |
| Std Deviation (100K) | 7,183ms | 780ms | **9.2x** |
| MSE Loss (100K) | 3,645ms | 450ms | **8.1x** |
| Batch Sigmoid (100K) | 5,173ms | 686ms | **7.5x** |
| Huber Loss (100K) | 6,524ms | 929ms | **7.0x** |
| Sliding Window (100K) | 3,505ms | 641ms | **5.5x** |
| L2 Normalize (100K) | 4,407ms | 820ms | **5.4x** |
| Softmax (10K) | 712ms | 160ms | **4.5x** |
| Layer Norm (100K) | 9,050ms | 2,313ms | **3.9x** |
| Batch Norm (100K) | 9,115ms | 2,414ms | **3.8x** |
| GELU Batch (100K) | 5,458ms | 2,626ms | **2.1x** |
| Mean (100K) | 327ms | 204ms | **1.6x** |
| **TOTAL** | **75,327ms** | **13,116ms** | **5.7x** |

> **Vitalis wins 13/13 benchmarks** — Average 7.5x faster, Peak 29.1x (Cosine Distance)

### 44 Hot-Path Native Ops

| Category | Operations |
|----------|----------|
| **Rate Limiting** | Sliding window, token bucket |
| **Statistics** | P95, percentile, mean, median, stddev, entropy |
| **ML Activations** | Softmax, sigmoid, ReLU, GELU, batch norm, layer norm |
| **Loss Functions** | Cross-entropy, MSE, Huber, KL divergence |
| **Vector Ops** | Cosine similarity/distance, L2 normalize, hamming distance |
| **Optimization** | Bayesian UCB, simulated annealing, Boltzmann selection, CMA-ES |
| **Analysis** | Code quality scoring, cognitive complexity, vote tallying |

### 14 Algorithm Libraries (269 FFI Functions)

| Module | Highlights | LOC |
|--------|-----------|-----|
| **Signal Processing** | FFT/IFFT, convolution, FIR/IIR filters, windowing, spectral analysis | 550 |
| **Cryptography** | SHA-256, HMAC, Base64, CRC-32, FNV-1a, XorShift PRNG | 440 |
| **Graph Algorithms** | BFS, DFS, Dijkstra, PageRank, Tarjan SCC, toposort, bipartite | 789 |
| **String Algorithms** | Levenshtein, Jaro-Winkler, Soundex, KMP, Rabin-Karp, Boyer-Moore-Horspool | 574 |
| **Numerical** | Matrix ops (det, inverse, multiply), eigenvalues, integration, root finding | 709 |
| **Compression** | RLE, Huffman, delta coding, LZ77 | 532 |
| **Statistics** | Distributions, Pearson correlation, regression, chi-squared, K-S test | 653 |
| **Quantum Simulator** | Statevector register, H/X/Y/Z/CNOT/RX/RY/RZ gates, QFT, Bell states, Bloch | 813 |
| **Quantum Math** | Gamma, Bessel, Riemann zeta, quaternions, Haar wavelets, Runge-Kutta RK4 | 1,004 |
| **Advanced Math** | Factorial, erf, Mandelbrot, Catalan numbers, Bell numbers, binomial | 943 |
| **Science** | 50+ physics/chemistry/astro formulas + physical constants | 504 |
| **Analytics** | SMA/EMA/WMA/DEMA, anomaly detection (z-score), SES/Holt forecasting, Apdex | 662 |
| **Security** | Input validation, SQL/XSS/path/command injection detection, password scoring | 421 |
| **Scoring** | Halstead metrics, Elo rating, Pareto ranking, Bayesian A/B testing, Cohen's d | 470 |

### Python Integration
```python
import vitalis

# Compile and run
result = vitalis.compile_and_run("fn main() -> i64 { 42 }")
assert result == 42

# Evolution
vitalis.evo_register("score", "@evolvable fn score(x: i64) -> i64 { x * 2 }")
vitalis.evo_evolve("score", "@evolvable fn score(x: i64) -> i64 { x * 3 }")
vitalis.evo_set_fitness("score", 0.95)

# Native hot-path ops (7.5x faster than Python)
p95 = vitalis.hotpath_p95([1.0, 2.0, 3.0, ..., 100.0])
scores = vitalis.hotpath_softmax([1.0, 2.0, 3.0])

# v9.0: Quantum simulator
q = vitalis.QuantumRegister(2)
q.h(0).cnot(0, 1)  # Bell state
print(q.prob(0))    # 0.5

# v9.0: Science, crypto, graph, stats...
print(vitalis.sha256("hello"))          # SHA-256 hash
print(vitalis.fibonacci(30))            # 832040
print(vitalis.physical_constant("c"))   # 299792458.0
print(vitalis.levenshtein("kitten", "sitting"))  # 3
print(vitalis.is_prime(999999937))      # True
```

## Quick Start

### Prerequisites
- Rust (stable, Edition 2024): https://rustup.rs/

### Build
```bash
git clone https://github.com/ModernOps888/vitalis.git
cd vitalis
cargo build
```

### Run
```bash
# Run a .sl file
cargo run -- run examples/hello.sl

# Evaluate an expression
cargo run -- eval -e "21 * 2"

# Type-check without executing
cargo run -- check examples/arithmetic.sl

# Dump AST
cargo run -- dump-ast examples/structs.sl

# Dump IR
cargo run -- dump-ir examples/arithmetic.sl

# Lex tokens
cargo run -- lex examples/hello.sl
```

### Test
```bash
cargo test
```

### Python Usage
```bash
cargo build  # produces vitalis.dll / libvitalis.so / libvitalis.dylib
cp python/vitalis.py .
python -c "import vitalis; print(vitalis.compile_and_run('fn main() -> i64 { 42 }'))"
```

## Project Structure

```
vitalis/
├── Cargo.toml              # Build configuration (v9.0.0)
├── src/
│   ├── main.rs             # CLI binary (vtc)
│   ├── lib.rs              # Library root (28 modules)
│   │
│   │  # Core Compiler
│   ├── lexer.rs            # Logos-based tokenizer
│   ├── parser.rs           # Recursive-descent parser
│   ├── ast.rs              # AST node definitions
│   ├── types.rs            # Type checker
│   ├── ir.rs               # IR lowering
│   ├── codegen.rs          # Cranelift JIT codegen
│   ├── stdlib.rs           # 97 built-in functions
│   │
│   │  # Evolution & Performance
│   ├── evolution.rs        # @evolvable function registry
│   ├── engine.rs           # Evolution cycle runner
│   ├── meta_evolution.rs   # Strategy evolution
│   ├── memory.rs           # Engram-based memory store
│   ├── hotpath.rs          # 44 native fast-path ops
│   ├── optimizer.rs        # Optimization passes
│   ├── simd_ops.rs         # SIMD-optimized operations
│   │
│   │  # v7.0 Algorithm Libraries
│   ├── signal_processing.rs  # FFT, filters, windowing
│   ├── crypto.rs             # SHA-256, HMAC, Base64, CRC
│   ├── graph.rs              # BFS, Dijkstra, PageRank, SCC
│   ├── string_algorithms.rs  # Levenshtein, KMP, Soundex
│   ├── numerical.rs          # Linear algebra, integration
│   ├── compression.rs        # RLE, Huffman, LZ77
│   ├── probability.rs        # Distributions, correlation
│   │
│   │  # v9.0 Advanced Modules
│   ├── quantum.rs            # Quantum circuit simulator
│   ├── quantum_math.rs       # Gamma, Bessel, quaternions
│   ├── advanced_math.rs      # Factorial, erf, Mandelbrot
│   ├── science.rs            # 50 physics/chemistry formulas
│   ├── analytics.rs          # Time series, anomaly detection
│   ├── security.rs           # Validation, injection detection
│   ├── scoring.rs            # Halstead, Elo, A/B testing
│   │
│   └── bridge.rs             # C FFI exports (405 functions)
├── examples/                 # Example .sl programs (8 demos)
├── python/
│   └── vitalis.py            # Python wrapper (3,036 LOC, 304 exports)
├── docs/
│   ├── LANGUAGE_GUIDE.md     # Complete language reference
│   └── EXTENDING.md          # Developer extension guide
└── tests/                    # Test suite (470 inline tests)
```

## CI/CD

GitHub Actions CI runs on every push and PR:
- **Test** — Cross-platform matrix (Ubuntu, Windows, macOS) with `cargo build` + `cargo test`
- **Lint** — `cargo fmt --check` + `cargo clippy -D warnings`
- **Docs** — `cargo doc --no-deps` with strict warnings

## Language Syntax

### Types
```rust
let x: i64 = 42;
let y: f64 = 3.14;
let flag: bool = true;
let name: string = "hello";
```

### Functions
```rust
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn main() -> i64 {
    add(20, 22)
}
```

### Control Flow
```rust
fn abs(x: i64) -> i64 {
    if x < 0 { -x } else { x }
}
```

### Pipe Operator
```rust
fn double(x: i64) -> i64 { x * 2 }
fn inc(x: i64) -> i64 { x + 1 }

fn main() -> i64 {
    5 |> double |> inc  // 11
}
```

### Structs
```rust
struct Point {
    x: i64,
    y: i64,
}

fn main() -> i64 {
    let p = Point { x: 3, y: 4 };
    p.x + p.y
}
```

### Evolution
```rust
@evolvable
fn strategy(input: i64) -> i64 {
    input * 2
}
```

## Benchmarks

### Hot-Path Ops vs Python (100K elements, 500 iterations, zero marshalling)

| Operation | Vitalis | Python | Speedup |
|-----------|---------|--------|--------|
| Cosine Distance | 479ms | 13,921ms | **29.1x** |
| Batch ReLU | 613ms | 6,308ms | **10.3x** |
| Std Deviation | 780ms | 7,183ms | **9.2x** |
| MSE Loss | 450ms | 3,645ms | **8.1x** |
| Batch Sigmoid | 686ms | 5,173ms | **7.5x** |
| Huber Loss | 929ms | 6,524ms | **7.0x** |
| Sliding Window | 641ms | 3,505ms | **5.5x** |
| L2 Normalize | 820ms | 4,407ms | **5.4x** |
| Softmax | 160ms | 712ms | **4.5x** |
| **Average** | | | **7.5x** |

### v9.0 Module Benchmarks (74 benchmarks via Python FFI)

| Category | Avg Throughput | Peak Function | Peak |
|----------|---------------|---------------|------|
| Science | **1.59M** ops/sec | `schwarzschild_radius` | 2.25M ops/sec |
| Advanced Math | **1.24M** ops/sec | `math_erf` | 2.09M ops/sec |
| Quantum Math | **1.15M** ops/sec | `golden_ratio` | 3.43M ops/sec |
| Scoring | **1.14M** ops/sec | `elo_expected` | 1.64M ops/sec |
| Statistics | **525.7K** ops/sec | `normal_pdf` | 1.37M ops/sec |
| String Algorithms | **314.8K** ops/sec | `jaro_winkler` | 465.1K ops/sec |
| Numerical | **298.5K** ops/sec | `horner` | 593.6K ops/sec |
| Analytics | **278.0K** ops/sec | `apdex` | 1.37M ops/sec |
| Quantum Simulator | **254.5K** ops/sec | `bell_state` | 306.5K ops/sec |
| Security | **151.9K** ops/sec | `password_entropy` | 293.2K ops/sec |
| Compression | **118.0K** ops/sec | `rle_encode` | 283.8K ops/sec |
| Crypto | **92.1K** ops/sec | `fnv1a_64` | 167.0K ops/sec |
| Signal Processing | **22.3K** ops/sec | `rms_energy` | 30.7K ops/sec |
| Core Compiler | **18.6K** ops/sec | `lex` | 46.3K ops/sec |
| Graph Algorithms | **10.8K** ops/sec | `is_bipartite` | 13.0K ops/sec |

> All measurements include Python→Rust FFI overhead. Pure Rust throughput is higher.
> 470 tests passing, 0 failures. Benchmark suite: 74 functions across 15 categories.

## v0.1 → v9.0 Growth

| Metric | v0.1.0 | v9.0.0 | Growth |
|--------|-------:|-------:|-------:|
| Source files | 17 | 31 | **+82%** |
| Rust LOC | ~13,500 | 24,769 | **+83%** |
| Tests | 234 | 470 | **+101%** |
| FFI exports | ~50 | 405 | **+710%** |
| Python exports | ~40 | 304 | **+660%** |
| Python wrapper LOC | 930 | 3,036 | **+226%** |
| Algorithm modules | 0 | 14 | **new** |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Areas where help is welcome:
- Language features (closures, traits, generics)
- Platform support (Linux/macOS CI)
- Editor support (VS Code extension, syntax highlighting)
- Documentation and tutorials
- Package manager for `.sl` dependencies

## Security

See [SECURITY.md](SECURITY.md) for responsible disclosure policy.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

Built with [Cranelift](https://cranelift.dev/) for JIT compilation, [Logos](https://github.com/maciejhirsz/logos) for lexing, and [Clap](https://github.com/clap-rs/clap) for CLI.
