//! Build MCP tool routes by `(group, api_key, channel)`.

use super::groups::{document, global, project, read_later, workspace};
use crate::mcp_host::ToolRoute;

pub fn build(group: &str, api: &str, channel: &str) -> Option<ToolRoute> {
    match group {
        global::GROUP_ID => global::build(api, channel),
        workspace::GROUP_ID => workspace::build(api, channel),
        project::GROUP_ID => project::build(api, channel),
        document::GROUP_ID => document::build(api, channel),
        read_later::GROUP_ID => read_later::build(api, channel),
        _ => None,
    }
}

pub fn group_for_migrated_api(api: &str) -> Option<&'static str> {
    if global::contains(api) {
        Some(global::GROUP_ID)
    } else if workspace::contains(api) {
        Some(workspace::GROUP_ID)
    } else if project::contains(api) {
        Some(project::GROUP_ID)
    } else if document::contains(api) {
        Some(document::GROUP_ID)
    } else if read_later::contains(api) {
        Some(read_later::GROUP_ID)
    } else {
        None
    }
}
