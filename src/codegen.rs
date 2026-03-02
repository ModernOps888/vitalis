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

// ─── Module runtime ─────────────────────────────────────────────────────
/// Placeholder: returns 1 (module is always "loaded" for now).
extern "C" fn slang_module_loaded(_name: *const i8) -> i8 { 1 }

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

// ── v18: Collection Methods ────────────────────────────────────────────

/// Push element to array → returns new array pointer (may reallocate)
extern "C" fn slang_array_push(data_ptr: *mut u8, value: i64) -> *mut u8 {
    if data_ptr.is_null() { return std::ptr::null_mut(); }
    let old_len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let new_len = old_len + 1;
    let stride = 8usize; // i64 elements
    let new_alloc_size = 8 + new_len * stride;
    let layout = unsafe { std::alloc::Layout::from_size_align_unchecked(new_alloc_size, 8) };
    let raw = unsafe { std::alloc::alloc(layout) };
    if raw.is_null() { return data_ptr; }
    let data = unsafe { raw.add(8) };
    unsafe { *(raw as *mut i64) = new_len as i64; }
    // Copy old data
    unsafe { std::ptr::copy_nonoverlapping(data_ptr, data, old_len * stride); }
    // Write new element
    unsafe { *(data as *mut i64).add(old_len) = value; }
    data
}

/// Pop last element from array → returns the element
extern "C" fn slang_array_pop(data_ptr: *mut u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { &mut *(data_ptr.sub(8) as *mut i64) };
    if *len <= 0 { return 0; }
    *len -= 1;
    unsafe { *(data_ptr as *const i64).add(*len as usize) }
}

/// Check if array contains value
extern "C" fn slang_array_contains(data_ptr: *const u8, value: i64) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } == value {
            return 1;
        }
    }
    0
}

/// Reverse array in-place
extern "C" fn slang_array_reverse(data_ptr: *mut u8) -> *mut u8 {
    if data_ptr.is_null() { return data_ptr; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let arr = data_ptr as *mut i64;
    for i in 0..len / 2 {
        unsafe {
            let a = *arr.add(i);
            let b = *arr.add(len - 1 - i);
            *arr.add(i) = b;
            *arr.add(len - 1 - i) = a;
        }
    }
    data_ptr
}

/// Sort array in-place (ascending)
extern "C" fn slang_array_sort(data_ptr: *mut u8) -> *mut u8 {
    if data_ptr.is_null() { return data_ptr; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let slice = unsafe { std::slice::from_raw_parts_mut(data_ptr as *mut i64, len) };
    slice.sort();
    data_ptr
}

/// Join array elements as string with delimiter
extern "C" fn slang_array_join(data_ptr: *const u8, delim: *const i8) -> *const i8 {
    if data_ptr.is_null() { return intern_cstr("") as *const i8; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let d = if delim.is_null() { "," } else {
        unsafe { std::ffi::CStr::from_ptr(delim).to_str().unwrap_or(",") }
    };
    let parts: Vec<String> = (0..len)
        .map(|i| unsafe { *(data_ptr as *const i64).add(i) }.to_string())
        .collect();
    intern_cstr(&parts.join(d)) as *const i8
}

/// Slice array → new array [start..end)
extern "C" fn slang_array_slice(data_ptr: *const u8, start: i64, end: i64) -> *mut u8 {
    if data_ptr.is_null() { return std::ptr::null_mut(); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let s = (start as usize).min(len);
    let e = (end as usize).min(len);
    if s >= e {
        return slang_array_alloc(0, 8);
    }
    let new_len = e - s;
    let new_ptr = slang_array_alloc(new_len as i64, 8);
    for i in 0..new_len {
        unsafe { *(new_ptr as *mut i64).add(i) = *(data_ptr as *const i64).add(s + i); }
    }
    new_ptr
}

/// Find index of value in array (-1 if not found)
extern "C" fn slang_array_find(data_ptr: *const u8, value: i64) -> i64 {
    if data_ptr.is_null() { return -1; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } == value {
            return i as i64;
        }
    }
    -1
}

// ── Iterator / Functional array operations ────────────────────────────

/// Create array [start..end)
extern "C" fn slang_array_range(start: i64, end: i64) -> *mut u8 {
    if end <= start {
        return slang_array_alloc(0, 8);
    }
    let count = (end - start) as usize;
    let ptr = slang_array_alloc(count as i64, 8);
    if ptr.is_null() { return ptr; }
    for i in 0..count {
        unsafe { *(ptr as *mut i64).add(i) = start + i as i64; }
    }
    ptr
}

/// Sum all i64 elements
extern "C" fn slang_array_sum(data_ptr: *const u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let mut total: i64 = 0;
    for i in 0..len {
        total += unsafe { *(data_ptr as *const i64).add(i) };
    }
    total
}

/// Minimum element (returns i64::MAX for empty)
extern "C" fn slang_array_min(data_ptr: *const u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    if len == 0 { return 0; }
    let mut m = unsafe { *(data_ptr as *const i64) };
    for i in 1..len {
        let v = unsafe { *(data_ptr as *const i64).add(i) };
        if v < m { m = v; }
    }
    m
}

/// Maximum element (returns i64::MIN for empty)
extern "C" fn slang_array_max(data_ptr: *const u8) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    if len == 0 { return 0; }
    let mut m = unsafe { *(data_ptr as *const i64) };
    for i in 1..len {
        let v = unsafe { *(data_ptr as *const i64).add(i) };
        if v > m { m = v; }
    }
    m
}

/// Check if any element equals val
extern "C" fn slang_array_any(data_ptr: *const u8, value: i64) -> i8 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } == value {
            return 1;
        }
    }
    0
}

/// Check if all elements > 0
extern "C" fn slang_array_all_positive(data_ptr: *const u8) -> i8 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    if len == 0 { return 1; }
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } <= 0 {
            return 0;
        }
    }
    1
}

/// Count occurrences of val
extern "C" fn slang_array_count(data_ptr: *const u8, value: i64) -> i64 {
    if data_ptr.is_null() { return 0; }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let mut c: i64 = 0;
    for i in 0..len {
        if unsafe { *(data_ptr as *const i64).add(i) } == value {
            c += 1;
        }
    }
    c
}

/// Flatten nested arrays (array of array pointers) into one flat array
extern "C" fn slang_array_flatten(data_ptr: *const u8) -> *mut u8 {
    if data_ptr.is_null() { return slang_array_alloc(0, 8); }
    let outer_len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    // First pass: count total elements
    let mut total = 0usize;
    for i in 0..outer_len {
        let inner_ptr = unsafe { *(data_ptr as *const *const u8).add(i) };
        if !inner_ptr.is_null() {
            let inner_len = unsafe { *(inner_ptr.sub(8) as *const i64) } as usize;
            total += inner_len;
        }
    }
    let result = slang_array_alloc(total as i64, 8);
    if result.is_null() { return result; }
    let mut idx = 0usize;
    for i in 0..outer_len {
        let inner_ptr = unsafe { *(data_ptr as *const *const u8).add(i) };
        if !inner_ptr.is_null() {
            let inner_len = unsafe { *(inner_ptr.sub(8) as *const i64) } as usize;
            for j in 0..inner_len {
                let val = unsafe { *(inner_ptr as *const i64).add(j) };
                unsafe { *(result as *mut i64).add(idx) = val; }
                idx += 1;
            }
        }
    }
    result
}

/// Zip two arrays: interleave [a0, b0, a1, b1, ...]
extern "C" fn slang_array_zip(a: *const u8, b: *const u8) -> *mut u8 {
    let a_len = if a.is_null() { 0 } else { (unsafe { *(a.sub(8) as *const i64) }) as usize };
    let b_len = if b.is_null() { 0 } else { (unsafe { *(b.sub(8) as *const i64) }) as usize };
    let min_len = a_len.min(b_len);
    let result = slang_array_alloc((min_len * 2) as i64, 8);
    if result.is_null() { return result; }
    for i in 0..min_len {
        let va = unsafe { *(a as *const i64).add(i) };
        let vb = unsafe { *(b as *const i64).add(i) };
        unsafe { *(result as *mut i64).add(i * 2) = va; }
        unsafe { *(result as *mut i64).add(i * 2 + 1) = vb; }
    }
    result
}

