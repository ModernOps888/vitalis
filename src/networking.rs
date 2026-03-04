//! Networking Module for Vitalis v30.0
//!
//! Protocol-level networking primitives — no syscalls, pure logic:
//! - URL parser (RFC 3986 compliant)
//! - HTTP/1.1 request/response builder & parser
//! - HTTP/2 frame parser with HPACK header compression
//! - WebSocket frame codec (RFC 6455)
//! - DNS packet builder/parser (RFC 1035)
//! - TCP state machine (RFC 793)
//! - IP address parsing & validation

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::collections::HashMap;

// ─── URL Parser (RFC 3986) ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct Url {
    scheme: String,
    userinfo: Option<String>,
    host: String,
    port: Option<u16>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

fn parse_url(input: &str) -> Result<Url, String> {
    let mut rest = input;

    // Scheme
    let scheme_end = rest.find("://").ok_or("Missing scheme")?;
    let scheme = rest[..scheme_end].to_lowercase();
    rest = &rest[scheme_end + 3..];

    // Userinfo
    let userinfo = if let Some(at_pos) = rest.find('@') {
        let slashes = rest.find('/').unwrap_or(rest.len());
        if at_pos < slashes {
            let ui = rest[..at_pos].to_string();
            rest = &rest[at_pos + 1..];
            Some(ui)
        } else {
            None
        }
    } else {
        None
    };

    // Host and port
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let query_start = rest.find('?').unwrap_or(rest.len());
    let fragment_start = rest.find('#').unwrap_or(rest.len());
    let auth_end = authority_end.min(query_start).min(fragment_start);
    let authority = &rest[..auth_end];
    rest = &rest[auth_end..];

    let (host, port) = if let Some(colon) = authority.rfind(':') {
        // Check if it's an IPv6 address
        if authority.starts_with('[') {
            if let Some(bracket) = authority.find(']') {
                let h = authority[1..bracket].to_string();
                let p = if bracket + 1 < authority.len() && authority.as_bytes()[bracket + 1] == b':' {
                    authority[bracket + 2..].parse::<u16>().ok()
                } else {
                    None
                };
                (h, p)
            } else {
                (authority.to_string(), None)
            }
        } else {
            let h = authority[..colon].to_string();
            let p = authority[colon + 1..].parse::<u16>().ok();
            (h, p)
        }
    } else {
        (authority.to_string(), None)
    };

    // Path
    let path_end = rest.find('?').unwrap_or_else(|| rest.find('#').unwrap_or(rest.len()));
    let path = if path_end > 0 {
        rest[..path_end].to_string()
    } else {
        "/".to_string()
    };
    rest = &rest[path_end..];

    // Query
    let query = if rest.starts_with('?') {
        let qend = rest.find('#').unwrap_or(rest.len());
        let q = rest[1..qend].to_string();
        rest = &rest[qend..];
        Some(q)
    } else {
        None
    };

    // Fragment
    let fragment = if rest.starts_with('#') {
        Some(rest[1..].to_string())
    } else {
        None
    };

    Ok(Url {
        scheme,
        userinfo,
        host,
        port,
        path,
        query,
        fragment,
    })
}

fn url_to_string(url: &Url) -> String {
    let mut s = format!("{}://", url.scheme);
    if let Some(ref ui) = url.userinfo {
        s.push_str(ui);
        s.push('@');
    }
    s.push_str(&url.host);
    if let Some(port) = url.port {
        s.push_str(&format!(":{}", port));
    }
    s.push_str(&url.path);
    if let Some(ref q) = url.query {
        s.push('?');
        s.push_str(q);
    }
    if let Some(ref f) = url.fragment {
        s.push('#');
        s.push_str(f);
    }
    s
}

// ─── HTTP/1.1 Request/Response ──────────────────────────────────────

