# Vitalis Python Wrapper

Python ctypes bindings for the Vitalis compiler and hot-path operations.

## Setup

1. Build the Vitalis shared library:

```bash
cd ..
cargo build  # produces target/debug/vitalis.dll (Windows) or libvitalis.so (Linux) or libvitalis.dylib (macOS)
```

2. Copy `vitalis.py` to your project, or add this directory to your Python path.

3. The wrapper auto-discovers the library in these locations:
   - `./vitalis.dll` / `./libvitalis.so` / `./libvitalis.dylib`
   - `./target/debug/vitalis.dll`
   - `./target/release/vitalis.dll`

## Quick Start

```python
import vitalis

# Compile and run Vitalis source code
result = vitalis.compile_and_run("fn main() -> i64 { 42 }")
print(result)  # 42

# Type-check source code
errors = vitalis.check("fn main() -> i64 { true }")  # returns error list

# Lex tokens
tokens = vitalis.lex("fn main() -> i64 { 42 }")

# Parse AST
ast = vitalis.parse_ast("fn main() -> i64 { 42 }")

# Dump IR
ir = vitalis.dump_ir("fn main() -> i64 { 42 }")
```

## Compiler API

| Function | Description | Returns |
|----------|-------------|---------|
| `compile_and_run(source)` | Compile and execute `.sl` source | `int` (i64 result) |
| `run_file(path)` | Compile and run a `.sl` file | `int` |
| `check(source)` | Type-check without executing | `list[str]` errors |
| `lex(source)` | Tokenize source | `list[tuple]` |
| `parse_ast(source)` | Parse to AST string | `str` |
| `dump_ir(source)` | Generate IR dump | `str` |

## Evolution API

```python
# Register an evolvable function
vitalis.evo_register("score", """
@evolvable
fn score(x: i64) -> i64 { x * 2 }
""")

# Evolve to a new variant (returns new generation number)
gen = vitalis.evo_evolve("score", """
@evolvable
fn score(x: i64) -> i64 { x * 3 }
""")

# Set fitness score (0.0 to 1.0)
vitalis.evo_set_fitness("score", 0.95)

# Rollback to a previous generation
vitalis.evo_rollback("score", 1)

# Load @evolvable functions from source
vitalis.evo_load(source)
```

## Hot-Path Operations

Native Rust implementations callable from Python, benchmarked at **7.6x avg faster** than pure Python equivalents.

### Statistics
```python
vitalis.hotpath_p95(values)                    # 95th percentile
vitalis.hotpath_mean(values)                   # arithmetic mean
vitalis.hotpath_median(values)                 # median
vitalis.hotpath_stddev(values)                 # standard deviation
vitalis.hotpath_percentile(values, pct)        # arbitrary percentile
vitalis.hotpath_variance(values)               # variance
vitalis.hotpath_entropy(values)                # Shannon entropy
```

### ML Activations
```python
vitalis.hotpath_softmax(values)                # softmax (returns list)
vitalis.hotpath_batch_norm(values, mean, var)  # batch normalization
vitalis.hotpath_layer_norm(values)             # layer normalization
```

### Loss Functions
```python
vitalis.hotpath_cross_entropy_loss(predictions, targets)
vitalis.hotpath_mse_loss(predictions, targets)
vitalis.hotpath_huber_loss(predictions, targets, delta)
vitalis.hotpath_kl_divergence(p, q)
```

### Vector Operations
```python
vitalis.hotpath_cosine_similarity(a, b)        # cosine similarity
vitalis.hotpath_cosine_distance(a, b)          # 1 - cosine similarity
vitalis.hotpath_l2_normalize(values)           # L2 normalization
vitalis.hotpath_dot_product(a, b)              # dot product
vitalis.hotpath_hamming_distance(a, b)         # hamming distance
```

### Rate Limiting
```python
vitalis.hotpath_sliding_window_count(timestamps, now, window)
vitalis.hotpath_token_bucket(tokens, max_tokens, refill_rate, elapsed, cost)
```

### Optimization
```python
vitalis.hotpath_bayesian_ucb(mean, variance, n_trials, total)
vitalis.hotpath_simulated_annealing_accept(current, candidate, temp)
vitalis.hotpath_boltzmann_selection(scores, temperature)
```

### Analysis
```python
vitalis.hotpath_code_quality_score(cyclomatic, cognitive, loc, funcs, issues, tests)
vitalis.hotpath_cognitive_complexity(depths)
vitalis.hotpath_weighted_score(metrics, weights)
vitalis.hotpath_tally_votes(votes)
vitalis.hotpath_tally_string_votes(votes)
```

## Important Notes

### String Memory Management

Functions returning strings use `ctypes.c_void_p` (not `c_char_p`) to prevent Python from auto-converting the pointer. The wrapper handles freeing via `slang_free_string()` internally.

### Thread Safety

The Vitalis compiler uses module-level state. Avoid calling `compile_and_run` from multiple threads simultaneously. Hot-path operations are stateless and thread-safe.

### Platform Support

| Platform | Library Name |
|----------|-------------|
| Windows | `vitalis.dll` |
| Linux | `libvitalis.so` |
| macOS | `libvitalis.dylib` |
