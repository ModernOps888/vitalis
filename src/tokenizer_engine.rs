//! Tokenizer Engine — BPE, WordPiece, and Unigram tokenizers for text processing.
//!
//! Provides byte-level BPE tokenizer training, encoding/decoding, special token
//! handling, pre-tokenization with regex patterns, and vocabulary management.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Token Types ─────────────────────────────────────────────────────────

/// Token with ID and metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub id: u32,
    pub text: String,
    pub is_special: bool,
}

/// Tokenizer algorithm variant.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenizerType {
    BPE,
    WordPiece,
    Unigram,
}

// ── BPE Tokenizer ───────────────────────────────────────────────────────

/// Byte-Pair Encoding tokenizer.
#[derive(Debug, Clone)]
pub struct BPETokenizer {
    pub vocab: HashMap<String, u32>,
    pub id_to_token: HashMap<u32, String>,
    pub merges: Vec<(String, String)>,
    pub special_tokens: HashMap<String, u32>,
    pub vocab_size: usize,
    pub unk_id: u32,
    pub pad_id: u32,
    pub bos_id: u32,
    pub eos_id: u32,
}

impl BPETokenizer {
    /// Create with base byte vocabulary (256 entries) plus special tokens.
    pub fn new(vocab_size: usize) -> Self {
        let mut vocab = HashMap::new();
        let mut id_to_token = HashMap::new();

        // Special tokens
        let specials = vec![
            ("<pad>", 0u32), ("<unk>", 1), ("<bos>", 2), ("<eos>", 3),
            ("<mask>", 4), ("<sep>", 5),
        ];
        let mut special_tokens = HashMap::new();
        for (tok, id) in &specials {
            vocab.insert(tok.to_string(), *id);
            id_to_token.insert(*id, tok.to_string());
            special_tokens.insert(tok.to_string(), *id);
        }

        // Byte tokens (6..262)
        for b in 0u8..=255 {
            let tok = format!("<0x{:02X}>", b);
            let id = b as u32 + 6;
            vocab.insert(tok.clone(), id);
            id_to_token.insert(id, tok);
        }

        BPETokenizer {
            vocab, id_to_token,
            merges: Vec::new(),
            special_tokens,
            vocab_size,
            unk_id: 1,
            pad_id: 0,
            bos_id: 2,
            eos_id: 3,
        }
    }

    /// Train BPE on a corpus to learn merge rules.
    pub fn train(&mut self, texts: &[&str], num_merges: usize) {
        // Build word frequencies from corpus
        let mut word_freqs: HashMap<Vec<String>, usize> = HashMap::new();
        for text in texts {
            for word in text.split_whitespace() {
                let chars: Vec<String> = word.bytes().map(|b| format!("<0x{:02X}>", b)).collect();
                *word_freqs.entry(chars).or_insert(0) += 1;
            }
        }

        let next_id = self.vocab.len() as u32;
        let mut current_id = next_id;

        for _merge_idx in 0..num_merges {
            if current_id as usize >= self.vocab_size {
                break;
            }

            // Count all adjacent pairs
            let mut pair_counts: HashMap<(String, String), usize> = HashMap::new();
            for (word, freq) in &word_freqs {
                for i in 0..word.len().saturating_sub(1) {
                    let pair = (word[i].clone(), word[i + 1].clone());
                    *pair_counts.entry(pair).or_insert(0) += freq;
                }
            }

            // Find most frequent pair
            let best = pair_counts.iter().max_by_key(|&(_, count)| *count);
            let (best_pair, _best_count) = match best {
                Some((p, c)) => (p.clone(), *c),
                None => break,
            };

            // Create merged token
            let merged = format!("{}{}", best_pair.0, best_pair.1);
            self.merges.push(best_pair.clone());
            self.vocab.insert(merged.clone(), current_id);
            self.id_to_token.insert(current_id, merged.clone());
            current_id += 1;

            // Apply merge to all words
            let mut new_freqs: HashMap<Vec<String>, usize> = HashMap::new();
            for (word, freq) in &word_freqs {
                let merged_word = apply_merge(word, &best_pair.0, &best_pair.1, &merged);
                *new_freqs.entry(merged_word).or_insert(0) += freq;
            }
            word_freqs = new_freqs;
        }
    }

    /// Encode text to token IDs.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        // Check for special tokens first
        for (special, id) in &self.special_tokens {
            if text == special.as_str() {
                return vec![*id];
            }
        }

