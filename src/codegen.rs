//! Vitalis Codegen — Cranelift JIT backend.
//!
//! Translates the SSA IR into native machine code via Cranelift.
//! Supports both JIT execution and AOT compilation.
//!
//! Phase 0: integer arithmetic, control flow, function calls, print.

use crate::ir;
use crate::ir::{IrModule, IrFunction, IrType, Inst, IrBinOp, IrUnOp, IrCmp};

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

// ─── String Arena ───────────────────────────────────────────────────────
// Keeps string literals alive for the lifetime of the process.
// Box<[u8]> ensures the heap address never moves.
static VITALIS_STRING_ARENA: Mutex<Vec<Box<[u8]>>> = Mutex::new(Vec::new());

fn intern_cstr(s: &str) -> *const u8 {
    let bytes: Box<[u8]> = s.bytes().chain(std::iter::once(0u8)).collect();
    let ptr = bytes.as_ptr();
    VITALIS_STRING_ARENA.lock().unwrap().push(bytes);
    ptr
}

// ─── Codegen Errors ─────────────────────────────────────────────────────
#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "codegen error: {}", self.message)
    }
}

impl std::error::Error for CodegenError {}

type CodegenResult<T> = Result<T, CodegenError>;

// ─── Runtime Support ────────────────────────────────────────────────────
/// Print an i64 to stdout. Called by generated code.
extern "C" fn slang_print_i64(val: i64) {
    println!("{}", val);
}

/// Print a string (pointer + length) to stdout.
extern "C" fn slang_print_str(ptr: *const u8, len: i64) {
    let s = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len as usize);
        std::str::from_utf8_unchecked(slice)
    };
    print!("{}", s);
}

/// Print a string with newline.
extern "C" fn slang_println_str(ptr: *const u8, len: i64) {
    let s = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len as usize);
        std::str::from_utf8_unchecked(slice)
    };
    println!("{}", s);
}

// ─── Typed print runtime ────────────────────────────────────────────────
extern "C" fn slang_println_i64(val: i64) { println!("{}", val); }
extern "C" fn slang_print_f64(val: f64)   { print!("{}", val); }
extern "C" fn slang_println_f64(val: f64) { println!("{}", val); }
extern "C" fn slang_print_bool(val: i8)   { print!("{}", if val != 0 { "true" } else { "false" }); }
extern "C" fn slang_println_bool(val: i8) { println!("{}", if val != 0 { "true" } else { "false" }); }
/// Print a null-terminated C string (used for StrConst literals).
extern "C" fn slang_print_cstr(ptr: *const i8) {
    if ptr.is_null() { return; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
    print!("{}", s.to_string_lossy());
}
extern "C" fn slang_println_cstr(ptr: *const i8) {
    if ptr.is_null() { return; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
    println!("{}", s.to_string_lossy());
}

// ─── Math runtime ───────────────────────────────────────────────────────
extern "C" fn slang_sqrt_f64(x: f64) -> f64   { x.sqrt() }
extern "C" fn slang_abs_i64(x: i64) -> i64    { x.abs() }
extern "C" fn slang_abs_f64(x: f64) -> f64    { x.abs() }
extern "C" fn slang_min_i64(a: i64, b: i64) -> i64 { a.min(b) }
extern "C" fn slang_max_i64(a: i64, b: i64) -> i64 { a.max(b) }
extern "C" fn slang_min_f64(a: f64, b: f64) -> f64 { a.min(b) }
extern "C" fn slang_max_f64(a: f64, b: f64) -> f64 { a.max(b) }
extern "C" fn slang_pow_f64(base: f64, exp: f64) -> f64 { base.powf(exp) }
extern "C" fn slang_floor_f64(x: f64) -> f64  { x.floor() }
extern "C" fn slang_ceil_f64(x: f64) -> f64   { x.ceil() }
extern "C" fn slang_round_f64(x: f64) -> f64  { x.round() }
extern "C" fn slang_ln_f64(x: f64) -> f64     { x.ln() }
extern "C" fn slang_log2_f64(x: f64) -> f64   { x.log2() }
extern "C" fn slang_log10_f64(x: f64) -> f64  { x.log10() }
extern "C" fn slang_sin_f64(x: f64) -> f64    { x.sin() }
extern "C" fn slang_cos_f64(x: f64) -> f64    { x.cos() }
extern "C" fn slang_exp_f64(x: f64) -> f64    { x.exp() }
extern "C" fn slang_atan2_f64(y: f64, x: f64) -> f64       { y.atan2(x) }
extern "C" fn slang_hypot_f64(a: f64, b: f64) -> f64        { a.hypot(b) }
extern "C" fn slang_clamp_f64(x: f64, lo: f64, hi: f64) -> f64 { x.clamp(lo, hi) }
extern "C" fn slang_clamp_i64(x: i64, lo: i64, hi: i64) -> i64 { x.clamp(lo, hi) }

// ─── Random runtime (Xorshift64 — no external dependencies) ──────────────
static VITALIS_RNG_STATE: AtomicU64 = AtomicU64::new(0x853c49e6748fea9b_u64);
fn xorshift64() -> u64 {
    let mut x = VITALIS_RNG_STATE.load(Ordering::Relaxed);
    if x == 0 { x = 0x853c49e6748fea9b; }
    x ^= x << 13; x ^= x >> 7; x ^= x << 17;
    VITALIS_RNG_STATE.store(x, Ordering::Relaxed);
    x
}
extern "C" fn slang_rand_f64() -> f64 {
    (xorshift64() >> 11) as f64 * (1.0_f64 / (1u64 << 53) as f64)
}
extern "C" fn slang_rand_i64() -> i64 {
    xorshift64() as i64
}

// ─── Type conversion runtime ─────────────────────────────────────────────
 extern "C" fn slang_i64_to_f64(x: i64) -> f64 { x as f64 }
extern "C" fn slang_f64_to_i64(x: f64) -> i64 { x as i64 }

// ─── Phase 4: Array heap runtime ───────────────────────────────────────────────
// Layout: [i64 length][elem0][elem1]...[elemN]
// The returned pointer points to elem0 (data region); header is at ptr - 8.
// All array memory is leaked intentionally: Vitalis uses arena semantics and
// GC is deferred to Phase 4C (tracing collector over the string/array arena).
extern "C" fn slang_array_alloc(count: i64, stride: i64) -> *mut u8 {
    if count <= 0 || stride <= 0 {
        // Return a zeroed sentinel: header = 0, no data.
        let layout = std::alloc::Layout::from_size_align(8, 8).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        return unsafe { ptr.add(8) };
    }
    let header = 8usize; // i64 length
    let data_size = (count as usize).saturating_mul(stride as usize);
    let total = header + data_size;
    let layout = std::alloc::Layout::from_size_align(total, 8)
        .unwrap_or_else(|_| std::alloc::Layout::from_size_align(16, 8).unwrap());
    let raw = unsafe { std::alloc::alloc_zeroed(layout) };
    if raw.is_null() { return std::ptr::null_mut(); }
    unsafe { *(raw as *mut i64) = count; }
    unsafe { raw.add(header) }
}

/// Read the length header of an array.
extern "C" fn slang_array_len(data_ptr: *const u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    unsafe { *(data_ptr.sub(8) as *const i64) }
}

/// Bounds-checked i64 element load.
extern "C" fn slang_array_get_i64(data_ptr: *const u8, index: i64) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) };
    if index < 0 || index >= len { return 0; }
    unsafe { *(data_ptr as *const i64).add(index as usize) }
}

/// Bounds-checked i64 element store.
extern "C" fn slang_array_set_i64(data_ptr: *mut u8, index: i64, value: i64) {
    if data_ptr.is_null() { return; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) };
    if index < 0 || index >= len { return; }
    unsafe { *(data_ptr as *mut i64).add(index as usize) = value; }
}

/// Bounds-checked f64 element load.
extern "C" fn slang_array_get_f64(data_ptr: *const u8, index: i64) -> f64 {
    if data_ptr.is_null() { return 0.0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) };
    if index < 0 || index >= len { return 0.0; }
    unsafe { *(data_ptr as *const f64).add(index as usize) }
}

/// Bounds-checked f64 element store.
extern "C" fn slang_array_set_f64(data_ptr: *mut u8, index: i64, value: f64) {
    if data_ptr.is_null() { return; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) };
    if index < 0 || index >= len { return; }
    unsafe { *(data_ptr as *mut f64).add(index as usize) = value; }
}

// ─── String operations runtime ───────────────────────────────────────────
extern "C" fn slang_str_len(ptr: *const i8) -> i64 {
    if ptr.is_null() { return 0; }
    unsafe { std::ffi::CStr::from_ptr(ptr).to_bytes().len() as i64 }
}
extern "C" fn slang_str_eq(a: *const i8, b: *const i8) -> i8 {
    if a.is_null() || b.is_null() { return 0; }
    let sa = unsafe { std::ffi::CStr::from_ptr(a) };
    let sb = unsafe { std::ffi::CStr::from_ptr(b) };
    if sa == sb { 1 } else { 0 }
}
/// Concatenate two null-terminated strings; result is interned into the arena.
extern "C" fn slang_str_cat(a: *const i8, b: *const i8) -> *const i8 {
    let sa = if a.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(a).to_string_lossy().into_owned() } };
    let sb = if b.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(b).to_string_lossy().into_owned() } };
    let combined = sa + &sb;
    intern_cstr(&combined) as *const i8
}

