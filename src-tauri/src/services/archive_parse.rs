//! Path helpers for archive notes (`common_path`, translations, entry ids).

pub fn expected_lang_common_path(primary_common_path: &str, lang: &str) -> Option<String> {
    if !primary_common_path.ends_with(".md") {
        return None;
    }
    let stem = &primary_common_path[..primary_common_path.len() - 3];
    Some(format!("{stem}-{lang}.md"))
}

pub fn expected_zh_common_path(primary_common_path: &str) -> Option<String> {
    expected_lang_common_path(primary_common_path, "zh")
}

pub fn is_valid_common_path(common_path: &str) -> bool {
    // `{folder}/{filename}` where `folder` has 0..=3 segments (none: the file is in the project root).
    // Generated notes use `{ts}-{rand}.md`. A caller-chosen name such as `architecture-notes.md`
    // is the same kind of path, so a later rename can still land on a valid one.
    let parts: Vec<&str> = common_path.split('/').collect();
    if parts.is_empty() || parts.len() > 4 {
        return false;
    }
    if parts[..parts.len() - 1].iter().any(|segment| !is_path_segment(segment)) {
        return false;
    }
    is_markdown_filename(parts[parts.len() - 1])
}

fn is_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.contains('\\')
        && !segment.contains("..")
        && segment
            .chars()
            .all(|c| !c.is_control() && !matches!(c, '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
}

fn is_markdown_filename(filename: &str) -> bool {
    let Some(stem) = filename.strip_suffix(".md") else {
        return false;
    };
    is_path_segment(stem) && !stem.ends_with(' ') && !stem.ends_with('.')
}

pub fn ts_from_common_path(common_path: &str) -> Option<String> {
    let filename = common_path.rsplit('/').next()?;
    if filename.len() < 13 {
        return None;
    }
    let ts = &filename[..12];
    if ts.chars().all(|c| c.is_ascii_digit()) {
        Some(ts.to_string())
    } else {
        None
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/archive_parse.rs"]
mod tests;
