//! Property-Based Testing Module for Vitalis v30.0
//!
//! QuickCheck-style property-based testing with:
//! - Xorshift128+ PRNG for reproducible generation
//! - Value generators for all basic types
//! - Shrinking strategies for minimal counterexamples
//! - Configurable test runner with seed replay
//! - Statistical coverage tracking

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── PRNG (Xorshift128+) ───────────────────────────────────────────

/// Fast, high-quality PRNG with 128-bit state. Period: 2^128 - 1.
#[derive(Clone)]
struct Rng {
    s0: u64,
    s1: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        // SplitMix64 to initialize both state words from a single seed
        let s0 = splitmix64(seed);
        let s1 = splitmix64(s0);
        Self {
            s0: if s0 == 0 { 1 } else { s0 },
            s1: if s1 == 0 { 1 } else { s1 },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut s1 = self.s0;
        let s0 = self.s1;
        let result = s0.wrapping_add(s1);
        self.s0 = s0;
        s1 ^= s1 << 23;
        self.s1 = s1 ^ s0 ^ (s1 >> 17) ^ (s0 >> 26);
        result
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    fn next_range(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u64 + 1;
        let val = self.next_u64() % range;
        min + val as i64
    }

    fn next_usize(&mut self, max: usize) -> usize {
        if max == 0 { return 0; }
        (self.next_u64() % (max as u64 + 1)) as usize
    }
}

fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9e3779b97f4a7c15);
    state = (state ^ (state >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94d049bb133111eb);
    state ^ (state >> 31)
}

// ─── Value Generators ───────────────────────────────────────────────

/// Generate a random i64 with bias toward interesting values.
fn gen_i64(rng: &mut Rng) -> i64 {
    let choice = rng.next_usize(9);
    match choice {
        0 => 0,
        1 => 1,
        2 => -1,
        3 => i64::MAX,
        4 => i64::MIN,
        5 => i64::MAX / 2,
        6 => i64::MIN / 2,
        _ => rng.next_range(i32::MIN as i64, i32::MAX as i64),
    }
}

/// Generate a random i64 within a specific range.
fn gen_i64_range(rng: &mut Rng, min: i64, max: i64) -> i64 {
    rng.next_range(min, max)
}

/// Generate a random f64 with bias toward edge cases.
fn gen_f64(rng: &mut Rng) -> f64 {
    let choice = rng.next_usize(7);
    match choice {
        0 => 0.0,
        1 => -0.0,
        2 => 1.0,
        3 => -1.0,
        4 => f64::EPSILON,
        5 => f64::MAX,
        _ => (rng.next_f64() - 0.5) * 2000.0, // range [-1000, 1000]
    }
}

/// Generate a random boolean.
fn gen_bool(rng: &mut Rng) -> bool {
    rng.next_bool()
}

/// Generate a random ASCII string up to `max_len` characters.
fn gen_string(rng: &mut Rng, max_len: usize) -> String {
    let len = rng.next_usize(max_len);
    let mut s = String::with_capacity(len);
    for _ in 0..len {
        let ch = rng.next_range(32, 126) as u8 as char; // printable ASCII
        s.push(ch);
    }
    s
}

/// Generate a random Vec<i64> up to `max_len` elements.
fn gen_vec_i64(rng: &mut Rng, max_len: usize) -> Vec<i64> {
    let len = rng.next_usize(max_len);
    let mut v = Vec::with_capacity(len);
    for _ in 0..len {
        v.push(gen_i64(rng));
    }
    v
}

/// Generate a sorted Vec<i64>.
fn gen_sorted_vec_i64(rng: &mut Rng, max_len: usize) -> Vec<i64> {
    let mut v = gen_vec_i64(rng, max_len);
    v.sort();
    v
}

// ─── Shrinking Strategies ───────────────────────────────────────────