// ─── Phase 5: New stdlib runtime functions ──────────────────────────────

// Time
extern "C" fn slang_clock_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64
}
extern "C" fn slang_clock_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// Assertions
extern "C" fn slang_assert_eq_i64(a: i64, b: i64) {
    if a != b {
        eprintln!("[VITALIS ASSERT FAILED] assert_eq: {} != {}", a, b);
    }
}
extern "C" fn slang_assert_true(cond: i8) {
    if cond == 0 {
        eprintln!("[VITALIS ASSERT FAILED] assert_true: got false");
    }
}

// Bitwise operations
extern "C" fn slang_popcount(x: i64) -> i64 { x.count_ones() as i64 }
extern "C" fn slang_leading_zeros(x: i64) -> i64 { x.leading_zeros() as i64 }
extern "C" fn slang_trailing_zeros(x: i64) -> i64 { x.trailing_zeros() as i64 }

// Extended math
extern "C" fn slang_sign_i64(x: i64) -> i64 { x.signum() }
extern "C" fn slang_gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs(); b = b.abs();
    while b != 0 { let t = b; b = a % b; a = t; }
    a
}
extern "C" fn slang_lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 { return 0; }
    (a.abs() / slang_gcd(a, b)) * b.abs()
}
extern "C" fn slang_factorial(n: i64) -> i64 {
    if n < 0 { return 0; }
    let mut result: i64 = 1;
    for i in 2..=n { result = result.saturating_mul(i); }
    result
}
extern "C" fn slang_fibonacci(n: i64) -> i64 {
    if n <= 0 { return 0; }
    if n == 1 { return 1; }
    let (mut a, mut b) = (0i64, 1i64);
    for _ in 2..=n { let t = a.saturating_add(b); a = b; b = t; }
    b
}
extern "C" fn slang_is_prime(n: i64) -> i8 {
    if n < 2 { return 0; }
    if n < 4 { return 1; }
    if n % 2 == 0 || n % 3 == 0 { return 0; }
    let mut i = 5i64;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 { return 0; }
        i += 6;
    }
    1
}
extern "C" fn slang_tan_f64(x: f64) -> f64    { x.tan() }
extern "C" fn slang_asin_f64(x: f64) -> f64   { x.asin() }
extern "C" fn slang_acos_f64(x: f64) -> f64   { x.acos() }
extern "C" fn slang_atan_f64(x: f64) -> f64   { x.atan() }

// ── Phase 21 stdlib: hash, interpolation, numeric ────────────────────
extern "C" fn slang_hash_i64(x: i64) -> i64 {
    // MurmurHash3-style finalizer
    let mut h = x as u64;
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    h as i64
}

