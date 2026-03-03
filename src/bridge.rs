//! C FFI Bridge — exposes the Vitalis compiler to external callers (Python, C, etc.)
//!
//! This module exports `extern "C"` functions that can be called via `ctypes` from Python,
//! or via any language with C FFI support.
//!
//! # Python Usage
//!
//! ```python
//! import vitalis  # uses the wrapper module
//!
//! result = vitalis.compile_and_run("fn main() -> i64 { 42 }")
//! assert result == 42
//! ```
//!
//! Or directly via ctypes:
//!
//! ```python
//! import ctypes, json
//! lib = ctypes.CDLL("vitalis.dll")
//! # ... see vitalis.py wrapper for details
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── Internal helpers ────────────────────────────────────────────────────────
fn alloc_cstring(s: &str) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("ERROR: internal null byte").unwrap())
        .into_raw()
}

/// Allocate a C string into the error out-parameter.
/// Called from within `unsafe extern "C"` functions only.
fn write_error(error_out: *mut *mut c_char, msg: &str) {
    if !error_out.is_null() {
        unsafe { *error_out = alloc_cstring(msg) };
    }
}

/// JSON-safe string escaping: handles \, ", \n, \r, \t, and NUL bytes.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"'  => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\u0000"),
            c    => out.push(c),
        }
    }
    out
}


/// Compile and JIT-execute Vitalis source.
/// Returns the i64 result of the `main` function.
/// If compilation fails, returns i64::MIN and writes the error to the error buffer.
///
/// # Safety
/// `source` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_compile_and_run(source: *const c_char, error_out: *mut *mut c_char) -> i64 {
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            write_error(error_out, &format!("invalid UTF-8 source: {}", e));
            return i64::MIN;
        }
    };

    match crate::codegen::compile_and_run(source) {
        Ok(result) => result,
        Err(e) => {
            write_error(error_out, &e);
            i64::MIN
        }
    }
}

/// Compile and JIT-execute a Vitalis `.sl` file by path.
/// Returns the i64 result of main(), or i64::MIN on error (error_out populated).
/// Returns i64::MIN as a sentinel if the file doesn't exist.
///
/// # Safety
/// `path` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_run_file(path: *const c_char, error_out: *mut *mut c_char) -> i64 {
    use std::fs;
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            write_error(error_out, &format!("invalid UTF-8 path: {}", e));
            return i64::MIN;
        }
    };
    let source = match fs::read_to_string(path_str) {
        Ok(s) => s,
        Err(e) => {
            write_error(error_out, &format!("cannot read file '{}': {}", path_str, e));
            return i64::MIN;
        }
    };
    match crate::codegen::compile_and_run(&source) {
        Ok(result) => result,
        Err(e) => {
            write_error(error_out, &e);
            i64::MIN
        }
    }
}

/// Type-check Vitalis source without executing.
/// Returns a JSON array of error strings.
/// Caller must free the returned string with `slang_free_string`.
///
/// # Safety
/// `source` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_check(source: *const c_char) -> *mut c_char {
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(e) => return alloc_cstring(&format!("[\"invalid UTF-8: {}\"]", e)),
    };

    let (program, parse_errors) = crate::parser::parse(source);
    let mut errors: Vec<String> = parse_errors.iter().map(|e| format!("parse: {}", e)).collect();

    let type_errors = crate::types::TypeChecker::new().check(&program);
    errors.extend(type_errors.iter().map(|e| format!("type: {:?}", e)));

    let json = format!(
        "[{}]",
        errors
            .iter()
            .map(|e| format!("\"{}\"", json_escape(e)))
            .collect::<Vec<_>>()
            .join(",")
    );
    alloc_cstring(&json)
}

/// Parse Vitalis source and return the AST as a debug string.
/// Caller must free the returned string with `slang_free_string`.
///
/// # Safety
/// `source` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_parse_ast(source: *const c_char) -> *mut c_char {
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring("ERROR: invalid UTF-8"),
    };

    let (program, _errors) = crate::parser::parse(source);
    alloc_cstring(&format!("{:#?}", program))
}

