//! Security & Guardrails Module for Vitalis v9.0
//!
//! Pure Rust implementations of input validation, injection detection,
//! password strength, resource limits, content safety, audit hashing,
//! rate limiting, sandboxing, and sanitization.

// --- Input Validation ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_validate_email(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len == 0 { return 0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 { return 0; }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || domain.is_empty() { return 0; }
    if !domain.contains('.') { return 0; }
    let dparts: Vec<&str> = domain.split('.').collect();
    if dparts.last().map_or(true, |t| t.len() < 2) { return 0; }
    for c in local.chars() {
        if !c.is_alphanumeric() && !"._+-".contains(c) { return 0; }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_validate_ipv4(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len == 0 { return 0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 { return 0; }
    for p in &parts {
        match p.parse::<u32>() {
            Ok(n) if n <= 255 => {},
            _ => return 0,
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_validate_range(value: f64, min: f64, max: f64) -> i32 {
    if value >= min && value <= max { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_validate_length(ptr: *const u8, len: usize, min_len: usize, max_len: usize) -> i32 {
    if ptr.is_null() { return 0; }
    if len >= min_len && len <= max_len { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_validate_url(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len < 10 { return 0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    if !s.starts_with("http://") && !s.starts_with("https://") { return 0; }
    let after = if s.starts_with("https://") { &s[8..] } else { &s[7..] };
    if after.is_empty() || !after.contains('.') { return 0; }
    1
}

// --- Injection Detection ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_detect_sqli(ptr: *const u8, len: usize) -> f64 {
    if ptr.is_null() || len == 0 { return 0.0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    let lower = s.to_lowercase();
    let mut score = 0.0;
    let patterns = ["union select", "or 1=1", "drop table", "--", ";", "'", "exec(",
        "insert into", "delete from", "update set", "xp_cmdshell", "information_schema"];
    for p in &patterns {
        if lower.contains(p) { score += 1.0; }
    }
    (score / patterns.len() as f64 * 100.0).min(100.0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_detect_xss(ptr: *const u8, len: usize) -> f64 {
    if ptr.is_null() || len == 0 { return 0.0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    let lower = s.to_lowercase();
    let mut score = 0.0;
    let patterns = ["<script", "javascript:", "onerror=", "onload=", "onclick=",
        "eval(", "document.cookie", "<iframe", "<img", "alert("];
    for p in &patterns {
        if lower.contains(p) { score += 1.0; }
    }
    (score / patterns.len() as f64 * 100.0).min(100.0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_detect_path_traversal(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len == 0 { return 0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    if s.contains("..") || s.contains("%2e%2e") || s.contains("%2E%2E") { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_detect_command_injection(ptr: *const u8, len: usize) -> f64 {
    if ptr.is_null() || len == 0 { return 0.0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    let lower = s.to_lowercase();
    let mut score = 0.0;
    let patterns = ["|", "&&", "`", "$(", ";", ">", "<", "rm ", "cat ", "wget ", "curl "];
    for p in &patterns {
        if lower.contains(p) { score += 1.0; }
    }
    (score / patterns.len() as f64 * 100.0).min(100.0)
}

// --- Password Strength ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_password_strength(ptr: *const u8, len: usize) -> f64 {
    if ptr.is_null() || len == 0 { return 0.0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    let mut score: f64 = 0.0;
    if s.len() >= 8 { score += 20.0; }
    if s.len() >= 12 { score += 10.0; }
    if s.len() >= 16 { score += 10.0; }
    if s.chars().any(|c| c.is_lowercase()) { score += 10.0; }
    if s.chars().any(|c| c.is_uppercase()) { score += 10.0; }
    if s.chars().any(|c| c.is_ascii_digit()) { score += 10.0; }
    if s.chars().any(|c| !c.is_alphanumeric()) { score += 15.0; }
    let unique: std::collections::HashSet<char> = s.chars().collect();
    let diversity = unique.len() as f64 / s.len() as f64;
    score += diversity * 15.0;
    score.min(100.0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_password_entropy(ptr: *const u8, len: usize) -> f64 {
    if ptr.is_null() || len == 0 { return 0.0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    let mut charset = 0u32;
    if s.chars().any(|c| c.is_ascii_lowercase()) { charset += 26; }
    if s.chars().any(|c| c.is_ascii_uppercase()) { charset += 26; }
    if s.chars().any(|c| c.is_ascii_digit()) { charset += 10; }
    if s.chars().any(|c| !c.is_alphanumeric()) { charset += 32; }
    if charset == 0 { return 0.0; }
    s.len() as f64 * (charset as f64).log2()
}

// --- Resource Limits ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_check_memory_quota(used_bytes: u64, max_bytes: u64) -> i32 {
    if used_bytes <= max_bytes { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_check_time_budget(elapsed_ms: f64, budget_ms: f64) -> i32 {
    if elapsed_ms <= budget_ms { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_check_recursion_depth(depth: u32, max_depth: u32) -> i32 {
    if depth <= max_depth { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_resource_utilization(used: f64, total: f64) -> f64 {
    if total <= 0.0 { return 0.0; }
    (used / total * 100.0).min(100.0)
}

// --- Content Safety ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_check_deny_list(input: *const u8, ilen: usize, deny: *const u8, dlen: usize) -> i32 {
    if input.is_null() || deny.is_null() || ilen == 0 || dlen == 0 { return 0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(input, ilen)) };
    let d = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(deny, dlen)) };
    let lower = s.to_lowercase();
    for word in d.split(',') {
        let w = word.trim().to_lowercase();
        if !w.is_empty() && lower.contains(&w) { return 1; }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_code_safety_score(ptr: *const u8, len: usize) -> f64 {
    if ptr.is_null() || len == 0 { return 100.0; }
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
    let lower = s.to_lowercase();
    let mut score: f64 = 100.0;
    let dangers = [("unsafe", 10.0), ("exec", 15.0), ("eval", 15.0), ("system(", 20.0),
        ("process", 5.0), ("raw_pointer", 10.0), ("transmute", 15.0), ("asm!", 20.0)];
    for (pat, penalty) in &dangers {
        if lower.contains(pat) { score -= penalty; }
    }
    score.max(0.0)
}

// --- Audit & Hash ---

fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_audit_hash(ptr: *const u8, len: usize) -> u64 {
    if ptr.is_null() || len == 0 { return 0; }
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    fnv1a(data)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_hash_chain(ptr: *const u8, len: usize, prev_hash: u64) -> u64 {
    if ptr.is_null() || len == 0 { return prev_hash; }
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    let combined: Vec<u8> = prev_hash.to_le_bytes().iter().chain(data.iter()).copied().collect();
    fnv1a(&combined)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_verify_chain(hashes: *const u64, entries: *const *const u8,
    lengths: *const usize, n: usize) -> i32 {
    if hashes.is_null() || entries.is_null() || lengths.is_null() || n == 0 { return 0; }
    let h = unsafe { std::slice::from_raw_parts(hashes, n) };
    let e = unsafe { std::slice::from_raw_parts(entries, n) };
    let l = unsafe { std::slice::from_raw_parts(lengths, n) };
    let first_data = unsafe { std::slice::from_raw_parts(e[0], l[0]) };
    if h[0] != fnv1a(first_data) { return 0; }
    for i in 1..n {
        let data = unsafe { std::slice::from_raw_parts(e[i], l[i]) };
        let combined: Vec<u8> = h[i-1].to_le_bytes().iter().chain(data.iter()).copied().collect();
        if h[i] != fnv1a(&combined) { return 0; }
    }
    1
}

// --- Rate Limiting ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_token_bucket_check(tokens: f64, max_tokens: f64, refill_rate: f64,
    elapsed_secs: f64, cost: f64) -> f64 {
    let refilled = (tokens + refill_rate * elapsed_secs).min(max_tokens);
    if refilled >= cost { refilled - cost } else { -1.0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sliding_window_check(timestamps: *const f64, n: usize,
    now: f64, window_secs: f64, max_requests: usize) -> i32 {
    if timestamps.is_null() || n == 0 { return 1; }
    let ts = unsafe { std::slice::from_raw_parts(timestamps, n) };
    let count = ts.iter().filter(|&&t| t >= now - window_secs).count();
    if count < max_requests { 1 } else { 0 }
}

// --- Sandbox Capabilities ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sandbox_grant(current: u64, capability: u64) -> u64 { current | capability }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sandbox_revoke(current: u64, capability: u64) -> u64 { current & !capability }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sandbox_check(current: u64, required: u64) -> i32 {
    if current & required == required { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sandbox_count(caps: u64) -> u32 { caps.count_ones() }

// --- HTML Sanitization ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_html_escape(ptr: *const u8, len: usize, out: *mut u8, out_cap: usize) -> usize {
    if ptr.is_null() || out.is_null() || len == 0 { return 0; }
    let s = unsafe { std::slice::from_raw_parts(ptr, len) };
    let buf = unsafe { std::slice::from_raw_parts_mut(out, out_cap) };
    let mut pos = 0;
    for &b in s {
        let replacement: &[u8] = match b {
            b'<' => b"&lt;",
            b'>' => b"&gt;",
            b'&' => b"&amp;",
            b'"' => b"&quot;",
            b'\'' => b"&#x27;",
            _ => std::slice::from_ref(&b),
        };
        if pos + replacement.len() > out_cap { break; }
        buf[pos..pos+replacement.len()].copy_from_slice(replacement);
        pos += replacement.len();
    }
    pos
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert_eq!(unsafe { vitalis_validate_email(b"user@example.com".as_ptr(), 16) }, 1);
        assert_eq!(unsafe { vitalis_validate_email(b"invalid".as_ptr(), 7) }, 0);
    }

    #[test]
    fn test_validate_ipv4() {
        assert_eq!(unsafe { vitalis_validate_ipv4(b"192.168.1.1".as_ptr(), 11) }, 1);
        assert_eq!(unsafe { vitalis_validate_ipv4(b"256.1.1.1".as_ptr(), 9) }, 0);
    }

    #[test]
    fn test_validate_url() {
        assert_eq!(unsafe { vitalis_validate_url(b"https://example.com".as_ptr(), 19) }, 1);
        assert_eq!(unsafe { vitalis_validate_url(b"ftp://bad".as_ptr(), 9) }, 0);
    }

    #[test]
    fn test_sqli_detection() {
        let s = b"SELECT * FROM users WHERE id=1 OR 1=1 --";
        let score = unsafe { vitalis_detect_sqli(s.as_ptr(), s.len()) };
        assert!(score > 0.0);
    }

    #[test]
    fn test_xss_detection() {
        let s = b"<script>alert(1)</script>";
        let score = unsafe { vitalis_detect_xss(s.as_ptr(), s.len()) };
        assert!(score > 0.0);
    }

    #[test]
    fn test_path_traversal() {
        assert_eq!(unsafe { vitalis_detect_path_traversal(b"../../etc/passwd".as_ptr(), 16) }, 1);
        assert_eq!(unsafe { vitalis_detect_path_traversal(b"safe/path".as_ptr(), 9) }, 0);
    }

    #[test]
    fn test_password_strength() {
        let weak = b"12345";
        let strong = b"C0mpl3x!P@ssw0rd#2024";
        let w = unsafe { vitalis_password_strength(weak.as_ptr(), weak.len()) };
        let s = unsafe { vitalis_password_strength(strong.as_ptr(), strong.len()) };
        assert!(s > w);
    }

    #[test]
    fn test_password_entropy() {
        let pw = b"Abcdef1!";
        let e = unsafe { vitalis_password_entropy(pw.as_ptr(), pw.len()) };
        assert!(e > 40.0);
    }

    #[test]
    fn test_memory_quota() {
        assert_eq!(unsafe { vitalis_check_memory_quota(100, 200) }, 1);
        assert_eq!(unsafe { vitalis_check_memory_quota(300, 200) }, 0);
    }

    #[test]
    fn test_code_safety() {
        let safe_code = b"fn hello() { println!(42); }";
        let risky = b"unsafe { eval(exec(system(cmd))); }";
        let ss = unsafe { vitalis_code_safety_score(safe_code.as_ptr(), safe_code.len()) };
        let rs = unsafe { vitalis_code_safety_score(risky.as_ptr(), risky.len()) };
        assert!(ss > rs);
    }

    #[test]
    fn test_audit_hash() {
        let d1 = b"hello";
        let d2 = b"world";
        let h1 = unsafe { vitalis_audit_hash(d1.as_ptr(), d1.len()) };
        let h2 = unsafe { vitalis_audit_hash(d2.as_ptr(), d2.len()) };
        assert_ne!(h1, h2);
        assert_ne!(h1, 0);
    }

    #[test]
    fn test_token_bucket() {
        let r = unsafe { vitalis_token_bucket_check(10.0, 100.0, 5.0, 2.0, 1.0) };
        assert!((r - 19.0).abs() < 1e-10);
    }

    #[test]
    fn test_sandbox_caps() {
        let caps = unsafe { vitalis_sandbox_grant(0, 0b0101) };
        assert_eq!(unsafe { vitalis_sandbox_check(caps, 0b0001) }, 1);
        assert_eq!(unsafe { vitalis_sandbox_check(caps, 0b1000) }, 0);
        assert_eq!(unsafe { vitalis_sandbox_count(caps) }, 2);
        let caps2 = unsafe { vitalis_sandbox_revoke(caps, 0b0001) };
        assert_eq!(unsafe { vitalis_sandbox_check(caps2, 0b0001) }, 0);
    }

    #[test]
    fn test_html_escape() {
        let input = b"<b>Hello</b>";
        let mut out = [0u8; 256];
        let n = unsafe { vitalis_html_escape(input.as_ptr(), input.len(), out.as_mut_ptr(), 256) };
        let result = std::str::from_utf8(&out[..n]).unwrap();
        assert!(result.contains("&lt;"));
        assert!(result.contains("&gt;"));
    }

    #[test]
    fn test_sliding_window() {
        let ts = [1.0, 2.0, 3.0, 4.0, 5.0];
        let r = unsafe { vitalis_sliding_window_check(ts.as_ptr(), 5, 6.0, 3.0, 10) };
        assert_eq!(r, 1);
    }

    #[test]
    fn test_deny_list() {
        let input = b"this contains badword here";
        let deny = b"badword,evil";
        assert_eq!(unsafe { vitalis_check_deny_list(input.as_ptr(), input.len(), deny.as_ptr(), deny.len()) }, 1);
    }
}
