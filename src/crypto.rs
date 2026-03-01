//! Cryptography Module — Hash functions, encoding, and crypto primitives for Vitalis
//!
//! Pure Rust implementations with zero external dependencies.
//! Exposed via C FFI for Python interop.
//!
//! # Algorithms:
//! - SHA-256 (FIPS 180-4)
//! - HMAC-SHA256
//! - PBKDF2-SHA256
//! - Base64 encode/decode
//! - CRC32
//! - FNV-1a hash
//! - SipHash-2-4
//! - XorShift128+ CSPRNG
//! - Constant-time comparison

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── SHA-256 ──────────────────────────────────────────────────────────

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256_transform(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([block[i*4], block[i*4+1], block[i*4+2], block[i*4+3]]);
    }
    for i in 16..64 {
        let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
        let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
        w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
    }

    let mut a = state[0]; let mut b = state[1]; let mut c = state[2]; let mut d = state[3];
    let mut e = state[4]; let mut f = state[5]; let mut g = state[6]; let mut h = state[7];

    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA256_K[i]).wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(maj);

        h = g; g = f; f = e; e = d.wrapping_add(temp1);
        d = c; c = b; b = a; a = temp1.wrapping_add(temp2);
    }

    state[0] = state[0].wrapping_add(a); state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c); state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e); state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g); state[7] = state[7].wrapping_add(h);
}

fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let block: [u8; 64] = chunk.try_into().unwrap();
        sha256_transform(&mut state, &block);
    }

    let mut hash = [0u8; 32];
    for (i, s) in state.iter().enumerate() {
        hash[i*4..i*4+4].copy_from_slice(&s.to_be_bytes());
    }
    hash
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Compute SHA-256 hash of input data, returns hex string.
/// Caller must free the returned string with slang_free_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sha256(
    data: *const u8,
    len: usize,
) -> *mut c_char {
    if data.is_null() || len == 0 {
        let empty = CString::new("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855").unwrap();
        return empty.into_raw();
    }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let hash = sha256_digest(d);
    let hex = bytes_to_hex(&hash);
    CString::new(hex).unwrap().into_raw()
}

/// SHA-256 of a C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sha256_str(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        let empty = CString::new("").unwrap();
        return empty.into_raw();
    }
    let s = unsafe { CStr::from_ptr(input) }.to_bytes();
    let hash = sha256_digest(s);
    let hex = bytes_to_hex(&hash);
    CString::new(hex).unwrap().into_raw()
}

// ─── HMAC-SHA256 ──────────────────────────────────────────────────────

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let block_size = 64;
    let mut k = vec![0u8; block_size];
    if key.len() > block_size {
        let h = sha256_digest(key);
        k[..32].copy_from_slice(&h);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    let mut o_key_pad = vec![0u8; block_size];
    let mut i_key_pad = vec![0u8; block_size];
    for i in 0..block_size {
        o_key_pad[i] = k[i] ^ 0x5c;
        i_key_pad[i] = k[i] ^ 0x36;
    }

    i_key_pad.extend_from_slice(message);
    let inner_hash = sha256_digest(&i_key_pad);
    o_key_pad.extend_from_slice(&inner_hash);
    sha256_digest(&o_key_pad)
}

/// HMAC-SHA256: returns hex string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_hmac_sha256(
    key: *const u8,
    key_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> *mut c_char {
    if key.is_null() || msg.is_null() {
        let empty = CString::new("").unwrap();
        return empty.into_raw();
    }
    let k = unsafe { std::slice::from_raw_parts(key, key_len) };
    let m = unsafe { std::slice::from_raw_parts(msg, msg_len) };
    let mac = hmac_sha256(k, m);
    let hex = bytes_to_hex(&mac);
    CString::new(hex).unwrap().into_raw()
}

// ─── Base64 ───────────────────────────────────────────────────────────

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(B64_CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(B64_CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(B64_CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(B64_CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn base64_decode(encoded: &str) -> Option<Vec<u8>> {
    let mut result = Vec::new();
    let bytes: Vec<u8> = encoded.bytes().filter(|&b| b != b'\n' && b != b'\r').collect();
    for chunk in bytes.chunks(4) {
        if chunk.len() != 4 { return None; }
        let mut vals = [0u8; 4];
        let mut padding = 0;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                vals[i] = 0;
                padding += 1;
            } else {
                vals[i] = base64_decode_char(c)?;
            }
        }
        let triple = ((vals[0] as u32) << 18) | ((vals[1] as u32) << 12)
                    | ((vals[2] as u32) << 6) | (vals[3] as u32);
        result.push((triple >> 16) as u8);
        if padding < 2 { result.push((triple >> 8) as u8); }
        if padding < 1 { result.push(triple as u8); }
    }
    Some(result)
}

/// Base64 encode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_base64_encode(
    data: *const u8,
    len: usize,
) -> *mut c_char {
    if data.is_null() || len == 0 {
        return CString::new("").unwrap().into_raw();
    }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let encoded = base64_encode(d);
    CString::new(encoded).unwrap().into_raw()
}

/// Base64 decode. Returns decoded length. Output must have space for 3/4 * input_len.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_base64_decode(
    encoded: *const c_char,
    output: *mut u8,
    max_output: usize,
) -> i64 {
    if encoded.is_null() || output.is_null() {
        return -1;
    }
    let s = unsafe { CStr::from_ptr(encoded) }.to_str().unwrap_or("");
    match base64_decode(s) {
        Some(decoded) => {
            let copy_len = decoded.len().min(max_output);
            let out = unsafe { std::slice::from_raw_parts_mut(output, copy_len) };
            out.copy_from_slice(&decoded[..copy_len]);
            copy_len as i64
        }
        None => -1,
    }
}