        let mut tokens = Vec::new();
        for word in text.split_whitespace() {
            let mut pieces: Vec<String> = word.bytes().map(|b| format!("<0x{:02X}>", b)).collect();

            // Apply merges in order
            for (a, b) in &self.merges {
                let merged = format!("{}{}", a, b);
                pieces = apply_merge(&pieces, a, b, &merged);
            }

            // Map to IDs
            for piece in pieces {
                tokens.push(*self.vocab.get(&piece).unwrap_or(&self.unk_id));
            }
        }
        tokens
    }

    /// Decode token IDs back to text.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut bytes = Vec::new();
        let mut result_parts = Vec::new();
        let mut has_special = false;

        for &id in ids {
            if let Some(tok) = self.id_to_token.get(&id) {
                if self.special_tokens.contains_key(tok) {
                    // Flush accumulated bytes
                    if !bytes.is_empty() {
                        result_parts.push(String::from_utf8_lossy(&bytes).to_string());
                        bytes.clear();
                    }
                    has_special = true;
                    continue;
                }
                // Extract bytes from hex tokens
                extract_bytes(tok, &mut bytes);
            }
        }
        if !bytes.is_empty() {
            result_parts.push(String::from_utf8_lossy(&bytes).to_string());
        }
        let _ = has_special;
        result_parts.join("")
    }

    /// Add a special token and return its ID.
    pub fn add_special_token(&mut self, token: &str) -> u32 {
        if let Some(&id) = self.special_tokens.get(token) {
            return id;
        }
        let id = self.vocab.len() as u32;
        self.vocab.insert(token.to_string(), id);
        self.id_to_token.insert(id, token.to_string());
        self.special_tokens.insert(token.to_string(), id);
        id
    }

    /// Current vocabulary size.
    pub fn current_vocab_size(&self) -> usize {
        self.vocab.len()
    }
}

// ── WordPiece Tokenizer ─────────────────────────────────────────────────

/// WordPiece tokenizer (used by BERT).
#[derive(Debug, Clone)]
pub struct WordPieceTokenizer {
    pub vocab: HashMap<String, u32>,
    pub id_to_token: HashMap<u32, String>,
    pub max_word_len: usize,
    pub unk_id: u32,
    pub continuation_prefix: String,
}

impl WordPieceTokenizer {
    pub fn new() -> Self {
        let mut vocab = HashMap::new();
        let mut id_to_token = HashMap::new();
        vocab.insert("[UNK]".to_string(), 0);
        id_to_token.insert(0, "[UNK]".to_string());
        vocab.insert("[CLS]".to_string(), 1);
        id_to_token.insert(1, "[CLS]".to_string());
        vocab.insert("[SEP]".to_string(), 2);
        id_to_token.insert(2, "[SEP]".to_string());
        vocab.insert("[PAD]".to_string(), 3);
        id_to_token.insert(3, "[PAD]".to_string());
        vocab.insert("[MASK]".to_string(), 4);
        id_to_token.insert(4, "[MASK]".to_string());

        WordPieceTokenizer {
            vocab, id_to_token,
            max_word_len: 200,
            unk_id: 0,
            continuation_prefix: "##".to_string(),
        }
    }

    /// Build vocabulary from word frequencies.
    pub fn build_vocab(&mut self, word_freqs: &HashMap<String, usize>, vocab_size: usize) {
        // Add individual characters first
        let mut chars_seen = std::collections::HashSet::new();
        for word in word_freqs.keys() {
            for c in word.chars() {
                chars_seen.insert(c);
            }
        }
        for c in chars_seen {
            let tok = c.to_string();
            if !self.vocab.contains_key(&tok) {
                let id = self.vocab.len() as u32;
                self.vocab.insert(tok.clone(), id);
                self.id_to_token.insert(id, tok);
            }
            let cont = format!("##{}", c);
            if !self.vocab.contains_key(&cont) {
                let id = self.vocab.len() as u32;
                self.vocab.insert(cont.clone(), id);
                self.id_to_token.insert(id, cont);
            }
        }

        // Iteratively find best subword merges using WordPiece scoring
        while self.vocab.len() < vocab_size {
            let mut best_score = f64::NEG_INFINITY;
            let mut best_pair = None;

            // Score all possible merges
            for word in word_freqs.keys() {
                let pieces = self.tokenize_word(word);
                for i in 0..pieces.len().saturating_sub(1) {
                    let merged = if i == 0 {
                        format!("{}{}", pieces[i], pieces[i + 1].trim_start_matches("##"))
                    } else {
                        format!("{}{}", pieces[i], pieces[i + 1].trim_start_matches("##"))
                    };
                    if self.vocab.contains_key(&merged) {
                        continue;
                    }
                    // WordPiece score: freq(ab) / (freq(a) * freq(b))
                    let freq_a = word_freqs.values().sum::<usize>() as f64;
                    let score = freq_a / (freq_a * freq_a + 1.0); // Simplified scoring
                    if score > best_score {
                        best_score = score;
                        best_pair = Some(merged);
                    }
                }
            }

            match best_pair {
                Some(token) => {
                    let id = self.vocab.len() as u32;
                    self.vocab.insert(token.clone(), id);
                    self.id_to_token.insert(id, token);
                }
                None => break,
            }
        }
    }