/// Lex Vitalis source and return tokens as a JSON array of [kind, text] pairs.
/// Caller must free the returned string with `slang_free_string`.
///
/// # Safety
/// `source` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_lex(source: *const c_char) -> *mut c_char {
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring("[]"),
    };

    let (tokens, _errors) = crate::lexer::lex(source);
    let json = format!(
        "[{}]",
        tokens
            .iter()
            .map(|t| {
                let text = &source[t.span.clone()];
                let kind = json_escape(&format!("{:?}", t.token));
                format!("[\"{}\",\"{}\"]", kind, json_escape(text))
            })
            .collect::<Vec<_>>()
            .join(",")
    );
    alloc_cstring(&json)
}

/// Lower Vitalis source to IR and return the dump as a string.
/// Caller must free the returned string with `slang_free_string`.
///
/// # Safety
/// `source` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_dump_ir(source: *const c_char) -> *mut c_char {
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring("ERROR: invalid UTF-8"),
    };

    let (program, parse_errors) = crate::parser::parse(source);
    if !parse_errors.is_empty() {
        return alloc_cstring(&format!(
            "ERROR: parse errors:\n{}",
            parse_errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
        ));
    }

    let _type_errors = crate::types::TypeChecker::new().check(&program);
    let ir_module = crate::ir::IrBuilder::new().build(&program);
    alloc_cstring(&format!("{:#?}", ir_module))
}

/// Return the Vitalis compiler version.
/// Caller must free the returned string with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn slang_version() -> *mut c_char {
    alloc_cstring(env!("CARGO_PKG_VERSION"))
}

/// Free a string allocated by the Vitalis library.
///
/// # Safety
/// `ptr` must have been returned by one of the `slang_*` functions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = unsafe { CString::from_raw(ptr) };
    }
}

/// Free an error string allocated by `slang_compile_and_run`.
///
/// # Safety  
/// `ptr` must have been returned via the `error_out` parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_free_error(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = unsafe { CString::from_raw(ptr) };
    }
}

// ─── Evolution FFI Exports ──────────────────────────────────────────────

use crate::evolution::with_registry;

/// Load a Vitalis program into the evolution registry,
/// automatically extracting `@evolvable` functions.
///
/// # Safety
/// `source` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_evo_load(source: *const c_char) {
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    with_registry(|reg| reg.load_program(source));
}

/// Register a function as evolvable with its source code.
///
/// # Safety
/// `name` and `source` must be valid null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_evo_register(name: *const c_char, source: *const c_char) {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    with_registry(|reg| reg.register(name, source));
}

/// Submit a new variant (mutation) for an evolvable function.
/// Returns the new generation number, or -1 on error.
///
/// # Safety
/// `name` and `new_source` must be valid null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_evo_evolve(name: *const c_char, new_source: *const c_char) -> i64 {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let new_source = match unsafe { CStr::from_ptr(new_source) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    with_registry(|reg| match reg.evolve(name, new_source) {
        Ok((generation, _hash)) => generation as i64,
        Err(_) => -1,
    })
}

/// Set the fitness score for the current variant of a function.
///
/// # Safety
/// `name` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_evo_set_fitness(name: *const c_char, score: f64) {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    with_registry(|reg| reg.set_fitness(name, score));
}

/// Get the fitness score for the current variant. Returns NaN if no score set.
///
/// # Safety
/// `name` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_evo_get_fitness(name: *const c_char) -> f64 {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return f64::NAN,
    };
    with_registry(|reg| reg.get_fitness(name).unwrap_or(f64::NAN))
}

/// Get the generation number for a function. Returns 0 if not found.
///
/// # Safety
/// `name` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_evo_get_generation(name: *const c_char) -> u64 {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    with_registry(|reg| reg.get_generation(name))
}

/// List all evolvable function names as a JSON array.
/// Caller must free the returned string with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn slang_evo_list() -> *mut c_char {
    with_registry(|reg| {
        let names = reg.list_evolvable();
        let json = format!(
            "[{}]",
            names
                .iter()
                .map(|n| format!("\"{}\"", n))
                .collect::<Vec<_>>()
                .join(",")
        );
        alloc_cstring(&json)
    })
}

/// Compile and execute the current evolved program. Returns main() result.
/// Returns i64::MIN on error.
#[unsafe(no_mangle)]
pub extern "C" fn slang_evo_run() -> i64 {
    with_registry(|reg| match reg.compile_and_run() {
        Ok(result) => result,
        Err(_) => i64::MIN,
    })
}

