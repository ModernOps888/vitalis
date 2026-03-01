"""
Vitalis — Python wrapper for the native Vitalis compiler + JIT runtime.

Usage:
    import vitalis

    # Compile and JIT-execute .sl source code (returns i64 result)
    result = vitalis.compile_and_run("fn main() -> i64 { 42 }")
    assert result == 42

    # Run a .sl file from disk
    result = vitalis.run_file("examples/hello.sl")

    # Type-check source (returns list of error strings, [] = valid)
    errors = vitalis.check("fn main() -> i64 { 42 }")

    # Parse source → AST debug string
    ast = vitalis.parse_ast("fn main() -> i64 { 1 + 2 }")

    # Lex source → list of (token_kind, text) tuples
    tokens = vitalis.lex("fn main() -> i64 { 42 }")

    # Lower source → IR dump string
    ir = vitalis.dump_ir("fn main() -> i64 { 1 + 2 }")

    # Compiler version
    print(vitalis.version())

    # Hot-path native operations
    p95 = vitalis.hotpath_p95([1.0, 2.0, ..., 100.0])
"""

import ctypes
import json
import os
import sys

# ─── Locate the DLL ─────────────────────────────────────────────────────

def _find_dll():
    """Find vitalis.dll — checks multiple locations."""
    candidates = [
        # Development build
        os.path.join(os.path.dirname(__file__), "slang", "target", "debug", "vitalis.dll"),
        os.path.join(os.path.dirname(__file__), "slang", "target", "release", "vitalis.dll"),
        # Installed alongside this module
        os.path.join(os.path.dirname(__file__), "vitalis.dll"),
        # System PATH
        "vitalis.dll",
    ]
    # Also check via env var
    env_path = os.environ.get("VITALIS_LIB_PATH") or os.environ.get("SLANG_LIB_PATH")
    if env_path:
        candidates.insert(0, env_path)

    for path in candidates:
        if os.path.isfile(path):
            return path

    raise FileNotFoundError(
        f"vitalis.dll not found. Searched:\n" +
        "\n".join(f"  - {p}" for p in candidates) +
        "\n\nBuild with: cd slang && cargo build"
    )


_dll_path = _find_dll()
_lib = ctypes.CDLL(_dll_path)

# ─── Define C function signatures ───────────────────────────────────────

# i64 slang_compile_and_run(const char* source, char** error_out)
_lib.slang_compile_and_run.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
_lib.slang_compile_and_run.restype = ctypes.c_int64

# char* slang_check(const char* source)  — returns allocated string, must free
_lib.slang_check.argtypes = [ctypes.c_char_p]
_lib.slang_check.restype = ctypes.c_void_p

# char* slang_parse_ast(const char* source)
_lib.slang_parse_ast.argtypes = [ctypes.c_char_p]
_lib.slang_parse_ast.restype = ctypes.c_void_p

# char* slang_lex(const char* source)
_lib.slang_lex.argtypes = [ctypes.c_char_p]
_lib.slang_lex.restype = ctypes.c_void_p

# char* slang_dump_ir(const char* source)
_lib.slang_dump_ir.argtypes = [ctypes.c_char_p]
_lib.slang_dump_ir.restype = ctypes.c_void_p

# char* slang_version()
_lib.slang_version.argtypes = []
_lib.slang_version.restype = ctypes.c_void_p

# void slang_free_string(char* ptr)
_lib.slang_free_string.argtypes = [ctypes.c_void_p]
_lib.slang_free_string.restype = None

# void slang_free_error(char* ptr)
_lib.slang_free_error.argtypes = [ctypes.c_void_p]
_lib.slang_free_error.restype = None


# ─── Helpers ─────────────────────────────────────────────────────────────

def _read_and_free(ptr):
    """Read a C string from a void pointer and free it."""
    if not ptr:
        return ""
    result = ctypes.string_at(ptr).decode("utf-8")
    _lib.slang_free_string(ptr)
    return result


# ─── Public API ──────────────────────────────────────────────────────────

def compile_and_run(source: str) -> int:
    """Compile Vitalis source and JIT-execute main(). Returns i64 result."""
    error_ptr = ctypes.c_void_p(None)
    result = _lib.slang_compile_and_run(
        source.encode("utf-8"),
        ctypes.byref(error_ptr),
    )
    if error_ptr.value is not None:
        error_msg = ctypes.string_at(error_ptr.value).decode("utf-8")
        _lib.slang_free_error(error_ptr)
        raise RuntimeError(f"Vitalis compilation error: {error_msg}")
    return result


def run_file(path: str) -> int:
    """Read a .sl file from disk, compile and JIT-execute it."""
    with open(path, "r", encoding="utf-8") as f:
        source = f.read()
    return compile_and_run(source)


