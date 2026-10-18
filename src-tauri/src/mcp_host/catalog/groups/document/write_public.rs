//! MCP-facing success shape for create_doc / update_doc.
//!
//! Host write functions still return internal paths for the app, message
//! center, and tests. MCP strips those paths after produce.

use serde_json::{json, Value};

fn nonempty_array(value: &Value) -> bool {
    value.as_array().is_some_and(|items| !items.is_empty())
}

/// Success becomes `{ok, id, workspace_id?, workspace_name?, project_id?, project_name?}` plus
/// written-flags. The workspace name is what a chat link shows:
/// `[workspace name：title](spark-doc:<id>)`. Failures pass through.
pub(super) fn mcp_doc_write_public(result: Value) -> Value {
    if result.get("error").is_some() || result.get("ok") != Some(&json!(true)) {
        return result;
    }
    let Some(id) = result.get("id").cloned() else {
        return result;
    };
    let mut public = json!({
        "ok": true,
        "id": id,
    });
    for key in ["workspace_id", "workspace_name", "project_id", "project_name"] {
        if let Some(value) = result.get(key) {
            public[key] = value.clone();
        }
    }
    if result.get("digest_path").is_some() {
        public["digest_written"] = json!(true);
    }
    if result.get("extra_paths").is_some_and(nonempty_array) {
        public["zh_written"] = json!(true);
    }
    if result.get("asset_paths").is_some_and(nonempty_array) {
        public["assets_copied"] = json!(true);
    }
    public
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_paths_and_keeps_written_flags() {
        let public = mcp_doc_write_public(json!({
            "ok": true,
            "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "common_path": "inbox/notes/n.md",
            "raw_path": "raw/inbox/notes/n.md",
            "created_at": "202606191430",
            "digest_path": "digest/inbox/notes/n.md",
            "extra_paths": ["raw/inbox/notes/n-zh.md"],
            "asset_paths": ["raw/inbox/notes/a.png"],
        }));
        assert_eq!(
            public,
            json!({
                "ok": true,
                "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "digest_written": true,
                "zh_written": true,
                "assets_copied": true,
            })
        );
    }

    #[test]
    fn keeps_the_workspace_and_project_ids_and_names() {
        let public = mcp_doc_write_public(json!({
            "ok": true,
            "id": "default-cccccccccccccccccccccccccccccccc",
            "workspace_id": "default",
            "workspace_name": "Default",
            "project_id": "inbox",
            "project_name": "Inbox",
            "created_at": "202606191430",
        }));
        assert_eq!(
            public,
            json!({
                "ok": true,
                "id": "default-cccccccccccccccccccccccccccccccc",
                "workspace_id": "default",
                "workspace_name": "Default",
                "project_id": "inbox",
                "project_name": "Inbox",
            })
        );
    }

    #[test]
    fn omits_flags_when_optional_layers_absent() {
        let public = mcp_doc_write_public(json!({
            "ok": true,
            "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "common_path": "inbox/notes/n.md",
            "raw_path": "raw/inbox/notes/n.md",
        }));
        assert_eq!(
            public,
            json!({
                "ok": true,
                "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            })
        );
    }

    #[test]
    fn passes_errors_through() {
        let err = json!({ "error": "Missing id", "_status": 400 });
        assert_eq!(mcp_doc_write_public(err.clone()), err);
    }
}
