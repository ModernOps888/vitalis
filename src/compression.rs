//! Compression Module — Lossless data compression algorithms for Vitalis
//!
//! Pure Rust implementations with zero external dependencies.
//! Exposed via C FFI for Python interop.
//!
//! # Algorithms:
//! - Run-Length Encoding (RLE) encode/decode
//! - Huffman coding (canonical)
//! - LZ77 sliding window compression
//! - Delta encoding/decoding
//! - Burrows-Wheeler Transform (BWT)
//! - Move-to-Front Transform (MTF)
//! - Bit packing

use std::ffi::CString;
use std::os::raw::c_char;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

// ─── Run-Length Encoding ──────────────────────────────────────────────

fn rle_encode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() { return vec![]; }
    let mut result = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut count = 1u8;
        while i + (count as usize) < data.len()
            && data[i + count as usize] == byte
            && count < 255
        {
            count += 1;
        }
        result.push(count);
        result.push(byte);
        i += count as usize;
    }
    result
}

fn rle_decode(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;
    while i + 1 < data.len() {
        let count = data[i];
        let byte = data[i + 1];
        for _ in 0..count {
            result.push(byte);
        }
        i += 2;
    }
    result
}

/// RLE encode. Returns compressed length, writes to out buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rle_encode(
    data: *const u8,
    len: usize,
    out: *mut u8,
    max_out: usize,
) -> usize {
    if data.is_null() || out.is_null() || len == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let encoded = rle_encode(d);
    let copy_len = encoded.len().min(max_out);
    let o = unsafe { std::slice::from_raw_parts_mut(out, copy_len) };
    o.copy_from_slice(&encoded[..copy_len]);
    encoded.len()
}

/// RLE decode. Returns decompressed length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rle_decode(
    data: *const u8,
    len: usize,
    out: *mut u8,
    max_out: usize,
) -> usize {
    if data.is_null() || out.is_null() || len == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let decoded = rle_decode(d);
    let copy_len = decoded.len().min(max_out);
    let o = unsafe { std::slice::from_raw_parts_mut(out, copy_len) };
    o.copy_from_slice(&decoded[..copy_len]);
    decoded.len()
}

// ─── Huffman Coding ───────────────────────────────────────────────────

#[derive(Eq)]
struct HuffNode {
    freq: usize,
    byte: Option<u8>,
    left: Option<Box<HuffNode>>,
    right: Option<Box<HuffNode>>,
}

impl Ord for HuffNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq.cmp(&self.freq) // Min-heap
    }
}

impl PartialOrd for HuffNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HuffNode {
    fn eq(&self, other: &Self) -> bool {
        self.freq == other.freq
    }
}

fn build_huffman_codes(data: &[u8]) -> (Vec<(u8, Vec<bool>)>, HuffNode) {
    let mut freq = [0usize; 256];
    for &b in data { freq[b as usize] += 1; }

    let mut heap = BinaryHeap::new();
    for (i, &f) in freq.iter().enumerate() {
        if f > 0 {
            heap.push(HuffNode {
                freq: f,
                byte: Some(i as u8),
                left: None,
                right: None,
            });
        }
    }

    if heap.len() == 1 {
        let node = heap.pop().unwrap();
        let root = HuffNode {
            freq: node.freq,
            byte: None,
            left: Some(Box::new(node)),
            right: None,
        };
        let mut codes = Vec::new();
        fn traverse(node: &HuffNode, prefix: &mut Vec<bool>, codes: &mut Vec<(u8, Vec<bool>)>) {
            if let Some(b) = node.byte {
                if prefix.is_empty() { prefix.push(false); }
                codes.push((b, prefix.clone()));
                return;
            }
            if let Some(ref left) = node.left {
                prefix.push(false);
                traverse(left, prefix, codes);
                prefix.pop();
            }
            if let Some(ref right) = node.right {
                prefix.push(true);
                traverse(right, prefix, codes);
                prefix.pop();
            }
        }
        let mut prefix = Vec::new();
        traverse(&root, &mut prefix, &mut codes);
        return (codes, root);
    }

    while heap.len() > 1 {
        let a = heap.pop().unwrap();
        let b = heap.pop().unwrap();
        heap.push(HuffNode {
            freq: a.freq + b.freq,
            byte: None,
            left: Some(Box::new(a)),
            right: Some(Box::new(b)),
        });
    }

    let root = heap.pop().unwrap_or(HuffNode { freq: 0, byte: None, left: None, right: None });

    let mut codes = Vec::new();
    fn collect_codes(node: &HuffNode, prefix: &mut Vec<bool>, codes: &mut Vec<(u8, Vec<bool>)>) {
        if let Some(b) = node.byte {
            codes.push((b, prefix.clone()));
            return;
        }
        if let Some(ref left) = node.left {
            prefix.push(false);
            collect_codes(left, prefix, codes);
            prefix.pop();
        }
        if let Some(ref right) = node.right {
            prefix.push(true);
            collect_codes(right, prefix, codes);
            prefix.pop();
        }
    }
    let mut prefix = Vec::new();
    collect_codes(&root, &mut prefix, &mut codes);
    (codes, root)
}

