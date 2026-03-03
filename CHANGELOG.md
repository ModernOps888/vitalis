# Changelog

All notable changes to Vitalis will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [20.0.0] - 2025-01-25

### Added — Nova ML Engine (6 modules, 42 FFI exports, +4,196 LOC)

#### Tensor Engine (`tensor_engine.rs`, ~700 LOC)
- N-dimensional tensor type with `DType` support (F32, F64, I32, I64, Bool)
- `Shape` validation, broadcasting rules, and contiguous stride layout
- `Storage` abstraction with CPU/CUDA device targeting
- 33+ tensor operations: arithmetic, matmul, transpose, reshape, slice, concat, stack, split
- Full autograd engine: computation graph, backward pass, gradient accumulation
- `GradientTape` for tracking operations and automatic differentiation
- 7 FFI exports: `vt_tensor_create`, `vt_tensor_matmul`, `vt_tensor_backward`, etc.

#### Deep Learning (`deep_learning.rs`, ~600 LOC)
- `Linear` layer with Xavier initialization and optional bias
- `RMSNorm` and `LayerNorm` normalization layers
- `TokenEmbedding` with vocabulary-sized weight matrices
- `MultiHeadAttention` with Rotary Position Embeddings (RoPE) and Grouped Query Attention (GQA)
- `SwiGLUFFN` feed-forward network (SwiGLU activation, gate/up/down projections)
- `TransformerBlock` combining attention + FFN + norm layers
- `Transformer` model with configurable depth, width, heads, and KV-heads
- 6 FFI exports: `vt_transformer_create`, `vt_transformer_forward`, etc.

#### GPU Compute (`gpu_compute.rs`, ~400 LOC)
- `DeviceInfo` struct for GPU detection (name, compute capability, VRAM)
- `CudaRuntime` for device management and kernel dispatch
- `GpuMemoryPool` with slab allocation and defragmentation
- 11 CUDA PTX kernel sources: `vector_add`, `matrix_multiply`, `softmax`, `layer_norm`, `gelu`, `rope`, `cross_entropy`, `adam_update`, `reduce_sum`, `transpose`, `embedding_lookup`
- 5 FFI exports: `vt_cuda_init`, `vt_cuda_device_count`, `vt_gpu_pool_create`, etc.

#### ML Training (`ml_training.rs`, ~500 LOC)
- `AdamW` optimizer with weight decay, bias correction, epsilon stability
- `CosineScheduler` with warmup steps and minimum learning rate
- `WarmupConstantScheduler` for linear LR warmup
- `clip_grad_norm` for gradient clipping by max norm
- `DataLoader` with batch iteration and optional shuffling
- `TrainCache` for gradient and optimizer state management
- `model_backward` function for loss-to-parameter gradient computation
- `save_weights` / `load_weights` for model checkpointing
- `Trainer` orchestrating forward → loss → backward → optimize loop
- `TrainStep` result struct with loss, learning rate, gradient norm, throughput
- 8 FFI exports: `vt_adamw_create`, `vt_trainer_create`, `vt_trainer_step`, etc.

#### BPE Tokenizer (`bpe_tokenizer.rs`, ~300 LOC)
- `BpeTokenizer` with byte-pair encoding merge rules
- `train()` from corpus text with configurable vocabulary size
- `encode()` text → token IDs, `decode()` token IDs → text
- `save()` / `load()` for tokenizer persistence (JSON format)
- Special token handling: `<pad>`, `<unk>`, `<bos>`, `<eos>`
- 6 FFI exports: `vt_tokenizer_create`, `vt_tokenizer_train`, `vt_tokenizer_encode`, etc.

#### Model Inference (`model_inference.rs`, ~400 LOC)
- `ModelConfig` with 4 presets: tiny (5M), small (125M), medium (1B), large (3B)
- `GenerateConfig` with temperature, top-k, top-p, repetition penalty, max tokens
- `sample_token` with combined top-k filtering and nucleus (top-p) sampling
- `generate()` autoregressive text generation with KV-cache support
- `generate_with_info()` returning tokens, text, and generation statistics
- 10 FFI exports: `vt_model_config_tiny` through `vt_generate`, etc.