/// Get the source code of an evolvable function.
/// Caller must free the returned string with `slang_free_string`.
///
/// # Safety
/// `name` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_evo_get_source(name: *const c_char) -> *mut c_char {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring(""),
    };
    with_registry(|reg| {
        let src = reg.get_source(name).unwrap_or("");
        alloc_cstring(src)
    })
}

/// Rollback an evolvable function to a previous generation.
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `name` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slang_evo_rollback(name: *const c_char, generation: u64) -> i64 {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    with_registry(|reg| match reg.rollback(name, generation) {
        Ok(_) => 0,
        Err(_) => -1,
    })
}

// ─── Engine FFI Exports ──────────────────────────────────────────────────

use crate::engine::with_engine;

/// Initialize the Vitalis Engine. Call once at startup.
/// Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_engine_init() -> i32 {
    with_engine(|_| {}); // Force initialization
    1
}

/// Register a function for evolution. The engine validates it first.
///
/// # Safety
/// `name` and `source` must be valid null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_engine_register(name: *const c_char, source: *const c_char) -> i32 {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    with_engine(|eng| {
        eng.register(name, source);
    });
    1
}

/// Evolve a function with a new variant. Returns JSON result.
/// Caller must free with `slang_free_string`.
///
/// # Safety
/// `name` and `new_source` must be valid null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_engine_evolve(name: *const c_char, new_source: *const c_char) -> *mut c_char {
    let name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring("{\"success\":false,\"error\":\"invalid UTF-8\"}"),
    };
    let new_source = match unsafe { CStr::from_ptr(new_source) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring("{\"success\":false,\"error\":\"invalid UTF-8\"}"),
    };
    with_engine(|eng| {
        let attempt = eng.evolve(name, new_source);
        alloc_cstring(&attempt.to_json())
    })
}

/// Validate Vitalis source code. Returns JSON validation result.
/// Caller must free with `slang_free_string`.
///
/// # Safety
/// `source` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_engine_validate(source: *const c_char) -> *mut c_char {
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring("{\"valid\":false}"),
    };
    with_engine(|eng| {
        let result = eng.validate(source);
        alloc_cstring(&result.to_json())
    })
}

/// Run one full evolution cycle. Returns JSON cycle result.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_engine_cycle() -> *mut c_char {
    with_engine(|eng| {
        let result = eng.run_cycle();
        alloc_cstring(&result.to_json())
    })
}

/// Get engine statistics as JSON.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_engine_stats() -> *mut c_char {
    with_engine(|eng| {
        alloc_cstring(&eng.stats_json())
    })
}

/// Get the fitness landscape (all functions + scores) as JSON.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_engine_landscape() -> *mut c_char {
    with_engine(|eng| {
        alloc_cstring(&eng.get_landscape())
    })
}

/// Get population-wide summary (diversity, fitness stats, total variants) as JSON.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_engine_population_summary() -> *mut c_char {
    with_registry(|reg| {
        let summary = reg.population_summary();
        alloc_cstring(&summary.to_json())
    })
}

/// Get full runtime diagnostics as JSON: OS/platform config, compile latency histogram
/// (P50/P95/P99), and error log summary (total + most recent error).
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_engine_diagnostics() -> *mut c_char {
    crate::engine::with_engine(|eng| {
        alloc_cstring(&eng.diagnostics_json())
    })
}

/// Get the last `limit` runtime error log entries as a JSON array.
/// Pass `limit = 0` to get all stored entries (up to 200).
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_engine_error_log(limit: u64) -> *mut c_char {
    crate::engine::with_engine(|eng| {
        alloc_cstring(&eng.error_log_json(limit as usize))
    })
}

// ─── Tests ──────────────────────────────────────────────────────────────

// ─── Memory FFI Exports ─────────────────────────────────────────────────

use crate::memory::{with_memory, EngramKind};

