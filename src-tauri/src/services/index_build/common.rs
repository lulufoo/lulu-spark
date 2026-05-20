//! Shared index-build helpers (aligned with `build_*_index.py`).

use serde_json::{json, Value};

pub const BATCH_SIZE: usize = 100;
pub const SKIP_FILES: &[&str] = &["_index.md", "README.md", "readme.md"];
pub const WORKBENCH_LAYERS: &[&str] = &["raw", "distilled", "digest", "diagnose"];

/// `re.sub(r'[^a-zA-Z0-9\-_]', '_', raw_id)[:511]` (workbench L118–120, knowledge L166–167).
pub fn sanitize_doc_id(raw_id: &str) -> String {
    raw_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(511)
        .collect()
}

pub fn workbench_doc_id(layer: &str, common_path: &str) -> String {
    sanitize_doc_id(&format!("{layer}__{common_path}"))
}

pub fn should_skip_md(name: &str) -> bool {
    SKIP_FILES.contains(&name)
}

/// Workbench title: first `# ` line, else filename stem without date prefix (L108–115).
pub fn extract_workbench_title(content: &str, filename: &str) -> String {
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

pub fn extract_date_from_filename(filename: &str) -> String {
    if filename.len() >= 8 && filename.as_bytes()[..8].iter().all(|b| b.is_ascii_digit()) {
        filename[..8].to_string()
    } else {
        String::new()
    }
}

pub fn workbench_topic_from_path(common_path: &str) -> String {
    let parts: Vec<&str> = common_path.split('/').collect();
    if parts.len() > 1 {
        parts[0].to_string()
    } else {
        String::new()
    }
}

pub fn build_workbench_document(
    layer: &str,
    common_path: &str,
    body: &str,
    filename: &str,
) -> Value {
    json!({
        "id": workbench_doc_id(layer, common_path),
        "layer": layer,
        "common_path": common_path,
        "title": extract_workbench_title(body, filename),
        "date": extract_date_from_filename(filename),
        "topic": workbench_topic_from_path(common_path),
        "body": body,
    })
}

pub fn upsert_batches<F>(batches: &[Vec<Value>], mut send: F) -> Result<usize, String>
where
    F: FnMut(&[Value]) -> Result<(), String>,
{
    let mut total = 0;
    for batch in batches {
        if batch.is_empty() {
            continue;
        }
        send(batch)?;
        total += batch.len();
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::search::build_knowledge_document;

    #[test]
    fn workbench_doc_id_matches_python_rules() {
        assert_eq!(
            workbench_doc_id("raw", "proj/note.md"),
            sanitize_doc_id("raw__proj/note.md")
        );
        assert_eq!(workbench_doc_id("raw", "a/b/c.md"), "raw__a_b_c_md");
    }

    #[test]
    fn workbench_title_from_heading_or_slug() {
        assert_eq!(
            extract_workbench_title("# My Title\n\nx", "20250101-note.md"),
            "My Title"
        );
        assert_eq!(
            extract_workbench_title("no heading", "20250101-my-slug.md"),
            "my slug"
        );
    }

    #[test]
    fn knowledge_doc_id_matches_build_knowledge_document() {
        let doc = build_knowledge_document("o/repo", "docs/a.md", "# T\n", "desc");
        let expected = sanitize_doc_id("repo__docs__a.md");
        assert_eq!(doc["id"], expected);
    }
}