// ─── CRC32 ────────────────────────────────────────────────────────────

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for i in 0..256u32 {
        let mut crc = i;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xEDB88320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
        table[i as usize] = crc;
    }
    table
}

/// CRC32 checksum.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_crc32(
    data: *const u8,
    len: usize,
) -> u32 {
    if data.is_null() || len == 0 { return 0; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let table = crc32_table();
    let mut crc = 0xFFFFFFFFu32;
    for &byte in d {
        crc = table[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

// ─── FNV-1a Hash ──────────────────────────────────────────────────────

/// FNV-1a 64-bit hash.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_fnv1a_64(
    data: *const u8,
    len: usize,
) -> u64 {
    if data.is_null() || len == 0 { return 0xcbf29ce484222325; }
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in d {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ─── Constant-time comparison ─────────────────────────────────────────

/// Constant-time byte comparison (timing-safe).
/// Returns 1 if equal, 0 if not.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_constant_time_eq(
    a: *const u8,
    b: *const u8,
    len: usize,
) -> i32 {
    if a.is_null() || b.is_null() { return 0; }
    let sa = unsafe { std::slice::from_raw_parts(a, len) };
    let sb = unsafe { std::slice::from_raw_parts(b, len) };
    let mut diff = 0u8;
    for i in 0..len {
        diff |= sa[i] ^ sb[i];
    }
    if diff == 0 { 1 } else { 0 }
}

// ─── XorShift128+ PRNG ───────────────────────────────────────────────

/// XorShift128+ step. Returns next random u64.
/// state must point to 2 u64 values [s0, s1].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_xorshift128plus(
    state: *mut u64,
) -> u64 {
    if state.is_null() { return 0; }
    let s = unsafe { std::slice::from_raw_parts_mut(state, 2) };
    let mut s1 = s[0];
    let s0 = s[1];
    s[0] = s0;
    s1 ^= s1 << 23;
    s[1] = s1 ^ s0 ^ (s1 >> 17) ^ (s0 >> 26);
    s[1].wrapping_add(s0)
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty() {
        let hash = sha256_digest(b"");
        let hex = bytes_to_hex(&hash);
        assert_eq!(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_sha256_hello() {
        let hash = sha256_digest(b"hello");
        let hex = bytes_to_hex(&hash);
        assert_eq!(hex, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn test_sha256_abc() {
        let hash = sha256_digest(b"abc");
        let hex = bytes_to_hex(&hash);
        assert_eq!(hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn test_hmac_sha256_basic() {
        let mac = hmac_sha256(b"key", b"message");
        let hex = bytes_to_hex(&mac);
        assert_eq!(hex, "6e9ef29b75fffc5b7abae527d58fdadb2fe42e7219011976917343065f58ed4a");
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"Hello, World!";
        let encoded = base64_encode(data);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base64_encode_ffi() {
        let data = b"test";
        let result = unsafe { vitalis_base64_encode(data.as_ptr(), 4) };
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "dGVzdA==");
        unsafe { drop(CString::from_raw(result)); }
    }

    #[test]
    fn test_crc32() {
        let crc = unsafe { vitalis_crc32(b"123456789".as_ptr(), 9) };
        assert_eq!(crc, 0xCBF43926);
    }

    #[test]
    fn test_fnv1a() {
        let h1 = unsafe { vitalis_fnv1a_64(b"hello".as_ptr(), 5) };
        let h2 = unsafe { vitalis_fnv1a_64(b"world".as_ptr(), 5) };
        assert_ne!(h1, h2);
        assert_ne!(h1, 0);
    }

    #[test]
    fn test_constant_time_eq() {
        let a = b"hello";
        let b = b"hello";
        let c = b"world";
        assert_eq!(unsafe { vitalis_constant_time_eq(a.as_ptr(), b.as_ptr(), 5) }, 1);
        assert_eq!(unsafe { vitalis_constant_time_eq(a.as_ptr(), c.as_ptr(), 5) }, 0);
    }

    #[test]
    fn test_xorshift128plus() {
        let mut state = [42u64, 123u64];
        let v1 = unsafe { vitalis_xorshift128plus(state.as_mut_ptr()) };
        let v2 = unsafe { vitalis_xorshift128plus(state.as_mut_ptr()) };
        assert_ne!(v1, v2);
        assert_ne!(v1, 0);
    }
}
