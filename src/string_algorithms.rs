//! String Algorithms Module — Text processing and pattern matching for Vitalis
//!
//! Pure Rust implementations with zero external dependencies.
//! Exposed via C FFI for Python interop.
//!
//! # Algorithms:
//! - KMP (Knuth-Morris-Pratt) pattern matching
//! - Rabin-Karp rolling hash pattern matching
//! - Levenshtein edit distance
//! - Longest Common Subsequence (LCS)
//! - Longest Common Substring
//! - Hamming distance
//! - Jaro-Winkler similarity
//! - Soundex phonetic encoding
//! - Run-length encoding/decoding
//! - Boyer-Moore-Horspool pattern matching
//! - String rotation check

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── KMP Pattern Matching ─────────────────────────────────────────────

fn kmp_failure(pattern: &[u8]) -> Vec<usize> {
    let m = pattern.len();
    let mut fail = vec![0usize; m];
    let mut j = 0;
    for i in 1..m {
        while j > 0 && pattern[i] != pattern[j] {
            j = fail[j - 1];
        }
        if pattern[i] == pattern[j] {
            j += 1;
        }
        fail[i] = j;
    }
    fail
}

fn kmp_search(text: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() { return vec![]; }
    let fail = kmp_failure(pattern);
    let mut matches = Vec::new();
    let mut j = 0;
    for i in 0..text.len() {
        while j > 0 && text[i] != pattern[j] {
            j = fail[j - 1];
        }
        if text[i] == pattern[j] {
            j += 1;
        }
        if j == pattern.len() {
            matches.push(i + 1 - pattern.len());
            j = fail[j - 1];
        }
    }
    matches
}

/// KMP pattern matching — find all occurrences.
/// Returns number of matches, fills out_positions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kmp_search(
    text: *const c_char,
    pattern: *const c_char,
    out_positions: *mut usize,
    max_results: usize,
) -> usize {
    if text.is_null() || pattern.is_null() { return 0; }
    let t = unsafe { CStr::from_ptr(text) }.to_bytes();
    let p = unsafe { CStr::from_ptr(pattern) }.to_bytes();
    let matches = kmp_search(t, p);
    let count = matches.len().min(max_results);
    if !out_positions.is_null() {
        let out = unsafe { std::slice::from_raw_parts_mut(out_positions, count) };
        for (i, &pos) in matches.iter().take(count).enumerate() {
            out[i] = pos;
        }
    }
    matches.len()
}

// ─── Rabin-Karp ───────────────────────────────────────────────────────

fn rabin_karp_search(text: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() || text.len() < pattern.len() { return vec![]; }
    let base: u64 = 256;
    let modulus: u64 = 1_000_000_007;
    let m = pattern.len();

    let mut p_hash: u64 = 0;
    let mut t_hash: u64 = 0;
    let mut h: u64 = 1;

    for _ in 0..m-1 {
        h = (h * base) % modulus;
    }
    for i in 0..m {
        p_hash = (base * p_hash + pattern[i] as u64) % modulus;
        t_hash = (base * t_hash + text[i] as u64) % modulus;
    }

    let mut matches = Vec::new();
    for i in 0..=text.len()-m {
        if p_hash == t_hash && &text[i..i+m] == pattern {
            matches.push(i);
        }
        if i < text.len() - m {
            t_hash = (base * (t_hash + modulus - (text[i] as u64 * h) % modulus) + text[i+m] as u64) % modulus;
        }
    }
    matches
}

/// Rabin-Karp pattern matching. Returns number of matches.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rabin_karp(
    text: *const c_char,
    pattern: *const c_char,
    out_positions: *mut usize,
    max_results: usize,
) -> usize {
    if text.is_null() || pattern.is_null() { return 0; }
    let t = unsafe { CStr::from_ptr(text) }.to_bytes();
    let p = unsafe { CStr::from_ptr(pattern) }.to_bytes();
    let matches = rabin_karp_search(t, p);
    let count = matches.len().min(max_results);
    if !out_positions.is_null() {
        let out = unsafe { std::slice::from_raw_parts_mut(out_positions, count) };
        for (i, &pos) in matches.iter().take(count).enumerate() {
            out[i] = pos;
        }
    }
    matches.len()
}

// ─── Levenshtein Distance ─────────────────────────────────────────────

fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i-1] == b[j-1] { 0 } else { 1 };
            dp[i][j] = (dp[i-1][j] + 1)
                .min(dp[i][j-1] + 1)
                .min(dp[i-1][j-1] + cost);
        }
    }
    dp[m][n]
}

/// Levenshtein edit distance between two strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_levenshtein(
    a: *const c_char,
    b: *const c_char,
) -> usize {
    if a.is_null() || b.is_null() { return 0; }
    let sa = unsafe { CStr::from_ptr(a) }.to_bytes();
    let sb = unsafe { CStr::from_ptr(b) }.to_bytes();
    levenshtein(sa, sb)
}

// ─── Longest Common Subsequence ───────────────────────────────────────

fn lcs_length(a: &[u8], b: &[u8]) -> usize {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i-1] == b[j-1] {
                dp[i][j] = dp[i-1][j-1] + 1;
            } else {
                dp[i][j] = dp[i-1][j].max(dp[i][j-1]);
            }
        }
    }
    dp[m][n]
}

fn lcs_string(a: &[u8], b: &[u8]) -> Vec<u8> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i-1] == b[j-1] {
                dp[i][j] = dp[i-1][j-1] + 1;
            } else {
                dp[i][j] = dp[i-1][j].max(dp[i][j-1]);
            }
        }
    }
    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if a[i-1] == b[j-1] {
            result.push(a[i-1]);
            i -= 1;
            j -= 1;
        } else if dp[i-1][j] > dp[i][j-1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

/// LCS length between two strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lcs_length(
    a: *const c_char,
    b: *const c_char,
) -> usize {
    if a.is_null() || b.is_null() { return 0; }
    let sa = unsafe { CStr::from_ptr(a) }.to_bytes();
    let sb = unsafe { CStr::from_ptr(b) }.to_bytes();
    lcs_length(sa, sb)
}

/// LCS string. Returns the actual subsequence. Caller frees with slang_free_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lcs_string(
    a: *const c_char,
    b: *const c_char,
) -> *mut c_char {
    if a.is_null() || b.is_null() { return CString::new("").unwrap().into_raw(); }
    let sa = unsafe { CStr::from_ptr(a) }.to_bytes();
    let sb = unsafe { CStr::from_ptr(b) }.to_bytes();
    let result = lcs_string(sa, sb);
    let s = String::from_utf8_lossy(&result).to_string();
    CString::new(s).unwrap().into_raw()
}

// ─── Longest Common Substring ─────────────────────────────────────────

fn longest_common_substring(a: &[u8], b: &[u8]) -> (usize, usize, usize) {
    // Returns (length, start_in_a, start_in_b)
    let m = a.len();
    let n = b.len();
    let mut max_len = 0;
    let mut end_a = 0;
    let mut end_b = 0;
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i-1] == b[j-1] {
                dp[i][j] = dp[i-1][j-1] + 1;
                if dp[i][j] > max_len {
                    max_len = dp[i][j];
                    end_a = i;
                    end_b = j;
                }
            }
        }
    }
    let _ = end_b; // suppress unused
    (max_len, end_a - max_len, end_a)
}

/// Longest common substring length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_longest_common_substring(
    a: *const c_char,
    b: *const c_char,
) -> usize {
    if a.is_null() || b.is_null() { return 0; }
    let sa = unsafe { CStr::from_ptr(a) }.to_bytes();
    let sb = unsafe { CStr::from_ptr(b) }.to_bytes();
    let (len, _, _) = longest_common_substring(sa, sb);
    len
}

// ─── Hamming Distance ─────────────────────────────────────────────────

/// Hamming distance between equal-length strings.
/// Returns -1 if lengths differ.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_hamming_distance(
    a: *const c_char,
    b: *const c_char,
) -> i64 {
    if a.is_null() || b.is_null() { return -1; }
    let sa = unsafe { CStr::from_ptr(a) }.to_bytes();
    let sb = unsafe { CStr::from_ptr(b) }.to_bytes();
    if sa.len() != sb.len() { return -1; }
    sa.iter().zip(sb.iter()).filter(|&(&x, &y)| x != y).count() as i64
}

// ─── Jaro-Winkler Similarity ─────────────────────────────────────────

