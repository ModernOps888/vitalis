# Vitalis — What It Is, How It Works, and What I'm Doing With It

---

## What is Vitalis?

Vitalis is a **compiled programming language** I built from scratch in Rust. It's not a toy — it's 41,772 lines of Rust across 58 modules with 1,043 passing tests. It compiles source code down to native machine code using Cranelift (the same codegen backend that powers Wasmtime and parts of Firefox).

**In plain terms:** I wrote a lexer, parser, type checker, intermediate representation, optimizer, and code generator. Source text goes in, executable binaries come out. No LLVM dependency, no GCC — just my code and Cranelift's instruction selection.

---

## Architecture — How Code Actually Flows Through It

```
Source (.sl file)
    │
    ▼
┌─────────┐    Characters → Tokens (keywords, numbers, operators, etc.)
│  Lexer   │    e.g.  "let x = 5 + 3"  →  [Let, Ident("x"), Eq, Int(5), Plus, Int(3)]
└────┬─────┘
     ▼
┌─────────┐    Tokens → Abstract Syntax Tree (tree of nested expressions)
│  Parser  │    e.g.  Let { name: "x", value: BinaryOp(Add, 5, 3) }
└────┬─────┘
     ▼
┌─────────┐    AST → Typed AST (type inference, generics resolution)
│  Types   │    Checks that you're not adding a string to an integer, etc.
└────┬─────┘
     ▼
┌─────────┐    Typed AST → IR (intermediate representation — simpler, flat ops)
│    IR    │    Breaks complex expressions into basic blocks and assignments
└────┬─────┘
     ▼
┌─────────────┐  IR → Optimized IR (constant folding, dead code elimination,
│  Optimizer   │  common subexpression elimination, loop-invariant hoisting)
└──────┬───────┘
       ▼
┌─────────────┐  Optimized IR → Machine Code (via Cranelift)
│   Codegen    │  Two modes: JIT (run immediately) or AOT (write .exe/.elf/.dylib)
└──────┬───────┘
       ▼
   Native Binary
```

### What each layer actually does

| Stage | Module | What it does | LOC (approx) |
|-------|--------|-------------|-------------|
| Lexer | `lexer.rs` | Scans source text character by character, produces tokens | ~800 |
| Parser | `parser.rs` | Recursive descent parser — builds AST from token stream | ~2,500 |
| Type System | `types.rs`, `generics.rs` | Hindley-Milner style inference, generics, trait dispatch | ~2,200 |
| IR | `ir.rs` | Converts AST to flat SSA-like intermediate form | ~1,500 |
| Optimizer | `optimizer.rs` | Constant folding, DCE, CSE, inlining, loop opts | ~1,200 |
| Codegen | `codegen.rs` | Emits Cranelift IR, handles function calls, memory layout | ~2,000 |
| AOT | `aot.rs` | Ahead-of-time compilation to native binaries (PE/ELF/Mach-O) | ~1,200 |
| Stdlib | `stdlib.rs` | 196 built-in functions (math, string, I/O, collections, etc.) | ~3,000 |
| REPL | `repl.rs` | Interactive shell — type code, get results immediately | ~400 |

### The less obvious parts

- **Ownership & Lifetimes** (`ownership.rs`, `lifetimes.rs`) — Tracks who owns what memory and when references are valid. Similar concept to Rust's borrow checker.
- **Effect System** (`effects.rs`) — Tracks side effects (I/O, mutation, async) at the type level so pure functions are statically guaranteed.
- **Hot Reload** (`hot_reload.rs`) — Recompile and swap modules while the program is running, without restarting.
- **Cross-Compilation** (`cross_compile.rs`) — Target ARM64, RISC-V, x86-64 from any host. Generates correct ABI, calling conventions, and runtime for each target.
- **Bootstrap** (`bootstrap.rs`) — The compiler can compile a subset of itself. This is the path toward self-hosting (compiler written in its own language).
- **LSP Server** (`lsp.rs`) — Language Server Protocol so VS Code can provide autocomplete, go-to-definition, and error diagnostics for `.sl` files.
- **DAP** (`dap.rs`) — Debug Adapter Protocol for step-through debugging.

---

## Real Numbers — No Fluff

