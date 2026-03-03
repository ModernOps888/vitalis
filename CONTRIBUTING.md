# Contributing to Vitalis

Thank you for considering contributing to Vitalis! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- **Rust** (Edition 2024, stable toolchain)
- **Python 3.10+** (for the Python wrapper, optional)
- **Git**

### Building from Source

```bash
git clone https://github.com/ModernOps888/vitalis.git
cd vitalis
cargo build
```

### Running Tests

```bash
cargo test
```

### Running a `.sl` Program

```bash
cargo run -- run examples/hello.sl
```

## How to Contribute

### Reporting Bugs

- Open an issue on GitHub with a clear title and description
- Include the `.sl` source code that triggers the bug
- Include the full error output
- Mention your OS and Rust version (`rustc --version`)

### Suggesting Features

- Open an issue with the `enhancement` label
- Describe the feature and why it would be useful
- If possible, include syntax examples for language features

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run `cargo test` and ensure all tests pass
5. Run `cargo clippy` and fix any warnings
6. Submit a pull request

### Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Use `cargo clippy` for linting
- Write doc comments for public APIs
- Add tests for new functionality
- Keep functions focused and well-named

## Architecture Overview

```
Source (.sl) → Lexer → Parser → AST → TypeChecker → IR → Optimizer → Cranelift JIT/AOT → Native
```

**61 modules**, ~45,703 LOC, 1,177 tests. Key modules:

| Module | Purpose |
|--------|---------|
| `lexer.rs` | Logos-based zero-copy tokenizer (~70 token variants) |
| `parser.rs` | Recursive-descent + Pratt parser → AST |
| `ast.rs` | 30+ expression variants with origin tracking |
| `types.rs` | Structural type checker with capability annotations |
| `generics.rs` | Generic functions/structs, monomorphization |
| `ownership.rs` | Borrow checker (Owned/Moved/Borrowed/Dropped) |
| `lifetimes.rs` | Region analysis, lifetime constraints |
| `nll.rs` | Non-lexical lifetimes — CFG, liveness, NLL borrow regions |
| `effects.rs` | Effect system (IO/Net/FS/Async/GPU capabilities) |
| `ir.rs` | SSA-form intermediate representation |
| `optimizer.rs` | Constant folding, DCE, CSE, loop tiling, inlining |
| `codegen.rs` | Cranelift 0.116 JIT backend (~3,853 lines) |
| `aot.rs` | AOT compilation → native binaries |
| `cross_compile.rs` | x86-64 / AArch64 / RISC-V targets |
| `stdlib.rs` | ~196 built-in functions |
| `evolution.rs` | `@evolvable` function registry + rollback |
| `engine.rs` | Autonomous evolution cycle runner |
| `hotpath.rs` | Native Rust fast-path operations (44 ops) |
| `bridge.rs` | C FFI exports for Python/C interop |
| `lsp.rs` | Language Server Protocol (diagnostics, hover, completion) |

See the full module map in [`README.md`](README.md).

## Areas Where Help Is Welcome

- **Standard library expansion** — new built-in functions
- **Language features** — effect handlers, pattern exhaustiveness
- **Platform support** — Linux/macOS builds and CI
- **Documentation** — tutorials, language guide, API docs
- **Benchmarks** — comparative benchmarks with other JIT languages
- **Editor support** — VS Code extension, syntax highlighting
- **Package manager** — registry and dependency resolution

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