fn jaro_similarity(s1: &[u8], s2: &[u8]) -> f64 {
    if s1.is_empty() && s2.is_empty() { return 1.0; }
    if s1.is_empty() || s2.is_empty() { return 0.0; }

    let match_distance = (s1.len().max(s2.len()) / 2).max(1) - 1;
    let mut s1_matches = vec![false; s1.len()];
    let mut s2_matches = vec![false; s2.len()];

    let mut matches = 0.0f64;
    let mut transpositions = 0.0f64;

    for i in 0..s1.len() {
        let start = if i > match_distance { i - match_distance } else { 0 };
        let end = (i + match_distance + 1).min(s2.len());
        for j in start..end {
            if s2_matches[j] || s1[i] != s2[j] { continue; }
            s1_matches[i] = true;
            s2_matches[j] = true;
            matches += 1.0;
            break;
        }
    }

    if matches == 0.0 { return 0.0; }

    let mut k = 0;
    for i in 0..s1.len() {
        if !s1_matches[i] { continue; }
        while !s2_matches[k] { k += 1; }
        if s1[i] != s2[k] { transpositions += 1.0; }
        k += 1;
    }

    (matches / s1.len() as f64 + matches / s2.len() as f64
     + (matches - transpositions / 2.0) / matches) / 3.0
}

/// Jaro-Winkler similarity (0.0 to 1.0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_jaro_winkler(
    a: *const c_char,
    b: *const c_char,
    prefix_weight: f64,
) -> f64 {
    if a.is_null() || b.is_null() { return 0.0; }
    let sa = unsafe { CStr::from_ptr(a) }.to_bytes();
    let sb = unsafe { CStr::from_ptr(b) }.to_bytes();

    let jaro = jaro_similarity(sa, sb);
    let prefix_len = sa.iter().zip(sb.iter())
        .take(4)
        .take_while(|&(&x, &y)| x == y)
        .count();
    let pw = if prefix_weight <= 0.0 || prefix_weight > 0.25 { 0.1 } else { prefix_weight };
    jaro + (prefix_len as f64 * pw * (1.0 - jaro))
}

// ─── Soundex ──────────────────────────────────────────────────────────

fn soundex(word: &str) -> String {
    if word.is_empty() { return "0000".to_string(); }
    let chars: Vec<u8> = word.to_uppercase().bytes().filter(|b| b.is_ascii_alphabetic()).collect();
    if chars.is_empty() { return "0000".to_string(); }

    let code = |c: u8| -> u8 {
        match c {
            b'B' | b'F' | b'P' | b'V' => b'1',
            b'C' | b'G' | b'J' | b'K' | b'Q' | b'S' | b'X' | b'Z' => b'2',
            b'D' | b'T' => b'3',
            b'L' => b'4',
            b'M' | b'N' => b'5',
            b'R' => b'6',
            _ => b'0',
        }
    };

    let mut result = vec![chars[0]];
    let mut last_code = code(chars[0]);
    for &c in &chars[1..] {
        let cd = code(c);
        if cd != b'0' && cd != last_code {
            result.push(cd);
            if result.len() == 4 { break; }
        }
        last_code = cd;
    }
    while result.len() < 4 { result.push(b'0'); }
    String::from_utf8(result).unwrap_or_else(|_| "0000".to_string())
}

/// Soundex phonetic encoding. Returns 4-char code. Caller frees.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_soundex(
    word: *const c_char,
) -> *mut c_char {
    if word.is_null() { return CString::new("0000").unwrap().into_raw(); }
    let s = unsafe { CStr::from_ptr(word) }.to_str().unwrap_or("");
    let code = soundex(s);
    CString::new(code).unwrap().into_raw()
}

// ─── Boyer-Moore-Horspool ─────────────────────────────────────────────

fn bmh_search(text: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() || text.len() < pattern.len() { return vec![]; }
    let m = pattern.len();

    // Build bad character table
    let mut skip = [m; 256];
    for i in 0..m-1 {
        skip[pattern[i] as usize] = m - 1 - i;
    }

    let mut matches = Vec::new();
    let mut i = 0;
    while i <= text.len() - m {
        let mut j = m - 1;
        while text[i + j] == pattern[j] {
            if j == 0 {
                matches.push(i);
                break;
            }
            j -= 1;
        }
        i += skip[text[i + m - 1] as usize];
    }
    matches
}