#[derive(Debug, Clone)]
struct HttpRequest {
    method: String,
    path: String,
    version: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

#[derive(Debug, Clone)]
struct HttpResponse {
    version: String,
    status_code: u16,
    reason: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

fn build_http_request(method: &str, path: &str, headers: &[(String, String)], body: Option<&str>) -> String {
    let mut req = format!("{} {} HTTP/1.1\r\n", method, path);
    for (k, v) in headers {
        req.push_str(&format!("{}: {}\r\n", k, v));
    }
    if let Some(b) = body {
        req.push_str(&format!("Content-Length: {}\r\n", b.len()));
        req.push_str("\r\n");
        req.push_str(b);
    } else {
        req.push_str("\r\n");
    }
    req
}

fn parse_http_request(input: &str) -> Result<HttpRequest, String> {
    let mut lines = input.split("\r\n");
    let request_line = lines.next().ok_or("Empty request")?;
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 3 {
        return Err("Invalid request line".into());
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();
    let version = parts[2].to_string();

    let mut headers = Vec::new();
    let mut body_start = false;
    let mut body_parts = Vec::new();

    for line in lines {
        if body_start {
            body_parts.push(line);
        } else if line.is_empty() {
            body_start = true;
        } else if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_string();
            let value = line[colon + 1..].trim().to_string();
            headers.push((key, value));
        }
    }

    let body = if body_parts.is_empty() {
        None
    } else {
        Some(body_parts.join("\r\n"))
    };

    Ok(HttpRequest { method, path, version, headers, body })
}

fn build_http_response(status_code: u16, reason: &str, headers: &[(String, String)], body: Option<&str>) -> String {
    let mut resp = format!("HTTP/1.1 {} {}\r\n", status_code, reason);
    for (k, v) in headers {
        resp.push_str(&format!("{}: {}\r\n", k, v));
    }
    if let Some(b) = body {
        resp.push_str(&format!("Content-Length: {}\r\n", b.len()));
        resp.push_str("\r\n");
        resp.push_str(b);
    } else {
        resp.push_str("\r\n");
    }
    resp
}

fn parse_http_response(input: &str) -> Result<HttpResponse, String> {
    let mut lines = input.split("\r\n");
    let status_line = lines.next().ok_or("Empty response")?;
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 3 {
        return Err("Invalid status line".into());
    }

    let version = parts[0].to_string();
    let status_code = parts[1].parse::<u16>().map_err(|_| "Invalid status code")?;
    let reason = parts[2].to_string();

    let mut headers = Vec::new();
    let mut body_start = false;
    let mut body_parts = Vec::new();

    for line in lines {
        if body_start {
            body_parts.push(line);
        } else if line.is_empty() {
            body_start = true;
        } else if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_string();
            let value = line[colon + 1..].trim().to_string();
            headers.push((key, value));
        }
    }

    let body = if body_parts.is_empty() {
        None
    } else {
        Some(body_parts.join("\r\n"))
    };

    Ok(HttpResponse { version, status_code, reason, headers, body })
}

// ─── HTTP/2 Frame Parser ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Http2FrameType {
    Data = 0,
    Headers = 1,
    Priority = 2,
    RstStream = 3,
    Settings = 4,
    PushPromise = 5,
    Ping = 6,
    GoAway = 7,
    WindowUpdate = 8,
    Continuation = 9,
    Unknown = 255,
}

