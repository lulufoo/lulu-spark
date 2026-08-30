//! Per-channel overlays on the slot tool table. Same MCP name, different Sidecar path.

use super::routes::{create_note_properties, object_schema, route};
use super::types::{HttpMethod, SlotToolTable, ToolRoute};

fn create_note_content_route() -> ToolRoute {
    route(
        "create_note",
        "Create a note from Markdown content. Send title and body; do not send source_path. Path is {project}/{theme}/{created_at}-{6-char}.md.",
        HttpMethod::Post,
        "/api/create-note-content",
        object_schema(create_note_properties(false), &["content", "title"]),
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
    Some(apply_channel_routes(super::routes::build_slot_tool_table(slot)?, channel))
}
