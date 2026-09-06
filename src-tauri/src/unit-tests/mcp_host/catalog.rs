use crate::mcp_host::build_channel_tool_table;
use crate::mcp_host::catalog::groups::global;
use crate::mcp_host::catalog::groups::knowledge;
use crate::mcp_host::catalog::groups::notes::{
    self, create_note_from_content, create_note_from_source, note_path_invoke,
    update_note_from_content, update_note_from_source,
};
use crate::mcp_host::catalog::groups::todo;
use crate::mcp_host::catalog::{build, build_routes_for_channel, group_for_migrated_api};
use crate::services::settings::mcp_catalog::GroupedEnabledCatalog;

const NOTES_APIS: &[&str] = &[
    "get_all_notes_catalog",
    "get_latest_digest_per_catalog",
    "get_notes_by_catalog",
    "get_note_digest_by_id",
    "get_note_content",
    "create_note",
    "update_note",
    "delete_note",
    "list_notes_categories",
    "create_notes_category",
    "update_notes_category",
    "delete_notes_category",
];

const KNOWLEDGE_APIS: &[&str] = &[
    "list_knowledge_categories",
    "list_knowledge_repos",
    "get_knowledge_content",
];

const GLOBAL_APIS: &[&str] = &["search_document"];

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

fn invoke_eq(a: fn(&serde_json::Value) -> serde_json::Value, b: fn(&serde_json::Value) -> serde_json::Value) -> bool {
    a == b
}

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
fn knowledge_registry_covers_all_snapshot_tools() {
    let snapshot = knowledge::catalog_snapshot_routes();
    assert_eq!(snapshot.len(), KNOWLEDGE_APIS.len());
    for api in KNOWLEDGE_APIS {
        assert!(knowledge::contains(api), "missing registry entry for {api}");
    }
}

#[test]
fn global_registry_covers_all_snapshot_tools() {
    let snapshot = global::catalog_snapshot_routes();
    assert_eq!(snapshot.len(), GLOBAL_APIS.len());
    for api in GLOBAL_APIS {
        assert!(global::contains(api), "missing registry entry for {api}");
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
                assert!(
                    invoke_eq(built.invoke, legacy.invoke),
                    "{api} invoke on {channel}"
                );
                assert_eq!(built.input_schema, legacy.input_schema, "{api} schema on {channel}");
            }
        }
    }
}

#[test]
fn factory_builds_all_knowledge_apis_with_channel_parity() {
    for (slot, channel) in [
        ("workbench", "workbench"),
        ("cursor_ide", "cursor_ide"),
        ("workbench", "mobile"),
    ] {
        for api in KNOWLEDGE_APIS {
            let built = build("knowledge", api, channel);
            let legacy = build_channel_tool_table(slot, channel)
                .and_then(|table| table.tools.into_iter().find(|r| r.name == *api));
            assert_eq!(
                built.is_some(),
                legacy.is_some(),
                "{api} availability mismatch on slot={slot} channel={channel}"
            );
            if let (Some(built), Some(legacy)) = (built, legacy) {
                assert_eq!(built.name, legacy.name, "{api} name on {channel}");
                assert!(
                    invoke_eq(built.invoke, legacy.invoke),
                    "{api} invoke on {channel}"
                );
            }
        }
    }
}

