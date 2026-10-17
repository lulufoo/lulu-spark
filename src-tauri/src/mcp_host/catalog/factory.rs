//! Build MCP tool routes by `(group, api_key, channel)`.

use super::groups::{docs, global};
use crate::mcp_host::ToolRoute;

pub fn build(group: &str, api: &str, channel: &str) -> Option<ToolRoute> {
    match group {
        docs::GROUP_ID => docs::build(api, channel),
        global::GROUP_ID => global::build(api, channel),
        _ => None,
    }
}

pub fn group_for_migrated_api(api: &str) -> Option<&'static str> {
    if docs::contains(api) {
        Some(docs::GROUP_ID)
    } else if global::contains(api) {
        Some(global::GROUP_ID)
    } else {
        None
    }
}