/// Enumerate: [idx0, val0, idx1, val1, ...]
extern "C" fn slang_array_enumerate(data_ptr: *const u8) -> *mut u8 {
    if data_ptr.is_null() { return slang_array_alloc(0, 8); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let result = slang_array_alloc((len * 2) as i64, 8);
    if result.is_null() { return result; }
    for i in 0..len {
        let val = unsafe { *(data_ptr as *const i64).add(i) };
        unsafe { *(result as *mut i64).add(i * 2) = i as i64; }
        unsafe { *(result as *mut i64).add(i * 2 + 1) = val; }
    }
    result
}

/// Take first n elements
extern "C" fn slang_array_take(data_ptr: *const u8, n: i64) -> *mut u8 {
    if data_ptr.is_null() || n <= 0 { return slang_array_alloc(0, 8); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let take = (n as usize).min(len);
    let result = slang_array_alloc(take as i64, 8);
    if result.is_null() { return result; }
    unsafe { std::ptr::copy_nonoverlapping(data_ptr, result, take * 8); }
    result
}

/// Drop first n elements
extern "C" fn slang_array_drop(data_ptr: *const u8, n: i64) -> *mut u8 {
    if data_ptr.is_null() { return slang_array_alloc(0, 8); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let skip = (n.max(0) as usize).min(len);
    let new_len = len - skip;
    if new_len == 0 { return slang_array_alloc(0, 8); }
    let result = slang_array_alloc(new_len as i64, 8);
    if result.is_null() { return result; }
    unsafe {
        std::ptr::copy_nonoverlapping(
            (data_ptr as *const i64).add(skip) as *const u8,
            result,
            new_len * 8,
        );
    }
    result
}

/// Remove duplicates (preserves first occurrence order)
extern "C" fn slang_array_unique(data_ptr: *const u8) -> *mut u8 {
    if data_ptr.is_null() { return slang_array_alloc(0, 8); }
    let len = unsafe { *(data_ptr.sub(8) as *const i64) } as usize;
    let mut seen = Vec::<i64>::with_capacity(len);
    for i in 0..len {
        let v = unsafe { *(data_ptr as *const i64).add(i) };
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    let result = slang_array_alloc(seen.len() as i64, 8);
    if result.is_null() { return result; }
    for (i, &v) in seen.iter().enumerate() {
        unsafe { *(result as *mut i64).add(i) = v; }
    }
    result
}

/// v18: error_message alias (returns interned string)
extern "C" fn slang_error_message() -> *const i8 {
    slang_error_msg()
}

// ─── Regex operations runtime ─────────────────────────────────────────────

/// Full-match: returns 1 if the entire text matches the pattern, 0 otherwise.
extern "C" fn slang_regex_match(pattern: *const i8, text: *const i8) -> i8 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    // Anchor the pattern for full-match semantics
    let anchored = format!("^(?:{})$", pat);
    match regex::Regex::new(&anchored) {
        Ok(re) => if re.is_match(&txt) { 1 } else { 0 },
        Err(_) => 0,
    }
}

/// Partial match: returns 1 if pattern is found anywhere in text, 0 otherwise.
extern "C" fn slang_regex_is_match(pattern: *const i8, text: *const i8) -> i8 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => if re.is_match(&txt) { 1 } else { 0 },
        Err(_) => 0,
    }
}

/// Find first match substring; returns empty string if no match.
extern "C" fn slang_regex_find(pattern: *const i8, text: *const i8) -> *const i8 {
    if pattern.is_null() || text.is_null() { return intern_cstr("") as *const i8; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => match re.find(&txt) {
            Some(m) => intern_cstr(m.as_str()) as *const i8,
            None => intern_cstr("") as *const i8,
        },
        Err(_) => intern_cstr("") as *const i8,
    }
}

/// Replace all occurrences of pattern in text with replacement.
extern "C" fn slang_regex_replace(pattern: *const i8, text: *const i8, replacement: *const i8) -> *const i8 {
    if pattern.is_null() || text.is_null() || replacement.is_null() {
        return intern_cstr("") as *const i8;
    }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    let rep = unsafe { std::ffi::CStr::from_ptr(replacement).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => {
            let result = re.replace_all(&txt, rep.as_ref());
            intern_cstr(&result) as *const i8
        }
        Err(_) => intern_cstr(&txt) as *const i8,
    }
}

/// Count of segments after splitting text by pattern.
extern "C" fn slang_regex_split_count(pattern: *const i8, text: *const i8) -> i64 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => re.split(&txt).count() as i64,
        Err(_) => 1, // no split, whole string is one segment
    }
}

/// Get the nth segment after splitting text by pattern.
extern "C" fn slang_regex_split_get(pattern: *const i8, text: *const i8, idx: i64) -> *const i8 {
    if pattern.is_null() || text.is_null() { return intern_cstr("") as *const i8; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => {
            let parts: Vec<&str> = re.split(&txt).collect();
            if idx < 0 || (idx as usize) >= parts.len() {
                intern_cstr("") as *const i8
            } else {
                intern_cstr(parts[idx as usize]) as *const i8
            }
        }
        Err(_) => intern_cstr("") as *const i8,
    }
}

/// Count all non-overlapping matches of pattern in text.
extern "C" fn slang_regex_find_all_count(pattern: *const i8, text: *const i8) -> i64 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => re.find_iter(&txt).count() as i64,
        Err(_) => 0,
    }
}

/// Get the nth match of pattern in text.
extern "C" fn slang_regex_find_all_get(pattern: *const i8, text: *const i8, idx: i64) -> *const i8 {
    if pattern.is_null() || text.is_null() { return intern_cstr("") as *const i8; }
    let pat = unsafe { std::ffi::CStr::from_ptr(pattern).to_string_lossy() };
    let txt = unsafe { std::ffi::CStr::from_ptr(text).to_string_lossy() };
    match regex::Regex::new(&pat) {
        Ok(re) => {
            let matches: Vec<regex::Match> = re.find_iter(&txt).collect();
            if idx < 0 || (idx as usize) >= matches.len() {
                intern_cstr("") as *const i8
            } else {
                intern_cstr(matches[idx as usize].as_str()) as *const i8
            }
        }
        Err(_) => intern_cstr("") as *const i8,
    }
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

// ── v15: String Operations ──────────────────────────────────────────────

/// Convert string to uppercase
extern "C" fn slang_str_upper(ptr: *const i8) -> *const i8 {
    if ptr.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    intern_cstr(&s.to_uppercase()) as *const i8
}

/// Convert string to lowercase
extern "C" fn slang_str_lower(ptr: *const i8) -> *const i8 {
    if ptr.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    intern_cstr(&s.to_lowercase()) as *const i8
}

/// Trim whitespace from both sides
extern "C" fn slang_str_trim(ptr: *const i8) -> *const i8 {
    if ptr.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    intern_cstr(s.trim()) as *const i8
}

/// Check if string contains substring → i8 bool
extern "C" fn slang_str_contains(haystack: *const i8, needle: *const i8) -> i8 {
    if haystack.is_null() || needle.is_null() { return 0; }
    let h = unsafe { std::ffi::CStr::from_ptr(haystack).to_string_lossy() };
    let n = unsafe { std::ffi::CStr::from_ptr(needle).to_string_lossy() };
    if h.contains(n.as_ref()) { 1 } else { 0 }
}

/// Check if string starts with prefix → i8 bool
extern "C" fn slang_str_starts_with(s: *const i8, prefix: *const i8) -> i8 {
    if s.is_null() || prefix.is_null() { return 0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy() };
    let pv = unsafe { std::ffi::CStr::from_ptr(prefix).to_string_lossy() };
    if sv.starts_with(pv.as_ref()) { 1 } else { 0 }
}

/// Check if string ends with suffix → i8 bool
extern "C" fn slang_str_ends_with(s: *const i8, suffix: *const i8) -> i8 {
    if s.is_null() || suffix.is_null() { return 0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy() };
    let sv2 = unsafe { std::ffi::CStr::from_ptr(suffix).to_string_lossy() };
    if sv.ends_with(sv2.as_ref()) { 1 } else { 0 }
}

/// Get character at index (returns single-char string, or "" if OOB)
extern "C" fn slang_str_char_at(s: *const i8, index: i64) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    if index < 0 || (index as usize) >= sv.len() { return intern_cstr("") as *const i8; }
    let ch = sv.chars().nth(index as usize).unwrap_or('\0');
    intern_cstr(&ch.to_string()) as *const i8
}

/// Substring: str_substr(s, start, len)
extern "C" fn slang_str_substr(s: *const i8, start: i64, len: i64) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let st = (start.max(0) as usize).min(sv.len());
    let end = (st + (len.max(0) as usize)).min(sv.len());
    intern_cstr(&sv[st..end]) as *const i8
}

/// Find index of substring (-1 if not found)
extern "C" fn slang_str_index_of(haystack: *const i8, needle: *const i8) -> i64 {
    if haystack.is_null() || needle.is_null() { return -1; }
    let h = unsafe { std::ffi::CStr::from_ptr(haystack).to_string_lossy().into_owned() };
    let n = unsafe { std::ffi::CStr::from_ptr(needle).to_string_lossy().into_owned() };
    h.find(&n).map(|i| i as i64).unwrap_or(-1)
}

/// Replace all occurrences of `old` with `new_s`
extern "C" fn slang_str_replace(s: *const i8, old: *const i8, new_s: *const i8) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let ov = if old.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(old).to_string_lossy().into_owned() } };
    let nv = if new_s.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(new_s).to_string_lossy().into_owned() } };
    if ov.is_empty() { return intern_cstr(&sv) as *const i8; }
    intern_cstr(&sv.replace(&ov, &nv)) as *const i8
}

/// Repeat string n times
extern "C" fn slang_str_repeat(s: *const i8, n: i64) -> *const i8 {
    if s.is_null() || n <= 0 { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    intern_cstr(&sv.repeat(n.min(100_000) as usize)) as *const i8
}

/// Reverse a string
extern "C" fn slang_str_reverse(s: *const i8) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let reversed: String = sv.chars().rev().collect();
    intern_cstr(&reversed) as *const i8
}

/// Split string by delimiter, return count of parts
extern "C" fn slang_str_split_count(s: *const i8, delim: *const i8) -> i64 {
    if s.is_null() { return 0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let dv = if delim.is_null() { " ".to_string() } else { unsafe { std::ffi::CStr::from_ptr(delim).to_string_lossy().into_owned() } };
    if dv.is_empty() { return sv.len() as i64; }
    sv.split(&dv).count() as i64
}

/// Get the n-th part after splitting by delimiter
extern "C" fn slang_str_split_get(s: *const i8, delim: *const i8, index: i64) -> *const i8 {
    if s.is_null() { return intern_cstr("") as *const i8; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    let dv = if delim.is_null() { " ".to_string() } else { unsafe { std::ffi::CStr::from_ptr(delim).to_string_lossy().into_owned() } };
    if dv.is_empty() { return intern_cstr("") as *const i8; }
    let parts: Vec<&str> = sv.split(&dv).collect();
    if index < 0 || (index as usize) >= parts.len() { return intern_cstr("") as *const i8; }
    intern_cstr(parts[index as usize]) as *const i8
}

/// Join: not a stdlib call (needs arrays), but convert int to string
extern "C" fn slang_to_string_i64(val: i64) -> *const i8 {
    intern_cstr(&val.to_string()) as *const i8
}

/// Convert f64 to string
extern "C" fn slang_to_string_f64(val: f64) -> *const i8 {
    intern_cstr(&val.to_string()) as *const i8
}

/// Convert bool to string
extern "C" fn slang_to_string_bool(val: i8) -> *const i8 {
    intern_cstr(if val != 0 { "true" } else { "false" }) as *const i8
}

// ─── String formatting runtime ──────────────────────────────────────────
/// Replace first `{}` in fmt with an i64 value.
extern "C" fn slang_str_format_i64(fmt: *const i8, val: i64) -> *const i8 {
    if fmt.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(fmt).to_string_lossy().into_owned() };
    let result = s.replacen("{}", &val.to_string(), 1);
    intern_cstr(&result) as *const i8
}
/// Replace first `{}` in fmt with an f64 value.
extern "C" fn slang_str_format_f64(fmt: *const i8, val: f64) -> *const i8 {
    if fmt.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(fmt).to_string_lossy().into_owned() };
    let result = s.replacen("{}", &val.to_string(), 1);
    intern_cstr(&result) as *const i8
}
/// Replace first `{}` in fmt with a string value.
extern "C" fn slang_str_format_str(fmt: *const i8, val: *const i8) -> *const i8 {
    if fmt.is_null() { return intern_cstr("") as *const i8; }
    let s = unsafe { std::ffi::CStr::from_ptr(fmt).to_string_lossy().into_owned() };
    let v = if val.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(val).to_string_lossy().into_owned() } };
    let result = s.replacen("{}", &v, 1);
    intern_cstr(&result) as *const i8
}