### Changed
- `Cargo.toml` version bumped from 19.0.0 → 20.0.0
- `lib.rs` expanded with 6 new `pub mod` declarations under "Nova ML Engine (v20.0)" section
- `bridge.rs` version string updated to "20.0.0"
- `README.md` comprehensive overhaul: new ML Engine section, updated stats, revised roadmap

### Metrics
| Metric | v19.0.0 | v20.0.0 | Delta |
|--------|---------|---------|-------|
| Rust source files | 41 | 47 | +6 |
| Rust LOC (total) | ~31,400 | 35,632 | +4,196 |
| Test cases | 708 | 748 | +40 |
| FFI exports | ~30 | 42 | +12 |
| Public functions | ~400 | 456 | +56 |

---

## [9.0.0] - 2026-03-01

### Added

#### v7.0 Algorithm Libraries (7 modules, 100+ FFI functions)
- **Signal Processing** (`signal_processing.rs`, 550 LOC) — FFT/IFFT, power spectrum, convolution, cross-correlation, windowing (Hann/Hamming/Blackman), FIR/IIR biquad filters, zero-crossing rate, RMS energy, spectral centroid, autocorrelation, linear resampling
- **Cryptography** (`crypto.rs`, 440 LOC) — SHA-256, HMAC-SHA256, Base64 encode/decode, CRC-32, FNV-1a 64-bit hash, constant-time comparison, XorShift128+ PRNG
- **Graph Algorithms** (`graph.rs`, 789 LOC) — BFS, DFS, Dijkstra shortest paths, cycle detection, bipartite check, connected components, topological sort, PageRank, Tarjan SCC
- **String Algorithms** (`string_algorithms.rs`, 574 LOC) — Levenshtein distance, LCS (length + string), longest common substring, Hamming distance, Jaro-Winkler similarity, Soundex, string rotation check, n-gram counting, KMP/Rabin-Karp/BMH pattern search
- **Numerical / Linear Algebra** (`numerical.rs`, 709 LOC) — Matrix multiply/determinant/inverse/trace, linear system solver, Simpson's/trapezoidal integration, Horner polynomial evaluation, Lagrange interpolation, power iteration (eigenvalue), dot/cross product, vector norm, Newton/bisection root finding
- **Compression** (`compression.rs`, 532 LOC) — RLE encode/decode, Huffman encoding, delta encode/decode, LZ77 compression
- **Probability & Statistics** (`probability.rs`, 653 LOC) — Mean/median/variance/stddev/skewness/kurtosis/mode, normal/exponential/Poisson/binomial distributions, Pearson/Spearman correlation, linear regression, Shannon entropy, chi-squared, Kolmogorov-Smirnov statistic, covariance matrix

#### v9.0 Advanced Modules (7 modules, 170+ FFI functions)
- **Quantum Simulator** (`quantum.rs`, 813 LOC) — Full statevector quantum register with H/X/Y/Z/CNOT/RX/RY/RZ gates, Bell state preparation, QFT, measurement, Bloch sphere coordinates, fidelity, purity, von Neumann entropy
- **Quantum Math** (`quantum_math.rs`, 1004 LOC) — Complex arithmetic, gamma/lgamma/beta functions, Bessel J0/J1, Riemann zeta (1000-term + Euler-Maclaurin), Monte Carlo integration, RK4 ODE solver, modular exponentiation, primality testing, GCD/LCM, Haar wavelet, Legendre polynomials, Fibonacci, golden ratio, Euler totient, quaternion multiply/rotate/SLERP, outer/Kronecker products
- **Advanced Math** (`advanced_math.rs`, 943 LOC) — Factorial, binomial coefficients, Catalan numbers, error function (erf), Mandelbrot iteration, integer partitions, Bell numbers
- **Science** (`science.rs`, 504 LOC) — Physical constants, kinematics (3 equations), energy (kinetic/potential), pendulum period, orbital/escape velocity, projectile range/height, ideal gas law, Carnot efficiency, radiation power, heat transfer, entropy change, Coulomb force, electric field, Ohm's law, capacitor energy, magnetic force, wavelength, photon energy, Doppler shift, Snell's law, de Broglie wavelength, radioactive decay, mass-energy equivalence, pH/pOH, Arrhenius equation, Nernst equation, dilution, Schwarzschild radius, luminosity, Hubble velocity, redshift, Reynolds number, drag force, Bernoulli pressure, unit conversions (6 types)
- **Analytics** (`analytics.rs`, 662 LOC) — SMA/EMA/WMA/DEMA moving averages, anomaly detection (z-score/IQR/MAD), linear trend, turning points, SES/Holt forecasting, min-max/z-score normalization, SLA uptime, error rate, throughput, Apdex score, MTBF, MTTR, cardinality
- **Security** (`security.rs`, 421 LOC) — Email/IPv4/URL validation, length/range validation, SQL injection/XSS/path traversal/command injection detection, password strength/entropy scoring, memory/time/recursion quota checks, resource utilization, code safety scoring, audit hashing, hash chains, token bucket/sliding window rate limiting, capability-based sandbox (grant/revoke/check), HTML escaping
- **Scoring** (`scoring.rs`, 470 LOC) — Maintainability index, tech debt ratio, composite code quality, Halstead metrics, weighted fitness, Pareto dominance/ranking, Elo rating (update/expected), Welch's t-test, Cohen's d, Mann-Whitney U, Wilson score conversion rate, Bayesian A/B testing, regression scoring, geometric/harmonic/power mean, latency scoring, efficiency ratios, system health composite, decay/sigmoid/tournament fitness functions

