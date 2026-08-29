//! Per-channel overlays on the slot tool table. Same MCP name, different Sidecar path.

use serde_json::json;

use super::routes::{build_slot_tool_table, object_schema, route};
use super::types::{HttpMethod, SlotToolTable, ToolRoute};

fn create_note_content_route() -> ToolRoute {
    route(
        "create_note",
        "Create a note from Markdown content. Send the note body; do not send source_path.",
        HttpMethod::Post,
        "/api/create-note-content",
        object_schema(
            json!({
                "content": {
                    "type": "string",
                    "description": "Markdown note body. Host writes this text; do not send source_path."
                },
                "source_type": {
                    "type": "string",
                    "description": "Optional source type; defaults to summary."
                },
                "translations": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "lang": { "type": "string" },
                            "content": { "type": "string" }
                        },
                        "required": ["lang"],
                        "additionalProperties": false
                    },
                    "description": "Optional translations as text."
                },
                "master_task_id": {
                    "type": "string",
                    "description": "Optional Todo task id; must be paired with sub_task_id."
                },
                "sub_task_id": {
                    "type": "string",
                    "description": "Optional Todo subtask id; must be paired with master_task_id."
                }
            }),
            &["content"],
        ),
        false,
        false,
    )
}

pub fn apply_channel_routes(mut table: SlotToolTable, channel: &str) -> SlotToolTable {
    if channel == "mobile" {
        for tool in &mut table.tools {
            if tool.name == "create_note" {
                *tool = create_note_content_route();
            }
        }
    }
    table
}

pub fn build_channel_tool_table(slot: &str, channel: &str) -> Option<SlotToolTable> {
    Some(apply_channel_routes(build_slot_tool_table(slot)?, channel))
}
