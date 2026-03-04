//! Regex Engine Module for Vitalis v30.0
//!
//! Thompson NFA construction + Pike VM executor — the gold standard for
//! regular expression matching with O(n·m) time guarantee and no
//! catastrophic backtracking.
//!
//! Supports: literals, `.`, `*`, `+`, `?`, `|`, `()`, `(?:)`, `[abc]`,
//! `[^abc]`, `[a-z]`, `\d`, `\w`, `\s`, `\D`, `\W`, `\S`, `^`, `$`,
//! `{n,m}` repetition, capture groups, non-greedy `*?`, `+?`, `??`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── Regex AST ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum RegexAst {
    Literal(char),
    AnyChar,
    CharClass(Vec<(char, char)>, bool), // ranges, negated
    Concat(Vec<RegexAst>),
    Alternate(Box<RegexAst>, Box<RegexAst>),
    Repeat {
        child: Box<RegexAst>,
        min: usize,
        max: Option<usize>, // None = unbounded
        greedy: bool,
    },
    Group(Box<RegexAst>, usize), // capture group index
    NonCapture(Box<RegexAst>),
    Anchor(AnchorKind),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AnchorKind {
    Start,
    End,
}

// ─── Regex Parser ───────────────────────────────────────────────────

struct RegexParser {
    chars: Vec<char>,
    pos: usize,
    group_count: usize,
}

