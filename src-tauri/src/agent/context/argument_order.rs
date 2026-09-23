//! Key order of a JSON object string.
//!
//! `serde_json` sorts object keys. The chat template walks tool-call
//! arguments in the order written in the argument string.

pub(super) fn object_key_order(text: &str) -> Option<Vec<String>> {
    let bytes = text.as_bytes();
    let mut index = skip_ws(bytes, 0);
    if bytes.get(index).copied() != Some(b'{') {
        return None;
    }
    index += 1;
    let mut keys = Vec::new();
    loop {
        index = skip_ws(bytes, index);
        if bytes.get(index).copied() == Some(b'}') {
            return Some(keys);
        }
        let (key, next) = parse_json_string(text, index)?;
        index = skip_ws(bytes, next);
        if bytes.get(index).copied() != Some(b':') {
            return None;
        }
        index = skip_json_value(bytes, index + 1)?;
        keys.push(key);
        index = skip_ws(bytes, index);
        match bytes.get(index).copied() {
            Some(b',') => index += 1,
            Some(b'}') => return Some(keys),
            _ => return None,
        }
    }
}

fn skip_ws(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

fn parse_json_string(text: &str, index: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(index).copied() != Some(b'"') {
        return None;
    }
    let mut out = String::new();
    let mut index = index + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => return Some((out, index + 1)),
            b'\\' => {
                let escaped = *bytes.get(index + 1)?;
                match escaped {
                    b'"' | b'\\' | b'/' => out.push(escaped as char),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = std::str::from_utf8(bytes.get(index + 2..index + 6)?).ok()?;
                        let code = u32::from_str_radix(hex, 16).ok()?;
                        out.push(char::from_u32(code)?);
                        index += 4;
                    }
                    _ => return None,
                }
                index += 2;
            }
            byte if byte < 0x80 => {
                out.push(byte as char);
                index += 1;
            }
            _ => {
                let ch = text.get(index..)?.chars().next()?;
                out.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    None
}

fn skip_json_value(bytes: &[u8], index: usize) -> Option<usize> {
    let index = skip_ws(bytes, index);
    match bytes.get(index).copied()? {
        b'"' => skip_json_string(bytes, index),
        b'{' => skip_container(bytes, index, b'{', b'}'),
        b'[' => skip_container(bytes, index, b'[', b']'),
        b't' => skip_literal(bytes, index, b"true"),
        b'f' => skip_literal(bytes, index, b"false"),
        b'n' => skip_literal(bytes, index, b"null"),
        b'-' | b'0'..=b'9' => Some(skip_number(bytes, index)),
        _ => None,
    }
}

fn skip_json_string(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index).copied() != Some(b'"') {
        return None;
    }
    let mut index = index + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => return Some(index + 1),
            b'\\' => {
                let escaped = *bytes.get(index + 1)?;
                index += if escaped == b'u' { 6 } else { 2 };
            }
            _ => index += 1,
        }
    }
    None
}

fn skip_container(bytes: &[u8], index: usize, open: u8, close: u8) -> Option<usize> {
    if bytes.get(index).copied() != Some(open) {
        return None;
    }
    let mut index = index + 1;
    loop {
        index = skip_ws(bytes, index);
        if bytes.get(index).copied() == Some(close) {
            return Some(index + 1);
        }
        if open == b'{' {
            index = skip_json_string(bytes, index)?;
            index = skip_ws(bytes, index);
            if bytes.get(index).copied() != Some(b':') {
                return None;
            }
            index += 1;
        }
        index = skip_json_value(bytes, index)?;
        index = skip_ws(bytes, index);
        match bytes.get(index).copied() {
            Some(b',') => index += 1,
            Some(byte) if byte == close => return Some(index + 1),
            _ => return None,
        }
    }
}

fn skip_literal(bytes: &[u8], index: usize, literal: &[u8]) -> Option<usize> {
    if bytes.get(index..index + literal.len()) == Some(literal) {
        Some(index + literal.len())
    } else {
        None
    }
}

fn skip_number(bytes: &[u8], mut index: usize) -> usize {
    while bytes
        .get(index)
        .is_some_and(|byte| matches!(byte, b'+' | b'-' | b'.' | b'e' | b'E' | b'0'..=b'9'))
    {
        index += 1;
    }
    index
}
