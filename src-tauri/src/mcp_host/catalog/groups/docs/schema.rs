//! Shared JSON Schema fragments for the docs MCP tools.

use serde_json::{json, Value};

pub fn source_type_schema() -> Value {
    json!({
        "type": "string",
        "enum": ["summary", "article", "theme-line", "dialogue", "transcript", "jot"],
        "description": "Optional source type; defaults to summary."
    })
}

fn workspace_schema() -> Value {
    json!({
        "type": "string",
        "description": "Optional workspace id from list_workspaces. Defaults to the Default workspace."
    })
}

pub fn workspace_property() -> Value {
    json!({ "workspace": workspace_schema() })
}

pub fn create_doc_properties(include_source_path: bool) -> Value {
    let mut props = json!({
        "title": {
            "type": "string",
            "description": "Required display title. May be Chinese."
        },
        "workspace": workspace_schema(),
        "project": {
            "type": "string",
            "description": "Optional project folder (from list_projects) inside the workspace. Defaults to inbox."
        },
        "folder": {
            "type": "string",
            "description": "Optional relative path inside the project, 1 to 3 levels such as ai/agents/notes. Defaults to notes."
        },
        "created_at": {
            "type": "string",
            "description": "Optional YYYYMMDDHHMM (UTC+8). Defaults to now."
        },
        "source_type": source_type_schema(),
        "digest": {
            "type": "string",
            "enum": ["auto", "always", "never"],
            "description": "Required. auto writes a digest when Host AD-0 applies; always writes; never skips. digest_body is required whenever a digest is written."
        },
        "digest_body": {
            "type": "string",
            "description": "Digest Markdown. Required when digest is always, or auto and the document meets Host digest rules."
        },
        "translations": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "lang": { "type": "string" },
                    "content": { "type": "string" },
                    "source_path": { "type": "string" }
                },
                "required": ["lang"],
                "additionalProperties": false
            },
            "description": "Optional translations. Prefer source_path. Host rejects stub or short zh. Full-English primary without translations.zh is rejected."
        }
    });
    if include_source_path {
        props["source_path"] = json!({
            "type": "string",
            "description": "Absolute path to a Markdown file."
        });
        props["asset_paths"] = json!({
            "type": "array",
            "items": { "type": "string" },
            "description": "Absolute paths to companion images referenced by relative ![]() or <img src> in the source Markdown. Required on this call when the body has such images; omit when it has none. Host copies each into the same directory as the raw document .md, keeping only the source basename; reference each image in the Markdown by that sibling basename (png/jpg/jpeg; max 8 files, 4 MiB each). Existing dest → 409. Never send image bytes. Host does not scan the Markdown."
        });
    } else {
        props["content"] = json!({
            "type": "string",
            "description": "Markdown document body. Host writes this text; do not send source_path."
        });
        if let Some(items) = props
            .get_mut("translations")
            .and_then(|t| t.get_mut("items"))
            .and_then(|i| i.get_mut("properties"))
            .and_then(|p| p.as_object_mut())
        {
            items.remove("source_path");
        }
    }
    props
}

pub fn update_doc_properties(include_source_path: bool) -> Value {
    let mut props = json!({
        "workspace": workspace_schema(),
        "id": {
            "type": "string",
            "description": "Document id (32-char hex) from search or create_doc."
        },
        "digest": {
            "type": "string",
            "enum": ["auto", "always", "never"],
            "description": "Required. auto writes a digest when Host AD-0 applies; always writes; never skips. digest_body is required whenever a digest is written."
        },
        "digest_body": {
            "type": "string",
            "description": "Digest Markdown. Required when digest is always, or auto and the document meets Host digest rules."
        }
    });
    if include_source_path {
        props["source_path"] = json!({
            "type": "string",
            "description": "Absolute path to an allow-listed Markdown file. Host reads this as the new raw body; destination is the existing document for id. Never send document text."
        });
    } else {
        props["content"] = json!({
            "type": "string",
            "description": "Replacement Markdown body. Host writes this text; do not send source_path."
        });
    }
    props
}
