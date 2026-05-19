use std::path::Path;

use serde_json::json;

use crate::repositories::index_json::{load_knowledge_entries, IndexJsonError};

/// Build JSON body aligned with `server.py::_handle_knowledge_index` / `_json_response`.
pub fn knowledge_index_json(index_path: &Path) -> serde_json::Value {
    match load_knowledge_entries(index_path) {
        Ok(entries) => {
            let cached_at = chrono::Utc::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            json!({
                "entries": entries
                    .into_iter()
                    .map(|e| {
                        json!({
                            "id": e.id,
                            "repo": e.repo,
                            "description": e.description,
                            "indexUrl": e.index_url,
                        })
                    })
                    .collect::<Vec<_>>(),
                "cached_at": cached_at,
            })
        }
        Err(IndexJsonError::NotFound) => json!({ "error": "knowledge-index.json not found" }),
        Err(IndexJsonError::InvalidFormat(msg)) => json!({ "error": msg }),
        Err(IndexJsonError::NoValidEntries) => {
            json!({ "error": "no valid entries in knowledge-index" })
        }
        Err(IndexJsonError::InvalidJson(msg)) => {
            json!({ "error": format!("invalid knowledge-index: {msg}") })
        }
    }
}

#[cfg(test)]
fn normalize_cached_at(v: &mut serde_json::Value) {
    if let Some(obj) = v.as_object_mut() {
        if obj.contains_key("cached_at") {
            obj.insert(
                "cached_at".to_string(),
                serde_json::Value::String("__NORMALIZED__".into()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/repositories/fixtures")
            .join(name)
    }

    #[test]
    fn success_body_matches_python_shape_after_cached_at_norm() {
        let path = fixture("knowledge-index-valid.json");
        let mut got = knowledge_index_json(&path);
        normalize_cached_at(&mut got);
        let expected = serde_json::json!({
            "entries": [{
                "id": "ai-notes",
                "repo": "lulufoo/ai-notes",
                "description": "AI notes corpus",
                "indexUrl": "https://github.com/lulufoo/ai-notes/blob/main/_index.md",
            }],
            "cached_at": "__NORMALIZED__",
        });
        assert_eq!(got, expected);
    }

    #[test]
    fn force_parameter_does_not_change_disk_read_semantics() {
        let path = fixture("knowledge-index-valid.json");
        let a = knowledge_index_json(&path);
        let b = knowledge_index_json(&path);
        assert_eq!(
            a.get("entries").unwrap(),
            b.get("entries").unwrap()
        );
    }

    #[test]
    fn not_found_error_body() {
        let path = fixture("does-not-exist.json");
        let v = knowledge_index_json(&path);
        assert_eq!(
            v,
            json!({ "error": "knowledge-index.json not found" })
        );
    }

    #[test]
    fn no_valid_entries_error_body() {
        let v = knowledge_index_json(&fixture("knowledge-index-empty.json"));
        assert_eq!(
            v,
            json!({ "error": "no valid entries in knowledge-index" })
        );
    }

    #[test]
    fn invalid_json_error_body_prefix() {
        let v = knowledge_index_json(&fixture("knowledge-index-broken.json"));
        let err = v.get("error").and_then(|x| x.as_str()).expect("error");
        assert!(
            err.starts_with("invalid knowledge-index:"),
            "got {err:?}"
        );
    }
}