/// Parse string to i64 (0 on failure)
extern "C" fn slang_parse_int(s: *const i8) -> i64 {
    if s.is_null() { return 0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    sv.trim().parse::<i64>().unwrap_or(0)
}

/// Parse string to f64 (0.0 on failure)
extern "C" fn slang_parse_float(s: *const i8) -> f64 {
    if s.is_null() { return 0.0; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    sv.trim().parse::<f64>().unwrap_or(0.0)
}

// ── v15: File I/O ───────────────────────────────────────────────────────

/// Read entire file to string. Returns "" on error.
extern "C" fn slang_file_read(path: *const i8) -> *const i8 {
    if path.is_null() { return intern_cstr("") as *const i8; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    match std::fs::read_to_string(&p) {
        Ok(content) => intern_cstr(&content) as *const i8,
        Err(_) => intern_cstr("") as *const i8,
    }
}

/// Write string to file. Returns 1 on success, 0 on failure.
extern "C" fn slang_file_write(path: *const i8, content: *const i8) -> i8 {
    if path.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    let c = if content.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(content).to_string_lossy().into_owned() } };
    match std::fs::write(&p, &c) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// Append string to file. Returns 1 on success, 0 on failure.
extern "C" fn slang_file_append(path: *const i8, content: *const i8) -> i8 {
    if path.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    let c = if content.is_null() { String::new() } else { unsafe { std::ffi::CStr::from_ptr(content).to_string_lossy().into_owned() } };
    use std::io::Write;
    match std::fs::OpenOptions::new().append(true).create(true).open(&p) {
        Ok(mut f) => if f.write_all(c.as_bytes()).is_ok() { 1 } else { 0 },
        Err(_) => 0,
    }
}

/// Check if file exists → i8 bool
extern "C" fn slang_file_exists(path: *const i8) -> i8 {
    if path.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    if std::path::Path::new(&p).exists() { 1 } else { 0 }
}

/// Delete a file. Returns 1 on success, 0 on failure.
extern "C" fn slang_file_delete(path: *const i8) -> i8 {
    if path.is_null() { return 0; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    if std::fs::remove_file(&p).is_ok() { 1 } else { 0 }
}

/// Get file size in bytes (-1 on error)
extern "C" fn slang_file_size(path: *const i8) -> i64 {
    if path.is_null() { return -1; }
    let p = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    match std::fs::metadata(&p) {
        Ok(m) => m.len() as i64,
        Err(_) => -1,
    }
}

// ── v15: HashMap Runtime ────────────────────────────────────────────────
// Maps are stored as opaque pointers to a Box<HashMap<String, i64>>
// arena-managed to prevent drops.

use std::collections::BTreeMap;

static VITALIS_MAP_ARENA: Mutex<Vec<Box<BTreeMap<String, i64>>>> = Mutex::new(Vec::new());
static VITALIS_FMAP_ARENA: Mutex<Vec<Box<BTreeMap<String, f64>>>> = Mutex::new(Vec::new());

/// Create a new empty map, return opaque handle (i64)
extern "C" fn slang_map_new() -> i64 {
    let map: Box<BTreeMap<String, i64>> = Box::new(BTreeMap::new());
    let ptr = Box::into_raw(map);
    VITALIS_MAP_ARENA.lock().unwrap().push(unsafe { Box::from_raw(ptr) });
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    (arena.len() - 1) as i64
}

/// Set key-value in map
extern "C" fn slang_map_set(handle: i64, key: *const i8, value: i64) {
    if key.is_null() { return; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    let mut arena = VITALIS_MAP_ARENA.lock().unwrap();
    if let Some(map) = arena.get_mut(handle as usize) {
        map.insert(k, value);
    }
}

/// Get value from map (returns 0 if key not found)
extern "C" fn slang_map_get(handle: i64, key: *const i8) -> i64 {
    if key.is_null() { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    arena.get(handle as usize).and_then(|m| m.get(&k).copied()).unwrap_or(0)
}

/// Check if key exists in map → bool
extern "C" fn slang_map_has(handle: i64, key: *const i8) -> i8 {
    if key.is_null() { return 0; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    if arena.get(handle as usize).map(|m| m.contains_key(&k)).unwrap_or(false) { 1 } else { 0 }
}

/// Remove key from map
extern "C" fn slang_map_remove(handle: i64, key: *const i8) {
    if key.is_null() { return; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    let mut arena = VITALIS_MAP_ARENA.lock().unwrap();
    if let Some(map) = arena.get_mut(handle as usize) {
        map.remove(&k);
    }
}

/// Count of entries in map
extern "C" fn slang_map_len(handle: i64) -> i64 {
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    arena.get(handle as usize).map(|m| m.len() as i64).unwrap_or(0)
}

/// Get all keys as a joined string (delimiter = ",")
extern "C" fn slang_map_keys(handle: i64) -> *const i8 {
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    match arena.get(handle as usize) {
        Some(map) => {
            let keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
            intern_cstr(&keys.join(",")) as *const i8
        }
        None => intern_cstr("") as *const i8,
    }
}

// ── v16: HashSet Runtime ────────────────────────────────────────────────
// Sets are stored in an arena indexed by handle, similar to maps.

use std::collections::HashSet;

static VITALIS_SET_ARENA: Mutex<Vec<Box<HashSet<i64>>>> = Mutex::new(Vec::new());

/// Create a new empty set, return opaque handle (i64)
extern "C" fn slang_set_new() -> i64 {
    let set: Box<HashSet<i64>> = Box::new(HashSet::new());
    let ptr = Box::into_raw(set);
    VITALIS_SET_ARENA.lock().unwrap().push(unsafe { Box::from_raw(ptr) });
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    (arena.len() - 1) as i64
}

/// Add value to set
extern "C" fn slang_set_add(handle: i64, value: i64) {
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    if let Some(set) = arena.get_mut(handle as usize) {
        set.insert(value);
    }
}

/// Check if value exists in set → 0/1
extern "C" fn slang_set_has(handle: i64, value: i64) -> i8 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    if arena.get(handle as usize).map(|s| s.contains(&value)).unwrap_or(false) { 1 } else { 0 }
}

/// Remove value from set
extern "C" fn slang_set_remove(handle: i64, value: i64) {
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    if let Some(set) = arena.get_mut(handle as usize) {
        set.remove(&value);
    }
}

/// Count of elements in set
extern "C" fn slang_set_len(handle: i64) -> i64 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.get(handle as usize).map(|s| s.len() as i64).unwrap_or(0)
}

/// Union of two sets → new handle
extern "C" fn slang_set_union(h1: i64, h2: i64) -> i64 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    let s1 = arena.get(h1 as usize);
    let s2 = arena.get(h2 as usize);
    let result: HashSet<i64> = match (s1, s2) {
        (Some(a), Some(b)) => a.union(b).copied().collect(),
        (Some(a), None)    => a.as_ref().clone(),
        (None, Some(b))    => b.as_ref().clone(),
        (None, None)       => HashSet::new(),
    };
    drop(arena);
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.push(Box::new(result));
    (arena.len() - 1) as i64
}

/// Intersection of two sets → new handle
extern "C" fn slang_set_intersect(h1: i64, h2: i64) -> i64 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    let s1 = arena.get(h1 as usize);
    let s2 = arena.get(h2 as usize);
    let result: HashSet<i64> = match (s1, s2) {
        (Some(a), Some(b)) => a.intersection(b).copied().collect(),
        _                  => HashSet::new(),
    };
    drop(arena);
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.push(Box::new(result));
    (arena.len() - 1) as i64
}

/// Difference of two sets (h1 - h2) → new handle
extern "C" fn slang_set_diff(h1: i64, h2: i64) -> i64 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    let s1 = arena.get(h1 as usize);
    let s2 = arena.get(h2 as usize);
    let result: HashSet<i64> = match (s1, s2) {
        (Some(a), Some(b)) => a.difference(b).copied().collect(),
        (Some(a), None)    => a.as_ref().clone(),
        _                  => HashSet::new(),
    };
    drop(arena);
    let mut arena = VITALIS_SET_ARENA.lock().unwrap();
    arena.push(Box::new(result));
    (arena.len() - 1) as i64
}

/// Convert set to string representation e.g. "{1,2,3}"
extern "C" fn slang_set_to_array(handle: i64) -> *const i8 {
    let arena = VITALIS_SET_ARENA.lock().unwrap();
    match arena.get(handle as usize) {
        Some(set) => {
            let mut vals: Vec<i64> = set.iter().copied().collect();
            vals.sort();
            let s = format!("{{{}}}", vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","));
            intern_cstr(&s) as *const i8
        }
        None => intern_cstr("{}") as *const i8,
    }
}

// ── v18: Tuple Runtime ──────────────────────────────────────────────────
// Tuples are immutable, heap-allocated arrays of i64 values, arena-managed.

static VITALIS_TUPLE_ARENA: Mutex<Vec<Box<Vec<i64>>>> = Mutex::new(Vec::new());

/// Create a 2-tuple, return opaque handle (i64)
extern "C" fn slang_tuple_new2(a: i64, b: i64) -> i64 {
    let mut arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.push(Box::new(vec![a, b]));
    (arena.len() - 1) as i64
}

/// Create a 3-tuple, return opaque handle (i64)
extern "C" fn slang_tuple_new3(a: i64, b: i64, c: i64) -> i64 {
    let mut arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.push(Box::new(vec![a, b, c]));
    (arena.len() - 1) as i64
}

/// Create a 4-tuple, return opaque handle (i64)
extern "C" fn slang_tuple_new4(a: i64, b: i64, c: i64, d: i64) -> i64 {
    let mut arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.push(Box::new(vec![a, b, c, d]));
    (arena.len() - 1) as i64
}

/// Get element at index from tuple (bounds-checked, returns 0 on OOB)
extern "C" fn slang_tuple_get(handle: i64, idx: i64) -> i64 {
    let arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.get(handle as usize)
        .and_then(|t| t.get(idx as usize).copied())
        .unwrap_or(0)
}

/// Get length of tuple
extern "C" fn slang_tuple_len(handle: i64) -> i64 {
    let arena = VITALIS_TUPLE_ARENA.lock().unwrap();
    arena.get(handle as usize).map(|t| t.len() as i64).unwrap_or(0)
}

// ── v15: Error Handling Runtime ─────────────────────────────────────────
// Simple error flag mechanism: functions can set an error, callers can check/clear it.

static VITALIS_ERROR_FLAG: AtomicU64 = AtomicU64::new(0);
static VITALIS_ERROR_MSG: Mutex<String> = Mutex::new(String::new());

/// Set error with code
extern "C" fn slang_error_set(code: i64, msg: *const i8) {
    VITALIS_ERROR_FLAG.store(code as u64, Ordering::SeqCst);
    if !msg.is_null() {
        let m = unsafe { std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned() };
        *VITALIS_ERROR_MSG.lock().unwrap() = m;
    }
}

/// Check if error is set → error code (0 = no error)
extern "C" fn slang_error_check() -> i64 {
    VITALIS_ERROR_FLAG.load(Ordering::SeqCst) as i64
}

/// Get error message
extern "C" fn slang_error_msg() -> *const i8 {
    let msg = VITALIS_ERROR_MSG.lock().unwrap().clone();
    intern_cstr(&msg) as *const i8
}

/// Clear error
extern "C" fn slang_error_clear() {
    VITALIS_ERROR_FLAG.store(0, Ordering::SeqCst);
    *VITALIS_ERROR_MSG.lock().unwrap() = String::new();
}

// ── v15: Environment & System ───────────────────────────────────────────

/// Get environment variable (returns "" if not set)
extern "C" fn slang_env_get(key: *const i8) -> *const i8 {
    if key.is_null() { return intern_cstr("") as *const i8; }
    let k = unsafe { std::ffi::CStr::from_ptr(key).to_string_lossy().into_owned() };
    match std::env::var(&k) {
        Ok(v) => intern_cstr(&v) as *const i8,
        Err(_) => intern_cstr("") as *const i8,
    }
}

/// Sleep for N milliseconds
extern "C" fn slang_sleep_ms(ms: i64) {
    if ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

/// Print to stderr
extern "C" fn slang_eprint(ptr: *const i8) {
    if ptr.is_null() { return; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
    eprint!("{}", s.to_string_lossy());
}

/// Print to stderr with newline
extern "C" fn slang_eprintln(ptr: *const i8) {
    if ptr.is_null() { return; }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
    eprintln!("{}", s.to_string_lossy());
}

/// Get process ID
extern "C" fn slang_pid() -> i64 {
    std::process::id() as i64
}

/// Format string: sprintf-lite. Replaces {} with i64 value.
extern "C" fn slang_format_int(template: *const i8, val: i64) -> *const i8 {
    if template.is_null() { return intern_cstr("") as *const i8; }
    let t = unsafe { std::ffi::CStr::from_ptr(template).to_string_lossy().into_owned() };
    intern_cstr(&t.replacen("{}", &val.to_string(), 1)) as *const i8
}

/// Format string: replaces {} with f64 value.
extern "C" fn slang_format_float(template: *const i8, val: f64) -> *const i8 {
    if template.is_null() { return intern_cstr("") as *const i8; }
    let t = unsafe { std::ffi::CStr::from_ptr(template).to_string_lossy().into_owned() };
    intern_cstr(&t.replacen("{}", &format!("{:.6}", val), 1)) as *const i8
}

// ── v15: JSON Runtime ───────────────────────────────────────────────────
// Minimal JSON: map handles can be serialized/deserialized

/// Serialize a map handle to JSON string
extern "C" fn slang_json_encode(handle: i64) -> *const i8 {
    let arena = VITALIS_MAP_ARENA.lock().unwrap();
    match arena.get(handle as usize) {
        Some(map) => {
            let entries: Vec<String> = map.iter()
                .map(|(k, v)| format!("\"{}\":{}", k, v))
                .collect();
            let json = format!("{{{}}}", entries.join(","));
            intern_cstr(&json) as *const i8
        }
        None => intern_cstr("{}") as *const i8,
    }
}

/// Deserialize JSON string to a new map handle (basic int-valued objects only)
extern "C" fn slang_json_decode(s: *const i8) -> i64 {
    let handle = slang_map_new();
    if s.is_null() { return handle; }
    let sv = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };
    // Simple parser for {"key":123, ...}
    let sv = sv.trim();
    if !sv.starts_with('{') || !sv.ends_with('}') { return handle; }
    let inner = &sv[1..sv.len()-1];
    for pair in inner.split(',') {
        let pair = pair.trim();
        if let Some(colon) = pair.find(':') {
            let key = pair[..colon].trim().trim_matches('"');
            let val = pair[colon+1..].trim();
            if let Ok(v) = val.parse::<i64>() {
                let key_cstr = intern_cstr(key);
                slang_map_set(handle, key_cstr as *const i8, v);
            }
        }
    }
    handle
}

// ─── v18: Async stubs ────────────────────────────────────────────────────
extern "C" fn slang_spawn(task_id: i64) -> i64 { task_id }
extern "C" fn slang_task_result(task_id: i64) -> i64 { task_id }

// ─── v18: Networking functions ───────────────────────────────────────────
extern "C" fn slang_http_get(url: *const i8) -> *const i8 {
    if url.is_null() { return intern_cstr("") as *const i8; }
    let url_str = unsafe { std::ffi::CStr::from_ptr(url).to_string_lossy() };
    match ureq::get(&url_str).call() {
        Ok(resp) => {
            let body = resp.into_string().unwrap_or_default();
            intern_cstr(&body) as *const i8
        }
        Err(_) => intern_cstr("") as *const i8,
    }
}

extern "C" fn slang_http_post(url: *const i8, body: *const i8) -> *const i8 {
    if url.is_null() { return intern_cstr("") as *const i8; }
    let url_str = unsafe { std::ffi::CStr::from_ptr(url).to_string_lossy() };
    let body_str = if body.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(body).to_string_lossy().into_owned() }
    };
    match ureq::post(&url_str).send_string(&body_str) {
        Ok(resp) => {
            let response_body = resp.into_string().unwrap_or_default();
            intern_cstr(&response_body) as *const i8
        }
        Err(_) => intern_cstr("") as *const i8,
    }
}

