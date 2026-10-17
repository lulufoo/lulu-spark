//! `search`: full-text search over every workspace, or over one.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::config::paths;
use crate::services::spark_data::list_workspaces;
use crate::services::search_index::{index_exists, parse_limit, search_in_cache, SearchFilter};

use super::resolve::{bad_request, internal, open_data, resolve_workspace, text_field};

/// `{ q, limit?, workspace? }` → `{ items: [{ id, title, snippet, workspace_name, workspace, project }] }`.
/// `workspace` is an id, `workspace_name` its current name; without it every workspace is searched.
pub fn search(payload: &Value) -> Value {
    let Some(q) = text_field(payload, "q") else {
        return bad_request("Missing q");
    };
    let limit = payload
        .get("limit")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    let workspace = match text_field(payload, "workspace") {
        None => None,
        Some(id) => match open_data().and_then(|conn| resolve_workspace(&conn, Some(id))) {
            Ok(row) => Some(row.id),
            Err(v) => return v,
        },
    };
    let cache = match paths::cache_dir() {
        Ok(p) => p,
        Err(e) => return internal(format!("{e:?}")),
    };
    if !index_exists(&cache) {
        return json!({ "error": "not_indexed", "_status": 503 });
    }
    let names: HashMap<String, String> = match open_data().and_then(|conn| list_workspaces(&conn).map_err(internal)) {
        Ok(rows) => rows.into_iter().map(|w| (w.id, w.name)).collect(),
        Err(v) => return v,
    };
    let filter = SearchFilter { workspace, ..SearchFilter::default() };
    match search_in_cache(&cache, q, &filter, parse_limit(limit, 10)) {
        Ok(hits) => json!({
            "items": hits
                .into_iter()
                .map(|h| json!({
                    "id": h.doc_id,
                    "title": h.title,
                    "snippet": h.snippet,
                    "workspace_name": names.get(&h.workspace_id).cloned().unwrap_or_default(),
                    "workspace": h.workspace_id,
                    "project": h.topic,
                }))
                .collect::<Vec<_>>()
        }),
        Err(e) => internal(e),
    }
}
