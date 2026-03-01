# Changelog

All notable changes to Vitalis will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
