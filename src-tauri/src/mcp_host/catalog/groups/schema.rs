//! Shared JSON Schema fragments for the workspace, project and document MCP tools.

use serde_json::{json, Value};

pub fn source_type_schema() -> Value {
    let ids = crate::services::docs::categories::exposed_ids();
    json!({
        "type": "string",
        "enum": ids,
        "description": "Required. Must be an id from list_doc_categories."
    })
}

fn doc_workspace_id_schema() -> Value {
    json!({
        "type": "string",
        "description": "Optional workspace id from list_workspaces. Defaults to the Default workspace. A document id already names its workspace, so it is not needed when a document id is given."
    })
}

/// The `workspace_id` input of the tools that take a document id.
pub fn doc_workspace_id_property() -> Value {
    json!({ "workspace_id": doc_workspace_id_schema() })
}

/// The `workspace_id` input of the workspace and project tools.
pub fn workspace_id_property() -> Value {
    json!({
        "workspace_id": {
            "type": "string",
            "description": "Optional workspace id from list_workspaces. Defaults to the Default workspace."
        }
    })
}

const DIGEST_DESCRIPTION: &str = "Required. auto writes a digest when Host AD-0 applies; always writes; never skips. digest_body is required whenever a digest is written.";

const DIGEST_BODY_DESCRIPTION: &str = "One paragraph, 100-300 characters, in the same language as the raw. No heading, Overview, Created line, relative images, or digest nav.";

fn digest_property() -> Value {
    json!({
        "type": "string",
        "enum": ["auto", "always", "never"],
        "description": DIGEST_DESCRIPTION
    })
}

fn digest_body_property() -> Value {
    json!({
        "type": "string",
        "description": DIGEST_BODY_DESCRIPTION
    })
}

pub fn create_doc_properties(include_source_path: bool) -> Value {
    let mut props = json!({
        "title": {
            "type": "string",
            "description": "Required display title. May be Chinese."
        },
        "workspace_id": {
            "type": "string",
            "description": "Required workspace id from list_workspaces."
        },
        "project_id": {
            "type": "string",
            "description": "Optional project id from list_projects. Omit to use the workspace inbox."
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
        "digest": digest_property(),
        "digest_body": digest_body_property(),
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
        "id": {
            "type": "string",
            "description": "Document id from search or create_doc: 32 lowercase hex characters. The document belongs to exactly one workspace."
        },
        "digest": digest_property(),
        "digest_body": digest_body_property()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_body_desc(props: &Value) -> &str {
        props["digest_body"]["description"].as_str().unwrap()
    }

    #[test]
    fn digest_body_description_is_the_paragraph_standard() {
        let create = create_doc_properties(true);
        let update = update_doc_properties(true);
        let mobile_create = create_doc_properties(false);
        let mobile_update = update_doc_properties(false);
        for desc in [
            digest_body_desc(&create),
            digest_body_desc(&update),
            digest_body_desc(&mobile_create),
            digest_body_desc(&mobile_update),
        ] {
            assert!(desc.contains("same language as the raw"), "{desc}");
            assert!(desc.contains("100-300"), "{desc}");
            assert!(!desc.contains("# Title"), "{desc}");
            assert!(!desc.contains("## Overview"), "{desc}");
        }
        assert_eq!(digest_body_desc(&create), digest_body_desc(&update));
    }
}