#### Python Wrapper Overhaul
- `python/vitalis.py` expanded from 930 → 2,500 lines
- 304 Python functions exposed via `__all__`
- `QuantumRegister` Python class with gate chaining, measurement, entropy
- Full ctypes coverage for all C FFI functions across 14 modules
- Helper utilities: `_str_buf()`, `_edges_flat_sz()`, `_to_double_array()`

### Changed
- `Cargo.toml` version bumped from 0.1.0 → 9.0.0
- `lib.rs` expanded from 17 → 28 public modules (10 organized sections)
- `bridge.rs` version string updated to "9.0.0"

### Metrics
| Metric | v0.1.0 | v9.0.0 | Delta |
|--------|--------|--------|-------|
| Rust source files | 17 | 31 | +14 |
| Rust LOC (total) | ~13,500 | 24,769 | +11,269 |
| Test cases | 234 | 470 | +236 |
| Python wrapper LOC | 930 | 2,500 | +1,570 |
| Python `__all__` exports | ~50 | 304 | +254 |
| Stdlib functions | 97 | 99 | +2 |
| Hot-path functions | 44 | 80 | +36 |
| Algorithm modules | 0 | 14 | +14 |

### Benchmark Scores (v9.0.0, 74 benchmarks, all passing)

| Category | Avg Throughput | Peak Function | Peak ops/sec |
|----------|---------------|---------------|-------------|
| Core Compiler | 18.4K ops/sec | `lex` | 46.1K |
| Signal Processing | 22.7K ops/sec | `rms_energy` | 31.6K |
| Crypto | 91.4K ops/sec | `fnv1a_64` | 163.0K |
| Graph Algorithms | 10.3K ops/sec | `is_bipartite` | 12.6K |
| String Algorithms | 294.2K ops/sec | `jaro_winkler` | 443.5K |
| Numerical | 292.0K ops/sec | `horner` | 574.7K |
| Compression | 104.7K ops/sec | `rle_encode` | 245.5K |
| Statistics | 413.7K ops/sec | `normal_pdf` | 716.6K |
| Quantum Simulator | 203.2K ops/sec | `bell_state` | 246.1K |
| Quantum Math | 1.04M ops/sec | `golden_ratio` | 2.12M |
| Advanced Math | 1.17M ops/sec | `math_erf` | 2.03M |
| Science | 1.50M ops/sec | `celsius_to_kelvin` | 2.14M |
| Analytics | 255.0K ops/sec | `apdex` | 1.23M |
| Security | 154.6K ops/sec | `password_entropy` | 290.7K |
| Scoring | 1.06M ops/sec | `elo_expected` | 1.55M |

[9.0.0]: https://github.com/ModernOps888/vitalis/compare/v0.1.0...HEAD

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
