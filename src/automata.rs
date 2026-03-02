//! Automata & Pattern Matching Module for Vitalis v10.0
//!
//! Pure Rust implementations: Aho-Corasick multi-pattern search, DFA/NFA regex,
//! Bloom filter, Count-Min Sketch, Trie, and finite state machines.

use std::collections::{HashMap, VecDeque};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── Aho-Corasick Multi-Pattern Search ──────────────────────────────

/// Internal Aho-Corasick automaton.
struct AhoCorasick {
    goto: Vec<HashMap<u8, usize>>,
    fail: Vec<usize>,
    output: Vec<Vec<usize>>,  // pattern indices at each state
    num_states: usize,
}

impl AhoCorasick {
    fn build(patterns: &[&[u8]]) -> Self {
        let mut goto: Vec<HashMap<u8, usize>> = vec![HashMap::new()];
        let mut output: Vec<Vec<usize>> = vec![vec![]];
        let mut num_states = 1;

        // Build trie
        for (pi, pat) in patterns.iter().enumerate() {
            let mut state = 0;
            for &ch in *pat {
                if !goto[state].contains_key(&ch) {
                    goto.push(HashMap::new());
                    output.push(vec![]);
                    goto[state].insert(ch, num_states);
                    num_states += 1;
                }
                state = goto[state][&ch];
            }
            output[state].push(pi);
        }

        // Build failure links (BFS)
        let mut fail = vec![0; num_states];
        let mut queue = VecDeque::new();
        for &s in goto[0].values() {
            fail[s] = 0;
            queue.push_back(s);
        }
        while let Some(u) = queue.pop_front() {
            let chars: Vec<(u8, usize)> = goto[u].iter().map(|(&c, &s)| (c, s)).collect();
            for (ch, v) in chars {
                queue.push_back(v);
                let mut f = fail[u];
                while f != 0 && !goto[f].contains_key(&ch) { f = fail[f]; }
                fail[v] = *goto[f].get(&ch).unwrap_or(&0);
                if fail[v] == v { fail[v] = 0; }
                let fail_out = output[fail[v]].clone();
                output[v].extend(fail_out);
            }
        }

        AhoCorasick { goto, fail, output, num_states }
    }

    fn search(&self, text: &[u8]) -> Vec<(usize, usize)> {
        // Returns (position, pattern_index) pairs
        let mut results = Vec::new();
        let mut state = 0;
        for (i, &ch) in text.iter().enumerate() {
            while state != 0 && !self.goto[state].contains_key(&ch) {
                state = self.fail[state];
            }
            state = *self.goto[state].get(&ch).unwrap_or(&0);
            for &pi in &self.output[state] {
                results.push((i, pi));
            }
        }
        results
    }
}

/// Aho-Corasick multi-pattern search.
/// `text` is the haystack, `patterns` is a null-terminated array of C strings.
/// Returns number of matches found. Writes match positions to `out_positions` and
/// pattern indices to `out_pattern_ids` (both must be at least `max_matches` long).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_aho_corasick(
    text: *const u8, text_len: usize,
    patterns: *const *const u8, pattern_lens: *const usize, n_patterns: usize,
    out_positions: *mut usize, out_pattern_ids: *mut usize, max_matches: usize,
) -> i32 {
    if text.is_null() || patterns.is_null() || pattern_lens.is_null() || n_patterns == 0 {
        return 0;
    }
    let text_bytes = unsafe { std::slice::from_raw_parts(text, text_len) };
    let pat_ptrs = unsafe { std::slice::from_raw_parts(patterns, n_patterns) };
    let pat_lens = unsafe { std::slice::from_raw_parts(pattern_lens, n_patterns) };

    let pats: Vec<&[u8]> = (0..n_patterns)
        .map(|i| unsafe { std::slice::from_raw_parts(pat_ptrs[i], pat_lens[i]) })
        .collect();

    let ac = AhoCorasick::build(&pats);
    let matches = ac.search(text_bytes);

    let count = matches.len().min(max_matches);
    if !out_positions.is_null() && !out_pattern_ids.is_null() {
        let positions = unsafe { std::slice::from_raw_parts_mut(out_positions, count) };
        let ids = unsafe { std::slice::from_raw_parts_mut(out_pattern_ids, count) };
        for (i, &(pos, pid)) in matches.iter().take(count).enumerate() {
            positions[i] = pos;
            ids[i] = pid;
        }
    }
    matches.len() as i32
}