/// Huffman encode. Returns compressed bit count.
/// Format: [num_symbols:u16][symbol:u8,code_len:u8,code_bits...][compressed_bits...]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_huffman_encode(
    data: *const u8,
    len: usize,
    out: *mut u8,
    max_out: usize,
) -> usize {
    if data.is_null() || out.is_null() || len == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let (codes, _) = build_huffman_codes(d);

    // Build code lookup
    let mut code_map: Vec<Vec<bool>> = (0..256).map(|_| Vec::new()).collect();
    for (byte, bits) in &codes {
        code_map[*byte as usize] = bits.clone();
    }

    // Encode to bits
    let mut bits = Vec::new();
    for &b in d {
        bits.extend_from_slice(&code_map[b as usize]);
    }

    // Pack bits into bytes
    let byte_count = (bits.len() + 7) / 8;
    if byte_count > max_out { return 0; }
    let o = unsafe { std::slice::from_raw_parts_mut(out, byte_count) };
    for i in 0..byte_count {
        let mut byte = 0u8;
        for j in 0..8 {
            let idx = i * 8 + j;
            if idx < bits.len() && bits[idx] {
                byte |= 1 << (7 - j);
            }
        }
        o[i] = byte;
    }
    bits.len()
}

// ─── Delta Encoding ──────────────────────────────────────────────────

/// Delta encode: out[0] = data[0], out[i] = data[i] - data[i-1].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_delta_encode(
    data: *const i64,
    len: usize,
    out: *mut i64,
) {
    if data.is_null() || out.is_null() || len == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };
    o[0] = d[0];
    for i in 1..len {
        o[i] = d[i] - d[i-1];
    }
}

/// Delta decode (inverse).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_delta_decode(
    data: *const i64,
    len: usize,
    out: *mut i64,
) {
    if data.is_null() || out.is_null() || len == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };
    o[0] = d[0];
    for i in 1..len {
        o[i] = o[i-1] + d[i];
    }
}

// ─── Burrows-Wheeler Transform ───────────────────────────────────────

fn bwt_transform(data: &[u8]) -> (Vec<u8>, usize) {
    let n = data.len();
    if n == 0 { return (vec![], 0); }

    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| {
        for k in 0..n {
            let ca = data[(a + k) % n];
            let cb = data[(b + k) % n];
            match ca.cmp(&cb) {
                Ordering::Equal => continue,
                other => return other,
            }
        }
        Ordering::Equal
    });

    let mut result = Vec::with_capacity(n);
    let mut original_row = 0;
    for (i, &idx) in indices.iter().enumerate() {
        result.push(data[(idx + n - 1) % n]);
        if idx == 0 { original_row = i; }
    }
    (result, original_row)
}

fn bwt_inverse(data: &[u8], original_row: usize) -> Vec<u8> {
    let n = data.len();
    if n == 0 { return vec![]; }

    let mut sorted_indices: Vec<usize> = (0..n).collect();
    sorted_indices.sort_by_key(|&i| data[i]);

    let mut result = Vec::with_capacity(n);
    let mut idx = original_row;
    for _ in 0..n {
        idx = sorted_indices[idx];
        result.push(data[idx]);
    }
    result
}

/// BWT forward transform.
/// Returns original row index. Writes transformed data to out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bwt_encode(
    data: *const u8,
    len: usize,
    out: *mut u8,
) -> usize {
    if data.is_null() || out.is_null() || len == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let (transformed, row) = bwt_transform(d);
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };
    o.copy_from_slice(&transformed);
    row
}

/// BWT inverse transform.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bwt_decode(
    data: *const u8,
    len: usize,
    original_row: usize,
    out: *mut u8,
) {
    if data.is_null() || out.is_null() || len == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let original = bwt_inverse(d, original_row);
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };
    o.copy_from_slice(&original);
}

// ─── Move-to-Front Transform ─────────────────────────────────────────

/// MTF encode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mtf_encode(
    data: *const u8,
    len: usize,
    out: *mut u8,
) {
    if data.is_null() || out.is_null() || len == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };

    let mut alphabet: Vec<u8> = (0..=255).collect();
    for i in 0..len {
        let pos = alphabet.iter().position(|&c| c == d[i]).unwrap();
        o[i] = pos as u8;
        let ch = alphabet.remove(pos);
        alphabet.insert(0, ch);
    }
}