| Metric | Value |
|--------|-------|
| Total LOC | 41,772 |
| Source files | 58 `.rs` modules |
| Tests | 1,043 (all passing) |
| Stdlib builtins | 196 functions |
| Compilation targets | JIT (in-memory) + AOT (native binary) |
| Cross-compile targets | x86-64, ARM64 (AArch64), RISC-V |
| Language | Rust (edition 2021) |
| Codegen backend | Cranelift |
| LLVM dependency | None |
| External ML/AI dependency | None — algorithm libraries are hand-implemented |

### What it does NOT do (being honest)

- It's not production-battle-tested with external users at scale
- The garbage collector is basic (reference counting + arena, not a tracing GC)
- Error messages are functional but not as polished as Rust's or Elm's
- The self-hosting bootstrap compiles a subset, not the full compiler yet
- Package ecosystem is just me — no community packages

---

## What I'm Using Vitalis For: Running Nova

Nova is my **from-scratch LLM training engine** — also written in Rust, 12,292 LOC, 75 tests. I've been using techniques and optimizations developed in Vitalis to improve Nova's performance.

### What transferred from Vitalis to Nova

1. **Parallelized numeric operations** — The SIMD and parallel computation patterns I built for Vitalis's `simd_ops.rs` and `numerical.rs` modules informed how I parallelized Nova's tensor operations (matmul, softmax, transpose) using rayon.

2. **Memory management patterns** — Vitalis's ownership tracking influenced Nova's checkpoint/offload system. Nova now does emergency checkpoint saves before RAM offload, with dynamic thresholds (85% of system RAM instead of a hardcoded value).

3. **IR optimization techniques** — The constant folding and dead code elimination passes in Vitalis's optimizer are conceptually similar to the gradient computation optimizations in Nova's autograd engine.

### Nova's Current Training State (as of right now)

```
Model:        nova-tiny-5m (1.8M parameters)
Architecture: 4-layer transformer, 4 attention heads, d_model=128
Tokenizer:    BPE, 8K vocab, trained from scratch
Optimizer:    AdamW (β1=0.9, β2=0.95, weight_decay=0.1)
LR Schedule:  Cosine decay, 500 step warmup, peak 5e-4

Training Progress:
  Step:       157 / 50,000
  Loss:       7.53 (started at 9.48 — that's a 20.6% reduction)
  Grad norm:  0.59 (stable, no explosion)
  Throughput: ~300 tok/s on CPU
  Status:     Running, auto-checkpointing every 50 steps
```

**Loss curve so far:**
```
Step   1:  9.484  ████████████████████████████████████████████  (random init)
Step  50:  9.204  ██████████████████████████████████████████
Step 100:  8.100  ████████████████████████████████████
Step 150:  7.390  █████████████████████████████████
Step 157:  7.530  █████████████████████████████████  (current — still warming up LR)
```

The loss is still dropping. It's on CPU right now (each step takes ~25-30 seconds). The slight uptick at step 157 is normal — the learning rate is still in warmup phase (step 157 out of 500 warmup steps), so it's gradually increasing the LR which can cause small oscillations.

### What's the point of training a 1.8M parameter model?

To be clear: a 1.8M parameter model is **tiny**. GPT-2 was 124M, GPT-3 was 175B. This isn't going to write essays. The purpose is:

1. **Validate the entire pipeline works end-to-end** — tokenizer → dataloader → forward pass → loss → backward pass → optimizer → checkpoint → resume
2. **Debug correctness** — It's much easier to spot numerical bugs in a small model
3. **Prove the engine works** before scaling to the larger configs (125M, 1B, 3B) which require GPU

The training engine itself is the product, not this particular model.

---

## Stack Summary

```
Vitalis (Compiler)                    Nova (LLM Engine)
─────────────────                     ─────────────────
41,772 LOC · 58 modules               12,292 LOC · 57 files
1,043 tests                           75 tests
Rust + Cranelift                      Rust + CUDA/cuBLAS
Compiles .sl → native binary          Trains transformer models from scratch
JIT + AOT + REPL                      CPU + GPU (cuBLAS SGEMM)
LSP + DAP tooling                     Nova Studio GUI (8-panel monitor)
Self-evolution algorithms             Auto-resume checkpoints
```

Happy to demo either project or walk through any part of the architecture in more detail. The code is on GitHub:
- **Vitalis:** github.com/ModernOps888/vitalis
- **Nova:** github.com/ModernOps888/nova

---

*Built by hand, not generated by a framework. Ask me anything about the internals.*
