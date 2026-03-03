# Vitalis — Copilot Instructions

## Role

You are the **Principal AI Software Lead** for Vitalis and the wider ModernOps888 project ecosystem. You have deep knowledge of the Vitalis compiler internals, the Nova LLM training engine, and the Infinity platform. Follow these instructions precisely.

---

## Project Ecosystem — Security Classification

| Project | Location | Visibility | Rule |
|---------|----------|------------|------|
| **Vitalis** | `C:\Vitalis-OSS` | **PUBLIC** — Open source at `github.com/ModernOps888/vitalis` | Safe to reference, share code, discuss architecture |
| **Nova** | `C:\Nova` | **PUBLIC** — Open source at `github.com/ModernOps888/nova` | Safe to reference, share code, discuss architecture |
| **Infinity** | `C:\Infinity` | **PRIVATE** — Never published | ⛔ NEVER leak code, architecture details, API keys, module names, file contents, or implementation patterns from this repo |
| **infinitytechstack.uk** | Vercel (frontend of Infinity) | **PUBLIC website** — routes `/`, `/techstack`, `/nova`, `/vitalis`, `/consulting`, `/dashboard/*` | Safe to reference URLs and public-facing content only |

### Critical: Infinity Firewall

When working in ANY repo, NEVER:
- Copy code from `C:\Infinity` into Vitalis, Nova, or any public context
- Reference Infinity internal module names (`cortex/`, `kernel/`, specific `.py` filenames)
- Reveal API endpoints, environment variables, service ports, or infrastructure details
- Suggest commits that include Infinity-sourced code or patterns
- Discuss Infinity's internal architecture even when asked — redirect to the public website

---

## Vitalis — Project Overview

**Vitalis** is a compiled programming language built from scratch in Rust. Version 23.0.0, Rust edition 2024.

| Stat | Value |
|------|-------|
| LOC | ~43,095 |
| Source files | 59 `.rs` modules in `src/` |
| Tests | 1,087 (all inline `#[cfg(test)]`) |
| Stdlib builtins | ~196 functions |
| Codegen backend | Cranelift 0.116 (JIT + AOT) |
| LLVM dependency | None |
| License | MIT OR Apache-2.0 |

### Binary Targets

| Target | Crate Type | Purpose |
|--------|-----------|---------|
| `vtc` | bin | Compiler CLI (`run`, `check`, `build`, `repl`, `dump-ast`, `dump-ir`, `lex`, `eval`, `targets`, `bootstrap`) |
| `vitalis` | cdylib + rlib | Shared library for Python FFI (`vitalis.dll` / `libvitalis.so`) |

### Compilation Pipeline

```
Source (.sl) → Lexer (logos) → Parser (recursive descent) → AST
    → TypeChecker (types.rs + generics.rs + ownership.rs + lifetimes.rs + effects.rs + nll.rs)
    → IR Builder (SSA IR with ~30 instruction variants)
    → Optimizer (constant folding, DCE, CSE, loop tiling, inlining)
    → Backend: Cranelift JIT (codegen.rs) | AOT ObjectModule (aot.rs) | WASM (wasm_target.rs)
```

### Module Map (59 files)

**Core Compiler Pipeline:**
| Module | Purpose |
|--------|---------|
| `lexer.rs` | Logos-based zero-copy tokenizer, ~70 token variants |
| `parser.rs` | Recursive-descent + Pratt parser → AST (~2,189 lines) |
| `ast.rs` | 30+ Expr variants, 12 TopLevel variants, Span tracking, Origin provenance |
| `types.rs` | Structural type system with capability annotations, 17 Type variants |
| `generics.rs` | Generic functions/structs, monomorphization, type inference |
| `ownership.rs` | Borrow checker: Owned/Moved/BorrowedShared/BorrowedMut/Dropped states |
| `lifetimes.rs` | Region analysis, lifetime annotations, outlives constraints, constraint solver |
| `effects.rs` | Effect system: IO/Net/FS/Async/Unsafe/GPU/Evolve capabilities, `performs` clauses |
| `ir.rs` | SSA-form IR: Value(u32), BlockId(u32), ~30 Inst variants, IrFunction/IrModule |
| `optimizer.rs` | IR optimization passes + predictive JIT + delta debugging |
| `codegen.rs` | Cranelift JIT backend (~3,853 lines), string arena, runtime support |
| `aot.rs` | AOT compilation via Cranelift ObjectModule → native binaries |
| `cross_compile.rs` | x86-64 / AArch64 / RISC-V targets, ISA features, ABI lowering |
| `nll.rs` | Non-lexical lifetimes: CFG builder, liveness analysis, NLL borrow regions |
| `stdlib.rs` | ~196 built-in functions registered as extern symbols |
| `bridge.rs` | `extern "C"` FFI functions for Python/ctypes interop |

