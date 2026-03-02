<div align="center">

<br />

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/%E2%9A%A1-Vitalis-00f0ff?style=for-the-badge&labelColor=0a0f1e&color=00f0ff">
  <img alt="Vitalis" src="https://img.shields.io/badge/%E2%9A%A1-Vitalis-00f0ff?style=for-the-badge&labelColor=0a0f1e&color=00f0ff">
</picture>

### A JIT-compiled language with self-evolving functions

<br />

[![Rust](https://img.shields.io/badge/Rust-Edition%202024-F74C00?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Cranelift](https://img.shields.io/badge/Backend-Cranelift%200.116-4B8BBE?style=flat-square)](https://cranelift.dev/)
[![Tests](https://img.shields.io/badge/Tests-470%20passing-00C853?style=flat-square)](.)
[![LOC](https://img.shields.io/badge/LOC-24%2C769%20Rust-blueviolet?style=flat-square)](.)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue?style=flat-square)](LICENSE-MIT)
[![CI](https://github.com/ModernOps888/vitalis/actions/workflows/ci.yml/badge.svg)](https://github.com/ModernOps888/vitalis/actions/workflows/ci.yml)

**v9.0** &nbsp;·&nbsp; 31 source files &nbsp;·&nbsp; 470 tests &nbsp;·&nbsp; 14 algorithm libraries &nbsp;·&nbsp; 405 FFI exports &nbsp;·&nbsp; 304 Python APIs

<br />

```
                     ╔══════════════════════════════════════╗
                     ║   7.5× faster than Python (avg)     ║
                     ║   29.1× peak · 13/13 benchmarks won ║
                     ╚══════════════════════════════════════╝
```

</div>

---

<br />

## 🔥 Why Vitalis?

Most languages make you choose: **fast** or **flexible**. Vitalis gives you both.

It compiles to native machine code via Cranelift JIT, runs 7.5× faster than Python on real workloads, and has a feature no other language has — **functions that can evolve themselves at runtime**.

```rust
@evolvable
fn strategy(input: i64) -> i64 {
    input * 2 + 1
}

fn main() -> i64 {
    strategy(21)  // → 43
}
```

Mark a function with `@evolvable`. Mutate it. Score its fitness. Roll it back if it fails. The compiler tracks every generation. Your code literally gets better over time.

<br />

---

## ⚡ 60-Second Quick Start

### 1. Clone & Build

```bash
git clone https://github.com/ModernOps888/vitalis.git
cd vitalis
cargo build
```

> **Requirements:** [Rust](https://rustup.rs/) stable (Edition 2024). That's it.

### 2. Write Your First Program

Create `hello.sl`:
```rust
fn main() -> i64 {
    println("Hello from Vitalis!");
    42
}
```

### 3. Run It

```bash
cargo run -- run hello.sl
```

```
Hello from Vitalis!
Result: 42
```

### 4. Use from Python

```bash
cargo build                    # builds vitalis.dll / libvitalis.so
cp python/vitalis.py .         # copy the Python wrapper
python -c "import vitalis; print(vitalis.compile_and_run('fn main() -> i64 { 42 }'))"
# → 42
```

That's it. You're running JIT-compiled native code from Python.

<br />

---

## 🏗️ How It Works

```
                    ┌─────────────────────────────────────────────┐
                    │             VITALIS COMPILER                │
                    │                                             │
  Source (.sl) ───▶ │  Lexer ──▶ Parser ──▶ AST ──▶ TypeChecker  │
                    │                                │            │
                    │                          IR (SSA form)      │
                    │                                │            │
                    │                       Cranelift 0.116 JIT   │
                    │                                │            │
                    │                      Native x86-64 code     │
                    └────────────────────────┬────────────────────┘
                                             │
                                    ┌────────┴────────┐
                                    │                 │
                               Direct exec      C FFI bridge
                               (vtc CLI)        (Python / C)
```

| Stage | Implementation | What It Does |
|-------|---------------|--------------|
| **Lexer** | Logos (zero-copy) | Tokenizes ~70 token variants with zero allocation |
| **Parser** | Recursive-descent + Pratt | Builds AST with operator precedence and `@annotation` support |
| **Type Checker** | Two-pass with scope chains | Catches type errors before codegen |
| **IR** | SSA form | Intermediate representation for optimization |
| **Codegen** | Cranelift 0.116 | Compiles to native x86-64 machine code |
| **FFI Bridge** | `extern "C"` (405 exports) | Zero-overhead interop with Python/C |

<br />

---

## 📊 Benchmarks: Vitalis vs Python

### Real numbers. Real workloads. No tricks.

**Setup:** 100,000 elements · 500 iterations · pre-allocated ctypes arrays (zero marshalling overhead)

This is how you'd actually use Vitalis in production — data stays in Rust-side buffers, Python calls into native code via FFI.

<br />

<div align="center">

```
 Python ████████████████████████████████████████████████  75,327ms
Vitalis ████████▌                                        13,116ms   ← 5.7× faster
```

</div>

<br />

| Operation | Python | Vitalis | Speedup | |
|-----------|-------:|--------:|--------:|-|
| Cosine Distance (100K) | 13,922ms | 479ms | **29.1×** | 🟢🟢🟢🟢🟢 |
| Batch ReLU (100K) | 6,308ms | 613ms | **10.3×** | 🟢🟢🟢🟢 |
| Std Deviation (100K) | 7,183ms | 780ms | **9.2×** | 🟢🟢🟢🟢 |
| MSE Loss (100K) | 3,645ms | 450ms | **8.1×** | 🟢🟢🟢 |
| Batch Sigmoid (100K) | 5,173ms | 686ms | **7.5×** | 🟢🟢🟢 |
| Huber Loss (100K) | 6,524ms | 929ms | **7.0×** | 🟢🟢🟢 |
| Sliding Window (100K) | 3,505ms | 641ms | **5.5×** | 🟢🟢 |
| L2 Normalize (100K) | 4,407ms | 820ms | **5.4×** | 🟢🟢 |
| Softmax (10K) | 712ms | 160ms | **4.5×** | 🟢🟢 |
| Layer Norm (100K) | 9,050ms | 2,313ms | **3.9×** | 🟢 |
| Batch Norm (100K) | 9,115ms | 2,414ms | **3.8×** | 🟢 |
| GELU Batch (100K) | 5,458ms | 2,626ms | **2.1×** | 🟢 |
| Mean (100K) | 327ms | 204ms | **1.6×** | 🟢 |

<br />

<div align="center">

| | |
|---|---|
| **Benchmarks won** | **13 / 13** (100%) |
| **Average speedup** | **7.5×** |
| **Peak speedup** | **29.1×** (Cosine Distance) |
| **Total time saved** | **62.2 seconds** (75.3s → 13.1s) |

</div>

<br />

### Why is Vitalis faster?

| Factor | Python | Vitalis |
|--------|--------|---------|
| **Execution** | Interpreted bytecode | Native x86-64 machine code |
| **Memory** | GC + boxing + heap alloc | Stack-allocated, zero-copy |
| **Math ops** | Dynamic dispatch per op | Inlined native instructions |
| **Data layout** | PyObjects (56 bytes each) | Flat `f64` arrays (8 bytes each) |
| **Function calls** | Frame creation + lookup | Direct `call` instruction |

> **Important:** These benchmarks measure pure computation with pre-allocated arrays (zero marshalling). This matches real production usage where data stays in Rust-side buffers. The first benchmark run (`benchmark_py_vs_vitalis`) which included Python↔C marshalling overhead showed different results — see [Benchmark Methodology](#-benchmark-methodology) for full transparency.

<br />

---

## 🔬 Module Performance (74 Benchmarks)

Every algorithm library benchmarked via Python FFI. These numbers **include** the Python→Rust FFI call overhead.

| Category | Avg Throughput | Peak Function | Peak Throughput |
|----------|---------------:|--------------|----------------:|
| 🔬 Science | **1.59M** ops/sec | `schwarzschild_radius` | 2.25M ops/sec |
| 🧮 Advanced Math | **1.24M** ops/sec | `math_erf` | 2.09M ops/sec |
| ⚛️ Quantum Math | **1.15M** ops/sec | `golden_ratio` | 3.43M ops/sec |
| 📏 Scoring | **1.14M** ops/sec | `elo_expected` | 1.64M ops/sec |
| 📈 Statistics | **525.7K** ops/sec | `normal_pdf` | 1.37M ops/sec |
| 🔤 String Algorithms | **314.8K** ops/sec | `jaro_winkler` | 465.1K ops/sec |
| 🔢 Numerical | **298.5K** ops/sec | `horner` | 593.6K ops/sec |
| 📊 Analytics | **278.0K** ops/sec | `apdex` | 1.37M ops/sec |
| 💫 Quantum Simulator | **254.5K** ops/sec | `bell_state` | 306.5K ops/sec |
| 🛡️ Security | **151.9K** ops/sec | `password_entropy` | 293.2K ops/sec |
| 📦 Compression | **118.0K** ops/sec | `rle_encode` | 283.8K ops/sec |
| 🔒 Crypto | **92.1K** ops/sec | `fnv1a_64` | 167.0K ops/sec |
| 📡 Signal Processing | **22.3K** ops/sec | `rms_energy` | 30.7K ops/sec |
| ⚙️ Core Compiler | **18.6K** ops/sec | `lex` | 46.3K ops/sec |
| 🕸️ Graph Algorithms | **10.8K** ops/sec | `is_bipartite` | 13.0K ops/sec |

> 74 benchmarks. 0 failures. 470 tests passing.

<br />

---

## 📋 Language Reference

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
    add(20, 22)  // → 42
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
    5 |> double |> inc  // → 11
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
    p.x + p.y  // → 7
}
```

### Pattern Matching
```rust
enum Shape {
    Circle(f64),
    Square(f64),
}

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Square(side) => side * side,
    }
}
```

### Evolution
```rust
@evolvable
fn strategy(input: i64) -> i64 {
    input * 2
}

// At runtime:
// evolve strategy → new variant
// fitness score → 0.95
// rollback → previous generation if fitness drops
```

> **Full language reference →** [docs/LANGUAGE_GUIDE.md](docs/LANGUAGE_GUIDE.md)

<br />

---

## 🐍 Python Integration

Vitalis ships a full Python wrapper with 304 exported functions. Import it and call native Rust from Python — no subprocess, no REST API, just FFI.

### Core Compiler

```python
import vitalis

# Compile and run Vitalis code
result = vitalis.compile_and_run("fn main() -> i64 { 42 }")
assert result == 42

# Type-check without running
errors = vitalis.check("fn main() -> i64 { true }")  # → type error

# Tokenize
tokens = vitalis.lex("let x = 42;")

# Parse to AST
ast = vitalis.parse_ast("fn main() -> i64 { 42 }")
```

### Evolution API

```python
# Register an evolvable function
vitalis.evo_register("score", "@evolvable fn score(x: i64) -> i64 { x * 2 }")

# Evolve it (creates generation 2)
vitalis.evo_evolve("score", "@evolvable fn score(x: i64) -> i64 { x * 3 }")

# Score fitness
vitalis.evo_set_fitness("score", 0.95)

# Roll back if needed
vitalis.evo_rollback("score", 1)
```

### Hot-Path Native Ops (7.5× faster than Python)

```python
# Statistics
p95 = vitalis.hotpath_p95([1.0, 2.0, 3.0, ..., 100000.0])
mean = vitalis.hotpath_mean(data)
stddev = vitalis.hotpath_stddev(data)

# ML Activations
scores = vitalis.hotpath_softmax([1.0, 2.0, 3.0])
activated = vitalis.hotpath_batch_relu(data)
normalized = vitalis.hotpath_layer_norm(data)

# Loss Functions
loss = vitalis.hotpath_mse(predictions, targets)
loss = vitalis.hotpath_huber(predictions, targets, delta=1.0)
```

### Algorithm Libraries (269 functions)

```python
# Quantum computing
q = vitalis.QuantumRegister(2)
q.h(0).cnot(0, 1)              # Bell state
print(q.prob(0))                # → 0.5

# Cryptography
vitalis.sha256("hello")         # → SHA-256 hash
vitalis.base64_encode("data")   # → base64 string

# Graph algorithms
vitalis.bfs(edges, start)       # → BFS traversal
vitalis.dijkstra(edges, s, t)   # → shortest path
vitalis.pagerank(edges, n)      # → PageRank scores

# String algorithms
vitalis.levenshtein("kitten", "sitting")  # → 3
vitalis.jaro_winkler("abc", "abd")        # → 0.933
vitalis.soundex("Robert")                 # → R163

# Science
vitalis.physical_constant("c")            # → 299792458.0
vitalis.schwarzschild_radius(1.989e30)    # → 2953.35m
vitalis.kinetic_energy(10.0, 5.0)         # → 125.0 J

# Security
vitalis.detect_sqli("'; DROP TABLE--")    # → True
vitalis.detect_xss("<script>alert(1)")    # → True
vitalis.password_strength("hunter2")      # → "weak"

# And 200+ more...
```

<br />

---

## 🧬 Code Evolution

This is Vitalis's unique feature. Functions marked `@evolvable` can be mutated at runtime while tracking every generation.

```
Generation 1: fn score(x) → x * 2         fitness: 0.72
Generation 2: fn score(x) → x * 3 + 1     fitness: 0.89  ← evolved
Generation 3: fn score(x) → x * 4 - 2     fitness: 0.61  ← regressed
         ↓ rollback to gen 2
Active:      fn score(x) → x * 3 + 1      fitness: 0.89  ✓
```

### How to use it:

```python
import vitalis

# 1. Register a function
vitalis.evo_register("predict",
    "@evolvable fn predict(x: i64) -> i64 { x * 2 }")

# 2. Evolve it with a new variant
gen = vitalis.evo_evolve("predict",
    "@evolvable fn predict(x: i64) -> i64 { x * 3 + 1 }")
# gen → 2

# 3. Score it
vitalis.evo_set_fitness("predict", 0.89)

# 4. If it regresses, roll back
vitalis.evo_rollback("predict", 1)  # back to gen 1

# 5. Load @evolvable functions from a .sl file
vitalis.evo_load("@evolvable fn f(x: i64) -> i64 { x }")
```

Use cases:
- **Self-optimizing ML pipelines** — scoring functions evolve based on accuracy
- **A/B testing at the function level** — deploy variants, measure, keep the winner
- **Autonomous agents** — strategies mutate and improve without redeployment

<br />

---

## 📦 What's Inside (14 Algorithm Libraries)

<details>
<summary><b>📡 Signal Processing</b> — FFT, filters, windowing, spectral analysis (550 LOC)</summary>

<br />

- FFT / IFFT (Cooley-Tukey)
- Convolution, cross-correlation, autocorrelation
- FIR / IIR filter application
- Hann, Hamming, Blackman window functions
- RMS energy, spectral centroid, spectral rolloff

</details>

<details>
<summary><b>🔒 Cryptography</b> — SHA-256, HMAC, Base64, CRC-32 (440 LOC)</summary>

<br />

- SHA-256 hash (full NIST implementation)
- HMAC-SHA256
- Base64 encode/decode
- CRC-32, FNV-1a hash
- XorShift64 PRNG

</details>

<details>
<summary><b>🕸️ Graph Algorithms</b> — BFS, Dijkstra, PageRank, SCC (789 LOC)</summary>

<br />

- BFS, DFS traversal
- Dijkstra's shortest path
- PageRank (iterative)
- Tarjan's strongly connected components
- Topological sort, cycle detection
- Bipartite checking

</details>

<details>
<summary><b>🔤 String Algorithms</b> — Edit distance, fuzzy matching, search (574 LOC)</summary>

<br />

- Levenshtein edit distance
- Jaro-Winkler similarity
- Soundex phonetic encoding
- KMP pattern search
- Rabin-Karp search
- Boyer-Moore-Horspool

</details>

<details>
<summary><b>🔢 Numerical</b> — Linear algebra, integration, root finding (709 LOC)</summary>

<br />

- Matrix: determinant, inverse, multiply, transpose
- Eigenvalue approximation (power iteration)
- Numerical integration (Simpson's rule, trapezoidal)
- Root finding (Newton-Raphson, bisection)
- Horner's polynomial evaluation
- Dot product, cross product

</details>

<details>
<summary><b>📦 Compression</b> — RLE, Huffman, delta coding, LZ77 (532 LOC)</summary>

<br />

- Run-length encoding/decoding
- Huffman coding (tree construction + encode/decode)
- Delta encoding/decoding
- LZ77 compression/decompression

</details>

<details>
<summary><b>📈 Statistics</b> — Distributions, regression, hypothesis testing (653 LOC)</summary>

<br />

- Normal/Poisson/Binomial/Exponential PDF & CDF
- Pearson correlation, Spearman rank correlation
- Linear regression (least squares)
- Chi-squared test, Kolmogorov-Smirnov test
- Skewness, kurtosis, z-score

</details>

<details>
<summary><b>💫 Quantum Simulator</b> — Statevector, gates, QFT, Bell states (813 LOC)</summary>

<br />

- N-qubit statevector register
- H, X, Y, Z, CNOT, Toffoli gates
- RX, RY, RZ parametric rotation gates
- Quantum Fourier Transform (QFT)
- Bell state preparation
- Bloch sphere coordinates
- Measurement with probability sampling

</details>

<details>
<summary><b>⚛️ Quantum Math</b> — Special functions, quaternions, wavelets (1,004 LOC)</summary>

<br />

- Gamma function, Beta function
- Bessel functions (J0, J1, Y0)
- Riemann zeta function
- Quaternion arithmetic
- Haar wavelets (forward/inverse transform)
- Runge-Kutta RK4 ODE solver

</details>

<details>
<summary><b>🧮 Advanced Math</b> — Combinatorics, fractals, special functions (943 LOC)</summary>

<br />

- Factorial, double factorial
- Binomial coefficient, Catalan numbers, Bell numbers
- Error function (erf)
- Fibonacci (fast), Lucas sequence
- Mandelbrot set iteration
- GCD, LCM, modular exponentiation
- Primality testing

</details>

<details>
<summary><b>🔬 Science</b> — Physics, chemistry, astronomy formulas (504 LOC)</summary>

<br />

- 50+ physical formulas
- Kinetic/potential energy, work-energy theorem
- Schwarzschild radius, escape velocity
- Ideal gas law, heat transfer
- Coulomb's law, Ohm's law
- 20+ physical constants (c, G, h, k_B, etc.)
- Unit conversions (Celsius↔Kelvin↔Fahrenheit)

</details>

<details>
<summary><b>📊 Analytics</b> — Time series, anomaly detection, forecasting (662 LOC)</summary>

<br />

- Moving averages: SMA, EMA, WMA, DEMA
- Anomaly detection (z-score method)
- Simple exponential smoothing (SES)
- Holt linear trend forecasting
- Apdex scoring
- Linear trend extraction

</details>

<details>
<summary><b>🛡️ Security</b> — Input validation, injection detection (421 LOC)</summary>

<br />

- SQL injection detection
- XSS detection
- Path traversal detection
- Command injection detection
- Email validation, URL validation
- Password strength scoring + entropy calculation
- HTML escaping
- Code safety scoring

</details>

<details>
<summary><b>📏 Scoring</b> — Metrics, ratings, A/B testing (470 LOC)</summary>

<br />

- Halstead software metrics
- Maintainability index
- Elo rating system
- Bayesian A/B testing
- Cohen's d effect size
- Pareto ranking
- Geometric mean

</details>

<br />

---

## 🔨 44 Hot-Path Native Ops

These are Rust-native operations exposed directly via FFI — no JIT compilation, pure Rust speed.

| Category | Operations |
|----------|-----------|
| **Rate Limiting** | Sliding window count, token bucket |
| **Statistics** | P95, percentile, mean, median, stddev, entropy |
| **ML Activations** | Softmax, sigmoid, ReLU, GELU, batch norm, layer norm |
| **Loss Functions** | Cross-entropy, MSE, Huber, KL divergence |
| **Vector Ops** | Cosine similarity, cosine distance, L2 normalize, hamming distance |
| **Optimization** | Bayesian UCB, simulated annealing, Boltzmann selection, CMA-ES |
| **Analysis** | Code quality scoring, cognitive complexity, vote tallying |
| **Consensus** | Weighted score, tally votes, string vote tallying |

<br />

---

## 🛠️ CLI Reference

```bash
# Run a .sl program
cargo run -- run examples/hello.sl

# Evaluate an inline expression
cargo run -- eval -e "21 * 2"

# Type-check without executing
cargo run -- check examples/arithmetic.sl

# Dump the AST
cargo run -- dump-ast examples/structs.sl

# Dump the IR
cargo run -- dump-ir examples/arithmetic.sl

# Tokenize a file
cargo run -- lex examples/hello.sl
```

<br />

---

## 📁 Project Structure

```
vitalis/
├── Cargo.toml                # Build config (v9.0.0, Edition 2024)
├── src/
│   ├── main.rs               # CLI binary (vtc)
│   ├── lib.rs                # Library root (28 modules)
│   │
│   │  ── Core Compiler ──
│   ├── lexer.rs              # Logos zero-copy tokenizer
│   ├── parser.rs             # Recursive-descent + Pratt parser
│   ├── ast.rs                # AST definitions (27 expression variants)
│   ├── types.rs              # Two-pass type checker
│   ├── ir.rs                 # SSA-form IR lowering
│   ├── codegen.rs            # Cranelift JIT codegen
│   ├── stdlib.rs             # 97 built-in functions
│   │
│   │  ── Evolution & Performance ──
│   ├── evolution.rs          # @evolvable function registry + rollback
│   ├── engine.rs             # Evolution cycle runner
│   ├── meta_evolution.rs     # Strategy-level evolution
│   ├── memory.rs             # Engram-based memory store
│   ├── hotpath.rs            # 44 native fast-path operations
│   ├── optimizer.rs          # Optimization passes
│   ├── simd_ops.rs           # SIMD-optimized operations
│   │
│   │  ── Algorithm Libraries (14 modules) ──
│   ├── signal_processing.rs  # FFT, filters, windowing
│   ├── crypto.rs             # SHA-256, HMAC, Base64, CRC
│   ├── graph.rs              # BFS, Dijkstra, PageRank, SCC
│   ├── string_algorithms.rs  # Levenshtein, KMP, Soundex
│   ├── numerical.rs          # Linear algebra, integration
│   ├── compression.rs        # RLE, Huffman, LZ77
│   ├── probability.rs        # Distributions, correlation, tests
│   ├── quantum.rs            # Quantum circuit simulator
│   ├── quantum_math.rs       # Gamma, Bessel, quaternions
│   ├── advanced_math.rs      # Factorial, erf, Mandelbrot
│   ├── science.rs            # 50+ physics/chemistry formulas
│   ├── analytics.rs          # Time series, anomaly detection
│   ├── security.rs           # Validation, injection detection
│   ├── scoring.rs            # Halstead, Elo, A/B testing
│   │
│   └── bridge.rs             # C FFI exports (405 functions)
│
├── python/
│   └── vitalis.py            # Python wrapper (3,036 LOC, 304 exports)
├── examples/                 # Example .sl programs
├── docs/
│   ├── LANGUAGE_GUIDE.md     # Complete language reference
│   └── EXTENDING.md          # Developer extension guide
└── tests/                    # 470 inline tests
```

<br />

---

## 📐 Benchmark Methodology

Full transparency on how benchmarks were run:

### v3 Benchmarks (the headline numbers)

| Parameter | Value |
|-----------|-------|
| **Machine** | Windows 11, AMD Ryzen / Intel i7 |
| **Data size** | 100,000 elements (10K for softmax) |
| **Iterations** | 500 per operation |
| **Array setup** | Pre-allocated `ctypes.c_double * N` arrays |
| **Marshalling** | Zero — data stays in C-compatible buffers |
| **Python baseline** | Pure Python loops (no NumPy) |
| **Vitalis** | Native Rust via `extern "C"` FFI |

**Why pre-allocated arrays?** In production, Vitalis holds data in Rust-side buffers. The Python→C marshalling cost (converting Python lists to C arrays per call) is a one-time setup cost, not a per-operation cost. These benchmarks measure what matters: **raw computation speed**.

### v1 Benchmarks (with marshalling overhead)

The first benchmark run used small arrays (200–1,000 elements) with 10,000 iterations and **included Python↔C marshalling on every call**. In that test, Python won 16/17 benchmarks because the marshalling overhead dominated the small computation. This is expected — you wouldn't convert a Python list to a C array 10,000 times in production.

| Version | Data Size | Marshalling | Vitalis Wins | Avg Speedup |
|---------|----------|-------------|-------------|-------------|
| **v1** (with marshalling) | 200–1K | Every call | 1/17 | 0.7× |
| **v3** (production-realistic) | 10K–100K | Pre-allocated | **13/13** | **7.5×** |

> Both benchmark results are reproducible. Run `python _benchmark_py_vs_vitalis.py` (v1) and `python _benchmark_v3_compute.py` (v3) yourself.

<br />

---

## 📈 Growth: v0.1 → v9.0

| Metric | v0.1 | v9.0 | Change |
|--------|-----:|-----:|-------:|
| Source files | 17 | 31 | +82% |
| Rust LOC | ~13,500 | 24,769 | +83% |
| Tests | 234 | 470 | +101% |
| FFI exports | ~50 | 405 | +710% |
| Python exports | ~40 | 304 | +660% |
| Python wrapper LOC | 930 | 3,036 | +226% |
| Algorithm modules | 0 | 14 | new |

<br />

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Areas where help is welcome:**

- 🔧 Language features — closures, traits, generics
- 🖥️ Platform support — Linux/macOS CI testing
- ✏️ Editor support — VS Code extension, syntax highlighting
- 📝 Documentation & tutorials
- 📦 Package manager for `.sl` dependencies

<br />

---

## 🔐 Security

See [SECURITY.md](SECURITY.md) for responsible disclosure policy.

<br />

---

## 📄 License

Dual-licensed under your choice of:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

<br />

---

## 🙏 Acknowledgments

Built with [Cranelift](https://cranelift.dev/) (JIT), [Logos](https://github.com/maciejhirsz/logos) (lexer), and [Clap](https://github.com/clap-rs/clap) (CLI).

<br />

---

<div align="center">

**Built solo from scratch by [Bart Chmiel](https://www.linkedin.com/in/modern-workplace-tech365/)**

[Website](https://infinitytechstack.uk) · [Tech Stack](https://infinitytechstack.uk/techstack) · [Consulting](https://infinitytechstack.uk/consulting)

<br />

⚡ *A language where code evolves itself.* ⚡

</div>