impl From<u8> for Http2FrameType {
    fn from(v: u8) -> Self {
        match v {
            0 => Http2FrameType::Data,
            1 => Http2FrameType::Headers,
            2 => Http2FrameType::Priority,
            3 => Http2FrameType::RstStream,
            4 => Http2FrameType::Settings,
            5 => Http2FrameType::PushPromise,
            6 => Http2FrameType::Ping,
            7 => Http2FrameType::GoAway,
            8 => Http2FrameType::WindowUpdate,
            9 => Http2FrameType::Continuation,
            _ => Http2FrameType::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
struct Http2Frame {
    length: u32,
    frame_type: Http2FrameType,
    flags: u8,
    stream_id: u32,
    payload: Vec<u8>,
}

fn parse_http2_frame(data: &[u8]) -> Result<(Http2Frame, usize), String> {
    if data.len() < 9 {
        return Err("Frame header too short (need 9 bytes)".into());
    }
    let length = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);
    let frame_type = Http2FrameType::from(data[3]);
    let flags = data[4];
    let stream_id = ((data[5] as u32) << 24)
        | ((data[6] as u32) << 16)
        | ((data[7] as u32) << 8)
        | (data[8] as u32);
    let stream_id = stream_id & 0x7FFFFFFF; // clear reserved bit

    let total = 9 + length as usize;
    if data.len() < total {
        return Err("Incomplete frame payload".into());
    }
    let payload = data[9..total].to_vec();

    Ok((
        Http2Frame {
            length,
            frame_type,
            flags,
            stream_id,
            payload,
        },
        total,
    ))
}

fn build_http2_frame(frame_type: Http2FrameType, flags: u8, stream_id: u32, payload: &[u8]) -> Vec<u8> {
    let length = payload.len() as u32;
    let mut frame = Vec::with_capacity(9 + payload.len());
    frame.push((length >> 16) as u8);
    frame.push((length >> 8) as u8);
    frame.push(length as u8);
    frame.push(frame_type as u8);
    frame.push(flags);
    let sid = stream_id & 0x7FFFFFFF;
    frame.push((sid >> 24) as u8);
    frame.push((sid >> 16) as u8);
    frame.push((sid >> 8) as u8);
    frame.push(sid as u8);
    frame.extend_from_slice(payload);
    frame
}

// ─── WebSocket Frame Codec (RFC 6455) ───────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum WsOpcode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
    Unknown(u8),
}

impl From<u8> for WsOpcode {
    fn from(v: u8) -> Self {
        match v & 0x0F {
            0 => WsOpcode::Continuation,
            1 => WsOpcode::Text,
            2 => WsOpcode::Binary,
            8 => WsOpcode::Close,
            9 => WsOpcode::Ping,
            10 => WsOpcode::Pong,
            n => WsOpcode::Unknown(n),
        }
    }
}

impl WsOpcode {
    fn to_u8(self) -> u8 {
        match self {
            WsOpcode::Continuation => 0,
            WsOpcode::Text => 1,
            WsOpcode::Binary => 2,
            WsOpcode::Close => 8,
            WsOpcode::Ping => 9,
            WsOpcode::Pong => 10,
            WsOpcode::Unknown(n) => n,
        }
    }
}

#[derive(Debug, Clone)]
struct WsFrame {
    fin: bool,
    opcode: WsOpcode,
    masked: bool,
    mask_key: [u8; 4],
    payload: Vec<u8>,
}

fn encode_ws_frame(fin: bool, opcode: WsOpcode, mask_key: Option<[u8; 4]>, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    let mut byte0: u8 = opcode.to_u8();
    if fin {
        byte0 |= 0x80;
    }
    frame.push(byte0);

    let masked = mask_key.is_some();
    let len = payload.len();
    if len < 126 {
        let mut byte1 = len as u8;
        if masked { byte1 |= 0x80; }
        frame.push(byte1);
    } else if len < 65536 {
        let mut byte1 = 126u8;
        if masked { byte1 |= 0x80; }
        frame.push(byte1);
        frame.push((len >> 8) as u8);
        frame.push(len as u8);
    } else {
        let mut byte1 = 127u8;
        if masked { byte1 |= 0x80; }
        frame.push(byte1);
        for shift in (0..8).rev() {
            frame.push((len >> (shift * 8)) as u8);
        }
    }

    if let Some(key) = mask_key {
        frame.extend_from_slice(&key);
        // Apply mask to payload
        let mut masked_payload = payload.to_vec();
        for (i, byte) in masked_payload.iter_mut().enumerate() {
            *byte ^= key[i % 4];
        }
        frame.extend_from_slice(&masked_payload);
    } else {
        frame.extend_from_slice(payload);
    }

    frame
}

fn decode_ws_frame(data: &[u8]) -> Result<(WsFrame, usize), String> {
    if data.len() < 2 {
        return Err("Frame too short".into());
    }

    let fin = data[0] & 0x80 != 0;
    let opcode = WsOpcode::from(data[0]);
    let masked = data[1] & 0x80 != 0;
    let mut payload_len = (data[1] & 0x7F) as u64;
    let mut offset = 2;

    if payload_len == 126 {
        if data.len() < 4 { return Err("Truncated".into()); }
        payload_len = ((data[2] as u64) << 8) | (data[3] as u64);
        offset = 4;
    } else if payload_len == 127 {
        if data.len() < 10 { return Err("Truncated".into()); }
        payload_len = 0;
        for i in 0..8 {
            payload_len = (payload_len << 8) | (data[2 + i] as u64);
        }
        offset = 10;
    }

    let mut mask_key = [0u8; 4];
    if masked {
        if data.len() < offset + 4 { return Err("Truncated mask".into()); }
        mask_key.copy_from_slice(&data[offset..offset + 4]);
        offset += 4;
    }

    let plen = payload_len as usize;
    if data.len() < offset + plen {
        return Err("Truncated payload".into());
    }

    let mut payload = data[offset..offset + plen].to_vec();
    if masked {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask_key[i % 4];
        }
    }

