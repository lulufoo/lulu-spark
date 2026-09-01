//! Per-channel overlays on the slot tool table. Same MCP name, different Sidecar path.

use serde_json::json;

use super::routes::{create_note_properties, object_schema, route};
use super::types::{HttpMethod, SlotToolTable, ToolRoute};

fn create_note_content_route() -> ToolRoute {
    route(
        "create_note",
        "Create a note from Markdown content. Send title and body; do not send source_path. Path is {project}/{theme}/{created_at}-{6-char}.md. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call.",
        HttpMethod::Post,
        "/api/create-note-content",
        object_schema(create_note_properties(false), &["content", "title", "digest"]),
        false,
        false,
    )
}

fn get_note_path_route() -> ToolRoute {
    route(
        "get_note_content_by_id",
        "Stage one note onto this Chat and return the staged document id (F1, F2, …). Input id is the archive entry id. Does not return file path or body. The user reads the file from the Stage list. Use get_note_digest_by_id for a digest.",
        HttpMethod::Post,
        "/api/note-path",
        object_schema(
            json!({
                "id": {
                    "type": "string",
                    "description": "Archive entry id (32-char hex)."
                }
            }),
            &["id"],
        ),
        true,
        false,
    )
}

pub fn apply_channel_routes(mut table: SlotToolTable, channel: &str) -> SlotToolTable {
    // Product hard-gate: delete_note is workbench-only (not Settings-checkbox soft hide).
    if channel != "workbench" {
        table.tools.retain(|t| t.name != "delete_note");
    }
    for tool in &mut table.tools {
        if channel == "mobile" && tool.name == "create_note" {
            *tool = create_note_content_route();
        }
        if channel == "workbench" && tool.name == "get_note_content_by_id" {
            *tool = get_note_path_route();
        }
    }
    table
}

pub fn build_channel_tool_table(slot: &str, channel: &str) -> Option<SlotToolTable> {
    Some(apply_channel_routes(super::routes::build_slot_tool_table(slot)?, channel))
}
