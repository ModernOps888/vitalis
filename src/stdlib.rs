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