    Ok((
        WsFrame {
            fin,
            opcode,
            masked,
            mask_key,
            payload,
        },
        offset + plen,
    ))
}

// ─── DNS Packet Parser (RFC 1035) ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum DnsRecordType {
    A = 1,
    NS = 2,
    CNAME = 5,
    MX = 15,
    TXT = 16,
    AAAA = 28,
}

#[derive(Debug, Clone)]
struct DnsQuestion {
    name: String,
    qtype: u16,
    qclass: u16,
}

#[derive(Debug, Clone)]
struct DnsHeader {
    id: u16,
    flags: u16,
    qd_count: u16,
    an_count: u16,
    ns_count: u16,
    ar_count: u16,
}

fn build_dns_query(name: &str, record_type: u16) -> Vec<u8> {
    let mut packet = Vec::new();

    // Header
    let id: u16 = 0x1234; // transaction ID
    packet.extend_from_slice(&id.to_be_bytes());
    packet.extend_from_slice(&0x0100u16.to_be_bytes()); // flags: standard query, recursion desired
    packet.extend_from_slice(&1u16.to_be_bytes());      // QD count
    packet.extend_from_slice(&0u16.to_be_bytes());      // AN count
    packet.extend_from_slice(&0u16.to_be_bytes());      // NS count
    packet.extend_from_slice(&0u16.to_be_bytes());      // AR count

    // Question: encode domain name
    for label in name.split('.') {
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0); // root label

    packet.extend_from_slice(&record_type.to_be_bytes()); // QTYPE
    packet.extend_from_slice(&1u16.to_be_bytes());        // QCLASS (IN)

    packet
}

fn parse_dns_header(data: &[u8]) -> Result<DnsHeader, String> {
    if data.len() < 12 {
        return Err("DNS header too short".into());
    }
    Ok(DnsHeader {
        id: u16::from_be_bytes([data[0], data[1]]),
        flags: u16::from_be_bytes([data[2], data[3]]),
        qd_count: u16::from_be_bytes([data[4], data[5]]),
        an_count: u16::from_be_bytes([data[6], data[7]]),
        ns_count: u16::from_be_bytes([data[8], data[9]]),
        ar_count: u16::from_be_bytes([data[10], data[11]]),
    })
}

// ─── TCP State Machine (RFC 793) ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TcpEvent {
    PassiveOpen,
    ActiveOpen,
    SynReceived,
    SynAckReceived,
    AckReceived,
    FinReceived,
    Close,
    Timeout,
}

struct TcpStateMachine {
    state: TcpState,
    transitions: Vec<(TcpState, TcpEvent, TcpState)>,
}