#[test]
fn factory_builds_all_global_apis_with_channel_parity() {
    for (slot, channel) in [
        ("workbench", "workbench"),
        ("cursor_ide", "cursor_ide"),
        ("workbench", "mobile"),
    ] {
        for api in GLOBAL_APIS {
            let built = build("global", api, channel);
            let legacy = build_channel_tool_table(slot, channel)
                .and_then(|table| table.tools.into_iter().find(|r| r.name == *api));
            assert_eq!(
                built.is_some(),
                legacy.is_some(),
                "{api} availability mismatch on slot={slot} channel={channel}"
            );
            if let (Some(built), Some(legacy)) = (built, legacy) {
                assert_eq!(built.name, legacy.name, "{api} name on {channel}");
                assert!(
                    invoke_eq(built.invoke, legacy.invoke),
                    "{api} invoke on {channel}"
                );
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
                assert!(
                    invoke_eq(built.invoke, legacy.invoke),
                    "{api} invoke on {channel}"
                );
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
    for api in KNOWLEDGE_APIS {
        assert_eq!(group_for_migrated_api(api), Some("knowledge"), "{api}");
    }
    for api in GLOBAL_APIS {
        assert_eq!(group_for_migrated_api(api), Some("global"), "{api}");
    }
    assert_eq!(group_for_migrated_api("not_a_tool"), None);
    assert_eq!(group_for_migrated_api("search_notes"), None);
    assert_eq!(group_for_migrated_api("search_knowledge"), None);
}

#[test]
fn runtime_builds_enabled_routes_from_factory_only() {
    let enabled = GroupedEnabledCatalog {
        notes: vec!["get_all_notes_catalog".into(), "create_note".into()],
        todo: vec!["list_todo_tasks".into(), "create_todo_task".into()],
        knowledge: vec!["get_knowledge_content".into()],
        global: vec!["search_document".into()],
    };
    let routes = build_routes_for_channel("workbench", "workbench", &enabled);
    let names: Vec<_> = routes.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"get_all_notes_catalog"));
    assert!(names.contains(&"create_note"));
    assert!(names.contains(&"list_todo_tasks"));
    assert!(names.contains(&"create_todo_task"));
    assert!(names.contains(&"get_knowledge_content"));
    assert!(names.contains(&"search_document"));
    assert!(!names.contains(&"search_notes"));
    assert!(!names.contains(&"search_knowledge"));
}

#[test]
fn mobile_create_note_uses_content_invoke() {
    let route = build("notes", "create_note", "mobile").expect("mobile create_note");
    assert!(invoke_eq(route.invoke, create_note_from_content));
    assert!(!invoke_eq(route.invoke, create_note_from_source));
    assert!(route.input_schema["properties"].get("asset_paths").is_none());
}

#[test]
fn desktop_create_note_schema_includes_asset_paths() {
    let route = build("notes", "create_note", "workbench").expect("workbench create_note");
    assert!(invoke_eq(route.invoke, create_note_from_source));
    assert!(route.input_schema["properties"].get("asset_paths").is_some());
    assert!(route.input_schema["properties"].get("source_path").is_some());
}

#[test]
fn mobile_update_note_uses_content_invoke() {
    let route = build("notes", "update_note", "mobile").expect("mobile update_note");
    assert!(invoke_eq(route.invoke, update_note_from_content));
    assert!(!invoke_eq(route.invoke, update_note_from_source));
}

#[test]
fn workbench_update_note_uses_source_invoke() {
    let route = build("notes", "update_note", "workbench").expect("workbench update_note");
    assert!(invoke_eq(route.invoke, update_note_from_source));
    assert!(!invoke_eq(route.invoke, update_note_from_content));
}

#[test]
fn workbench_get_note_content_stages_via_note_path() {
    let route = build("notes", "get_note_content", "workbench").expect("workbench");
    assert!(invoke_eq(route.invoke, note_path_invoke));
}

#[test]
fn cursor_ide_get_note_content_returns_path_not_body() {
    let workbench = build("notes", "get_note_content", "workbench").expect("wb");
    let ide = build("notes", "get_note_content", "cursor_ide").expect("ide");
    let mobile = build("notes", "get_note_content", "mobile").expect("mobile");
    assert!(invoke_eq(ide.invoke, note_path_invoke));
    assert!(invoke_eq(mobile.invoke, note_path_invoke));
    assert!(ide.description.contains("absolute file path"));
    assert!(!ide.description.contains("Stage"));
    assert!(workbench.description.contains("Stage"));
    assert!(!workbench.description.to_ascii_lowercase().contains("absolute path"));
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

#[test]
fn get_knowledge_content_channel_descriptions_split() {
    let workbench = build("knowledge", "get_knowledge_content", "workbench").expect("wb");
    let ide = build("knowledge", "get_knowledge_content", "cursor_ide").expect("ide");
    assert!(workbench.description.contains("Stage"));
    assert!(ide.description.contains("absolute file path"));
    assert!(!ide.description.contains("Stage"));
}

#[path = "produce_wiring.rs"]
mod produce_wiring;