def check(source: str) -> list[str]:
    """Type-check Vitalis source. Returns list of error strings ([] = valid)."""
    ptr = _lib.slang_check(source.encode("utf-8"))
    result = _read_and_free(ptr)
    try:
        return json.loads(result) if result else []
    except json.JSONDecodeError:
        return [result] if result else []


def parse_ast(source: str) -> str:
    """Parse Vitalis source and return AST as debug string."""
    ptr = _lib.slang_parse_ast(source.encode("utf-8"))
    return _read_and_free(ptr)


def lex(source: str) -> list[tuple[str, str]]:
    """Lex Vitalis source. Returns list of (token_kind, text) tuples."""
    ptr = _lib.slang_lex(source.encode("utf-8"))
    result = _read_and_free(ptr)
    try:
        pairs = json.loads(result) if result else []
        return [(kind, text) for kind, text in pairs]
    except json.JSONDecodeError:
        return []


def dump_ir(source: str) -> str:
    """Lower Vitalis source to IR and return dump string."""
    ptr = _lib.slang_dump_ir(source.encode("utf-8"))
    return _read_and_free(ptr)


def version() -> str:
    """Return the Vitalis compiler version."""
    ptr = _lib.slang_version()
    return _read_and_free(ptr) or "unknown"


# ─── Evolution API ──────────────────────────────────────────────────────

# void slang_evo_load(const char* source)
_lib.slang_evo_load.argtypes = [ctypes.c_char_p]
_lib.slang_evo_load.restype = None

# void slang_evo_register(const char* name, const char* source)
_lib.slang_evo_register.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
_lib.slang_evo_register.restype = None

# i64 slang_evo_evolve(const char* name, const char* new_source)
_lib.slang_evo_evolve.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
_lib.slang_evo_evolve.restype = ctypes.c_int64

# void slang_evo_set_fitness(const char* name, f64 score)
_lib.slang_evo_set_fitness.argtypes = [ctypes.c_char_p, ctypes.c_double]
_lib.slang_evo_set_fitness.restype = None

# f64 slang_evo_get_fitness(const char* name)
_lib.slang_evo_get_fitness.argtypes = [ctypes.c_char_p]
_lib.slang_evo_get_fitness.restype = ctypes.c_double

# u64 slang_evo_get_generation(const char* name)
_lib.slang_evo_get_generation.argtypes = [ctypes.c_char_p]
_lib.slang_evo_get_generation.restype = ctypes.c_uint64

# char* slang_evo_list()
_lib.slang_evo_list.argtypes = []
_lib.slang_evo_list.restype = ctypes.c_void_p

# i64 slang_evo_run()
_lib.slang_evo_run.argtypes = []
_lib.slang_evo_run.restype = ctypes.c_int64

# char* slang_evo_get_source(const char* name)
_lib.slang_evo_get_source.argtypes = [ctypes.c_char_p]
_lib.slang_evo_get_source.restype = ctypes.c_void_p

# i64 slang_evo_rollback(const char* name, u64 generation)
_lib.slang_evo_rollback.argtypes = [ctypes.c_char_p, ctypes.c_uint64]
_lib.slang_evo_rollback.restype = ctypes.c_int64


def evo_load(source: str) -> None:
    """Load a program into the evolution registry, extracting @evolvable functions."""
    _lib.slang_evo_load(source.encode("utf-8"))


def evo_register(name: str, source: str) -> None:
    """Register a function as evolvable."""
    _lib.slang_evo_register(name.encode("utf-8"), source.encode("utf-8"))


def evo_evolve(name: str, new_source: str) -> int:
    """Submit a new variant. Returns new generation number (-1 on error)."""
    return _lib.slang_evo_evolve(name.encode("utf-8"), new_source.encode("utf-8"))


def evo_set_fitness(name: str, score: float) -> None:
    """Set fitness score for the current variant."""
    _lib.slang_evo_set_fitness(name.encode("utf-8"), score)


def evo_get_fitness(name: str) -> float | None:
    """Get fitness score. Returns None if not set."""
    result = _lib.slang_evo_get_fitness(name.encode("utf-8"))
    import math
    return None if math.isnan(result) else result


def evo_get_generation(name: str) -> int:
    """Get the current generation number for a function."""
    return _lib.slang_evo_get_generation(name.encode("utf-8"))


def evo_list() -> list[str]:
    """List all evolvable function names."""
    ptr = _lib.slang_evo_list()
    result = _read_and_free(ptr)
    try:
        return json.loads(result) if result else []
    except json.JSONDecodeError:
        return []


def evo_run() -> int:
    """Compile and execute the evolved program. Returns main() result."""
    return _lib.slang_evo_run()