extern "C" fn slang_http_status(url: *const i8) -> i64 {
    if url.is_null() { return 0; }
    let url_str = unsafe { std::ffi::CStr::from_ptr(url).to_string_lossy() };
    match ureq::get(&url_str).call() {
        Ok(resp) => resp.status() as i64,
        Err(ureq::Error::Status(code, _)) => code as i64,
        Err(_) => 0,
    }
}

extern "C" fn slang_tcp_connect(_host: *const i8, _port: i64) -> i64 { 1 }
extern "C" fn slang_tcp_send(_handle: i64, _data: *const i8) -> i64 { 0 }
extern "C" fn slang_tcp_close(_handle: i64) { }

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
        // ── v15: String operations ───────────────────────────────────────
        builder.symbol("slang_str_upper",        slang_str_upper        as *const u8);
        builder.symbol("slang_str_lower",        slang_str_lower        as *const u8);
        builder.symbol("slang_str_trim",         slang_str_trim         as *const u8);
        builder.symbol("slang_str_contains",     slang_str_contains     as *const u8);
        builder.symbol("slang_str_starts_with",  slang_str_starts_with  as *const u8);
        builder.symbol("slang_str_ends_with",    slang_str_ends_with    as *const u8);
        builder.symbol("slang_str_char_at",      slang_str_char_at      as *const u8);
        builder.symbol("slang_str_substr",       slang_str_substr       as *const u8);
        builder.symbol("slang_str_index_of",     slang_str_index_of     as *const u8);
        builder.symbol("slang_str_replace",      slang_str_replace      as *const u8);
        builder.symbol("slang_str_repeat",       slang_str_repeat       as *const u8);
        builder.symbol("slang_str_reverse",      slang_str_reverse      as *const u8);
        builder.symbol("slang_str_split_count",  slang_str_split_count  as *const u8);
        builder.symbol("slang_str_split_get",    slang_str_split_get    as *const u8);
        builder.symbol("slang_to_string_i64",    slang_to_string_i64    as *const u8);
        builder.symbol("slang_to_string_f64",    slang_to_string_f64    as *const u8);
        builder.symbol("slang_to_string_bool",   slang_to_string_bool   as *const u8);
        builder.symbol("slang_str_format_i64",   slang_str_format_i64   as *const u8);
        builder.symbol("slang_str_format_f64",   slang_str_format_f64   as *const u8);
        builder.symbol("slang_str_format_str",   slang_str_format_str   as *const u8);
        builder.symbol("slang_parse_int",        slang_parse_int        as *const u8);
        builder.symbol("slang_parse_float",      slang_parse_float      as *const u8);
        // ── v15: File I/O ────────────────────────────────────────────────
        builder.symbol("slang_file_read",        slang_file_read        as *const u8);
        builder.symbol("slang_file_write",       slang_file_write       as *const u8);
        builder.symbol("slang_file_append",      slang_file_append      as *const u8);
        builder.symbol("slang_file_exists",      slang_file_exists      as *const u8);
        builder.symbol("slang_file_delete",      slang_file_delete      as *const u8);
        builder.symbol("slang_file_size",        slang_file_size        as *const u8);
        // ── v15: Map operations ──────────────────────────────────────────
        builder.symbol("slang_map_new",          slang_map_new          as *const u8);
        builder.symbol("slang_map_set",          slang_map_set          as *const u8);
        builder.symbol("slang_map_get",          slang_map_get          as *const u8);
        builder.symbol("slang_map_has",          slang_map_has          as *const u8);
        builder.symbol("slang_map_remove",       slang_map_remove       as *const u8);
        builder.symbol("slang_map_len",          slang_map_len          as *const u8);
        builder.symbol("slang_map_keys",         slang_map_keys         as *const u8);
        // ── v16: Set operations ──────────────────────────────────────────
        builder.symbol("slang_set_new",          slang_set_new          as *const u8);
        builder.symbol("slang_set_add",          slang_set_add          as *const u8);
        builder.symbol("slang_set_has",          slang_set_has          as *const u8);
        builder.symbol("slang_set_remove",       slang_set_remove       as *const u8);
        builder.symbol("slang_set_len",          slang_set_len          as *const u8);
        builder.symbol("slang_set_union",        slang_set_union        as *const u8);
        builder.symbol("slang_set_intersect",    slang_set_intersect    as *const u8);
        builder.symbol("slang_set_diff",         slang_set_diff         as *const u8);
        builder.symbol("slang_set_to_array",     slang_set_to_array     as *const u8);
        // ── v18: Tuple operations ────────────────────────────────────────
        builder.symbol("slang_tuple_new2",       slang_tuple_new2       as *const u8);
        builder.symbol("slang_tuple_new3",       slang_tuple_new3       as *const u8);
        builder.symbol("slang_tuple_new4",       slang_tuple_new4       as *const u8);
        builder.symbol("slang_tuple_get",        slang_tuple_get        as *const u8);
        builder.symbol("slang_tuple_len",        slang_tuple_len        as *const u8);
        // ── v15: Error handling ──────────────────────────────────────────
        builder.symbol("slang_error_set",        slang_error_set        as *const u8);
        builder.symbol("slang_error_check",      slang_error_check      as *const u8);
        builder.symbol("slang_error_msg",        slang_error_msg        as *const u8);
        builder.symbol("slang_error_clear",      slang_error_clear      as *const u8);
        // ── v15: Environment & System ────────────────────────────────────
        builder.symbol("slang_env_get",          slang_env_get          as *const u8);
        builder.symbol("slang_sleep_ms",         slang_sleep_ms         as *const u8);
        builder.symbol("slang_eprint",           slang_eprint           as *const u8);
        builder.symbol("slang_eprintln",         slang_eprintln         as *const u8);
        builder.symbol("slang_pid",              slang_pid              as *const u8);
        builder.symbol("slang_format_int",       slang_format_int       as *const u8);
        builder.symbol("slang_format_float",     slang_format_float     as *const u8);
        // ── v15: JSON ────────────────────────────────────────────────────
        builder.symbol("slang_json_encode",      slang_json_encode      as *const u8);
        builder.symbol("slang_json_decode",      slang_json_decode      as *const u8);

        // ── v18: Collection methods ──────────────────────────────────────
        builder.symbol("slang_array_push",       slang_array_push       as *const u8);
        builder.symbol("slang_array_pop",        slang_array_pop        as *const u8);
        builder.symbol("slang_array_contains",   slang_array_contains   as *const u8);
        builder.symbol("slang_array_reverse",    slang_array_reverse    as *const u8);
        builder.symbol("slang_array_sort",       slang_array_sort       as *const u8);
        builder.symbol("slang_array_join",       slang_array_join       as *const u8);
        builder.symbol("slang_array_slice",      slang_array_slice      as *const u8);
        builder.symbol("slang_array_find",       slang_array_find       as *const u8);
        // ── Iterator / functional array ops ───────────────────────────
        builder.symbol("slang_array_range",        slang_array_range        as *const u8);
        builder.symbol("slang_array_sum",          slang_array_sum          as *const u8);
        builder.symbol("slang_array_min",          slang_array_min          as *const u8);
        builder.symbol("slang_array_max",          slang_array_max          as *const u8);
        builder.symbol("slang_array_any",          slang_array_any          as *const u8);
        builder.symbol("slang_array_all_positive", slang_array_all_positive as *const u8);
        builder.symbol("slang_array_count",        slang_array_count        as *const u8);
        builder.symbol("slang_array_flatten",      slang_array_flatten      as *const u8);
        builder.symbol("slang_array_zip",          slang_array_zip          as *const u8);
        builder.symbol("slang_array_enumerate",    slang_array_enumerate    as *const u8);
        builder.symbol("slang_array_take",         slang_array_take         as *const u8);
        builder.symbol("slang_array_drop",         slang_array_drop         as *const u8);
        builder.symbol("slang_array_unique",       slang_array_unique       as *const u8);
        builder.symbol("slang_error_message",    slang_error_message    as *const u8);

        // Regex
        builder.symbol("slang_regex_match",          slang_regex_match          as *const u8);
        builder.symbol("slang_regex_is_match",       slang_regex_is_match       as *const u8);
        builder.symbol("slang_regex_find",           slang_regex_find           as *const u8);
        builder.symbol("slang_regex_replace",        slang_regex_replace        as *const u8);
        builder.symbol("slang_regex_split_count",    slang_regex_split_count    as *const u8);
        builder.symbol("slang_regex_split_get",      slang_regex_split_get      as *const u8);
        builder.symbol("slang_regex_find_all_count", slang_regex_find_all_count as *const u8);
        builder.symbol("slang_regex_find_all_get",   slang_regex_find_all_get   as *const u8);
        // ── v18: Async stubs ─────────────────────────────────────────────
        builder.symbol("slang_spawn",          slang_spawn          as *const u8);
        builder.symbol("slang_task_result",    slang_task_result    as *const u8);
        // ── v18: Networking ──────────────────────────────────────────────
        builder.symbol("slang_http_get",       slang_http_get       as *const u8);
        builder.symbol("slang_http_post",      slang_http_post      as *const u8);
        builder.symbol("slang_http_status",    slang_http_status    as *const u8);
        builder.symbol("slang_tcp_connect",    slang_tcp_connect    as *const u8);
        builder.symbol("slang_tcp_send",       slang_tcp_send       as *const u8);
        builder.symbol("slang_tcp_close",      slang_tcp_close      as *const u8);
        // ── Module system ────────────────────────────────────────────────
        builder.symbol("slang_module_loaded",     slang_module_loaded     as *const u8);

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
        // ── v15: String operations ───────────────────────────────────────
        decl_fn!("slang_str_upper",        [ptr_type],                    ptr_type);
        decl_fn!("slang_str_lower",        [ptr_type],                    ptr_type);
        decl_fn!("slang_str_trim",         [ptr_type],                    ptr_type);
        decl_fn!("slang_str_contains",     [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_str_starts_with",  [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_str_ends_with",    [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_str_char_at",      [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_str_substr",       [ptr_type, types::I64, types::I64], ptr_type);
        decl_fn!("slang_str_index_of",     [ptr_type, ptr_type],          types::I64);
        decl_fn!("slang_str_replace",      [ptr_type, ptr_type, ptr_type], ptr_type);
        decl_fn!("slang_str_repeat",       [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_str_reverse",      [ptr_type],                    ptr_type);
        decl_fn!("slang_str_split_count",  [ptr_type, ptr_type],          types::I64);
        decl_fn!("slang_str_split_get",    [ptr_type, ptr_type, types::I64], ptr_type);
        decl_fn!("slang_to_string_i64",    [types::I64],                  ptr_type);
        decl_fn!("slang_to_string_f64",    [types::F64],                  ptr_type);
        decl_fn!("slang_to_string_bool",   [types::I8],                   ptr_type);
        decl_fn!("slang_str_format_i64",   [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_str_format_f64",   [ptr_type, types::F64],        ptr_type);
        decl_fn!("slang_str_format_str",   [ptr_type, ptr_type],          ptr_type);
        decl_fn!("slang_parse_int",        [ptr_type],                    types::I64);
        decl_fn!("slang_parse_float",      [ptr_type],                    types::F64);
        // ── v15: File I/O ────────────────────────────────────────────────
        decl_fn!("slang_file_read",        [ptr_type],                    ptr_type);
        decl_fn!("slang_file_write",       [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_file_append",      [ptr_type, ptr_type],          types::I8);
        decl_fn!("slang_file_exists",      [ptr_type],                    types::I8);
        decl_fn!("slang_file_delete",      [ptr_type],                    types::I8);
        decl_fn!("slang_file_size",        [ptr_type],                    types::I64);
        // ── v15: Map operations ──────────────────────────────────────────
        decl_fn!("slang_map_new",          [],                            types::I64);
        decl_fn!("slang_map_set",          [types::I64, ptr_type, types::I64], types::INVALID);
        decl_fn!("slang_map_get",          [types::I64, ptr_type],        types::I64);
        decl_fn!("slang_map_has",          [types::I64, ptr_type],        types::I8);
        decl_fn!("slang_map_remove",       [types::I64, ptr_type],        types::INVALID);
        decl_fn!("slang_map_len",          [types::I64],                  types::I64);
        decl_fn!("slang_map_keys",         [types::I64],                  ptr_type);
        // ── v16: Set operations ──────────────────────────────────────────
        decl_fn!("slang_set_new",          [],                            types::I64);
        decl_fn!("slang_set_add",          [types::I64, types::I64],      types::INVALID);
        decl_fn!("slang_set_has",          [types::I64, types::I64],      types::I8);
        decl_fn!("slang_set_remove",       [types::I64, types::I64],      types::INVALID);
        decl_fn!("slang_set_len",          [types::I64],                  types::I64);
        decl_fn!("slang_set_union",        [types::I64, types::I64],      types::I64);
        decl_fn!("slang_set_intersect",    [types::I64, types::I64],      types::I64);
        decl_fn!("slang_set_diff",         [types::I64, types::I64],      types::I64);
        decl_fn!("slang_set_to_array",     [types::I64],                  ptr_type);
        // ── v18: Tuple operations ────────────────────────────────────────
        decl_fn!("slang_tuple_new2",       [types::I64, types::I64],                       types::I64);
        decl_fn!("slang_tuple_new3",       [types::I64, types::I64, types::I64],            types::I64);
        decl_fn!("slang_tuple_new4",       [types::I64, types::I64, types::I64, types::I64], types::I64);
        decl_fn!("slang_tuple_get",        [types::I64, types::I64],                       types::I64);
        decl_fn!("slang_tuple_len",        [types::I64],                                   types::I64);
        // ── v15: Error handling ──────────────────────────────────────────
        decl_fn!("slang_error_set",        [types::I64, ptr_type],        types::INVALID);
        decl_fn!("slang_error_check",      [],                            types::I64);
        decl_fn!("slang_error_msg",        [],                            ptr_type);
        decl_fn!("slang_error_clear",      [],                            types::INVALID);
        // ── v15: Environment & System ────────────────────────────────────
        decl_fn!("slang_env_get",          [ptr_type],                    ptr_type);
        decl_fn!("slang_sleep_ms",         [types::I64],                  types::INVALID);
        decl_fn!("slang_eprint",           [ptr_type],                    types::INVALID);
        decl_fn!("slang_eprintln",         [ptr_type],                    types::INVALID);
        decl_fn!("slang_pid",              [],                            types::I64);
        decl_fn!("slang_format_int",       [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_format_float",     [ptr_type, types::F64],        ptr_type);
        // ── v15: JSON ────────────────────────────────────────────────────
        decl_fn!("slang_json_encode",      [types::I64],                  ptr_type);
        decl_fn!("slang_json_decode",      [ptr_type],                    types::I64);

        // ── v18: Collection methods ──────────────────────────────────────
        decl_fn!("slang_array_push",       [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_array_pop",        [ptr_type],                    types::I64);
        decl_fn!("slang_array_contains",   [ptr_type, types::I64],        types::I64);
        decl_fn!("slang_array_reverse",    [ptr_type],                    ptr_type);
        decl_fn!("slang_array_sort",       [ptr_type],                    ptr_type);
        decl_fn!("slang_array_join",       [ptr_type, ptr_type],          ptr_type);
        decl_fn!("slang_array_slice",      [ptr_type, types::I64, types::I64], ptr_type);
        decl_fn!("slang_array_find",       [ptr_type, types::I64],        types::I64);
        // ── Iterator / functional array ops ───────────────────────────
        decl_fn!("slang_array_range",        [types::I64, types::I64],      ptr_type);
        decl_fn!("slang_array_sum",          [ptr_type],                    types::I64);
        decl_fn!("slang_array_min",          [ptr_type],                    types::I64);
        decl_fn!("slang_array_max",          [ptr_type],                    types::I64);
        decl_fn!("slang_array_any",          [ptr_type, types::I64],        types::I8);
        decl_fn!("slang_array_all_positive", [ptr_type],                    types::I8);
        decl_fn!("slang_array_count",        [ptr_type, types::I64],        types::I64);
        decl_fn!("slang_array_flatten",      [ptr_type],                    ptr_type);
        decl_fn!("slang_array_zip",          [ptr_type, ptr_type],          ptr_type);
        decl_fn!("slang_array_enumerate",    [ptr_type],                    ptr_type);
        decl_fn!("slang_array_take",         [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_array_drop",         [ptr_type, types::I64],        ptr_type);
        decl_fn!("slang_array_unique",       [ptr_type],                    ptr_type);
        decl_fn!("slang_error_message",    [],                            ptr_type);

        // Regex
        decl_fn!("slang_regex_match",          [ptr_type, ptr_type],                    types::I8);
        decl_fn!("slang_regex_is_match",       [ptr_type, ptr_type],                    types::I8);
        decl_fn!("slang_regex_find",           [ptr_type, ptr_type],                    ptr_type);
        decl_fn!("slang_regex_replace",        [ptr_type, ptr_type, ptr_type],           ptr_type);
        decl_fn!("slang_regex_split_count",    [ptr_type, ptr_type],                    types::I64);
        decl_fn!("slang_regex_split_get",      [ptr_type, ptr_type, types::I64],         ptr_type);
        decl_fn!("slang_regex_find_all_count", [ptr_type, ptr_type],                    types::I64);
        decl_fn!("slang_regex_find_all_get",   [ptr_type, ptr_type, types::I64],         ptr_type);

        // v18: Async stubs
        decl_fn!("slang_spawn",        [types::I64],              types::I64);
        decl_fn!("slang_task_result",  [types::I64],              types::I64);

        // v18: Networking
        decl_fn!("slang_http_get",     [ptr_type],                ptr_type);
        decl_fn!("slang_http_post",    [ptr_type, ptr_type],      ptr_type);
        decl_fn!("slang_http_status",  [ptr_type],                types::I64);
        decl_fn!("slang_tcp_connect",  [ptr_type, types::I64],    types::I64);
        decl_fn!("slang_tcp_send",     [types::I64, ptr_type],    types::I64);
        decl_fn!("slang_tcp_close",    [types::I64],              types::INVALID);

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
                    // ── v15: String operations ───────────────────────────
                    "str_upper"       => "slang_str_upper".into(),
                    "str_lower"       => "slang_str_lower".into(),
                    "str_trim"        => "slang_str_trim".into(),
                    "str_contains"    => "slang_str_contains".into(),
                    "str_starts_with" => "slang_str_starts_with".into(),
                    "str_ends_with"   => "slang_str_ends_with".into(),
                    "str_char_at"     => "slang_str_char_at".into(),
                    "str_substr"      => "slang_str_substr".into(),
                    "str_index_of"    => "slang_str_index_of".into(),
                    "str_replace"     => "slang_str_replace".into(),
                    "str_repeat"      => "slang_str_repeat".into(),
                    "str_reverse"     => "slang_str_reverse".into(),
                    "str_split_count" => "slang_str_split_count".into(),
                    "str_split_get"   => "slang_str_split_get".into(),
                    "to_string_i64"   => "slang_to_string_i64".into(),
                    "to_string_f64"   => "slang_to_string_f64".into(),
                    "to_string_bool"  => "slang_to_string_bool".into(),
                    "str_format_i64"  => "slang_str_format_i64".into(),
                    "str_format_f64"  => "slang_str_format_f64".into(),
                    "str_format_str"  => "slang_str_format_str".into(),
                    "parse_int"       => "slang_parse_int".into(),
                    "parse_float"     => "slang_parse_float".into(),
                    // ── v15: File I/O ────────────────────────────────────
                    "file_read"       => "slang_file_read".into(),
                    "file_write"      => "slang_file_write".into(),
                    "file_append"     => "slang_file_append".into(),
                    "file_exists"     => "slang_file_exists".into(),
                    "file_delete"     => "slang_file_delete".into(),
                    "file_size"       => "slang_file_size".into(),
                    // ── v15: Map operations ──────────────────────────────
                    "map_new"         => "slang_map_new".into(),
                    "map_set"         => "slang_map_set".into(),
                    "map_get"         => "slang_map_get".into(),
                    "map_has"         => "slang_map_has".into(),
                    "map_remove"      => "slang_map_remove".into(),
                    "map_len"         => "slang_map_len".into(),
                    "map_keys"        => "slang_map_keys".into(),
                    // ── v16: Set operations ──────────────────────────────
                    "set_new"         => "slang_set_new".into(),
                    "set_add"         => "slang_set_add".into(),
                    "set_has"         => "slang_set_has".into(),
                    "set_remove"      => "slang_set_remove".into(),
                    "set_len"         => "slang_set_len".into(),
                    "set_union"       => "slang_set_union".into(),
                    "set_intersect"   => "slang_set_intersect".into(),
                    "set_diff"        => "slang_set_diff".into(),
                    "set_to_array"    => "slang_set_to_array".into(),
                    // ── v18: Tuple operations ────────────────────────────
                    "tuple_new2"      => "slang_tuple_new2".into(),
                    "tuple_new3"      => "slang_tuple_new3".into(),
                    "tuple_new4"      => "slang_tuple_new4".into(),
                    "tuple_get"       => "slang_tuple_get".into(),
                    "tuple_len"       => "slang_tuple_len".into(),
                    // ── v15: Error handling ──────────────────────────────
                    "error_set"       => "slang_error_set".into(),
                    "error_check"     => "slang_error_check".into(),
                    "error_msg"       => "slang_error_msg".into(),
                    "error_clear"     => "slang_error_clear".into(),
                    // ── v15: Environment & System ────────────────────────
                    "env_get"         => "slang_env_get".into(),
                    "sleep_ms"        => "slang_sleep_ms".into(),
                    "eprint"          => "slang_eprint".into(),
                    "eprintln"        => "slang_eprintln".into(),
                    "pid"             => "slang_pid".into(),
                    "format_int"      => "slang_format_int".into(),
                    "format_float"    => "slang_format_float".into(),
                    // ── v15: JSON ────────────────────────────────────────
                    "json_encode"     => "slang_json_encode".into(),
                    "json_decode"     => "slang_json_decode".into(),
                    // ── v18: Collection methods ──────────────────────────
                    "array_push"      => "slang_array_push".into(),
                    "array_pop"       => "slang_array_pop".into(),
                    "array_contains"  => "slang_array_contains".into(),
                    "array_reverse"   => "slang_array_reverse".into(),
                    "array_sort"      => "slang_array_sort".into(),
                    "array_join"      => "slang_array_join".into(),
                    "array_slice"     => "slang_array_slice".into(),
                    "array_find"      => "slang_array_find".into(),
                    // Iterator / functional array ops
                    "array_range"        => "slang_array_range".into(),
                    "array_sum"          => "slang_array_sum".into(),
                    "array_min"          => "slang_array_min".into(),
                    "array_max"          => "slang_array_max".into(),
                    "array_any"          => "slang_array_any".into(),
                    "array_all_positive" => "slang_array_all_positive".into(),
                    "array_count"        => "slang_array_count".into(),
                    "array_flatten"      => "slang_array_flatten".into(),
                    "array_zip"          => "slang_array_zip".into(),
                    "array_enumerate"    => "slang_array_enumerate".into(),
                    "array_take"         => "slang_array_take".into(),
                    "array_drop"         => "slang_array_drop".into(),
                    "array_unique"       => "slang_array_unique".into(),
                    "error_message"   => "slang_error_message".into(),

                    // Regex
                    "regex_match"          => "slang_regex_match".into(),
                    "regex_is_match"       => "slang_regex_is_match".into(),
                    "regex_find"           => "slang_regex_find".into(),
                    "regex_replace"        => "slang_regex_replace".into(),
                    "regex_split_count"    => "slang_regex_split_count".into(),
                    "regex_split_get"      => "slang_regex_split_get".into(),
                    "regex_find_all_count" => "slang_regex_find_all_count".into(),
                    "regex_find_all_get"   => "slang_regex_find_all_get".into(),

                    // v18: Async stubs
                    "spawn"           => "slang_spawn".into(),
                    "task_result"     => "slang_task_result".into(),

                    // v18: Networking
                    "http_get"        => "slang_http_get".into(),
                    "http_post"       => "slang_http_post".into(),
                    "http_status"     => "slang_http_status".into(),
                    "tcp_connect"     => "slang_tcp_connect".into(),
                    "tcp_send"        => "slang_tcp_send".into(),
                    "tcp_close"       => "slang_tcp_close".into(),

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
                    // Collect Phi args for then_bb
                    let then_args: Vec<cranelift::prelude::Value> =
                        if let Some(phis) = phis_by_block.get(then_bb) {
                            phis.iter().map(|phi| {
                                phi.incoming.iter()
                                    .find(|(_, from)| *from == current_bb)
                                    .map(|(v, _)| Self::get_value(*v, value_map, builder))
                                    .unwrap_or_else(|| builder.ins().iconst(types::I64, 0))
                            }).collect()
                        } else { vec![] };
                    // Collect Phi args for else_bb
                    let else_args: Vec<cranelift::prelude::Value> =
                        if let Some(phis) = phis_by_block.get(else_bb) {
                            phis.iter().map(|phi| {
                                phi.incoming.iter()
                                    .find(|(_, from)| *from == current_bb)
                                    .map(|(v, _)| Self::get_value(*v, value_map, builder))
                                    .unwrap_or_else(|| builder.ins().iconst(types::I64, 0))
                            }).collect()
                        } else { vec![] };
                    builder.ins().brif(cv, tb, &then_args, eb, &else_args);
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
            // ── Phase 5 → v15: Closure — return real function pointer ────────
            Inst::ClosureAlloc { result, func, .. } => {
                if let Some(func_id) = func_ids.get(func) {
                    let func_ref = module.declare_func_in_func(*func_id, builder.func);
                    let ptr = builder.ins().func_addr(pointer_type, func_ref);
                    value_map.insert(*result, ptr);
                } else {
                    // Fallback: unknown lambda — null sentinel
                    value_map.insert(*result, builder.ins().iconst(pointer_type, 0));
                }
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

    // ── v15: String stdlib tests ──────────────────────────────────────
    #[test]
    fn test_v15_str_len() {
        let r = compile_and_run(r#"fn main() -> i64 { str_len("hello") }"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v15_str_contains() {
        let r = compile_and_run(r#"fn main() -> i64 { if str_contains("hello world", "world") { 1 } else { 0 } }"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v15_str_index_of() {
        let r = compile_and_run(r#"fn main() -> i64 { str_index_of("hello world", "world") }"#);
        assert_eq!(r.unwrap(), 6);
    }

    #[test]
    fn test_v15_str_split_count() {
        let r = compile_and_run(r#"fn main() -> i64 { str_split_count("a,b,c,d", ",") }"#);
        assert_eq!(r.unwrap(), 4);
    }

    #[test]
    fn test_v15_parse_int() {
        let r = compile_and_run(r#"fn main() -> i64 { parse_int("42") }"#);
        assert_eq!(r.unwrap(), 42);
    }

    // ── v15: Map tests ────────────────────────────────────────────────
    #[test]
    fn test_v15_map_new_len() {
        let r = compile_and_run(r#"fn main() -> i64 { let m: i64 = map_new(); map_len(m) }"#);
        assert_eq!(r.unwrap(), 0);
    }

    #[test]
    fn test_v15_map_set_get() {
        let r = compile_and_run(r#"fn main() -> i64 { let m: i64 = map_new(); map_set(m, "key", 42); map_get(m, "key") }"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v15_map_len_after_insert() {
        let r = compile_and_run(r#"fn main() -> i64 { let m: i64 = map_new(); map_set(m, "a", 1); map_set(m, "b", 2); map_len(m) }"#);
        assert_eq!(r.unwrap(), 2);
    }

    // ── v15: Error handling tests ─────────────────────────────────────
    #[test]
    fn test_v15_error_check_initial() {
        let r = compile_and_run("fn main() -> i64 { error_clear(); error_check() }");
        assert_eq!(r.unwrap(), 0);
    }

    #[test]
    fn test_v15_error_set_check() {
        let r = compile_and_run(r#"fn main() -> i64 { error_set(99, "oops"); error_check() }"#);
        assert_eq!(r.unwrap(), 99);
    }

    // ── v15: System tests ─────────────────────────────────────────────
    #[test]
    fn test_v15_pid() {
        let r = compile_and_run("fn main() -> i64 { pid() }");
        assert!(r.unwrap() > 0);
    }

    // ── v15: File I/O tests ───────────────────────────────────────────
    #[test]
    fn test_v15_file_write_read() {
        let r = compile_and_run(r#"fn main() -> i64 {
            file_write("_v15_test.tmp", "hello");
            let len: i64 = str_len(file_read("_v15_test.tmp"));
            file_delete("_v15_test.tmp");
            len
        }"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v15_file_exists() {
        let r = compile_and_run(r#"fn main() -> i64 {
            file_write("_v15_exist.tmp", "x");
            let e: i64 = if file_exists("_v15_exist.tmp") { 1 } else { 0 };
            file_delete("_v15_exist.tmp");
            e
        }"#);
        assert_eq!(r.unwrap(), 1);
    }

    // ══════════════════════════════════════════════════════════════════
    // v18 Tests — OOP, error handling, collections, for-each, break/continue
    // ══════════════════════════════════════════════════════════════════

    // ── v18: Array collection methods ─────────────────────────────────

    #[test]
    fn test_v18_array_push() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
let arr2: [i64] = array_push(arr, 40);
arr2[3]
}"#);
        assert_eq!(r.unwrap(), 40);
    }

    #[test]
    fn test_v18_array_contains() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_contains(arr, 20)
}"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v18_array_contains_missing() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_contains(arr, 99)
}"#);
        assert_eq!(r.unwrap(), 0);
    }

    #[test]
    fn test_v18_array_find() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_find(arr, 30)
}"#);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn test_v18_array_find_missing() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_find(arr, 99)
}"#);
        assert_eq!(r.unwrap(), -1);
    }

    #[test]
    fn test_v18_array_pop() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_pop(arr)
}"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_array_sort() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [30, 10, 20];
array_sort(arr);
arr[0]
}"#);
        assert_eq!(r.unwrap(), 10);
    }

    #[test]
    fn test_v18_array_reverse() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_reverse(arr);
arr[0]
}"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_array_slice() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30, 40, 50];
let s: [i64] = array_slice(arr, 1, 4);
s[0]
}"#);
        assert_eq!(r.unwrap(), 20);
    }

    #[test]
    fn test_v18_array_slice_len() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30, 40, 50];
let s: [i64] = array_slice(arr, 1, 4);
s.len()
}"#);
        assert_eq!(r.unwrap(), 3);
    }

    // ── v18: For-each over arrays ─────────────────────────────────────

    #[test]
    fn test_v18_for_each_array() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
let mut sum: i64 = 0;
for x in arr {
    sum = sum + x;
}
sum
}"#);
        assert_eq!(r.unwrap(), 60);
    }

    // ── v18: Break / Continue ─────────────────────────────────────────

    #[test]
    fn test_v18_while_break() {
        let r = compile_and_run(r#"fn main() -> i64 {
let mut i: i64 = 0;
while i < 100 {
    if i == 5 { break }
    i = i + 1;
}
i
}"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v18_for_break() {
        let r = compile_and_run(r#"fn main() -> i64 {
let mut result: i64 = 0;
for i in 0..100 {
    if i == 10 { break }
    result = i;
}
result
}"#);
        assert_eq!(r.unwrap(), 9);
    }

    // ── v18: Struct field access ──────────────────────────────────────

    #[test]
    fn test_v18_struct_field_by_name() {
        let r = compile_and_run(r#"
struct Point { x: i64, y: i64 }

fn main() -> i64 {
    let p: Point = Point { x: 10, y: 20 };
    p.x + p.y
}"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_struct_second_field() {
        let r = compile_and_run(r#"
struct Pair { first: i64, second: i64 }

fn main() -> i64 {
    let p: Pair = Pair { first: 3, second: 7 };
    p.second
}"#);
        assert_eq!(r.unwrap(), 7);
    }

    // ── v18: Impl methods ────────────────────────────────────────────

    #[test]
    fn test_v18_impl_method_call() {
        let r = compile_and_run(r#"
struct Rect { w: i64, h: i64 }

impl Rect {
    fn area(self: Rect) -> i64 {
        self.w * self.h
    }
}

fn main() -> i64 {
    let r: Rect = Rect { w: 5, h: 3 };
    r.area()
}"#);
        assert_eq!(r.unwrap(), 15);
    }

    // ── v18: Try/Catch ───────────────────────────────────────────────

    #[test]
    fn test_v18_try_catch_no_error() {
        let r = compile_and_run(r#"fn main() -> i64 {
let result: i64 = try {
    42
} catch e {
    0
};
result
}"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v18_try_catch_with_throw() {
        let r = compile_and_run(r#"fn main() -> i64 {
let result: i64 = try {
    throw(1, "oops");
    42
} catch e {
    99
};
result
}"#);
        assert_eq!(r.unwrap(), 99);
    }

    // ── v18: Error handling flow ─────────────────────────────────────

    #[test]
    fn test_v18_struct_fn_call() {
        // Free function with struct argument (NOT impl method)
        let r = compile_and_run(r#"
struct Rect { w: i64, h: i64 }

fn get_w(r: Rect) -> i64 {
    r.w
}

fn main() -> i64 {
    let r: Rect = Rect { w: 5, h: 3 };
    get_w(r)
}"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v18_throw_sets_error() {
        let r = compile_and_run(r#"fn main() -> i64 {
error_clear();
throw(42, "test error");
error_check()
}"#);
        assert_eq!(r.unwrap(), 42);
    }

    // ── v16: Set tests ──────────────────────────────────────────────────
    #[test]
    fn test_v16_set_new_len() {
        let r = compile_and_run(r#"fn main() -> i64 { let s: i64 = set_new(); set_len(s) }"#);
        assert_eq!(r.unwrap(), 0);
    }

    #[test]
    fn test_v16_set_add_has() {
        let r = compile_and_run(r#"fn main() -> i64 {
let s: i64 = set_new();
set_add(s, 42);
let h: i64 = if set_has(s, 42) { 1 } else { 0 };
h
}"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v16_set_add_len() {
        let r = compile_and_run(r#"fn main() -> i64 {
let s: i64 = set_new();
set_add(s, 1);
set_add(s, 2);
set_add(s, 3);
set_add(s, 2);
set_len(s)
}"#);
        assert_eq!(r.unwrap(), 3); // duplicates ignored
    }

    #[test]
    fn test_v16_set_remove() {
        let r = compile_and_run(r#"fn main() -> i64 {
let s: i64 = set_new();
set_add(s, 10);
set_add(s, 20);
set_remove(s, 10);
set_len(s)
}"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v16_set_union() {
        let r = compile_and_run(r#"fn main() -> i64 {
let a: i64 = set_new();
set_add(a, 1);
set_add(a, 2);
let b: i64 = set_new();
set_add(b, 2);
set_add(b, 3);
let c: i64 = set_union(a, b);
set_len(c)
}"#);
        assert_eq!(r.unwrap(), 3); // {1,2,3}
    }

    #[test]
    fn test_v16_set_intersect() {
        let r = compile_and_run(r#"fn main() -> i64 {
let a: i64 = set_new();
set_add(a, 1);
set_add(a, 2);
set_add(a, 3);
let b: i64 = set_new();
set_add(b, 2);
set_add(b, 3);
set_add(b, 4);
let c: i64 = set_intersect(a, b);
set_len(c)
}"#);
        assert_eq!(r.unwrap(), 2); // {2,3}
    }

    #[test]
    fn test_v16_set_diff() {
        let r = compile_and_run(r#"fn main() -> i64 {
let a: i64 = set_new();
set_add(a, 1);
set_add(a, 2);
set_add(a, 3);
let b: i64 = set_new();
set_add(b, 2);
let c: i64 = set_diff(a, b);
set_len(c)
}"#);
        assert_eq!(r.unwrap(), 2); // {1,3}
    }

    // ── v18: Tuple tests ─────────────────────────────────────────────

    #[test]
    fn test_v18_tuple_new2() {
        let r = compile_and_run(r#"fn main() -> i64 {
let t: i64 = tuple_new2(10, 20);
let a: i64 = tuple_get(t, 0);
let b: i64 = tuple_get(t, 1);
a + b
}"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_tuple_new3() {
        let r = compile_and_run(r#"fn main() -> i64 {
let t: i64 = tuple_new3(1, 2, 3);
let a: i64 = tuple_get(t, 0);
let b: i64 = tuple_get(t, 1);
let c: i64 = tuple_get(t, 2);
a + b + c
}"#);
        assert_eq!(r.unwrap(), 6);
    }

    #[test]
    fn test_v18_tuple_len() {
        let r = compile_and_run(r#"fn main() -> i64 {
let t: i64 = tuple_new2(5, 6);
tuple_len(t)
}"#);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn test_v18_tuple_get() {
        let r = compile_and_run(r#"fn main() -> i64 {
let t: i64 = tuple_new3(100, 200, 300);
tuple_get(t, 2)
}"#);
        assert_eq!(r.unwrap(), 300);
    }

    // ── v18: Regex tests ──────────────────────────────────────────────

    #[test]
    fn test_v18_regex_is_match() {
        // partial match: digits found inside "abc123"
        let r = compile_and_run(r#"fn main() -> i64 { if regex_is_match("\d+", "abc123") { 1 } else { 0 } }"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v18_regex_match() {
        // full match: "abc" matches ^abc$
        let r = compile_and_run(r#"fn main() -> i64 { if regex_match("^abc$", "abc") { 1 } else { 0 } }"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v18_regex_find_all_count() {
        // "\d+" matches "1", "22", "333" => 3
        let r = compile_and_run(r#"fn main() -> i64 { regex_find_all_count("\d+", "a1b22c333") }"#);
        assert_eq!(r.unwrap(), 3);
    }

    #[test]
    fn test_v18_regex_split_count() {
        // splitting "a,b,c,d" by "," => 4 segments
        let r = compile_and_run(r#"fn main() -> i64 { regex_split_count(",", "a,b,c,d") }"#);
        assert_eq!(r.unwrap(), 4);
    }

    #[test]
    fn test_v18_regex_replace() {
        // replace digits with "XX" in "a1b2c3" -> "aXXbXXcXX" (len 9)
        let r = compile_and_run(r#"fn main() -> i64 {
let s: str = regex_replace("\d", "a1b2c3", "XX");
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 9);
    }

    // ── v18: String formatting ────────────────────────────────────────

    #[test]
    fn test_v18_str_format_i64() {
        // str_format_i64("val={}", 42) → "val=42" (len 6)
        let r = compile_and_run(r#"fn main() -> i64 {
let s: str = str_format_i64("val={}", 42);
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 6);
    }

    #[test]
    fn test_v18_str_format_f64() {
        // str_format_f64("pi={}", 3.14) → "pi=3.14" (len 7)
        let r = compile_and_run(r#"fn main() -> i64 {
let s: str = str_format_f64("pi={}", 3.14);
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 7);
    }

    #[test]
    fn test_v18_str_format_str() {
        // str_format_str("hi {}", "world") → "hi world" (len 8)
        let r = compile_and_run(r#"fn main() -> i64 {
let s: str = str_format_str("hi {}", "world");
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 8);
    }

    #[test]
    fn test_v18_str_format_chain() {
        // Chain: str_format_i64(str_format_str("{} = {}", "x"), 42) → "x = 42" (len 6)
        let r = compile_and_run(r#"fn main() -> i64 {
let s: str = str_format_i64(str_format_str("{} = {}", "x"), 42);
str_len(s)
}"#);
        assert_eq!(r.unwrap(), 6);
    }

    // ── Iterator / functional array tests ─────────────────────────────

    #[test]
    fn test_v18_array_range() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = array_range(1, 5);
arr.len()
}"#);
        assert_eq!(r.unwrap(), 4);
    }

    #[test]
    fn test_v18_array_sum() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30];
array_sum(arr)
}"#);
        assert_eq!(r.unwrap(), 60);
    }

    #[test]
    fn test_v18_array_min_max() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [5, 2, 8, 1];
array_min(arr)
}"#);
        assert_eq!(r.unwrap(), 1);
        let r2 = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [5, 2, 8, 1];
array_max(arr)
}"#);
        assert_eq!(r2.unwrap(), 8);
    }

    #[test]
    fn test_v18_array_count() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [1, 2, 1, 3, 1];
array_count(arr, 1)
}"#);
        assert_eq!(r.unwrap(), 3);
    }

    #[test]
    fn test_v18_array_unique() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [1, 2, 2, 3, 3, 3];
let u: [i64] = array_unique(arr);
u.len()
}"#);
        assert_eq!(r.unwrap(), 3);
    }

    #[test]
    fn test_v18_array_take() {
        let r = compile_and_run(r#"fn main() -> i64 {
let arr: [i64] = [10, 20, 30, 40];
let t: [i64] = array_take(arr, 2);
t.len()
}"#);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn test_v18_module_basic() {
        let r = compile_and_run(r#"
module math {
    fn add(a: i64, b: i64) -> i64 {
        a + b
    }
}

fn main() -> i64 {
    math::add(10, 20)
}
"#);
        assert_eq!(r.unwrap(), 30);
    }

    #[test]
    fn test_v18_module_nested_fn() {
        let r = compile_and_run(r#"
module math {
    fn add(a: i64, b: i64) -> i64 { a + b }
    fn mul(a: i64, b: i64) -> i64 { a * b }
}

fn main() -> i64 {
    math::add(3, 4) + math::mul(5, 6)
}
"#);
        assert_eq!(r.unwrap(), 37);
    }

    #[test]
    fn test_v18_async_fn() {
        let r = compile_and_run(r#"
async fn compute() -> i64 { 42 }
fn main() -> i64 { compute() }
"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v18_spawn_stub() {
        let r = compile_and_run(r#"
fn main() -> i64 { spawn(1) }
"#);
        assert_eq!(r.unwrap(), 1);
    }

    #[test]
    fn test_v18_task_result_stub() {
        let r = compile_and_run(r#"
fn main() -> i64 { task_result(99) }
"#);
        assert_eq!(r.unwrap(), 99);
    }

    #[test]
    fn test_v18_tcp_stub() {
        let r = compile_and_run(r#"
fn main() -> i64 {
    let h: i64 = tcp_connect("127.0.0.1", 80);
    tcp_close(h);
    42
}
"#);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v18_await_expr() {
        let r = compile_and_run(r#"
async fn fetch() -> i64 { 7 }
fn main() -> i64 { await fetch() }
"#);
        assert_eq!(r.unwrap(), 7);
    }

    #[test]
    fn test_v18_networking_compiles() {
        // Verify http_get, http_post, http_status compile without error
        // (we don't actually make network calls)
        let r = compile_and_run(r#"
fn main() -> i64 {
    let h: i64 = tcp_connect("localhost", 8080);
    let sent: i64 = tcp_send(h, "hello");
    tcp_close(h);
    42
}
"#);
        assert_eq!(r.unwrap(), 42);
    }
}
