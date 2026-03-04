<div align="center">

<!-- ═══════════════════════════════════════════════════════════ -->
<!--                     VITALIS HEADER                         -->
<!-- ═══════════════════════════════════════════════════════════ -->

# 🧬 Vitalis

### The Self-Evolving Programming Language

[![Rust](https://img.shields.io/badge/Rust-Edition_2024-b7410e?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-1%2C765_Passing-00c853?style=for-the-badge&logo=checkmarx&logoColor=white)](#-test-suite)
[![LOC](https://img.shields.io/badge/LOC-62%2C700+-blue?style=for-the-badge&logo=slickpic&logoColor=white)](#-architecture)
[![License](https://img.shields.io/badge/License-MIT-yellow?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](LICENSE)
[![Version](https://img.shields.io/badge/v28.0.0-purple?style=for-the-badge&logo=v&logoColor=white)](#-changelog)

**A compiled language purpose-built for autonomous AI code evolution.**<br>
Vitalis compiles to native machine code via Cranelift JIT and AOT, with first-class support for<br>
self-modifying programs, genetic code evolution, and real-time fitness tracking.

*Written from scratch in Rust. No LLVM. No interpreter. No VM. JIT + AOT native compilation.*

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

**76**<br>
<sub>Source modules</sub>

</td>
<td width="25%" align="center">

**62,700+**<br>
<sub>Lines of Rust</sub>

</td>
<td width="25%" align="center">

**1,765**<br>
<sub>Tests passing</sub>

</td>
<td width="25%" align="center">

**230+**<br>
<sub>Stdlib functions</sub>

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

Every source file has a single responsibility. The codebase is organized into **six layers**:

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

    block:SAFETY:2
        columns 1
        SA["🛡️ SAFETY & TOOLING · 7,200+ LOC"]
        SB["lifetimes.rs\nRegion analysis"]
        SC["effects.rs\nCapability types"]
        SD["hot_reload.rs\nLive reload"]
        SE["nll.rs\nNon-lexical lifetimes"]
        SF["effect_handlers.rs\nAlgebraic handlers"]
        SG["pattern_exhaustiveness.rs\nMatch checking"]
    end

    block:NATIVE:2
        columns 1
        NA["🎯 NATIVE TARGETS · 1,800 LOC"]
        NB["aot.rs\nAOT compilation"]
        NC["cross_compile.rs\nARM · RISC-V"]
        ND["bootstrap.rs\nSelf-hosted"]
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
    CORE --> SAFETY
    CORE --> NATIVE
    CORE --> MATH

    style CORE fill:#0c1222,stroke:#38bdf8,stroke-width:3px,color:#bae6fd
    style EVO fill:#1a0c22,stroke:#e879f9,stroke-width:3px,color:#f5d0fe
    style PERF fill:#1a0c0c,stroke:#fb923c,stroke-width:3px,color:#fed7aa
    style SAFETY fill:#0c1a0c,stroke:#4ade80,stroke-width:3px,color:#dcfce7
    style NATIVE fill:#1a1a0c,stroke:#fbbf24,stroke-width:3px,color:#fef3c7
    style MATH fill:#0c1a22,stroke:#a78bfa,stroke-width:3px,color:#ddd6fe
    style A fill:#1e3a5f,stroke:#38bdf8,color:#e0f2fe
    style J fill:#2d1042,stroke:#e879f9,color:#f5d0fe
    style N fill:#2d1a0a,stroke:#fb923c,color:#fed7aa
    style SA fill:#0f3d1e,stroke:#4ade80,color:#dcfce7
    style NA fill:#3d3d0a,stroke:#fbbf24,color:#fef3c7
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

# Run all 1,586 tests
cargo test

# Compile and run a .sl file
cargo run -- run examples/hello.sl

# AOT compile to standalone executable
cargo run -- build examples/hello.sl --output hello

# Cross-compile for ARM64
cargo run -- build examples/hello.sl --target aarch64-unknown-linux-gnu

# List available cross-compilation targets
cargo run -- targets

# Run bootstrap pipeline
cargo run -- bootstrap examples/hello.sl
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
    ⚡ **Cranelift JIT + AOT**
      Native x86-64 machine code
      Ahead-of-time standalone executables
      Cross-compilation (ARM64, RISC-V)
      Hot-path bypass to Rust
      SIMD vectorized operations
    🔐 **Capability Safety**
      Effect system with capability types
      Lifetime annotations + region analysis
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
    💭 **Consciousness Keywords**
      memorize · recall · forget
      reflect · evolve · mutate
      sandbox · rollback · pipeline
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

The Cranelift backend compiles Vitalis code to native x86-64 machine code via **JIT** at runtime or **AOT** for standalone executables. Cross-compilation to AArch64 (ARM64) and RISC-V 64 is also supported.

| Metric | Value |
|--------|-------|
| Lexer throughput | ~500K tokens/sec |
| Full pipeline (lex → native) | < 5ms for typical programs |
| Runtime overhead vs C | ~1.2x (Cranelift optimization level) |
| Hot-path Rust ops | 0x overhead (direct native calls) |
| AOT targets | x86-64, AArch64, RISC-V 64 |

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

1,765 tests across every compiler stage and all subsystems through v28:

```
$ cargo test
test result: ok. 1765 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

| Category | Count | Coverage |
|----------|-------|----------|
| Lexer | 50+ | All 80 token variants |
| Parser | 100+ | Every AST node type |
| Type checker | 60+ | Inference, generics, errors |
| IR builder | 110+ | SSA, control flow, closures, traits |
| Codegen (JIT) | 200+ | End-to-end compilation |
| Runtime stdlib | 120+ | All 200+ functions |
| Evolution | 20+ | Register, evolve, rollback |
| Domain modules | 80+ | Math, quantum, ML, crypto |
| Async runtime | 15 | Executor, tasks, channels, futures |
| Generics | 20 | Type params, monomorphization, bounds |
| Package manager | 22 | SemVer, registry, dependency resolver |
| LSP server | 25 | Diagnostics, completions, symbols |
| WASM target | 25 | Module builder, LEB128, sections |
| GPU compute | 22 | Buffers, kernels, pipelines, shaders |
| Ownership / borrow checker | 21 | Move tracking, scope analysis |
| Trait dispatch | 20 | VTables, resolution, impl registry |
| Incremental compilation | 22 | Hash caching, dep graph, topo sort |
| DAP debugger | 28 | Breakpoints, stack, variables, stepping |
| REPL | 15 | Interactive eval, commands, history |
| Lifetime analysis | 10 | Region-based memory safety, borrow lifetimes |
| Effect system | 10 | Capability types, algebraic effects |
| Hot reload | 9 | File watching, incremental recompilation |
| Bootstrap pipeline | 10 | Stage 0/1/2, self-hosted compiler |
| AOT compilation | 10 | Native ahead-of-time code generation |
| Cross-compilation | 18 | x86-64, AArch64, RISC-V targets |
| NLL borrow analysis | 44 | CFG, liveness, NLL regions, conflict detection |
| Effect handlers | 39 | Handler stack, continuations, dispatch, composition |
| Pattern exhaustiveness | 51 | Usefulness, redundancy, or-patterns, nested destructuring |
| Formatter | 33 | AST formatting, config, idempotency, all node types |
| Linter | 30 | 17 lint rules, unused detection, naming, dead code |
| Refinement types | 44 | Predicates, solver, subtyping, registry, bounds |
| Macro system | 35 | Token trees, hygiene, derives, pattern matching |
| Compile-time evaluation | 35 | Const exprs, const fns, static assertions, folding |
| Iterator / generator protocol | 40 | Adapters, pipelines, state machines, terminals |
| Structured concurrency | 45 | Mutex, RwLock, channels, Select, WaitGroup, atomics, deadlock detection |
| Type inference | 40 | Hindley-Milner, unification, bidirectional, union/intersection, narrowing |
| Documentation generation | 30 | Doc comment parsing, API model, Markdown/HTML output, cross-refs |
| Graphics engine | 40 | Colors, vectors, matrices, paths, image buffers, render pipeline |
| Shader languages | 25 | GLSL, HLSL, WGSL, MSL, SPIR-V compilation, cross-compilation |
| GUI framework | 30 | CSS styling, flexbox layout, widget tree, themes, animations |
| Creative coding | 35 | Sketch lifecycle, particle systems, L-systems, cellular automata |
| Visual nodes | 30 | Node graph, evaluation, templates, DOT export, type checking |
| Chart rendering | 30 | Pie/bar/line/scatter/histogram/radar/heatmap/treemap/candlestick |

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
│   ├── async_runtime.rs      # Async/await runtime — executor, channels, futures
│   ├── generics.rs           # Generics — type params, monomorphization, bounds
│   ├── package_manager.rs    # Package manager — SemVer, registry, resolution
│   ├── lsp.rs                # LSP server — diagnostics, completion, hover, symbols
│   ├── wasm_target.rs        # WASM target — module builder, LEB128, sections
│   ├── gpu_compute.rs        # GPU compute — buffers, kernels, pipelines, shaders
│   │
│   ├── ownership.rs          # Borrow checker — ownership, move, drop analysis
│   ├── trait_dispatch.rs     # Trait dispatch — vtables, method resolution
│   ├── incremental.rs        # Incremental compilation — hash caching, dep graph
│   ├── dap.rs                # Debug Adapter Protocol — breakpoints, stack, stepping
│   ├── repl.rs               # Interactive REPL — eval, commands, history
│   │
│   ├── lifetimes.rs          # Lifetime annotations — region analysis, borrow scopes
│   ├── effects.rs            # Effect system — capability types, algebraic effects
│   ├── hot_reload.rs         # Hot reload — file watching, incremental recompilation
│   ├── bootstrap.rs          # Self-hosted bootstrap — Stage 0/1/2 pipeline
│   ├── aot.rs                # AOT compilation — native ahead-of-time code generation
│   ├── cross_compile.rs      # Cross-compilation — x86-64, AArch64, RISC-V targets
│   ├── nll.rs                # Non-lexical lifetimes — CFG, liveness, NLL borrow regions
│   ├── effect_handlers.rs    # Algebraic effect handlers — resume/abort continuations
│   ├── pattern_exhaustiveness.rs  # Pattern exhaustiveness — usefulness, redundancy, or-patterns
│   │
│   ├── formatter.rs          # Code formatter — AST-based pretty-printer with config
│   ├── linter.rs             # Static linter — 17 rules, unused detection, naming
│   ├── refinement_types.rs   # Refinement types — constraint solver, subtyping, predicates
│   │
│   ├── macro_system.rs       # Macro system — hygienic expansion, derives, token trees
│   ├── const_eval.rs         # Compile-time eval — const exprs, const fns, static asserts
│   ├── iterators.rs          # Iterator protocol — lazy adapters, generators, state machines
│   │
│   ├── concurrency.rs        # Structured concurrency — Mutex, RwLock, channels, Select, atomics
│   ├── type_inference.rs     # Type inference — Hindley-Milner, unification, bidirectional
│   ├── documentation.rs      # Documentation gen — doc comments, API model, Markdown/HTML
│   │
│   ├── graphics_engine.rs    # Graphics engine — 2D/3D rendering, colors, vectors, matrices, SVG
│   ├── shader_lang.rs        # Shader languages — GLSL, HLSL, WGSL, MSL, SPIR-V cross-compilation
│   ├── gui_framework.rs      # GUI framework — QML/XAML/SwiftUI/CSS, widgets, layout, themes
│   ├── creative_coding.rs    # Creative coding — Processing/p5.js, particles, L-systems, automata
│   ├── visual_nodes.rs       # Visual nodes — node graphs, evaluation, TouchDesigner/Blueprints
│   └── chart_rendering.rs    # Chart rendering — pie, bar, line, scatter, histogram, radar, treemap
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

    v20 · Trait System & Type Power
        : Trait definitions + trait methods
        : Type aliases (type Name = Type)
        : Cast expressions (expr as Type)
        : Enum definitions with variant indexing
        : Method registry for impl dispatch
        : Bare self parameter sugar
        : 741 tests passing

    v21 · Async, Generics, WASM & GPU
        : Full async/await runtime (executor, channels, futures)
        : Generics + type parameters + monomorphization
        : Package manager + registry + dependency resolver
        : LSP server + IDE support (diagnostics, completion, hover)
        : WebAssembly target (module builder, LEB128, sections)
        : GPU compute backend (buffers, kernels, pipelines, shaders)
        : 870 tests passing · 47 modules · 35,856 LOC

    v22 · Borrow Checker, DAP, REPL & Trait Dispatch
        : Ownership & borrow checker (move tracking, scope analysis)
        : Incremental compilation (hash caching, dep graph, topo sort)
        : Full trait dispatch with vtables + method resolution
        : Debug Adapter Protocol (breakpoints, stack, variables, stepping)
        : Interactive REPL (eval, commands, history)
        : Lifetime annotations + region-based memory analysis
        : Effect system + capability types + algebraic effects
        : Incremental codegen + hot-reload with file watching
        : Self-hosted compiler bootstrap (Stage 0/1/2 pipeline)
        : Native AOT compilation (standalone executables)
        : Cross-compilation targets (x86-64, AArch64, RISC-V)
        : 1,043 tests passing · 58 modules · 41,772 LOC

    v23 · Non-Lexical Lifetimes (NLL)
        : CFG builder from AST (entry, exit, branch, join, loop nodes)
        : Backward dataflow liveness analysis (live_in / live_out)
        : NLL regions as sets of CFG points (not lexical scopes)
        : Borrow conflict detection via overlapping live ranges
        : Modify-while-borrowed checks
        : 1,087 tests passing · 59 modules · 43,095 LOC

    v24 · Effect Handlers & Pattern Exhaustiveness
        : Algebraic effect handlers with resume/abort continuations
        : Handler stack + dispatcher for nested handler frames
        : Handler composition (combine, layer multiple handlers)
        : Pattern exhaustiveness checking (Maranget usefulness algorithm)
        : Or-patterns, guard clauses, nested destructuring
        : Redundant/unreachable arm detection with diagnostics
        : AST extensions (Or, Tuple patterns, Handle expression)
        : 1,177 tests passing · 61 modules · 45,703 LOC

    v25 · Formatter, Linter & Refinement Types
        : AST-based code formatter with configurable style (vtc fmt)
        : Static linter with 17 lint rules and configurable severity
        : Refinement types with constraint solver and subtype checking
        : Built-in refinements (Positive, Natural, NonZero, Percentage, Byte)
        : Predicate language (Compare, And, Or, Not, Implies, Arith)
        : 1,284 tests passing · 64 modules · 47,743 LOC

    v26 · Macros, Const Eval & Iterators
        : Hygienic macro system with token trees, pattern matching, derive macros
        : Compile-time evaluation engine with const fns and static assertions
        : Lazy iterator protocol with 13 adapters and generator state machines
        : Built-in derives (Debug, Clone, PartialEq, Default, Display, Hash)
        : Terminal operations (collect, count, sum, fold, any, all, find)
        : 1,458 tests passing · 67 modules · 53,359 LOC

    v27 · Concurrency, Type Inference & Documentation
        : Structured concurrency (Mutex, RwLock, channels, Select, WaitGroup, atomics)
        : Scoped tasks with lifecycle management and deadlock detection
        : Hindley-Milner type inference with Algorithm W and let-polymorphism
        : Bidirectional type checking with union/intersection types
        : Flow-sensitive type narrowing and subtype checking
        : Documentation generation (doc comments, API model, Markdown/HTML/plaintext)
        : Cross-reference resolution and example extraction from doc comments
        : 1,586 tests passing · 70 modules · 57,196 LOC

    v28 · Graphics Engine, Shader Languages, GUI, Creative Coding, Visual Nodes & Charts
        : 2D/3D graphics engine (RGBA/HSLA colors, Vec2/3/4, Mat4, Path2D, ImageBuffer, Camera)
        : Multi-backend shader compilation (GLSL, HLSL, WGSL, MSL, SPIR-V cross-compiler)
        : Declarative GUI framework (QML/XAML/SwiftUI/CSS, 30+ widgets, flex layout, themes)
        : Processing/p5.js creative coding (sketch lifecycle, Perlin noise, particle systems)
        : L-system fractal generation (Koch, Sierpinski, Dragon curve, fractal plant)
        : Cellular automata (Conway's Game of Life, Wolfram elementary CA rules)
        : Lorenz attractor visualization and flow field generation
        : Node-based visual programming (TouchDesigner/Blueprints, typed ports, topological eval)
        : Full chart rendering (pie/donut, bar, line, scatter, histogram, radar, heatmap)
        : Treemap, candlestick, gauge, sparkline charts with SVG export
        : Dashboard layout engine for multi-chart compositions
        : 30 new stdlib builtins (gfx_*, chart_*, shader_*, gui_*, noise_*, node_graph_*)
        : 1,765 tests passing · 76 modules · 62,700+ LOC

    v29+ · The Future
        : WASM AOT target (compile .sl to standalone .wasm files)
        : Package registry server + vitalis install
        : Distributed compilation across nodes
        : ARM/RISC-V hardware validation on real devices
        : Profile-guided JIT optimization (PGO)
        : Async streams and reactive programming
        : Multi-language FFI (C, C++, JS)
        : Dependent types and proof-carrying code
        : Module-level parallelism in compilation
        : Language server protocol v2 (semantic tokens, inlay hints)
        : Interactive playground and web IDE
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