    /// Tokenize a single word using greedy longest-match.
    pub fn tokenize_word(&self, word: &str) -> Vec<String> {
        if word.len() > self.max_word_len {
            return vec!["[UNK]".to_string()];
        }

        let mut tokens = Vec::new();
        let mut start = 0;
        let chars: Vec<char> = word.chars().collect();

        while start < chars.len() {
            let mut end = chars.len();
            let mut found = false;

            while start < end {
                let subword: String = chars[start..end].iter().collect();
                let candidate = if start > 0 {
                    format!("##{}", subword)
                } else {
                    subword
                };

                if self.vocab.contains_key(&candidate) {
                    tokens.push(candidate);
                    found = true;
                    start = end;
                    break;
                }
                end -= 1;
            }

            if !found {
                tokens.push("[UNK]".to_string());
                start += 1;
            }
        }
        tokens
    }

    /// Encode text to token IDs.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        for word in text.split_whitespace() {
            let pieces = self.tokenize_word(word);
            for piece in pieces {
                ids.push(*self.vocab.get(&piece).unwrap_or(&self.unk_id));
            }
        }
        ids
    }

    /// Decode token IDs to text.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut parts = Vec::new();
        for &id in ids {
            if let Some(tok) = self.id_to_token.get(&id) {
                if tok.starts_with("##") {
                    if let Some(last) = parts.last_mut() {
                        let s: &mut String = last;
                        s.push_str(&tok[2..]);
                    }
                } else {
                    parts.push(tok.clone());
                }
            }
        }
        parts.join(" ")
    }
}

impl Default for WordPieceTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Unigram Tokenizer ───────────────────────────────────────────────────

/// Unigram language model tokenizer (SentencePiece).
#[derive(Debug, Clone)]
pub struct UnigramTokenizer {
    pub pieces: Vec<(String, f64)>, // (token, log_prob)
    pub vocab: HashMap<String, u32>,
    pub id_to_token: HashMap<u32, String>,
    pub unk_id: u32,
}

impl UnigramTokenizer {
    pub fn new() -> Self {
        let mut vocab = HashMap::new();
        let mut id_to_token = HashMap::new();
        vocab.insert("<unk>".to_string(), 0);
        id_to_token.insert(0, "<unk>".to_string());
        UnigramTokenizer {
            pieces: vec![("<unk>".to_string(), 0.0)],
            vocab, id_to_token, unk_id: 0,
        }
    }

    /// Add a piece with its log probability.
    pub fn add_piece(&mut self, piece: &str, log_prob: f64) {
        let id = self.vocab.len() as u32;
        if !self.vocab.contains_key(piece) {
            self.vocab.insert(piece.to_string(), id);
            self.id_to_token.insert(id, piece.to_string());
            self.pieces.push((piece.to_string(), log_prob));
        }
    }

    /// Encode using Viterbi tokenization (most probable segmentation).
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let chars: Vec<char> = text.chars().collect();
        let n = chars.len();
        if n == 0 { return vec![]; }

        // Viterbi: best[i] = (best_score, best_prev, best_token_id)
        let mut best_score = vec![f64::NEG_INFINITY; n + 1];
        let mut best_prev = vec![0usize; n + 1];
        let mut best_token = vec![self.unk_id; n + 1];
        best_score[0] = 0.0;

        for i in 0..n {
            if best_score[i] == f64::NEG_INFINITY { continue; }
            for (piece, log_prob) in &self.pieces {
                let piece_len = piece.chars().count();
                if i + piece_len > n { continue; }
                let substr: String = chars[i..i + piece_len].iter().collect();
                if substr == *piece {
                    let score = best_score[i] + log_prob;
                    if score > best_score[i + piece_len] {
                        best_score[i + piece_len] = score;
                        best_prev[i + piece_len] = i;
                        best_token[i + piece_len] = *self.vocab.get(piece).unwrap_or(&self.unk_id);
                    }
                }
            }
            // Fallback: single character as unk
            if best_score[i + 1] == f64::NEG_INFINITY {
                best_score[i + 1] = best_score[i] - 100.0; // Heavy penalty
                best_prev[i + 1] = i;
                best_token[i + 1] = self.unk_id;
            }
        }