/// Store a new engram in the memory system. Returns the engram ID.
///
/// kind: 0=episodic, 1=semantic, 2=procedural, 3=working, 4=emotional
///
/// # Safety
/// `content`, `tags_json`, and `context` must be valid null-terminated C strings.
/// `tags_json` should be a JSON array of strings, e.g. `["tag1","tag2"]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_memory_store(
    kind: u32, content: *const c_char, tags_json: *const c_char,
    importance: f64, context: *const c_char, cycle: u64,
) -> u64 {
    let content = match unsafe { CStr::from_ptr(content) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let tags_str = match unsafe { CStr::from_ptr(tags_json) }.to_str() {
        Ok(s) => s,
        Err(_) => "[]",
    };
    let context = match unsafe { CStr::from_ptr(context) }.to_str() {
        Ok(s) => s,
        Err(_) => "",
    };

    let engram_kind = match kind {
        0 => EngramKind::Episodic,
        1 => EngramKind::Semantic,
        2 => EngramKind::Procedural,
        3 => EngramKind::Working,
        4 => EngramKind::Emotional,
        _ => EngramKind::Episodic,
    };

    // Parse simple JSON tag array
    let tags: Vec<String> = tags_str
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();

    with_memory(|m| {
        m.store(engram_kind, content, &tag_refs, importance, context, cycle)
    })
}

/// Recall memories by tag. Returns JSON array.
/// Caller must free with `slang_free_string`.
///
/// # Safety
/// `tag` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_memory_recall(tag: *const c_char, cycle: u64) -> *mut c_char {
    let tag = match unsafe { CStr::from_ptr(tag) }.to_str() {
        Ok(s) => s,
        Err(_) => return alloc_cstring("[]"),
    };

    with_memory(|m| {
        let recalled = m.recall_by_tag(tag, cycle);
        let entries: Vec<String> = recalled.iter().map(|e| {
            format!(
                "{{\"id\":{},\"kind\":\"{}\",\"content\":\"{}\",\"importance\":{:.3},\"strength\":{:.3}}}",
                e.id,
                e.kind.name(),
                e.content.replace('"', "\\\""),
                e.importance,
                e.strength,
            )
        }).collect();
        alloc_cstring(&format!("[{}]", entries.join(",")))
    })
}

/// Forget a specific memory by ID. Returns 1 on success, 0 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_memory_forget(id: u64) -> i32 {
    with_memory(|m| if m.forget(id) { 1 } else { 0 })
}

/// Apply decay to all memories.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_memory_decay(cycle: u64) {
    with_memory(|m| m.decay(cycle));
}

/// Consolidate memories. Returns JSON result.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_memory_consolidate(cycle: u64) -> *mut c_char {
    with_memory(|m| {
        let result = m.consolidate(cycle);
        alloc_cstring(&result.to_json())
    })
}

/// Get memory statistics as JSON.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_memory_stats() -> *mut c_char {
    with_memory(|m| {
        alloc_cstring(&m.stats_json())
    })
}

/// Get total active engram count.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_memory_count() -> u64 {
    with_memory(|m| m.count() as u64)
}

// ─── Meta-Evolution FFI Exports ─────────────────────────────────────────

use crate::meta_evolution::with_meta;

/// Select the best evolution strategy. Returns strategy name as string.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_meta_select_strategy() -> *mut c_char {
    with_meta(|m| {
        let strategy = m.select_strategy();
        let name = strategy.map(|s| s.name.as_str()).unwrap_or("balanced");
        alloc_cstring(name)
    })
}

/// Record a strategy result.
///
/// # Safety
/// `strategy_name` and `function_name` must be valid null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_meta_record_result(
    strategy_name: *const c_char, function_name: *const c_char,
    fitness_before: f64, fitness_after: f64, success: i32, cycle: u64,
) {
    let strategy = match unsafe { CStr::from_ptr(strategy_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    let func = match unsafe { CStr::from_ptr(function_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    with_meta(|m| {
        m.record_result(strategy, func, fitness_before, fitness_after, success != 0, cycle);
    });
}

/// Run a meta-evolution cycle. Returns JSON result.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_meta_evolve() -> *mut c_char {
    with_meta(|m| {
        let result = m.meta_evolve();
        alloc_cstring(&result.to_json())
    })
}

/// Get the strategy landscape as JSON.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_meta_landscape() -> *mut c_char {
    with_meta(|m| {
        alloc_cstring(&m.landscape_json())
    })
}

/// Get meta-evolution statistics as JSON.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_meta_stats() -> *mut c_char {
    with_meta(|m| {
        alloc_cstring(&m.stats_json())
    })
}

/// Get active strategy parameters as JSON.
/// Caller must free with `slang_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_meta_active_params() -> *mut c_char {
    with_meta(|m| {
        let params = m.active_params();
        let json = format!(
            concat!(
                "{{",
                "\"mutation_rate\":{:.3},",
                "\"explore_rate\":{:.3},",
                "\"risk_level\":{:.3},",
                "\"candidate_count\":{},",
                "\"fitness_threshold\":{:.3},",
                "\"use_crossover\":{},",
                "\"use_elitism\":{}",
                "}}"
            ),
            params.mutation_rate,
            params.explore_rate,
            params.risk_level,
            params.candidate_count,
            params.fitness_threshold,
            params.use_crossover,
            params.use_elitism,
        );
        alloc_cstring(&json)
    })
}

