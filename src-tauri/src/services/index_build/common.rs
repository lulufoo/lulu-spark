//! Shared index-build helpers: skip rules, title fallback.

pub const SKIP_FILES: &[&str] = &["_index.md", "README.md", "readme.md"];

pub fn should_skip_md(name: &str) -> bool {
    if SKIP_FILES.contains(&name) {
        return true;
    }
    // Translation variants: …-{lang}.md where lang is ISO 639-1 (two lowercase letters).
    // Indexed via entry.translations, not as standalone search-index docs.
    name.strip_suffix(".md").is_some_and(|stem| {
        stem.rsplit_once('-')
            .is_some_and(|(_, lang)| lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase()))
    })
}

/// Spark title: first `# ` line, else filename stem without date prefix.
pub fn extract_spark_title(content: &str, filename: &str) -> String {
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("# ") {
            let title = rest.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    let slug = strip_date_prefix(stem);
    slug.replace('-', " ")
}

fn strip_date_prefix(stem: &str) -> &str {
    let bytes = stem.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && i < 14 && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i >= 8 && bytes.get(i) == Some(&b'-') {
        &stem[i + 1..]
    } else {
        stem
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/index_build/common.rs"]
mod tests;
