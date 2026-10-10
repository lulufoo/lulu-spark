//! Resolve a knowledge document id for the in-app open flow (`[title](knowledge:<id>)`).

use std::path::{Component, Path};

use serde_json::{json, Value};

use crate::config::paths;

use super::mcp::get_knowledge_path_by_id;

/// Look up knowledge document `id` and turn its file into the knowledge page's route
/// parameters: `repo` (library directory) and `path` (relative, `/`-separated).
/// Success is `{id, ok, repo, path}`. Never returns a file body or an absolute path.
pub fn resolve_knowledge_for_open(id: &str) -> Value {
    let located = get_knowledge_path_by_id(id);
    if located.get("ok") != Some(&json!(true)) {
        return located;
    }
    let doc_id = located.get("id").and_then(Value::as_str).unwrap_or(id.trim());
    let Some(file) = located.get("path").and_then(Value::as_str) else {
        return fail(doc_id, "Missing path");
    };
    let root = match paths::knowledge_root() {
        Ok(root) => root.canonicalize().unwrap_or(root),
        Err(_) => return fail(doc_id, "Knowledge library not found"),
    };
    let file = Path::new(file);
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    match library_location(&root, &file) {
        Some((repo, path)) => json!({ "id": doc_id, "ok": true, "repo": repo, "path": path }),
        None => fail(doc_id, "Not in the knowledge library"),
    }
}

fn fail(id: &str, error: &str) -> Value {
    json!({ "id": id, "ok": false, "error": error })
}

/// `{root}/<repo>/<a>/<b>.md` → `("<repo>", "a/b.md")`. A file needs a repo directory.
fn library_location(root: &Path, file: &Path) -> Option<(String, String)> {
    let rel = file.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for comp in rel.components() {
        match comp {
            Component::Normal(name) => parts.push(name.to_str()?.to_string()),
            _ => return None,
        }
    }
    if parts.len() < 2 {
        return None;
    }
    let repo = parts.remove(0);
    Some((repo, parts.join("/")))
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/open.rs"]
mod tests;