def evo_get_source(name: str) -> str:
    """Get the current source code of an evolvable function."""
    ptr = _lib.slang_evo_get_source(name.encode("utf-8"))
    return _read_and_free(ptr)


def evo_rollback(name: str, generation: int) -> bool:
    """Rollback a function to a previous generation. Returns True on success."""
    return _lib.slang_evo_rollback(name.encode("utf-8"), generation) == 0


# ─── Hot-path API ───────────────────────────────────────────────────────
# Native Rust implementations of performance-critical operations.
# These bypass Vitalis compilation and call Rust code directly.

# usize hotpath_sliding_window_count(const f64*, usize, f64, f64)
_lib.hotpath_sliding_window_count.argtypes = [
    ctypes.POINTER(ctypes.c_double), ctypes.c_size_t, ctypes.c_double, ctypes.c_double
]
_lib.hotpath_sliding_window_count.restype = ctypes.c_size_t

# f64 hotpath_token_bucket(f64, f64, f64, f64, f64)
_lib.hotpath_token_bucket.argtypes = [
    ctypes.c_double, ctypes.c_double, ctypes.c_double, ctypes.c_double, ctypes.c_double
]
_lib.hotpath_token_bucket.restype = ctypes.c_double

# f64 hotpath_p95(const f64*, usize)
_lib.hotpath_p95.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_p95.restype = ctypes.c_double

# f64 hotpath_percentile(const f64*, usize, f64)
_lib.hotpath_percentile.argtypes = [
    ctypes.POINTER(ctypes.c_double), ctypes.c_size_t, ctypes.c_double
]
_lib.hotpath_percentile.restype = ctypes.c_double

# f64 hotpath_mean(const f64*, usize)
_lib.hotpath_mean.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_mean.restype = ctypes.c_double

# f64 hotpath_weighted_score(const f64*, const f64*, usize)
_lib.hotpath_weighted_score.argtypes = [
    ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double), ctypes.c_size_t
]
_lib.hotpath_weighted_score.restype = ctypes.c_double

# f64 hotpath_code_quality_score(f64, f64, f64, f64, f64, i32)
_lib.hotpath_code_quality_score.argtypes = [
    ctypes.c_double, ctypes.c_double, ctypes.c_double,
    ctypes.c_double, ctypes.c_double, ctypes.c_int32
]
_lib.hotpath_code_quality_score.restype = ctypes.c_double

# i32 hotpath_tally_votes(const i32*, usize, f64*)
_lib.hotpath_tally_votes.argtypes = [
    ctypes.POINTER(ctypes.c_int32), ctypes.c_size_t, ctypes.POINTER(ctypes.c_double)
]
_lib.hotpath_tally_votes.restype = ctypes.c_int32

# char* hotpath_tally_string_votes(const char*)
_lib.hotpath_tally_string_votes.argtypes = [ctypes.c_char_p]
_lib.hotpath_tally_string_votes.restype = ctypes.c_void_p

# u64 hotpath_cognitive_complexity(const u32*, usize)
_lib.hotpath_cognitive_complexity.argtypes = [
    ctypes.POINTER(ctypes.c_uint32), ctypes.c_size_t
]
_lib.hotpath_cognitive_complexity.restype = ctypes.c_uint64

# ─── Quantum-Inspired & Mathematical Optimization Bindings ──────────────

# i32 hotpath_quantum_anneal_accept(f64, f64, f64, f64, f64, f64)
_lib.hotpath_quantum_anneal_accept.argtypes = [
    ctypes.c_double, ctypes.c_double, ctypes.c_double,
    ctypes.c_double, ctypes.c_double, ctypes.c_double
]
_lib.hotpath_quantum_anneal_accept.restype = ctypes.c_int32

# f64 hotpath_bayesian_ucb(f64, u64, u64, f64)
_lib.hotpath_bayesian_ucb.argtypes = [
    ctypes.c_double, ctypes.c_uint64, ctypes.c_uint64, ctypes.c_double
]
_lib.hotpath_bayesian_ucb.restype = ctypes.c_double

# void hotpath_boltzmann_select(const f64*, usize, f64, f64*)
_lib.hotpath_boltzmann_select.argtypes = [
    ctypes.POINTER(ctypes.c_double), ctypes.c_size_t,
    ctypes.c_double, ctypes.POINTER(ctypes.c_double)
]
_lib.hotpath_boltzmann_select.restype = None

# f64 hotpath_shannon_diversity(const f64*, usize)
_lib.hotpath_shannon_diversity.argtypes = [
    ctypes.POINTER(ctypes.c_double), ctypes.c_size_t
]
_lib.hotpath_shannon_diversity.restype = ctypes.c_double

