<div align="center">

<!-- ═══════════════════════════════════════════════════════════ -->
<!--                     VITALIS HEADER                         -->
<!-- ═══════════════════════════════════════════════════════════ -->

# 🧬 Vitalis

### The Self-Evolving Programming Language

[![Rust](https://img.shields.io/badge/Rust-Edition_2024-b7410e?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-748_Passing-00c853?style=for-the-badge&logo=checkmarx&logoColor=white)](#-test-suite)
[![LOC](https://img.shields.io/badge/LOC-35%2C632-blue?style=for-the-badge&logo=slickpic&logoColor=white)](#-architecture)
[![License](https://img.shields.io/badge/License-MIT-yellow?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](LICENSE)
[![Version](https://img.shields.io/badge/v20.0.0-purple?style=for-the-badge&logo=v&logoColor=white)](#-changelog)

**A compiled language purpose-built for autonomous AI code evolution.**<br>
Vitalis compiles to native machine code via Cranelift JIT, with first-class support for<br>
self-modifying programs, genetic code evolution, and real-time fitness tracking.

*Written from scratch in Rust. No LLVM. No interpreter. No VM. Pure native JIT.*

<br>

> **`fn main() -> i64 { println("Hello, Evolution."); 42 }`**

<br>

[Quick Start](#-quick-start) · [Language Guide](#-language-guide) · [Architecture](#-architecture) · [API Reference](#-api-reference) · [Benchmarks](#-performance)

</div>

---

<br>

## 📊 At a Glance

<table>
<tr>
<td width="25%" align="center">

**47**<br>
<sub>Source modules</sub>

</td>
<td width="25%" align="center">

**35,632**<br>
<sub>Lines of Rust</sub>

</td>
<td width="25%" align="center">

**748**<br>
<sub>Tests passing</sub>

</td>
<td width="25%" align="center">

**42**<br>
<sub>FFI exports</sub>

</td>
</tr>
</table>

<br>

## 🏗 Architecture

The compiler transforms source code through six stages, each producing a well-defined intermediate form:

```mermaid
flowchart TB
    A["📄 Source Code\n.sl files"] -->|tokenize| B["🔤 LEXER\nlexer.rs"]
    B -->|parse| C["🌳 PARSER\nparser.rs"]
    C -->|validate| D["✅ TYPE CHECKER\ntypes.rs"]
    D -->|lower| E["📐 IR BUILDER\nir.rs"]
    E -->|compile| F["⚡ CODEGEN\ncodegen.rs"]
    F -->|emit| G["🖥️ NATIVE MACHINE CODE\nx86-64"]

    style A fill:#2d1b69,stroke:#a855f7,stroke-width:3px,color:#f0e6ff,font-weight:bold
    style B fill:#1e3a5f,stroke:#60a5fa,stroke-width:2px,color:#dbeafe
    style C fill:#1e3a5f,stroke:#60a5fa,stroke-width:2px,color:#dbeafe
    style D fill:#1e3a5f,stroke:#60a5fa,stroke-width:2px,color:#dbeafe
    style E fill:#1e3a5f,stroke:#60a5fa,stroke-width:2px,color:#dbeafe
    style F fill:#4a1942,stroke:#f472b6,stroke-width:3px,color:#fce7f3,font-weight:bold
    style G fill:#0f3d1e,stroke:#4ade80,stroke-width:4px,color:#dcfce7,font-weight:bold
```

### Compiler Pipeline Detail

```mermaid
flowchart TB
    subgraph LEX ["🔤 STAGE 1 · LEXER"]
        direction TB
        L1(["📄 Source text"]) --> L2["Logos zero-copy tokenizer\n~80 token variants"]
        L2 --> L3(["Token stream"])
    end

    subgraph PARSE ["🌳 STAGE 2 · PARSER"]
        direction TB
        P1["Recursive descent + Pratt precedence"] --> P2["30+ Expr · 10 Stmt · 12 TopLevel"]
        P2 --> P3(["AST with Span tracking"])
    end

    subgraph TYPE ["✅ STAGE 3 · TYPE CHECKER"]
        direction TB
        T1["Pass 1 → Collect all signatures"] --> T2["Pass 2 → Check all bodies"]
        T2 --> T3(["Fully typed + validated AST"])
    end

    subgraph IR ["📐 STAGE 4 · IR BUILDER"]
        direction TB
        I1["SSA construction · 26+ instructions"] --> I2["Struct layout · Closures · Loops"]
        I2 --> I3(["SSA IR Module"])
    end

    subgraph CG ["⚡ STAGE 5 · CODEGEN"]
        direction TB
        C1["Cranelift 0.116 JIT"] --> C2["Register alloc · Instruction select"]
        C2 --> C3["204 extern C runtime functions"]
        C3 --> C4(["Native x86-64 machine code"])
    end

    subgraph FFI ["🐍 STAGE 6 · FFI BRIDGE"]
        direction TB
        F1["bridge.rs · extern C ABI"] --> F2["vitalis.dll / libvitalis.so"]
        F2 --> F3["vitalis.py · ctypes"]
        F3 --> F4(["Python interop"])
    end

    LEX ==> PARSE
    PARSE ==> TYPE
    TYPE ==> IR
    IR ==> CG
    CG ==> FFI

    style LEX fill:#0c1222,stroke:#38bdf8,stroke-width:2px,color:#e0f2fe
    style PARSE fill:#0c1222,stroke:#818cf8,stroke-width:2px,color:#e0e7ff
    style TYPE fill:#0c1222,stroke:#a78bfa,stroke-width:2px,color:#ede9fe
    style IR fill:#0c1222,stroke:#c084fc,stroke-width:2px,color:#f3e8ff
    style CG fill:#1a0c0c,stroke:#fb923c,stroke-width:3px,color:#fff7ed
    style FFI fill:#0c1a0c,stroke:#4ade80,stroke-width:3px,color:#dcfce7
```

<br>

### Module Map

Every source file has a single responsibility. The codebase is organized into **five layers**:

```mermaid
block-beta
    columns 4

    block:CORE:4
        columns 4
        A["⚙️ CORE COMPILER · 10,400 LOC"]:4
        B["lexer.rs\n637 lines"]
        C["parser.rs\n1,905 lines"]
        D["ast.rs\n594 lines"]
        E["types.rs\n929 lines"]
        F["ir.rs\n2,025 lines"]
        G["codegen.rs\n3,852 lines"]
        H["stdlib.rs\n290 lines"]
        I["bridge.rs\n845 lines"]
    end

    block:EVO:2
        columns 1
        J["🧬 EVOLUTION · 2,500 LOC"]
        K["evolution.rs\nGenerational tracking"]
        L["evolution_advanced.rs\nMulti-strategy"]
        M["meta_evolution.rs\nSelf-modifying"]
    end

    block:PERF:2
        columns 1
        N["🚀 PERFORMANCE · 5,800 LOC"]
        O["hotpath.rs · 2,106 lines"]
        P["simd_ops.rs · 846 lines"]
        Q["optimizer.rs · 1,294 lines"]
    end

    block:ML:4
        columns 4
        ML1["🤖 NOVA ML ENGINE · 3,600 LOC (v20.0)"]:4
        ML2["tensor_engine.rs\n700 lines"]
        ML3["deep_learning.rs\n600 lines"]
        ML4["gpu_compute.rs\n400 lines"]
        ML5["ml_training.rs\n500 lines"]
        ML6["bpe_tokenizer.rs\n300 lines"]
        ML7["model_inference.rs\n400 lines"]
        space3[""]
        space4[""]
    end

    block:MATH:4
        columns 4
        R["📊 DOMAIN LIBRARIES · 17,000 LOC"]:4
        S["ml.rs\nNeural nets"]
        T["quantum.rs\nCircuits"]
        U["graph.rs\nDijkstra"]
        V["numerical.rs\nODE · FFT"]
        W["signal.rs\nDSP"]
        X["bio.rs\nDNA"]
        Y["neuro.rs\nSpiking"]
        Z["crypto.rs\nSHA-256"]
    end

    CORE --> EVO
    CORE --> PERF
    CORE --> ML
    CORE --> MATH

    style CORE fill:#0c1222,stroke:#38bdf8,stroke-width:3px,color:#bae6fd
    style EVO fill:#1a0c22,stroke:#e879f9,stroke-width:3px,color:#f5d0fe
    style PERF fill:#1a0c0c,stroke:#fb923c,stroke-width:3px,color:#fed7aa
    style ML fill:#0c1a1a,stroke:#22d3ee,stroke-width:3px,color:#cffafe
    style MATH fill:#0c1a22,stroke:#a78bfa,stroke-width:3px,color:#ddd6fe
    style A fill:#1e3a5f,stroke:#38bdf8,color:#e0f2fe
    style J fill:#2d1042,stroke:#e879f9,color:#f5d0fe
    style N fill:#2d1a0a,stroke:#fb923c,color:#fed7aa
    style ML1 fill:#0a2d2d,stroke:#22d3ee,color:#cffafe
    style R fill:#1a1040,stroke:#a78bfa,color:#ddd6fe
```

<br>

## 🚀 Quick Start

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| **Rust** | nightly / stable 1.85+ | Edition 2024 compiler |
| **Python** | 3.12+ | FFI wrapper (`vitalis.py`) |

### Build & Test

```bash
# Clone
git clone https://github.com/ModernOps888/vitalis.git
cd vitalis

# Build compiler + DLL
cargo build

# Run all 748 tests
cargo test

# Compile and run a .sl file
cargo run -- run examples/hello.sl
```

### Hello World

```rust
// hello.sl — Your first Vitalis program
fn main() -> i64 {
    println("Hello from Vitalis!");
    
    let x: i64 = 40;
    let y: i64 = 2;
    x + y
}
```

```bash
$ vtc run hello.sl
Hello from Vitalis!
42
```

<br>

## 📖 Language Guide

### Type System

| Type | Description | Example |
|------|-------------|---------|
| `i64` | 64-bit signed integer | `42` |
| `f64` | 64-bit float | `3.14` |
| `bool` | Boolean | `true` / `false` |
| `str` | Interned string | `"hello"` |
| `[i64]` | Heap array | `[1, 2, 3]` |

### Variables & Mutability

```rust
let x: i64 = 10;         // immutable binding
let mut count: i64 = 0;  // mutable — can reassign
count = count + 1;
```

### Control Flow

```rust
// If / else (expression — returns a value)
let val: i64 = if x > 0 { x } else { -x };

// While loop
let mut i: i64 = 0;
while i < 10 {
    println(to_string_i64(i));
    i = i + 1;
}

// For-each over arrays
let arr: [i64] = [10, 20, 30];
for item in arr {
    println(to_string_i64(item));
}

// Match expression
let result: i64 = match x {
    1 => 100,
    2 => 200,
    _ => 0,
};

// Break / Continue
while true {
    if done { break; }
    if skip { continue; }
}
```

### Functions

```rust
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn greet(name: str) {
    println(str_cat("Hello, ", name));
}

// Closures / Lambdas
let double = |x: i64| -> i64 { x * 2 };
let result: i64 = double(21);  // → 42
```

### Structs & Impl Blocks

```rust
struct Rect {
    w: i64,
    h: i64,
}

impl Rect {
    fn area(self: Rect) -> i64 {
        self.w * self.h
    }
}

fn main() -> i64 {
    let r: Rect = Rect { w: 5, h: 3 };
    r.area()  // → 15
}
```

### Modules

```rust
module math {
    fn add(a: i64, b: i64) -> i64 { a + b }
    fn mul(a: i64, b: i64) -> i64 { a * b }
}

fn main() -> i64 {
    math::add(10, math::mul(3, 4))  // → 22
}
```

### Error Handling

```rust
// Try / Catch — expressions that return values
let result: i64 = try {
    let data: i64 = risky_operation();
    data * 2
} catch e {
    println(e);  // error message
    0            // fallback value
};

// Throw sets error state
throw(404, "not found");
```

### Async Functions (Stubs)

```rust
async fn fetch_data() -> i64 {
    let result: i64 = await compute();
    result
}
```

### Collections

```rust
// Arrays — heap-allocated, variable length
let arr: [i64] = [1, 2, 3];
let pushed = arr.push(4);        // → [1, 2, 3, 4]
let found: i64 = arr.find(2);    // → 1 (index)
let sorted = arr.sort();         // → [1, 2, 3, 4]
let sliced = arr.slice(0, 2);    // → [1, 2]
let has_it = arr.contains(3);    // → true
let reversed = arr.reverse();    // → [3, 2, 1]
let joined: str = arr.join(","); // → "1,2,3"
let popped: i64 = arr.pop();    // → 3

// Functional operations
let nums = array_range(1, 100);
let total = array_sum(nums);
let smallest = array_min(nums);
let biggest = array_max(nums);
let first5 = array_take(nums, 5);
let rest = array_drop(nums, 5);
let deduped = array_unique(nums);
let counted = array_count(nums, 42);

// Maps — key-value store
let m: i64 = map_new();
map_set(m, "name", 42);
let val: i64 = map_get(m, "name");
let exists: bool = map_has(m, "name");

// Sets — unique element collection
let s: i64 = set_new();
set_add(s, 10);
set_add(s, 20);
let has: bool = set_has(s, 10);       // → true
let count: i64 = set_len(s);          // → 2
let union: i64 = set_union(s1, s2);
let inter: i64 = set_intersect(s1, s2);
let diff: i64 = set_diff(s1, s2);

// Tuples — fixed-size immutable groups
let t = tuple_new3(10, 20, 30);
let first: i64 = tuple_get(t, 0);    // → 10
let size: i64 = tuple_len(t);        // → 3
```

### String Operations

```rust
let s: str = "Hello, World!";
let upper: str = s.to_upper();        // → "HELLO, WORLD!"
let lower: str = s.to_lower();        // → "hello, world!"
let trimmed: str = s.trim();
let has: bool = str_contains(s, "World");
let idx: i64 = s.index_of("World");   // → 7
let sub: str = s.substring(0, 5);     // → "Hello"
let rep: str = s.replace("World", "Vitalis");
let len: i64 = str_len(s);            // → 13

// Formatting
let msg = str_format_i64("value = {}", 42);
let pi = str_format_f64("pi = {}", 3.14159);
let greeting = str_format_str("Hello, {}!", "world");

// Conversion
let num_str: str = to_string_i64(42);
let parsed: i64 = parse_int("123");
```

### Regex

```rust
let matched = regex_is_match("\\d+", "abc123");     // → 1
let full = regex_match("^hello$", "hello");          // → 1
let found: str = regex_find("\\d+", "abc123def");    // → "123"
let replaced = regex_replace("\\d+", "a1b2c3", "X"); // → "aXbXcX"
let parts = regex_split_count(",", "a,b,c,d");       // → 4
```

### File I/O

```rust
file_write("output.txt", "Hello from Vitalis!");
let content: str = file_read("output.txt");
let exists: bool = file_exists("output.txt");
let size: i64 = file_size("output.txt");
file_append("log.txt", "new line\n");
file_delete("temp.txt");
```

### Networking (HTTP)

```rust
let body: str = http_get("https://api.example.com/data");
let resp: str = http_post("https://api.example.com/submit", "{\"key\":\"val\"}");
let status: i64 = http_status("https://example.com");
```

<br>

## 🧬 Evolution System

Vitalis's signature feature: **programs that evolve themselves.**

```mermaid
flowchart TB
    R(["📝 REGISTER\n@evolvable fn"]) ==> M

    M["🧪 MUTATE\nsource code"] ==> E["📊 EVALUATE\nfitness score"]
    E ==> S{"🏆 Fitter?"}

    S -->|"✅ Yes"| P["⬆️ PROMOTE\nnew generation"]
    S -->|"❌ No"| K["⬇️ ROLLBACK\nkeep previous"]

    P -.->|"next cycle"| M
    K -.->|"retry"| M

    style R fill:#2d1b69,stroke:#a855f7,stroke-width:3px,color:#f0e6ff,font-weight:bold
    style M fill:#1e3a5f,stroke:#60a5fa,stroke-width:2px,color:#dbeafe
    style E fill:#1e3a5f,stroke:#818cf8,stroke-width:2px,color:#e0e7ff
    style S fill:#4a3800,stroke:#fbbf24,stroke-width:3px,color:#fef3c7,font-weight:bold
    style P fill:#0f3d1e,stroke:#4ade80,stroke-width:3px,color:#dcfce7,font-weight:bold
    style K fill:#3d0f0f,stroke:#f87171,stroke-width:3px,color:#fee2e2,font-weight:bold
```

```rust
// Mark a function as evolvable
@evolvable
fn optimize(data: [i64]) -> i64 {
    array_sum(data)
}

// The evolution engine can:
// 1. Register variants
// 2. Track generational history
// 3. Measure fitness scores
// 4. Rollback to previous generations
// 5. Meta-evolve the evolution strategy itself
```

### Evolution API (Python FFI)

```python
import vitalis

# Register a function for evolution
vitalis.evo_register("sort", "fn sort(arr: [i64]) -> [i64] { arr }")

# Evolve to a new generation
gen = vitalis.evo_evolve("sort", new_source)

# Set fitness score
vitalis.evo_set_fitness("sort", 0.95)

# Rollback if needed
vitalis.evo_rollback("sort", previous_gen)
```

<br>

## 🔬 What Makes Vitalis Unique

Eight features that no other language combines:

```mermaid
mindmap
  root(("🧬 VITALIS"))
    ::icon(🧬)
    🧬 **Code Evolution**
      @evolvable functions
      Generational tracking
      Fitness-driven mutation
      Automatic rollback on regression
    🧠 **Engram Memory**
      Persistent cross-session state
      Pattern recall and recognition
      Temporal decay and reinforcement
    🔄 **Meta-Evolution**
      Strategies that evolve themselves
      Multi-strategy tournament selection
      Adaptive mutation rates via EMA
    ⚡ **Cranelift JIT**
      Native x86-64 machine code
      Hot-path bypass to Rust
      SIMD vectorized operations
    🤖 **Nova ML Engine**
      Tensor ops with autograd
      Transformer architectures
      GPU compute with CUDA kernels
      BPE tokenizer + training loop
    🔐 **Capability Safety**
      Sandboxed execution environment
      Permission-based file and network I/O
      Asimov's Laws enforcement layer
    🧪 **Origin Tracking**
      Every AST node carries Span
      Full provenance chain to source
      Debug trace through IR to native
    🔗 **Pipe Operator**
      Declarative data flow pipelines
      Functional stage composition
      First-class pipeline values
```

<br>

## 🤖 Nova ML Engine <sup>v20.0</sup>

Vitalis v20.0 ships a **complete deep learning engine** — tensors, transformer architectures,
GPU compute, training infrastructure, BPE tokenization, and inference — all in pure Rust,
exposed through FFI for use from Vitalis programs and Python.

### Architecture

```mermaid
flowchart TB
    T["🔢 TENSOR ENGINE\ntensor_engine.rs\n33+ ops · autograd"] --> DL
    DL["🧠 DEEP LEARNING\ndeep_learning.rs\nRMSNorm · RoPE · GQA · SwiGLU"] --> TR
    GPU["⚡ GPU COMPUTE\ngpu_compute.rs\n11 CUDA kernels"] --> TR
    BPE["📝 BPE TOKENIZER\nbpe_tokenizer.rs\ntrain · encode · decode"] --> TR
    TR["🏋️ ML TRAINING\nml_training.rs\nAdamW · cosine scheduler · backprop"] --> INF
    INF["🎯 INFERENCE\nmodel_inference.rs\ntop-k · top-p · temperature"]

    style T fill:#0c1a2d,stroke:#22d3ee,stroke-width:3px,color:#cffafe,font-weight:bold
    style DL fill:#0c1a2d,stroke:#22d3ee,stroke-width:2px,color:#cffafe
    style GPU fill:#1a0c0c,stroke:#f97316,stroke-width:3px,color:#fed7aa,font-weight:bold
    style BPE fill:#0c1222,stroke:#818cf8,stroke-width:2px,color:#e0e7ff
    style TR fill:#1a0c22,stroke:#e879f9,stroke-width:3px,color:#f5d0fe,font-weight:bold
    style INF fill:#0f3d1e,stroke:#4ade80,stroke-width:3px,color:#dcfce7,font-weight:bold
```

### Modules

| Module | LOC | Description |
|--------|-----|-------------|
| `tensor_engine.rs` | ~700 | N-dimensional tensors, 33+ ops (matmul, softmax, cross-entropy, embedding, GELU, SiLU), autograd with backward graph, broadcasting, Kaiming/Xavier init |
| `deep_learning.rs` | ~600 | `Linear`, `RMSNorm`, `LayerNorm`, `TokenEmbedding`, `MultiHeadAttention` (RoPE, GQA, causal mask), `SwiGLUFFN`, `GeluFFN`, `TransformerBlock`, full `Transformer` model with tied weights |
| `gpu_compute.rs` | ~400 | `DeviceInfo` (Blackwell/Ampere detection, BF16/FP16 support), `CudaRuntime`, `GpuMemoryPool`, 11 production PTX CUDA kernels (matmul, attention, softmax, RMSNorm, SwiGLU, GELU, embedding, AdamW, cross-entropy, RoPE) |
| `ml_training.rs` | ~500 | `AdamW` optimizer (decoupled weight decay), `CosineScheduler` + `WarmupConstantScheduler`, gradient clipping, full analytical backward pass through transformer layers, `DataLoader`, binary checkpoint save/load, `Trainer` with step logging |
| `bpe_tokenizer.rs` | ~300 | `BpeTokenizer` with `train` / `encode` / `decode` / `save` / `load`, special tokens (PAD/BOS/EOS/UNK), byte-level fallback, file serialization |
| `model_inference.rs` | ~400 | `ModelConfig` presets (tiny 5M → large 3B), `GenerateConfig` (temperature, top-k, top-p, repetition penalty), autoregressive `generate()` with structured `GenerationResult` |

### Model Presets

| Preset | Params | d_model | Layers | Heads | d_ff | Context |
|--------|--------|---------|--------|-------|------|---------|
| `tiny_5m` | ~1.8M | 128 | 4 | 4 | 344 | 512 |
| `small_125m` | ~125M | 768 | 12 | 12 | 2,048 | 2,048 |
| `medium_1b` | ~1B | 2,048 | 22 | 16 | 5,461 | 4,096 |
| `large_3b` | ~3B | 3,200 | 26 | 32 | 8,640 | 4,096 |

### CUDA Kernels (PTX)

11 production-ready kernel sources for GPU acceleration:

| Kernel | Purpose |
|--------|---------|
| `MATMUL_KERNEL` | Tiled FP32 matrix multiplication (16×16 tiles) |
| `MATMUL_FP16_KERNEL` | Half-precision matmul for Tensor Cores |
| `ATTENTION_KERNEL` | Fused multi-head attention with causal mask |
| `SOFTMAX_KERNEL` | Numerically stable softmax (row-wise) |
| `RMSNORM_KERNEL` | Root Mean Square normalization |
| `SWIGLU_KERNEL` | SwiGLU activation (gate × SiLU) |
| `GELU_KERNEL` | Gaussian Error Linear Unit |
| `EMBEDDING_KERNEL` | Token lookup with gather |
| `ADAMW_KERNEL` | Fused AdamW optimizer step |
| `CROSS_ENTROPY_KERNEL` | Cross-entropy loss with log-softmax |
| `ROPE_KERNEL` | Rotary Position Embedding |

### FFI Exports (ML)

```python
# Tensor operations
handle = vitalis.vitalis_tensor_zeros(rows, cols)     # Create zero tensor
handle = vitalis.vitalis_tensor_randn(rows, cols)     # Random normal tensor
result = vitalis.vitalis_tensor_matmul(a, b)          # Matrix multiply
value  = vitalis.vitalis_tensor_item(handle, idx)     # Get scalar value

# Model management
model  = vitalis.vitalis_transformer_new(preset)      # Create model (0=tiny, 1=small, 2=medium)
params = vitalis.vitalis_transformer_params(model)     # Parameter count

# Training
opt    = vitalis.vitalis_adamw_new(lr, b1, b2, eps, wd) # Create optimizer
lr     = vitalis.vitalis_cosine_lr(step, warmup, total, max_lr, min_lr)

# Tokenizer
tok    = vitalis.vitalis_tokenizer_new(vocab_size)    # Create BPE tokenizer
vocab  = vitalis.vitalis_tokenizer_vocab_size(tok)    # Actual vocab size

# GPU info
avail  = vitalis.vitalis_gpu_available()              # GPU detected?
vram   = vitalis.vitalis_gpu_memory_total()           # Total VRAM bytes
count  = vitalis.vitalis_gpu_kernel_count()           # Number of kernels

# Inference
params = vitalis.vitalis_model_params(preset)         # Estimated param count
vram   = vitalis.vitalis_model_vram(preset)           # Estimated VRAM bytes
token  = vitalis.vitalis_sample_token(logits, vocab, temp, top_k, seed)
```

<br>

## 📐 Standard Library

### 200+ Built-in Functions

<details>
<summary><b>🔢 Mathematics — 60+ functions</b></summary>

| Function | Description |
|----------|-------------|
| `sqrt`, `cbrt`, `pow`, `abs` | Basic math |
| `sin`, `cos`, `tan`, `asin`, `acos`, `atan` | Trigonometry |
| `sinh`, `cosh`, `tanh` | Hyperbolic |
| `ln`, `log2`, `log10`, `exp`, `exp2` | Logarithmic |
| `floor`, `ceil`, `round`, `trunc`, `fract` | Rounding |
| `min`, `max`, `clamp`, `lerp`, `smoothstep` | Interpolation |
| `gcd`, `lcm`, `factorial`, `fibonacci`, `is_prime` | Number theory |
| `sigmoid`, `relu`, `tanh`, `gelu`, `swish`, `mish` | Activation functions |
| `selu`, `elu`, `leaky_relu`, `softplus`, `softsign` | More activations |
| `rand_i64`, `rand_f64` | Random numbers |
| `fma`, `copysign`, `hypot`, `atan2` | IEEE 754 |
| `hash_i64`, `popcount`, `leading_zeros` | Bit operations |

</details>

<details>
<summary><b>📝 Strings — 20+ functions</b></summary>

| Function | Description |
|----------|-------------|
| `str_len`, `str_cat`, `str_eq` | Core |
| `to_upper`, `to_lower`, `trim` | Case & whitespace |
| `starts_with`, `ends_with`, `contains` | Matching |
| `index_of`, `replace`, `repeat`, `reverse` | Manipulation |
| `substring`, `char_at`, `split` | Indexing |
| `to_string_i64`, `to_string_f64`, `parse_int`, `parse_float` | Conversion |
| `str_format_i64`, `str_format_f64`, `str_format_str` | Formatting |

</details>

<details>
<summary><b>📦 Collections — 40+ functions</b></summary>

| Category | Functions |
|----------|-----------|
| **Arrays** | `push`, `pop`, `sort`, `reverse`, `slice`, `find`, `contains`, `join` |
| **Functional** | `array_range`, `array_sum`, `array_min`, `array_max`, `array_unique`, `array_take`, `array_drop`, `array_count`, `array_zip`, `array_enumerate`, `array_flatten` |
| **Maps** | `map_new`, `map_set`, `map_get`, `map_has`, `map_remove`, `map_len`, `map_keys` |
| **Sets** | `set_new`, `set_add`, `set_has`, `set_remove`, `set_len`, `set_union`, `set_intersect`, `set_diff` |
| **Tuples** | `tuple_new2`, `tuple_new3`, `tuple_new4`, `tuple_get`, `tuple_len` |

</details>

<details>
<summary><b>🔍 Regex — 8 functions</b></summary>

| Function | Description |
|----------|-------------|
| `regex_match` | Full match (anchored) |
| `regex_is_match` | Partial/contains match |
| `regex_find` | First match substring |
| `regex_replace` | Replace all occurrences |
| `regex_split_count`, `regex_split_get` | Split by pattern |
| `regex_find_all_count`, `regex_find_all_get` | Find all matches |

</details>

<details>
<summary><b>📁 File I/O — 6 functions</b></summary>

| Function | Description |
|----------|-------------|
| `file_read` | Read entire file to string |
| `file_write` | Write string to file |
| `file_append` | Append string to file |
| `file_exists` | Check if file exists |
| `file_delete` | Delete a file |
| `file_size` | Get file size in bytes |

</details>

<details>
<summary><b>🌐 Networking — 6 functions</b></summary>

| Function | Description |
|----------|-------------|
| `http_get` | HTTP GET → response body |
| `http_post` | HTTP POST → response body |
| `http_status` | HTTP GET → status code |
| `tcp_connect` | TCP connection (stub) |
| `tcp_send` | Send data over TCP (stub) |
| `tcp_close` | Close TCP connection (stub) |

</details>

<details>
<summary><b>⚠️ Error Handling — 4 functions</b></summary>

| Function | Description |
|----------|-------------|
| `error_set` | Set error code + message |
| `error_check` | Check if error is set (0 = no error) |
| `error_msg` | Get error message string |
| `error_clear` | Clear error state |

</details>

<details>
<summary><b>🔧 System — 10+ functions</b></summary>

| Function | Description |
|----------|-------------|
| `clock_ns`, `clock_ms`, `epoch_secs` | Timing |
| `sleep_ms` | Thread sleep |
| `pid` | Process ID |
| `env_get` | Environment variables |
| `eprint`, `eprintln` | Stderr output |
| `assert_eq_i64`, `assert_true` | Testing |
| `json_encode`, `json_decode` | JSON serialization |
| `spawn`, `task_result` | Async stubs |

</details>

<br>

## 🐍 Python FFI

Vitalis compiles to a shared library (`vitalis.dll` / `libvitalis.so`) with a full Python API:

```python
import vitalis

# Compile and run
result = vitalis.compile_and_run("fn main() -> i64 { 42 }")  # → 42

# Static analysis
errors = vitalis.check(source)       # type errors
tokens = vitalis.lex(source)         # [(kind, text), ...]
ast = vitalis.parse_ast(source)      # AST debug dump
ir = vitalis.dump_ir(source)         # IR dump

# Native hot-path operations (Rust, bypass JIT)
p95 = vitalis.hotpath_p95(latencies)
mean = vitalis.hotpath_mean(values)
score = vitalis.hotpath_code_quality_score(
    cyclomatic=5, cognitive=3, loc=100, funcs=10, issues=0, tests=50
)

# Evolution
vitalis.evo_register("fn_name", source)
vitalis.evo_evolve("fn_name", new_source)
vitalis.evo_set_fitness("fn_name", 0.95)
vitalis.evo_rollback("fn_name", gen)
```

<br>

## ⚡ Performance

### Compilation Speed

The Cranelift JIT backend compiles Vitalis code to native x86-64 machine code **at runtime** — no ahead-of-time compilation step required.

| Metric | Value |
|--------|-------|
| Lexer throughput | ~500K tokens/sec |
| Full pipeline (lex → native) | < 5ms for typical programs |
| Runtime overhead vs C | ~1.2x (Cranelift optimization level) |
| Hot-path Rust ops | 0x overhead (direct native calls) |

### Hot-Path Architecture

Performance-critical operations bypass the JIT entirely and call native Rust functions directly:

```mermaid
flowchart TB
    V["📄 Vitalis Code"] --> JIT
    V --> HP

    subgraph JIT ["⚡ CRANELIFT JIT"]
        direction TB
        J1["IR lowering"] --> J2["Register allocation"]
        J2 --> J3(["x86-64 native code"])
    end

    subgraph HP ["🚀 HOT-PATH BYPASS"]
        direction TB
        H1["Direct Rust FFI"] --> H2["SIMD vectorized ops"]
        H1 --> H3["Zero-copy native"]
        H2 --> H4(["< 1ns overhead"])
        H3 --> H4
    end

    style V fill:#2d1b69,stroke:#a855f7,stroke-width:3px,color:#f0e6ff,font-weight:bold
    style JIT fill:#0c1222,stroke:#60a5fa,stroke-width:2px,color:#dbeafe
    style HP fill:#1a0c0c,stroke:#f97316,stroke-width:3px,color:#fed7aa
    style H4 fill:#0f3d1e,stroke:#4ade80,stroke-width:2px,color:#dcfce7
    style J3 fill:#1e3a5f,stroke:#60a5fa,stroke-width:2px,color:#dbeafe
```

<br>

## 🧪 Test Suite

748 tests across every compiler stage:

```
$ cargo test
test result: ok. 748 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

| Category | Count | Coverage |
|----------|-------|----------|
| Lexer | 45+ | All 80 token variants |
| Parser | 80+ | Every AST node type |
| Type checker | 60+ | Inference, generics, errors |
| IR builder | 100+ | SSA, control flow, closures |
| Codegen (JIT) | 200+ | End-to-end compilation |
| Runtime stdlib | 120+ | All 200+ functions |
| Evolution | 20+ | Register, evolve, rollback |
| Domain modules | 80+ | Math, quantum, ML, crypto |
| **Nova ML Engine** | **40+** | **Tensors, training, tokenizer, inference, GPU** |

<br>

## 📁 Source Map

```
vitalis/
├── src/
│   ├── lexer.rs              # Logos tokenizer — 80 token variants
│   ├── parser.rs             # Recursive-descent + Pratt parser
│   ├── ast.rs                # 30+ Expr, 10 Stmt, 12 TopLevel variants
│   ├── types.rs              # Two-pass type checker with scope chains
│   ├── ir.rs                 # SSA-form IR with 26+ instruction types
│   ├── codegen.rs            # Cranelift JIT backend + 204 runtime functions
│   ├── stdlib.rs             # 200 built-in function registrations
│   ├── optimizer.rs          # IR optimization passes
│   ├── bridge.rs             # extern "C" FFI for Python/C interop
│   ├── main.rs               # CLI binary (vtc) with clap subcommands
│   ├── lib.rs                # Library root
│   │
│   ├── evolution.rs          # @evolvable function registry + tracking
│   ├── evolution_advanced.rs # Multi-strategy evolution
│   ├── meta_evolution.rs     # Meta-evolution — strategies evolving themselves
│   │
│   ├── hotpath.rs            # Native Rust hot-path operations (2,106 LOC)
│   ├── simd_ops.rs           # SIMD vectorized operations (F64x4)
│   ├── engine.rs             # Pipeline execution engine
│   ├── memory.rs             # Engram memory system
│   ├── scoring.rs            # Fitness scoring algorithms
│   │
│   ├── ml.rs                 # Neural networks, regression, k-means
│   ├── quantum.rs            # Quantum state simulation
│   ├── quantum_algorithms.rs # Grover, Shor, QFT
│   ├── quantum_math.rs       # Quantum math primitives
│   ├── graph.rs              # Graph algorithms (Dijkstra, BFS, MST)
│   ├── numerical.rs          # ODE solvers, integration, FFT
│   ├── signal_processing.rs  # DSP, filters, convolution
│   ├── bioinformatics.rs     # DNA sequencing, alignment
│   ├── neuromorphic.rs       # Spiking neural networks, STDP
│   ├── advanced_math.rs      # Special functions, distributions
│   ├── geometry.rs           # Computational geometry
│   ├── sorting.rs            # Parallel sorting algorithms
│   ├── automata.rs           # Finite state machines, regex engines
│   ├── combinatorial.rs      # Permutations, graph coloring
│   ├── probability.rs        # Distributions, sampling
│   ├── analytics.rs          # Statistical analysis
│   ├── compression.rs        # LZ77, Huffman coding
│   ├── chemistry_advanced.rs # Molecular dynamics
│   ├── string_algorithms.rs  # KMP, Rabin-Karp, suffix arrays
│   ├── crypto.rs             # SHA-256, AES, HMAC
│   ├── security.rs           # Sanitization, capability checks
│   ├── science.rs            # Physics simulations
│   │
│   │── ── Nova ML Engine (v20.0) ─────────────────
│   ├── tensor_engine.rs      # N-dim tensors, 33+ ops, autograd
│   ├── deep_learning.rs      # Transformer layers (RoPE, GQA, SwiGLU)
│   ├── gpu_compute.rs        # CUDA runtime, 11 PTX kernels, memory pool
│   ├── ml_training.rs        # AdamW, cosine scheduler, backward pass
│   ├── bpe_tokenizer.rs      # BPE train/encode/decode/save/load
│   └── model_inference.rs    # Model configs (5M-3B), sampling, generation
│
├── examples/                 # .sl example programs
├── vitalis.py                # Python FFI wrapper (ctypes)
├── Cargo.toml                # Rust manifest — Cranelift 0.116, regex, ureq
└── README.md                 # ← You are here
```

<br>

## 🔧 Building from Source

### Requirements

- **Rust** nightly or stable 1.85+ (Edition 2024)
- **Python 3.12+** (optional, for FFI wrapper)
- **Windows / Linux / macOS** (Cranelift supports all major platforms)

### Build Commands

```bash
# Debug build (fast compilation)
cargo build

# Release build (optimized binary)
cargo build --release

# Run tests
cargo test

# Build + run a file
cargo run -- run examples/fibonacci.sl

# Generate documentation
cargo doc --open
```

### Edition 2024 Notes

Vitalis uses Rust Edition 2024 which has stricter rules:

- `#[unsafe(no_mangle)]` instead of `#[no_mangle]`
- `gen` is a reserved keyword — use `generation` instead
- All `unsafe` blocks require explicit `unsafe {}` wrapping

<br>

## 🗺 Roadmap

```mermaid
timeline
    title Vitalis — From Zero to Self-Evolving Language

    v1 · Foundation
        : Lexer with Logos tokenizer
        : Recursive-descent Parser
        : AST with 30+ expression types
        : Cranelift 0.116 JIT backend

    v5 · Type System
        : Two-pass type checker
        : i64, f64, bool, str types
        : Heap-allocated arrays
        : SSA-form IR builder

    v10 · Standard Library
        : 100+ math functions
        : String operations + interning
        : Array methods + sorting
        : Random + hash + bit ops

    v15 · Language Power
        : Closures + Lambda expressions
        : File I/O + Maps + JSON
        : Error handling system
        : Evolution engine + @evolvable
        : 46 new stdlib functions

    v19 · General Purpose
        : Structs + Impl blocks
        : Try/Catch/Throw
        : Sets + Tuples + Regex
        : Module system with namespaces
        : HTTP networking + async stubs
        : Iterator protocol + comprehensions

    v20 · Nova ML Engine
        : Tensor engine with autograd (33+ ops)
        : Transformer architecture (RoPE, GQA, SwiGLU)
        : GPU compute backend (11 CUDA kernels)
        : ML training pipeline (AdamW, cosine LR)
        : BPE tokenizer (train/encode/decode)
        : Model inference (top-k/top-p sampling)
        : 42 FFI exports · 748 tests

    v21+ · The Future
        : Full async/await runtime
        : Trait system + user-defined generics
        : Package manager + registry
        : LSP server + IDE support
        : WebAssembly compilation target
```

<br>

## 📄 License

[MIT License](LICENSE) — use it, fork it, evolve it.

<br>

---

<div align="center">

**Built with 🧬 by [ModernOps888](https://github.com/ModernOps888)**

*A language that writes itself.*

</div>