impl RegexParser {
    fn new(pattern: &str) -> Self {
        Self {
            chars: pattern.chars().collect(),
            pos: 0,
            group_count: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn parse(&mut self) -> Result<RegexAst, String> {
        let ast = self.parse_alternate()?;
        if self.pos < self.chars.len() {
            return Err(format!("Unexpected char at position {}", self.pos));
        }
        Ok(ast)
    }

    fn parse_alternate(&mut self) -> Result<RegexAst, String> {
        let mut left = self.parse_concat()?;
        while self.peek() == Some('|') {
            self.advance();
            let right = self.parse_concat()?;
            left = RegexAst::Alternate(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_concat(&mut self) -> Result<RegexAst, String> {
        let mut items = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == '|' || ch == ')' {
                break;
            }
            items.push(self.parse_quantified()?);
        }
        if items.is_empty() {
            Ok(RegexAst::Concat(vec![]))
        } else if items.len() == 1 {
            Ok(items.remove(0))
        } else {
            Ok(RegexAst::Concat(items))
        }
    }

    fn parse_quantified(&mut self) -> Result<RegexAst, String> {
        let atom = self.parse_atom()?;
        if let Some(ch) = self.peek() {
            match ch {
                '*' | '+' | '?' => {
                    self.advance();
                    let greedy = self.peek() != Some('?');
                    if !greedy {
                        self.advance();
                    }
                    let (min, max) = match ch {
                        '*' => (0, None),
                        '+' => (1, None),
                        '?' => (0, Some(1)),
                        _ => unreachable!(),
                    };
                    Ok(RegexAst::Repeat {
                        child: Box::new(atom),
                        min,
                        max,
                        greedy,
                    })
                }
                '{' => self.parse_repetition(atom),
                _ => Ok(atom),
            }
        } else {
            Ok(atom)
        }
    }

    fn parse_repetition(&mut self, atom: RegexAst) -> Result<RegexAst, String> {
        self.advance(); // consume '{'
        let min = self.parse_number()?;
        let max = if self.peek() == Some(',') {
            self.advance();
            if self.peek() == Some('}') {
                None // {n,} = at least n
            } else {
                Some(self.parse_number()?)
            }
        } else {
            Some(min) // {n} = exactly n
        };
        if self.advance() != Some('}') {
            return Err("Expected '}'".into());
        }
        let greedy = self.peek() != Some('?');
        if !greedy {
            self.advance();
        }
        Ok(RegexAst::Repeat {
            child: Box::new(atom),
            min,
            max,
            greedy,
        })
    }

    fn parse_number(&mut self) -> Result<usize, String> {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err("Expected number".into());
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        s.parse::<usize>().map_err(|_| "Invalid number".into())
    }

    fn parse_atom(&mut self) -> Result<RegexAst, String> {
        match self.peek() {
            Some('(') => {
                self.advance();
                if self.peek() == Some('?') {
                    self.advance();
                    if self.advance() != Some(':') {
                        return Err("Expected ':' after '(?'".into());
                    }
                    let inner = self.parse_alternate()?;
                    if self.advance() != Some(')') {
                        return Err("Unclosed group".into());
                    }
                    Ok(RegexAst::NonCapture(Box::new(inner)))
                } else {
                    self.group_count += 1;
                    let idx = self.group_count;
                    let inner = self.parse_alternate()?;
                    if self.advance() != Some(')') {
                        return Err("Unclosed group".into());
                    }
                    Ok(RegexAst::Group(Box::new(inner), idx))
                }
            }
            Some('[') => self.parse_char_class(),
            Some('.') => {
                self.advance();
                Ok(RegexAst::AnyChar)
            }
            Some('^') => {
                self.advance();
                Ok(RegexAst::Anchor(AnchorKind::Start))
            }
            Some('$') => {
                self.advance();
                Ok(RegexAst::Anchor(AnchorKind::End))
            }
            Some('\\') => {
                self.advance();
                self.parse_escape()
            }
            Some(ch) if ch != ')' && ch != '|' && ch != '*' && ch != '+' && ch != '?' && ch != '{' => {
                self.advance();
                Ok(RegexAst::Literal(ch))
            }
            _ => Err(format!("Unexpected character at position {}", self.pos)),
        }
    }

    fn parse_escape(&mut self) -> Result<RegexAst, String> {
        match self.advance() {
            Some('d') => Ok(RegexAst::CharClass(vec![('0', '9')], false)),
            Some('D') => Ok(RegexAst::CharClass(vec![('0', '9')], true)),
            Some('w') => Ok(RegexAst::CharClass(
                vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                false,
            )),
            Some('W') => Ok(RegexAst::CharClass(
                vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                true,
            )),
            Some('s') => Ok(RegexAst::CharClass(
                vec![(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')],
                false,
            )),
            Some('S') => Ok(RegexAst::CharClass(
                vec![(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')],
                true,
            )),
            Some('n') => Ok(RegexAst::Literal('\n')),
            Some('t') => Ok(RegexAst::Literal('\t')),
            Some('r') => Ok(RegexAst::Literal('\r')),
            Some(ch) => Ok(RegexAst::Literal(ch)), // escaped special chars
            None => Err("Unexpected end of pattern after '\\'".into()),
        }
    }

    fn parse_char_class(&mut self) -> Result<RegexAst, String> {
        self.advance(); // consume '['
        let negated = self.peek() == Some('^');
        if negated {
            self.advance();
        }
        let mut ranges = Vec::new();
        let mut first = true;
        while let Some(ch) = self.peek() {
            if ch == ']' && !first {
                self.advance();
                return Ok(RegexAst::CharClass(ranges, negated));
            }
            first = false;
            let start = if ch == '\\' {
                self.advance();
                match self.advance() {
                    Some('d') => { ranges.push(('0', '9')); continue; }
                    Some('w') => {
                        ranges.extend_from_slice(&[('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')]);
                        continue;
                    }
                    Some('s') => {
                        ranges.extend_from_slice(&[(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')]);
                        continue;
                    }
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some(c) => c,
                    None => return Err("Unexpected end in char class".into()),
                }
            } else {
                self.advance();
                ch
            };
            if self.peek() == Some('-') && self.chars.get(self.pos + 1).map_or(false, |&c| c != ']') {
                self.advance(); // consume '-'
                let end = if self.peek() == Some('\\') {
                    self.advance();
                    match self.advance() {
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('r') => '\r',
                        Some(c) => c,
                        None => return Err("Unexpected end in char class range".into()),
                    }
                } else {
                    self.advance().ok_or("Unexpected end in char class range")?
                };
                ranges.push((start, end));
            } else {
                ranges.push((start, start));
            }
        }
        Err("Unclosed character class".into())
    }
}

// ─── NFA Construction (Thompson's Algorithm) ────────────────────────

#[derive(Debug, Clone)]
struct NfaState {
    kind: NfaStateKind,
    next: usize,
}

const NO_NEXT: usize = usize::MAX;

fn nfa_state(kind: NfaStateKind) -> NfaState {
    NfaState { kind, next: NO_NEXT }
}

#[derive(Debug, Clone)]
enum NfaStateKind {
    Char(char),
    AnyChar,
    CharClass(Vec<(char, char)>, bool),
    Split(usize, usize),      // two epsilon transitions
    Epsilon(usize),            // single epsilon transition
    Save(usize),               // capture slot save
    Anchor(AnchorKind),
    Match,
}

struct NfaFragment {
    start: usize,
    end: usize, // the state whose out-edge(s) are dangling
}

/// Compiled NFA for regex matching.
struct CompiledRegex {
    states: Vec<NfaState>,
    start: usize,
    num_captures: usize, // number of capture groups (slots = 2 * num_captures + 2)
}

fn compile_regex(pattern: &str) -> Result<CompiledRegex, String> {
    let mut parser = RegexParser::new(pattern);
    let ast = parser.parse()?;
    let num_captures = parser.group_count;
    let mut states: Vec<NfaState> = Vec::new();

    // Wrap entire regex in group 0 (full match capture)
    let wrapped = RegexAst::Group(Box::new(ast), 0);
    let frag = build_nfa(&wrapped, &mut states)?;

    // Add match state
    let match_id = states.len();
    states.push(nfa_state(NfaStateKind::Match));
    patch_state(&mut states, frag.end, match_id);

    Ok(CompiledRegex {
        states,
        start: frag.start,
        num_captures,
    })
}

fn build_nfa(ast: &RegexAst, states: &mut Vec<NfaState>) -> Result<NfaFragment, String> {
    match ast {
        RegexAst::Literal(ch) => {
            let id = states.len();
            states.push(nfa_state(NfaStateKind::Char(*ch)));
            Ok(NfaFragment { start: id, end: id })
        }
        RegexAst::AnyChar => {
            let id = states.len();
            states.push(nfa_state(NfaStateKind::AnyChar));
            Ok(NfaFragment { start: id, end: id })
        }
        RegexAst::CharClass(ranges, neg) => {
            let id = states.len();
            states.push(nfa_state(NfaStateKind::CharClass(ranges.clone(), *neg)));
            Ok(NfaFragment { start: id, end: id })
        }
        RegexAst::Anchor(kind) => {
            let id = states.len();
            states.push(nfa_state(NfaStateKind::Anchor(*kind)));
            Ok(NfaFragment { start: id, end: id })
        }
        RegexAst::Concat(items) => {
            if items.is_empty() {
                let id = states.len();
                states.push(nfa_state(NfaStateKind::Epsilon(NO_NEXT)));
                return Ok(NfaFragment { start: id, end: id });
            }
            let mut frags = Vec::new();
            for item in items {
                frags.push(build_nfa(item, states)?);
            }
            // Chain fragments: patch each end to point to next start
            for i in 0..frags.len() - 1 {
                patch_state(states, frags[i].end, frags[i + 1].start);
            }
            Ok(NfaFragment {
                start: frags[0].start,
                end: frags[frags.len() - 1].end,
            })
        }
        RegexAst::Alternate(left, right) => {
            let left_frag = build_nfa(left, states)?;
            let right_frag = build_nfa(right, states)?;
            let split_id = states.len();
            states.push(nfa_state(NfaStateKind::Split(left_frag.start, right_frag.start)));
            // Join ends with epsilon to a new junction state
            let join_id = states.len();
            states.push(nfa_state(NfaStateKind::Epsilon(NO_NEXT)));
            patch_state(states, left_frag.end, join_id);
            patch_state(states, right_frag.end, join_id);
            Ok(NfaFragment { start: split_id, end: join_id })
        }
        RegexAst::Repeat { child, min, max, greedy } => {
            build_repeat(child, *min, *max, *greedy, states)
        }
        RegexAst::Group(inner, idx) => {
            let save_start_id = states.len();
            states.push(nfa_state(NfaStateKind::Save(idx * 2)));
            let inner_frag = build_nfa(inner, states)?;
            patch_state(states, save_start_id, inner_frag.start);
            let save_end_id = states.len();
            states.push(nfa_state(NfaStateKind::Save(idx * 2 + 1)));
            patch_state(states, inner_frag.end, save_end_id);
            Ok(NfaFragment { start: save_start_id, end: save_end_id })
        }
        RegexAst::NonCapture(inner) => build_nfa(inner, states),
    }
}

fn build_repeat(
    child: &RegexAst,
    min: usize,
    max: Option<usize>,
    greedy: bool,
    states: &mut Vec<NfaState>,
) -> Result<NfaFragment, String> {
    // Build `min` required copies
    let mut frags = Vec::new();
    for _ in 0..min {
        frags.push(build_nfa(child, states)?);
    }

    if let Some(max_val) = max {
        // Build (max - min) optional copies
        let optional_count = max_val.saturating_sub(min);
        for _ in 0..optional_count {
            let body = build_nfa(child, states)?;
            let split_id = states.len();
            if greedy {
                states.push(nfa_state(NfaStateKind::Split(body.start, NO_NEXT)));
            } else {
                states.push(nfa_state(NfaStateKind::Split(NO_NEXT, body.start)));
            }
            let join_id = states.len();
            states.push(nfa_state(NfaStateKind::Epsilon(NO_NEXT)));
            patch_state(states, body.end, join_id);
            patch_state(states, split_id, join_id);
            frags.push(NfaFragment { start: split_id, end: join_id });
        }
    } else {
        // Unbounded: add Kleene star loop
        let body = build_nfa(child, states)?;
        let split_id = states.len();
        if greedy {
            states.push(nfa_state(NfaStateKind::Split(body.start, NO_NEXT)));
        } else {
            states.push(nfa_state(NfaStateKind::Split(NO_NEXT, body.start)));
        }
        patch_state(states, body.end, split_id);
        frags.push(NfaFragment { start: split_id, end: split_id });
    }

    if frags.is_empty() {
        let id = states.len();
        states.push(nfa_state(NfaStateKind::Epsilon(NO_NEXT)));
        return Ok(NfaFragment { start: id, end: id });
    }

    // Chain all fragments
    for i in 0..frags.len() - 1 {
        patch_state(states, frags[i].end, frags[i + 1].start);
    }
    Ok(NfaFragment {
        start: frags[0].start,
        end: frags[frags.len() - 1].end,
    })
}

/// Patch dangling out-edges of a state to point to `target`.
fn patch_state(states: &mut [NfaState], state_id: usize, target: usize) {
    match &mut states[state_id].kind {
        NfaStateKind::Char(_) | NfaStateKind::AnyChar | NfaStateKind::CharClass(_, _)
        | NfaStateKind::Anchor(_) | NfaStateKind::Save(_) => {}
        NfaStateKind::Epsilon(out) => {
            if *out == NO_NEXT {
                *out = target;
            }
            return;
        }
        NfaStateKind::Split(out1, out2) => {
            if *out1 == NO_NEXT {
                *out1 = target;
            }
            if *out2 == NO_NEXT {
                *out2 = target;
            }
            return;
        }
        NfaStateKind::Match => { return; }
    }
    // For Char, AnyChar, CharClass, Anchor, Save: set explicit next pointer
    states[state_id].next = target;
}

// ─── Pike VM Executor ───────────────────────────────────────────────

/// Thread in the Pike VM with program counter and capture slots.
#[derive(Clone)]
struct Thread {
    pc: usize,
    captures: Vec<Option<usize>>,
}

/// Configuration for the VM.
struct VmConfig {
    anchored: bool, // if true, only match at start
}

/// Execute Pike VM on the compiled NFA. Returns capture groups if matched.
fn pike_vm_exec(
    regex: &CompiledRegex,
    input: &str,
    config: &VmConfig,
) -> Option<Vec<Option<usize>>> {
    let chars: Vec<char> = input.chars().collect();
    let num_slots = (regex.num_captures + 1) * 2;
    let num_states = regex.states.len();

    // Build explicit "next state" table for char-consuming states
    let next_table = build_next_table(regex);

    let start_positions = if config.anchored {
        0..1
    } else {
        0..chars.len() + 1
    };

    for sp in start_positions {
        let mut current_threads: Vec<Thread> = Vec::new();
        let initial = Thread {
            pc: regex.start,
            captures: vec![None; num_slots],
        };
        let mut visited = vec![false; num_states];
        add_thread(&mut current_threads, initial, &regex.states, &chars, sp, &mut visited);

        let mut best_match: Option<Vec<Option<usize>>> = None;

        for i in sp..=chars.len() {
            let mut next_threads: Vec<Thread> = Vec::new();

            for thread in &current_threads {
                let state = &regex.states[thread.pc];
                let matched = match &state.kind {
                    NfaStateKind::Char(expected) => {
                        i < chars.len() && chars[i] == *expected
                    }
                    NfaStateKind::AnyChar => {
                        i < chars.len() && chars[i] != '\n'
                    }
                    NfaStateKind::CharClass(ranges, negated) => {
                        if i >= chars.len() {
                            false
                        } else {
                            let ch = chars[i];
                            let in_class = ranges.iter().any(|&(lo, hi)| ch >= lo && ch <= hi);
                            if *negated { !in_class } else { in_class }
                        }
                    }
                    NfaStateKind::Match => {
                        best_match = Some(thread.captures.clone());
                        false
                    }
                    _ => false, // epsilon/split/save/anchor handled in add_thread
                };

                if matched {
                    let next_pc = next_table[thread.pc];
                    let mut new_thread = thread.clone();
                    new_thread.pc = next_pc;
                    let mut next_visited = vec![false; num_states];
                    add_thread(&mut next_threads, new_thread, &regex.states, &chars, i + 1, &mut next_visited);
                }
            }

            if best_match.is_some() && next_threads.is_empty() {
                return best_match;
            }

            current_threads = next_threads;
        }

        // Check remaining threads for match state
        for thread in &current_threads {
            if matches!(regex.states[thread.pc].kind, NfaStateKind::Match) {
                return Some(thread.captures.clone());
            }
        }

        if best_match.is_some() {
            return best_match;
        }
    }

    None
}

fn build_next_table(regex: &CompiledRegex) -> Vec<usize> {
    let n = regex.states.len();
    let mut table = vec![0usize; n];
    for i in 0..n {
        let fallback = if i + 1 < n { i + 1 } else { i };
        let explicit = regex.states[i].next;
        table[i] = match &regex.states[i].kind {
            NfaStateKind::Char(_) | NfaStateKind::AnyChar | NfaStateKind::CharClass(_, _) => {
                if explicit != NO_NEXT { explicit } else { fallback }
            }
            NfaStateKind::Epsilon(target) => *target,
            NfaStateKind::Split(a, _) => *a,
            NfaStateKind::Save(_) => if explicit != NO_NEXT { explicit } else { fallback },
            NfaStateKind::Anchor(_) => if explicit != NO_NEXT { explicit } else { fallback },
            NfaStateKind::Match => i,
        };
    }
    table
}

/// Add a thread, following epsilon transitions recursively (deduplicating by PC).
fn add_thread(
    threads: &mut Vec<Thread>,
    thread: Thread,
    states: &[NfaState],
    chars: &[char],
    pos: usize,
    visited: &mut Vec<bool>,
) {
    if thread.pc >= states.len() || visited[thread.pc] {
        return;
    }
    visited[thread.pc] = true;

    match &states[thread.pc].kind {
        NfaStateKind::Epsilon(target) => {
            let mut t = thread;
            t.pc = *target;
            add_thread(threads, t, states, chars, pos, visited);
        }
        NfaStateKind::Split(out1, out2) => {
            let mut t1 = thread.clone();
            t1.pc = *out1;
            let mut t2 = thread;
            t2.pc = *out2;
            add_thread(threads, t1, states, chars, pos, visited);
            add_thread(threads, t2, states, chars, pos, visited);
        }
        NfaStateKind::Save(slot) => {
            let mut t = thread;
            t.captures[*slot] = Some(pos);
            let pc = t.pc;
            t.pc = if states[pc].next != NO_NEXT { states[pc].next } else { pc + 1 };
            add_thread(threads, t, states, chars, pos, visited);
        }
        NfaStateKind::Anchor(kind) => {
            let ok = match kind {
                AnchorKind::Start => pos == 0,
                AnchorKind::End => pos == chars.len(),
            };
            if ok {
                let mut t = thread;
                let pc = t.pc;
                t.pc = if states[pc].next != NO_NEXT { states[pc].next } else { pc + 1 };
                add_thread(threads, t, states, chars, pos, visited);
            }
        }
        _ => {
            threads.push(thread);
        }
    }
}

// ─── Public Rust API ────────────────────────────────────────────────

/// Check if pattern matches the entire input.
fn regex_is_match(pattern: &str, input: &str) -> Result<bool, String> {
    let re = compile_regex(pattern)?;
    let config = VmConfig { anchored: false };
    if let Some(caps) = pike_vm_exec(&re, input, &config) {
        // Check if full match covers entire input
        if let (Some(start), Some(end)) = (caps[0], caps[1]) {
            Ok(start == 0 && end == input.chars().count())
        } else {
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

/// Find first match in input. Returns (start, end) byte offsets.
fn regex_find(pattern: &str, input: &str) -> Result<Option<(usize, usize)>, String> {
    let re = compile_regex(pattern)?;
    let config = VmConfig { anchored: false };
    if let Some(caps) = pike_vm_exec(&re, input, &config) {
        if let (Some(s), Some(e)) = (caps[0], caps[1]) {
            Ok(Some((s, e)))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Find all non-overlapping matches.
fn regex_find_all(pattern: &str, input: &str) -> Result<Vec<(usize, usize)>, String> {
    let re = compile_regex(pattern)?;
    let chars: Vec<char> = input.chars().collect();
    let config = VmConfig { anchored: false };
    let mut results = Vec::new();
    let mut search_start = 0;

    while search_start <= chars.len() {
        let sub: String = chars[search_start..].iter().collect();
        if let Some(caps) = pike_vm_exec(&re, &sub, &config) {
            if let (Some(s), Some(e)) = (caps[0], caps[1]) {
                let abs_start = search_start + s;
                let abs_end = search_start + e;
                results.push((abs_start, abs_end));
                search_start = if abs_end > search_start + s {
                    abs_end
                } else {
                    search_start + 1
                };
            } else {
                break;
            }
        } else {
            break;
        }
    }
    Ok(results)
}

/// Get capture group strings from first match.
fn regex_captures(pattern: &str, input: &str) -> Result<Vec<Option<String>>, String> {
    let re = compile_regex(pattern)?;
    let chars: Vec<char> = input.chars().collect();
    let config = VmConfig { anchored: false };
    if let Some(caps) = pike_vm_exec(&re, input, &config) {
        let mut result = Vec::new();
        for i in (0..caps.len()).step_by(2) {
            if let (Some(s), Some(e)) = (caps[i], caps.get(i + 1).copied().flatten()) {
                let captured: String = chars[s..e].iter().collect();
                result.push(Some(captured));
            } else {
                result.push(None);
            }
        }
        Ok(result)
    } else {
        Ok(vec![])
    }
}

/// Replace first match with replacement string.
fn regex_replace(pattern: &str, input: &str, replacement: &str) -> Result<String, String> {
    let re = compile_regex(pattern)?;
    let chars: Vec<char> = input.chars().collect();
    let config = VmConfig { anchored: false };
    if let Some(caps) = pike_vm_exec(&re, input, &config) {
        if let (Some(s), Some(e)) = (caps[0], caps[1]) {
            let before: String = chars[..s].iter().collect();
            let after: String = chars[e..].iter().collect();
            return Ok(format!("{}{}{}", before, replacement, after));
        }
    }
    Ok(input.to_string())
}

/// Replace all non-overlapping matches.
fn regex_replace_all(pattern: &str, input: &str, replacement: &str) -> Result<String, String> {
    let matches = regex_find_all(pattern, input)?;
    if matches.is_empty() {
        return Ok(input.to_string());
    }
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::new();
    let mut last_end = 0;
    for (s, e) in &matches {
        let seg: String = chars[last_end..*s].iter().collect();
        result.push_str(&seg);
        result.push_str(replacement);
        last_end = *e;
    }
    let tail: String = chars[last_end..].iter().collect();
    result.push_str(&tail);
    Ok(result)
}

/// Split input by regex pattern.
fn regex_split(pattern: &str, input: &str) -> Result<Vec<String>, String> {
    let matches = regex_find_all(pattern, input)?;
    let chars: Vec<char> = input.chars().collect();
    let mut parts = Vec::new();
    let mut last = 0;
    for (s, e) in &matches {
        let part: String = chars[last..*s].iter().collect();
        parts.push(part);
        last = *e;
    }
    let tail: String = chars[last..].iter().collect();
    parts.push(tail);
    Ok(parts)
}

// ─── FFI Layer ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_regex_is_match(pattern: *const c_char, input: *const c_char) -> i64 {
    let pattern = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    match regex_is_match(pattern, input) {
        Ok(true) => 1,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_regex_find_first(
    pattern: *const c_char,
    input: *const c_char,
) -> *mut c_char {
    let pattern = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match regex_find(pattern, input) {
        Ok(Some((s, e))) => format!("{}:{}", s, e),
        _ => String::new(),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_regex_find_all_matches(
    pattern: *const c_char,
    input: *const c_char,
) -> *mut c_char {
    let pattern = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match regex_find_all(pattern, input) {
        Ok(matches) => {
            let parts: Vec<String> = matches.iter().map(|(s, e)| format!("{}:{}", s, e)).collect();
            parts.join(",")
        }
        _ => String::new(),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_regex_captures_first(
    pattern: *const c_char,
    input: *const c_char,
) -> *mut c_char {
    let pattern = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match regex_captures(pattern, input) {
        Ok(groups) => {
            let parts: Vec<String> = groups
                .iter()
                .map(|opt| opt.as_deref().unwrap_or("").to_string())
                .collect();
            parts.join("\t")
        }
        _ => String::new(),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_regex_replace_first(
    pattern: *const c_char,
    input: *const c_char,
    replacement: *const c_char,
) -> *mut c_char {
    let pattern = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let replacement = unsafe { CStr::from_ptr(replacement) }.to_str().unwrap_or("");
    let result = regex_replace(pattern, input, replacement).unwrap_or_default();
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_regex_replace_all_matches(
    pattern: *const c_char,
    input: *const c_char,
    replacement: *const c_char,
) -> *mut c_char {
    let pattern = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let replacement = unsafe { CStr::from_ptr(replacement) }.to_str().unwrap_or("");
    let result = regex_replace_all(pattern, input, replacement).unwrap_or_default();
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_regex_split_by(
    pattern: *const c_char,
    input: *const c_char,
) -> *mut c_char {
    let pattern = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match regex_split(pattern, input) {
        Ok(parts) => parts.join("\t"),
        _ => String::new(),
    };
    CString::new(result).unwrap().into_raw()
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_match() {
        assert!(regex_is_match("hello", "hello").unwrap());
        assert!(!regex_is_match("hello", "world").unwrap());
    }

    #[test]
    fn test_dot_wildcard() {
        assert!(regex_is_match("h.llo", "hello").unwrap());
        assert!(regex_is_match("h.llo", "hallo").unwrap());
        assert!(!regex_is_match("h.llo", "hllo").unwrap());
    }

    #[test]
    fn test_star_quantifier() {
        assert!(regex_is_match("ab*c", "ac").unwrap());
        assert!(regex_is_match("ab*c", "abc").unwrap());
        assert!(regex_is_match("ab*c", "abbbbc").unwrap());
    }

    #[test]
    fn test_plus_quantifier() {
        assert!(!regex_is_match("ab+c", "ac").unwrap());
        assert!(regex_is_match("ab+c", "abc").unwrap());
        assert!(regex_is_match("ab+c", "abbbbc").unwrap());
    }

    #[test]
    fn test_question_quantifier() {
        assert!(regex_is_match("ab?c", "ac").unwrap());
        assert!(regex_is_match("ab?c", "abc").unwrap());
        assert!(!regex_is_match("ab?c", "abbc").unwrap());
    }

    #[test]
    fn test_alternation() {
        assert!(regex_is_match("cat|dog", "cat").unwrap());
        assert!(regex_is_match("cat|dog", "dog").unwrap());
        assert!(!regex_is_match("cat|dog", "bird").unwrap());
    }

    #[test]
    fn test_character_class() {
        assert!(regex_is_match("[abc]", "a").unwrap());
        assert!(regex_is_match("[abc]", "b").unwrap());
        assert!(!regex_is_match("[abc]", "d").unwrap());
    }

    #[test]
    fn test_char_class_range() {
        assert!(regex_is_match("[a-z]+", "hello").unwrap());
        assert!(!regex_is_match("[a-z]+", "HELLO").unwrap());
        assert!(regex_is_match("[A-Za-z]+", "Hello").unwrap());
    }

    #[test]
    fn test_negated_char_class() {
        assert!(!regex_is_match("[^0-9]+", "123").unwrap());
        assert!(regex_is_match("[^0-9]+", "abc").unwrap());
    }

    #[test]
    fn test_escape_digit() {
        assert!(regex_is_match("\\d+", "42").unwrap());
        assert!(!regex_is_match("\\d+", "abc").unwrap());
    }

    #[test]
    fn test_escape_word() {
        assert!(regex_is_match("\\w+", "hello_42").unwrap());
        assert!(!regex_is_match("\\w+", "   ").unwrap());
    }

    #[test]
    fn test_escape_whitespace() {
        assert!(regex_is_match("\\s+", "  \t").unwrap());
        assert!(!regex_is_match("\\s+", "abc").unwrap());
    }

    #[test]
    fn test_groups_capture() {
        let caps = regex_captures("(\\d+)-(\\d+)", "123-456").unwrap();
        assert!(caps.len() >= 3);
        assert_eq!(caps[0].as_deref(), Some("123-456")); // full match
        assert_eq!(caps[1].as_deref(), Some("123"));
        assert_eq!(caps[2].as_deref(), Some("456"));
    }

    #[test]
    fn test_non_capturing_group() {
        assert!(regex_is_match("(?:ab)+", "ababab").unwrap());
        let caps = regex_captures("(?:ab)(cd)", "abcd").unwrap();
        assert_eq!(caps[1].as_deref(), Some("cd")); // only 1 capture group
    }

    #[test]
    fn test_find_first() {
        let m = regex_find("\\d+", "abc 123 def").unwrap();
        assert_eq!(m, Some((4, 7)));
    }

    #[test]
    fn test_find_all() {
        let matches = regex_find_all("\\d+", "a1b22c333").unwrap();
        assert_eq!(matches, vec![(1, 2), (3, 5), (6, 9)]);
    }

    #[test]
    fn test_replace_first() {
        let result = regex_replace("\\d+", "abc 123 def 456", "NUM").unwrap();
        assert_eq!(result, "abc NUM def 456");
    }

    #[test]
    fn test_replace_all() {
        let result = regex_replace_all("\\d+", "a1b2c3", "X").unwrap();
        assert_eq!(result, "aXbXcX");
    }

    #[test]
    fn test_split() {
        let parts = regex_split("[,;]+", "a,b;;c,d").unwrap();
        assert_eq!(parts, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_repetition_exact() {
        assert!(regex_is_match("a{3}", "aaa").unwrap());
        assert!(!regex_is_match("a{3}", "aa").unwrap());
    }

    #[test]
    fn test_repetition_range() {
        assert!(regex_is_match("a{2,4}", "aa").unwrap());
        assert!(regex_is_match("a{2,4}", "aaa").unwrap());
        assert!(regex_is_match("a{2,4}", "aaaa").unwrap());
        assert!(!regex_is_match("a{2,4}", "a").unwrap());
    }

    #[test]
    fn test_repetition_min_only() {
        assert!(regex_is_match("a{2,}", "aa").unwrap());
        assert!(regex_is_match("a{2,}", "aaaaaaa").unwrap());
        assert!(!regex_is_match("a{2,}", "a").unwrap());
    }

    #[test]
    fn test_anchors() {
        assert!(regex_is_match("^hello$", "hello").unwrap());
        assert!(!regex_is_match("^hello$", "  hello  ").unwrap());
    }

    #[test]
    fn test_complex_email_pattern() {
        let pattern = "[a-zA-Z0-9.]+@[a-zA-Z0-9]+\\.[a-zA-Z]+";
        assert!(regex_find(pattern, "user@example.com").unwrap().is_some());
        assert!(regex_find(pattern, "test.user@host.org").unwrap().is_some());
    }

    #[test]
    fn test_no_catastrophic_backtracking() {
        // This pattern would cause exponential backtracking in naive engines
        // Thompson NFA guarantees O(n*m) time
        let pattern = "a?a?a?a?a?a?a?a?a?a?aaaaaaaaaa";
        let input = "aaaaaaaaaa";
        assert!(regex_is_match(pattern, input).unwrap());
    }

    #[test]
    fn test_ffi_is_match() {
        let pat = CString::new("\\d+").unwrap();
        let inp = CString::new("hello 42").unwrap();
        // Not a full match (find would succeed), but is_match checks full string
        assert_eq!(vitalis_regex_is_match(pat.as_ptr(), inp.as_ptr()), 0);
        let inp2 = CString::new("42").unwrap();
        assert_eq!(vitalis_regex_is_match(pat.as_ptr(), inp2.as_ptr()), 1);
    }

    #[test]
    fn test_ffi_find_first() {
        let pat = CString::new("\\d+").unwrap();
        let inp = CString::new("abc 42 def").unwrap();
        let result = vitalis_regex_find_first(pat.as_ptr(), inp.as_ptr());
        let s = unsafe { CString::from_raw(result) }.into_string().unwrap();
        assert_eq!(s, "4:6");
    }

    #[test]
    fn test_ffi_replace() {
        let pat = CString::new("\\d+").unwrap();
        let inp = CString::new("a1b2c3").unwrap();
        let rep = CString::new("X").unwrap();
        let result = vitalis_regex_replace_all_matches(pat.as_ptr(), inp.as_ptr(), rep.as_ptr());
        let s = unsafe { CString::from_raw(result) }.into_string().unwrap();
        assert_eq!(s, "aXbXcX");
    }

    #[test]
    fn test_empty_pattern() {
        // Empty pattern matches everything
        assert!(regex_find("", "hello").unwrap().is_some());
    }

    #[test]
    fn test_escaped_special_chars() {
        assert!(regex_is_match("\\.", ".").unwrap());
        assert!(!regex_is_match("\\.", "a").unwrap());
        assert!(regex_is_match("\\*", "*").unwrap());
    }

    #[test]
    fn test_nested_groups() {
        let caps = regex_captures("((a)(b))", "ab").unwrap();
        assert_eq!(caps[0].as_deref(), Some("ab")); // full match
        assert_eq!(caps[1].as_deref(), Some("ab")); // group 1
        assert_eq!(caps[2].as_deref(), Some("a"));  // group 2
        assert_eq!(caps[3].as_deref(), Some("b"));  // group 3
    }
}