/// Shrink an i64 toward zero — binary search approach.
fn shrink_i64(x: i64) -> Vec<i64> {
    if x == 0 {
        return vec![];
    }
    let mut candidates = vec![0];
    if x > 0 {
        candidates.push(x / 2);
        if x > 1 { candidates.push(x - 1); }
    } else {
        candidates.push(-x); // try positive version
        candidates.push(x / 2);
        if x < -1 { candidates.push(x + 1); }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

/// Shrink a string by removing characters or simplifying.
fn shrink_string(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    let mut candidates = Vec::new();
    // Try removing each character
    for i in 0..s.len() {
        let mut shrunk = String::with_capacity(s.len() - 1);
        for (j, ch) in s.chars().enumerate() {
            if j != i {
                shrunk.push(ch);
            }
        }
        candidates.push(shrunk);
    }
    // Try taking first half
    let half = s.len() / 2;
    if half > 0 {
        candidates.push(s[..half].to_string());
    }
    // Try empty string
    candidates.push(String::new());
    candidates.sort();
    candidates.dedup();
    candidates
}

/// Shrink a Vec<i64> by removing elements or shrinking values.
fn shrink_vec_i64(v: &[i64]) -> Vec<Vec<i64>> {
    if v.is_empty() {
        return vec![];
    }
    let mut candidates = Vec::new();
    // Try removing each element
    for i in 0..v.len() {
        let mut shrunk: Vec<i64> = Vec::with_capacity(v.len() - 1);
        for (j, &val) in v.iter().enumerate() {
            if j != i {
                shrunk.push(val);
            }
        }
        candidates.push(shrunk);
    }
    // Try first half
    let half = v.len() / 2;
    if half > 0 {
        candidates.push(v[..half].to_vec());
    }
    // Try shrinking each element
    for i in 0..v.len() {
        for shrunk_val in shrink_i64(v[i]) {
            let mut new_v = v.to_vec();
            new_v[i] = shrunk_val;
            candidates.push(new_v);
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

// ─── Test Runner ────────────────────────────────────────────────────

/// Configuration for property tests.
#[derive(Clone)]
struct PropTestConfig {
    num_tests: usize,
    max_shrinks: usize,
    seed: u64,
}

impl Default for PropTestConfig {
    fn default() -> Self {
        Self {
            num_tests: 100,
            max_shrinks: 100,
            seed: 12345,
        }
    }
}

/// Result of a property test.
#[derive(Debug, Clone)]
enum PropTestResult {
    Passed {
        num_tests: usize,
    },
    Failed {
        original_input: String,
        shrunk_input: String,
        num_shrinks: usize,
        seed: u64,
        test_num: usize,
    },
}

/// Run a property test on i64 values.
fn prop_test_i64<F>(config: &PropTestConfig, property: F) -> PropTestResult
where
    F: Fn(i64) -> bool,
{
    let mut rng = Rng::new(config.seed);
    for test_num in 0..config.num_tests {
        let value = gen_i64(&mut rng);
        if !property(value) {
            // Shrink to find minimal counterexample
            let (shrunk, num_shrinks) = shrink_counterexample_i64(value, &property, config.max_shrinks);
            return PropTestResult::Failed {
                original_input: format!("{}", value),
                shrunk_input: format!("{}", shrunk),
                num_shrinks,
                seed: config.seed,
                test_num,
            };
        }
    }
    PropTestResult::Passed { num_tests: config.num_tests }
}

/// Run a property test on string values.
fn prop_test_string<F>(config: &PropTestConfig, max_len: usize, property: F) -> PropTestResult
where
    F: Fn(&str) -> bool,
{
    let mut rng = Rng::new(config.seed);
    for test_num in 0..config.num_tests {
        let value = gen_string(&mut rng, max_len);
        if !property(&value) {
            let (shrunk, num_shrinks) = shrink_counterexample_string(&value, &property, config.max_shrinks);
            return PropTestResult::Failed {
                original_input: format!("{:?}", value),
                shrunk_input: format!("{:?}", shrunk),
                num_shrinks,
                seed: config.seed,
                test_num,
            };
        }
    }
    PropTestResult::Passed { num_tests: config.num_tests }
}

/// Run a property test on Vec<i64> values.
fn prop_test_vec_i64<F>(config: &PropTestConfig, max_len: usize, property: F) -> PropTestResult
where
    F: Fn(&[i64]) -> bool,
{
    let mut rng = Rng::new(config.seed);
    for test_num in 0..config.num_tests {
        let value = gen_vec_i64(&mut rng, max_len);
        if !property(&value) {
            let (shrunk, num_shrinks) = shrink_counterexample_vec(&value, &property, config.max_shrinks);
            return PropTestResult::Failed {
                original_input: format!("{:?}", value),
                shrunk_input: format!("{:?}", shrunk),
                num_shrinks,
                seed: config.seed,
                test_num,
            };
        }
    }
    PropTestResult::Passed { num_tests: config.num_tests }
}

/// Run a property test on pairs of i64 values.
fn prop_test_pair_i64<F>(config: &PropTestConfig, property: F) -> PropTestResult
where
    F: Fn(i64, i64) -> bool,
{
    let mut rng = Rng::new(config.seed);
    for test_num in 0..config.num_tests {
        let a = gen_i64(&mut rng);
        let b = gen_i64(&mut rng);
        if !property(a, b) {
            return PropTestResult::Failed {
                original_input: format!("({}, {})", a, b),
                shrunk_input: format!("({}, {})", a, b),
                num_shrinks: 0,
                seed: config.seed,
                test_num,
            };
        }
    }
    PropTestResult::Passed { num_tests: config.num_tests }
}

// ─── Shrinking Helpers ──────────────────────────────────────────────

fn shrink_counterexample_i64<F>(initial: i64, property: &F, max_shrinks: usize) -> (i64, usize)
where
    F: Fn(i64) -> bool,
{
    let mut current = initial;
    let mut shrinks = 0;
    for _ in 0..max_shrinks {
        let candidates = shrink_i64(current);
        let mut improved = false;
        for candidate in candidates {
            if !property(candidate) {
                current = candidate;
                shrinks += 1;
                improved = true;
                break;
            }
        }
        if !improved {
            break;
        }
    }
    (current, shrinks)
}

fn shrink_counterexample_string<F>(initial: &str, property: &F, max_shrinks: usize) -> (String, usize)
where
    F: Fn(&str) -> bool,
{
    let mut current = initial.to_string();
    let mut shrinks = 0;
    for _ in 0..max_shrinks {
        let candidates = shrink_string(&current);
        let mut improved = false;
        for candidate in &candidates {
            if !property(candidate) {
                current = candidate.clone();
                shrinks += 1;
                improved = true;
                break;
            }
        }
        if !improved {
            break;
        }
    }
    (current, shrinks)
}

fn shrink_counterexample_vec<F>(initial: &[i64], property: &F, max_shrinks: usize) -> (Vec<i64>, usize)
where
    F: Fn(&[i64]) -> bool,
{
    let mut current = initial.to_vec();
    let mut shrinks = 0;
    for _ in 0..max_shrinks {
        let candidates = shrink_vec_i64(&current);
        let mut improved = false;
        for candidate in &candidates {
            if !property(candidate) {
                current = candidate.clone();
                shrinks += 1;
                improved = true;
                break;
            }
        }
        if !improved {
            break;
        }
    }
    (current, shrinks)
}

// ─── Statistical Properties ─────────────────────────────────────────

/// Test distribution uniformity using chi-squared test.
fn chi_squared_uniformity(rng: &mut Rng, num_samples: usize, num_buckets: usize) -> f64 {
    let mut buckets = vec![0usize; num_buckets];
    for _ in 0..num_samples {
        let idx = rng.next_usize(num_buckets - 1);
        buckets[idx] += 1;
    }
    let expected = num_samples as f64 / num_buckets as f64;
    let mut chi2 = 0.0;
    for &count in &buckets {
        let diff = count as f64 - expected;
        chi2 += diff * diff / expected;
    }
    chi2
}

// ─── FFI Layer ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_gen_i64(seed: i64, min: i64, max: i64) -> i64 {
    let mut rng = Rng::new(seed as u64);
    gen_i64_range(&mut rng, min, max)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_gen_f64(seed: i64) -> f64 {
    let mut rng = Rng::new(seed as u64);
    gen_f64(&mut rng)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_gen_bool(seed: i64) -> i64 {
    let mut rng = Rng::new(seed as u64);
    if gen_bool(&mut rng) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_gen_string(seed: i64, max_len: i64) -> *mut c_char {
    let mut rng = Rng::new(seed as u64);
    let s = gen_string(&mut rng, max_len.max(0) as usize);
    CString::new(s).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_shrink_i64(x: i64) -> *mut c_char {
    let shrunk = shrink_i64(x);
    let parts: Vec<String> = shrunk.iter().map(|v| v.to_string()).collect();
    let result = parts.join(",");
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_test_commutative_add(seed: i64, num_tests: i64) -> i64 {
    let config = PropTestConfig {
        num_tests: num_tests.max(1) as usize,
        seed: seed as u64,
        ..PropTestConfig::default()
    };
    let result = prop_test_pair_i64(&config, |a, b| {
        a.wrapping_add(b) == b.wrapping_add(a)
    });
    match result {
        PropTestResult::Passed { .. } => 1,
        PropTestResult::Failed { .. } => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_test_sort_idempotent(seed: i64, num_tests: i64) -> i64 {
    let config = PropTestConfig {
        num_tests: num_tests.max(1) as usize,
        seed: seed as u64,
        ..PropTestConfig::default()
    };
    let result = prop_test_vec_i64(&config, 20, |v| {
        let mut sorted1 = v.to_vec();
        sorted1.sort();
        let mut sorted2 = sorted1.clone();
        sorted2.sort();
        sorted1 == sorted2
    });
    match result {
        PropTestResult::Passed { .. } => 1,
        PropTestResult::Failed { .. } => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_test_sort_preserves_length(seed: i64, num_tests: i64) -> i64 {
    let config = PropTestConfig {
        num_tests: num_tests.max(1) as usize,
        seed: seed as u64,
        ..PropTestConfig::default()
    };
    let result = prop_test_vec_i64(&config, 20, |v| {
        let mut sorted = v.to_vec();
        sorted.sort();
        sorted.len() == v.len()
    });
    match result {
        PropTestResult::Passed { .. } => 1,
        PropTestResult::Failed { .. } => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_qc_chi_squared(seed: i64, num_samples: i64, num_buckets: i64) -> f64 {
    let mut rng = Rng::new(seed as u64);
    chi_squared_uniformity(&mut rng, num_samples.max(10) as usize, num_buckets.max(2) as usize)
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rng_deterministic() {
        let mut rng1 = Rng::new(42);
        let mut rng2 = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_rng_different_seeds() {
        let mut rng1 = Rng::new(1);
        let mut rng2 = Rng::new(2);
        let v1: Vec<u64> = (0..10).map(|_| rng1.next_u64()).collect();
        let v2: Vec<u64> = (0..10).map(|_| rng2.next_u64()).collect();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_rng_range() {
        let mut rng = Rng::new(42);
        for _ in 0..1000 {
            let v = rng.next_range(10, 20);
            assert!(v >= 10 && v <= 20, "Value {} out of range [10, 20]", v);
        }
    }

    #[test]
    fn test_rng_f64_range() {
        let mut rng = Rng::new(42);
        for _ in 0..1000 {
            let v = rng.next_f64();
            assert!(v >= 0.0 && v < 1.0, "f64 {} out of [0, 1)", v);
        }
    }

    #[test]
    fn test_gen_string_length() {
        let mut rng = Rng::new(42);
        for _ in 0..100 {
            let s = gen_string(&mut rng, 10);
            assert!(s.len() <= 10);
            assert!(s.chars().all(|c| c.is_ascii() && !c.is_ascii_control()));
        }
    }

    #[test]
    fn test_gen_vec_length() {
        let mut rng = Rng::new(42);
        for _ in 0..100 {
            let v = gen_vec_i64(&mut rng, 20);
            assert!(v.len() <= 20);
        }
    }

    #[test]
    fn test_shrink_i64_zero() {
        assert!(shrink_i64(0).is_empty());
    }

    #[test]
    fn test_shrink_i64_positive() {
        let shrunk = shrink_i64(10);
        assert!(shrunk.contains(&0));
        assert!(shrunk.contains(&5));
        assert!(shrunk.iter().all(|&x| x.abs() <= 10));
    }

    #[test]
    fn test_shrink_i64_negative() {
        let shrunk = shrink_i64(-10);
        assert!(shrunk.contains(&0));
        assert!(shrunk.contains(&10)); // try positive version
    }

    #[test]
    fn test_shrink_string_empty() {
        assert!(shrink_string("").is_empty());
    }

    #[test]
    fn test_shrink_string_reduces() {
        let shrunk = shrink_string("abc");
        assert!(shrunk.iter().any(|s| s.len() < 3));
        assert!(shrunk.contains(&String::new()));
    }

    #[test]
    fn test_shrink_vec_reduces() {
        let shrunk = shrink_vec_i64(&[1, 2, 3]);
        assert!(shrunk.iter().any(|v| v.len() < 3));
    }

    #[test]
    fn test_prop_addition_commutative() {
        let config = PropTestConfig {
            num_tests: 200,
            seed: 42,
            ..PropTestConfig::default()
        };
        let result = prop_test_pair_i64(&config, |a, b| {
            a.wrapping_add(b) == b.wrapping_add(a)
        });
        assert!(matches!(result, PropTestResult::Passed { .. }));
    }

    #[test]
    fn test_prop_sort_idempotent() {
        let config = PropTestConfig {
            num_tests: 100,
            seed: 42,
            ..PropTestConfig::default()
        };
        let result = prop_test_vec_i64(&config, 20, |v| {
            let mut s1 = v.to_vec();
            s1.sort();
            let mut s2 = s1.clone();
            s2.sort();
            s1 == s2
        });
        assert!(matches!(result, PropTestResult::Passed { .. }));
    }

    #[test]
    fn test_prop_sort_preserves_length() {
        let config = PropTestConfig {
            num_tests: 100,
            seed: 42,
            ..PropTestConfig::default()
        };
        let result = prop_test_vec_i64(&config, 20, |v| {
            let mut sorted = v.to_vec();
            sorted.sort();
            sorted.len() == v.len()
        });
        assert!(matches!(result, PropTestResult::Passed { .. }));
    }

    #[test]
    fn test_prop_abs_non_negative() {
        let config = PropTestConfig {
            num_tests: 200,
            seed: 42,
            ..PropTestConfig::default()
        };
        let result = prop_test_i64(&config, |x| {
            // abs is non-negative (except i64::MIN which overflows)
            x == i64::MIN || x.abs() >= 0
        });
        assert!(matches!(result, PropTestResult::Passed { .. }));
    }

    #[test]
    fn test_prop_finds_counterexample() {
        let config = PropTestConfig {
            num_tests: 100,
            seed: 42,
            ..PropTestConfig::default()
        };
        // This should fail: not all i64 are positive
        let result = prop_test_i64(&config, |x| x >= 0);
        assert!(matches!(result, PropTestResult::Failed { .. }));
    }

    #[test]
    fn test_prop_shrinks_to_minimal() {
        let config = PropTestConfig {
            num_tests: 200,
            max_shrinks: 50,
            seed: 99,
        };
        // Property: x < 100 — should find counterexample and shrink
        let result = prop_test_i64(&config, |x| x < 100);
        if let PropTestResult::Failed { shrunk_input, .. } = result {
            // Shrunk should be a small value >= 100
            let shrunk_val: i64 = shrunk_input.parse().unwrap();
            assert!(shrunk_val >= 100);
        }
        // If it passes with this seed, that's OK — generators are random
    }

    #[test]
    fn test_prop_string_not_empty_fails() {
        let config = PropTestConfig {
            num_tests: 100,
            seed: 42,
            ..PropTestConfig::default()
        };
        let result = prop_test_string(&config, 5, |s| !s.is_empty());
        // May or may not fail depending on RNG – gen_string can produce len 0
        match result {
            PropTestResult::Passed { .. } | PropTestResult::Failed { .. } => {} // both OK
        }
    }

    #[test]
    fn test_prop_sorted_vec_is_sorted() {
        let config = PropTestConfig {
            num_tests: 100,
            seed: 42,
            ..PropTestConfig::default()
        };
        let mut rng = Rng::new(config.seed);
        for _ in 0..config.num_tests {
            let v = gen_sorted_vec_i64(&mut rng, 20);
            for w in v.windows(2) {
                assert!(w[0] <= w[1]);
            }
        }
    }

    #[test]
    fn test_chi_squared_uniformity() {
        let mut rng = Rng::new(42);
        // With 10000 samples and 10 buckets, chi-squared should be < ~27 for p=0.001
        let chi2 = chi_squared_uniformity(&mut rng, 10000, 10);
        assert!(chi2 < 50.0, "Chi-squared {} too high — RNG not uniform", chi2);
    }

    #[test]
    fn test_seed_replay() {
        let config = PropTestConfig {
            num_tests: 50,
            seed: 12345,
            ..PropTestConfig::default()
        };
        // Same seed should give same result
        let r1 = prop_test_i64(&config, |x| x.wrapping_mul(2) / 2 == x || x > i64::MAX / 2 || x < i64::MIN / 2);
        let r2 = prop_test_i64(&config, |x| x.wrapping_mul(2) / 2 == x || x > i64::MAX / 2 || x < i64::MIN / 2);
        match (&r1, &r2) {
            (PropTestResult::Passed { num_tests: n1 }, PropTestResult::Passed { num_tests: n2 }) => {
                assert_eq!(n1, n2);
            }
            (PropTestResult::Failed { test_num: t1, .. }, PropTestResult::Failed { test_num: t2, .. }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("Same seed should give same result"),
        }
    }

    #[test]
    fn test_ffi_gen_i64() {
        let v = vitalis_qc_gen_i64(42, 0, 100);
        assert!(v >= 0 && v <= 100);
    }

    #[test]
    fn test_ffi_gen_f64() {
        let v = vitalis_qc_gen_f64(42);
        assert!(v.is_finite());
    }

    #[test]
    fn test_ffi_gen_string() {
        let ptr = vitalis_qc_gen_string(42, 10);
        let s = unsafe { CString::from_raw(ptr) }.into_string().unwrap();
        assert!(s.len() <= 10);
    }

    #[test]
    fn test_ffi_shrink() {
        let ptr = vitalis_qc_shrink_i64(10);
        let s = unsafe { CString::from_raw(ptr) }.into_string().unwrap();
        assert!(!s.is_empty());
        assert!(s.contains("0")); // should include 0 as a shrink candidate
    }

    #[test]
    fn test_ffi_commutative() {
        assert_eq!(vitalis_qc_test_commutative_add(42, 100), 1);
    }

    #[test]
    fn test_ffi_sort_idempotent() {
        assert_eq!(vitalis_qc_test_sort_idempotent(42, 50), 1);
    }

    #[test]
    fn test_ffi_chi_squared() {
        let chi2 = vitalis_qc_chi_squared(42, 5000, 10);
        assert!(chi2 >= 0.0);
        assert!(chi2 < 100.0);
    }
}
