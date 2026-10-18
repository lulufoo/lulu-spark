//! `search`: full-text search over every workspace, or over one.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::config::paths;
use crate::services::spark_data::{get_project, list_workspaces};
use crate::services::search_index::{index_exists, parse_limit, search_in_cache, SearchFilter};

use super::resolve::{bad_request, internal, open_data, resolve_workspace, text_field};

/// `{ q, limit?, workspace_id? }` →
/// `{ items: [{ id, title, snippet, workspace_id, workspace_name, project_id, project_name }] }`.
/// The names are the current ones; without `workspace_id` every workspace is searched.
pub fn search(payload: &Value) -> Value {
    let Some(q) = text_field(payload, "q") else {
        return bad_request("Missing q");
    };
    let limit = payload
        .get("limit")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    let workspace = match text_field(payload, "workspace_id") {
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
    let hits = match search_in_cache(&cache, q, &filter, parse_limit(limit, 10)) {
        Ok(hits) => hits,
        Err(e) => return internal(e),
    };
    let project_names = match project_names(hits.iter().map(|h| h.topic.as_str())) {
        Ok(names) => names,
        Err(v) => return v,
    };
    json!({
        "items": hits
            .into_iter()
            .map(|h| json!({
                "id": h.doc_id,
                "title": h.title,
                "snippet": h.snippet,
                "workspace_id": h.workspace_id,
                "workspace_name": names.get(&h.workspace_id).cloned().unwrap_or_default(),
                "project_name": project_names.get(&h.topic).cloned().unwrap_or_default(),
                "project_id": h.topic,
            }))
            .collect::<Vec<_>>()
    })
}

/// The current name of each project id (`hit.topic`), looked up once per distinct id.
fn project_names<'a>(ids: impl Iterator<Item = &'a str>) -> Result<HashMap<String, String>, Value> {
    let conn = open_data()?;
    let mut names = HashMap::new();
    for id in ids {
        if id.is_empty() || names.contains_key(id) {
            continue;
        }
        if let Some(row) = get_project(&conn, id).map_err(internal)? {
            names.insert(row.id, row.name);
        }
    }
    Ok(names)
}
