//! Shared JSON Schema fragments for notes MCP tools.

use serde_json::{json, Value};

pub fn source_type_schema() -> Value {
    json!({
        "type": "string",
        "enum": ["summary", "article", "theme-line", "dialogue", "transcript", "jot"],
        "description": "Optional source type; defaults to summary."
    })
}

pub fn create_note_properties(include_source_path: bool) -> Value {
    let mut props = json!({
        "title": {
            "type": "string",
            "description": "Required display title. May be Chinese."
        },
        "project": {
            "type": "string",
            "description": "Optional notes category folder (first path segment). Must exist in notes/categories.json. Defaults to inbox."
        },
        "theme": {
            "type": "string",
            "description": "Optional second path segment. Defaults to notes."
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
            "description": "Digest Markdown. Required when digest is always, or auto and the note meets Host digest rules."
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
            "description": "Optional translations. Prefer source_path. Host rejects stub or short zh."
        }
    });
    if include_source_path {
        props["source_path"] = json!({
            "type": "string",
            "description": "Absolute path to an allow-listed Markdown file."
        });
    } else {
        props["content"] = json!({
            "type": "string",
            "description": "Markdown note body. Host writes this text; do not send source_path."
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