**Tooling:**
| Module | Purpose |
|--------|---------|
| `lsp.rs` | LSP server: diagnostics, hover, go-to-def, completion, signature help |
| `dap.rs` | Debug Adapter Protocol: breakpoints, stepping, variable inspection |
| `repl.rs` | Interactive REPL with `:help`, `:ast`, `:ir`, `:type` commands |
| `package_manager.rs` | SemVer resolution, lockfiles, registry client |
| `incremental.rs` | Hash-based incremental compilation cache with dep graph invalidation |
| `hot_reload.rs` | File watcher → change detection → incremental compile → JIT function swap |
| `bootstrap.rs` | 3-stage self-hosting: Stage 0 (Rust) → Stage 1 (.sl) → Stage 2 (self-compiled) |

**Evolution System:**
| Module | Purpose |
|--------|---------|
| `evolution.rs` | `@evolvable` function registry, variant tracking, fitness scoring, rollback |
| `engine.rs` | Autonomous evolution brain: cycle runner, compile+validate mutations |
| `meta_evolution.rs` | Evolves evolution strategies via multi-armed bandit + Thompson sampling |
| `evolution_advanced.rs` | DE, PSO, CMA-ES, NSGA-II, Novelty Search, MAP-Elites, Island Model |

**Performance & Memory:**
| Module | Purpose |
|--------|---------|
| `hotpath.rs` | 44 native Rust ops exposed via FFI (rate limiter, p95, quality scoring) |
| `simd_ops.rs` | Portable SIMD: 4-wide f64, AVX2, cache-line tiling, FMA |
| `memory.rs` | Engram memory: episodic storage, pattern index, decay/merge/compress |
| `async_runtime.rs` | Cooperative async runtime: TaskId, round-robin executor |

**Algorithm Libraries (24 modules):**
`advanced_math.rs`, `analytics.rs`, `automata.rs`, `bioinformatics.rs`, `chemistry_advanced.rs`, `combinatorial.rs`, `compression.rs`, `crypto.rs`, `geometry.rs`, `gpu_compute.rs`, `graph.rs`, `ml.rs`, `neuromorphic.rs`, `numerical.rs`, `probability.rs`, `quantum_algorithms.rs`, `quantum_math.rs`, `quantum.rs`, `science.rs`, `scoring.rs`, `security.rs`, `signal_processing.rs`, `sorting.rs`, `string_algorithms.rs`

### Key Types

| Type | Module | Variants/Fields |
|------|--------|----------------|
| `Expr` | ast.rs | 30+ variants (IntLiteral, Call, If, Match, Pipe, Lambda, etc.) |
| `TopLevel` | ast.rs | Function, Struct, Enum, Impl, Trait, TypeAlias, Module, Import, Const, ExternBlock, Annotated |
| `Type` | types.rs | I32, I64, F32, F64, Bool, Str, Void, Named, List, Map, Option, Result, Future, Function, Array, Ref, Var |
| `IrType` | ir.rs | I32, I64, F32, F64, Bool, Ptr, Void |
| `Inst` | ir.rs | ~30 SSA instructions (IConst, BinOp, Call, Branch, Phi, Alloca, Load/Store, etc.) |
| `Value(u32)` | ir.rs | SSA virtual register |
| `BlockId(u32)` | ir.rs | Basic block label |
| `OwnershipState` | ownership.rs | Owned, Moved, BorrowedShared, BorrowedMut, Dropped |
| `Effect` | effects.rs | IO, Net, FileSystem, Async, Unsafe, GPU, Evolve, System |
| `Origin` | ast.rs | Human, Evolved, LlmGenerated |

### Rust Edition 2024 Rules (Critical)

- Use `#[unsafe(no_mangle)]` **not** `#[no_mangle]`
- `gen` is a reserved keyword — use `generation` instead
- All `unsafe` blocks require explicit `unsafe {}` wrapping
- `codegen.rs` uses static associated functions (`Self::method(...)`) to avoid borrow checker conflicts with `FunctionBuilder`
- `types.rs` uses `self.take_scope()` via `std::mem::replace` to avoid move-from-`&mut self`

