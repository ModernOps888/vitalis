//! BPE Tokenizer — Byte Pair Encoding for language model tokenization
//!
//! Self-contained BPE tokenizer with training, encoding, decoding,
//! serialization, and special token support. Ported from the Nova ML engine.
//!
//! # Example
//!
//! ```rust,no_run
//! use vitalis::bpe_tokenizer::BpeTokenizer;
//! let mut tokenizer = BpeTokenizer::new(8000);
//! tokenizer.train("Hello world. This is a test.");
//! let ids = tokenizer.encode("Hello world");
//! let text = tokenizer.decode(&ids);
//! ```

use std::collections::HashMap;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Constants
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Reserved special tokens
pub const PAD_TOKEN: u32 = 0;
pub const BOS_TOKEN: u32 = 1;
pub const EOS_TOKEN: u32 = 2;
pub const UNK_TOKEN: u32 = 3;
const NUM_SPECIAL: u32 = 4;

/// Token representing an unknown byte
const BYTE_OFFSET: u32 = NUM_SPECIAL;
const NUM_BYTE_TOKENS: u32 = 256;
const MERGE_OFFSET: u32 = BYTE_OFFSET + NUM_BYTE_TOKENS; // = 260

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BPE Tokenizer
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A merge rule: (token_a, token_b) → merged_token
#[derive(Debug, Clone)]
pub struct MergeRule {
    pub a: u32,
    pub b: u32,
    pub merged: u32,
}

/// Byte Pair Encoding tokenizer.
pub struct BpeTokenizer {
    pub vocab_size: usize,
    merges: Vec<MergeRule>,
    vocab: Vec<Vec<u8>>,    // token_id → byte sequence
    merge_map: HashMap<(u32, u32), u32>,  // (a, b) → merged
}

impl BpeTokenizer {
    /// Create an empty tokenizer with target vocabulary size.
    pub fn new(vocab_size: usize) -> Self {
        let vocab_size = vocab_size.max(MERGE_OFFSET as usize + 1);
        let mut vocab: Vec<Vec<u8>> = Vec::with_capacity(vocab_size);

        // Special tokens
        vocab.push(b"<pad>".to_vec());
        vocab.push(b"<bos>".to_vec());
        vocab.push(b"<eos>".to_vec());
        vocab.push(b"<unk>".to_vec());

        // Individual byte tokens (256)
        for b in 0..=255u8 {
            vocab.push(vec![b]);
        }

        Self {
            vocab_size,
            merges: Vec::new(),
            vocab,
            merge_map: HashMap::new(),
        }
    }

    /// Train BPE merges from text corpus.
    pub fn train(&mut self, text: &str) {
        let mut ids: Vec<u32> = text.as_bytes().iter()
            .map(|&b| b as u32 + BYTE_OFFSET)
            .collect();

        let target_merges = self.vocab_size - MERGE_OFFSET as usize;

        for merge_idx in 0..target_merges {
            if ids.len() < 2 { break; }

            // Count all adjacent pairs
            let mut pair_counts: HashMap<(u32, u32), usize> = HashMap::new();
            for w in ids.windows(2) {
                *pair_counts.entry((w[0], w[1])).or_insert(0) += 1;
            }

            // Find most frequent pair
            let best = match pair_counts.into_iter().max_by_key(|&(_, c)| c) {
                Some((pair, count)) if count >= 2 => pair,
                _ => break,
            };

            let new_id = MERGE_OFFSET + merge_idx as u32;
            let rule = MergeRule { a: best.0, b: best.1, merged: new_id };

            // Merge in-place
            let mut new_ids = Vec::with_capacity(ids.len());
            let mut i = 0;
            while i < ids.len() {
                if i + 1 < ids.len() && ids[i] == best.0 && ids[i + 1] == best.1 {
                    new_ids.push(new_id);
                    i += 2;
                } else {
                    new_ids.push(ids[i]);
                    i += 1;
                }
            }
            ids = new_ids;

            // Build vocab entry: concatenate byte sequences
            let mut bytes = self.get_bytes(best.0);
            bytes.extend_from_slice(&self.get_bytes(best.1));
            if (new_id as usize) < self.vocab.len() {
                self.vocab[new_id as usize] = bytes;
            } else {
                while self.vocab.len() < new_id as usize {
                    self.vocab.push(vec![]);
                }
                self.vocab.push(bytes);
            }

            self.merge_map.insert(best, new_id);
            self.merges.push(rule);
        }
    }

