//! ATX heading chunks, 1200-char cap, short title-only merge.

pub const MAX_CHUNK_CHARS: usize = 1200;
pub const SHORT_TITLE_MERGE_CHARS: usize = 80;

pub fn chunk_markdown(text: &str) -> Vec<String> {
    let sections = split_atx(text);
    let sized = split_oversize(sections);
    merge_short_titles(sized)
}

fn is_atx_heading(line: &str) -> bool {
    let t = line.trim_start();
    let hashes = t.chars().take_while(|c| *c == '#').count();
    if hashes < 1 || hashes > 6 {
        return false;
    }
    let rest = &t[t.char_indices().nth(hashes).map(|(i, _)| i).unwrap_or(t.len())..];
    rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t')
}

fn split_atx(text: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();
    let mut saw_heading = false;
    for line in text.lines() {
        if is_atx_heading(line) {
            saw_heading = true;
            if !current.trim().is_empty() {
                sections.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.trim().is_empty() {
        sections.push(current);
    } else if !saw_heading && !text.trim().is_empty() {
        sections.push(text.to_string());
    }
    if sections.is_empty() && !text.is_empty() {
        sections.push(text.to_string());
    }
    sections
}

fn split_oversize(sections: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for section in sections {
        if char_len(&section) <= MAX_CHUNK_CHARS {
            out.push(section);
            continue;
        }
        let mut buf = String::new();
        for line in section.lines() {
            let candidate = if buf.is_empty() {
                line.to_string()
            } else {
                format!("{buf}\n{line}")
            };
            if char_len(&candidate) <= MAX_CHUNK_CHARS {
                buf = candidate;
                continue;
            }
            if !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
            if char_len(line) <= MAX_CHUNK_CHARS {
                buf = line.to_string();
            } else {
                out.extend(split_chars(line, MAX_CHUNK_CHARS));
            }
        }
        if !buf.is_empty() {
            out.push(buf);
        }
    }
    out
}

fn split_chars(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        if char_len(&buf) >= max {
            out.push(std::mem::take(&mut buf));
        }
        buf.push(c);
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

fn is_pure_title(text: &str) -> bool {
    let mut any = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if !is_atx_heading(t) {
            return false;
        }
        any = true;
    }
    any
}

fn merge_short_titles(chunks: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut pending: Option<String> = None;
    for chunk in chunks {
        if is_pure_title(&chunk) && char_len(&chunk) < SHORT_TITLE_MERGE_CHARS {
            pending = Some(match pending {
                Some(prev) => format!("{prev}\n{chunk}"),
                None => chunk,
            });
            continue;
        }
        let merged = match pending.take() {
            Some(title) => format!("{title}\n{chunk}"),
            None => chunk,
        };
        out.push(merged);
    }
    if let Some(title) = pending {
        out.push(title);
    }
    out
}

pub fn char_len(text: &str) -> usize {
    text.chars().count()
}

pub fn first_heading_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix('#') {
            let title = rest.trim_start_matches('#').trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}
