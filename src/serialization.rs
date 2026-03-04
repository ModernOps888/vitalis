//! Serialization Module for Vitalis v30.0
//!
//! Zero-copy serialization framework with best-in-class codecs:
//! - JSON codec (recursive descent parser + pretty printer)
//! - MessagePack binary codec (compact, schema-less)
//! - Base64 encoding/decoding (RFC 4648)
//! - Hex encoding/decoding
//! - Varint encoding (LEB128-style, protobuf-compatible)
//! - URL percent-encoding (RFC 3986)

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── JSON Value Model ───────────────────────────────────────────────

/// Universal serialization value — JSON-compatible with extensions.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Str(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>), // ordered map (preserves insertion order)
}

// ─── JSON Parser (Recursive Descent) ────────────────────────────────

struct JsonParser {
    chars: Vec<char>,
    pos: usize,
}

impl JsonParser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
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

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.pos < self.chars.len() {
            return Err(format!("Trailing data at position {}", self.pos));
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('"') => self.parse_string().map(JsonValue::Str),
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('t') | Some('f') => self.parse_bool(),
            Some('n') => self.parse_null(),
            Some(ch) if ch == '-' || ch.is_ascii_digit() => self.parse_number(),
            Some(ch) => Err(format!("Unexpected character '{}' at position {}", ch, self.pos)),
            None => Err("Unexpected end of input".into()),
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.advance() != Some('"') {
            return Err("Expected '\"'".into());
        }
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('\\') => match self.advance() {
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('/') => s.push('/'),
                    Some('b') => s.push('\u{0008}'),
                    Some('f') => s.push('\u{000C}'),
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('u') => {
                        let hex = self.take_n(4)?;
                        let cp = u32::from_str_radix(&hex, 16)
                            .map_err(|_| format!("Invalid unicode escape: \\u{}", hex))?;
                        if let Some(ch) = char::from_u32(cp) {
                            s.push(ch);
                        } else {
                            return Err(format!("Invalid unicode codepoint: U+{:04X}", cp));
                        }
                    }
                    _ => return Err("Invalid escape sequence".into()),
                },
                Some('"') => return Ok(s),
                Some(ch) => s.push(ch),
                None => return Err("Unterminated string".into()),
            }
        }
    }

    fn take_n(&mut self, n: usize) -> Result<String, String> {
        let mut s = String::new();
        for _ in 0..n {
            match self.advance() {
                Some(ch) => s.push(ch),
                None => return Err("Unexpected end".into()),
            }
        }
        Ok(s)
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;
        let mut is_float = false;

        if self.peek() == Some('-') {
            self.advance();
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        if self.peek() == Some('.') {
            is_float = true;
            self.advance();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        if let Some('e' | 'E') = self.peek() {
            is_float = true;
            self.advance();
            if let Some('+' | '-') = self.peek() {
                self.advance();
            }
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let num_str: String = self.chars[start..self.pos].iter().collect();
        if is_float {
            num_str
                .parse::<f64>()
                .map(JsonValue::Float)
                .map_err(|_| format!("Invalid number: {}", num_str))
        } else {
            num_str
                .parse::<i64>()
                .map(JsonValue::Integer)
                .map_err(|_| format!("Invalid integer: {}", num_str))
        }
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.try_consume("true") {
            Ok(JsonValue::Bool(true))
        } else if self.try_consume("false") {
            Ok(JsonValue::Bool(false))
        } else {
            Err("Expected 'true' or 'false'".into())
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.try_consume("null") {
            Ok(JsonValue::Null)
        } else {
            Err("Expected 'null'".into())
        }
    }

    fn try_consume(&mut self, expected: &str) -> bool {
        let expected_chars: Vec<char> = expected.chars().collect();
        if self.pos + expected_chars.len() > self.chars.len() {
            return false;
        }
        for (i, &ch) in expected_chars.iter().enumerate() {
            if self.chars[self.pos + i] != ch {
                return false;
            }
        }
        self.pos += expected_chars.len();
        true
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.advance(); // consume '['
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek() == Some(']') {
            self.advance();
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some(']') => {
                    self.advance();
                    return Ok(JsonValue::Array(items));
                }
                _ => return Err("Expected ',' or ']'".into()),
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.advance(); // consume '{'
        self.skip_whitespace();
        let mut entries = Vec::new();
        if self.peek() == Some('}') {
            self.advance();
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.advance() != Some(':') {
                return Err("Expected ':' after object key".into());
            }
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some('}') => {
                    self.advance();
                    return Ok(JsonValue::Object(entries));
                }
                _ => return Err("Expected ',' or '}'".into()),
            }
        }
    }
}

// ─── JSON Stringify ─────────────────────────────────────────────────

fn json_stringify(value: &JsonValue, pretty: bool) -> String {
    if pretty {
        json_stringify_pretty(value, 0)
    } else {
        json_stringify_compact(value)
    }
}

fn json_stringify_compact(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(b) => if *b { "true" } else { "false" }.into(),
        JsonValue::Integer(n) => n.to_string(),
        JsonValue::Float(f) => {
            if f.is_infinite() || f.is_nan() {
                "null".into()
            } else {
                format!("{}", f)
            }
        }
        JsonValue::Str(s) => json_escape_string(s),
        JsonValue::Array(items) => {
            let parts: Vec<String> = items.iter().map(json_stringify_compact).collect();
            format!("[{}]", parts.join(","))
        }
        JsonValue::Object(entries) => {
            let parts: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}:{}", json_escape_string(k), json_stringify_compact(v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

fn json_stringify_pretty(value: &JsonValue, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let inner_pad = "  ".repeat(indent + 1);
    match value {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(b) => if *b { "true" } else { "false" }.into(),
        JsonValue::Integer(n) => n.to_string(),
        JsonValue::Float(f) => {
            if f.is_infinite() || f.is_nan() {
                "null".into()
            } else {
                format!("{}", f)
            }
        }
        JsonValue::Str(s) => json_escape_string(s),
        JsonValue::Array(items) => {
            if items.is_empty() {
                return "[]".into();
            }
            let parts: Vec<String> = items
                .iter()
                .map(|v| format!("{}{}", inner_pad, json_stringify_pretty(v, indent + 1)))
                .collect();
            format!("[\n{}\n{}]", parts.join(",\n"), pad)
        }
        JsonValue::Object(entries) => {
            if entries.is_empty() {
                return "{}".into();
            }
            let parts: Vec<String> = entries
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}{}: {}",
                        inner_pad,
                        json_escape_string(k),
                        json_stringify_pretty(v, indent + 1)
                    )
                })
                .collect();
            format!("{{\n{}\n{}}}", parts.join(",\n"), pad)
        }
    }
}