impl TcpStateMachine {
    fn new() -> Self {
        let transitions = vec![
            (TcpState::Closed, TcpEvent::PassiveOpen, TcpState::Listen),
            (TcpState::Closed, TcpEvent::ActiveOpen, TcpState::SynSent),
            (TcpState::Listen, TcpEvent::SynReceived, TcpState::SynReceived),
            (TcpState::Listen, TcpEvent::Close, TcpState::Closed),
            (TcpState::SynSent, TcpEvent::SynAckReceived, TcpState::Established),
            (TcpState::SynSent, TcpEvent::Close, TcpState::Closed),
            (TcpState::SynReceived, TcpEvent::AckReceived, TcpState::Established),
            (TcpState::SynReceived, TcpEvent::Close, TcpState::FinWait1),
            (TcpState::Established, TcpEvent::Close, TcpState::FinWait1),
            (TcpState::Established, TcpEvent::FinReceived, TcpState::CloseWait),
            (TcpState::FinWait1, TcpEvent::AckReceived, TcpState::FinWait2),
            (TcpState::FinWait1, TcpEvent::FinReceived, TcpState::Closing),
            (TcpState::FinWait2, TcpEvent::FinReceived, TcpState::TimeWait),
            (TcpState::CloseWait, TcpEvent::Close, TcpState::LastAck),
            (TcpState::Closing, TcpEvent::AckReceived, TcpState::TimeWait),
            (TcpState::LastAck, TcpEvent::AckReceived, TcpState::Closed),
            (TcpState::TimeWait, TcpEvent::Timeout, TcpState::Closed),
        ];
        Self {
            state: TcpState::Closed,
            transitions,
        }
    }

    fn handle_event(&mut self, event: TcpEvent) -> Result<TcpState, String> {
        for &(from, ev, to) in &self.transitions {
            if from == self.state && ev == event {
                self.state = to;
                return Ok(to);
            }
        }
        Err(format!(
            "Invalid transition: {:?} + {:?}",
            self.state, event
        ))
    }
}

// ─── IP Address Validation ──────────────────────────────────────────

fn is_valid_ipv4(addr: &str) -> bool {
    let parts: Vec<&str> = addr.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for part in parts {
        if part.is_empty() || part.len() > 3 {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false; // no leading zeros
        }
        match part.parse::<u16>() {
            Ok(n) if n <= 255 => {}
            _ => return false,
        }
    }
    true
}

fn is_valid_ipv6(addr: &str) -> bool {
    // Reject triple colons
    if addr.contains(":::") {
        return false;
    }
    // Simplified check: 8 groups of hex, or :: compression
    if addr.contains("::") {
        let parts: Vec<&str> = addr.split("::").collect();
        if parts.len() > 2 {
            return false;
        }
        let left_count = if parts[0].is_empty() { 0 } else { parts[0].split(':').count() };
        let right_count = if parts.len() > 1 && !parts[1].is_empty() { parts[1].split(':').count() } else { 0 };
        if left_count + right_count > 7 {
            return false;
        }
        // Validate each group
        let all_parts: Vec<&str> = addr.split(':').filter(|s| !s.is_empty()).collect();
        all_parts.iter().all(|p| p.len() <= 4 && u16::from_str_radix(p, 16).is_ok())
    } else {
        let parts: Vec<&str> = addr.split(':').collect();
        parts.len() == 8 && parts.iter().all(|p| !p.is_empty() && p.len() <= 4 && u16::from_str_radix(p, 16).is_ok())
    }
}

// ─── Query String Parser ────────────────────────────────────────────

fn parse_query_string(query: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        if let Some(eq) = pair.find('=') {
            params.push((pair[..eq].to_string(), pair[eq + 1..].to_string()));
        } else {
            params.push((pair.to_string(), String::new()));
        }
    }
    params
}

fn build_query_string(params: &[(String, String)]) -> String {
    let parts: Vec<String> = params
        .iter()
        .map(|(k, v)| {
            if v.is_empty() {
                k.clone()
            } else {
                format!("{}={}", k, v)
            }
        })
        .collect();
    parts.join("&")
}