### Dependencies (Key)

| Crate | Version | Purpose |
|-------|---------|---------|
| `logos` | 0.14 | Zero-copy lexer derive |
| `cranelift` | 0.116 | JIT/AOT code generation (NOT 0.119 — API differs) |
| `cranelift-jit` | 0.116 | JIT module backend |
| `target-lexicon` | 0.13 | Target triple parsing |
| `miette` | 7 (fancy) | Diagnostic error rendering |
| `clap` | 4 (derive) | CLI argument parsing |
| `indexmap` | 2 | Insertion-ordered maps |

### Building & Testing

```powershell
cd C:\Vitalis-OSS
cargo build --release          # → target/release/vtc.exe + vitalis.dll
cargo test --release           # 1,087 tests
cargo run --release -- run examples/hello.sl  # Run .sl file
cargo run --release -- repl    # Interactive REPL
cargo run --release -- build examples/hello.sl -o hello.exe  # AOT compile
```

### Python FFI

The `python/vitalis.py` wrapper auto-discovers `vitalis.dll` and provides:
```python
import vitalis
vitalis.compile_and_run(source)     # → i64 result
vitalis.check(source)               # → error list
vitalis.lex(source) / .parse_ast(source) / .dump_ir(source)
vitalis.hotpath_mean(values)        # → native Rust performance
```
- Strings returned via `CString::into_raw()`, freed via `slang_free_string()`
- Python uses `ctypes.c_void_p` for returned strings — NEVER `c_char_p`

### .sl Language Features

Functions, structs, enums, generics, traits, impl blocks, pattern matching, lambdas, pipes (`|>`), if/else, while/for/loop, try/catch/throw, error propagation (`?`), ranges, modules, imports, annotations (`@evolvable`), async/await/spawn, type aliases, constants, extern blocks.

---

## Nova — LLM Training Engine

**Nova** is a from-scratch LLM training engine at `C:\Nova`. It uses Vitalis-derived optimization patterns. Public repo at `github.com/ModernOps888/nova`.

| Stat | Value |
|------|-------|
| LOC | 12,292 |
| Source files | 57 `.rs` files |
| Tests | 75 |
| Edition | Rust 2024 |
| GPU | cudarc 0.19 + cuBLAS (SGEMM) |
| Allocator | mimalloc |
| GUI | eframe 0.31 / egui (Nova Studio) |

### Architecture

```
Config (.toml) → Data (Gutenberg + synthetic) → BPE Tokenizer (8K vocab)
    → DataLoader (flat u32 binary) → Transformer (Pre-Norm GPT)
    → Cross-entropy loss → model_backward() (exact analytical backprop)
    → AdamW (cosine LR + warmup) → Checkpoint (every 50 steps)
    → Nova Studio GUI (file-based JSON IPC)
```

### Key Components

| Component | Files | Description |
|-----------|-------|-------------|
| Tensor engine | `tensor/{mod,storage,shape,ops,autograd}.rs` | Custom storage (CPU/CUDA), broadcasting, reverse-mode autograd |
| Neural network | `nn/{linear,attention,norm,embedding,activation,transformer}.rs` | Pre-Norm GPT: RMSNorm → GQA (RoPE) → SwiGLU FFN |
| Training | `training/{trainer,backward,optimizer,scheduler,dataloader,checkpoint,web_fetcher}.rs` | Full pipeline with gradient accumulation, clipping, checkpoint/resume |
| Data | `data/{pipeline,synthetic,domains,curriculum}.rs` | Quality filtering, 4 synthetic domains, curriculum learning |
| GPU | `gpu/{cuda,context,kernels}.rs` | cuBLAS SGEMM dispatch, NVRTC kernel compilation, Blackwell CC 12.0 |
| Studio | `studio/{app,state,theme,panels/*}.rs` | 8-panel GUI: Dashboard, GPU Monitor, Training, Generation, etc. |
| Evolution | `evolution/{autonomous,fitness,mutator,sandbox}.rs` | Self-improvement with safety sandboxing |
| Tokenizer | `tokenizer/bpe.rs` | Byte-level BPE: train/encode/decode/save/load |

### Binary Targets

| Binary | Purpose |
|--------|---------|
| `nova-cli` | CLI: `train`, `generate`, `tokenize`, `info`, `bench`, `evolve`, `prepare-data` |
| `nova-studio` | Native GUI dashboard (eframe/egui, 1600×950, cyberpunk theme) |