fn json_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ─── Base64 Encoding (RFC 4648) ─────────────────────────────────────

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(BASE64_CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(BASE64_CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(BASE64_CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(BASE64_CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode_char(ch: u8) -> Result<u8, String> {
    match ch {
        b'A'..=b'Z' => Ok(ch - b'A'),
        b'a'..=b'z' => Ok(ch - b'a' + 26),
        b'0'..=b'9' => Ok(ch - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(format!("Invalid base64 character: {}", ch as char)),
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let trimmed: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if trimmed.len() % 4 != 0 {
        return Err("Invalid base64 length".into());
    }
    let mut result = Vec::with_capacity(trimmed.len() * 3 / 4);
    for chunk in trimmed.chunks(4) {
        let a = base64_decode_char(chunk[0])? as u32;
        let b = base64_decode_char(chunk[1])? as u32;
        let triple0 = (a << 18) | (b << 12);
        result.push((triple0 >> 16) as u8);

        if chunk[2] != b'=' {
            let c = base64_decode_char(chunk[2])? as u32;
            let triple1 = triple0 | (c << 6);
            result.push((triple1 >> 8) as u8);
            if chunk[3] != b'=' {
                let d = base64_decode_char(chunk[3])? as u32;
                let triple2 = triple1 | d;
                result.push(triple2 as u8);
            }
        }
    }
    Ok(result)
}

// ─── Hex Encoding ───────────────────────────────────────────────────

fn hex_encode(input: &[u8]) -> String {
    let mut result = String::with_capacity(input.len() * 2);
    for &byte in input {
        result.push_str(&format!("{:02x}", byte));
    }
    result
}

fn hex_decode(input: &str) -> Result<Vec<u8>, String> {
    let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.len() % 2 != 0 {
        return Err("Hex string must have even length".into());
    }
    let mut result = Vec::with_capacity(clean.len() / 2);
    for i in (0..clean.len()).step_by(2) {
        let byte_str = &clean[i..i + 2];
        let byte = u8::from_str_radix(byte_str, 16)
            .map_err(|_| format!("Invalid hex byte: {}", byte_str))?;
        result.push(byte);
    }
    Ok(result)
}

// ─── Varint Encoding (LEB128) ───────────────────────────────────────

fn varint_encode(mut n: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(10);
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if n == 0 {
            break;
        }
    }
    buf
}

fn varint_decode(bytes: &[u8]) -> Result<(u64, usize), String> {
    let mut result: u64 = 0;
    let mut shift = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        if shift >= 64 {
            return Err("Varint too long".into());
        }
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        shift += 7;
    }
    Err("Unterminated varint".into())
}

// ─── MessagePack Codec ──────────────────────────────────────────────

fn msgpack_encode(value: &JsonValue) -> Vec<u8> {
    let mut buf = Vec::new();
    msgpack_encode_value(value, &mut buf);
    buf
}

fn msgpack_encode_value(value: &JsonValue, buf: &mut Vec<u8>) {
    match value {
        JsonValue::Null => buf.push(0xc0),
        JsonValue::Bool(false) => buf.push(0xc2),
        JsonValue::Bool(true) => buf.push(0xc3),
        JsonValue::Integer(n) => {
            let n = *n;
            if n >= 0 && n <= 127 {
                buf.push(n as u8);
            } else if n >= -32 && n < 0 {
                buf.push(n as u8); // negative fixint
            } else if n >= 0 && n <= 255 {
                buf.push(0xcc);
                buf.push(n as u8);
            } else if n >= 0 && n <= 65535 {
                buf.push(0xcd);
                buf.extend_from_slice(&(n as u16).to_be_bytes());
            } else if n >= 0 && n <= u32::MAX as i64 {
                buf.push(0xce);
                buf.extend_from_slice(&(n as u32).to_be_bytes());
            } else if n >= 0 {
                buf.push(0xcf);
                buf.extend_from_slice(&(n as u64).to_be_bytes());
            } else if n >= i8::MIN as i64 {
                buf.push(0xd0);
                buf.push(n as i8 as u8);
            } else if n >= i16::MIN as i64 {
                buf.push(0xd1);
                buf.extend_from_slice(&(n as i16).to_be_bytes());
            } else if n >= i32::MIN as i64 {
                buf.push(0xd2);
                buf.extend_from_slice(&(n as i32).to_be_bytes());
            } else {
                buf.push(0xd3);
                buf.extend_from_slice(&n.to_be_bytes());
            }
        }
        JsonValue::Float(f) => {
            buf.push(0xcb);
            buf.extend_from_slice(&f.to_be_bytes());
        }
        JsonValue::Str(s) => {
            let bytes = s.as_bytes();
            let len = bytes.len();
            if len <= 31 {
                buf.push(0xa0 | len as u8);
            } else if len <= 255 {
                buf.push(0xd9);
                buf.push(len as u8);
            } else if len <= 65535 {
                buf.push(0xda);
                buf.extend_from_slice(&(len as u16).to_be_bytes());
            } else {
                buf.push(0xdb);
                buf.extend_from_slice(&(len as u32).to_be_bytes());
            }
            buf.extend_from_slice(bytes);
        }
        JsonValue::Array(items) => {
            let len = items.len();
            if len <= 15 {
                buf.push(0x90 | len as u8);
            } else if len <= 65535 {
                buf.push(0xdc);
                buf.extend_from_slice(&(len as u16).to_be_bytes());
            } else {
                buf.push(0xdd);
                buf.extend_from_slice(&(len as u32).to_be_bytes());
            }
            for item in items {
                msgpack_encode_value(item, buf);
            }
        }
        JsonValue::Object(entries) => {
            let len = entries.len();
            if len <= 15 {
                buf.push(0x80 | len as u8);
            } else if len <= 65535 {
                buf.push(0xde);
                buf.extend_from_slice(&(len as u16).to_be_bytes());
            } else {
                buf.push(0xdf);
                buf.extend_from_slice(&(len as u32).to_be_bytes());
            }
            for (k, v) in entries {
                msgpack_encode_value(&JsonValue::Str(k.clone()), buf);
                msgpack_encode_value(v, buf);
            }
        }
    }
}

fn msgpack_decode(bytes: &[u8]) -> Result<(JsonValue, usize), String> {
    if bytes.is_empty() {
        return Err("Empty data".into());
    }
    let tag = bytes[0];
    match tag {
        0xc0 => Ok((JsonValue::Null, 1)),
        0xc2 => Ok((JsonValue::Bool(false), 1)),
        0xc3 => Ok((JsonValue::Bool(true), 1)),
        // Positive fixint
        0x00..=0x7f => Ok((JsonValue::Integer(tag as i64), 1)),
        // Negative fixint
        0xe0..=0xff => Ok((JsonValue::Integer(tag as i8 as i64), 1)),
        // uint8
        0xcc => {
            if bytes.len() < 2 { return Err("Truncated".into()); }
            Ok((JsonValue::Integer(bytes[1] as i64), 2))
        }
        // uint16
        0xcd => {
            if bytes.len() < 3 { return Err("Truncated".into()); }
            let v = u16::from_be_bytes([bytes[1], bytes[2]]);
            Ok((JsonValue::Integer(v as i64), 3))
        }
        // uint32
        0xce => {
            if bytes.len() < 5 { return Err("Truncated".into()); }
            let v = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
            Ok((JsonValue::Integer(v as i64), 5))
        }
        // uint64
        0xcf => {
            if bytes.len() < 9 { return Err("Truncated".into()); }
            let v = u64::from_be_bytes(bytes[1..9].try_into().unwrap());
            Ok((JsonValue::Integer(v as i64), 9))
        }
        // int8
        0xd0 => {
            if bytes.len() < 2 { return Err("Truncated".into()); }
            Ok((JsonValue::Integer(bytes[1] as i8 as i64), 2))
        }
        // int16
        0xd1 => {
            if bytes.len() < 3 { return Err("Truncated".into()); }
            let v = i16::from_be_bytes([bytes[1], bytes[2]]);
            Ok((JsonValue::Integer(v as i64), 3))
        }
        // int32
        0xd2 => {
            if bytes.len() < 5 { return Err("Truncated".into()); }
            let v = i32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
            Ok((JsonValue::Integer(v as i64), 5))
        }
        // int64
        0xd3 => {
            if bytes.len() < 9 { return Err("Truncated".into()); }
            let v = i64::from_be_bytes(bytes[1..9].try_into().unwrap());
            Ok((JsonValue::Integer(v), 9))
        }
        // float64
        0xcb => {
            if bytes.len() < 9 { return Err("Truncated".into()); }
            let v = f64::from_be_bytes(bytes[1..9].try_into().unwrap());
            Ok((JsonValue::Float(v), 9))
        }
        // fixstr
        0xa0..=0xbf => {
            let len = (tag & 0x1f) as usize;
            if bytes.len() < 1 + len { return Err("Truncated".into()); }
            let s = std::str::from_utf8(&bytes[1..1 + len])
                .map_err(|_| "Invalid UTF-8")?
                .to_string();
            Ok((JsonValue::Str(s), 1 + len))
        }
        // str8
        0xd9 => {
            if bytes.len() < 2 { return Err("Truncated".into()); }
            let len = bytes[1] as usize;
            if bytes.len() < 2 + len { return Err("Truncated".into()); }
            let s = std::str::from_utf8(&bytes[2..2 + len])
                .map_err(|_| "Invalid UTF-8")?
                .to_string();
            Ok((JsonValue::Str(s), 2 + len))
        }
        // fixarray
        0x90..=0x9f => {
            let count = (tag & 0x0f) as usize;
            let mut pos = 1;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                let (val, consumed) = msgpack_decode(&bytes[pos..])?;
                items.push(val);
                pos += consumed;
            }
            Ok((JsonValue::Array(items), pos))
        }
        // fixmap
        0x80..=0x8f => {
            let count = (tag & 0x0f) as usize;
            let mut pos = 1;
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                let (key, consumed_k) = msgpack_decode(&bytes[pos..])?;
                pos += consumed_k;
                let (val, consumed_v) = msgpack_decode(&bytes[pos..])?;
                pos += consumed_v;
                let key_str = match key {
                    JsonValue::Str(s) => s,
                    other => json_stringify_compact(&other),
                };
                entries.push((key_str, val));
            }
            Ok((JsonValue::Object(entries), pos))
        }
        _ => Err(format!("Unsupported msgpack tag: 0x{:02x}", tag)),
    }
}

// ─── URL Percent-Encoding (RFC 3986) ────────────────────────────────

fn url_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.' || byte == b'~' {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{:02X}", byte));
        }
    }
    result
}

