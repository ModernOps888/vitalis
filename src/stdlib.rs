//! Vitalis Standard Library — Phase 1 built-in functions.
//!
//! Covers: typed I/O, math (f64 + i64), type conversions, string operations.
//! All functions are registered with the JIT compiler as extern symbols
//! and callable directly from Vitalis (.sl) source code.

use std::collections::HashMap;
use crate::ir::IrType;

/// Describes a built-in function for the compiler.
#[derive(Debug, Clone)]
pub struct BuiltinFn {
    pub name: String,
    pub params: Vec<(&'static str, IrType)>,
    pub ret: IrType,
    pub runtime_name: String,
}

/// Returns the full set of Phase 1 built-in functions.
pub fn builtins() -> Vec<BuiltinFn> {
    vec![
        // ── I/O ──────────────────────────────────────────────────────
        BuiltinFn { name: "print".into(),        params: vec![("value", IrType::I64)],  ret: IrType::Void, runtime_name: "slang_print_i64".into() },
        BuiltinFn { name: "println".into(),      params: vec![("value", IrType::I64)],  ret: IrType::Void, runtime_name: "slang_println_i64".into() },
        BuiltinFn { name: "print_f64".into(),    params: vec![("value", IrType::F64)],  ret: IrType::Void, runtime_name: "slang_print_f64".into() },
        BuiltinFn { name: "println_f64".into(),  params: vec![("value", IrType::F64)],  ret: IrType::Void, runtime_name: "slang_println_f64".into() },
        BuiltinFn { name: "print_bool".into(),   params: vec![("value", IrType::Bool)], ret: IrType::Void, runtime_name: "slang_print_bool".into() },
        BuiltinFn { name: "println_bool".into(), params: vec![("value", IrType::Bool)], ret: IrType::Void, runtime_name: "slang_println_bool".into() },
        BuiltinFn { name: "print_str".into(),    params: vec![("s", IrType::Ptr)],      ret: IrType::Void, runtime_name: "slang_print_cstr".into() },
        BuiltinFn { name: "println_str".into(),  params: vec![("s", IrType::Ptr)],      ret: IrType::Void, runtime_name: "slang_println_cstr".into() },

        // ── Math (f64) ────────────────────────────────────────────────
        BuiltinFn { name: "sqrt".into(),   params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sqrt_f64".into() },
        BuiltinFn { name: "ln".into(),     params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_ln_f64".into() },
        BuiltinFn { name: "log2".into(),   params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log2_f64".into() },
        BuiltinFn { name: "log10".into(),  params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log10_f64".into() },
        BuiltinFn { name: "sin".into(),    params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sin_f64".into() },
        BuiltinFn { name: "cos".into(),    params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_cos_f64".into() },
        BuiltinFn { name: "exp".into(),    params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_exp_f64".into() },
        BuiltinFn { name: "floor".into(),  params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_floor_f64".into() },
        BuiltinFn { name: "ceil".into(),   params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_ceil_f64".into() },
        BuiltinFn { name: "round".into(),  params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_round_f64".into() },
        BuiltinFn { name: "abs_f64".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_abs_f64".into() },
        BuiltinFn { name: "pow".into(),    params: vec![("base", IrType::F64), ("exp", IrType::F64)], ret: IrType::F64, runtime_name: "slang_pow_f64".into() },
        BuiltinFn { name: "min_f64".into(), params: vec![("a", IrType::F64), ("b", IrType::F64)], ret: IrType::F64, runtime_name: "slang_min_f64".into() },
        BuiltinFn { name: "max_f64".into(), params: vec![("a", IrType::F64), ("b", IrType::F64)], ret: IrType::F64, runtime_name: "slang_max_f64".into() },

        // ── Math (i64) ────────────────────────────────────────────────
        BuiltinFn { name: "abs".into(),    params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_abs_i64".into() },
        BuiltinFn { name: "min".into(),    params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_min_i64".into() },
        BuiltinFn { name: "max".into(),    params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_max_i64".into() },

        // ── Type conversions ──────────────────────────────────────────
        BuiltinFn { name: "to_f64".into(),    params: vec![("x", IrType::I64)], ret: IrType::F64, runtime_name: "slang_i64_to_f64".into() },
        BuiltinFn { name: "to_i64".into(),    params: vec![("x", IrType::F64)], ret: IrType::I64, runtime_name: "slang_f64_to_i64".into() },
        BuiltinFn { name: "i64_to_f64".into(), params: vec![("x", IrType::I64)], ret: IrType::F64, runtime_name: "slang_i64_to_f64".into() },
        BuiltinFn { name: "f64_to_i64".into(), params: vec![("x", IrType::F64)], ret: IrType::I64, runtime_name: "slang_f64_to_i64".into() },

        // ── String operations ─────────────────────────────────────────
        BuiltinFn { name: "str_len".into(), params: vec![("s", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_str_len".into() },
        BuiltinFn { name: "str_eq".into(),  params: vec![("a", IrType::Ptr), ("b", IrType::Ptr)], ret: IrType::Bool, runtime_name: "slang_str_eq".into() },
        BuiltinFn { name: "str_cat".into(), params: vec![("a", IrType::Ptr), ("b", IrType::Ptr)], ret: IrType::Ptr,  runtime_name: "slang_str_cat".into() },

        // ── Extended math ─────────────────────────────────────────────
        BuiltinFn { name: "atan2".into(),     params: vec![("y", IrType::F64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_atan2_f64".into() },
        BuiltinFn { name: "hypot".into(),     params: vec![("a", IrType::F64), ("b", IrType::F64)], ret: IrType::F64, runtime_name: "slang_hypot_f64".into() },
        BuiltinFn { name: "clamp_f64".into(), params: vec![("x", IrType::F64), ("lo", IrType::F64), ("hi", IrType::F64)], ret: IrType::F64, runtime_name: "slang_clamp_f64".into() },
        BuiltinFn { name: "clamp_i64".into(), params: vec![("x", IrType::I64), ("lo", IrType::I64), ("hi", IrType::I64)], ret: IrType::I64, runtime_name: "slang_clamp_i64".into() },
        BuiltinFn { name: "clamp".into(),     params: vec![("x", IrType::F64), ("lo", IrType::F64), ("hi", IrType::F64)], ret: IrType::F64, runtime_name: "slang_clamp_f64".into() },

        // ── Randomness (Xorshift64, no deps) ─────────────────────────
        BuiltinFn { name: "rand_f64".into(), params: vec![], ret: IrType::F64, runtime_name: "slang_rand_f64".into() },
        BuiltinFn { name: "rand_i64".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_rand_i64".into() },

        // ── Time ──────────────────────────────────────────────────────
        BuiltinFn { name: "clock_ns".into(),  params: vec![], ret: IrType::I64, runtime_name: "slang_clock_ns".into() },
        BuiltinFn { name: "clock_ms".into(),  params: vec![], ret: IrType::I64, runtime_name: "slang_clock_ms".into() },

        // ── Assertions (for test / debugging) ─────────────────────────
        BuiltinFn { name: "assert_eq".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::Void, runtime_name: "slang_assert_eq_i64".into() },
        BuiltinFn { name: "assert_true".into(), params: vec![("cond", IrType::Bool)], ret: IrType::Void, runtime_name: "slang_assert_true".into() },

        // ── Bitwise operations ────────────────────────────────────────
        BuiltinFn { name: "popcount".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_popcount".into() },
        BuiltinFn { name: "leading_zeros".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_leading_zeros".into() },
        BuiltinFn { name: "trailing_zeros".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_trailing_zeros".into() },

        // ── Extended math ─────────────────────────────────────────────
        BuiltinFn { name: "sign".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_sign_i64".into() },
        BuiltinFn { name: "gcd".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gcd".into() },
        BuiltinFn { name: "lcm".into(), params: vec![("a", IrType::I64), ("b", IrType::I64)], ret: IrType::I64, runtime_name: "slang_lcm".into() },
        BuiltinFn { name: "factorial".into(), params: vec![("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_factorial".into() },
        BuiltinFn { name: "fibonacci".into(), params: vec![("n", IrType::I64)], ret: IrType::I64, runtime_name: "slang_fibonacci".into() },
        BuiltinFn { name: "is_prime".into(), params: vec![("n", IrType::I64)], ret: IrType::Bool, runtime_name: "slang_is_prime".into() },
        BuiltinFn { name: "tan".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_tan_f64".into() },
        BuiltinFn { name: "asin".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_asin_f64".into() },
        BuiltinFn { name: "acos".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_acos_f64".into() },
        BuiltinFn { name: "atan".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_atan_f64".into() },

        // ── Hash & identity ───────────────────────────────────────────
        BuiltinFn { name: "hash".into(), params: vec![("x", IrType::I64)], ret: IrType::I64, runtime_name: "slang_hash_i64".into() },

        // ── Numeric utils ─────────────────────────────────────────────
        BuiltinFn { name: "lerp".into(), params: vec![("a", IrType::F64), ("b", IrType::F64), ("t", IrType::F64)], ret: IrType::F64, runtime_name: "slang_lerp_f64".into() },
        BuiltinFn { name: "smoothstep".into(), params: vec![("edge0", IrType::F64), ("edge1", IrType::F64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_smoothstep_f64".into() },
        BuiltinFn { name: "wrap".into(), params: vec![("x", IrType::I64), ("lo", IrType::I64), ("hi", IrType::I64)], ret: IrType::I64, runtime_name: "slang_wrap_i64".into() },
        BuiltinFn { name: "map_range".into(), params: vec![("x", IrType::F64), ("in_lo", IrType::F64), ("in_hi", IrType::F64), ("out_lo", IrType::F64), ("out_hi", IrType::F64)], ret: IrType::F64, runtime_name: "slang_map_range_f64".into() },

        // ── Epoch timestamp ───────────────────────────────────────────
        BuiltinFn { name: "epoch_secs".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_epoch_secs".into() },

        // ── Phase 22: AI & Numeric functions ──────────────────────────
        BuiltinFn { name: "fma".into(), params: vec![("a", IrType::F64), ("b", IrType::F64), ("c", IrType::F64)], ret: IrType::F64, runtime_name: "slang_fma_f64".into() },
        BuiltinFn { name: "cbrt".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_cbrt_f64".into() },
        BuiltinFn { name: "deg_to_rad".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_deg_to_rad".into() },
        BuiltinFn { name: "rad_to_deg".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_rad_to_deg".into() },
        BuiltinFn { name: "sigmoid".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sigmoid_f64".into() },
        BuiltinFn { name: "relu".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_relu_f64".into() },
        BuiltinFn { name: "tanh".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_tanh_f64".into() },
        BuiltinFn { name: "ipow".into(), params: vec![("base", IrType::I64), ("exp", IrType::I64)], ret: IrType::I64, runtime_name: "slang_ipow".into() },

        // ── Phase 23: Extended math & AI activations ───────────────────
        BuiltinFn { name: "sinh".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sinh_f64".into() },
        BuiltinFn { name: "cosh".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_cosh_f64".into() },
        BuiltinFn { name: "log".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log_f64".into() },
        BuiltinFn { name: "exp2".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_exp2_f64".into() },
        BuiltinFn { name: "copysign".into(), params: vec![("x", IrType::F64), ("y", IrType::F64)], ret: IrType::F64, runtime_name: "slang_copysign_f64".into() },
        BuiltinFn { name: "fract".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_fract_f64".into() },
        BuiltinFn { name: "trunc".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_trunc_f64".into() },
        BuiltinFn { name: "step".into(), params: vec![("edge", IrType::F64), ("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_step_f64".into() },
        BuiltinFn { name: "leaky_relu".into(), params: vec![("x", IrType::F64), ("alpha", IrType::F64)], ret: IrType::F64, runtime_name: "slang_leaky_relu_f64".into() },
        BuiltinFn { name: "elu".into(), params: vec![("x", IrType::F64), ("alpha", IrType::F64)], ret: IrType::F64, runtime_name: "slang_elu_f64".into() },

        // ── Phase 24: Advanced AI activations & math ──────────────────
        BuiltinFn { name: "swish".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_swish_f64".into() },
        BuiltinFn { name: "gelu".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_gelu_f64".into() },
        BuiltinFn { name: "softplus".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_softplus_f64".into() },
        BuiltinFn { name: "mish".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_mish_f64".into() },
        BuiltinFn { name: "log1p".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log1p_f64".into() },
        BuiltinFn { name: "expm1".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_expm1_f64".into() },
        BuiltinFn { name: "recip".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_recip_f64".into() },
        BuiltinFn { name: "rsqrt".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_rsqrt_f64".into() },

        // ── Phase 25: Numerical & advanced activations ────────────────
        BuiltinFn { name: "selu".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_selu_f64".into() },
        BuiltinFn { name: "hard_sigmoid".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_hard_sigmoid_f64".into() },
        BuiltinFn { name: "hard_swish".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_hard_swish_f64".into() },
        BuiltinFn { name: "log_sigmoid".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_log_sigmoid_f64".into() },
        BuiltinFn { name: "celu".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_celu_f64".into() },
        BuiltinFn { name: "softsign".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_softsign_f64".into() },
        BuiltinFn { name: "gaussian".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_gaussian_f64".into() },
        BuiltinFn { name: "sinc".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_sinc_f64".into() },
        BuiltinFn { name: "inv_sqrt_approx".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_inv_sqrt_approx_f64".into() },
        BuiltinFn { name: "logit".into(), params: vec![("x", IrType::F64)], ret: IrType::F64, runtime_name: "slang_logit_f64".into() },

        // ── v15: String operations ────────────────────────────────────
        BuiltinFn { name: "str_upper".into(),       params: vec![("s", IrType::Ptr)],                                              ret: IrType::Ptr,  runtime_name: "slang_str_upper".into() },
        BuiltinFn { name: "str_lower".into(),       params: vec![("s", IrType::Ptr)],                                              ret: IrType::Ptr,  runtime_name: "slang_str_lower".into() },
        BuiltinFn { name: "str_trim".into(),        params: vec![("s", IrType::Ptr)],                                              ret: IrType::Ptr,  runtime_name: "slang_str_trim".into() },
        BuiltinFn { name: "str_contains".into(),    params: vec![("s", IrType::Ptr), ("sub", IrType::Ptr)],                        ret: IrType::Bool, runtime_name: "slang_str_contains".into() },
        BuiltinFn { name: "str_starts_with".into(), params: vec![("s", IrType::Ptr), ("pfx", IrType::Ptr)],                        ret: IrType::Bool, runtime_name: "slang_str_starts_with".into() },
        BuiltinFn { name: "str_ends_with".into(),   params: vec![("s", IrType::Ptr), ("sfx", IrType::Ptr)],                        ret: IrType::Bool, runtime_name: "slang_str_ends_with".into() },
        BuiltinFn { name: "str_char_at".into(),     params: vec![("s", IrType::Ptr), ("i", IrType::I64)],                          ret: IrType::Ptr,  runtime_name: "slang_str_char_at".into() },
        BuiltinFn { name: "str_substr".into(),      params: vec![("s", IrType::Ptr), ("start", IrType::I64), ("len", IrType::I64)], ret: IrType::Ptr,  runtime_name: "slang_str_substr".into() },
        BuiltinFn { name: "str_index_of".into(),    params: vec![("s", IrType::Ptr), ("sub", IrType::Ptr)],                        ret: IrType::I64,  runtime_name: "slang_str_index_of".into() },
        BuiltinFn { name: "str_replace".into(),     params: vec![("s", IrType::Ptr), ("old", IrType::Ptr), ("new", IrType::Ptr)],  ret: IrType::Ptr,  runtime_name: "slang_str_replace".into() },
        BuiltinFn { name: "str_repeat".into(),      params: vec![("s", IrType::Ptr), ("n", IrType::I64)],                          ret: IrType::Ptr,  runtime_name: "slang_str_repeat".into() },
        BuiltinFn { name: "str_reverse".into(),     params: vec![("s", IrType::Ptr)],                                              ret: IrType::Ptr,  runtime_name: "slang_str_reverse".into() },
        BuiltinFn { name: "str_split_count".into(), params: vec![("s", IrType::Ptr), ("delim", IrType::Ptr)],                      ret: IrType::I64,  runtime_name: "slang_str_split_count".into() },
        BuiltinFn { name: "str_split_get".into(),   params: vec![("s", IrType::Ptr), ("delim", IrType::Ptr), ("i", IrType::I64)],  ret: IrType::Ptr,  runtime_name: "slang_str_split_get".into() },
        BuiltinFn { name: "to_string_i64".into(),   params: vec![("x", IrType::I64)],                                              ret: IrType::Ptr,  runtime_name: "slang_to_string_i64".into() },
        BuiltinFn { name: "to_string_f64".into(),   params: vec![("x", IrType::F64)],                                              ret: IrType::Ptr,  runtime_name: "slang_to_string_f64".into() },
        BuiltinFn { name: "to_string_bool".into(),  params: vec![("x", IrType::Bool)],                                             ret: IrType::Ptr,  runtime_name: "slang_to_string_bool".into() },
        BuiltinFn { name: "str_format_i64".into(),  params: vec![("fmt", IrType::Ptr), ("val", IrType::I64)],                       ret: IrType::Ptr,  runtime_name: "slang_str_format_i64".into() },
        BuiltinFn { name: "str_format_f64".into(),  params: vec![("fmt", IrType::Ptr), ("val", IrType::F64)],                       ret: IrType::Ptr,  runtime_name: "slang_str_format_f64".into() },
        BuiltinFn { name: "str_format_str".into(),  params: vec![("fmt", IrType::Ptr), ("val", IrType::Ptr)],                       ret: IrType::Ptr,  runtime_name: "slang_str_format_str".into() },
        BuiltinFn { name: "parse_int".into(),       params: vec![("s", IrType::Ptr)],                                              ret: IrType::I64,  runtime_name: "slang_parse_int".into() },
        BuiltinFn { name: "parse_float".into(),     params: vec![("s", IrType::Ptr)],                                              ret: IrType::F64,  runtime_name: "slang_parse_float".into() },

        // ── v15: File I/O ─────────────────────────────────────────────
        BuiltinFn { name: "file_read".into(),       params: vec![("path", IrType::Ptr)],                              ret: IrType::Ptr,  runtime_name: "slang_file_read".into() },
        BuiltinFn { name: "file_write".into(),      params: vec![("path", IrType::Ptr), ("content", IrType::Ptr)],    ret: IrType::Bool, runtime_name: "slang_file_write".into() },
        BuiltinFn { name: "file_append".into(),     params: vec![("path", IrType::Ptr), ("content", IrType::Ptr)],    ret: IrType::Bool, runtime_name: "slang_file_append".into() },
        BuiltinFn { name: "file_exists".into(),     params: vec![("path", IrType::Ptr)],                              ret: IrType::Bool, runtime_name: "slang_file_exists".into() },
        BuiltinFn { name: "file_delete".into(),     params: vec![("path", IrType::Ptr)],                              ret: IrType::Bool, runtime_name: "slang_file_delete".into() },
        BuiltinFn { name: "file_size".into(),       params: vec![("path", IrType::Ptr)],                              ret: IrType::I64,  runtime_name: "slang_file_size".into() },

        // ── v15: Map operations ───────────────────────────────────────
        BuiltinFn { name: "map_new".into(),         params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_map_new".into() },
        BuiltinFn { name: "map_set".into(),         params: vec![("m", IrType::I64), ("k", IrType::Ptr), ("v", IrType::I64)], ret: IrType::Void, runtime_name: "slang_map_set".into() },
        BuiltinFn { name: "map_get".into(),         params: vec![("m", IrType::I64), ("k", IrType::Ptr)],             ret: IrType::I64,  runtime_name: "slang_map_get".into() },
        BuiltinFn { name: "map_has".into(),         params: vec![("m", IrType::I64), ("k", IrType::Ptr)],             ret: IrType::Bool, runtime_name: "slang_map_has".into() },
        BuiltinFn { name: "map_remove".into(),      params: vec![("m", IrType::I64), ("k", IrType::Ptr)],             ret: IrType::Void, runtime_name: "slang_map_remove".into() },
        BuiltinFn { name: "map_len".into(),         params: vec![("m", IrType::I64)],                                 ret: IrType::I64,  runtime_name: "slang_map_len".into() },
        BuiltinFn { name: "map_keys".into(),        params: vec![("m", IrType::I64)],                                 ret: IrType::Ptr,  runtime_name: "slang_map_keys".into() },

        // ── v16: Set operations ───────────────────────────────────────
        BuiltinFn { name: "set_new".into(),         params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_set_new".into() },
        BuiltinFn { name: "set_add".into(),         params: vec![("s", IrType::I64), ("v", IrType::I64)],             ret: IrType::Void, runtime_name: "slang_set_add".into() },
        BuiltinFn { name: "set_has".into(),         params: vec![("s", IrType::I64), ("v", IrType::I64)],             ret: IrType::Bool, runtime_name: "slang_set_has".into() },
        BuiltinFn { name: "set_remove".into(),      params: vec![("s", IrType::I64), ("v", IrType::I64)],             ret: IrType::Void, runtime_name: "slang_set_remove".into() },
        BuiltinFn { name: "set_len".into(),         params: vec![("s", IrType::I64)],                                 ret: IrType::I64,  runtime_name: "slang_set_len".into() },
        BuiltinFn { name: "set_union".into(),       params: vec![("a", IrType::I64), ("b", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_set_union".into() },
        BuiltinFn { name: "set_intersect".into(),   params: vec![("a", IrType::I64), ("b", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_set_intersect".into() },
        BuiltinFn { name: "set_diff".into(),        params: vec![("a", IrType::I64), ("b", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_set_diff".into() },
        BuiltinFn { name: "set_to_array".into(),    params: vec![("s", IrType::I64)],                                 ret: IrType::Ptr,  runtime_name: "slang_set_to_array".into() },

        // ── v18: Tuple operations ─────────────────────────────────────
        BuiltinFn { name: "tuple_new2".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_tuple_new2".into() },
        BuiltinFn { name: "tuple_new3".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64), ("c", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tuple_new3".into() },
        BuiltinFn { name: "tuple_new4".into(),     params: vec![("a", IrType::I64), ("b", IrType::I64), ("c", IrType::I64), ("d", IrType::I64)], ret: IrType::I64, runtime_name: "slang_tuple_new4".into() },
        BuiltinFn { name: "tuple_get".into(),      params: vec![("t", IrType::I64), ("idx", IrType::I64)],           ret: IrType::I64,  runtime_name: "slang_tuple_get".into() },
        BuiltinFn { name: "tuple_len".into(),      params: vec![("t", IrType::I64)],                                 ret: IrType::I64,  runtime_name: "slang_tuple_len".into() },

        // ── v15: Error handling ───────────────────────────────────────
        BuiltinFn { name: "error_set".into(),       params: vec![("code", IrType::I64), ("msg", IrType::Ptr)],        ret: IrType::Void, runtime_name: "slang_error_set".into() },
        BuiltinFn { name: "error_check".into(),     params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_error_check".into() },
        BuiltinFn { name: "error_msg".into(),       params: vec![],                                                   ret: IrType::Ptr,  runtime_name: "slang_error_msg".into() },
        BuiltinFn { name: "error_clear".into(),     params: vec![],                                                   ret: IrType::Void, runtime_name: "slang_error_clear".into() },

        // ── v15: Environment & System ─────────────────────────────────
        BuiltinFn { name: "env_get".into(),         params: vec![("key", IrType::Ptr)],                               ret: IrType::Ptr,  runtime_name: "slang_env_get".into() },
        BuiltinFn { name: "sleep_ms".into(),        params: vec![("ms", IrType::I64)],                                ret: IrType::Void, runtime_name: "slang_sleep_ms".into() },
        BuiltinFn { name: "eprint".into(),          params: vec![("s", IrType::Ptr)],                                 ret: IrType::Void, runtime_name: "slang_eprint".into() },
        BuiltinFn { name: "eprintln".into(),        params: vec![("s", IrType::Ptr)],                                 ret: IrType::Void, runtime_name: "slang_eprintln".into() },
        BuiltinFn { name: "pid".into(),             params: vec![],                                                   ret: IrType::I64,  runtime_name: "slang_pid".into() },
        BuiltinFn { name: "format_int".into(),      params: vec![("fmt", IrType::Ptr), ("val", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_format_int".into() },
        BuiltinFn { name: "format_float".into(),    params: vec![("fmt", IrType::Ptr), ("val", IrType::F64)],         ret: IrType::Ptr,  runtime_name: "slang_format_float".into() },

        // ── v15: JSON ─────────────────────────────────────────────────
        BuiltinFn { name: "json_encode".into(),     params: vec![("m", IrType::I64)],                                 ret: IrType::Ptr,  runtime_name: "slang_json_encode".into() },
        BuiltinFn { name: "json_decode".into(),     params: vec![("s", IrType::Ptr)],                                 ret: IrType::I64,  runtime_name: "slang_json_decode".into() },

        // ── v18: Collection methods ───────────────────────────────────
        BuiltinFn { name: "array_push".into(),      params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_array_push".into() },
        BuiltinFn { name: "array_pop".into(),       params: vec![("arr", IrType::Ptr)],                               ret: IrType::I64,  runtime_name: "slang_array_pop".into() },
        BuiltinFn { name: "array_contains".into(),  params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],         ret: IrType::Bool, runtime_name: "slang_array_contains".into() },
        BuiltinFn { name: "array_reverse".into(),   params: vec![("arr", IrType::Ptr)],                               ret: IrType::Ptr,  runtime_name: "slang_array_reverse".into() },
        BuiltinFn { name: "array_sort".into(),      params: vec![("arr", IrType::Ptr)],                               ret: IrType::Ptr,  runtime_name: "slang_array_sort".into() },
        BuiltinFn { name: "array_join".into(),      params: vec![("arr", IrType::Ptr), ("delim", IrType::Ptr)],       ret: IrType::Ptr,  runtime_name: "slang_array_join".into() },
        BuiltinFn { name: "array_slice".into(),     params: vec![("arr", IrType::Ptr), ("start", IrType::I64), ("end", IrType::I64)], ret: IrType::Ptr, runtime_name: "slang_array_slice".into() },
        BuiltinFn { name: "array_find".into(),      params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],         ret: IrType::I64,  runtime_name: "slang_array_find".into() },
        // ── Iterator / functional array ops ────────────────────────
        BuiltinFn { name: "array_range".into(),        params: vec![("start", IrType::I64), ("end", IrType::I64)],     ret: IrType::Ptr,  runtime_name: "slang_array_range".into() },
        BuiltinFn { name: "array_sum".into(),          params: vec![("arr", IrType::Ptr)],                             ret: IrType::I64,  runtime_name: "slang_array_sum".into() },
        BuiltinFn { name: "array_min".into(),          params: vec![("arr", IrType::Ptr)],                             ret: IrType::I64,  runtime_name: "slang_array_min".into() },
        BuiltinFn { name: "array_max".into(),          params: vec![("arr", IrType::Ptr)],                             ret: IrType::I64,  runtime_name: "slang_array_max".into() },
        BuiltinFn { name: "array_any".into(),          params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],       ret: IrType::Bool, runtime_name: "slang_array_any".into() },
        BuiltinFn { name: "array_all_positive".into(), params: vec![("arr", IrType::Ptr)],                             ret: IrType::Bool, runtime_name: "slang_array_all_positive".into() },
        BuiltinFn { name: "array_count".into(),        params: vec![("arr", IrType::Ptr), ("val", IrType::I64)],       ret: IrType::I64,  runtime_name: "slang_array_count".into() },
        BuiltinFn { name: "array_flatten".into(),      params: vec![("arr", IrType::Ptr)],                             ret: IrType::Ptr,  runtime_name: "slang_array_flatten".into() },
        BuiltinFn { name: "array_zip".into(),          params: vec![("a", IrType::Ptr), ("b", IrType::Ptr)],           ret: IrType::Ptr,  runtime_name: "slang_array_zip".into() },
        BuiltinFn { name: "array_enumerate".into(),    params: vec![("arr", IrType::Ptr)],                             ret: IrType::Ptr,  runtime_name: "slang_array_enumerate".into() },
        BuiltinFn { name: "array_take".into(),         params: vec![("arr", IrType::Ptr), ("n", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_array_take".into() },
        BuiltinFn { name: "array_drop".into(),         params: vec![("arr", IrType::Ptr), ("n", IrType::I64)],         ret: IrType::Ptr,  runtime_name: "slang_array_drop".into() },
        BuiltinFn { name: "array_unique".into(),       params: vec![("arr", IrType::Ptr)],                             ret: IrType::Ptr,  runtime_name: "slang_array_unique".into() },
        BuiltinFn { name: "error_message".into(),   params: vec![],                                                   ret: IrType::Ptr,  runtime_name: "slang_error_message".into() },

        // ── v18: Regex ────────────────────────────────────────────────
        BuiltinFn { name: "regex_match".into(),          params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::Bool, runtime_name: "slang_regex_match".into() },
        BuiltinFn { name: "regex_is_match".into(),       params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::Bool, runtime_name: "slang_regex_is_match".into() },
        BuiltinFn { name: "regex_find".into(),           params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::Ptr,  runtime_name: "slang_regex_find".into() },
        BuiltinFn { name: "regex_replace".into(),        params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("replacement", IrType::Ptr)],  ret: IrType::Ptr,  runtime_name: "slang_regex_replace".into() },
        BuiltinFn { name: "regex_split_count".into(),    params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::I64,  runtime_name: "slang_regex_split_count".into() },
        BuiltinFn { name: "regex_split_get".into(),      params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("idx", IrType::I64)],          ret: IrType::Ptr,  runtime_name: "slang_regex_split_get".into() },
        BuiltinFn { name: "regex_find_all_count".into(), params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                                ret: IrType::I64,  runtime_name: "slang_regex_find_all_count".into() },
        BuiltinFn { name: "regex_find_all_get".into(),   params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("idx", IrType::I64)],          ret: IrType::Ptr,  runtime_name: "slang_regex_find_all_get".into() },

        // ── v18: Async stubs ──────────────────────────────────────────
        BuiltinFn { name: "spawn".into(),         params: vec![("task_id", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "slang_spawn".into() },
        BuiltinFn { name: "task_result".into(),   params: vec![("task_id", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "slang_task_result".into() },

        // ── v18: Networking ───────────────────────────────────────────
        BuiltinFn { name: "http_get".into(),      params: vec![("url", IrType::Ptr)],                                                  ret: IrType::Ptr,  runtime_name: "slang_http_get".into() },
        BuiltinFn { name: "http_post".into(),     params: vec![("url", IrType::Ptr), ("body", IrType::Ptr)],                           ret: IrType::Ptr,  runtime_name: "slang_http_post".into() },
        BuiltinFn { name: "http_status".into(),   params: vec![("url", IrType::Ptr)],                                                  ret: IrType::I64,  runtime_name: "slang_http_status".into() },
        BuiltinFn { name: "tcp_connect".into(),   params: vec![("host", IrType::Ptr), ("port", IrType::I64)],                          ret: IrType::I64,  runtime_name: "slang_tcp_connect".into() },
        BuiltinFn { name: "tcp_send".into(),      params: vec![("handle", IrType::I64), ("data", IrType::Ptr)],                        ret: IrType::I64,  runtime_name: "slang_tcp_send".into() },
        BuiltinFn { name: "tcp_close".into(),     params: vec![("handle", IrType::I64)],                                               ret: IrType::Void, runtime_name: "slang_tcp_close".into() },
    ]
}

/// Returns a mapping of user-visible names → runtime symbol names.
pub fn builtin_aliases() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for b in builtins() {
        map.insert(b.name, b.runtime_name);
    }
    map
}
