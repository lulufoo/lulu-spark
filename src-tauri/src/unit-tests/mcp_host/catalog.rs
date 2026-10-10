use crate::mcp_host::build_channel_tool_table;
use crate::mcp_host::invoke_eq;
use crate::mcp_host::catalog::groups::global;
use crate::mcp_host::catalog::groups::knowledge::{self, knowledge_file_invoke};
use crate::mcp_host::catalog::groups::notes::{
    self, create_note_from_content, create_note_from_source, note_file_invoke, note_path_invoke,
    update_note_from_content, update_note_from_source,
};
use crate::mcp_host::catalog::{build, build_routes_for_channel, catalog_groups, group_for_migrated_api};
use crate::services::settings::mcp_catalog::GroupedEnabledCatalog;

const NOTES_APIS: &[&str] = &[
    "get_all_notes_catalog",
    "get_note_digest",
    "get_note_content",
    "get_note_file",
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
    "get_knowledge_content",
    "get_knowledge_file",
];

const GLOBAL_APIS: &[&str] = &["search_document", "get_read_later"];

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
        ("spark", "spark"),
        ("cursor_ide", "cursor_ide"),
        ("spark", "mobile"),
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
        ("spark", "spark"),
        ("cursor_ide", "cursor_ide"),
        ("spark", "mobile"),
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
        ("spark", "spark"),
        ("cursor_ide", "cursor_ide"),
        ("spark", "mobile"),
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
fn factory_does_not_build_todo_apis() {
    for channel in ["spark", "cursor_ide", "mobile"] {
        for api in TODO_APIS {
            assert!(
                build("todo", api, channel).is_none(),
                "{api} must not build on channel={channel}"
            );
        }
    }
}

#[test]
fn group_for_migrated_api_maps_all_catalog_keys() {
    for api in NOTES_APIS {
        assert_eq!(group_for_migrated_api(api), Some("notes"), "{api}");
    }
    for api in TODO_APIS {
        assert_eq!(group_for_migrated_api(api), None, "{api}");
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
        knowledge: vec!["get_knowledge_file".into()],
        global: vec!["search_document".into()],
        ..GroupedEnabledCatalog::default()
    };
    let routes = build_routes_for_channel("spark", "spark", &enabled);
    let names: Vec<_> = routes.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"get_all_notes_catalog"));
    assert!(names.contains(&"create_note"));
    assert!(!names.contains(&"list_todo_tasks"));
    assert!(!names.contains(&"create_todo_task"));
    assert!(names.contains(&"get_knowledge_file"));
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
    let route = build("notes", "create_note", "spark").expect("spark create_note");
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
fn spark_update_note_uses_source_invoke() {
    let route = build("notes", "update_note", "spark").expect("spark update_note");
    assert!(invoke_eq(route.invoke, update_note_from_source));
    assert!(!invoke_eq(route.invoke, update_note_from_content));
}

#[test]
fn get_note_content_is_mobile_only() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        assert!(build("notes", "get_note_content", channel).is_none(), "{channel}");
    }
    let mobile = build("notes", "get_note_content", "mobile").expect("mobile");
    assert!(invoke_eq(mobile.invoke, note_path_invoke));
    assert!(mobile.description.contains("absolute file path"));
    assert!(!mobile.description.contains("Stage"));
}

#[test]
fn get_note_file_is_desktop_only_and_requires_dest_dir() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        let route = build("notes", "get_note_file", channel).expect(channel);
        assert!(invoke_eq(route.invoke, note_file_invoke), "{channel}");
        assert_eq!(route.input_schema["required"], serde_json::json!(["id", "dest_dir"]));
        assert!(route.input_schema["properties"]["dest_dir"]["description"]
            .as_str()
            .unwrap()
            .contains("read and write"));
    }
    assert!(build("notes", "get_note_file", "mobile").is_none());
}

#[test]
fn get_knowledge_file_is_desktop_only_and_requires_dest_dir() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        let route = build("knowledge", "get_knowledge_file", channel).expect(channel);
        assert!(invoke_eq(route.invoke, knowledge_file_invoke), "{channel}");
        assert_eq!(route.input_schema["required"], serde_json::json!(["id", "dest_dir"]));
    }
    assert!(build("knowledge", "get_knowledge_file", "mobile").is_none());
}

#[test]
fn delete_note_only_available_on_spark() {
    assert!(build("notes", "delete_note", "spark").is_some());
    assert!(build("notes", "delete_note", "cursor_ide").is_none());
    assert!(build("notes", "delete_note", "mobile").is_none());
}

#[test]
fn factory_unknown_group_returns_none() {
    assert!(build("unknown", "get_all_notes_catalog", "spark").is_none());
}

#[test]
fn get_read_later_is_spark_only() {
    assert!(build("global", "get_read_later", "spark").is_some());
    for channel in ["cursor", "cursor_ide", "codex", "claude", "mobile"] {
        assert!(
            build("global", "get_read_later", channel).is_none(),
            "{channel}"
        );
    }
}

#[test]
fn get_knowledge_content_is_mobile_only() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        assert!(build("knowledge", "get_knowledge_content", channel).is_none(), "{channel}");
    }
    let mobile = build("knowledge", "get_knowledge_content", "mobile").expect("mobile");
    assert!(mobile.description.contains("absolute file path"));
    assert!(!mobile.description.contains("Stage"));
}

#[test]
fn search_document_points_each_channel_at_its_own_follow_up_tools() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        let route = build("global", "search_document", channel).expect(channel);
        assert!(route.description.contains("get_note_file"), "{channel}");
        assert!(route.description.contains("get_knowledge_file"), "{channel}");
        assert!(!route.description.contains("get_note_content"), "{channel}");
        assert!(!route.description.contains("get_knowledge_content"), "{channel}");
    }
    let mobile = build("global", "search_document", "mobile").expect("mobile");
    assert!(mobile.description.contains("get_note_content"));
    assert!(mobile.description.contains("get_knowledge_content"));
    assert!(!mobile.description.contains("get_note_file"));
}

#[test]
fn settings_snapshot_keeps_mobile_only_tools() {
    let names: Vec<String> = catalog_groups()
        .into_iter()
        .flat_map(|(_, tools)| tools.into_iter().map(|t| t.name))
        .collect();
    assert!(names.contains(&"get_note_content".to_string()));
    assert!(names.contains(&"get_knowledge_content".to_string()));
}

#[path = "produce_wiring.rs"]
mod produce_wiring;

#[path = "todo_group_removed.rs"]
mod todo_group_removed;

#[path = "todo_commands_removed.rs"]
mod todo_commands_removed;

#[path = "todo_service_removed.rs"]
mod todo_service_removed;

#[path = "retired_catalog.rs"]
mod retired_catalog;
