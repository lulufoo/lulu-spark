//! Build MCP tool routes by `(group, api_key, channel)`.

use super::groups::{knowledge, notes, todo};
use crate::mcp_host::ToolRoute;

pub fn build(group: &str, api: &str, channel: &str) -> Option<ToolRoute> {
    match group {
        notes::GROUP_ID => notes::build(api, channel),
        todo::GROUP_ID => todo::build(api, channel),
        knowledge::GROUP_ID => knowledge::build(api, channel),
        _ => None,
    }
}

pub fn group_for_migrated_api(api: &str) -> Option<&'static str> {
    if notes::contains(api) {
        Some(notes::GROUP_ID)
    } else if todo::contains(api) {
        Some(todo::GROUP_ID)
    } else if knowledge::contains(api) {
        Some(knowledge::GROUP_ID)
    } else {
        None
    }
}