// ─── Bloom Filter ───────────────────────────────────────────────────

/// Bloom filter state (opaque, managed via FFI).
struct BloomFilter {
    bits: Vec<bool>,
    num_hashes: usize,
    size: usize,
}

impl BloomFilter {
    fn new(size: usize, num_hashes: usize) -> Self {
        BloomFilter { bits: vec![false; size], num_hashes, size }
    }

    fn hash(&self, item: u64, seed: usize) -> usize {
        let mut h = item.wrapping_mul(6364136223846793005).wrapping_add(seed as u64 * 1442695040888963407);
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        (h as usize) % self.size
    }

    fn insert(&mut self, item: u64) {
        for i in 0..self.num_hashes {
            let idx = self.hash(item, i);
            self.bits[idx] = true;
        }
    }

    fn contains(&self, item: u64) -> bool {
        (0..self.num_hashes).all(|i| self.bits[self.hash(item, i)])
    }
}

/// Create a new Bloom filter. Returns opaque pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bloom_new(size: usize, num_hashes: usize) -> *mut BloomFilter {
    let bf = Box::new(BloomFilter::new(size, num_hashes));
    Box::into_raw(bf)
}

/// Insert item into Bloom filter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bloom_insert(bf: *mut BloomFilter, item: u64) {
    if !bf.is_null() { unsafe { (*bf).insert(item); } }
}

/// Check if item might be in Bloom filter. Returns 1 if possibly yes, 0 if definitely no.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bloom_contains(bf: *const BloomFilter, item: u64) -> i32 {
    if bf.is_null() { return 0; }
    if unsafe { (*bf).contains(item) } { 1 } else { 0 }
}

/// Free Bloom filter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bloom_free(bf: *mut BloomFilter) {
    if !bf.is_null() { unsafe { drop(Box::from_raw(bf)); } }
}

// ─── Count-Min Sketch ───────────────────────────────────────────────

struct CountMinSketch {
    table: Vec<Vec<u64>>,
    width: usize,
    depth: usize,
}

impl CountMinSketch {
    fn new(width: usize, depth: usize) -> Self {
        CountMinSketch {
            table: vec![vec![0; width]; depth],
            width,
            depth,
        }
    }

    fn hash(&self, item: u64, row: usize) -> usize {
        let mut h = item.wrapping_mul(0x517cc1b727220a95).wrapping_add((row as u64).wrapping_mul(0x6c62272e07bb0142));
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        (h as usize) % self.width
    }

    fn add(&mut self, item: u64, count: u64) {
        for row in 0..self.depth {
            let col = self.hash(item, row);
            self.table[row][col] += count;
        }
    }

    fn estimate(&self, item: u64) -> u64 {
        (0..self.depth)
            .map(|row| self.table[row][self.hash(item, row)])
            .min()
            .unwrap_or(0)
    }
}

/// Create a new Count-Min Sketch.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cms_new(width: usize, depth: usize) -> *mut CountMinSketch {
    Box::into_raw(Box::new(CountMinSketch::new(width, depth)))
}

/// Add count for item.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cms_add(cms: *mut CountMinSketch, item: u64, count: u64) {
    if !cms.is_null() { unsafe { (*cms).add(item, count); } }
}

/// Estimate count for item.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cms_estimate(cms: *const CountMinSketch, item: u64) -> u64 {
    if cms.is_null() { return 0; }
    unsafe { (*cms).estimate(item) }
}

/// Free Count-Min Sketch.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cms_free(cms: *mut CountMinSketch) {
    if !cms.is_null() { unsafe { drop(Box::from_raw(cms)); } }
}