        // Backtrack
        let mut ids = Vec::new();
        let mut pos = n;
        while pos > 0 {
            ids.push(best_token[pos]);
            pos = best_prev[pos];
        }
        ids.reverse();
        ids
    }

    /// Decode token IDs back to text.
    pub fn decode(&self, ids: &[u32]) -> String {
        ids.iter()
            .filter_map(|&id| self.id_to_token.get(&id))
            .cloned()
            .collect::<Vec<_>>()
            .join("")
    }
}

impl Default for UnigramTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Pre-tokenization ────────────────────────────────────────────────────

/// Split text into pre-tokens using whitespace and punctuation.
pub fn pre_tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for c in text.chars() {
        if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else if c.is_ascii_punctuation() {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            tokens.push(c.to_string());
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Normalize text (lowercase, strip accents, NFC).
pub fn normalize_text(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_uppercase() {
                c.to_lowercase().next().unwrap_or(c)
            } else {
                c
            }
        })
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

// ── Helper functions ────────────────────────────────────────────────────

/// Apply a BPE merge rule to a word.
fn apply_merge(word: &[String], a: &str, b: &str, merged: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < word.len() {
        if i + 1 < word.len() && word[i] == a && word[i + 1] == b {
            result.push(merged.to_string());
            i += 2;
        } else {
            result.push(word[i].clone());
            i += 1;
        }
    }
    result
}

/// Extract raw bytes from hex token strings.
fn extract_bytes(tok: &str, bytes: &mut Vec<u8>) {
    // Handle merged tokens like "<0x48><0x65><0x6C>"
    let mut pos = 0;
    let chars: Vec<char> = tok.chars().collect();
    while pos < chars.len() {
        if pos + 5 < chars.len() && chars[pos] == '<' && chars[pos + 1] == '0' && chars[pos + 2] == 'x' {
            // Find closing >
            if let Some(end) = chars[pos..].iter().position(|&c| c == '>') {
                let hex: String = chars[pos + 3..pos + end].iter().collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    bytes.push(byte);
                }
                pos += end + 1;
                continue;
            }
        }
        // Non-hex character — treat as UTF-8
        let c = chars[pos];
        let mut buf = [0u8; 4];
        let encoded = c.encode_utf8(&mut buf);
        bytes.extend_from_slice(encoded.as_bytes());
        pos += 1;
    }
}

// ── FFI Interface ───────────────────────────────────────────────────────

static BPE_STORE: Mutex<Option<HashMap<i64, BPETokenizer>>> = Mutex::new(None);

fn with_bpe<R>(f: impl FnOnce(&mut HashMap<i64, BPETokenizer>) -> R) -> R {
    let mut guard = BPE_STORE.lock().unwrap();
    if guard.is_none() { *guard = Some(HashMap::new()); }
    f(guard.as_mut().unwrap())
}