fn url_decode(input: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(input.len());
    let chars: Vec<u8> = input.bytes().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == b'%' {
            if i + 2 >= chars.len() {
                return Err("Incomplete percent-encoding".into());
            }
            let hi = (chars[i + 1] as char).to_digit(16).ok_or("Invalid hex in URL")?;
            let lo = (chars[i + 2] as char).to_digit(16).ok_or("Invalid hex in URL")?;
            bytes.push((hi * 16 + lo) as u8);
            i += 3;
        } else if chars[i] == b'+' {
            bytes.push(b' ');
            i += 1;
        } else {
            bytes.push(chars[i]);
            i += 1;
        }
    }
    String::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in decoded URL".into())
}

// ─── JSON Path Query (Dot-Notation) ─────────────────────────────────

fn json_get_path(value: &JsonValue, path: &str) -> Option<JsonValue> {
    let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    let mut current = value.clone();
    for part in parts {
        match &current {
            JsonValue::Object(entries) => {
                if let Some((_, v)) = entries.iter().find(|(k, _)| k == part) {
                    current = v.clone();
                } else {
                    return None;
                }
            }
            JsonValue::Array(items) => {
                if let Ok(idx) = part.parse::<usize>() {
                    if idx < items.len() {
                        current = items[idx].clone();
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(current)
}

// ─── FFI Layer ──────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_json_parse(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match JsonParser::new(input).parse() {
        Ok(val) => json_stringify(&val, false),
        Err(e) => format!("{{\"error\":{}}}", json_escape_string(&e)),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_json_stringify(input: *const c_char, pretty: i64) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match JsonParser::new(input).parse() {
        Ok(val) => json_stringify(&val, pretty != 0),
        Err(e) => format!("{{\"error\":{}}}", json_escape_string(&e)),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_json_get(json: *const c_char, path: *const c_char) -> *mut c_char {
    let json = unsafe { CStr::from_ptr(json) }.to_str().unwrap_or("");
    let path = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("");
    let result = match JsonParser::new(json).parse() {
        Ok(val) => match json_get_path(&val, path) {
            Some(v) => json_stringify(&v, false),
            None => "null".into(),
        },
        Err(_) => "null".into(),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ser_base64_encode(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = base64_encode(input.as_bytes());
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ser_base64_decode(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match base64_decode(input) {
        Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
        Err(_) => String::new(),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ser_hex_encode(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = hex_encode(input.as_bytes());
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ser_hex_decode(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match hex_decode(input) {
        Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
        Err(_) => String::new(),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ser_url_encode(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = url_encode(input);
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_ser_url_decode(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = url_decode(input).unwrap_or_default();
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_msgpack_roundtrip(input: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    let result = match JsonParser::new(input).parse() {
        Ok(val) => {
            let encoded = msgpack_encode(&val);
            match msgpack_decode(&encoded) {
                Ok((decoded, _)) => json_stringify(&decoded, false),
                Err(e) => format!("{{\"error\":{}}}", json_escape_string(&e)),
            }
        }
        Err(e) => format!("{{\"error\":{}}}", json_escape_string(&e)),
    };
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_varint_encode(n: i64) -> *mut c_char {
    let encoded = varint_encode(n as u64);
    let result = base64_encode(&encoded);
    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_varint_decode(input: *const c_char) -> i64 {
    let input = unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("");
    match base64_decode(input) {
        Ok(bytes) => match varint_decode(&bytes) {
            Ok((val, _)) => val as i64,
            Err(_) => -1,
        },
        Err(_) => -1,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // JSON parsing tests
    #[test]
    fn test_json_parse_null() {
        let v = JsonParser::new("null").parse().unwrap();
        assert_eq!(v, JsonValue::Null);
    }

    #[test]
    fn test_json_parse_bool() {
        assert_eq!(JsonParser::new("true").parse().unwrap(), JsonValue::Bool(true));
        assert_eq!(JsonParser::new("false").parse().unwrap(), JsonValue::Bool(false));
    }

    #[test]
    fn test_json_parse_integer() {
        assert_eq!(JsonParser::new("42").parse().unwrap(), JsonValue::Integer(42));
        assert_eq!(JsonParser::new("-17").parse().unwrap(), JsonValue::Integer(-17));
    }

    #[test]
    fn test_json_parse_float() {
        assert_eq!(JsonParser::new("3.14").parse().unwrap(), JsonValue::Float(3.14));
        assert_eq!(JsonParser::new("-1.5e2").parse().unwrap(), JsonValue::Float(-150.0));
    }

    #[test]
    fn test_json_parse_string() {
        assert_eq!(
            JsonParser::new(r#""hello""#).parse().unwrap(),
            JsonValue::Str("hello".into())
        );
    }

    #[test]
    fn test_json_parse_string_escapes() {
        let v = JsonParser::new(r#""line1\nline2\ttab""#).parse().unwrap();
        assert_eq!(v, JsonValue::Str("line1\nline2\ttab".into()));
    }

    #[test]
    fn test_json_parse_unicode() {
        let v = JsonParser::new(r#""\u0041\u0042""#).parse().unwrap();
        assert_eq!(v, JsonValue::Str("AB".into()));
    }

    #[test]
    fn test_json_parse_array() {
        let v = JsonParser::new("[1, 2, 3]").parse().unwrap();
        assert_eq!(
            v,
            JsonValue::Array(vec![
                JsonValue::Integer(1),
                JsonValue::Integer(2),
                JsonValue::Integer(3),
            ])
        );
    }

    #[test]
    fn test_json_parse_empty_array() {
        assert_eq!(JsonParser::new("[]").parse().unwrap(), JsonValue::Array(vec![]));
    }

    #[test]
    fn test_json_parse_object() {
        let v = JsonParser::new(r#"{"a": 1, "b": "hello"}"#).parse().unwrap();
        assert_eq!(
            v,
            JsonValue::Object(vec![
                ("a".into(), JsonValue::Integer(1)),
                ("b".into(), JsonValue::Str("hello".into())),
            ])
        );
    }

    #[test]
    fn test_json_parse_nested() {
        let v = JsonParser::new(r#"{"items": [1, {"x": true}]}"#).parse().unwrap();
        match v {
            JsonValue::Object(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].0, "items");
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_json_roundtrip() {
        let input = r#"{"name":"vitalis","version":30,"features":["regex","json"],"active":true}"#;
        let parsed = JsonParser::new(input).parse().unwrap();
        let output = json_stringify(&parsed, false);
        let reparsed = JsonParser::new(&output).parse().unwrap();
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_json_pretty_print() {
        let v = JsonValue::Object(vec![
            ("a".into(), JsonValue::Integer(1)),
            ("b".into(), JsonValue::Array(vec![JsonValue::Integer(2)])),
        ]);
        let pretty = json_stringify(&v, true);
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("  "));
    }

    #[test]
    fn test_json_get_path() {
        let v = JsonParser::new(r#"{"a": {"b": {"c": 42}}}"#).parse().unwrap();
        let result = json_get_path(&v, "a.b.c");
        assert_eq!(result, Some(JsonValue::Integer(42)));
    }

    #[test]
    fn test_json_get_path_array() {
        let v = JsonParser::new(r#"{"items": [10, 20, 30]}"#).parse().unwrap();
        let result = json_get_path(&v, "items.1");
        assert_eq!(result, Some(JsonValue::Integer(20)));
    }

    #[test]
    fn test_json_get_path_missing() {
        let v = JsonParser::new(r#"{"a": 1}"#).parse().unwrap();
        assert_eq!(json_get_path(&v, "b"), None);
    }

    // Base64 tests
    #[test]
    fn test_base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn test_base64_roundtrip() {
        let inputs = [b"hello" as &[u8], b"Vitalis v30", b"a", b"ab", b"abc", b"\x00\x01\x02\xff"];
        for input in inputs {
            let encoded = base64_encode(input);
            let decoded = base64_decode(&encoded).unwrap();
            assert_eq!(&decoded, input);
        }
    }

    #[test]
    fn test_base64_known_vectors() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
    }

    // Hex tests
    #[test]
    fn test_hex_roundtrip() {
        let input = b"Vitalis";
        let encoded = hex_encode(input);
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(&decoded, input);
    }

    #[test]
    fn test_hex_known() {
        assert_eq!(hex_encode(b"\xde\xad\xbe\xef"), "deadbeef");
        assert_eq!(hex_decode("deadbeef").unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
    }

    // Varint tests
    #[test]
    fn test_varint_roundtrip() {
        let values = [0u64, 1, 127, 128, 300, 65535, 1_000_000, u64::MAX];
        for &v in &values {
            let encoded = varint_encode(v);
            let (decoded, consumed) = varint_decode(&encoded).unwrap();
            assert_eq!(decoded, v);
            assert_eq!(consumed, encoded.len());
        }
    }

    #[test]
    fn test_varint_encoding_size() {
        assert_eq!(varint_encode(0).len(), 1);
        assert_eq!(varint_encode(127).len(), 1);
        assert_eq!(varint_encode(128).len(), 2);
        assert_eq!(varint_encode(16383).len(), 2);
        assert_eq!(varint_encode(16384).len(), 3);
    }

    // MessagePack tests
    #[test]
    fn test_msgpack_null() {
        let encoded = msgpack_encode(&JsonValue::Null);
        let (decoded, _) = msgpack_decode(&encoded).unwrap();
        assert_eq!(decoded, JsonValue::Null);
    }

    #[test]
    fn test_msgpack_bool() {
        let encoded_t = msgpack_encode(&JsonValue::Bool(true));
        let encoded_f = msgpack_encode(&JsonValue::Bool(false));
        assert_eq!(msgpack_decode(&encoded_t).unwrap().0, JsonValue::Bool(true));
        assert_eq!(msgpack_decode(&encoded_f).unwrap().0, JsonValue::Bool(false));
    }

    #[test]
    fn test_msgpack_integers() {
        let values = [0i64, 1, 42, 127, 128, 255, 256, 65535, -1, -32, -33, -128, -32768];
        for &v in &values {
            let encoded = msgpack_encode(&JsonValue::Integer(v));
            let (decoded, _) = msgpack_decode(&encoded).unwrap();
            assert_eq!(decoded, JsonValue::Integer(v), "Failed for {}", v);
        }
    }

    #[test]
    fn test_msgpack_string() {
        let encoded = msgpack_encode(&JsonValue::Str("hello".into()));
        let (decoded, _) = msgpack_decode(&encoded).unwrap();
        assert_eq!(decoded, JsonValue::Str("hello".into()));
    }

    #[test]
    fn test_msgpack_complex() {
        let value = JsonValue::Object(vec![
            ("name".into(), JsonValue::Str("vitalis".into())),
            ("version".into(), JsonValue::Integer(30)),
            ("features".into(), JsonValue::Array(vec![
                JsonValue::Str("regex".into()),
                JsonValue::Bool(true),
            ])),
        ]);
        let encoded = msgpack_encode(&value);
        let (decoded, _) = msgpack_decode(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    // URL encoding tests
    #[test]
    fn test_url_encode_passthrough() {
        assert_eq!(url_encode("hello"), "hello");
        assert_eq!(url_encode("abc-def_ghi.jkl~mno"), "abc-def_ghi.jkl~mno");
    }

    #[test]
    fn test_url_encode_special() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a=b&c=d"), "a%3Db%26c%3Dd");
    }

    #[test]
    fn test_url_roundtrip() {
        let inputs = ["hello world", "a=1&b=2", "path/to/file?q=x#frag", "日本語"];
        for input in inputs {
            let encoded = url_encode(input);
            let decoded = url_decode(&encoded).unwrap();
            assert_eq!(decoded, input);
        }
    }

    // FFI tests
    #[test]
    fn test_ffi_json_parse() {
        let input = CString::new(r#"{"x": 42}"#).unwrap();
        let result = vitalis_json_parse(input.as_ptr());
        let s = unsafe { CString::from_raw(result) }.into_string().unwrap();
        assert!(s.contains("42"));
    }

    #[test]
    fn test_ffi_base64() {
        let input = CString::new("Vitalis").unwrap();
        let encoded = vitalis_ser_base64_encode(input.as_ptr());
        let decoded = vitalis_ser_base64_decode(encoded);
        let s = unsafe { CString::from_raw(decoded) }.into_string().unwrap();
        assert_eq!(s, "Vitalis");
    }

    #[test]
    fn test_ffi_hex() {
        let input = CString::new("AB").unwrap();
        let encoded = vitalis_ser_hex_encode(input.as_ptr());
        let decoded = vitalis_ser_hex_decode(encoded);
        let s = unsafe { CString::from_raw(decoded) }.into_string().unwrap();
        assert_eq!(s, "AB");
    }
}
