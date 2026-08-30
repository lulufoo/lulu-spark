//! Mechanical zh-translation checks before notes write.
//!
//! Skill text cannot stop a stub. Host rejects markers, short bodies, and
//! heading/turn mismatch. Keep these rules in sync with
//! `note-task/scripts/check_zh_parity.py`.

/// Tokens that mean the agent did not send a real translation.
pub const STUB_MARKERS: &[&str] = &[
    "SEE_FILE",
    "PLACEHOLDER",
    "FULL_ZH",
    "ZH_BODY",
    "SEE_FULL",
    "见文件",
];

pub fn body_after_separator(document: &str) -> &str {
    for sep in ["\n---\n", "\n---\r\n"] {
        if let Some(idx) = document.find(sep) {
            return &document[idx + sep.len()..];
        }
    }
    ""
}

pub fn has_stub_marker(text: &str) -> bool {
    let ascii_hay = text.to_ascii_uppercase();
    for marker in STUB_MARKERS {
        if marker.chars().all(|c| c.is_ascii()) {
            if ascii_hay.contains(&marker.to_ascii_uppercase()) {
                return true;
            }
        } else if text.contains(marker) {
            return true;
        }
    }
    false
}

pub fn count_headings(body: &str) -> usize {
    body.lines()
        .filter(|line| line.trim_start().starts_with("## "))
        .count()
}

pub fn count_turns(body: &str) -> usize {
    body.lines().filter(|line| is_turn_line(line)).count()
}

fn is_turn_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('>') {
        return false;
    }
    let head: String = trimmed.chars().take(80).collect();
    let lower = head.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return false;
    }
    head.contains(':') || head.contains('：')
}

/// True when zh body char count is under 1/4 of the English body.
pub fn zh_too_short(en_body: &str, zh_body: &str) -> bool {
    let en = en_body.trim().chars().count();
    let zh = zh_body.trim().chars().count();
    zh == 0 || zh.saturating_mul(4) < en
}

/// True when zh turn count is under 80% of English turn count.
pub fn zh_turns_too_few(en_turns: usize, zh_turns: usize) -> bool {
    en_turns > 0 && zh_turns.saturating_mul(5) < en_turns.saturating_mul(4)
}

pub fn check_zh_parity(en_doc: &str, zh_doc: &str) -> Result<(), String> {
    if has_stub_marker(zh_doc) {
        return Err("zh translation looks like a stub".to_string());
    }
    let en_body = body_after_separator(en_doc);
    let zh_body = body_after_separator(zh_doc);
    if zh_body.trim().is_empty() {
        return Err("zh translation body is empty".to_string());
    }
    if zh_too_short(en_body, zh_body) {
        return Err("zh translation is too short relative to the English body".to_string());
    }
    let en_headings = count_headings(en_body);
    let zh_headings = count_headings(zh_body);
    if en_headings > 0 && zh_headings != en_headings {
        return Err(format!(
            "zh heading count {zh_headings} != English {en_headings}"
        ));
    }
    let en_turns = count_turns(en_body);
    let zh_turns = count_turns(zh_body);
    if zh_turns_too_few(en_turns, zh_turns) {
        return Err(format!(
            "zh turn count {zh_turns} below 80% of English {en_turns}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = "# T\n\n> 创建时间：2026年1月1日 12:00\n\n---\n\n## A\nHost: hello\n\n## B\nGuest: hi\n";
    const ZH_OK: &str = "# 题\n\n> 创建时间：2026年1月1日 12:00\n\n---\n\n## 甲\n主持：你好\n\n## 乙\n嘉宾：嗨\n";

    #[test]
    fn accepts_aligned_zh() {
        assert!(check_zh_parity(EN, ZH_OK).is_ok());
    }

    #[test]
    fn rejects_stub_marker() {
        let zh = format!("{ZH_OK}\nSEE_FILE\n");
        assert!(check_zh_parity(EN, &zh).unwrap_err().contains("stub"));
    }

    #[test]
    fn rejects_short_body() {
        let long_en = format!(
            "# T\n\n> 创建时间：2026年1月1日 12:00\n\n---\n\n{}\nHost: x\n",
            "word ".repeat(80)
        );
        let zh = "# 题\n\n> 创建时间：2026年1月1日 12:00\n\n---\n\n短\n主持：x\n";
        assert!(check_zh_parity(&long_en, zh)
            .unwrap_err()
            .contains("too short"));
    }

    #[test]
    fn rejects_heading_mismatch() {
        let zh = "# 题\n\n> 创建时间：2026年1月1日 12:00\n\n---\n\n## 甲\n主持：你好\n嘉宾：嗨\n";
        assert!(check_zh_parity(EN, zh)
            .unwrap_err()
            .contains("heading"));
    }
}