# i32 hotpath_pareto_dominates(const f64*, const f64*, usize)
_lib.hotpath_pareto_dominates.argtypes = [
    ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double), ctypes.c_size_t
]
_lib.hotpath_pareto_dominates.restype = ctypes.c_int32

# usize hotpath_pareto_front(const f64*, usize, usize, u32*)
_lib.hotpath_pareto_front.argtypes = [
    ctypes.POINTER(ctypes.c_double), ctypes.c_size_t, ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_uint32)
]
_lib.hotpath_pareto_front.restype = ctypes.c_size_t

# void hotpath_cma_es_mean_update(const f64*, const f64*, usize, usize, usize, f64*)
_lib.hotpath_cma_es_mean_update.argtypes = [
    ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double),
    ctypes.c_size_t, ctypes.c_size_t, ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_double)
]
_lib.hotpath_cma_es_mean_update.restype = None

# f64 hotpath_ema_update(f64, f64, f64)
_lib.hotpath_ema_update.argtypes = [ctypes.c_double, ctypes.c_double, ctypes.c_double]
_lib.hotpath_ema_update.restype = ctypes.c_double

# f64 hotpath_levy_step(f64, f64, f64, f64)
_lib.hotpath_levy_step.argtypes = [
    ctypes.c_double, ctypes.c_double, ctypes.c_double, ctypes.c_double
]
_lib.hotpath_levy_step.restype = ctypes.c_double

# f64 hotpath_adaptive_fitness(f64, f64, f64, f64, u64)
_lib.hotpath_adaptive_fitness.argtypes = [
    ctypes.c_double, ctypes.c_double, ctypes.c_double, ctypes.c_double, ctypes.c_uint64
]
_lib.hotpath_adaptive_fitness.restype = ctypes.c_double

# Phase 21: Vector & statistical ops
_lib.hotpath_cosine_similarity.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_cosine_similarity.restype = ctypes.c_double

_lib.hotpath_l2_normalize.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_l2_normalize.restype = ctypes.c_double

_lib.hotpath_stddev.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_stddev.restype = ctypes.c_double

_lib.hotpath_median.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_median.restype = ctypes.c_double

# Phase 22: Advanced analytics
_lib.hotpath_exponential_moving_average.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t, ctypes.c_double]
_lib.hotpath_exponential_moving_average.restype = ctypes.c_double

_lib.hotpath_entropy.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_entropy.restype = ctypes.c_double

_lib.hotpath_min_max_normalize.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_min_max_normalize.restype = ctypes.c_double

_lib.hotpath_hamming_distance.argtypes = [ctypes.c_int64, ctypes.c_int64]
_lib.hotpath_hamming_distance.restype = ctypes.c_int64

# Phase 23: ML operations
_lib.hotpath_softmax.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_softmax.restype = None

_lib.hotpath_cross_entropy.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_cross_entropy.restype = ctypes.c_double

_lib.hotpath_batch_sigmoid.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_batch_sigmoid.restype = None

_lib.hotpath_argmax.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_argmax.restype = ctypes.c_size_t

_lib.hotpath_batch_relu.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_batch_relu.restype = None

# Phase 24: Advanced ML hotpath ops
_lib.hotpath_batch_leaky_relu.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t, ctypes.c_double]
_lib.hotpath_batch_leaky_relu.restype = None

_lib.hotpath_batch_norm.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t, ctypes.c_double, ctypes.c_double, ctypes.c_double]
_lib.hotpath_batch_norm.restype = ctypes.c_double

_lib.hotpath_kl_divergence.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_kl_divergence.restype = ctypes.c_double

_lib.hotpath_gelu_batch.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t]
_lib.hotpath_gelu_batch.restype = None

_lib.hotpath_clip.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_size_t, ctypes.c_double, ctypes.c_double]
_lib.hotpath_clip.restype = None


def _to_double_array(values):
    """Convert a list of numbers to a ctypes double array."""
    arr = (ctypes.c_double * len(values))(*values)
    return arr, len(values)


def hotpath_sliding_window_count(timestamps: list[float], now: float, window_seconds: float) -> int:
    """Count timestamps within a sliding window. Native Rust implementation."""
    arr, n = _to_double_array(timestamps)
    return _lib.hotpath_sliding_window_count(arr, n, now, window_seconds)


def hotpath_token_bucket(tokens: float, max_tokens: float, refill_rate: float,
                         elapsed_seconds: float, cost: float) -> float:
    """Token bucket rate limiter. Returns remaining tokens (< 0 = denied). Native Rust."""
    return _lib.hotpath_token_bucket(tokens, max_tokens, refill_rate, elapsed_seconds, cost)


def hotpath_p95(values: list[float]) -> float:
    """Compute 95th percentile of a list. Native Rust implementation."""
    if not values:
        return -1.0
    arr, n = _to_double_array(values)
    return _lib.hotpath_p95(arr, n)


