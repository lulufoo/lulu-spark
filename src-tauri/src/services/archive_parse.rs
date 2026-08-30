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
    let parts: Vec<&str> = common_path.split('/').collect();
    if parts.len() != 3 {
        return false;
    }
    let filename = parts[2];
    if !filename.ends_with(".md") {
        return false;
    }
    let stem = &filename[..filename.len() - 3];
    let Some((ts, slug)) = stem.split_once('-') else {
        return false;
    };
    ts.len() == 12
        && ts.chars().all(|c| c.is_ascii_digit())
        && !slug.is_empty()
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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

pub fn is_valid_entry_id(id: &str) -> bool {
    id.len() == 32
        && id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
}

#[cfg(test)]
#[path = "../unit-tests/services/archive_parse.rs"]
mod tests;