/// Boyer-Moore-Horspool search. Returns number of matches.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bmh_search(
    text: *const c_char,
    pattern: *const c_char,
    out_positions: *mut usize,
    max_results: usize,
) -> usize {
    if text.is_null() || pattern.is_null() { return 0; }
    let t = unsafe { CStr::from_ptr(text) }.to_bytes();
    let p = unsafe { CStr::from_ptr(pattern) }.to_bytes();
    let matches = bmh_search(t, p);
    let count = matches.len().min(max_results);
    if !out_positions.is_null() {
        let out = unsafe { std::slice::from_raw_parts_mut(out_positions, count) };
        for (i, &pos) in matches.iter().take(count).enumerate() {
            out[i] = pos;
        }
    }
    matches.len()
}

// ─── String Rotation Check ───────────────────────────────────────────

/// Check if b is a rotation of a (e.g., "abcde" → "cdeab").
/// Returns 1 if rotation, 0 if not.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_is_rotation(
    a: *const c_char,
    b: *const c_char,
) -> i32 {
    if a.is_null() || b.is_null() { return 0; }
    let sa = unsafe { CStr::from_ptr(a) }.to_bytes();
    let sb = unsafe { CStr::from_ptr(b) }.to_bytes();
    if sa.len() != sb.len() { return 0; }
    if sa.is_empty() { return 1; }
    // Concatenate a+a and search for b
    let doubled: Vec<u8> = [sa, sa].concat();
    if kmp_search(&doubled, sb).is_empty() { 0 } else { 1 }
}

// ─── N-gram generation ───────────────────────────────────────────────

/// Generate character n-grams count. Returns number of unique n-grams.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ngram_count(
    text: *const c_char,
    n: usize,
) -> usize {
    if text.is_null() || n == 0 { return 0; }
    let s = unsafe { CStr::from_ptr(text) }.to_bytes();
    if s.len() < n { return 0; }
    let mut seen = std::collections::HashSet::new();
    for window in s.windows(n) {
        seen.insert(window);
    }
    seen.len()
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kmp_basic() {
        let matches = kmp_search(b"abcabcabc", b"abc");
        assert_eq!(matches, vec![0, 3, 6]);
    }

    #[test]
    fn test_kmp_no_match() {
        let matches = kmp_search(b"hello world", b"xyz");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_rabin_karp() {
        let matches = rabin_karp_search(b"aabaabaa", b"aab");
        assert_eq!(matches, vec![0, 3]);
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein(b"kitten", b"sitting"), 3);
        assert_eq!(levenshtein(b"", b"abc"), 3);
        assert_eq!(levenshtein(b"same", b"same"), 0);
    }

    #[test]
    fn test_lcs() {
        assert_eq!(lcs_length(b"ABCBDAB", b"BDCAB"), 4);
        let s = lcs_string(b"ABCBDAB", b"BDCAB");
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn test_longest_common_substring() {
        let (len, _, _) = longest_common_substring(b"ABABC", b"BABCB");
        assert_eq!(len, 4); // "BABC"
    }

    #[test]
    fn test_hamming() {
        let a = std::ffi::CString::new("karolin").unwrap();
        let b = std::ffi::CString::new("kathrin").unwrap();
        let d = unsafe { vitalis_hamming_distance(a.as_ptr(), b.as_ptr()) };
        assert_eq!(d, 3);
    }

    #[test]
    fn test_jaro_winkler() {
        let a = std::ffi::CString::new("MARTHA").unwrap();
        let b = std::ffi::CString::new("MARHTA").unwrap();
        let sim = unsafe { vitalis_jaro_winkler(a.as_ptr(), b.as_ptr(), 0.1) };
        assert!(sim > 0.95);
    }

    #[test]
    fn test_soundex() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
    }

    #[test]
    fn test_bmh_search() {
        let matches = bmh_search(b"hello world hello", b"hello");
        assert_eq!(matches, vec![0, 12]);
    }

    #[test]
    fn test_is_rotation() {
        let a = std::ffi::CString::new("abcde").unwrap();
        let b = std::ffi::CString::new("cdeab").unwrap();
        let c = std::ffi::CString::new("abced").unwrap();
        assert_eq!(unsafe { vitalis_is_rotation(a.as_ptr(), b.as_ptr()) }, 1);
        assert_eq!(unsafe { vitalis_is_rotation(a.as_ptr(), c.as_ptr()) }, 0);
    }

    #[test]
    fn test_ngram_count() {
        let s = std::ffi::CString::new("abcabc").unwrap();
        let count = unsafe { vitalis_ngram_count(s.as_ptr(), 2) };
        assert_eq!(count, 3); // "ab", "bc", "ca"
    }
}