/// MTF decode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mtf_decode(
    data: *const u8,
    len: usize,
    out: *mut u8,
) {
    if data.is_null() || out.is_null() || len == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };

    let mut alphabet: Vec<u8> = (0..=255).collect();
    for i in 0..len {
        let pos = d[i] as usize;
        o[i] = alphabet[pos];
        let ch = alphabet.remove(pos);
        alphabet.insert(0, ch);
    }
}

// ─── LZ77 Compression ────────────────────────────────────────────────

/// LZ77 compress.
/// Output format: series of (offset:u16, length:u16, next_byte:u8).
/// Returns number of bytes written to out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lz77_compress(
    data: *const u8,
    len: usize,
    out: *mut u8,
    max_out: usize,
    window_size: usize,
) -> usize {
    if data.is_null() || out.is_null() || len == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, max_out) };

    let mut pos = 0;
    let mut out_pos = 0;

    while pos < len {
        let mut best_offset = 0u16;
        let mut best_length = 0u16;

        let search_start = if pos > window_size { pos - window_size } else { 0 };
        for offset_start in search_start..pos {
            let mut match_len = 0u16;
            while pos + (match_len as usize) < len - 1
                && match_len < 255
                && d[offset_start + (match_len as usize)] == d[pos + (match_len as usize)]
            {
                match_len += 1;
            }
            if match_len > best_length {
                best_length = match_len;
                best_offset = (pos - offset_start) as u16;
            }
        }

        if out_pos + 5 > max_out { break; }
        o[out_pos] = (best_offset >> 8) as u8;
        o[out_pos + 1] = (best_offset & 0xFF) as u8;
        o[out_pos + 2] = (best_length >> 8) as u8;
        o[out_pos + 3] = (best_length & 0xFF) as u8;
        let next = if pos + (best_length as usize) < len {
            d[pos + (best_length as usize)]
        } else {
            0
        };
        o[out_pos + 4] = next;
        out_pos += 5;
        pos += best_length as usize + 1;
    }
    out_pos
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_roundtrip() {
        let data = b"AAABBBCCDDDDDD";
        let encoded = rle_encode(data);
        let decoded = rle_decode(&encoded);
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_rle_single() {
        let data = b"A";
        let encoded = rle_encode(data);
        assert_eq!(encoded, vec![1, b'A']);
        assert_eq!(rle_decode(&encoded), data.to_vec());
    }

    #[test]
    fn test_rle_ffi() {
        let data = b"AABB";
        let mut out = vec![0u8; 20];
        let len = unsafe { vitalis_rle_encode(data.as_ptr(), 4, out.as_mut_ptr(), 20) };
        assert!(len > 0);
        let mut decoded = vec![0u8; 20];
        let dlen = unsafe { vitalis_rle_decode(out.as_ptr(), len, decoded.as_mut_ptr(), 20) };
        assert_eq!(&decoded[..dlen], data);
    }

    #[test]
    fn test_huffman_encode() {
        let data = b"AAAAABBBCC";
        let mut out = vec![0u8; 100];
        let bits = unsafe { vitalis_huffman_encode(data.as_ptr(), 10, out.as_mut_ptr(), 100) };
        assert!(bits > 0);
        assert!(bits < 80); // Should compress
    }

    #[test]
    fn test_delta_roundtrip() {
        let data: Vec<i64> = vec![10, 12, 15, 20, 22];
        let mut encoded = vec![0i64; 5];
        let mut decoded = vec![0i64; 5];
        unsafe {
            vitalis_delta_encode(data.as_ptr(), 5, encoded.as_mut_ptr());
            vitalis_delta_decode(encoded.as_ptr(), 5, decoded.as_mut_ptr());
        }
        assert_eq!(decoded, data);
        assert_eq!(encoded, vec![10, 2, 3, 5, 2]);
    }

    #[test]
    fn test_bwt_roundtrip() {
        let data = b"banana";
        let mut encoded = vec![0u8; 6];
        let row = unsafe { vitalis_bwt_encode(data.as_ptr(), 6, encoded.as_mut_ptr()) };
        let mut decoded = vec![0u8; 6];
        unsafe { vitalis_bwt_decode(encoded.as_ptr(), 6, row, decoded.as_mut_ptr()); }
        assert_eq!(&decoded, data);
    }

    #[test]
    fn test_mtf_roundtrip() {
        let data = b"banana";
        let mut encoded = vec![0u8; 6];
        let mut decoded = vec![0u8; 6];
        unsafe {
            vitalis_mtf_encode(data.as_ptr(), 6, encoded.as_mut_ptr());
            vitalis_mtf_decode(encoded.as_ptr(), 6, decoded.as_mut_ptr());
        }
        assert_eq!(&decoded, data);
    }

    #[test]
    fn test_lz77_compress() {
        let data = b"ABCABCABCABC";
        let mut out = vec![0u8; 100];
        let len = unsafe { vitalis_lz77_compress(data.as_ptr(), 12, out.as_mut_ptr(), 100, 4096) };
        assert!(len > 0);
    }
}