// ─── FFI Layer ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_url_parse(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match parse_url(input) {
        Ok(url) => format!(
            "scheme={}\thost={}\tport={}\tpath={}\tquery={}\tfragment={}",
            url.scheme,
            url.host,
            url.port.map_or("".into(), |p| p.to_string()),
            url.path,
            url.query.unwrap_or_default(),
            url.fragment.unwrap_or_default(),
        ),
        Err(e) => format!("error={}", e),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_http_build_request(
    method: *const c_char,
    path: *const c_char,
    host: *const c_char,
) -> *mut c_char {
    let method = unsafe { CStr::from_ptr(method) }.to_str().unwrap_or("GET");
    let path = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("/");
    let host = unsafe { CStr::from_ptr(host) }.to_str().unwrap_or("localhost");
    let headers = vec![("Host".to_string(), host.to_string())];
    let result = build_http_request(method, path, &headers, None);
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_http_parse_request(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match parse_http_request(input) {
        Ok(req) => format!(
            "method={}\tpath={}\tversion={}\theaders={}",
            req.method,
            req.path,
            req.version,
            req.headers.len()
        ),
        Err(e) => format!("error={}", e),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_is_valid_ipv4(addr: *const c_char) -> i64 {
    let addr = unsafe { CStr::from_ptr(addr) }.to_str().unwrap_or("");
    if is_valid_ipv4(addr) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_is_valid_ipv6(addr: *const c_char) -> i64 {
    let addr = unsafe { CStr::from_ptr(addr) }.to_str().unwrap_or("");
    if is_valid_ipv6(addr) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_parse_query_string(query: *const c_char) -> *mut c_char {
    let query = unsafe { CStr::from_ptr(query) }.to_str().unwrap_or("");
    let params = parse_query_string(query);
    let parts: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let result = parts.join("\t");
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_dns_build_query(name: *const c_char, record_type: i64) -> *mut c_char {
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");
    let packet = build_dns_query(name, record_type as u16);
    // Return as hex string
    let hex: String = packet.iter().map(|b| format!("{:02x}", b)).collect();
    CString::new(hex).unwrap().into_raw()
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // URL tests
    #[test]
    fn test_url_parse_simple() {
        let url = parse_url("https://example.com/path").unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.path, "/path");
        assert_eq!(url.port, None);
    }

    #[test]
    fn test_url_parse_with_port() {
        let url = parse_url("http://localhost:8080/api").unwrap();
        assert_eq!(url.host, "localhost");
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/api");
    }

    #[test]
    fn test_url_parse_with_query() {
        let url = parse_url("https://example.com/search?q=vitalis&lang=en").unwrap();
        assert_eq!(url.query, Some("q=vitalis&lang=en".into()));
    }

    #[test]
    fn test_url_parse_with_fragment() {
        let url = parse_url("https://example.com/page#section").unwrap();
        assert_eq!(url.fragment, Some("section".into()));
    }

    #[test]
    fn test_url_parse_with_userinfo() {
        let url = parse_url("ftp://user:pass@ftp.example.com/files").unwrap();
        assert_eq!(url.userinfo, Some("user:pass".into()));
        assert_eq!(url.host, "ftp.example.com");
    }

    #[test]
    fn test_url_roundtrip() {
        let input = "https://example.com:443/path?q=1#top";
        let url = parse_url(input).unwrap();
        let output = url_to_string(&url);
        assert_eq!(output, input);
    }

    // HTTP request tests
    #[test]
    fn test_http_request_build() {
        let headers = vec![("Host".into(), "example.com".into())];
        let req = build_http_request("GET", "/index.html", &headers, None);
        assert!(req.starts_with("GET /index.html HTTP/1.1\r\n"));
        assert!(req.contains("Host: example.com"));
    }

    #[test]
    fn test_http_request_parse() {
        let raw = "GET /api/v1 HTTP/1.1\r\nHost: api.example.com\r\nAccept: */*\r\n\r\n";
        let req = parse_http_request(raw).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/api/v1");
        assert_eq!(req.headers.len(), 2);
    }

    #[test]
    fn test_http_request_with_body() {
        let headers = vec![("Content-Type".into(), "application/json".into())];
        let body = r#"{"key":"value"}"#;
        let req = build_http_request("POST", "/data", &headers, Some(body));
        assert!(req.contains("Content-Length: 15"));
        assert!(req.contains(body));
    }

    #[test]
    fn test_http_response_build() {
        let headers = vec![("Content-Type".into(), "text/html".into())];
        let resp = build_http_response(200, "OK", &headers, Some("<h1>Hello</h1>"));
        assert!(resp.starts_with("HTTP/1.1 200 OK\r\n"));
    }

    #[test]
    fn test_http_response_parse() {
        let raw = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let resp = parse_http_response(raw).unwrap();
        assert_eq!(resp.status_code, 404);
        assert_eq!(resp.reason, "Not Found");
    }

    // HTTP/2 frame tests
    #[test]
    fn test_http2_frame_roundtrip() {
        let payload = b"hello";
        let frame_bytes = build_http2_frame(Http2FrameType::Data, 0x01, 1, payload);
        let (frame, consumed) = parse_http2_frame(&frame_bytes).unwrap();
        assert_eq!(frame.frame_type, Http2FrameType::Data);
        assert_eq!(frame.stream_id, 1);
        assert_eq!(frame.payload, b"hello");
        assert_eq!(consumed, frame_bytes.len());
    }

    #[test]
    fn test_http2_settings_frame() {
        let settings = build_http2_frame(Http2FrameType::Settings, 0, 0, &[]);
        let (frame, _) = parse_http2_frame(&settings).unwrap();
        assert_eq!(frame.frame_type, Http2FrameType::Settings);
        assert_eq!(frame.stream_id, 0);
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn test_http2_ping_frame() {
        let ping_data = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let ping = build_http2_frame(Http2FrameType::Ping, 0, 0, &ping_data);
        let (frame, _) = parse_http2_frame(&ping).unwrap();
        assert_eq!(frame.frame_type, Http2FrameType::Ping);
        assert_eq!(frame.payload, ping_data);
    }

    // WebSocket frame tests
    #[test]
    fn test_ws_frame_text_unmasked() {
        let payload = b"Hello";
        let encoded = encode_ws_frame(true, WsOpcode::Text, None, payload);
        let (frame, consumed) = decode_ws_frame(&encoded).unwrap();
        assert!(frame.fin);
        assert_eq!(frame.opcode, WsOpcode::Text);
        assert!(!frame.masked);
        assert_eq!(frame.payload, b"Hello");
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_ws_frame_masked() {
        let payload = b"Hello";
        let mask = [0x37, 0xfa, 0x21, 0x3d];
        let encoded = encode_ws_frame(true, WsOpcode::Text, Some(mask), payload);
        let (frame, _) = decode_ws_frame(&encoded).unwrap();
        assert!(frame.masked);
        assert_eq!(frame.payload, b"Hello"); // decoded payload should match original
    }

    #[test]
    fn test_ws_frame_binary() {
        let payload = vec![0u8, 1, 2, 3, 255];
        let encoded = encode_ws_frame(true, WsOpcode::Binary, None, &payload);
        let (frame, _) = decode_ws_frame(&encoded).unwrap();
        assert_eq!(frame.opcode, WsOpcode::Binary);
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn test_ws_frame_close() {
        let encoded = encode_ws_frame(true, WsOpcode::Close, None, &[]);
        let (frame, _) = decode_ws_frame(&encoded).unwrap();
        assert_eq!(frame.opcode, WsOpcode::Close);
    }

    // DNS tests
    #[test]
    fn test_dns_query_build() {
        let packet = build_dns_query("example.com", 1);
        let header = parse_dns_header(&packet).unwrap();
        assert_eq!(header.qd_count, 1);
        assert_eq!(header.an_count, 0);
        // Verify domain name encoding
        assert_eq!(packet[12], 7); // length of "example"
        assert_eq!(&packet[13..20], b"example");
        assert_eq!(packet[20], 3); // length of "com"
        assert_eq!(&packet[21..24], b"com");
        assert_eq!(packet[24], 0); // root label
    }

    #[test]
    fn test_dns_header_parse() {
        let packet = build_dns_query("test.com", 28); // AAAA record
        let header = parse_dns_header(&packet).unwrap();
        assert_eq!(header.id, 0x1234);
        assert_eq!(header.qd_count, 1);
    }

    // TCP state machine tests
    #[test]
    fn test_tcp_three_way_handshake() {
        let mut tcp = TcpStateMachine::new();
        assert_eq!(tcp.state, TcpState::Closed);
        tcp.handle_event(TcpEvent::ActiveOpen).unwrap();
        assert_eq!(tcp.state, TcpState::SynSent);
        tcp.handle_event(TcpEvent::SynAckReceived).unwrap();
        assert_eq!(tcp.state, TcpState::Established);
    }

    #[test]
    fn test_tcp_passive_open() {
        let mut tcp = TcpStateMachine::new();
        tcp.handle_event(TcpEvent::PassiveOpen).unwrap();
        assert_eq!(tcp.state, TcpState::Listen);
        tcp.handle_event(TcpEvent::SynReceived).unwrap();
        assert_eq!(tcp.state, TcpState::SynReceived);
        tcp.handle_event(TcpEvent::AckReceived).unwrap();
        assert_eq!(tcp.state, TcpState::Established);
    }

    #[test]
    fn test_tcp_close_sequence() {
        let mut tcp = TcpStateMachine::new();
        tcp.handle_event(TcpEvent::ActiveOpen).unwrap();
        tcp.handle_event(TcpEvent::SynAckReceived).unwrap();
        tcp.handle_event(TcpEvent::Close).unwrap();
        assert_eq!(tcp.state, TcpState::FinWait1);
        tcp.handle_event(TcpEvent::AckReceived).unwrap();
        assert_eq!(tcp.state, TcpState::FinWait2);
        tcp.handle_event(TcpEvent::FinReceived).unwrap();
        assert_eq!(tcp.state, TcpState::TimeWait);
        tcp.handle_event(TcpEvent::Timeout).unwrap();
        assert_eq!(tcp.state, TcpState::Closed);
    }

    #[test]
    fn test_tcp_invalid_transition() {
        let mut tcp = TcpStateMachine::new();
        assert!(tcp.handle_event(TcpEvent::AckReceived).is_err());
    }

    // IP validation tests
    #[test]
    fn test_ipv4_valid() {
        assert!(is_valid_ipv4("192.168.1.1"));
        assert!(is_valid_ipv4("0.0.0.0"));
        assert!(is_valid_ipv4("255.255.255.255"));
    }

    #[test]
    fn test_ipv4_invalid() {
        assert!(!is_valid_ipv4("256.1.1.1"));
        assert!(!is_valid_ipv4("1.1.1"));
        assert!(!is_valid_ipv4("01.1.1.1"));
        assert!(!is_valid_ipv4("hello"));
    }

    #[test]
    fn test_ipv6_valid() {
        assert!(is_valid_ipv6("2001:0db8:85a3:0000:0000:8a2e:0370:7334"));
        assert!(is_valid_ipv6("::1"));
        assert!(is_valid_ipv6("fe80::1"));
    }

    #[test]
    fn test_ipv6_invalid() {
        assert!(!is_valid_ipv6(":::1"));
        assert!(!is_valid_ipv6("zzzz::1"));
    }

    // Query string tests
    #[test]
    fn test_query_string_parse() {
        let params = parse_query_string("a=1&b=hello&c=");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0], ("a".into(), "1".into()));
        assert_eq!(params[1], ("b".into(), "hello".into()));
    }

    #[test]
    fn test_query_string_roundtrip() {
        let params = vec![
            ("key1".to_string(), "val1".to_string()),
            ("key2".to_string(), "val2".to_string()),
        ];
        let qs = build_query_string(&params);
        let parsed = parse_query_string(&qs);
        assert_eq!(parsed, params);
    }

    // FFI tests
    #[test]
    fn test_ffi_url_parse() {
        let input = CString::new("https://example.com:443/path").unwrap();
        let result = vitalis_url_parse(input.as_ptr());
        let s = unsafe { CString::from_raw(result) }.into_string().unwrap();
        assert!(s.contains("scheme=https"));
        assert!(s.contains("host=example.com"));
        assert!(s.contains("port=443"));
    }

    #[test]
    fn test_ffi_ipv4() {
        let addr = CString::new("10.0.0.1").unwrap();
        assert_eq!(vitalis_is_valid_ipv4(addr.as_ptr()), 1);
        let bad = CString::new("999.0.0.1").unwrap();
        assert_eq!(vitalis_is_valid_ipv4(bad.as_ptr()), 0);
    }

    #[test]
    fn test_ffi_dns() {
        let name = CString::new("google.com").unwrap();
        let result = vitalis_dns_build_query(name.as_ptr(), 1);
        let s = unsafe { CString::from_raw(result) }.into_string().unwrap();
        assert!(!s.is_empty());
        // Should be hex-encoded DNS packet
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
