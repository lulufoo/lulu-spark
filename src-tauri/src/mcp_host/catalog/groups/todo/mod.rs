//! Todo business group — API registry.

mod add_todo_attachment;
mod add_todo_sub;
mod complete_todo;
mod create_todo_task;
mod delete_todo_sub;
mod delete_todo_task;
mod get_todo_attachment;
mod get_todo_task;
mod link_todo_archive;
mod list_todo_attachments;
mod list_todo_categories;
mod list_todo_tasks;
mod update_todo_attachment;
mod update_todo_sub;
mod update_todo_task;

use crate::mcp_host::ToolRoute;

pub const GROUP_ID: &str = "todo";

const SNAPSHOT_CHANNEL: &str = "workbench";

type BuildFn = fn(&str) -> Option<ToolRoute>;

const REGISTRY: &[(&str, BuildFn)] = &[
    ("create_todo_task", create_todo_task::build as BuildFn),
    ("update_todo_task", update_todo_task::build as BuildFn),
    ("list_todo_tasks", list_todo_tasks::build as BuildFn),
    ("list_todo_categories", list_todo_categories::build as BuildFn),
    ("get_todo_task", get_todo_task::build as BuildFn),
    ("delete_todo_task", delete_todo_task::build as BuildFn),
    ("add_todo_sub", add_todo_sub::build as BuildFn),
    ("update_todo_sub", update_todo_sub::build as BuildFn),
    ("delete_todo_sub", delete_todo_sub::build as BuildFn),
    ("complete_todo", complete_todo::build as BuildFn),
    ("link_todo_archive", link_todo_archive::build as BuildFn),
    ("add_todo_attachment", add_todo_attachment::build as BuildFn),
    ("list_todo_attachments", list_todo_attachments::build as BuildFn),
    ("get_todo_attachment", get_todo_attachment::build as BuildFn),
    ("update_todo_attachment", update_todo_attachment::build as BuildFn),
];

pub fn contains(api: &str) -> bool {
    REGISTRY.iter().any(|(key, _)| *key == api)
}

pub fn build(api: &str, channel: &str) -> Option<ToolRoute> {
    REGISTRY
        .iter()
        .find(|(key, _)| *key == api)
        .and_then(|(_, build)| build(channel))
}

pub fn catalog_snapshot_routes() -> Vec<ToolRoute> {
    REGISTRY
        .iter()
        .map(|(key, build_fn)| {
            build_fn(SNAPSHOT_CHANNEL)
                .unwrap_or_else(|| panic!("catalog snapshot missing for {key}"))
        })
        .collect()
}

pub fn routes_for_channel(channel: &str) -> Vec<ToolRoute> {
    REGISTRY
        .iter()
        .filter_map(|(_, build_fn)| build_fn(channel))
        .collect()
}