def hotpath_percentile(values: list[float], pct: float) -> float:
    """Compute arbitrary percentile (0.0-1.0) of a list. Native Rust."""
    if not values:
        return -1.0
    arr, n = _to_double_array(values)
    return _lib.hotpath_percentile(arr, n, pct)


def hotpath_mean(values: list[float]) -> float:
    """Compute mean of a list. Native Rust implementation."""
    if not values:
        return 0.0
    arr, n = _to_double_array(values)
    return _lib.hotpath_mean(arr, n)


def hotpath_weighted_score(metrics: list[float], weights: list[float]) -> float:
    """Compute weighted score, clamped to [0, 1]. Native Rust implementation."""
    assert len(metrics) == len(weights), "metrics and weights must have same length"
    m_arr, n = _to_double_array(metrics)
    w_arr, _ = _to_double_array(weights)
    return _lib.hotpath_weighted_score(m_arr, w_arr, n)


def hotpath_code_quality_score(cyclomatic: float, cognitive: float, loc: float,
                                num_functions: float, security_issues: float,
                                has_tests: bool) -> float:
    """Compute code quality score (0-100). Native Rust implementation."""
    return _lib.hotpath_code_quality_score(
        cyclomatic, cognitive, loc, num_functions, security_issues, int(has_tests)
    )


def hotpath_tally_votes(votes: list[int]) -> tuple[int, float]:
    """Tally integer votes. Returns (winner, agreement_pct). Native Rust."""
    if not votes:
        return (-1, 0.0)
    arr = (ctypes.c_int32 * len(votes))(*votes)
    agreement = ctypes.c_double(0.0)
    winner = _lib.hotpath_tally_votes(arr, len(votes), ctypes.byref(agreement))
    return (winner, agreement.value)


def hotpath_tally_string_votes(votes: list[str]) -> dict:
    """Tally string votes. Returns {"winner": str, "agreement": float, "counts": dict}."""
    import json as j
    json_input = j.dumps(votes)
    ptr = _lib.hotpath_tally_string_votes(json_input.encode("utf-8"))
    result = _read_and_free(ptr)
    try:
        return j.loads(result)
    except j.JSONDecodeError:
        return {"winner": "", "agreement": 0.0, "counts": {}}


def hotpath_cognitive_complexity(depths: list[int]) -> int:
    """Compute cognitive complexity from nesting depths. Native Rust."""
    if not depths:
        return 0
    arr = (ctypes.c_uint32 * len(depths))(*depths)
    return _lib.hotpath_cognitive_complexity(arr, len(depths))


# ─── Quantum-Inspired & Mathematical Optimization ───────────────────────

def hotpath_quantum_anneal_accept(old_fitness: float, new_fitness: float,
                                   temperature: float, tunnel_strength: float = 0.1,
                                   barrier_width: float = 1.0,
                                   rand_uniform: float | None = None) -> bool:
    """Quantum-inspired annealing acceptance criterion.

    Combines Metropolis criterion with quantum tunneling to escape local optima.
    Always accepts improvements; probabilistically accepts worse solutions based
    on temperature and tunneling parameters. Native Rust.
    """
    import random
    if rand_uniform is None:
        rand_uniform = random.random()
    result = _lib.hotpath_quantum_anneal_accept(
        old_fitness, new_fitness, temperature,
        tunnel_strength, barrier_width, rand_uniform
    )
    return bool(result)


def hotpath_bayesian_ucb(mean_fitness: float, num_trials: int,
                          total_trials: int, kappa: float = 1.414) -> float:
    """Bayesian UCB1 acquisition score for explore/exploit balance.

    Higher score = should evolve this function next. Functions tried fewer
    times get an exploration bonus. kappa=√2 is theoretically optimal. Native Rust.
    """
    return _lib.hotpath_bayesian_ucb(mean_fitness, num_trials, total_trials, kappa)


def hotpath_boltzmann_select(fitnesses: list[float], temperature: float = 1.0) -> list[float]:
    """Boltzmann (softmax) selection probabilities.

    Temperature controls selection pressure:
    - T→0: greedy (always pick best)
    - T→∞: uniform random
    - T≈1: moderate pressure
    Returns list of selection probabilities that sum to 1.0. Native Rust.
    """
    if not fitnesses:
        return []
    arr, n = _to_double_array(fitnesses)
    probs = (ctypes.c_double * n)()
    _lib.hotpath_boltzmann_select(arr, n, temperature, probs)
    return list(probs)


def hotpath_shannon_diversity(probs: list[float]) -> float:
    """Shannon entropy diversity metric, normalized to [0, 1].

    1.0 = maximum diversity (uniform distribution).
    0.0 = minimum diversity (all weight on one option). Native Rust.
    """
    if len(probs) <= 1:
        return 0.0
    arr, n = _to_double_array(probs)
    return _lib.hotpath_shannon_diversity(arr, n)