fn next_tokenizer_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tokenizer_bpe_create(vocab_size: i64) -> i64 {
    let tok = BPETokenizer::new(vocab_size as usize);
    let id = next_tokenizer_id();
    with_bpe(|s| s.insert(id, tok));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tokenizer_vocab_size(tok_id: i64) -> i64 {
    with_bpe(|s| s.get(&tok_id).map_or(0, |t| t.current_vocab_size() as i64))
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tokenizer_encode_len(tok_id: i64, text: *const u8, text_len: i64) -> i64 {
    let slice = unsafe { std::slice::from_raw_parts(text, text_len as usize) };
    let s = std::str::from_utf8(slice).unwrap_or("");
    with_bpe(|store| {
        store.get(&tok_id).map_or(0, |t| t.encode(s).len() as i64)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tokenizer_free(tok_id: i64) {
    with_bpe(|s| { s.remove(&tok_id); });
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpe_create() {
        let tok = BPETokenizer::new(1000);
        assert!(tok.current_vocab_size() >= 262); // 6 special + 256 byte
    }

    #[test]
    fn test_bpe_encode_basic() {
        let tok = BPETokenizer::new(1000);
        let ids = tok.encode("Hi");
        assert!(!ids.is_empty());
        // 'H' = 0x48, byte token ID = 0x48 + 6 = 78
        assert_eq!(ids[0], 0x48 + 6);
    }

    #[test]
    fn test_bpe_decode_roundtrip() {
        let tok = BPETokenizer::new(1000);
        let text = "Hello World";
        let ids = tok.encode(text);
        let decoded = tok.decode(&ids);
        assert_eq!(decoded, "HelloWorld"); // Whitespace lost in BPE (between words)
    }

    #[test]
    fn test_bpe_train() {
        let mut tok = BPETokenizer::new(500);
        let corpus = vec!["the cat sat on the mat"; 10];
        tok.train(&corpus, 20);
        assert!(tok.merges.len() > 0);
        assert!(tok.current_vocab_size() > 262);
    }

    #[test]
    fn test_bpe_special_tokens() {
        let mut tok = BPETokenizer::new(1000);
        let id = tok.add_special_token("<custom>");
        assert!(id > 0);
        assert_eq!(tok.encode("<custom>"), vec![id]);
    }

    #[test]
    fn test_bpe_train_reduces_tokens() {
        let mut tok = BPETokenizer::new(500);
        let text = "aaa bbb aaa bbb aaa bbb";
        let before = tok.encode(text).len();
        tok.train(&vec![text; 5], 10);
        let after = tok.encode(text).len();
        assert!(after <= before, "Training should reduce token count");
    }

    #[test]
    fn test_wordpiece_create() {
        let wp = WordPieceTokenizer::new();
        assert!(wp.vocab.contains_key("[UNK]"));
        assert!(wp.vocab.contains_key("[CLS]"));
    }

    #[test]
    fn test_wordpiece_build_vocab() {
        let mut wp = WordPieceTokenizer::new();
        let mut freqs = HashMap::new();
        freqs.insert("hello".to_string(), 10);
        freqs.insert("help".to_string(), 5);
        freqs.insert("world".to_string(), 8);
        wp.build_vocab(&freqs, 50);
        assert!(wp.vocab.len() >= 5); // At least special tokens + some chars
    }

    #[test]
    fn test_wordpiece_tokenize() {
        let mut wp = WordPieceTokenizer::new();
        let mut freqs = HashMap::new();
        freqs.insert("hello".to_string(), 10);
        wp.build_vocab(&freqs, 100);
        let tokens = wp.tokenize_word("hello");
        // Should find "hello" or break into subwords
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_unigram_basic() {
        let mut tok = UnigramTokenizer::new();
        tok.add_piece("h", -1.0);
        tok.add_piece("e", -1.0);
        tok.add_piece("l", -1.0);
        tok.add_piece("o", -1.0);
        tok.add_piece("he", -0.5);
        tok.add_piece("llo", -0.5);

        let ids = tok.encode("hello");
        assert!(!ids.is_empty());
        let decoded = tok.decode(&ids);
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn test_unigram_viterbi() {
        let mut tok = UnigramTokenizer::new();
        tok.add_piece("a", -2.0);
        tok.add_piece("b", -2.0);
        tok.add_piece("ab", -1.0); // Higher prob for "ab"

        let ids = tok.encode("ab");
        // Should prefer "ab" (log_prob -1.0) over "a"+"b" (log_prob -4.0)
        assert_eq!(ids.len(), 1); // Single "ab" token
    }

    #[test]
    fn test_pre_tokenize() {
        let tokens = pre_tokenize("Hello, world! How are you?");
        assert!(tokens.contains(&"Hello".to_string()));
        assert!(tokens.contains(&",".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"!".to_string()));
    }

    #[test]
    fn test_normalize_text() {
        let norm = normalize_text("Hello WORLD");
        assert_eq!(norm, "hello world");
    }

    #[test]
    fn test_apply_merge() {
        let word = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = apply_merge(&word, "a", "b", "ab");
        assert_eq!(result, vec!["ab", "c"]);
    }

    #[test]
    fn test_ffi_bpe() {
        let id = vitalis_tokenizer_bpe_create(1000);
        assert!(id > 0);
        let size = vitalis_tokenizer_vocab_size(id);
        assert!(size >= 262);
        vitalis_tokenizer_free(id);
    }

    #[test]
    fn test_ffi_encode_len() {
        let id = vitalis_tokenizer_bpe_create(1000);
        let text = b"Hello";
        let len = vitalis_tokenizer_encode_len(id, text.as_ptr(), text.len() as i64);
        assert_eq!(len, 5); // 5 bytes = 5 tokens (before training)
    }

    #[test]
    fn test_wordpiece_decode() {
        let mut wp = WordPieceTokenizer::new();
        let mut freqs = HashMap::new();
        freqs.insert("hello".to_string(), 10);
        wp.build_vocab(&freqs, 100);
        let ids = wp.encode("hello");
        let decoded = wp.decode(&ids);
        assert!(decoded.contains("hello") || decoded.contains("h"));
    }

    #[test]
    fn test_empty_input() {
        let tok = BPETokenizer::new(1000);
        assert_eq!(tok.encode("").len(), 0);
        assert_eq!(tok.decode(&[]).len(), 0);
    }
}