    /// Encode text to token IDs.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut ids: Vec<u32> = text.as_bytes().iter()
            .map(|&b| b as u32 + BYTE_OFFSET)
            .collect();

        // Apply merges in order
        for rule in &self.merges {
            let mut new_ids = Vec::with_capacity(ids.len());
            let mut i = 0;
            while i < ids.len() {
                if i + 1 < ids.len() && ids[i] == rule.a && ids[i + 1] == rule.b {
                    new_ids.push(rule.merged);
                    i += 2;
                } else {
                    new_ids.push(ids[i]);
                    i += 1;
                }
            }
            ids = new_ids;
        }
        ids
    }

    /// Encode with BOS and EOS special tokens.
    pub fn encode_with_special(&self, text: &str) -> Vec<u32> {
        let mut ids = vec![BOS_TOKEN];
        ids.extend(self.encode(text));
        ids.push(EOS_TOKEN);
        ids
    }

    /// Decode token IDs back to text.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut bytes = Vec::new();
        for &id in ids {
            if id == PAD_TOKEN || id == BOS_TOKEN || id == EOS_TOKEN { continue; }
            if id == UNK_TOKEN { bytes.extend_from_slice(b"<unk>"); continue; }
            bytes.extend_from_slice(&self.get_bytes(id));
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Get byte sequence for a token.
    fn get_bytes(&self, id: u32) -> Vec<u8> {
        if (id as usize) < self.vocab.len() {
            self.vocab[id as usize].clone()
        } else {
            vec![]
        }
    }

    /// Current vocabulary size (actual).
    pub fn actual_vocab_size(&self) -> usize {
        MERGE_OFFSET as usize + self.merges.len()
    }

    /// Number of learned merges.
    pub fn num_merges(&self) -> usize {
        self.merges.len()
    }

    /// Look up token → string.
    pub fn id_to_token(&self, id: u32) -> String {
        match id {
            PAD_TOKEN => "<pad>".into(),
            BOS_TOKEN => "<bos>".into(),
            EOS_TOKEN => "<eos>".into(),
            UNK_TOKEN => "<unk>".into(),
            _ => String::from_utf8_lossy(&self.get_bytes(id)).into_owned(),
        }
    }

    /// Save tokenizer to binary file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(path)?;
        // Header: vocab_size, num_merges
        f.write_all(&(self.vocab_size as u32).to_le_bytes())?;
        f.write_all(&(self.merges.len() as u32).to_le_bytes())?;
        // Write each merge rule
        for rule in &self.merges {
            f.write_all(&rule.a.to_le_bytes())?;
            f.write_all(&rule.b.to_le_bytes())?;
            f.write_all(&rule.merged.to_le_bytes())?;
        }
        Ok(())
    }

    /// Load tokenizer from binary file.
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        use std::io::Read;
        let mut f = std::fs::File::open(path)?;
        let mut buf4 = [0u8; 4];

        f.read_exact(&mut buf4)?;
        let vocab_size = u32::from_le_bytes(buf4) as usize;
        f.read_exact(&mut buf4)?;
        let num_merges = u32::from_le_bytes(buf4) as usize;

        let mut tokenizer = Self::new(vocab_size);

        for _ in 0..num_merges {
            f.read_exact(&mut buf4)?;
            let a = u32::from_le_bytes(buf4);
            f.read_exact(&mut buf4)?;
            let b = u32::from_le_bytes(buf4);
            f.read_exact(&mut buf4)?;
            let merged = u32::from_le_bytes(buf4);

            let mut bytes = tokenizer.get_bytes(a);
            bytes.extend_from_slice(&tokenizer.get_bytes(b));
            while tokenizer.vocab.len() <= merged as usize {
                tokenizer.vocab.push(vec![]);
            }
            tokenizer.vocab[merged as usize] = bytes;
            tokenizer.merge_map.insert((a, b), merged);
            tokenizer.merges.push(MergeRule { a, b, merged });
        }
        Ok(tokenizer)
    }

    /// Tokenize text file and return flat token vector.
    pub fn tokenize_file(&self, path: &std::path::Path) -> std::io::Result<Vec<u32>> {
        let text = std::fs::read_to_string(path)?;
        Ok(self.encode(&text))
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FFI — extern "C" functions for Vitalis stdlib
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a BPE tokenizer with given vocab size.
#[unsafe(no_mangle)]
pub extern "C" fn vitalis_tokenizer_new(vocab_size: i64) -> i64 {
    let tok = Box::new(BpeTokenizer::new(vocab_size as usize));
    Box::into_raw(tok) as i64
}

/// Get actual vocabulary size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tokenizer_vocab_size(handle: i64) -> i64 {
    let tok = unsafe { &*(handle as *const BpeTokenizer) };
    tok.actual_vocab_size() as i64
}