// ─── Simple DFA regex engine ────────────────────────────────────────

/// Simple regex test: supports `.`, `*`, `+`, `?`, `|`, `[a-z]`, `^`, `$`.
/// Returns 1 if `text` matches `pattern`, 0 otherwise.
/// This is a recursive backtracking implementation for basic patterns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_regex_match(
    pattern: *const u8, pat_len: usize,
    text: *const u8, text_len: usize,
) -> i32 {
    if pattern.is_null() || text.is_null() { return 0; }
    let pat = unsafe { std::slice::from_raw_parts(pattern, pat_len) };
    let txt = unsafe { std::slice::from_raw_parts(text, text_len) };
    if regex_match_impl(pat, txt) { 1 } else { 0 }
}

fn regex_match_impl(pat: &[u8], txt: &[u8]) -> bool {
    if pat.is_empty() { return txt.is_empty(); }

    // Handle alternation at top level
    if let Some(pipe_pos) = find_top_level_pipe(pat) {
        return regex_match_impl(&pat[..pipe_pos], txt) || regex_match_impl(&pat[pipe_pos + 1..], txt);
    }

    // Check for quantifier after current element
    let (elem_len, elem) = parse_element(pat);
    let rest = &pat[elem_len..];

    if rest.first() == Some(&b'*') {
        // Zero or more
        let rest2 = &rest[1..];
        return regex_match_impl(rest2, txt) ||
            (!txt.is_empty() && matches_element(&elem, txt[0]) && regex_match_impl(pat, &txt[1..]));
    }
    if rest.first() == Some(&b'+') {
        let rest2 = &rest[1..];
        return !txt.is_empty() && matches_element(&elem, txt[0]) &&
            (regex_match_impl(rest2, &txt[1..]) || regex_match_impl(pat, &txt[1..]));
    }
    if rest.first() == Some(&b'?') {
        let rest2 = &rest[1..];
        return regex_match_impl(rest2, txt) ||
            (!txt.is_empty() && matches_element(&elem, txt[0]) && regex_match_impl(rest2, &txt[1..]));
    }

    // No quantifier: must match exactly one
    if txt.is_empty() { return false; }
    matches_element(&elem, txt[0]) && regex_match_impl(rest, &txt[1..])
}

#[derive(Debug)]
enum RegexElem {
    Literal(u8),
    Dot,
    CharClass(Vec<u8>),
}

fn parse_element(pat: &[u8]) -> (usize, RegexElem) {
    match pat[0] {
        b'.' => (1, RegexElem::Dot),
        b'[' => {
            if let Some(end) = pat.iter().position(|&c| c == b']') {
                let chars = expand_char_class(&pat[1..end]);
                (end + 1, RegexElem::CharClass(chars))
            } else {
                (1, RegexElem::Literal(b'['))
            }
        }
        b'\\' if pat.len() > 1 => (2, RegexElem::Literal(pat[1])),
        ch => (1, RegexElem::Literal(ch)),
    }
}

fn expand_char_class(spec: &[u8]) -> Vec<u8> {
    let mut chars = Vec::new();
    let mut i = 0;
    while i < spec.len() {
        if i + 2 < spec.len() && spec[i + 1] == b'-' {
            for ch in spec[i]..=spec[i + 2] { chars.push(ch); }
            i += 3;
        } else {
            chars.push(spec[i]);
            i += 1;
        }
    }
    chars
}

fn matches_element(elem: &RegexElem, ch: u8) -> bool {
    match elem {
        RegexElem::Literal(c) => ch == *c,
        RegexElem::Dot => true,
        RegexElem::CharClass(chars) => chars.contains(&ch),
    }
}