/// Get the number of strategies.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_meta_strategy_count() -> u64 {
    with_meta(|m| m.strategy_count() as u64)
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_ffi_compile_and_run() {
        let source = CString::new("fn main() -> i64 { 42 }").unwrap();
        let mut error: *mut c_char = std::ptr::null_mut();
        let result = unsafe { slang_compile_and_run(source.as_ptr(), &mut error) };
        assert_eq!(result, 42);
        assert!(error.is_null());
    }

    #[test]
    fn test_ffi_compile_error() {
        let source = CString::new("fn main(").unwrap();
        let mut error: *mut c_char = std::ptr::null_mut();
        let result = unsafe { slang_compile_and_run(source.as_ptr(), &mut error) };
        assert_eq!(result, i64::MIN);
        assert!(!error.is_null());
        unsafe { slang_free_error(error) };
    }

    #[test]
    fn test_ffi_version() {
        let ver = slang_version();
        assert!(!ver.is_null());
        let s = unsafe { CStr::from_ptr(ver) }.to_str().unwrap();
        assert_eq!(s, "22.0.0");
        unsafe { slang_free_string(ver) };
    }

    #[test]
    fn test_ffi_check_valid() {
        let source = CString::new("fn main() -> i64 { 42 }").unwrap();
        let result = unsafe { slang_check(source.as_ptr()) };
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "[]");
        unsafe { slang_free_string(result) };
    }

    #[test]
    fn test_ffi_lex() {
        let source = CString::new("fn main() { }").unwrap();
        let result = unsafe { slang_lex(source.as_ptr()) };
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert!(s.starts_with('['));
        assert!(s.contains("Fn"));
        unsafe { slang_free_string(result) };
    }

    // ── v15 FFI Bridge Tests ──────────────────────────────────────────

    #[test]
    fn test_v15_strings_bridge() {
        // str_len returns i64 directly
        let r = crate::codegen::compile_and_run(r#"fn main() -> i64 { str_len("hello") }"#);
        assert_eq!(r.unwrap(), 5);
        // str_contains returns bool — convert via if/else
        let r2 = crate::codegen::compile_and_run(
            r#"fn main() -> i64 { if str_contains("hello world", "world") { 1 } else { 0 } }"#,
        );
        assert_eq!(r2.unwrap(), 1);
    }

    #[test]
    fn test_v15_maps_bridge() {
        let r = crate::codegen::compile_and_run(
            r#"fn main() -> i64 { let m: i64 = map_new(); map_set(m, "key", 42); map_get(m, "key") }"#,
        );
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn test_v15_file_io_bridge() {
        let r = crate::codegen::compile_and_run(r#"fn main() -> i64 {
            file_write("_v15_bridge_test.tmp", "hello");
            let len: i64 = str_len(file_read("_v15_bridge_test.tmp"));
            file_delete("_v15_bridge_test.tmp");
            len
        }"#);
        assert_eq!(r.unwrap(), 5);
    }

    #[test]
    fn test_v15_error_handling_bridge() {
        // Clear any residual error state from parallel tests, then set + check
        let r = crate::codegen::compile_and_run(
            r#"fn main() -> i64 { error_clear(); error_set(99, "oops"); error_check() }"#,
        );
        assert_eq!(r.unwrap(), 99);
    }
}