/// Number of learned merges.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tokenizer_num_merges(handle: i64) -> i64 {
    let tok = unsafe { &*(handle as *const BpeTokenizer) };
    tok.num_merges() as i64
}

/// Free a tokenizer handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tokenizer_free(handle: i64) {
    if handle != 0 { let _ = unsafe { Box::from_raw(handle as *mut BpeTokenizer) }; }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_encode_decode() {
        let mut tok = BpeTokenizer::new(300);
        tok.train("hello world hello world hello world hello");
        let ids = tok.encode("hello");
        let decoded = tok.decode(&ids);
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn test_special_tokens() {
        let tok = BpeTokenizer::new(300);
        let ids = tok.encode_with_special("test");
        assert_eq!(ids[0], BOS_TOKEN);
        assert_eq!(*ids.last().unwrap(), EOS_TOKEN);
    }

    #[test]
    fn test_roundtrip() {
        let mut tok = BpeTokenizer::new(500);
        let text = "The quick brown fox jumps over the lazy dog. ";
        let corpus = text.repeat(50);
        tok.train(&corpus);
        let ids = tok.encode(text);
        let decoded = tok.decode(&ids);
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_compression() {
        let mut tok = BpeTokenizer::new(1000);
        let text = "abcabc".repeat(100);
        tok.train(&text);
        let ids = tok.encode(&text);
        // BPE should compress repeated patterns
        assert!(ids.len() < text.len());
    }

    #[test]
    fn test_id_to_token() {
        let tok = BpeTokenizer::new(300);
        assert_eq!(tok.id_to_token(PAD_TOKEN), "<pad>");
        assert_eq!(tok.id_to_token(BOS_TOKEN), "<bos>");
        assert_eq!(tok.id_to_token(EOS_TOKEN), "<eos>");
        assert_eq!(tok.id_to_token(UNK_TOKEN), "<unk>");
    }

    #[test]
    fn test_save_load_roundtrip() {
        let mut tok = BpeTokenizer::new(500);
        tok.train(&"hello world ".repeat(100));
        let ids_before = tok.encode("hello world");

        let tmp = std::env::temp_dir().join("vitalis_bpe_test.bin");
        tok.save(&tmp).unwrap();
        let tok2 = BpeTokenizer::load(&tmp).unwrap();
        let ids_after = tok2.encode("hello world");
        assert_eq!(ids_before, ids_after);
        let _ = std::fs::remove_file(&tmp);
    }
}
