use crate::mcp_host::build_channel_tool_table;
use crate::mcp_host::catalog::groups::{notes, todo};
use crate::mcp_host::catalog::{build, build_routes_for_channel, group_for_migrated_api};
use crate::services::settings::mcp_catalog::GroupedEnabledCatalog;

const NOTES_APIS: &[&str] = &[
    "get_all_notes_catalog",
    "get_latest_digest_per_catalog",
    "get_notes_by_catalog",
    "get_note_digest_by_id",
    "get_note_content_by_id",
    "search_notes",
    "create_note",
    "delete_note",
    "list_notes_categories",
    "create_notes_category",
    "update_notes_category",
    "delete_notes_category",
];

const TODO_APIS: &[&str] = &[
    "create_todo_task",
    "update_todo_task",
    "list_todo_tasks",
    "list_todo_categories",
    "get_todo_task",
    "delete_todo_task",
    "add_todo_sub",
    "update_todo_sub",
    "delete_todo_sub",
    "complete_todo",
    "link_todo_archive",
    "add_todo_attachment",
    "list_todo_attachments",
    "get_todo_attachment",
    "update_todo_attachment",
];

#[test]
fn notes_registry_covers_all_snapshot_tools() {
    let snapshot = notes::catalog_snapshot_routes();
    assert_eq!(snapshot.len(), NOTES_APIS.len());
    for api in NOTES_APIS {
        assert!(notes::contains(api), "missing registry entry for {api}");
    }
}

#[test]
fn todo_registry_covers_all_snapshot_tools() {
    let snapshot = todo::catalog_snapshot_routes();
    assert_eq!(snapshot.len(), TODO_APIS.len());
    for api in TODO_APIS {
        assert!(todo::contains(api), "missing registry entry for {api}");
    }
}

#[test]
fn factory_builds_all_notes_apis_with_channel_parity() {
    for (slot, channel) in [
        ("workbench", "workbench"),
        ("cursor_ide", "cursor_ide"),
        ("workbench", "mobile"),
    ] {
        for api in NOTES_APIS {
            let built = build("notes", api, channel);
            let legacy = build_channel_tool_table(slot, channel)
                .and_then(|table| table.tools.into_iter().find(|r| r.name == *api));
            assert_eq!(
                built.is_some(),
                legacy.is_some(),
                "{api} availability mismatch on slot={slot} channel={channel}"
            );
            if let (Some(built), Some(legacy)) = (built, legacy) {
                assert_eq!(built.name, legacy.name, "{api} name on {channel}");
                assert_eq!(built.api_path, legacy.api_path, "{api} path on {channel}");
                assert_eq!(built.method, legacy.method, "{api} method on {channel}");
            }
        }
    }
}

#[test]
fn factory_builds_all_todo_apis_with_channel_parity() {
    for (slot, channel) in [
        ("workbench", "workbench"),
        ("cursor_ide", "cursor_ide"),
        ("workbench", "mobile"),
    ] {
        for api in TODO_APIS {
            let built = build("todo", api, channel);
            let legacy = build_channel_tool_table(slot, channel)
                .and_then(|table| table.tools.into_iter().find(|r| r.name == *api));
            assert_eq!(
                built.is_some(),
                legacy.is_some(),
                "{api} availability mismatch on slot={slot} channel={channel}"
            );
            if let (Some(built), Some(legacy)) = (built, legacy) {
                assert_eq!(built.name, legacy.name, "{api} name on {channel}");
                assert_eq!(built.api_path, legacy.api_path, "{api} path on {channel}");
                assert_eq!(built.method, legacy.method, "{api} method on {channel}");
            }
        }
    }
}

#[test]
fn group_for_migrated_api_maps_all_catalog_keys() {
    for api in NOTES_APIS {
        assert_eq!(group_for_migrated_api(api), Some("notes"), "{api}");
    }
    for api in TODO_APIS {
        assert_eq!(group_for_migrated_api(api), Some("todo"), "{api}");
    }
    assert_eq!(group_for_migrated_api("not_a_tool"), None);
}

#[test]
fn runtime_builds_enabled_routes_from_factory_only() {
    let enabled = GroupedEnabledCatalog {
        notes: vec!["get_all_notes_catalog".into(), "create_note".into()],
        todo: vec!["list_todo_tasks".into(), "create_todo_task".into()],
    };
    let routes = build_routes_for_channel("workbench", "workbench", &enabled);
    let names: Vec<_> = routes.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"get_all_notes_catalog"));
    assert!(names.contains(&"create_note"));
    assert!(names.contains(&"list_todo_tasks"));
    assert!(names.contains(&"create_todo_task"));
}

#[test]
fn mobile_create_note_uses_content_api() {
    let route = build("notes", "create_note", "mobile").expect("mobile create_note");
    assert_eq!(route.api_path, "/api/create-note-content");
}

#[test]
fn workbench_get_note_content_stages_via_note_path() {
    let route = build("notes", "get_note_content_by_id", "workbench").expect("workbench");
    assert_eq!(route.api_path, "/api/note-path");
}

#[test]
fn delete_note_only_available_on_workbench() {
    assert!(build("notes", "delete_note", "workbench").is_some());
    assert!(build("notes", "delete_note", "cursor_ide").is_none());
    assert!(build("notes", "delete_note", "mobile").is_none());
}

#[test]
fn factory_unknown_group_returns_none() {
    assert!(build("unknown", "get_all_notes_catalog", "workbench").is_none());
}