extern "C" fn slang_lerp_f64(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

extern "C" fn slang_smoothstep_f64(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

extern "C" fn slang_wrap_i64(x: i64, lo: i64, hi: i64) -> i64 {
    if hi <= lo { return lo; }
    let range = hi - lo;
    ((x - lo).rem_euclid(range)) + lo
}

extern "C" fn slang_map_range_f64(x: f64, in_lo: f64, in_hi: f64, out_lo: f64, out_hi: f64) -> f64 {
    let in_range = in_hi - in_lo;
    if in_range.abs() < 1e-15 { return out_lo; }
    let t = (x - in_lo) / in_range;
    out_lo + t * (out_hi - out_lo)
}

extern "C" fn slang_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── Phase 22 stdlib runtime functions ─────────────────────────────────

/// Fused multiply-add: a * b + c
extern "C" fn slang_fma_f64(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}

/// Cube root
extern "C" fn slang_cbrt_f64(x: f64) -> f64 {
    x.cbrt()
}

/// Degrees to radians
extern "C" fn slang_deg_to_rad(x: f64) -> f64 {
    x * std::f64::consts::PI / 180.0
}

/// Radians to degrees
extern "C" fn slang_rad_to_deg(x: f64) -> f64 {
    x * 180.0 / std::f64::consts::PI
}

/// Sigmoid / logistic function: 1 / (1 + e^(-x))
extern "C" fn slang_sigmoid_f64(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// ReLU: max(0, x)
extern "C" fn slang_relu_f64(x: f64) -> f64 {
    if x > 0.0 { x } else { 0.0 }
}

/// Tanh activation (already in std)
extern "C" fn slang_tanh_f64(x: f64) -> f64 {
    x.tanh()
}

/// Integer power (i64^i64) with overflow protection
extern "C" fn slang_ipow(base: i64, exp: i64) -> i64 {
    if exp < 0 { return 0; }
    let mut result: i64 = 1;
    let mut b = base;
    let mut e = exp as u64;
    while e > 0 {
        if e & 1 == 1 {
            result = result.wrapping_mul(b);
        }
        b = b.wrapping_mul(b);
        e >>= 1;
    }
    result
}

// ── Phase 23 stdlib runtime functions ─────────────────────────────────

/// Hyperbolic sine
extern "C" fn slang_sinh_f64(x: f64) -> f64 { x.sinh() }

/// Hyperbolic cosine
extern "C" fn slang_cosh_f64(x: f64) -> f64 { x.cosh() }

/// Natural log (alias for ln)
extern "C" fn slang_log_f64(x: f64) -> f64 { x.ln() }

/// Base-2 exponential: 2^x
extern "C" fn slang_exp2_f64(x: f64) -> f64 { (2.0_f64).powf(x) }

/// Copy sign of y onto magnitude of x
extern "C" fn slang_copysign_f64(x: f64, y: f64) -> f64 { x.copysign(y) }

/// Fractional part of x
extern "C" fn slang_fract_f64(x: f64) -> f64 { x.fract() }

/// Truncate toward zero
extern "C" fn slang_trunc_f64(x: f64) -> f64 { x.trunc() }

/// Step function: 0.0 if x < edge, else 1.0
extern "C" fn slang_step_f64(edge: f64, x: f64) -> f64 {
    if x < edge { 0.0 } else { 1.0 }
}

/// Leaky ReLU: x if x > 0, else alpha * x
extern "C" fn slang_leaky_relu_f64(x: f64, alpha: f64) -> f64 {
    if x > 0.0 { x } else { alpha * x }
}

/// ELU activation: x if x > 0, else alpha * (e^x - 1)
extern "C" fn slang_elu_f64(x: f64, alpha: f64) -> f64 {
    if x > 0.0 { x } else { alpha * (x.exp() - 1.0) }
}

// ── Phase 24 stdlib runtime functions ─────────────────────────────────

/// Swish activation: x * sigmoid(x)
extern "C" fn slang_swish_f64(x: f64) -> f64 { x / (1.0 + (-x).exp()) }

/// GELU activation (approximate): x * 0.5 * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
extern "C" fn slang_gelu_f64(x: f64) -> f64 {
    let c = (2.0_f64 / std::f64::consts::PI).sqrt();
    0.5 * x * (1.0 + (c * (x + 0.044715 * x.powi(3))).tanh())
}

/// Softplus: ln(1 + e^x)
extern "C" fn slang_softplus_f64(x: f64) -> f64 {
    if x > 20.0 { x } else { (1.0 + x.exp()).ln() }
}

/// Mish activation: x * tanh(softplus(x))
extern "C" fn slang_mish_f64(x: f64) -> f64 {
    let sp = if x > 20.0 { x } else { (1.0 + x.exp()).ln() };
    x * sp.tanh()
}

/// log(1 + x), numerically stable for small x
extern "C" fn slang_log1p_f64(x: f64) -> f64 { x.ln_1p() }

/// e^x - 1, numerically stable for small x
extern "C" fn slang_expm1_f64(x: f64) -> f64 { x.exp_m1() }

/// Reciprocal: 1/x
extern "C" fn slang_recip_f64(x: f64) -> f64 { x.recip() }

/// Inverse square root: 1/sqrt(x)
extern "C" fn slang_rsqrt_f64(x: f64) -> f64 { 1.0 / x.sqrt() }

// ── Phase 25 stdlib runtime functions ─────────────────────────────────

/// SELU activation: lambda * (x if x > 0, alpha*(e^x - 1) if x <= 0)
extern "C" fn slang_selu_f64(x: f64) -> f64 {
    const ALPHA: f64 = 1.6732632423543772;
    const LAMBDA: f64 = 1.0507009873554805;
    if x > 0.0 { LAMBDA * x } else { LAMBDA * ALPHA * (x.exp() - 1.0) }
}

/// Hard sigmoid: clamp((x + 3) / 6, 0, 1)
extern "C" fn slang_hard_sigmoid_f64(x: f64) -> f64 {
    ((x + 3.0) / 6.0).clamp(0.0, 1.0)
}

/// Hard swish: x * hard_sigmoid(x)
extern "C" fn slang_hard_swish_f64(x: f64) -> f64 {
    x * ((x + 3.0) / 6.0).clamp(0.0, 1.0)
}

/// Log sigmoid: log(sigmoid(x)), numerically stable
extern "C" fn slang_log_sigmoid_f64(x: f64) -> f64 {
    if x >= 0.0 { -((-x).exp().ln_1p()) } else { x - (x.exp().ln_1p()) }
}

/// CELU activation: max(0,x) + min(0, alpha*(e^(x/alpha) - 1))
extern "C" fn slang_celu_f64(x: f64) -> f64 {
    if x >= 0.0 { x } else { x.exp() - 1.0 }
}

/// Softsign: x / (1 + |x|)
extern "C" fn slang_softsign_f64(x: f64) -> f64 { x / (1.0 + x.abs()) }

/// Gaussian: e^(-x²)
extern "C" fn slang_gaussian_f64(x: f64) -> f64 { (-x * x).exp() }

/// Normalized sinc: sin(πx)/(πx), sinc(0) = 1
extern "C" fn slang_sinc_f64(x: f64) -> f64 {
    if x.abs() < 1e-15 { 1.0 } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

/// Fast inverse sqrt (Quake-style, then Newton refinement)
extern "C" fn slang_inv_sqrt_approx_f64(x: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    let mut y = 1.0 / x.sqrt();
    y = y * (1.5 - 0.5 * x * y * y); // Newton step
    y
}

/// Logit: log(x / (1 - x)), inverse sigmoid
extern "C" fn slang_logit_f64(x: f64) -> f64 {
    let x = x.clamp(1e-7, 1.0 - 1e-7);
    (x / (1.0 - x)).ln()
}

// ─── JIT Compiler ───────────────────────────────────────────────────────
pub struct JitCompiler {
    module: JITModule,
    ctx: codegen::Context,
    func_ids: HashMap<String, FuncId>,
}

/// Info about a Phi instruction for block-parameter passing.
struct PhiInfo {
    result: ir::Value,
    ty: IrType,
    incoming: Vec<(ir::Value, ir::BlockId)>,
}

impl JitCompiler {
    pub fn new() -> CodegenResult<Self> {
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").map_err(|e| CodegenError {
            message: format!("failed to set opt_level: {}", e),
        })?;

        let isa_builder = cranelift_native::builder().map_err(|e| CodegenError {
            message: format!("failed to create ISA builder: {}", e),
        })?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CodegenError {
                message: format!("failed to build ISA: {}", e),
            })?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Register runtime functions
        builder.symbol("slang_print_i64", slang_print_i64 as *const u8);
        builder.symbol("slang_print_str", slang_print_str as *const u8);
        builder.symbol("slang_println_str", slang_println_str as *const u8);
        builder.symbol("slang_println_i64", slang_println_i64 as *const u8);
        builder.symbol("slang_print_f64", slang_print_f64 as *const u8);
        builder.symbol("slang_println_f64", slang_println_f64 as *const u8);
        builder.symbol("slang_print_bool", slang_print_bool as *const u8);
        builder.symbol("slang_println_bool", slang_println_bool as *const u8);
        builder.symbol("slang_print_cstr", slang_print_cstr as *const u8);
        builder.symbol("slang_println_cstr", slang_println_cstr as *const u8);
        // Math
        builder.symbol("slang_sqrt_f64", slang_sqrt_f64 as *const u8);
        builder.symbol("slang_abs_i64", slang_abs_i64 as *const u8);
        builder.symbol("slang_abs_f64", slang_abs_f64 as *const u8);
        builder.symbol("slang_min_i64", slang_min_i64 as *const u8);
        builder.symbol("slang_max_i64", slang_max_i64 as *const u8);
        builder.symbol("slang_min_f64", slang_min_f64 as *const u8);
        builder.symbol("slang_max_f64", slang_max_f64 as *const u8);
        builder.symbol("slang_pow_f64", slang_pow_f64 as *const u8);
        builder.symbol("slang_floor_f64", slang_floor_f64 as *const u8);
        builder.symbol("slang_ceil_f64", slang_ceil_f64 as *const u8);
        builder.symbol("slang_round_f64", slang_round_f64 as *const u8);
        builder.symbol("slang_ln_f64", slang_ln_f64 as *const u8);
        builder.symbol("slang_log2_f64", slang_log2_f64 as *const u8);
        builder.symbol("slang_log10_f64", slang_log10_f64 as *const u8);
        builder.symbol("slang_sin_f64", slang_sin_f64 as *const u8);
        builder.symbol("slang_cos_f64", slang_cos_f64 as *const u8);
        builder.symbol("slang_exp_f64", slang_exp_f64 as *const u8);
        // Conversion
        builder.symbol("slang_i64_to_f64", slang_i64_to_f64 as *const u8);
        builder.symbol("slang_f64_to_i64", slang_f64_to_i64 as *const u8);
        // Strings
        builder.symbol("slang_str_len", slang_str_len as *const u8);
        builder.symbol("slang_str_eq", slang_str_eq as *const u8);
        builder.symbol("slang_str_cat", slang_str_cat as *const u8);
        // Extended math
        builder.symbol("slang_atan2_f64", slang_atan2_f64 as *const u8);
        builder.symbol("slang_hypot_f64", slang_hypot_f64 as *const u8);
        builder.symbol("slang_clamp_f64", slang_clamp_f64 as *const u8);
        builder.symbol("slang_clamp_i64", slang_clamp_i64 as *const u8);
        builder.symbol("slang_rand_f64",  slang_rand_f64  as *const u8);
        builder.symbol("slang_rand_i64",  slang_rand_i64  as *const u8);
        // Phase 4: Array heap runtime
        builder.symbol("slang_array_alloc",   slang_array_alloc   as *const u8);
        builder.symbol("slang_array_len",     slang_array_len     as *const u8);
        builder.symbol("slang_array_get_i64", slang_array_get_i64 as *const u8);
        builder.symbol("slang_array_set_i64", slang_array_set_i64 as *const u8);
        builder.symbol("slang_array_get_f64", slang_array_get_f64 as *const u8);
        builder.symbol("slang_array_set_f64", slang_array_set_f64 as *const u8);
        // Phase 5: New stdlib runtime symbols
        builder.symbol("slang_clock_ns",         slang_clock_ns         as *const u8);
        builder.symbol("slang_clock_ms",         slang_clock_ms         as *const u8);
        builder.symbol("slang_assert_eq_i64",    slang_assert_eq_i64    as *const u8);
        builder.symbol("slang_assert_true",      slang_assert_true      as *const u8);
        builder.symbol("slang_popcount",         slang_popcount         as *const u8);
        builder.symbol("slang_leading_zeros",    slang_leading_zeros    as *const u8);
        builder.symbol("slang_trailing_zeros",   slang_trailing_zeros   as *const u8);
        builder.symbol("slang_sign_i64",         slang_sign_i64         as *const u8);
        builder.symbol("slang_gcd",              slang_gcd              as *const u8);
        builder.symbol("slang_lcm",              slang_lcm              as *const u8);
        builder.symbol("slang_factorial",        slang_factorial        as *const u8);
        builder.symbol("slang_fibonacci",        slang_fibonacci        as *const u8);
        builder.symbol("slang_is_prime",         slang_is_prime         as *const u8);
        builder.symbol("slang_tan_f64",          slang_tan_f64          as *const u8);
        builder.symbol("slang_asin_f64",         slang_asin_f64         as *const u8);
        builder.symbol("slang_acos_f64",         slang_acos_f64         as *const u8);
        builder.symbol("slang_atan_f64",         slang_atan_f64         as *const u8);
        // Phase 21 stdlib symbols
        builder.symbol("slang_hash_i64",         slang_hash_i64         as *const u8);
        builder.symbol("slang_lerp_f64",         slang_lerp_f64         as *const u8);
        builder.symbol("slang_smoothstep_f64",   slang_smoothstep_f64   as *const u8);
        builder.symbol("slang_wrap_i64",         slang_wrap_i64         as *const u8);
        builder.symbol("slang_map_range_f64",    slang_map_range_f64    as *const u8);
        builder.symbol("slang_epoch_secs",       slang_epoch_secs       as *const u8);
        // Phase 22 stdlib symbols
        builder.symbol("slang_fma_f64",          slang_fma_f64          as *const u8);
        builder.symbol("slang_cbrt_f64",         slang_cbrt_f64         as *const u8);
        builder.symbol("slang_deg_to_rad",       slang_deg_to_rad       as *const u8);
        builder.symbol("slang_rad_to_deg",       slang_rad_to_deg       as *const u8);
        builder.symbol("slang_sigmoid_f64",      slang_sigmoid_f64      as *const u8);
        builder.symbol("slang_relu_f64",         slang_relu_f64         as *const u8);
        builder.symbol("slang_tanh_f64",         slang_tanh_f64         as *const u8);
        builder.symbol("slang_ipow",             slang_ipow             as *const u8);
        // Phase 23 stdlib symbols
        builder.symbol("slang_sinh_f64",         slang_sinh_f64         as *const u8);
        builder.symbol("slang_cosh_f64",         slang_cosh_f64         as *const u8);
        builder.symbol("slang_log_f64",          slang_log_f64          as *const u8);
        builder.symbol("slang_exp2_f64",         slang_exp2_f64         as *const u8);
        builder.symbol("slang_copysign_f64",     slang_copysign_f64     as *const u8);
        builder.symbol("slang_fract_f64",        slang_fract_f64        as *const u8);
        builder.symbol("slang_trunc_f64",        slang_trunc_f64        as *const u8);
        builder.symbol("slang_step_f64",         slang_step_f64         as *const u8);
        builder.symbol("slang_leaky_relu_f64",   slang_leaky_relu_f64   as *const u8);
        builder.symbol("slang_elu_f64",          slang_elu_f64          as *const u8);
        // Phase 24 stdlib symbols
        builder.symbol("slang_swish_f64",        slang_swish_f64        as *const u8);
        builder.symbol("slang_gelu_f64",         slang_gelu_f64         as *const u8);
        builder.symbol("slang_softplus_f64",     slang_softplus_f64     as *const u8);
        builder.symbol("slang_mish_f64",         slang_mish_f64         as *const u8);
        builder.symbol("slang_log1p_f64",        slang_log1p_f64        as *const u8);
        builder.symbol("slang_expm1_f64",        slang_expm1_f64        as *const u8);
        builder.symbol("slang_recip_f64",        slang_recip_f64        as *const u8);
        builder.symbol("slang_rsqrt_f64",        slang_rsqrt_f64        as *const u8);
        // Phase 25 stdlib symbols
        builder.symbol("slang_selu_f64",          slang_selu_f64          as *const u8);
        builder.symbol("slang_hard_sigmoid_f64",  slang_hard_sigmoid_f64  as *const u8);
        builder.symbol("slang_hard_swish_f64",    slang_hard_swish_f64    as *const u8);
        builder.symbol("slang_log_sigmoid_f64",   slang_log_sigmoid_f64   as *const u8);
        builder.symbol("slang_celu_f64",          slang_celu_f64          as *const u8);
        builder.symbol("slang_softsign_f64",      slang_softsign_f64      as *const u8);
        builder.symbol("slang_gaussian_f64",      slang_gaussian_f64      as *const u8);
        builder.symbol("slang_sinc_f64",          slang_sinc_f64          as *const u8);
        builder.symbol("slang_inv_sqrt_approx_f64", slang_inv_sqrt_approx_f64 as *const u8);
        builder.symbol("slang_logit_f64",         slang_logit_f64         as *const u8);

        let module = JITModule::new(builder);
        let ctx = module.make_context();

        Ok(Self {
            module,
            ctx,
            func_ids: HashMap::new(),
        })
    }

    /// Compile an IR module into native code.
    pub fn compile(&mut self, ir_module: &IrModule) -> CodegenResult<()> {
        // Declare all functions first (for forward references)
        for func in &ir_module.functions {
            self.declare_function(func)?;
        }

        // Declare runtime support functions
        self.declare_runtime_functions()?;

        // Define all functions
        for func in &ir_module.functions {
            self.define_function(func)?;
        }

        // Finalize all functions
        self.module.finalize_definitions().map_err(|e| CodegenError {
            message: format!("failed to finalize: {}", e),
        })?;

        Ok(())
    }

    fn declare_runtime_functions(&mut self) -> CodegenResult<()> {
        // slang_print_i64(i64) -> void
        {
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(types::I64));
            let id = self
                .module
                .declare_function("slang_print_i64", Linkage::Import, &sig)
                .map_err(|e| CodegenError {
                    message: format!("declare slang_print_i64: {}", e),
                })?;
            self.func_ids.insert("slang_print_i64".into(), id);
        }

        // slang_print_str(ptr, len) -> void
        {
            let ptr_type = self.module.target_config().pointer_type();
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(ptr_type));
            sig.params.push(AbiParam::new(types::I64));
            let id = self
                .module
                .declare_function("slang_print_str", Linkage::Import, &sig)
                .map_err(|e| CodegenError {
                    message: format!("declare slang_print_str: {}", e),
                })?;
            self.func_ids.insert("slang_print_str".into(), id);
        }

        // slang_println_str(ptr, len) -> void
        {
            let ptr_type = self.module.target_config().pointer_type();
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(ptr_type));
            sig.params.push(AbiParam::new(types::I64));
            let id = self
                .module
                .declare_function("slang_println_str", Linkage::Import, &sig)
                .map_err(|e| CodegenError {
                    message: format!("declare slang_println_str: {}", e),
                })?;
            self.func_ids.insert("slang_println_str".into(), id);
        }

        // ── Typed printing ──────────────────────────────────────────────
        macro_rules! decl_fn {
            ($name:literal, [ $($param:expr),* ], $ret:expr) => {{
                let mut sig = self.module.make_signature();
                $( sig.params.push(AbiParam::new($param)); )*
                if $ret != types::INVALID { sig.returns.push(AbiParam::new($ret)); }
                let id = self.module.declare_function($name, Linkage::Import, &sig)
                    .map_err(|e| CodegenError { message: format!("declare {}: {}", $name, e) })?;
                self.func_ids.insert($name.to_string(), id);
            }};
        }
        let ptr_type = self.module.target_config().pointer_type();
        decl_fn!("slang_println_i64",   [types::I64],              types::INVALID);
        decl_fn!("slang_print_f64",     [types::F64],              types::INVALID);
        decl_fn!("slang_println_f64",   [types::F64],              types::INVALID);
        decl_fn!("slang_print_bool",    [types::I8],               types::INVALID);
        decl_fn!("slang_println_bool",  [types::I8],               types::INVALID);
        decl_fn!("slang_print_cstr",    [ptr_type],                types::INVALID);
        decl_fn!("slang_println_cstr",  [ptr_type],                types::INVALID);
        // Math (f64 → f64)
        decl_fn!("slang_sqrt_f64",  [types::F64],              types::F64);
        decl_fn!("slang_ln_f64",    [types::F64],              types::F64);
        decl_fn!("slang_log2_f64",  [types::F64],              types::F64);
        decl_fn!("slang_log10_f64", [types::F64],              types::F64);
        decl_fn!("slang_sin_f64",   [types::F64],              types::F64);
        decl_fn!("slang_cos_f64",   [types::F64],              types::F64);
        decl_fn!("slang_exp_f64",   [types::F64],              types::F64);
        decl_fn!("slang_floor_f64", [types::F64],              types::F64);
        decl_fn!("slang_ceil_f64",  [types::F64],              types::F64);
        decl_fn!("slang_round_f64", [types::F64],              types::F64);
        decl_fn!("slang_abs_f64",   [types::F64],              types::F64);
        decl_fn!("slang_pow_f64",   [types::F64, types::F64],  types::F64);
        decl_fn!("slang_min_f64",   [types::F64, types::F64],  types::F64);
        decl_fn!("slang_max_f64",   [types::F64, types::F64],  types::F64);
        // Math (i64)
        decl_fn!("slang_abs_i64",   [types::I64],              types::I64);
        decl_fn!("slang_min_i64",   [types::I64, types::I64],  types::I64);
        decl_fn!("slang_max_i64",   [types::I64, types::I64],  types::I64);
        // Conversion
        decl_fn!("slang_i64_to_f64", [types::I64], types::F64);
        decl_fn!("slang_f64_to_i64", [types::F64], types::I64);
        // Strings
        decl_fn!("slang_str_len",   [ptr_type],            types::I64);
        decl_fn!("slang_str_eq",    [ptr_type, ptr_type],  types::I8);
        decl_fn!("slang_str_cat",   [ptr_type, ptr_type],  ptr_type);
        // Extended math
        decl_fn!("slang_atan2_f64",  [types::F64, types::F64],                          types::F64);
        decl_fn!("slang_hypot_f64",  [types::F64, types::F64],                          types::F64);
        decl_fn!("slang_clamp_f64",  [types::F64, types::F64, types::F64],              types::F64);
        decl_fn!("slang_clamp_i64",  [types::I64, types::I64, types::I64],              types::I64);
        decl_fn!("slang_rand_f64",   [],                                                types::F64);
        decl_fn!("slang_rand_i64",   [],                                                types::I64);
        // Phase 4: Array heap runtime
        decl_fn!("slang_array_alloc",   [types::I64, types::I64],    ptr_type);
        decl_fn!("slang_array_len",     [ptr_type],                  types::I64);
        decl_fn!("slang_array_get_i64", [ptr_type, types::I64],      types::I64);
        decl_fn!("slang_array_set_i64", [ptr_type, types::I64, types::I64],   types::INVALID);
        decl_fn!("slang_array_get_f64", [ptr_type, types::I64],      types::F64);
        decl_fn!("slang_array_set_f64", [ptr_type, types::I64, types::F64],   types::INVALID);
        // Phase 5: New stdlib declarations
        decl_fn!("slang_clock_ns",        [],                            types::I64);
        decl_fn!("slang_clock_ms",        [],                            types::I64);
        decl_fn!("slang_assert_eq_i64",   [types::I64, types::I64],     types::INVALID);
        decl_fn!("slang_assert_true",     [types::I8],                   types::INVALID);
        decl_fn!("slang_popcount",        [types::I64],                  types::I64);
        decl_fn!("slang_leading_zeros",   [types::I64],                  types::I64);
        decl_fn!("slang_trailing_zeros",  [types::I64],                  types::I64);
        decl_fn!("slang_sign_i64",        [types::I64],                  types::I64);
        decl_fn!("slang_gcd",             [types::I64, types::I64],      types::I64);
        decl_fn!("slang_lcm",             [types::I64, types::I64],      types::I64);
        decl_fn!("slang_factorial",       [types::I64],                  types::I64);
        decl_fn!("slang_fibonacci",       [types::I64],                  types::I64);
        decl_fn!("slang_is_prime",        [types::I64],                  types::I8);
        decl_fn!("slang_tan_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_asin_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_acos_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_atan_f64",        [types::F64],                  types::F64);
        // Phase 21 stdlib declarations
        decl_fn!("slang_hash_i64",        [types::I64],                  types::I64);
        decl_fn!("slang_lerp_f64",        [types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_smoothstep_f64",  [types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_wrap_i64",        [types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_map_range_f64",   [types::F64, types::F64, types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_epoch_secs",      [],                            types::I64);
        // Phase 22 stdlib declarations
        decl_fn!("slang_fma_f64",         [types::F64, types::F64, types::F64], types::F64);
        decl_fn!("slang_cbrt_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_deg_to_rad",      [types::F64],                  types::F64);
        decl_fn!("slang_rad_to_deg",      [types::F64],                  types::F64);
        decl_fn!("slang_sigmoid_f64",     [types::F64],                  types::F64);
        decl_fn!("slang_relu_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_tanh_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_ipow",            [types::I64, types::I64],      types::I64);
        // Phase 23 stdlib declarations
        decl_fn!("slang_sinh_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_cosh_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_log_f64",          [types::F64],                  types::F64);
        decl_fn!("slang_exp2_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_copysign_f64",     [types::F64, types::F64],      types::F64);
        decl_fn!("slang_fract_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_trunc_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_step_f64",         [types::F64, types::F64],      types::F64);
        decl_fn!("slang_leaky_relu_f64",   [types::F64, types::F64],      types::F64);
        decl_fn!("slang_elu_f64",          [types::F64, types::F64],      types::F64);
        // Phase 24 stdlib declarations
        decl_fn!("slang_swish_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_gelu_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_softplus_f64",     [types::F64],                  types::F64);
        decl_fn!("slang_mish_f64",         [types::F64],                  types::F64);
        decl_fn!("slang_log1p_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_expm1_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_recip_f64",        [types::F64],                  types::F64);
        decl_fn!("slang_rsqrt_f64",        [types::F64],                  types::F64);
        // Phase 25 stdlib declarations
        decl_fn!("slang_selu_f64",          [types::F64],                  types::F64);
        decl_fn!("slang_hard_sigmoid_f64",  [types::F64],                  types::F64);
        decl_fn!("slang_hard_swish_f64",    [types::F64],                  types::F64);
        decl_fn!("slang_log_sigmoid_f64",   [types::F64],                  types::F64);
        decl_fn!("slang_celu_f64",          [types::F64],                  types::F64);
        decl_fn!("slang_softsign_f64",      [types::F64],                  types::F64);
        decl_fn!("slang_gaussian_f64",      [types::F64],                  types::F64);
        decl_fn!("slang_sinc_f64",          [types::F64],                  types::F64);
        decl_fn!("slang_inv_sqrt_approx_f64", [types::F64],                types::F64);
        decl_fn!("slang_logit_f64",         [types::F64],                  types::F64);

        Ok(())
    }

    fn ir_type_to_cl(ty: &IrType, pointer_type: cranelift::prelude::Type) -> cranelift::prelude::Type {
        match ty {
            IrType::I32 => types::I32,
            IrType::I64 => types::I64,
            IrType::F32 => types::F32,
            IrType::F64 => types::F64,
            IrType::Bool => types::I8,
            IrType::Ptr => pointer_type,
            IrType::Void => types::I64, // Cranelift needs a type; we ignore the value
        }
    }

    fn declare_function(&mut self, func: &IrFunction) -> CodegenResult<()> {
        let mut sig = self.module.make_signature();
        let pointer_type = self.module.target_config().pointer_type();

        for (_, ty) in &func.params {
            sig.params.push(AbiParam::new(Self::ir_type_to_cl(ty, pointer_type)));
        }

        if func.ret_type != IrType::Void {
            sig.returns.push(AbiParam::new(Self::ir_type_to_cl(&func.ret_type, pointer_type)));
        }

        let id = self
            .module
            .declare_function(&func.name, Linkage::Export, &sig)
            .map_err(|e| CodegenError {
                message: format!("declare {}: {}", func.name, e),
            })?;

        self.func_ids.insert(func.name.clone(), id);
        Ok(())
    }

    fn define_function(&mut self, func: &IrFunction) -> CodegenResult<()> {
        let func_id = *self.func_ids.get(&func.name).ok_or_else(|| CodegenError {
            message: format!("function {} not declared", func.name),
        })?;

        // Build signature
        self.ctx.func.signature = self.module.declarations().get_function_decl(func_id).signature.clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        let pointer_type = self.module.target_config().pointer_type();
        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        // Create Cranelift blocks for each IR block
        let mut block_map: HashMap<ir::BlockId, cranelift::prelude::Block> = HashMap::new();
        for bb in &func.blocks {
            let cl_block = builder.create_block();
            block_map.insert(bb.id, cl_block);
        }

        // ── Pre-pass: collect Phi info for block parameters ──
        // For each block that contains Phi instructions, we need to add
        // Cranelift block parameters so values can flow across blocks.
        let mut phis_by_block: HashMap<ir::BlockId, Vec<PhiInfo>> = HashMap::new();
        for bb in &func.blocks {
            for inst in &bb.insts {
                if let Inst::Phi { result, incoming, ty } = inst {
                    phis_by_block.entry(bb.id).or_default().push(PhiInfo {
                        result: *result,
                        ty: ty.clone(),
                        incoming: incoming.clone(),
                    });
                }
            }
        }

        // Add block parameters for blocks that have Phi instructions
        for (block_id, phis) in &phis_by_block {
            if let Some(cl_block) = block_map.get(block_id) {
                for phi in phis {
                    builder.append_block_param(*cl_block, Self::ir_type_to_cl(&phi.ty, pointer_type));
                }
            }
        }

        // Set up entry block with function parameters
        let entry_block = block_map[&func.entry];
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Map IR values to Cranelift values
        let mut value_map: HashMap<ir::Value, cranelift::prelude::Value> = HashMap::new();
        // Semantic type map: lets print/println dispatch to the right runtime function
        let mut type_map: HashMap<ir::Value, IrType> = HashMap::new();

        // Bind function parameters
        let func_params: Vec<cranelift::prelude::Value> = builder.block_params(entry_block).to_vec();
        for (i, (_name, ty)) in func.params.iter().enumerate() {
            if i < func_params.len() {
                value_map.insert(ir::Value(i as u32), func_params[i]);
                type_map.insert(ir::Value(i as u32), ty.clone());
            }
        }

        // Translate each IR block
        let mut first = true;
        for bb in &func.blocks {
            if !first {
                let cl_block = block_map[&bb.id];
                builder.switch_to_block(cl_block);
                // Don't seal here — seal all blocks at end to handle arbitrary CFGs
            }
            first = false;

            // If this block has Phi params, map them to the block parameters
            if let Some(phis) = phis_by_block.get(&bb.id) {
                let cl_block = block_map[&bb.id];
                let params = builder.block_params(cl_block).to_vec();
                for (i, phi) in phis.iter().enumerate() {
                    if i < params.len() {
                        value_map.insert(phi.result, params[i]);
                    }
                }
            }

            for inst in &bb.insts {
                Self::translate_inst(
                    &mut self.module,
                    &self.func_ids,
                    pointer_type,
                    inst,
                    &mut builder,
                    &block_map,
                    &mut value_map,
                    &mut type_map,
                    &func.ret_type,
                    bb.id,
                    &phis_by_block,
                )?;
            }
        }

        // Ensure the last block is terminated
        let needs_terminator = func.blocks.last().map_or(true, |bb| {
            !bb.insts.iter().any(|i| matches!(i, Inst::Return { .. } | Inst::Jump { .. } | Inst::Branch { .. }))
        });
        if needs_terminator {
            if func.ret_type == IrType::Void {
                builder.ins().return_(&[]);
            } else if func.ret_type == IrType::F64 {
                // iconst is integer-only; float returns need f64const/f32const
                let zero = builder.ins().f64const(0.0_f64);
                builder.ins().return_(&[zero]);
            } else if func.ret_type == IrType::F32 {
                let zero = builder.ins().f32const(0.0_f32);
                builder.ins().return_(&[zero]);
            } else {
                let zero = builder.ins().iconst(Self::ir_type_to_cl(&func.ret_type, pointer_type), 0);
                builder.ins().return_(&[zero]);
            }
        }

        // Seal all blocks at once (correct for arbitrary control flow)
        builder.seal_all_blocks();
        builder.finalize();

        // Define the function
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| CodegenError {
                message: format!("define {}: {}", func.name, e),
            })?;

        self.module.clear_context(&mut self.ctx);
        Ok(())
    }

    fn translate_inst(
        module: &mut JITModule,
        func_ids: &HashMap<String, FuncId>,
        pointer_type: cranelift::prelude::Type,
        inst: &Inst,
        builder: &mut FunctionBuilder,
        block_map: &HashMap<ir::BlockId, cranelift::prelude::Block>,
        value_map: &mut HashMap<ir::Value, cranelift::prelude::Value>,
        type_map: &mut HashMap<ir::Value, IrType>,
        _ret_type: &IrType,
        current_bb: ir::BlockId,
        phis_by_block: &HashMap<ir::BlockId, Vec<PhiInfo>>,
    ) -> CodegenResult<()> {
        match inst {
            Inst::IConst { result, value, ty } => {
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = builder.ins().iconst(cl_ty, *value);
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::FConst { result, value, ty } => {
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = if cl_ty == types::F64 {
                    builder.ins().f64const(*value)
                } else {
                    builder.ins().f32const(*value as f32)
                };
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::BConst { result, value } => {
                let val = builder.ins().iconst(types::I8, if *value { 1 } else { 0 });
                value_map.insert(*result, val);
                type_map.insert(*result, IrType::Bool);
            }
            Inst::StrConst { result, value } => {
                // Intern into arena; return raw pointer as iconst
                let ptr = intern_cstr(value) as i64;
                let val = builder.ins().iconst(pointer_type, ptr);
                value_map.insert(*result, val);
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::BinOp { result, op, lhs, rhs, ty } => {
                let l = Self::get_value(*lhs, value_map, builder);
                let r = Self::get_value(*rhs, value_map, builder);
                let val = match op {
                    IrBinOp::Add => builder.ins().iadd(l, r),
                    IrBinOp::Sub => builder.ins().isub(l, r),
                    IrBinOp::Mul => builder.ins().imul(l, r),
                    IrBinOp::Div => builder.ins().sdiv(l, r),
                    IrBinOp::Mod => builder.ins().srem(l, r),
                    IrBinOp::FAdd => builder.ins().fadd(l, r),
                    IrBinOp::FSub => builder.ins().fsub(l, r),
                    IrBinOp::FMul => builder.ins().fmul(l, r),
                    IrBinOp::FDiv => builder.ins().fdiv(l, r),
                    IrBinOp::And => builder.ins().band(l, r),
                    IrBinOp::Or => builder.ins().bor(l, r),
                };
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::UnOp { result, op, operand, ty } => {
                let v = Self::get_value(*operand, value_map, builder);
                let val = match op {
                    IrUnOp::Neg => builder.ins().ineg(v),
                    IrUnOp::FNeg => builder.ins().fneg(v),
                    IrUnOp::Not => {
                        let one = builder.ins().iconst(types::I8, 1);
                        builder.ins().bxor(v, one)
                    }
                };
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::ICmp { result, cond, lhs, rhs } => {
                let l = Self::get_value(*lhs, value_map, builder);
                let r = Self::get_value(*rhs, value_map, builder);
                let cc = match cond {
                    IrCmp::Eq => IntCC::Equal,
                    IrCmp::Ne => IntCC::NotEqual,
                    IrCmp::Lt => IntCC::SignedLessThan,
                    IrCmp::Gt => IntCC::SignedGreaterThan,
                    IrCmp::Le => IntCC::SignedLessThanOrEqual,
                    IrCmp::Ge => IntCC::SignedGreaterThanOrEqual,
                };
                let val = builder.ins().icmp(cc, l, r);
                value_map.insert(*result, val);
                type_map.insert(*result, IrType::Bool);
            }
            Inst::FCmp { result, cond, lhs, rhs } => {
                let l = Self::get_value(*lhs, value_map, builder);
                let r = Self::get_value(*rhs, value_map, builder);
                let cc = match cond {
                    IrCmp::Eq => FloatCC::Equal,
                    IrCmp::Ne => FloatCC::NotEqual,
                    IrCmp::Lt => FloatCC::LessThan,
                    IrCmp::Gt => FloatCC::GreaterThan,
                    IrCmp::Le => FloatCC::LessThanOrEqual,
                    IrCmp::Ge => FloatCC::GreaterThanOrEqual,
                };
                let val = builder.ins().fcmp(cc, l, r);
                value_map.insert(*result, val);
                type_map.insert(*result, IrType::Bool);
            }
            Inst::Call { result, func, args, ret_ty } => {
                // Determine argument semantic type for print/println dispatch
                let arg0_itype = args.first()
                    .and_then(|v| type_map.get(v))
                    .cloned()
                    .unwrap_or(IrType::I64);

                // Route to the correct typed runtime function
                let callee_name: String = match func.as_str() {
                    "print" => match arg0_itype {
                        IrType::F64 | IrType::F32 => "slang_print_f64".into(),
                        IrType::Bool              => "slang_print_bool".into(),
                        IrType::Ptr               => "slang_print_cstr".into(),
                        _                         => "slang_print_i64".into(),
                    },
                    "println" => match arg0_itype {
                        IrType::F64 | IrType::F32 => "slang_println_f64".into(),
                        IrType::Bool              => "slang_println_bool".into(),
                        IrType::Ptr               => "slang_println_cstr".into(),
                        _                         => "slang_println_i64".into(),
                    },
                    // User-callable named math / string builtins
                    "sqrt"      => "slang_sqrt_f64".into(),
                    "ln"        => "slang_ln_f64".into(),
                    "log2"      => "slang_log2_f64".into(),
                    "log10"     => "slang_log10_f64".into(),
                    "sin"       => "slang_sin_f64".into(),
                    "cos"       => "slang_cos_f64".into(),
                    "exp"       => "slang_exp_f64".into(),
                    "floor"     => "slang_floor_f64".into(),
                    "ceil"      => "slang_ceil_f64".into(),
                    "round"     => "slang_round_f64".into(),
                    "abs"       => match arg0_itype { IrType::F64 => "slang_abs_f64".into(), _ => "slang_abs_i64".into() },
                    "abs_f64"   => "slang_abs_f64".into(),
                    "abs_i64"   => "slang_abs_i64".into(),
                    "min"       => match arg0_itype { IrType::F64 => "slang_min_f64".into(), _ => "slang_min_i64".into() },
                    "min_f64"   => "slang_min_f64".into(),
                    "min_i64"   => "slang_min_i64".into(),
                    "max"       => match arg0_itype { IrType::F64 => "slang_max_f64".into(), _ => "slang_max_i64".into() },
                    "max_f64"   => "slang_max_f64".into(),
                    "max_i64"   => "slang_max_i64".into(),
                    "pow"       => "slang_pow_f64".into(),
                    "pow_f64"   => "slang_pow_f64".into(),
                    "sqrt_f64"  => "slang_sqrt_f64".into(),
                    "ln_f64"    => "slang_ln_f64".into(),
                    "log2_f64"  => "slang_log2_f64".into(),
                    "log10_f64" => "slang_log10_f64".into(),
                    "sin_f64"   => "slang_sin_f64".into(),
                    "cos_f64"   => "slang_cos_f64".into(),
                    "exp_f64"   => "slang_exp_f64".into(),
                    "floor_f64" => "slang_floor_f64".into(),
                    "ceil_f64"  => "slang_ceil_f64".into(),
                    "round_f64" => "slang_round_f64".into(),
                    "i64_to_f64" | "to_f64" => "slang_i64_to_f64".into(),
                    "f64_to_i64" | "to_i64" => "slang_f64_to_i64".into(),
                    "str_len"   => "slang_str_len".into(),
                    "str_eq"    => "slang_str_eq".into(),
                    "str_cat"   => "slang_str_cat".into(),
                    "atan2"     => "slang_atan2_f64".into(),
                    "atan2_f64" => "slang_atan2_f64".into(),
                    "hypot"     => "slang_hypot_f64".into(),
                    "hypot_f64" => "slang_hypot_f64".into(),
                    "clamp"     => match arg0_itype {
                        IrType::F64 => "slang_clamp_f64".into(),
                        _ => "slang_clamp_i64".into()
                    },
                    "clamp_f64" => "slang_clamp_f64".into(),
                    "clamp_i64" => "slang_clamp_i64".into(),
                    "rand_f64"  => "slang_rand_f64".into(),
                    "rand_i64"  => "slang_rand_i64".into(),
                    // Phase 5: new stdlib
                    "clock_ns"        => "slang_clock_ns".into(),
                    "clock_ms"        => "slang_clock_ms".into(),
                    "assert_eq"       => "slang_assert_eq_i64".into(),
                    "assert_true"     => "slang_assert_true".into(),
                    "popcount"        => "slang_popcount".into(),
                    "leading_zeros"   => "slang_leading_zeros".into(),
                    "trailing_zeros"  => "slang_trailing_zeros".into(),
                    "sign"            => "slang_sign_i64".into(),
                    "gcd"             => "slang_gcd".into(),
                    "lcm"             => "slang_lcm".into(),
                    "factorial"       => "slang_factorial".into(),
                    "fibonacci"       => "slang_fibonacci".into(),
                    "is_prime"        => "slang_is_prime".into(),
                    "tan"             => "slang_tan_f64".into(),
                    "tan_f64"         => "slang_tan_f64".into(),
                    "asin"            => "slang_asin_f64".into(),
                    "asin_f64"        => "slang_asin_f64".into(),
                    "acos"            => "slang_acos_f64".into(),
                    "acos_f64"        => "slang_acos_f64".into(),
                    "atan"            => "slang_atan_f64".into(),
                    "atan_f64"        => "slang_atan_f64".into(),
                    // Phase 21 stdlib
                    "hash"            => "slang_hash_i64".into(),
                    "lerp"            => "slang_lerp_f64".into(),
                    "smoothstep"      => "slang_smoothstep_f64".into(),
                    "wrap"            => "slang_wrap_i64".into(),
                    "map_range"       => "slang_map_range_f64".into(),
                    "epoch_secs"      => "slang_epoch_secs".into(),
                    // Phase 22 stdlib
                    "fma"             => "slang_fma_f64".into(),
                    "cbrt"            => "slang_cbrt_f64".into(),
                    "deg_to_rad"      => "slang_deg_to_rad".into(),
                    "rad_to_deg"      => "slang_rad_to_deg".into(),
                    "sigmoid"         => "slang_sigmoid_f64".into(),
                    "relu"            => "slang_relu_f64".into(),
                    "tanh"            => "slang_tanh_f64".into(),
                    "ipow"            => "slang_ipow".into(),
                    // Phase 23 stdlib
                    "sinh"            => "slang_sinh_f64".into(),
                    "cosh"            => "slang_cosh_f64".into(),
                    "log"             => "slang_log_f64".into(),
                    "exp2"            => "slang_exp2_f64".into(),
                    "copysign"        => "slang_copysign_f64".into(),
                    "fract"           => "slang_fract_f64".into(),
                    "trunc"           => "slang_trunc_f64".into(),
                    "step"            => "slang_step_f64".into(),
                    "leaky_relu"      => "slang_leaky_relu_f64".into(),
                    "elu"             => "slang_elu_f64".into(),
                    // Phase 24 stdlib
                    "swish"           => "slang_swish_f64".into(),
                    "gelu"            => "slang_gelu_f64".into(),
                    "softplus"        => "slang_softplus_f64".into(),
                    "mish"            => "slang_mish_f64".into(),
                    "log1p"           => "slang_log1p_f64".into(),
                    "expm1"           => "slang_expm1_f64".into(),
                    "recip"           => "slang_recip_f64".into(),
                    "rsqrt"           => "slang_rsqrt_f64".into(),
                    // Phase 25 stdlib
                    "selu"            => "slang_selu_f64".into(),
                    "hard_sigmoid"    => "slang_hard_sigmoid_f64".into(),
                    "hard_swish"      => "slang_hard_swish_f64".into(),
                    "log_sigmoid"     => "slang_log_sigmoid_f64".into(),
                    "celu"            => "slang_celu_f64".into(),
                    "softsign"        => "slang_softsign_f64".into(),
                    "gaussian"        => "slang_gaussian_f64".into(),
                    "sinc"            => "slang_sinc_f64".into(),
                    "inv_sqrt_approx" => "slang_inv_sqrt_approx_f64".into(),
                    "logit"           => "slang_logit_f64".into(),
                    other       => other.to_string(),
                };

                if let Some(func_id) = func_ids.get(&callee_name) {
                    let func_ref = module.declare_func_in_func(*func_id, builder.func);
                    let arg_vals: Vec<cranelift::prelude::Value> = args
                        .iter()
                        .map(|a| Self::get_value(*a, value_map, builder))
                        .collect();
                    let call = builder.ins().call(func_ref, &arg_vals);
                    let results = builder.inst_results(call);
                    if !results.is_empty() {
                        value_map.insert(*result, results[0]);
                    } else {
                        let dummy = builder.ins().iconst(types::I64, 0);
                        value_map.insert(*result, dummy);
                    }
                } else {
                    // Unknown function — insert type-correct dummy value
                    let dummy = match ret_ty {
                        IrType::F64 => builder.ins().f64const(0.0_f64),
                        IrType::F32 => builder.ins().f32const(0.0_f32),
                        _ => builder.ins().iconst(Self::ir_type_to_cl(ret_ty, pointer_type), 0),
                    };
                    value_map.insert(*result, dummy);
                }
                type_map.insert(*result, ret_ty.clone());
            }
            Inst::Return { value } => {
                if let Some(val) = value {
                    let v = Self::get_value(*val, value_map, builder);
                    builder.ins().return_(&[v]);
                } else {
                    builder.ins().return_(&[]);
                }
            }
            Inst::Jump { target } => {
                if let Some(cl_block) = block_map.get(target) {
                    // If target has Phi instructions, pass the incoming values
                    // from this source block as block parameters
                    if let Some(phis) = phis_by_block.get(target) {
                        let mut args = Vec::new();
                        for phi in phis {
                            let val = phi.incoming.iter()
                                .find(|(_, from)| *from == current_bb)
                                .map(|(v, _)| Self::get_value(*v, value_map, builder))
                                .unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                            args.push(val);
                        }
                        builder.ins().jump(*cl_block, &args);
                    } else {
                        builder.ins().jump(*cl_block, &[]);
                    }
                }
            }
            Inst::Branch { cond, then_bb, else_bb } => {
                let cv = Self::get_value(*cond, value_map, builder);
                let then_block = block_map.get(then_bb).copied();
                let else_block = block_map.get(else_bb).copied();
                if let (Some(tb), Some(eb)) = (then_block, else_block) {
                    builder.ins().brif(cv, tb, &[], eb, &[]);
                }
            }
            Inst::Phi { result, incoming: _, ty } => {
                if !value_map.contains_key(result) {
                    let dummy = builder.ins().iconst(types::I64, 0);
                    value_map.insert(*result, dummy);
                }
                type_map.insert(*result, ty.clone());
            }
            Inst::Alloca { result, size } => {
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    *size,
                    0,
                ));
                let addr = builder.ins().stack_addr(pointer_type, slot, 0);
                value_map.insert(*result, addr);
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::Load { result, ptr, ty } => {
                let p = Self::get_value(*ptr, value_map, builder);
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = builder.ins().load(cl_ty, MemFlags::new(), p, 0);
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::Store { value, ptr } => {
                let v = Self::get_value(*value, value_map, builder);
                let p = Self::get_value(*ptr, value_map, builder);
                builder.ins().store(MemFlags::new(), v, p, 0);
            }
            Inst::Copy { result, source } => {
                let v = Self::get_value(*source, value_map, builder);
                value_map.insert(*result, v);
                if let Some(ty) = type_map.get(source).cloned() {
                    type_map.insert(*result, ty);
                }
            }
            Inst::Nop => {}
            // ── Phase 4: Array instructions ──────────────────────────────────
            Inst::ArrayAlloc { result, elem_ty, count } => {
                let count_val = Self::get_value(*count, value_map, builder);
                let stride = builder.ins().iconst(types::I64, elem_ty.byte_size() as i64);
                if let Some(&func_id) = func_ids.get("slang_array_alloc") {
                    let func_ref = module.declare_func_in_func(func_id, builder.func);
                    let call = builder.ins().call(func_ref, &[count_val, stride]);
                    let results = builder.inst_results(call);
                    let v = if results.is_empty() {
                        builder.ins().iconst(pointer_type, 0)
                    } else {
                        results[0]
                    };
                    value_map.insert(*result, v);
                } else {
                    value_map.insert(*result, builder.ins().iconst(pointer_type, 0));
                }
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::ArrayGet { result, array, index, elem_ty } => {
                let arr_val = Self::get_value(*array, value_map, builder);
                let idx_val = Self::get_value(*index, value_map, builder);
                let callee = match elem_ty {
                    IrType::F64 | IrType::F32 => "slang_array_get_f64",
                    _ => "slang_array_get_i64",
                };
                if let Some(&func_id) = func_ids.get(callee) {
                    let func_ref = module.declare_func_in_func(func_id, builder.func);
                    let call = builder.ins().call(func_ref, &[arr_val, idx_val]);
                    let results = builder.inst_results(call);
                    let v = if results.is_empty() {
                        builder.ins().iconst(types::I64, 0)
                    } else {
                        results[0]
                    };
                    value_map.insert(*result, v);
                } else {
                    value_map.insert(*result, builder.ins().iconst(types::I64, 0));
                }
                type_map.insert(*result, elem_ty.clone());
            }
            Inst::ArraySet { array, index, value, elem_ty } => {
                let arr_val = Self::get_value(*array, value_map, builder);
                let idx_val = Self::get_value(*index, value_map, builder);
                let v = Self::get_value(*value, value_map, builder);
                let callee = match elem_ty {
                    IrType::F64 | IrType::F32 => "slang_array_set_f64",
                    _ => "slang_array_set_i64",
                };
                if let Some(&func_id) = func_ids.get(callee) {
                    let func_ref = module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[arr_val, idx_val, v]);
                }
            }
            Inst::ArrayLen { result, array } => {
                let arr_val = Self::get_value(*array, value_map, builder);
                if let Some(&func_id) = func_ids.get("slang_array_len") {
                    let func_ref = module.declare_func_in_func(func_id, builder.func);
                    let call = builder.ins().call(func_ref, &[arr_val]);
                    let results = builder.inst_results(call);
                    let v = if results.is_empty() {
                        builder.ins().iconst(types::I64, 0)
                    } else {
                        results[0]
                    };
                    value_map.insert(*result, v);
                } else {
                    value_map.insert(*result, builder.ins().iconst(types::I64, 0));
                }
                type_map.insert(*result, IrType::I64);
            }
            // ── Phase 5: Closure scaffolding ────────────────────────────────────
            Inst::ClosureAlloc { result, .. } => {
                // Returns null ptr until Phase 5 capture analysis is complete.
                value_map.insert(*result, builder.ins().iconst(pointer_type, 0));
                type_map.insert(*result, IrType::Ptr);
            }
            // ── Phase 6: Struct scaffolding ────────────────────────────────────
            Inst::StructAlloc { result, fields, .. } => {
                // Allocate a flat record: [n_fields * 8] bytes, fill fields inline.
                let n = fields.len();
                if n == 0 {
                    value_map.insert(*result, builder.ins().iconst(pointer_type, 0));
                } else {
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot, (n as u32) * 8, 0,
                    ));
                    let base = builder.ins().stack_addr(pointer_type, slot, 0);
                    for (i, fv) in fields.iter().enumerate() {
                        let fval = Self::get_value(*fv, value_map, builder);
                        builder.ins().store(MemFlags::new(), fval, base, (i as i32) * 8);
                    }
                    value_map.insert(*result, base);
                }
                type_map.insert(*result, IrType::Ptr);
            }
            Inst::FieldGet { result, object, field_index, ty } => {
                let obj_val = Self::get_value(*object, value_map, builder);
                let offset = (*field_index as i32) * 8;
                let cl_ty = Self::ir_type_to_cl(ty, pointer_type);
                let val = builder.ins().load(cl_ty, MemFlags::new(), obj_val, offset);
                value_map.insert(*result, val);
                type_map.insert(*result, ty.clone());
            }
            Inst::FieldSet { object, field_index, value } => {
                let obj_val = Self::get_value(*object, value_map, builder);
                let val = Self::get_value(*value, value_map, builder);
                let offset = (*field_index as i32) * 8;
                builder.ins().store(MemFlags::new(), val, obj_val, offset);
            }
        }
        Ok(())
    }

    fn get_value(
        val: ir::Value,
        value_map: &HashMap<ir::Value, cranelift::prelude::Value>,
        builder: &mut FunctionBuilder,
    ) -> cranelift::prelude::Value {
        value_map
            .get(&val)
            .copied()
            .unwrap_or_else(|| builder.ins().iconst(types::I64, 0))
    }

    /// Execute the "main" function and return its i64 result.
    pub fn run_main(&self) -> CodegenResult<i64> {
        let main_id = self.func_ids.get("main").ok_or_else(|| CodegenError {
            message: "no 'main' function found".into(),
        })?;

        let code_ptr = self.module.get_finalized_function(*main_id);
        let main_fn: fn() -> i64 = unsafe { std::mem::transmute(code_ptr) };
        Ok(main_fn())
    }

    /// Execute a named function with no args, returning i64.
    pub fn run_function(&self, name: &str) -> CodegenResult<i64> {
        let func_id = self.func_ids.get(name).ok_or_else(|| CodegenError {
            message: format!("function '{}' not found", name),
        })?;

        let code_ptr = self.module.get_finalized_function(*func_id);
        let func: fn() -> i64 = unsafe { std::mem::transmute(code_ptr) };
        Ok(func())
    }
}

// ─── Public API ─────────────────────────────────────────────────────────
/// Compile source code and JIT-execute the main function.
pub fn compile_and_run(source: &str) -> Result<i64, String> {
    // Parse
    let (program, parse_errors) = crate::parser::parse(source);
    if !parse_errors.is_empty() {
        return Err(format!(
            "Parse errors:\n{}",
            parse_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
        ));
    }

    // Type check
    let type_errors = crate::types::TypeChecker::new().check(&program);
    if !type_errors.is_empty() {
        // Warn but don't fail — Phase 0 type checking is permissive
        eprintln!(
            "Type warnings:\n{}",
            type_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
        );
    }

    // Lower to IR
    let ir_module = crate::ir::IrBuilder::new().build(&program);

    // Compile via Cranelift
    let mut jit = JitCompiler::new().map_err(|e| e.to_string())?;
    jit.compile(&ir_module).map_err(|e| e.to_string())?;

    // Run main
    jit.run_main().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_constant() {
        let result = compile_and_run("fn main() -> i64 { 42 }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_addition() {
        let result = compile_and_run("fn main() -> i64 { 20 + 22 }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_arithmetic() {
        let result = compile_and_run("fn main() -> i64 { 10 * 4 + 2 }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_subtraction() {
        let result = compile_and_run("fn main() -> i64 { 50 - 8 }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_negation() {
        let result = compile_and_run("fn main() -> i64 { 0 - 42 }");
        assert_eq!(result.unwrap(), -42);
    }

    #[test]
    fn test_jit_let_binding() {
        let result = compile_and_run("fn main() -> i64 { let x: i64 = 40; let y: i64 = 2; x + y }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_function_call() {
        let result = compile_and_run("fn double(x: i64) -> i64 { x * 2 } fn main() -> i64 { double(21) }");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_while_loop_sum() {
        let result = compile_and_run(
            "fn main() -> i64 { let mut sum: i64 = 0; let mut i: i64 = 1; while i <= 10 { sum = sum + i; i = i + 1; } sum }"
        );
        assert_eq!(result.unwrap(), 55);
    }

    #[test]
    fn test_jit_nested_while() {
        let result = compile_and_run(
            "fn main() -> i64 { let mut total: i64 = 0; let mut i: i64 = 1; while i <= 5 { let mut j: i64 = 1; while j <= i { total = total + 1; j = j + 1; } i = i + 1; } total }"
        );
        assert_eq!(result.unwrap(), 15);
    }

    #[test]
    fn test_jit_mutable_variable() {
        let result = compile_and_run(
            "fn main() -> i64 { let mut x: i64 = 0; x = 42; x }"
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_compound_assign() {
        let result = compile_and_run(
            "fn main() -> i64 { let mut x: i64 = 10; x += 5; x *= 2; x }"
        );
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn test_jit_for_range_sum() {
        // 0+1+2+3+4 = 10
        let result = compile_and_run(
            "fn main() -> i64 { let mut s: i64 = 0; for i in 0..5 { s = s + i } s }"
        );
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_jit_for_range_count() {
        // count iterations from 3 to 7 = 4
        let result = compile_and_run(
            "fn main() -> i64 { let mut s: i64 = 0; for i in 3..7 { s = s + 1 } s }"
        );
        assert_eq!(result.unwrap(), 4);
    }

    #[test]
    fn test_jit_for_range_empty() {
        // 5..5 is empty, body never runs
        let result = compile_and_run(
            "fn main() -> i64 { let mut s: i64 = 99; for i in 5..5 { s = 0 } s }"
        );
        assert_eq!(result.unwrap(), 99);
    }

    #[test]
    fn test_jit_match_first_arm() {
        let result = compile_and_run(
            "fn main() -> i64 { match 1 { 1 => 10, 2 => 20, _ => 0, } }"
        );
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_jit_match_second_arm() {
        let result = compile_and_run(
            "fn main() -> i64 { match 2 { 1 => 10, 2 => 20, _ => 0, } }"
        );
        assert_eq!(result.unwrap(), 20);
    }

    #[test]
    fn test_jit_match_wildcard() {
        let result = compile_and_run(
            "fn main() -> i64 { match 99 { 1 => 10, 2 => 20, _ => 42, } }"
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_match_with_variable() {
        let result = compile_and_run(
            "fn main() -> i64 { let x: i64 = 3; match x { 1 => 10, 2 => 20, 3 => 30, _ => 0, } }"
        );
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn test_jit_pipe_single() {
        let result = compile_and_run(
            "fn double(x: i64) -> i64 { x * 2 } fn main() -> i64 { 21 |> double }"
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_jit_pipe_chain() {
        let result = compile_and_run(
            "fn add1(x: i64) -> i64 { x + 1 } fn double(x: i64) -> i64 { x * 2 } fn main() -> i64 { 5 |> add1 |> double }"
        );
        assert_eq!(result.unwrap(), 12);
    }
}
