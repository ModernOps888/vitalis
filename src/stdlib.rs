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

        // ── v28: Graphics & Visualization ──────────────────────────────
        BuiltinFn { name: "gfx_create_canvas".into(),  params: vec![("width", IrType::I64), ("height", IrType::I64)],                ret: IrType::I64,  runtime_name: "slang_gfx_create_canvas".into() },
        BuiltinFn { name: "gfx_draw_rect".into(),      params: vec![("x", IrType::I64), ("y", IrType::I64), ("w", IrType::I64), ("h", IrType::I64)], ret: IrType::Void, runtime_name: "slang_gfx_draw_rect".into() },
        BuiltinFn { name: "gfx_draw_circle".into(),    params: vec![("cx", IrType::I64), ("cy", IrType::I64), ("r", IrType::I64)], ret: IrType::Void, runtime_name: "slang_gfx_draw_circle".into() },
        BuiltinFn { name: "gfx_draw_line".into(),      params: vec![("x1", IrType::I64), ("y1", IrType::I64), ("x2", IrType::I64), ("y2", IrType::I64)], ret: IrType::Void, runtime_name: "slang_gfx_draw_line".into() },
        BuiltinFn { name: "gfx_set_color".into(),      params: vec![("r", IrType::I64), ("g", IrType::I64), ("b", IrType::I64), ("a", IrType::I64)], ret: IrType::Void, runtime_name: "slang_gfx_set_color".into() },
        BuiltinFn { name: "gfx_fill".into(),            params: vec![("r", IrType::I64), ("g", IrType::I64), ("b", IrType::I64)],   ret: IrType::Void, runtime_name: "slang_gfx_fill".into() },
        BuiltinFn { name: "gfx_stroke".into(),          params: vec![("r", IrType::I64), ("g", IrType::I64), ("b", IrType::I64)],   ret: IrType::Void, runtime_name: "slang_gfx_stroke".into() },
        BuiltinFn { name: "gfx_stroke_weight".into(),   params: vec![("weight", IrType::I64)],                                        ret: IrType::Void, runtime_name: "slang_gfx_stroke_weight".into() },
        BuiltinFn { name: "gfx_translate".into(),       params: vec![("x", IrType::I64), ("y", IrType::I64)],                         ret: IrType::Void, runtime_name: "slang_gfx_translate".into() },
        BuiltinFn { name: "gfx_rotate".into(),          params: vec![("angle", IrType::I64)],                                          ret: IrType::Void, runtime_name: "slang_gfx_rotate".into() },
        BuiltinFn { name: "gfx_scale".into(),           params: vec![("sx", IrType::I64), ("sy", IrType::I64)],                       ret: IrType::Void, runtime_name: "slang_gfx_scale".into() },
        BuiltinFn { name: "gfx_to_svg".into(),          params: vec![("canvas", IrType::I64)],                                         ret: IrType::Ptr,  runtime_name: "slang_gfx_to_svg".into() },
        BuiltinFn { name: "chart_pie".into(),            params: vec![("data", IrType::Ptr), ("title", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_chart_pie".into() },
        BuiltinFn { name: "chart_bar".into(),            params: vec![("data", IrType::Ptr), ("title", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_chart_bar".into() },
        BuiltinFn { name: "chart_line".into(),           params: vec![("data", IrType::Ptr), ("title", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_chart_line".into() },
        BuiltinFn { name: "chart_scatter".into(),        params: vec![("data", IrType::Ptr), ("title", IrType::Ptr)],                  ret: IrType::Ptr,  runtime_name: "slang_chart_scatter".into() },
        BuiltinFn { name: "chart_histogram".into(),      params: vec![("data", IrType::Ptr), ("bins", IrType::I64)],                   ret: IrType::Ptr,  runtime_name: "slang_chart_histogram".into() },
        BuiltinFn { name: "shader_compile".into(),       params: vec![("source", IrType::Ptr), ("backend", IrType::Ptr)],              ret: IrType::Ptr,  runtime_name: "slang_shader_compile".into() },
        BuiltinFn { name: "gui_create_window".into(),    params: vec![("title", IrType::Ptr), ("w", IrType::I64), ("h", IrType::I64)], ret: IrType::I64, runtime_name: "slang_gui_create_window".into() },
        BuiltinFn { name: "gui_add_button".into(),       params: vec![("label", IrType::Ptr)],                                          ret: IrType::I64,  runtime_name: "slang_gui_add_button".into() },
        BuiltinFn { name: "gui_add_text".into(),         params: vec![("content", IrType::Ptr)],                                        ret: IrType::I64,  runtime_name: "slang_gui_add_text".into() },
        BuiltinFn { name: "gui_add_slider".into(),       params: vec![("min", IrType::I64), ("max", IrType::I64)],                     ret: IrType::I64,  runtime_name: "slang_gui_add_slider".into() },
        BuiltinFn { name: "gui_set_theme".into(),        params: vec![("theme", IrType::Ptr)],                                          ret: IrType::Void, runtime_name: "slang_gui_set_theme".into() },
        BuiltinFn { name: "noise_perlin".into(),         params: vec![("x", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "slang_noise_perlin".into() },
        BuiltinFn { name: "noise_perlin2d".into(),       params: vec![("x", IrType::I64), ("y", IrType::I64)],                         ret: IrType::I64,  runtime_name: "slang_noise_perlin2d".into() },
        BuiltinFn { name: "particle_create".into(),      params: vec![("x", IrType::I64), ("y", IrType::I64)],                         ret: IrType::I64,  runtime_name: "slang_particle_create".into() },
        BuiltinFn { name: "node_graph_create".into(),    params: vec![("name", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "slang_node_graph_create".into() },
        BuiltinFn { name: "node_graph_add_node".into(),  params: vec![("graph", IrType::I64), ("name", IrType::Ptr), ("op", IrType::Ptr)], ret: IrType::I64, runtime_name: "slang_node_graph_add_node".into() },
        BuiltinFn { name: "node_graph_connect".into(),   params: vec![("graph", IrType::I64), ("from", IrType::I64), ("to", IrType::I64)], ret: IrType::I64, runtime_name: "slang_node_graph_connect".into() },
        BuiltinFn { name: "node_graph_evaluate".into(),  params: vec![("graph", IrType::I64)],                                          ret: IrType::Ptr,  runtime_name: "slang_node_graph_evaluate".into() },

        // ── v18: Networking ───────────────────────────────────────────
        BuiltinFn { name: "http_get".into(),      params: vec![("url", IrType::Ptr)],                                                  ret: IrType::Ptr,  runtime_name: "slang_http_get".into() },
        BuiltinFn { name: "http_post".into(),     params: vec![("url", IrType::Ptr), ("body", IrType::Ptr)],                           ret: IrType::Ptr,  runtime_name: "slang_http_post".into() },
        BuiltinFn { name: "http_status".into(),   params: vec![("url", IrType::Ptr)],                                                  ret: IrType::I64,  runtime_name: "slang_http_status".into() },
        BuiltinFn { name: "tcp_connect".into(),   params: vec![("host", IrType::Ptr), ("port", IrType::I64)],                          ret: IrType::I64,  runtime_name: "slang_tcp_connect".into() },
        BuiltinFn { name: "tcp_send".into(),      params: vec![("handle", IrType::I64), ("data", IrType::Ptr)],                        ret: IrType::I64,  runtime_name: "slang_tcp_send".into() },
        BuiltinFn { name: "tcp_close".into(),     params: vec![("handle", IrType::I64)],                                               ret: IrType::Void, runtime_name: "slang_tcp_close".into() },

        // ── v29: Profiler & PGO ──────────────────────────────────────────────
        BuiltinFn { name: "profiler_start".into(),        params: vec![("name", IrType::Ptr)],                                           ret: IrType::Void, runtime_name: "slang_profiler_start".into() },
        BuiltinFn { name: "profiler_stop".into(),         params: vec![("name", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "slang_profiler_stop".into() },
        BuiltinFn { name: "profiler_report".into(),       params: vec![],                                                                 ret: IrType::Ptr,  runtime_name: "slang_profiler_report".into() },
        BuiltinFn { name: "profiler_flamegraph".into(),   params: vec![],                                                                 ret: IrType::Ptr,  runtime_name: "slang_profiler_flamegraph".into() },
        BuiltinFn { name: "profiler_hotpath".into(),      params: vec![("threshold", IrType::F64)],                                      ret: IrType::Ptr,  runtime_name: "slang_profiler_hotpath".into() },

        // ── v29: Memory Pools ────────────────────────────────────────────────
        BuiltinFn { name: "arena_create".into(),          params: vec![("capacity", IrType::I64)],                                       ret: IrType::I64,  runtime_name: "slang_arena_create".into() },
        BuiltinFn { name: "arena_alloc".into(),           params: vec![("arena", IrType::I64), ("size", IrType::I64)],                   ret: IrType::I64,  runtime_name: "slang_arena_alloc".into() },
        BuiltinFn { name: "arena_reset".into(),           params: vec![("arena", IrType::I64)],                                          ret: IrType::Void, runtime_name: "slang_arena_reset".into() },
        BuiltinFn { name: "pool_create".into(),           params: vec![("block_size", IrType::I64), ("count", IrType::I64)],             ret: IrType::I64,  runtime_name: "slang_pool_create".into() },
        BuiltinFn { name: "pool_alloc".into(),            params: vec![("pool", IrType::I64)],                                           ret: IrType::I64,  runtime_name: "slang_pool_alloc".into() },
        BuiltinFn { name: "pool_free".into(),             params: vec![("pool", IrType::I64), ("ptr", IrType::I64)],                     ret: IrType::Void, runtime_name: "slang_pool_free".into() },

        // ── v29: FFI Bindgen ─────────────────────────────────────────────────
        BuiltinFn { name: "ffi_type_size".into(),         params: vec![("type_name", IrType::Ptr)],                                      ret: IrType::I64,  runtime_name: "slang_ffi_type_size".into() },
        BuiltinFn { name: "ffi_type_align".into(),        params: vec![("type_name", IrType::Ptr)],                                      ret: IrType::I64,  runtime_name: "slang_ffi_type_align".into() },
        BuiltinFn { name: "ffi_gen_header".into(),        params: vec![("module", IrType::Ptr)],                                         ret: IrType::Ptr,  runtime_name: "slang_ffi_gen_header".into() },
        BuiltinFn { name: "ffi_gen_typescript".into(),    params: vec![("module", IrType::Ptr)],                                         ret: IrType::Ptr,  runtime_name: "slang_ffi_gen_typescript".into() },

        // ── v29: Type Classes ────────────────────────────────────────────────
        BuiltinFn { name: "kind_check".into(),            params: vec![("type_expr", IrType::Ptr)],                                      ret: IrType::Ptr,  runtime_name: "slang_kind_check".into() },
        BuiltinFn { name: "resolve_instance".into(),      params: vec![("class", IrType::Ptr), ("type_arg", IrType::Ptr)],               ret: IrType::Ptr,  runtime_name: "slang_resolve_instance".into() },

        // ── v29: Build System ────────────────────────────────────────────────
        BuiltinFn { name: "build_graph_create".into(),    params: vec![],                                                                 ret: IrType::I64,  runtime_name: "slang_build_graph_create".into() },
        BuiltinFn { name: "build_add_unit".into(),        params: vec![("graph", IrType::I64), ("name", IrType::Ptr)],                   ret: IrType::I64,  runtime_name: "slang_build_add_unit".into() },
        BuiltinFn { name: "build_add_dep".into(),         params: vec![("graph", IrType::I64), ("from", IrType::I64), ("to", IrType::I64)], ret: IrType::I64, runtime_name: "slang_build_add_dep".into() },
        BuiltinFn { name: "build_topo_sort".into(),       params: vec![("graph", IrType::I64)],                                          ret: IrType::Ptr,  runtime_name: "slang_build_topo_sort".into() },
        BuiltinFn { name: "content_hash".into(),          params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "slang_content_hash".into() },

        // ── v29: Benchmarks ──────────────────────────────────────────────────
        BuiltinFn { name: "bench_mean".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::F64,  runtime_name: "slang_bench_mean".into() },
        BuiltinFn { name: "bench_median".into(),          params: vec![("data", IrType::Ptr)],                                           ret: IrType::F64,  runtime_name: "slang_bench_median".into() },
        BuiltinFn { name: "bench_stddev".into(),          params: vec![("data", IrType::Ptr)],                                           ret: IrType::F64,  runtime_name: "slang_bench_stddev".into() },
        BuiltinFn { name: "bench_percentile".into(),      params: vec![("data", IrType::Ptr), ("p", IrType::F64)],                       ret: IrType::F64,  runtime_name: "slang_bench_percentile".into() },
        BuiltinFn { name: "bench_ci95".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "slang_bench_ci95".into() },
        BuiltinFn { name: "bench_regression".into(),      params: vec![("old", IrType::Ptr), ("new", IrType::Ptr)],                      ret: IrType::Ptr,  runtime_name: "slang_bench_regression".into() },

        // ── v30: Regex Engine ────────────────────────────────────────────────
        BuiltinFn { name: "regex_match".into(),           params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::I64,  runtime_name: "vitalis_regex_is_match".into() },
        BuiltinFn { name: "regex_find".into(),            params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::Ptr,  runtime_name: "vitalis_regex_find_first".into() },
        BuiltinFn { name: "regex_find_all".into(),        params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::Ptr,  runtime_name: "vitalis_regex_find_all_matches".into() },
        BuiltinFn { name: "regex_captures".into(),        params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::Ptr,  runtime_name: "vitalis_regex_captures_first".into() },
        BuiltinFn { name: "regex_replace".into(),         params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("rep", IrType::Ptr)], ret: IrType::Ptr, runtime_name: "vitalis_regex_replace_first".into() },
        BuiltinFn { name: "regex_replace_all".into(),     params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr), ("rep", IrType::Ptr)], ret: IrType::Ptr, runtime_name: "vitalis_regex_replace_all_matches".into() },
        BuiltinFn { name: "regex_split".into(),           params: vec![("pattern", IrType::Ptr), ("text", IrType::Ptr)],                 ret: IrType::Ptr,  runtime_name: "vitalis_regex_split_by".into() },

        // ── v30: Serialization ───────────────────────────────────────────────
        BuiltinFn { name: "json_parse".into(),            params: vec![("json", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_json_parse".into() },
        BuiltinFn { name: "json_stringify".into(),        params: vec![("json", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_json_stringify".into() },
        BuiltinFn { name: "json_get".into(),              params: vec![("json", IrType::Ptr), ("path", IrType::Ptr)],                    ret: IrType::Ptr,  runtime_name: "vitalis_json_get".into() },
        BuiltinFn { name: "base64_encode".into(),         params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_base64_encode".into() },
        BuiltinFn { name: "base64_decode".into(),         params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_base64_decode".into() },
        BuiltinFn { name: "hex_encode".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_hex_encode".into() },
        BuiltinFn { name: "hex_decode".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_hex_decode".into() },
        BuiltinFn { name: "url_encode".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_url_encode".into() },
        BuiltinFn { name: "url_decode".into(),            params: vec![("data", IrType::Ptr)],                                           ret: IrType::Ptr,  runtime_name: "vitalis_ser_url_decode".into() },

        // ── v30: Property Testing ────────────────────────────────────────────
        BuiltinFn { name: "qc_gen_i64".into(),            params: vec![("seed", IrType::I64)],                                           ret: IrType::I64,  runtime_name: "vitalis_qc_gen_i64".into() },
        BuiltinFn { name: "qc_gen_f64".into(),            params: vec![("seed", IrType::I64)],                                           ret: IrType::F64,  runtime_name: "vitalis_qc_gen_f64".into() },
        BuiltinFn { name: "qc_gen_bool".into(),           params: vec![("seed", IrType::I64)],                                           ret: IrType::I64,  runtime_name: "vitalis_qc_gen_bool".into() },
        BuiltinFn { name: "qc_shrink_i64".into(),         params: vec![("val", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_qc_shrink_i64".into() },
        BuiltinFn { name: "qc_test_commutative".into(),   params: vec![("seed", IrType::I64), ("count", IrType::I64)],                  ret: IrType::I64,  runtime_name: "vitalis_qc_test_commutative_add".into() },
        BuiltinFn { name: "qc_test_sort_idempotent".into(), params: vec![("seed", IrType::I64), ("count", IrType::I64)],                ret: IrType::I64,  runtime_name: "vitalis_qc_test_sort_idempotent".into() },
        BuiltinFn { name: "qc_chi_squared".into(),        params: vec![("seed", IrType::I64), ("count", IrType::I64)],                  ret: IrType::F64,  runtime_name: "vitalis_qc_chi_squared".into() },

        // ── v30: Data Structures ─────────────────────────────────────────────
        BuiltinFn { name: "btree_create".into(),          params: vec![("min_degree", IrType::I64)],                                     ret: IrType::I64,  runtime_name: "vitalis_btree_create".into() },
        BuiltinFn { name: "btree_insert".into(),          params: vec![("tree", IrType::I64), ("key", IrType::I64)],                     ret: IrType::I64,  runtime_name: "vitalis_btree_insert".into() },
        BuiltinFn { name: "btree_search".into(),          params: vec![("tree", IrType::I64), ("key", IrType::I64)],                     ret: IrType::I64,  runtime_name: "vitalis_btree_search".into() },
        BuiltinFn { name: "btree_len".into(),             params: vec![("tree", IrType::I64)],                                           ret: IrType::I64,  runtime_name: "vitalis_btree_len".into() },
        BuiltinFn { name: "ringbuf_create".into(),        params: vec![("capacity", IrType::I64)],                                       ret: IrType::I64,  runtime_name: "vitalis_ringbuf_create".into() },
        BuiltinFn { name: "ringbuf_push".into(),          params: vec![("buf", IrType::I64), ("val", IrType::I64)],                      ret: IrType::I64,  runtime_name: "vitalis_ringbuf_push_back".into() },
        BuiltinFn { name: "ringbuf_pop".into(),           params: vec![("buf", IrType::I64)],                                            ret: IrType::I64,  runtime_name: "vitalis_ringbuf_pop_front".into() },
        BuiltinFn { name: "uf_create".into(),             params: vec![("n", IrType::I64)],                                              ret: IrType::I64,  runtime_name: "vitalis_uf_create".into() },
        BuiltinFn { name: "uf_union".into(),              params: vec![("uf", IrType::I64), ("a", IrType::I64), ("b", IrType::I64)],     ret: IrType::I64,  runtime_name: "vitalis_uf_union".into() },
        BuiltinFn { name: "uf_find".into(),               params: vec![("uf", IrType::I64), ("x", IrType::I64)],                         ret: IrType::I64,  runtime_name: "vitalis_uf_find".into() },
        BuiltinFn { name: "uf_connected".into(),          params: vec![("uf", IrType::I64), ("a", IrType::I64), ("b", IrType::I64)],     ret: IrType::I64,  runtime_name: "vitalis_uf_connected".into() },
        BuiltinFn { name: "lru_create".into(),            params: vec![("capacity", IrType::I64)],                                       ret: IrType::I64,  runtime_name: "vitalis_lru_create".into() },
        BuiltinFn { name: "lru_put".into(),               params: vec![("cache", IrType::I64), ("key", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_lru_put".into() },
        BuiltinFn { name: "lru_get".into(),               params: vec![("cache", IrType::I64), ("key", IrType::I64)],                    ret: IrType::I64,  runtime_name: "vitalis_lru_get".into() },

        // ── v30: Networking ──────────────────────────────────────────────────
        BuiltinFn { name: "url_parse".into(),             params: vec![("url", IrType::Ptr)],                                            ret: IrType::Ptr,  runtime_name: "vitalis_url_parse".into() },
        BuiltinFn { name: "http_build_request".into(),    params: vec![("method", IrType::Ptr), ("path", IrType::Ptr), ("host", IrType::Ptr)], ret: IrType::Ptr, runtime_name: "vitalis_http_build_request".into() },
        BuiltinFn { name: "http_parse_request".into(),    params: vec![("raw", IrType::Ptr)],                                            ret: IrType::Ptr,  runtime_name: "vitalis_http_parse_request".into() },
        BuiltinFn { name: "is_valid_ipv4".into(),         params: vec![("addr", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "vitalis_is_valid_ipv4".into() },
        BuiltinFn { name: "is_valid_ipv6".into(),         params: vec![("addr", IrType::Ptr)],                                           ret: IrType::I64,  runtime_name: "vitalis_is_valid_ipv6".into() },
        BuiltinFn { name: "parse_query_string".into(),    params: vec![("query", IrType::Ptr)],                                          ret: IrType::Ptr,  runtime_name: "vitalis_parse_query_string".into() },
        BuiltinFn { name: "dns_build_query".into(),       params: vec![("name", IrType::Ptr), ("type", IrType::I64)],                    ret: IrType::Ptr,  runtime_name: "vitalis_dns_build_query".into() },

        // ── v30: ECS ─────────────────────────────────────────────────────────
        BuiltinFn { name: "ecs_world_create".into(),      params: vec![],                                                                 ret: IrType::I64,  runtime_name: "vitalis_ecs_world_create".into() },
        BuiltinFn { name: "ecs_spawn".into(),             params: vec![("world", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_ecs_spawn".into() },
        BuiltinFn { name: "ecs_despawn".into(),           params: vec![("world", IrType::I64), ("entity", IrType::I64)],                 ret: IrType::I64,  runtime_name: "vitalis_ecs_despawn".into() },
        BuiltinFn { name: "ecs_add_component".into(),     params: vec![("world", IrType::I64), ("entity", IrType::I64), ("type", IrType::I64), ("val", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_ecs_add_component".into() },
        BuiltinFn { name: "ecs_get_component".into(),     params: vec![("world", IrType::I64), ("entity", IrType::I64), ("type", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_ecs_get_component".into() },
        BuiltinFn { name: "ecs_has_component".into(),     params: vec![("world", IrType::I64), ("entity", IrType::I64), ("type", IrType::I64)], ret: IrType::I64, runtime_name: "vitalis_ecs_has_component".into() },
        BuiltinFn { name: "ecs_entity_count".into(),      params: vec![("world", IrType::I64)],                                          ret: IrType::I64,  runtime_name: "vitalis_ecs_entity_count".into() },
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
