<div align="center">

<br />

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/%E2%9A%A1-Vitalis-00f0ff?style=for-the-badge&labelColor=0a0f1e&color=00f0ff">
  <img alt="Vitalis" src="https://img.shields.io/badge/%E2%9A%A1-Vitalis-00f0ff?style=for-the-badge&labelColor=0a0f1e&color=00f0ff">
</picture>

# Vitalis

### A JIT-compiled language with self-evolving functions

<br />

[![Rust](https://img.shields.io/badge/Rust-Edition%202024-F74C00?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Cranelift](https://img.shields.io/badge/Backend-Cranelift%200.116-4B8BBE?style=flat-square)](https://cranelift.dev/)
[![Tests](https://img.shields.io/badge/Tests-651%20passing-00C853?style=flat-square)](.)
[![LOC](https://img.shields.io/badge/Rust%20LOC-33%2C500-blueviolet?style=flat-square)](.)
[![Python APIs](https://img.shields.io/badge/Python%20APIs-499-ff69b4?style=flat-square)](python/vitalis.py)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue?style=flat-square)](LICENSE-MIT)
[![CI](https://github.com/ModernOps888/vitalis/actions/workflows/ci.yml/badge.svg)](https://github.com/ModernOps888/vitalis/actions/workflows/ci.yml)

```
╔══════════════════════════════════════════════════════════════════════╗
║                                                                      ║
║   ┌─────────┐    ┌────────┐    ┌─────┐    ┌───────────┐    ┌─────┐  ║
║   │ Source   │───▶│ Lexer  │───▶│ AST │───▶│ TypeCheck │───▶│ IR  │  ║
║   │  (.sl)   │    │ (Logos)│    │     │    │ (2-pass)  │    │(SSA)│  ║
║   └─────────┘    └────────┘    └─────┘    └───────────┘    └──┬──┘  ║
║                                                                │     ║
║                  ┌────────────────────────────────────────────┘     ║
║                  ▼                                                   ║
║   ┌──────────────────────┐    ┌──────────────┐    ┌──────────────┐  ║
║   │   Cranelift 0.116    │───▶│  Native x86  │───▶│  C FFI / Py  │  ║
║   │   JIT Backend        │    │  Machine Code│    │  516 APIs    │  ║
║   └──────────────────────┘    └──────────────┘    └──────────────┘  ║
║                                                                      ║
║   ⚡ 7.5× avg faster than Python  ·  29.1× peak speedup             ║
║   🧪 651 tests  ·  41 source files  ·  24 algorithm libraries       ║
║   🧬 Self-evolving @evolvable functions with fitness tracking        ║
║   📦 v15: Strings, File I/O, Maps, Error handling, JSON, Closures   ║
║                                                                      ║
╚══════════════════════════════════════════════════════════════════════╝
```

*Write it in Vitalis. Compile to native. Evolve it at runtime.*

</div>

<br />

---

## Table of Contents

- [Why Vitalis?](#-why-vitalis)
- [v15.0 — What's New](#-v150--whats-new)
- [Quick Start](#-60-second-quick-start)
- [Architecture](#️-architecture)
- [Language Features](#-language-features)
- [Code Evolution](#-code-evolution--the-killer-feature)
- [Python Integration](#-python-integration-482-apis)
- [Algorithm Libraries](#-24-algorithm-libraries)
- [Benchmarks](#-benchmarks-vitalis-vs-python)
- [Hot-Path Native Ops](#-hot-path-native-operations)
- [CLI Reference](#️-cli-reference)
- [Project Structure](#-project-structure)
- [Contributing](#-contributing)

<br />

---

## 🔥 Why Vitalis?

Most languages make you choose: **fast** or **flexible**. Vitalis gives you both.

| | Python | C/Rust | **Vitalis** |
|---|:---:|:---:|:---:|
| **Compiles to native** | ✗ | ✓ | ✓ |
| **Python interop** | — | FFI wrappers | Built-in |
| **Runtime evolution** | ✗ | ✗ | **✓** |
| **Type checking** | Optional | Required | Required |
| **Quick prototyping** | ✓ | ✗ | ✓ |
| **Production speed** | Slow | Fast | **Fast** |

**Three things make Vitalis different:**

1. **Native speed, zero friction** — Cranelift JIT compiles to x86-64 machine code. No VM, no interpreter, no GC. 7.5× faster than Python on real workloads.

2. **Functions that evolve themselves** — Mark any function `@evolvable`. Mutate it at runtime. Score its fitness. Roll back if it regresses. The compiler tracks every generation. Your code literally gets better over time.

3. **Python as a first-class citizen** — Import `vitalis` in Python, get 482 native Rust functions. No subprocess, no REST API, no serialization. Pure FFI.

```rust
// hello.sl — Your first Vitalis program
fn main() -> i64 {
    println("Hello from Vitalis!");
    42
}
```

```rust
// evolution.sl — Self-improving code
@evolvable
fn strategy(input: i64) -> i64 {
    input * 2 + 1
}

fn main() -> i64 {
    strategy(21)  // → 43, but this function can evolve
}
```

<br />

---

## 🚀 v15.0 — What's New

**v15.0** is the largest stdlib expansion in Vitalis history — 46 new built-in functions, working closures, and full 5-place JIT registration for every new feature.

### New Standard Library Categories

| Category | Functions | Examples |
|----------|-----------|---------|
| **String Operations** (19) | `str_upper`, `str_lower`, `str_trim`, `str_contains`, `str_starts_with`, `str_ends_with`, `str_char_at`, `str_substr`, `str_index_of`, `str_replace`, `str_repeat`, `str_reverse`, `str_split_count`, `str_split_get`, `to_string_i64`, `to_string_f64`, `to_string_bool`, `parse_int`, `parse_float` | `str_contains("hello world", "world")` → `true` |
| **File I/O** (6) | `file_read`, `file_write`, `file_append`, `file_exists`, `file_delete`, `file_size` | `file_write("out.txt", "data")` → writes file |
| **Hash Maps** (7) | `map_new`, `map_set`, `map_get`, `map_has`, `map_remove`, `map_len`, `map_keys` | `let m = map_new(); map_set(m, "key", 42)` |
| **Error Handling** (4) | `error_set`, `error_check`, `error_msg`, `error_clear` | `error_set(404, "not found"); error_check()` → `404` |
| **System/Env** (7) | `env_get`, `sleep_ms`, `eprint`, `eprintln`, `pid`, `format_int`, `format_float` | `pid()` → process ID |
| **JSON** (2) | `json_encode`, `json_decode` | `json_encode(map_handle)` → `{"key":42}` |

### Working Closures

Lambdas now produce real function pointers (previously emitted null):

```sl
let double = |x: i64| x * 2;
```

The lambda body is lowered as a separate `IrFunction`, compiled to native code via Cranelift, and the closure value is a live function pointer — not a null sentinel.

### Stats at a Glance

| Metric | v13.0 | v15.0 |
|--------|------:|------:|
| Rust tests passing | 634 | **651** |
| Built-in stdlib functions | 83 | **129** |
| Python API exports | 482 | **499** |
| Closures work? | ✗ (null) | **✓** |
| File I/O? | ✗ | **✓** |
| Hash maps? | ✗ | **✓** |
| Error handling? | ✗ | **✓** |

<br />

---

## ⚡ 60-Second Quick Start

**Prerequisites:** [Rust](https://rustup.rs/) stable (Edition 2024). That's it.

### 1. Clone & Build

```bash
git clone https://github.com/ModernOps888/vitalis.git
cd vitalis
cargo build
```

### 2. Run a Program

```bash
# Run an example
cargo run -- run examples/hello.sl

# Evaluate inline
cargo run -- eval -e "21 * 2"
# → Result: 42
```

### 3. Use from Python

```bash
cargo build                     # produces vitalis.dll / libvitalis.so
cp python/vitalis.py .          # grab the Python wrapper
python -c "
import vitalis
result = vitalis.compile_and_run('fn main() -> i64 { 42 }')
print(f'JIT result: {result}')  # → JIT result: 42
"
```

You're now running JIT-compiled native code from Python. No intermediate steps.

<br />

---

## 🏗️ Architecture

```
                      ┌───────────────────────────────────────────┐
                      │            VITALIS  COMPILER              │
                      │                                           │
   Source (.sl) ────▶ │  Lexer ──▶ Parser ──▶ AST ──▶ TypeCheck  │
                      │    │         │         │          │       │
                      │  Logos    Pratt +    27 expr   2-pass    │
                      │  zero-   recursive   variants  scope    │
                      │  copy    descent               chains   │
                      │                                  │       │
                      │                            IR (SSA)      │
                      │                             │    │       │
                      │                      Optimizer   │       │
                      │                             │    │       │
                      │                    Cranelift 0.116 JIT   │
                      │                             │            │
                      │                     Native x86-64        │
                      └─────────────────────┬───────────────────┘
                                            │
                               ┌────────────┼────────────┐
                               │            │            │
                          Direct exec   C FFI bridge  Evolution
                          (vtc CLI)     (Python / C)  (@evolvable)
```

| Stage | Implementation | Details |
|-------|---------------|---------|
| **Lexer** | [Logos](https://github.com/maciejhirsz/logos) zero-copy | ~70 token variants, zero allocation |
| **Parser** | Recursive-descent + Pratt | Operator precedence, `@annotation` support |
| **AST** | 27 expression variants | Origin tracking, struct/enum/match/pipe |
| **Type Checker** | Two-pass with scope chains | Full type inference before codegen |
| **IR** | SSA form | Intermediate representation for optimization |
| **Optimizer** | Multi-pass (1,294 LOC) | Dead code elimination, constant folding, SIMD |
| **Codegen** | Cranelift 0.116 | Native x86-64 machine code via JIT |
| **FFI** | `extern "C"` bridge | 21 native bridge functions + 99 stdlib built-ins |

<br />

---

## 📋 Language Features

### Types & Variables

```rust
let x: i64 = 42;
let pi: f64 = 3.14159;
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
fn classify(x: i64) -> i64 {
    if x > 100 {
        3
    } else if x > 10 {
        2
    } else {
        1
    }
}
```

### Pipe Operator

```rust
fn double(x: i64) -> i64 { x * 2 }
fn inc(x: i64) -> i64 { x + 1 }
fn square(x: i64) -> i64 { x * x }

fn main() -> i64 {
    5 |> double |> inc |> square  // 5 → 10 → 11 → 121
}
```

### Structs

```rust
struct Point {
    x: i64,
    y: i64,
}

fn manhattan(p: Point) -> i64 {
    p.x + p.y
}

fn main() -> i64 {
    let p = Point { x: 3, y: 4 };
    manhattan(p)  // → 7
}
```

### Enums & Pattern Matching

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

### 99 Built-in Functions

The standard library includes 99 functions available in any `.sl` program — math, string manipulation, I/O, and more. See [docs/LANGUAGE_GUIDE.md](docs/LANGUAGE_GUIDE.md) for the full reference.

<br />

---

## 🧬 Code Evolution — The Killer Feature

This is what no other compiled language does. Functions marked `@evolvable` can be **mutated, scored, and rolled back at runtime** while the compiler tracks every generation.

```
Generation 1:  fn score(x) → x * 2           fitness: 0.72
Generation 2:  fn score(x) → x * 3 + 1       fitness: 0.89   ← improved
Generation 3:  fn score(x) → x * 4 - 2       fitness: 0.61   ← regressed
           ↓   rollback to gen 2
Active:        fn score(x) → x * 3 + 1       fitness: 0.89   ✓
```

### How It Works

```python
import vitalis

# 1. Register an evolvable function
vitalis.evo_register("predict",
    "@evolvable fn predict(x: i64) -> i64 { x * 2 }")

# 2. Evolve with a new variant
gen = vitalis.evo_evolve("predict",
    "@evolvable fn predict(x: i64) -> i64 { x * 3 + 1 }")
# gen → 2

# 3. Score its fitness
vitalis.evo_set_fitness("predict", 0.89)

# 4. If it regresses, roll back
vitalis.evo_rollback("predict", 1)  # back to gen 1

# 5. Batch-load from source
vitalis.evo_load("@evolvable fn f(x: i64) -> i64 { x }")
```

### Use Cases

| Scenario | How Evolution Helps |
|----------|-------------------|
| **ML pipeline optimization** | Scoring functions evolve based on accuracy metrics |
| **A/B testing at function level** | Deploy variants, measure, keep the winner |
| **Autonomous agents** | Strategies mutate and improve without redeployment |
| **Self-tuning systems** | Parameters converge on optimal values automatically |

### Evolution Engine (852 LOC)

Beyond basic `@evolvable`, Vitalis includes a full evolution engine:

- **Population management** — maintain multiple variants simultaneously
- **Fitness landscape** — visualize how variants perform across parameter space
- **Meta-evolution** (830 LOC) — the evolution strategy itself can evolve
- **Engram memory** (802 LOC) — evolutionary knowledge persists across sessions with decay and consolidation

<br />

---

## 🐍 Python Integration (482 APIs)

Vitalis ships a complete Python wrapper (`python/vitalis.py`, 4,906 LOC) with 482 exported functions. Import it and call native Rust from Python — no subprocess, no REST API, just FFI.

### Core Compiler

```python
import vitalis

# Compile and JIT-execute
result = vitalis.compile_and_run("fn main() -> i64 { 42 }")

# Type-check without running
errors = vitalis.check("fn main() -> i64 { true }")  # → type error

# Tokenize
tokens = vitalis.lex("let x = 42;")
# → [("Let", "let"), ("Ident", "x"), ("Assign", "="), ("Int", "42"), ...]

# Parse to AST
ast = vitalis.parse_ast("fn main() -> i64 { 42 }")

# Dump IR
ir = vitalis.dump_ir("fn main() -> i64 { 1 + 2 }")
```

### Hot-Path Native Ops

```python
# Zero-overhead native Rust — no JIT compilation needed
p95 = vitalis.hotpath_p95(data)
mean = vitalis.hotpath_mean(data)
stddev = vitalis.hotpath_stddev(data)

# ML activations
scores = vitalis.hotpath_softmax([1.0, 2.0, 3.0])
activated = vitalis.hotpath_batch_relu(data)
normalized = vitalis.hotpath_layer_norm(data)

# Loss functions
loss = vitalis.hotpath_mse(predictions, targets)
loss = vitalis.hotpath_huber(predictions, targets, delta=1.0)
```

### Algorithm Libraries

```python
# === Quantum Computing ===
q = vitalis.QuantumRegister(2)
q.h(0).cnot(0, 1)                # Bell state
prob = q.prob(0)                  # → 0.5

# === Quantum Algorithms (v13) ===
vitalis.quantum_deutsch_jozsa(2, False)   # Constant oracle → "constant"
vitalis.quantum_grover(8, 5)              # Search for 5 in 8 items
vitalis.quantum_shor_factor(15)           # Factor 15 → (3, 5)
vitalis.quantum_vqe(0.5)                  # Variational eigensolver

# === Bioinformatics (v13) ===
vitalis.bio_gc_content("ATGCGCTA")        # → 0.5
vitalis.bio_needleman_wunsch("ACGT", "ACGT", 1, -1, -2)  # Global alignment
vitalis.bio_sir_model(0.3, 0.1, 999, 1, 0, 10)           # SIR epidemic
vitalis.bio_translate("AUGUUUAAA")        # → "MFK" (codon table)

# === Chemistry & Physics (v13) ===
vitalis.chem_henderson_hasselbalch(4.75, 0.1, 0.1)  # pH = pKa
vitalis.phys_lorentz_factor(0.9)                      # γ ≈ 2.294
vitalis.phys_schwarzschild_radius_adv(1.989e30)       # Solar mass → m
vitalis.chem_particle_in_box(1, 1e-9, 9.109e-31)      # Quantum confinement

# === Neuromorphic Computing (v13) ===
vitalis.neuro_lif(0.5, 1.0, 0.02, 0.01, 1.0)   # LIF neuron step
vitalis.neuro_stdp_delta(0.005, 0.01, 20.0)     # STDP plasticity
vitalis.neuro_spike_entropy(spike_train)          # Information content

# === Evolutionary Computation (v13) ===
vitalis.evo_differential_evolution("sphere", 2, 20, 100, 0.8, 0.9)
vitalis.evo_pso_step(positions, velocities, bests, gbest, 0.7, 1.5, 1.5)
vitalis.evo_novelty_score(behavior, archive, 5)  # k-NN novelty

# === Cryptography ===
h = vitalis.sha256("hello")      # SHA-256
b = vitalis.base64_encode("hi")  # Base64
c = vitalis.crc32("data")        # CRC-32

# === Graph Algorithms ===
path = vitalis.dijkstra(edges, start, end)
ranks = vitalis.pagerank(edges, n)
comps = vitalis.connected_components(edges, n)

# === String Algorithms ===
d = vitalis.levenshtein("kitten", "sitting")  # → 3
s = vitalis.jaro_winkler("abc", "abd")        # → 0.933
p = vitalis.soundex("Robert")                 # → R163

# === Science & Physics ===
c = vitalis.physical_constant("c")            # → 299792458.0
r = vitalis.schwarzschild_radius(1.989e30)    # → 2953.35
e = vitalis.kinetic_energy(10.0, 5.0)         # → 125.0

# === Security ===
vitalis.detect_sqli("'; DROP TABLE--")        # → True
vitalis.detect_xss("<script>alert(1)")        # → True
vitalis.password_strength("hunter2")          # → "weak"

# === Statistics ===
p = vitalis.normal_pdf(0.0, 0.0, 1.0)        # Standard normal
r = vitalis.pearson_correlation(xs, ys)       # Pearson r
reg = vitalis.linear_regression(xs, ys)       # Least squares

# And 370+ more...
```

<br />

---

## 📦 24 Algorithm Libraries

Every module is written in pure Rust with full test coverage. All functions are callable from Python via FFI.

<details>
<summary><b>📡 Signal Processing</b> — 550 LOC · 14 tests</summary>

FFT/IFFT (Cooley-Tukey) · Power spectrum · Convolution · Cross-correlation · Autocorrelation · FIR/IIR biquad filters · Hann/Hamming/Blackman windows · Zero-crossing rate · RMS energy · Spectral centroid · Spectral rolloff · Linear resampling

</details>

<details>
<summary><b>🔒 Cryptography</b> — 440 LOC · 10 tests</summary>

SHA-256 (full NIST) · HMAC-SHA256 · Base64 encode/decode · CRC-32 · FNV-1a 64-bit hash · Constant-time comparison · XorShift128+ PRNG

</details>

<details>
<summary><b>🕸️ Graph Algorithms</b> — 789 LOC · 13 tests</summary>

BFS · DFS · Dijkstra shortest paths · Cycle detection · Bipartite check · Connected components · Topological sort · PageRank (iterative) · Tarjan SCC

</details>

<details>
<summary><b>🔤 String Algorithms</b> — 574 LOC · 12 tests</summary>

Levenshtein distance · LCS (length + string) · Longest common substring · Hamming distance · Jaro-Winkler similarity · Soundex · String rotation check · N-gram counting · KMP search · Rabin-Karp search · Boyer-Moore-Horspool

</details>

<details>
<summary><b>🔢 Numerical / Linear Algebra</b> — 709 LOC · 15 tests</summary>

Matrix multiply/determinant/inverse/trace · Linear system solver · Simpson's/Trapezoidal integration · Horner polynomial eval · Lagrange interpolation · Power iteration (eigenvalue) · Dot/Cross product · Vector norm · Newton-Raphson · Bisection root finding

</details>

<details>
<summary><b>📦 Compression</b> — 532 LOC · 8 tests</summary>

Run-length encoding/decoding · Huffman coding (tree + encode/decode) · Delta encoding/decoding · LZ77 compression/decompression

</details>

<details>
<summary><b>📈 Statistics & Probability</b> — 653 LOC · 16 tests</summary>

Normal/Poisson/Binomial/Exponential PDF & CDF · Pearson/Spearman correlation · Linear regression (least squares) · Chi-squared test · Kolmogorov-Smirnov · Skewness · Kurtosis · Z-score · Shannon entropy · Covariance matrix

</details>

<details>
<summary><b>💫 Quantum Simulator</b> — 813 LOC · 23 tests</summary>

N-qubit statevector register · H/X/Y/Z/CNOT/Toffoli gates · RX/RY/RZ parametric rotations · Quantum Fourier Transform · Bell state preparation · Bloch sphere coordinates · Fidelity · Purity · Von Neumann entropy · Measurement with probability sampling

</details>

<details>
<summary><b>⚛️ Quantum Math</b> — 1,004 LOC · 23 tests</summary>

Complex arithmetic · Gamma/lgamma/Beta functions · Bessel J0/J1 · Riemann zeta (1000-term + Euler-Maclaurin) · Monte Carlo integration · RK4 ODE solver · Haar wavelets · Legendre polynomials · Quaternion multiply/rotate/SLERP · Outer/Kronecker products · Fibonacci · Golden ratio · Euler totient

</details>

<details>
<summary><b>🧮 Advanced Math</b> — 943 LOC · 31 tests</summary>

Factorial · Double factorial · Binomial coefficients · Catalan numbers · Bell numbers · Error function (erf) · Mandelbrot iteration · Integer partitions · Lucas sequence · GCD/LCM · Modular exponentiation · Primality testing

</details>

<details>
<summary><b>🔬 Science & Physics</b> — 504 LOC · 17 tests</summary>

20+ physical constants (c, G, h, k_B, etc.) · Kinematics (3 equations) · Energy (kinetic/potential) · Pendulum · Orbital/Escape velocity · Projectile motion · Ideal gas law · Carnot efficiency · Heat transfer · Coulomb/Electric field · Ohm's law · Wavelength · Photon energy · Doppler shift · Snell's law · de Broglie · Radioactive decay · E=mc² · pH/pOH · Arrhenius · Nernst · Schwarzschild radius · Luminosity · Hubble velocity · Reynolds number · Drag force · Bernoulli · 6 unit conversions

</details>

<details>
<summary><b>📊 Analytics</b> — 662 LOC · 19 tests</summary>

SMA/EMA/WMA/DEMA moving averages · Anomaly detection (z-score/IQR/MAD) · Linear trend · Turning points · SES/Holt forecasting · Min-max/Z-score normalization · SLA uptime · Error rate · Throughput · Apdex score · MTBF · MTTR · Cardinality

</details>

<details>
<summary><b>🛡️ Security</b> — 421 LOC · 16 tests</summary>

SQL injection detection · XSS detection · Path traversal detection · Command injection detection · Email/IPv4/URL validation · Password strength + entropy · HTML escaping · Code safety scoring · Token bucket rate limiting · Capability-based sandbox

</details>

<details>
<summary><b>📏 Scoring & Metrics</b> — 470 LOC · 19 tests</summary>

Halstead software metrics · Maintainability index · Tech debt ratio · Elo rating (update + expected) · Welch's t-test · Cohen's d · Mann-Whitney U · Wilson score · Bayesian A/B testing · Pareto dominance/ranking · Geometric/Harmonic/Power mean · Latency scoring · System health composite

</details>

<details>
<summary><b>🧠 Machine Learning</b> — 580 LOC · 14 tests <sup>v10.0</sup></summary>

K-means clustering · KNN classification · Gaussian Naive Bayes · Logistic regression (train + predict) · PCA (power iteration) · SVD (singular values) · Decision stump (Gini) · DBSCAN density clustering · LDA 2-class · Adam/SGD/RMSProp optimizers · Accuracy/Precision/Recall/F1 · MSE/MAE/R² · Silhouette score · Cosine similarity · K-fold cross-validation

</details>

<details>
<summary><b>📐 Computational Geometry</b> — 490 LOC · 18 tests <sup>v10.0</sup></summary>

Convex hull (Andrew's monotone chain) · Point-in-polygon (ray casting) · Line segment intersection · Closest pair of points · Polygon area/centroid/perimeter · Triangle area · Is-convex check · Point-to-line/segment distance · Circumscribed circle · Minimum enclosing circle (Welzl's) · Bounding box · Fan triangulation · 2D rotation · Collinearity · Angle between vectors · 3D cross product/distance · Spherical-to-Cartesian

</details>

<details>
<summary><b>🔀 Sorting & Searching</b> — 380 LOC · 15 tests <sup>v10.0</sup></summary>

QuickSort (median-of-three) · MergeSort · HeapSort · RadixSort · InsertionSort · ShellSort (Knuth) · CountingSort · Binary search · Lower/Upper bound · Interpolation search · QuickSelect (k-th) · Reservoir sampling · Is-sorted check · Inversion count · Partial sort · Rank computation

</details>

<details>
<summary><b>🤖 Automata & Patterns</b> — 440 LOC · 10 tests <sup>v10.0</sup></summary>

Aho-Corasick multi-pattern search (full automaton with fail links) · Bloom filter (create/insert/contains) · Count-Min Sketch (create/add/estimate) · Simple regex engine (`.`, `*`, `+`, `?`, `|`, `[a-z]`) · Trie (prefix tree: insert/contains/starts_with/count_prefix) · Finite state machine simulator · Levenshtein automaton (threshold check)

</details>

<details>
<summary><b>🎯 Combinatorial Optimization</b> — 470 LOC · 12 tests <sup>v10.0</sup></summary>

0/1 Knapsack (DP) · Fractional knapsack (greedy) · Hungarian assignment · Simplex LP solver · Genetic algorithm (sphere function) · Ant colony optimization (TSP) · TSP nearest neighbor · First Fit Decreasing bin packing · Weighted job scheduling · Coin change (DP) · Longest increasing subsequence · Activity selection · Matrix chain multiplication

</details>

<details>
<summary><b>⚛️ Quantum Algorithms</b> — 988 LOC · 17 tests <sup>v13.0</sup></summary>

Deutsch-Jozsa (constant/balanced oracle detection) · Bernstein-Vazirani (hidden string recovery) · Quantum Phase Estimation (QPE with inverse QFT) · Shor's factoring (period-finding + classical post-processing) · Variational Quantum Eigensolver (VQE, 2-qubit) · QAOA MaxCut (Quantum Approximate Optimization) · Quantum Walk (discrete-time on line graph) · Quantum Teleportation (Bell state + corrections) · Quantum Error Correction (3-qubit bit-flip code) · BB84 QKD (QBER estimation) · Simon's Algorithm (hidden period finding) · Grover's Search (amplitude amplification, optimal iterations)

</details>

<details>
<summary><b>🧬 Bioinformatics</b> — 784 LOC · 22 tests <sup>v13.0</sup></summary>

GC Content · DNA Complement/Reverse Complement · Transcription (DNA→RNA) · Nucleotide Frequency · Codon Translation (full genetic code) · Needleman-Wunsch (global alignment) · Smith-Waterman (local alignment) · Hamming Distance · Edit Distance · K-mer Counting · Linguistic Complexity · Hardy-Weinberg Equilibrium · Lotka-Volterra (predator-prey dynamics) · SIR Epidemic Model · SEIR Epidemic Model · Basic Reproduction Number (R₀) · Michaelis-Menten Kinetics · Competitive Inhibition · Hill Equation (cooperativity) · Jukes-Cantor & Kimura Evolutionary Distance · Protein Molecular Weight · GRAVY Hydropathicity Index · Logistic Growth · Wright-Fisher Drift Simulation · Shannon & Simpson Diversity Indices

</details>

<details>
<summary><b>⚗️ Chemistry & Physics</b> — 634 LOC · 25 tests <sup>v13.0</sup></summary>

**Acid-Base:** Henderson-Hasselbalch · Buffer Capacity · Ionization Fraction  
**Thermodynamics:** Keq from Gibbs · Gibbs Free Energy · Van't Hoff · Clausius-Clapeyron  
**Kinetics:** 1st/2nd Order Decay · Half-Life · Eyring · Arrhenius  
**Electrochemistry:** Butler-Volmer · Tafel · Faraday's Mass Deposition  
**Statistical Mechanics:** Boltzmann Probability · Partition Function · Fermi-Dirac · Bose-Einstein · Maxwell-Boltzmann Speed Distribution · Mean Thermal Energy · Einstein & Debye Specific Heat  
**Special Relativity:** Lorentz Factor · Time Dilation · Length Contraction · Relativistic Momentum/Energy/KE · Velocity Addition · Mass-Energy Equivalence · Relativistic Doppler  
**General Relativity:** Schwarzschild Radius · Gravitational Time Dilation · Gravitational Redshift · ISCO Radius  
**Material Science:** Hooke's Stress · Thermal Expansion · Poisson Transverse Strain · Bulk & Shear Modulus · Fourier Heat Flux  
**Quantum Chemistry:** Hydrogen Energy Levels · Rydberg Wavelength · de Broglie Wavelength · Heisenberg Δp · Particle-in-Box · Harmonic Oscillator · Morse Potential  
**Gas Laws:** Ideal Gas (advanced) · Van der Waals · Compressibility Factor

</details>

<details>
<summary><b>🧠 Neuromorphic Computing</b> — 880 LOC · 20 tests <sup>v13.0</sup></summary>

**Neuron Models:** Leaky Integrate-and-Fire (LIF) · Izhikevich (20+ firing patterns) · Adaptive Exponential (AdEx)  
**Synaptic Plasticity:** Hebbian Learning · Spike-Timing Dependent Plasticity (STDP) · BCM Theory · Homeostatic Scaling  
**Spike Analysis:** Firing Rate · ISI Statistics (mean, CV) · Population Decode (vector) · Fano Factor · Spike Train Correlation · Spike Entropy · Burst Detection  
**Network Models:** Watts-Strogatz Small-World · Barabási-Albert Scale-Free  
**Reservoir Computing:** Echo State Network (ESN) forward pass  
**Neuroevolution:** NEAT Compatibility Distance  
**Oscillatory Dynamics:** Kuramoto Coupled Oscillators (phase synchronization)  
**Utilities:** Sigmoid Activation · Mutual Information Estimation

</details>

<details>
<summary><b>🧬 Evolutionary Computation</b> — 921 LOC · 14 tests <sup>v13.0</sup></summary>

**Metaheuristics:** Differential Evolution (DE/rand/1/bin) · Particle Swarm Optimization (PSO) · CMA-ES (Covariance Matrix Adaptation) · Simulated Annealing  
**Multi-Objective:** NSGA-II Non-Dominated Sorting · Crowding Distance  
**Quality-Diversity:** Novelty Search (archive-based) · MAP-Elites (insert + coverage)  
**Island Models:** Migration-based Island Model Evolution  
**Coevolution:** Competitive Coevolution Fitness  
**Self-Adaptation:** Adaptive Mutation Rate (F) · Adaptive Crossover Rate (CR)  
**Analysis:** Fitness-Distance Correlation (FDC)  
**Test Functions:** Sphere · Rastrigin · Rosenbrock · Ackley · Griewank

</details>

<br />

---

## 📊 Benchmarks: Vitalis vs Python

### Real numbers. Real workloads. No tricks.

**Setup:** 100,000 elements · 500 iterations · pre-allocated `ctypes` arrays · pure Python baseline (no NumPy)

<div align="center">

```
 Python ██████████████████████████████████████████████████  75,327 ms
Vitalis ██████████▌                                        13,116 ms   ← 5.7× faster (aggregate)
```

</div>

| Operation | Python | Vitalis | Speedup |
|-----------|-------:|--------:|--------:|
| Cosine Distance (100K) | 13,922 ms | 479 ms | **29.1×** |
| Batch ReLU (100K) | 6,308 ms | 613 ms | **10.3×** |
| Std Deviation (100K) | 7,183 ms | 780 ms | **9.2×** |
| MSE Loss (100K) | 3,645 ms | 450 ms | **8.1×** |
| Batch Sigmoid (100K) | 5,173 ms | 686 ms | **7.5×** |
| Huber Loss (100K) | 6,524 ms | 929 ms | **7.0×** |
| Sliding Window (100K) | 3,505 ms | 641 ms | **5.5×** |
| L2 Normalize (100K) | 4,407 ms | 820 ms | **5.4×** |
| Softmax (10K) | 712 ms | 160 ms | **4.5×** |
| Layer Norm (100K) | 9,050 ms | 2,313 ms | **3.9×** |
| Batch Norm (100K) | 9,115 ms | 2,414 ms | **3.8×** |
| GELU Batch (100K) | 5,458 ms | 2,626 ms | **2.1×** |
| Mean (100K) | 327 ms | 204 ms | **1.6×** |

<div align="center">

| Metric | Value |
|--------|-------|
| **Benchmarks won** | **13 / 13** (100%) |
| **Average speedup** | **7.5×** |
| **Peak speedup** | **29.1×** (Cosine Distance) |
| **Total time saved** | **62.2 seconds** (75.3s → 13.1s) |

</div>

### Why is Vitalis faster?

| | Python | Vitalis |
|--|--------|---------|
| **Execution** | Interpreted bytecode | Native x86-64 machine code |
| **Memory** | GC + boxing + heap alloc | Stack-allocated, zero-copy |
| **Math** | Dynamic dispatch per op | Inlined native instructions |
| **Data layout** | PyObjects (56 bytes each) | Flat `f64` arrays (8 bytes each) |
| **Function calls** | Frame creation + dict lookup | Direct `call` instruction |

### Methodology note

These benchmarks measure **pure computation with pre-allocated arrays** — matching how you'd actually use Vitalis in production (data stays in native buffers). The first benchmark version (v1) with per-call Python→C marshalling showed different results where marshalling overhead dominated small computations. Both are reproducible — run `python _benchmark_v3_compute.py` (production-realistic) or `python _benchmark_py_vs_vitalis.py` (with marshalling).

<br />

---

## 🔬 Module Throughput (74 Benchmarks)

Every algorithm library benchmarked via Python FFI. These numbers **include** Python→Rust call overhead.

| Category | Avg Throughput | Peak Function | Peak |
|----------|---------------:|--------------|-----:|
| 🔬 Science | **1.59M** ops/s | `schwarzschild_radius` | 2.25M |
| 🧮 Advanced Math | **1.24M** ops/s | `math_erf` | 2.09M |
| ⚛️ Quantum Math | **1.15M** ops/s | `golden_ratio` | 3.43M |
| 📏 Scoring | **1.14M** ops/s | `elo_expected` | 1.64M |
| 📈 Statistics | **525.7K** ops/s | `normal_pdf` | 1.37M |
| 🔤 Strings | **314.8K** ops/s | `jaro_winkler` | 465.1K |
| 🔢 Numerical | **298.5K** ops/s | `horner` | 593.6K |
| 📊 Analytics | **278.0K** ops/s | `apdex` | 1.37M |
| 💫 Quantum Sim | **254.5K** ops/s | `bell_state` | 306.5K |
| 🛡️ Security | **151.9K** ops/s | `password_entropy` | 293.2K |
| 📦 Compression | **118.0K** ops/s | `rle_encode` | 283.8K |
| 🔒 Crypto | **92.1K** ops/s | `fnv1a_64` | 167.0K |
| 📡 Signal | **22.3K** ops/s | `rms_energy` | 30.7K |
| ⚙️ Compiler | **18.6K** ops/s | `lex` | 46.3K |
| 🕸️ Graph | **10.8K** ops/s | `is_bipartite` | 13.0K |

> 74 benchmarks · 0 failures · 634 tests passing

<br />

---

## ⚡ Hot-Path Native Operations

80 Rust-native functions exposed directly via FFI — no JIT compilation, pure Rust speed. These are the fastest path from Python to native code.

| Category | Operations |
|----------|-----------|
| **Statistics** | P95, percentile, mean, median, stddev, entropy, variance |
| **ML Activations** | Softmax, sigmoid, ReLU, GELU, batch norm, layer norm |
| **Loss Functions** | Cross-entropy, MSE, Huber, KL divergence, cosine distance |
| **Vector Ops** | Cosine similarity, L2 normalize, hamming distance, dot product |
| **Rate Limiting** | Sliding window count, token bucket |
| **Optimization** | Bayesian UCB, simulated annealing, Boltzmann selection, CMA-ES |
| **Analysis** | Code quality scoring, cognitive complexity, vote tallying |
| **SIMD Ops** | Vectorized batch math, SIMD-accelerated operations (846 LOC) |

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

# Dump the IR (SSA form)
cargo run -- dump-ir examples/arithmetic.sl

# Tokenize a file
cargo run -- lex examples/hello.sl
```

<br />

---

## 📁 Project Structure

```
vitalis/
├── Cargo.toml                  # v13.0.0, Rust Edition 2024
├── src/
│   ├── main.rs                 # CLI binary (vtc) with clap
│   ├── lib.rs                  # Library root — 38 public modules
│   │
│   │  ── Core Compiler (6 files, 6,968 LOC) ──
│   ├── lexer.rs                # Logos zero-copy tokenizer (~70 tokens)
│   ├── parser.rs               # Recursive-descent + Pratt parser
│   ├── ast.rs                  # 27 expression variants, Origin tracking
│   ├── types.rs                # Two-pass type checker with scope chains
│   ├── ir.rs                   # SSA-form intermediate representation
│   ├── codegen.rs              # Cranelift 0.116 JIT backend
│   │
│   │  ── Runtime & Stdlib (3 files, 3,074 LOC) ──
│   ├── stdlib.rs               # 99 built-in functions
│   ├── hotpath.rs              # 80 native fast-path operations
│   ├── bridge.rs               # extern "C" FFI exports
│   │
│   │  ── Evolution System (4 files, 3,268 LOC) ──
│   ├── evolution.rs            # @evolvable function registry + rollback
│   ├── engine.rs               # Evolution cycle runner + populations
│   ├── meta_evolution.rs       # Strategy-level meta-evolution
│   ├── memory.rs               # Engram-based memory with decay
│   │
│   │  ── Optimization (2 files, 2,140 LOC) ──
│   ├── optimizer.rs            # Multi-pass optimization
│   ├── simd_ops.rs             # SIMD-accelerated operations
│   │
│   │  ── Algorithm Libraries (24 modules, 16,354 LOC) ──
│   ├── signal_processing.rs    # FFT, filters, windowing (550 LOC)
│   ├── crypto.rs               # SHA-256, HMAC, Base64 (440 LOC)
│   ├── graph.rs                # BFS, Dijkstra, PageRank (789 LOC)
│   ├── string_algorithms.rs    # Levenshtein, KMP, Soundex (574 LOC)
│   ├── numerical.rs            # Linear algebra, integration (709 LOC)
│   ├── compression.rs          # RLE, Huffman, LZ77 (532 LOC)
│   ├── probability.rs          # Distributions, correlation (653 LOC)
│   ├── quantum.rs              # Quantum circuit simulator (813 LOC)
│   ├── quantum_math.rs         # Gamma, Bessel, quaternions (1,004 LOC)
│   ├── advanced_math.rs        # Factorial, erf, Mandelbrot (943 LOC)
│   ├── science.rs              # 50+ physics/chemistry (504 LOC)
│   ├── analytics.rs            # Time series, anomaly detection (662 LOC)
│   ├── security.rs             # Validation, injection detection (421 LOC)
│   ├── scoring.rs              # Halstead, Elo, A/B testing (470 LOC)
│   ├── ml.rs                   # K-means, KNN, PCA, DBSCAN (580 LOC)
│   ├── geometry.rs             # Convex hull, Welzl's (490 LOC)
│   ├── sorting.rs              # QuickSort, RadixSort (380 LOC)
│   ├── automata.rs             # Aho-Corasick, Bloom filter (440 LOC)
│   ├── combinatorial.rs        # Knapsack, TSP, Simplex (470 LOC)
│   ├── quantum_algorithms.rs   # DJ, Shor, VQE, Grover (988 LOC) ← v13
│   ├── bioinformatics.rs       # Sequence alignment, SIR (784 LOC) ← v13
│   ├── chemistry_advanced.rs   # Stat-mech, relativity (634 LOC) ← v13
│   ├── neuromorphic.rs         # LIF, STDP, ESN (880 LOC) ← v13
│   └── evolution_advanced.rs   # DE, PSO, CMA-ES, NSGA-II (921 LOC) ← v13
│
├── python/
│   └── vitalis.py              # Python wrapper — 4,906 LOC, 482 exports
├── examples/                   # 8 example .sl programs
├── docs/
│   ├── LANGUAGE_GUIDE.md       # Complete language reference
│   └── EXTENDING.md            # Developer extension guide
├── tests/                      # Integration tests
├── .github/workflows/ci.yml    # CI: Ubuntu + Windows + macOS
├── CHANGELOG.md                # Full release history
├── CONTRIBUTING.md             # Contribution guidelines
└── SECURITY.md                 # Security policy
```

<br />

---

## 📈 v0.1 → v15.0 Growth

| Metric | v0.1 | v9.0 | v10.0 | v13.0 | **v15.0** | Growth |
|--------|-----:|-----:|------:|------:|----------:|-------:|
| Source files | 17 | 31 | 36 | 41 | **41** | +141% |
| Rust LOC | ~13,500 | 24,769 | 28,412 | 32,638 | **33,500** | +148% |
| Tests | 234 | 470 | 542 | 634 | **651** | +178% |
| Stdlib functions | 97 | 99 | 99 | 99 | **129** | +33% |
| Python wrapper LOC | 930 | 2,500 | 3,600 | 4,906 | **4,913** | +428% |
| Python `__all__` exports | ~50 | 304 | 354 | 482 | **499** | +898% |
| Algorithm modules | 0 | 14 | 19 | 24 | **24** | — |
| Hot-path ops | 44 | 80 | 80 | 80 | **80** | +82% |
| Domains covered | 0 | 8 | 13 | 18 | **18** | — |

<br />

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines. PRs welcome.

**Areas where help is needed:**

| Area | What | Impact |
|------|------|--------|
| 🔧 Language features | Closures, traits, generics | Core language capability |
| 🖥️ Platform | Linux/macOS CI testing | Cross-platform reliability |
| ✏️ Editor support | VS Code extension, syntax highlighting | Developer experience |
| 🧬 Biology/Chemistry | More bioinformatics algorithms | Domain expansion |
| 🧠 Neuromorphic | Cortical column models | Brain-inspired computing |
| 📦 Package manager | `.sl` dependency system | Ecosystem |

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

## Acknowledgments

Built with [Cranelift](https://cranelift.dev/) (JIT), [Logos](https://github.com/maciejhirsz/logos) (lexer), and [Clap](https://github.com/clap-rs/clap) (CLI).

<br />

---

<div align="center">

**Solo-built from scratch by [Bart Chmiel](https://www.linkedin.com/in/modern-workplace-tech365/)**

[Website](https://infinitytechstack.uk) · [Tech Stack](https://infinitytechstack.uk/techstack) · [Consulting](https://infinitytechstack.uk/consulting)

<br />

⚡ *A language where code evolves itself.* ⚡

</div>