fn find_top_level_pipe(pat: &[u8]) -> Option<usize> {
    let mut depth = 0;
    for (i, &ch) in pat.iter().enumerate() {
        match ch {
            b'[' => depth += 1,
            b']' => depth -= 1,
            b'|' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

// ─── Trie (prefix tree) ─────────────────────────────────────────────

struct TrieNode {
    children: HashMap<u8, usize>,
    is_end: bool,
}

struct Trie {
    nodes: Vec<TrieNode>,
}

impl Trie {
    fn new() -> Self {
        Trie { nodes: vec![TrieNode { children: HashMap::new(), is_end: false }] }
    }

    fn insert(&mut self, word: &[u8]) {
        let mut node = 0;
        for &ch in word {
            let next = self.nodes.len();
            let n = &mut self.nodes[node];
            if !n.children.contains_key(&ch) {
                self.nodes.push(TrieNode { children: HashMap::new(), is_end: false });
                self.nodes[node].children.insert(ch, next);
                node = next;
            } else {
                node = self.nodes[node].children[&ch];
            }
        }
        self.nodes[node].is_end = true;
    }

    fn contains(&self, word: &[u8]) -> bool {
        let mut node = 0;
        for &ch in word {
            if let Some(&next) = self.nodes[node].children.get(&ch) {
                node = next;
            } else {
                return false;
            }
        }
        self.nodes[node].is_end
    }

    fn starts_with(&self, prefix: &[u8]) -> bool {
        let mut node = 0;
        for &ch in prefix {
            if let Some(&next) = self.nodes[node].children.get(&ch) {
                node = next;
            } else {
                return false;
            }
        }
        true
    }

    fn count_prefix(&self, prefix: &[u8]) -> usize {
        let mut node = 0;
        for &ch in prefix {
            if let Some(&next) = self.nodes[node].children.get(&ch) {
                node = next;
            } else {
                return 0;
            }
        }
        self.count_from(node)
    }

    fn count_from(&self, node: usize) -> usize {
        let mut count = if self.nodes[node].is_end { 1 } else { 0 };
        for &child in self.nodes[node].children.values() {
            count += self.count_from(child);
        }
        count
    }
}

/// Create a new Trie.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_trie_new() -> *mut Trie {
    Box::into_raw(Box::new(Trie::new()))
}

/// Insert a word into trie.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_trie_insert(trie: *mut Trie, word: *const u8, len: usize) {
    if trie.is_null() || word.is_null() { return; }
    let w = unsafe { std::slice::from_raw_parts(word, len) };
    unsafe { (*trie).insert(w); }
}

/// Check if word exists in trie. Returns 1 if yes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_trie_contains(trie: *const Trie, word: *const u8, len: usize) -> i32 {
    if trie.is_null() || word.is_null() { return 0; }
    let w = unsafe { std::slice::from_raw_parts(word, len) };
    if unsafe { (*trie).contains(w) } { 1 } else { 0 }
}

/// Check if any word starts with prefix. Returns 1 if yes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_trie_starts_with(trie: *const Trie, prefix: *const u8, len: usize) -> i32 {
    if trie.is_null() || prefix.is_null() { return 0; }
    let p = unsafe { std::slice::from_raw_parts(prefix, len) };
    if unsafe { (*trie).starts_with(p) } { 1 } else { 0 }
}

/// Count words with given prefix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_trie_count_prefix(trie: *const Trie, prefix: *const u8, len: usize) -> i32 {
    if trie.is_null() || prefix.is_null() { return 0; }
    let p = unsafe { std::slice::from_raw_parts(prefix, len) };
    unsafe { (*trie).count_prefix(p) as i32 }
}

/// Free trie.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_trie_free(trie: *mut Trie) {
    if !trie.is_null() { unsafe { drop(Box::from_raw(trie)); } }
}

// ─── Finite State Machine ───────────────────────────────────────────