def hotpath_pareto_dominates(a: list[float], b: list[float]) -> bool:
    """Check if solution A Pareto-dominates solution B.

    A dominates B iff all objectives of A ≥ B and at least one is strictly >. Native Rust.
    """
    assert len(a) == len(b), "objective vectors must have same length"
    arr_a, n = _to_double_array(a)
    arr_b, _ = _to_double_array(b)
    return bool(_lib.hotpath_pareto_dominates(arr_a, arr_b, n))


def hotpath_pareto_front(solutions: list[list[float]]) -> list[int]:
    """Compute the Pareto front (non-dominated solution indices).

    Input: list of solutions, each a list of objective values (higher = better).
    Returns: indices of non-dominated solutions. Native Rust.
    """
    if not solutions:
        return []
    n_obj = len(solutions[0])
    pop_size = len(solutions)
    flat = []
    for sol in solutions:
        assert len(sol) == n_obj, "all solutions must have same number of objectives"
        flat.extend(sol)
    arr = (ctypes.c_double * len(flat))(*flat)
    out = (ctypes.c_uint32 * pop_size)()
    count = _lib.hotpath_pareto_front(arr, pop_size, n_obj, out)
    return [int(out[i]) for i in range(count)]


def hotpath_cma_es_mean_update(solutions: list[list[float]], fitnesses: list[float],
                                mu: int | None = None) -> list[float]:
    """CMA-ES weighted mean update from best µ solutions.

    Computes the updated distribution mean using log-linear CMA-ES weights
    on the µ best solutions (ranked by fitness). Native Rust.
    """
    if not solutions or not fitnesses:
        return []
    lam = len(solutions)
    dim = len(solutions[0])
    if mu is None:
        mu = max(1, lam // 2)
    flat = []
    for sol in solutions:
        flat.extend(sol)
    sol_arr = (ctypes.c_double * len(flat))(*flat)
    fit_arr, _ = _to_double_array(fitnesses)
    mean_out = (ctypes.c_double * dim)()
    _lib.hotpath_cma_es_mean_update(sol_arr, fit_arr, lam, mu, dim, mean_out)
    return list(mean_out)


def hotpath_ema_update(ema_old: float, new_value: float, alpha: float = 0.3) -> float:
    """Exponential Moving Average update for fitness trend tracking. Native Rust."""
    return _lib.hotpath_ema_update(ema_old, new_value, alpha)


def hotpath_levy_step(u_normal: float, v_normal: float,
                       beta: float = 1.5, scale: float = 1.0) -> float:
    """Lévy flight step magnitude for mutation distance.

    Generates heavy-tailed step sizes: frequent small mutations with
    occasional large jumps. Caller provides pre-generated standard normal
    samples u and v. Beta=1.5 is typical. Native Rust.
    """
    return _lib.hotpath_levy_step(u_normal, v_normal, beta, scale)


def hotpath_adaptive_fitness(speed: float, correctness: float,
                              complexity: float, security: float,
                              generation: int = 0) -> float:
    """Multi-objective adaptive fitness score.

    Combines speed, correctness, complexity, and security scores (each 0-1)
    with generation-adaptive weights:
    - Early generations: prioritize correctness + simplicity
    - Late generations: prioritize speed + security
    Native Rust.
    """
    return _lib.hotpath_adaptive_fitness(speed, correctness, complexity, security, generation)


# ─── Phase 21: Vector & Statistical Operations ─────────────────────────

def hotpath_cosine_similarity(a: list[float], b: list[float]) -> float:
    """Cosine similarity between two vectors. Returns [-1, 1]. Native Rust."""
    n = min(len(a), len(b))
    if n == 0:
        return 0.0
    arr_a = (ctypes.c_double * n)(*a[:n])
    arr_b = (ctypes.c_double * n)(*b[:n])
    return _lib.hotpath_cosine_similarity(arr_a, arr_b, n)


def hotpath_l2_normalize(values: list[float]) -> tuple[list[float], float]:
    """L2-normalize a vector. Returns (normalized_vector, original_magnitude). Native Rust."""
    if not values:
        return [], 0.0
    arr = (ctypes.c_double * len(values))(*values)
    magnitude = _lib.hotpath_l2_normalize(arr, len(values))
    return list(arr), magnitude


def hotpath_stddev(values: list[float]) -> float:
    """Standard deviation (sample) of values. Native Rust."""
    if len(values) < 2:
        return 0.0
    arr = (ctypes.c_double * len(values))(*values)
    return _lib.hotpath_stddev(arr, len(values))


def hotpath_median(values: list[float]) -> float:
    """Median of values. Native Rust."""
    if not values:
        return 0.0
    arr = (ctypes.c_double * len(values))(*values)
    return _lib.hotpath_median(arr, len(values))


def hotpath_exponential_moving_average(values: list[float], alpha: float = 0.3) -> float:
    """Exponential moving average over a series. Native Rust."""
    if not values:
        return 0.0
    arr = (ctypes.c_double * len(values))(*values)
    return _lib.hotpath_exponential_moving_average(arr, len(values), alpha)


def hotpath_entropy(probs: list[float]) -> float:
    """Shannon entropy of a probability distribution (bits). Native Rust."""
    if not probs:
        return 0.0
    arr = (ctypes.c_double * len(probs))(*probs)
    return _lib.hotpath_entropy(arr, len(probs))


def hotpath_min_max_normalize(values: list[float]) -> tuple[list[float], float]:
    """Min-max normalize a vector. Returns (normalized_values, range). Native Rust."""
    if not values:
        return [], 0.0
    arr = (ctypes.c_double * len(values))(*values)
    data_range = _lib.hotpath_min_max_normalize(arr, len(values))
    return list(arr), data_range


def hotpath_hamming_distance(a: int, b: int) -> int:
    """Hamming distance (differing bits) between two integers. Native Rust."""
    return _lib.hotpath_hamming_distance(a, b)


def hotpath_softmax(values: list[float]) -> list[float]:
    """Softmax over a vector (numerically stable). Native Rust."""
    if not values:
        return []
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_softmax(arr, len(values))
    return list(arr)


def hotpath_cross_entropy(target: list[float], predicted: list[float]) -> float:
    """Cross-entropy loss. Native Rust."""
    if not target or not predicted:
        return 0.0
    n = min(len(target), len(predicted))
    t_arr = (ctypes.c_double * n)(*target[:n])
    p_arr = (ctypes.c_double * n)(*predicted[:n])
    return _lib.hotpath_cross_entropy(t_arr, p_arr, n)


def hotpath_batch_sigmoid(values: list[float]) -> list[float]:
    """Apply sigmoid to each element. Native Rust."""
    if not values:
        return []
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_batch_sigmoid(arr, len(values))
    return list(arr)


def hotpath_argmax(values: list[float]) -> int:
    """Index of the maximum value. Native Rust."""
    if not values:
        return 0
    arr = (ctypes.c_double * len(values))(*values)
    return _lib.hotpath_argmax(arr, len(values))


def hotpath_batch_relu(values: list[float]) -> list[float]:
    """Apply ReLU to each element. Native Rust."""
    if not values:
        return []
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_batch_relu(arr, len(values))
    return list(arr)


# ── Phase 24: Advanced ML hotpath wrappers ─────────────────────────────

def hotpath_batch_leaky_relu(values: list[float], alpha: float = 0.01) -> list[float]:
    """Apply Leaky ReLU (alpha * x for x < 0) to each element. Native Rust."""
    if not values:
        return []
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_batch_leaky_relu(arr, len(values), alpha)
    return list(arr)


def hotpath_batch_norm(values: list[float], gamma: float = 1.0, beta: float = 0.0, epsilon: float = 1e-5) -> tuple[float, list[float]]:
    """Batch normalize values: (x - mean) / std * gamma + beta. Returns (mean, normalized). Native Rust."""
    if not values:
        return 0.0, []
    arr = (ctypes.c_double * len(values))(*values)
    mean = _lib.hotpath_batch_norm(arr, len(values), gamma, beta, epsilon)
    return mean, list(arr)


def hotpath_kl_divergence(p: list[float], q: list[float]) -> float:
    """KL divergence D_KL(P || Q). Native Rust."""
    n = min(len(p), len(q))
    if n == 0:
        return 0.0
    p_arr = (ctypes.c_double * n)(*p[:n])
    q_arr = (ctypes.c_double * n)(*q[:n])
    return _lib.hotpath_kl_divergence(p_arr, q_arr, n)


def hotpath_gelu_batch(values: list[float]) -> list[float]:
    """Apply GELU activation to each element. Native Rust."""
    if not values:
        return []
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_gelu_batch(arr, len(values))
    return list(arr)


def hotpath_clip(values: list[float], min_val: float, max_val: float) -> list[float]:
    """Clip/clamp all values to [min_val, max_val]. Native Rust."""
    if not values:
        return []
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_clip(arr, len(values), min_val, max_val)
    return list(arr)


# ── Phase 25: Numerical Linear Algebra & Loss Operations ────────────────

def hotpath_layer_norm(values: list[float], gamma: float = 1.0, beta: float = 0.0, epsilon: float = 1e-5) -> tuple[list[float], float]:
    """Layer normalization: (x - mean) / sqrt(var + eps) * gamma + beta. Returns (normalized, mean). Native Rust."""
    if not values:
        return [], 0.0
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_layer_norm.restype = ctypes.c_double
    mean = _lib.hotpath_layer_norm(arr, len(values), ctypes.c_double(gamma), ctypes.c_double(beta), ctypes.c_double(epsilon))
    return list(arr), mean


def hotpath_dropout_mask(values: list[float], keep_prob: float = 0.8, seed: int = 42) -> list[float]:
    """Apply deterministic dropout mask. Elements are either zeroed or scaled up. Native Rust."""
    if not values:
        return []
    arr = (ctypes.c_double * len(values))(*values)
    _lib.hotpath_dropout_mask(arr, len(values), ctypes.c_double(keep_prob), ctypes.c_uint64(seed))
    return list(arr)


def hotpath_cosine_distance(a: list[float], b: list[float]) -> float:
    """Cosine distance: 1 - cosine_similarity. Native Rust."""
    n = min(len(a), len(b))
    if n == 0:
        return 1.0
    a_arr = (ctypes.c_double * n)(*a[:n])
    b_arr = (ctypes.c_double * n)(*b[:n])
    _lib.hotpath_cosine_distance.restype = ctypes.c_double
    return _lib.hotpath_cosine_distance(a_arr, b_arr, n)


def hotpath_huber_loss(targets: list[float], predicted: list[float], delta: float = 1.0) -> float:
    """Huber loss (smooth L1): robust to outliers. Native Rust."""
    n = min(len(targets), len(predicted))
    if n == 0:
        return 0.0
    t_arr = (ctypes.c_double * n)(*targets[:n])
    p_arr = (ctypes.c_double * n)(*predicted[:n])
    _lib.hotpath_huber_loss.restype = ctypes.c_double
    return _lib.hotpath_huber_loss(t_arr, p_arr, n, ctypes.c_double(delta))


def hotpath_mse_loss(targets: list[float], predicted: list[float]) -> float:
    """Mean squared error loss. Native Rust."""
    n = min(len(targets), len(predicted))
    if n == 0:
        return 0.0
    t_arr = (ctypes.c_double * n)(*targets[:n])
    p_arr = (ctypes.c_double * n)(*predicted[:n])
    _lib.hotpath_mse_loss.restype = ctypes.c_double
    return _lib.hotpath_mse_loss(t_arr, p_arr, n)


# ─── Module metadata ────────────────────────────────────────────────────

__version__ = version()
__all__ = [
    # Core compiler
    "compile_and_run", "run_file", "check", "parse_ast", "lex", "dump_ir", "version",
    # Evolution
    "evo_load", "evo_register", "evo_evolve", "evo_set_fitness", "evo_get_fitness",
    "evo_get_generation", "evo_list", "evo_run", "evo_get_source", "evo_rollback",
    # Hot-path native ops (original)
    "hotpath_sliding_window_count", "hotpath_token_bucket",
    "hotpath_p95", "hotpath_percentile", "hotpath_mean",
    "hotpath_weighted_score", "hotpath_code_quality_score",
    "hotpath_tally_votes", "hotpath_tally_string_votes",
    "hotpath_cognitive_complexity",
    # Quantum-inspired & mathematical optimization (native Rust)
    "hotpath_quantum_anneal_accept", "hotpath_bayesian_ucb",
    "hotpath_boltzmann_select", "hotpath_shannon_diversity",
    "hotpath_pareto_dominates", "hotpath_pareto_front",
    "hotpath_cma_es_mean_update", "hotpath_ema_update",
    "hotpath_levy_step", "hotpath_adaptive_fitness",
    # Phase 21: Vector & statistical ops (native Rust)
    "hotpath_cosine_similarity", "hotpath_l2_normalize",
    "hotpath_stddev", "hotpath_median",
    # Phase 22: Advanced analytics (native Rust)
    "hotpath_exponential_moving_average", "hotpath_entropy",
    "hotpath_min_max_normalize", "hotpath_hamming_distance",
    # Phase 23: ML operations (native Rust)
    "hotpath_softmax", "hotpath_cross_entropy",
    "hotpath_batch_sigmoid", "hotpath_argmax", "hotpath_batch_relu",
    # Phase 24: Advanced ML operations (native Rust)
    "hotpath_batch_leaky_relu", "hotpath_batch_norm",
    "hotpath_kl_divergence", "hotpath_gelu_batch", "hotpath_clip",
    # Phase 25: Numerical linear algebra & loss ops (native Rust)
    "hotpath_layer_norm", "hotpath_dropout_mask",
    "hotpath_cosine_distance", "hotpath_huber_loss", "hotpath_mse_loss",
]