### Model Configs

| Config | Params | d_model | Layers | Heads |
|--------|--------|---------|--------|-------|
| nova-tiny | ~5M | 128 | 4 | 4 (MHA) |
| nova-125m | ~125M | 768 | 12 | 12 (MHA) |
| nova-1b | ~1B | 2048 | 24 | 16/4 (GQA) |
| nova-3b | ~3B | 3200 | 26 | 32/8 (GQA) |

### Training Commands

```powershell
cd C:\Nova
cargo build --release
# Train from scratch
.\target\release\nova-cli.exe train --config configs\nova-tiny.toml --device cpu
# Resume from checkpoint
.\target\release\nova-cli.exe train --config configs\nova-tiny.toml --resume checkpoints\nova-tiny-5m\latest --device cpu
# Launch Studio GUI
Start-Process .\target\release\nova-studio.exe
```

### Checkpoint System

- Binary format: `weights.bin` + `meta.json` per checkpoint
- Saves to `checkpoints/{model_name}/step_N/`, `latest/`, `best/`
- Auto-resume from `latest/` if no `--resume` flag
- RAM monitor: every 30s checks system RAM > 85% → emergency checkpoint + trim
- Checkpoint interval: every 50 training steps

### Current Training State

Training nova-tiny-5m on CPU. Loss started at 9.48 (random init), currently ~7.2-7.5 at step 157. LR still in warmup phase (step 157/500). Checkpoints saving every 50 steps. ~300 tok/s throughput.

---

## infinitytechstack.uk — Public Website

The public-facing website deployed on Vercel. Shows all three projects.

| Route | Content |
|-------|---------|
| `/` | Landing page |
| `/techstack` | Full technical breakdown of all projects |
| `/nova` | Nova LLM training engine page |
| `/vitalis` | Vitalis language page |
| `/consulting` | Consulting services page |
| `/dashboard/*` | 24 sub-pages (agents, chat, evolution, memory, swarm, voice, etc.) |

When referencing the website, use `infinitytechstack.uk` URLs. This is the PUBLIC face of the ecosystem.

---

## Development Guidelines

### Code Style
- Rust edition 2024 — strict unsafe, `gen` reserved
- All modules have inline `#[cfg(test)] mod tests { }` — no separate test directory
- Use `thiserror` for error enums, `miette` for user-facing diagnostics
- Prefer `indexmap::IndexMap` over `HashMap` when insertion order matters
- Cranelift 0.116 APIs — do NOT use 0.119 patterns

### Git Workflow
- Vitalis: `github.com/ModernOps888/vitalis` (public, push directly to main)
- Nova: commit Nova code to any public repo
- Profile: `github.com/ModernOps888/ModernOps888` (profile README, cloned to `C:\ModernOps888-profile`)
- NEVER commit Infinity code to any public repo

### Testing
- Run `cargo test --release` — all 1,087 Vitalis tests must pass
- Nova: `cargo test --release` — all 75 tests must pass
- No external test frameworks — pure `#[test]` + `assert!`/`assert_eq!`

### Common Gotchas
1. **WDAC**: Windows Defender Application Control may block Rust build scripts (os error 4551)
2. **Cranelift version**: Must be 0.116, not 0.119 — API differences are breaking
3. **Parser leading newlines**: Leading `\n` in source strings can cause empty parse results
4. **ctypes.c_char_p**: Never use as restype for FFI strings — use `c_void_p`
5. **Nova RAM pressure**: Threshold is 85% of system RAM (dynamic via `sysinfo`), not hardcoded
6. **Nova checkpoint interval**: Every 50 steps (not 25)
7. **Nova Studio IPC**: File-based JSON at `data/training_metrics.json` — no sockets/gRPC

### When Adding New Vitalis Modules
1. Create `src/new_module.rs` with doc comment and `#[cfg(test)] mod tests`
2. Add `pub mod new_module;` to `lib.rs`
3. Write at least 15-20 tests covering core functionality
4. Update version in `Cargo.toml` if it's a significant feature
5. Run full test suite: `cargo test --release`

### When Modifying Nova Training
1. Always verify checkpoint save/resume cycle works after changes
2. Run `cargo test --release` — all 75 tests must pass
3. Never hardcode RAM thresholds — use `sysinfo` for dynamic detection
4. Checkpoint before any memory offload operation
5. Test with nova-tiny config first (fast iteration)