/// Simple FSM: given transition table, simulate from initial state on input.
/// `transitions[state * alphabet_size + symbol]` = next state (or -1 for reject).
/// `accept_states[i]` = 1 if state i is accepting.
/// Returns 1 if input is accepted, 0 if rejected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_fsm_simulate(
    transitions: *const i32, n_states: usize, alphabet_size: usize,
    accept_states: *const i32, initial_state: usize,
    input: *const usize, input_len: usize,
) -> i32 {
    if transitions.is_null() || accept_states.is_null() || input.is_null() { return 0; }
    let trans = unsafe { std::slice::from_raw_parts(transitions, n_states * alphabet_size) };
    let accept = unsafe { std::slice::from_raw_parts(accept_states, n_states) };
    let inp = unsafe { std::slice::from_raw_parts(input, input_len) };

    let mut state = initial_state;
    for &symbol in inp {
        if symbol >= alphabet_size { return 0; }
        let next = trans[state * alphabet_size + symbol];
        if next < 0 || next as usize >= n_states { return 0; }
        state = next as usize;
    }
    if accept[state] != 0 { 1 } else { 0 }
}

// ─── Levenshtein Automaton ──────────────────────────────────────────

/// Levenshtein automaton: returns 1 if edit distance between two byte strings <= max_dist.
/// More efficient than computing full edit distance when only checking threshold.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_levenshtein_within(
    a: *const u8, a_len: usize,
    b: *const u8, b_len: usize,
    max_dist: usize,
) -> i32 {
    if a.is_null() || b.is_null() { return 0; }
    let sa = unsafe { std::slice::from_raw_parts(a, a_len) };
    let sb = unsafe { std::slice::from_raw_parts(b, b_len) };

    // Quick length check
    if a_len.abs_diff(b_len) > max_dist { return 0; }

    // Single-row DP with early termination
    let mut prev = vec![0usize; b_len + 1];
    for j in 0..=b_len { prev[j] = j; }

    for i in 1..=a_len {
        let mut curr = vec![0usize; b_len + 1];
        curr[0] = i;
        let mut row_min = i;
        for j in 1..=b_len {
            let cost = if sa[i - 1] == sb[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            row_min = row_min.min(curr[j]);
        }
        if row_min > max_dist { return 0; }
        prev = curr;
    }
    if prev[b_len] <= max_dist { 1 } else { 0 }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aho_corasick_basic() {
        let text = b"ushershehis";
        let p1 = b"he";
        let p2 = b"she";
        let p3 = b"his";
        let p4 = b"hers";
        let patterns = [p1.as_ptr(), p2.as_ptr(), p3.as_ptr(), p4.as_ptr()];
        let lens = [2usize, 3, 3, 4];
        let mut positions = [0usize; 10];
        let mut ids = [0usize; 10];
        let count = unsafe {
            vitalis_aho_corasick(
                text.as_ptr(), text.len(),
                patterns.as_ptr(), lens.as_ptr(), 4,
                positions.as_mut_ptr(), ids.as_mut_ptr(), 10,
            )
        };
        assert!(count >= 4); // at least: "she", "he", "hers", "his"
    }

    #[test]
    fn test_bloom_filter() {
        let bf = unsafe { vitalis_bloom_new(1000, 7) };
        unsafe {
            vitalis_bloom_insert(bf, 42);
            vitalis_bloom_insert(bf, 100);
            vitalis_bloom_insert(bf, 200);
            assert_eq!(vitalis_bloom_contains(bf, 42), 1);
            assert_eq!(vitalis_bloom_contains(bf, 100), 1);
            assert_eq!(vitalis_bloom_contains(bf, 999999), 0); // probably not present
            vitalis_bloom_free(bf);
        }
    }

    #[test]
    fn test_count_min_sketch() {
        let cms = unsafe { vitalis_cms_new(100, 5) };
        unsafe {
            vitalis_cms_add(cms, 1, 3);
            vitalis_cms_add(cms, 2, 7);
            vitalis_cms_add(cms, 1, 2);
            assert!(vitalis_cms_estimate(cms, 1) >= 5);
            assert!(vitalis_cms_estimate(cms, 2) >= 7);
            assert_eq!(vitalis_cms_estimate(cms, 999), 0);
            vitalis_cms_free(cms);
        }
    }

    #[test]
    fn test_regex_match_literal() {
        let pat = b"hello";
        let txt = b"hello";
        assert_eq!(unsafe { vitalis_regex_match(pat.as_ptr(), 5, txt.as_ptr(), 5) }, 1);
        let txt2 = b"world";
        assert_eq!(unsafe { vitalis_regex_match(pat.as_ptr(), 5, txt2.as_ptr(), 5) }, 0);
    }

    #[test]
    fn test_regex_match_dot_star() {
        let pat = b"h.llo";
        let txt = b"hello";
        assert_eq!(unsafe { vitalis_regex_match(pat.as_ptr(), 5, txt.as_ptr(), 5) }, 1);

        let pat2 = b"he*lo";
        let txt2 = b"hlo";
        assert_eq!(unsafe { vitalis_regex_match(pat2.as_ptr(), 5, txt2.as_ptr(), 3) }, 1);
    }

    #[test]
    fn test_regex_alternation() {
        let pat = b"cat|dog";
        let txt1 = b"cat";
        let txt2 = b"dog";
        let txt3 = b"rat";
        assert_eq!(unsafe { vitalis_regex_match(pat.as_ptr(), 7, txt1.as_ptr(), 3) }, 1);
        assert_eq!(unsafe { vitalis_regex_match(pat.as_ptr(), 7, txt2.as_ptr(), 3) }, 1);
        assert_eq!(unsafe { vitalis_regex_match(pat.as_ptr(), 7, txt3.as_ptr(), 3) }, 0);
    }

    #[test]
    fn test_regex_char_class() {
        let pat = b"[a-z]+";
        let txt = b"hello";
        assert_eq!(unsafe { vitalis_regex_match(pat.as_ptr(), 6, txt.as_ptr(), 5) }, 1);
    }

    #[test]
    fn test_trie() {
        let trie = unsafe { vitalis_trie_new() };
        unsafe {
            vitalis_trie_insert(trie, b"hello".as_ptr(), 5);
            vitalis_trie_insert(trie, b"help".as_ptr(), 4);
            vitalis_trie_insert(trie, b"world".as_ptr(), 5);

            assert_eq!(vitalis_trie_contains(trie, b"hello".as_ptr(), 5), 1);
            assert_eq!(vitalis_trie_contains(trie, b"hell".as_ptr(), 4), 0);
            assert_eq!(vitalis_trie_starts_with(trie, b"hel".as_ptr(), 3), 1);
            assert_eq!(vitalis_trie_count_prefix(trie, b"hel".as_ptr(), 3), 2);
            assert_eq!(vitalis_trie_starts_with(trie, b"xyz".as_ptr(), 3), 0);

            vitalis_trie_free(trie);
        }
    }

    #[test]
    fn test_fsm_simulate() {
        // Simple FSM: accepts strings ending in 'b' (symbol 1)
        // State 0: initial, State 1: accepting
        // alphabet: {a=0, b=1}
        let trans = [0i32, 1, 0, 1]; // state0→{a:0,b:1}, state1→{a:0,b:1}
        let accept = [0i32, 1]; // state 1 is accepting
        let input_accept = [0usize, 1]; // "ab"
        let input_reject = [1usize, 0]; // "ba"
        assert_eq!(unsafe { vitalis_fsm_simulate(
            trans.as_ptr(), 2, 2, accept.as_ptr(), 0,
            input_accept.as_ptr(), 2
        ) }, 1);
        assert_eq!(unsafe { vitalis_fsm_simulate(
            trans.as_ptr(), 2, 2, accept.as_ptr(), 0,
            input_reject.as_ptr(), 2
        ) }, 0);
    }

    #[test]
    fn test_levenshtein_within() {
        assert_eq!(unsafe { vitalis_levenshtein_within(b"kitten".as_ptr(), 6, b"sitting".as_ptr(), 7, 3) }, 1);
        assert_eq!(unsafe { vitalis_levenshtein_within(b"kitten".as_ptr(), 6, b"sitting".as_ptr(), 7, 2) }, 0);
        assert_eq!(unsafe { vitalis_levenshtein_within(b"hello".as_ptr(), 5, b"hello".as_ptr(), 5, 0) }, 1);
    }
}
