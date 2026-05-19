use std::path::Path;

use serde::{Deserialize, Serialize};

const DEFAULT_OWNER: &str = "lulufoo";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub repo: String,
    pub description: String,
    #[serde(rename = "indexUrl")]
    pub index_url: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IndexJsonError {
    NotFound,
    InvalidJson(String),
    InvalidFormat(String),
    NoValidEntries,
}

impl std::fmt::Display for IndexJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexJsonError::NotFound => write!(f, "knowledge-index.json not found"),
            IndexJsonError::InvalidJson(msg) => write!(f, "invalid knowledge-index: {msg}"),
            IndexJsonError::InvalidFormat(msg) => write!(f, "{msg}"),
            IndexJsonError::NoValidEntries => write!(f, "no valid entries in knowledge-index"),
        }
    }
}

impl std::error::Error for IndexJsonError {}

pub fn load_knowledge_entries(index_path: &Path) -> Result<Vec<KnowledgeEntry>, IndexJsonError> {
    if !index_path.is_file() {
        return Err(IndexJsonError::NotFound);
    }

    let raw = std::fs::read_to_string(index_path).map_err(|e| IndexJsonError::InvalidJson(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| IndexJsonError::InvalidJson(e.to_string()))?;

    let entries_in = match value {
        serde_json::Value::Array(items) => serde_json::Value::Array(items),
        serde_json::Value::Object(map) => map
            .get("entries")
            .cloned()
            .ok_or_else(|| {
                IndexJsonError::InvalidFormat("knowledge-index JSON must have 'entries' array".into())
            })?,
        _ => {
            return Err(IndexJsonError::InvalidFormat(
                "knowledge-index JSON must be object or array".into(),
            ));
        }
    };

    let Some(entries_array) = entries_in.as_array() else {
        return Err(IndexJsonError::InvalidFormat("'entries' must be an array".into()));
    };

    let mut out = Vec::new();
    for item in entries_array {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let entry_id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if entry_id.is_empty() {
            continue;
        }
        let description = obj
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if description.is_empty() {
            continue;
        }
        let mut repo = obj
            .get("repo")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if repo.is_empty() {
            repo = format!("{DEFAULT_OWNER}/{entry_id}");
        }
        let mut index_url = obj
            .get("indexUrl")
            .or_else(|| obj.get("index_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if index_url.is_empty() {
            index_url = format!("https://github.com/{repo}/blob/main/_index.md");
        }
        out.push(KnowledgeEntry {
            id: entry_id,
            repo,
            description,
            index_url,
        });
    }

    if out.is_empty() {
        return Err(IndexJsonError::NoValidEntries);
    }
    Ok(out)
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
    fn load_valid_object_entries() {
        let entries = load_knowledge_entries(&fixture("knowledge-index-valid.json")).expect("load");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "ai-notes");
        assert_eq!(entries[0].repo, "lulufoo/ai-notes");
        assert_eq!(entries[0].description, "AI notes corpus");
        assert!(
            entries[0]
                .index_url
                .contains("lulufoo/ai-notes/blob/main/_index.md")
        );
    }

    #[test]
    fn load_valid_root_array() {
        let path = fixture("knowledge-index-array.json");
        let entries = load_knowledge_entries(&path).expect("load array root");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "wb");
    }

    #[test]
    fn missing_file_returns_not_found() {
        let err = load_knowledge_entries(&fixture("missing.json")).unwrap_err();
        assert_eq!(err, IndexJsonError::NotFound);
    }

    #[test]
    fn invalid_json_returns_invalid_json_error() {
        let err = load_knowledge_entries(&fixture("knowledge-index-broken.json")).unwrap_err();
        assert!(matches!(err, IndexJsonError::InvalidJson(_)));
    }

    #[test]
    fn empty_valid_entries_returns_no_valid_entries() {
        let err = load_knowledge_entries(&fixture("knowledge-index-empty.json")).unwrap_err();
        assert_eq!(err, IndexJsonError::NoValidEntries);
    }

    #[test]
    fn knowledge_index_path_under_cache_dir() {
        let path = crate::config::paths::knowledge_index_path().expect("path");
        assert!(path.ends_with(".cache/knowledge-index.json"));
    }
}
