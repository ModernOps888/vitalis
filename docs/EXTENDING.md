# Extending Vitalis

This guide walks you through adding new features to the Vitalis compiler, standard library, and hot-path system.

---

## Table of Contents

1. [Adding a Stdlib Function](#adding-a-stdlib-function)
2. [Adding a Hot-Path Operation](#adding-a-hot-path-operation)
3. [Adding a New Token / Keyword](#adding-a-new-token--keyword)
4. [Adding an AST Node](#adding-an-ast-node)
5. [Adding a Type](#adding-a-type)
6. [Adding Optimizer Passes](#adding-optimizer-passes)
7. [Exposing to Python via FFI](#exposing-to-python-via-ffi)
8. [Testing Guidelines](#testing-guidelines)

---

## Adding a Stdlib Function

Stdlib functions are callable from `.sl` source code and linked at JIT-compile time.

### Step 1: Register the Function in `stdlib.rs`

Add a new `BuiltinFn` entry to the `builtins()` vector:

```rust
// stdlib.rs — inside builtins() vec![]
BuiltinFn {
    name: "my_func".into(),               // name in .sl source
    params: vec![("x", IrType::F64)],     // parameter name + type
    ret: IrType::F64,                      // return type
    runtime_name: "slang_my_func".into(),  // C symbol name
},
```

### Step 2: Implement the Runtime Function in `codegen.rs`

Find the runtime functions section and add:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn slang_my_func(x: f64) -> f64 {
    // Your implementation
    x.sin() * x.cos()
}
```

The function must be `extern "C"` with `#[unsafe(no_mangle)]` so the JIT linker can find it.

### Step 3: Register as JIT Symbol in `codegen.rs`

Find the `define_builtins()` or symbol registration section and ensure your runtime function is declared as a JIT symbol. The compiler auto-registers everything from `builtins()`, so this usually works automatically.

### Step 4: Test

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_my_func() {
        let src = "fn main() -> i64 { to_i64(my_func(1.0) * 1000.0) }";
        let result = crate::codegen::compile_and_run(src).unwrap();
        assert_eq!(result, 454); // sin(1)*cos(1)*1000 ≈ 454
    }
}
```

### Available `IrType` variants:
- `IrType::I64` — 64-bit integer
- `IrType::F64` — 64-bit float
- `IrType::Bool` — boolean
- `IrType::Ptr` — pointer (used for strings)
- `IrType::Void` — no return value

---

## Adding a Hot-Path Operation

Hot-path ops are Rust-native functions exposed via C FFI for Python and other callers. They bypass the JIT compiler entirely — they're pure Rust for maximum performance.

### Step 1: Implement in `hotpath.rs`

```rust
/// Compute the geometric mean of an array of f64 values.
///
/// # Safety
/// `data` must point to a valid array of `count` f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hotpath_geometric_mean(
    data: *const f64,
    count: usize,
) -> f64 {
    if data.is_null() || count == 0 {
        return 0.0;
    }
    let slice = unsafe { std::slice::from_raw_parts(data, count) };
    let product: f64 = slice.iter().product();
    product.powf(1.0 / count as f64)
}
```

**Conventions:**
- Function name starts with `hotpath_`
- Always use `#[unsafe(no_mangle)]` and `pub unsafe extern "C"`
- First check for null pointers and zero counts
- Use `std::slice::from_raw_parts()` to convert raw pointers to slices
- Return primitive types (`f64`, `i64`, `usize`, `bool` as `i32`)

### Step 2: Export in `bridge.rs` (if needed for Python)

If you want a higher-level FFI wrapper (e.g., returning allocated strings), add it to `bridge.rs`. For simple numeric functions, the hotpath function itself is directly callable from Python.

### Step 3: Add Python Wrapper in `python/vitalis.py`

```python
def hotpath_geometric_mean(values: list[float]) -> float:
    """Compute geometric mean using native Rust implementation."""
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_geometric_mean.restype = ctypes.c_double
    return _lib.hotpath_geometric_mean(arr, len(values))
```

### Step 4: Test

```rust
#[test]
fn test_hotpath_geometric_mean() {
    let data = [2.0, 8.0];
    let result = unsafe { hotpath_geometric_mean(data.as_ptr(), data.len()) };
    assert!((result - 4.0).abs() < 1e-10);
}
```

---

## Adding a New Token / Keyword

### Step 1: Add to the Token enum in `lexer.rs`

```rust
// lexer.rs — inside the Token enum
#[token("mytoken")]
MyToken,
```

For keywords that are identifiers:
```rust
#[token("mykeyword")]
KwMyKeyword,
```

### Step 2: Handle in the Parser

Add parsing logic in `parser.rs` to handle the new token:

```rust
Token::KwMyKeyword => {
    self.advance();
    // Parse the construct...
    Expr::MyConstruct { ... }
}
```

### Step 3: Add AST Node (if new construct)

See [Adding an AST Node](#adding-an-ast-node).

---

## Adding an AST Node

### Step 1: Add variant to `Expr` in `ast.rs`

```rust
// ast.rs — inside the Expr enum
MyConstruct {
    value: Box<Expr>,
    origin: Origin,
},
```

Every node should include an `origin: Origin` for error reporting.

### Step 2: Handle in Type Checker (`types.rs`)

```rust
// types.rs — inside the type_check_expr match
Expr::MyConstruct { value, origin } => {
    let val_type = self.check_expr(value)?;
    // Validate types...
    Ok(val_type)
}
```

### Step 3: Handle in IR Lowering (`ir.rs`)

```rust
// ir.rs — inside the lower_expr match
Expr::MyConstruct { value, .. } => {
    let val_ir = self.lower_expr(value)?;
    // Generate IR nodes...
    Ok(val_ir)
}
```

### Step 4: Handle in Codegen (`codegen.rs`)

```rust
// codegen.rs — inside the compile_expr match
IrNode::MyConstruct { value } => {
    let val = self.compile_node(value, builder)?;
    // Generate Cranelift instructions...
    Ok(val)
}
```

---

## Adding a Type

### Step 1: Add to `IrType` in `ir.rs`

```rust
// ir.rs
pub enum IrType {
    I64,
    F64,
    Bool,
    Ptr,
    Void,
    MyType,  // new!
}
```

### Step 2: Map to Cranelift type in `codegen.rs`

```rust
// codegen.rs — where IrType maps to cranelift types
IrType::MyType => types::I128, // or whatever Cranelift type
```

### Step 3: Add type checking rules in `types.rs`

Handle conversion, comparison, and operation rules for the new type.

---

## Adding Optimizer Passes

The optimizer in `optimizer.rs` runs passes over the IR before codegen.

### Adding a New Pass

```rust
// optimizer.rs
pub fn my_optimization_pass(nodes: &mut Vec<IrNode>) {
    // Walk the IR and transform nodes
    for node in nodes.iter_mut() {
        match node {
            IrNode::BinaryOp { op: BinOp::Add, left, right } => {
                // Example: constant folding
                if let (IrNode::Const(a), IrNode::Const(b)) = (left.as_ref(), right.as_ref()) {
                    *node = IrNode::Const(a + b);
                }
            }
            _ => {}
        }
    }
}
```

Register the pass in the optimization pipeline.

---

## Exposing to Python via FFI

### Bridge Function Pattern (`bridge.rs`)

For functions that return strings:

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_my_feature(input: *const c_char) -> *mut c_char {
    let input_str = if input.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("")
    };

    let result = do_something(input_str);

    CString::new(result).unwrap_or_default().into_raw()
}
```

**Critical:** Strings returned via `CString::into_raw()` must be freed by the caller using `slang_free_string()`.

### Python Side (`python/vitalis.py`)

```python
def my_feature(input: str) -> str:
    src = input.encode("utf-8")
    ptr = _lib.slang_my_feature(src)
    return _read_and_free(ptr)
```

**Critical:** Use `ctypes.c_void_p` as `restype` for string-returning functions, NOT `ctypes.c_char_p` (which auto-converts and loses the pointer needed for free).

---

## Testing Guidelines

### Inline Tests (Preferred for Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Test via compile_and_run for end-to-end
        let src = r#"fn main() -> i64 { 42 }"#;
        let result = crate::codegen::compile_and_run(src).unwrap();
        assert_eq!(result, 42);
    }
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests in a specific module
cargo test hotpath

# Run a specific test
cargo test test_feature_name

# Run with output visible
cargo test -- --nocapture
```

### Test Coverage Expectations

- Every new stdlib function: at least 1 test
- Every new hotpath op: at least 1 test with edge cases (null, empty, normal)
- Every new AST node: parser test + type-check test + codegen test
- Every new FFI function: test that it's callable and returns expected results

---

## Architecture Quick Reference

```
Source (.sl) → Lexer (lexer.rs) → Tokens
    → Parser (parser.rs) → AST (ast.rs)
    → TypeChecker (types.rs) → Validated AST
    → IR Lowering (ir.rs) → IR Nodes
    → Optimizer (optimizer.rs) → Optimized IR
    → Codegen (codegen.rs) → Cranelift JIT → Native x86-64
```

| File | What to modify |
|------|---------------|
| `lexer.rs` | New tokens / keywords |
| `parser.rs` | New syntax / expressions |
| `ast.rs` | New AST node types |
| `types.rs` | Type rules / checking |
| `ir.rs` | IR representation |
| `codegen.rs` | Machine code generation + runtime functions |
| `stdlib.rs` | Register new built-in functions |
| `hotpath.rs` | High-performance native operations |
| `bridge.rs` | C FFI exports |
| `optimizer.rs` | Optimization passes |
| `evolution.rs` | Evolution system |
| `engine.rs` | Evolution cycle runner |
| `memory.rs` | Engram memory store |
