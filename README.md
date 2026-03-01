<div align="center">

# ⚡ Vitalis

**A JIT-compiled programming language with built-in code evolution**

[![Rust](https://img.shields.io/badge/Rust-Edition%202024-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue)](LICENSE-MIT)
[![Cranelift](https://img.shields.io/badge/Backend-Cranelift%200.116-green)](https://cranelift.dev/)

*A language where functions can evolve themselves at runtime.*

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
- 98 built-in stdlib functions (math, ML activations, string ops)
- Evolution keywords: `evolve`, `fitness`, `mutation`, `rollback`, `recall`, `memorize`

### Native Performance (44 Hot-Path Ops)
Rust-implemented operations exposed via C FFI, benchmarked at **7.6x avg / 29.7x peak faster than Python**:

| Category | Operations |
|----------|-----------|
| **Rate Limiting** | Sliding window, token bucket |
| **Statistics** | P95, percentile, mean, median, stddev, entropy |
| **ML Activations** | Softmax, sigmoid, ReLU, GELU, batch norm, layer norm |
| **Loss Functions** | Cross-entropy, MSE, Huber, KL divergence |
| **Vector Ops** | Cosine similarity/distance, L2 normalize, hamming distance |
| **Optimization** | Bayesian UCB, simulated annealing, Boltzmann selection, CMA-ES |
| **Analysis** | Code quality scoring, cognitive complexity, vote tallying |

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

# Native hot-path ops (7.6x faster than Python)
p95 = vitalis.hotpath_p95([1.0, 2.0, 3.0, ..., 100.0])
scores = vitalis.hotpath_softmax([1.0, 2.0, 3.0])
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
├── Cargo.toml          # Build configuration
├── src/
│   ├── main.rs         # CLI binary (vtc)
│   ├── lib.rs          # Library root
│   ├── lexer.rs        # Logos-based tokenizer
│   ├── parser.rs       # Recursive-descent parser
│   ├── ast.rs          # AST node definitions
│   ├── types.rs        # Type checker
│   ├── ir.rs           # IR lowering
│   ├── codegen.rs      # Cranelift JIT codegen
│   ├── stdlib.rs       # 98 built-in functions
│   ├── evolution.rs    # @evolvable function registry
│   ├── engine.rs       # Evolution cycle runner
│   ├── meta_evolution.rs # Strategy evolution
│   ├── memory.rs       # Engram-based memory store
│   ├── hotpath.rs      # 44 native fast-path ops
│   ├── optimizer.rs    # Optimization passes
│   ├── simd_ops.rs     # SIMD-optimized operations
│   └── bridge.rs       # C FFI exports
├── examples/           # Example .sl programs
├── python/
│   └── vitalis.py      # Python ctypes wrapper
└── tests/              # Test suite
```

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

Vitalis hot-path operations vs Python (100K elements, 500 iterations, pre-allocated arrays):

| Operation | Vitalis | Python | Speedup |
|-----------|---------|--------|---------|
| Cosine Distance | 0.15ms | 4.47ms | **29.7x** |
| Softmax | 0.22ms | 2.84ms | **12.9x** |
| MSE Loss | 0.10ms | 1.28ms | **12.8x** |
| Layer Norm | 0.31ms | 2.43ms | **7.8x** |
| Batch Norm | 0.28ms | 1.47ms | **5.3x** |
| **Average** | | | **7.6x** |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Areas where help is welcome:
- Language features (closures, traits, generics)
- Platform support (Linux/macOS CI)
- Editor support (VS Code extension, syntax highlighting)
- Documentation and tutorials
- Package manager for `.sl` dependencies

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

Built with [Cranelift](https://cranelift.dev/) for JIT compilation, [Logos](https://github.com/maciejhirsz/logos) for lexing, and [Clap](https://github.com/clap-rs/clap) for CLI.
