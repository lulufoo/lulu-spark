use crate::mcp_host::build_channel_tool_table;
use crate::mcp_host::invoke_eq;
use crate::mcp_host::catalog::groups::docs::{
    self, create_doc_from_content, create_doc_from_source, update_doc_from_content,
    update_doc_from_source,
};
use crate::mcp_host::catalog::groups::global;
use crate::mcp_host::catalog::{build, build_routes_for_channel, catalog_groups, group_for_migrated_api};
use crate::services::docs as docs_service;
use crate::services::settings::mcp_catalog::GroupedEnabledCatalog;

const DOCS_APIS: &[&str] = &[
    "list_workspaces",
    "create_workspace",
    "rename_workspace",
    "list_projects",
    "create_project",
    "update_project",
    "get_doc_digest",
    "get_doc_content",
    "get_doc_file",
    "create_doc",
    "update_doc",
    "delete_doc",
];

const RETIRED_APIS: &[&str] = &[
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
    "list_knowledge_categories",
    "get_knowledge_content",
    "get_knowledge_file",
    "search_notes",
    "search_knowledge",
    "search_document",
];

const GLOBAL_APIS: &[&str] = &["search", "get_read_later"];

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
fn docs_registry_covers_all_snapshot_tools() {
    let snapshot = docs::catalog_snapshot_routes();
    assert_eq!(snapshot.len(), DOCS_APIS.len());
    for api in DOCS_APIS {
        assert!(docs::contains(api), "missing registry entry for {api}");
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

fn assert_channel_parity(group: &str, apis: &[&str]) {
    for (slot, channel) in [
        ("spark", "spark"),
        ("cursor_ide", "cursor_ide"),
        ("spark", "mobile"),
    ] {
        for api in apis {
            let built = build(group, api, channel);
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
fn factory_builds_all_docs_apis_with_channel_parity() {
    assert_channel_parity("docs", DOCS_APIS);
}

#[test]
fn factory_builds_all_global_apis_with_channel_parity() {
    assert_channel_parity("global", GLOBAL_APIS);
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
    for api in DOCS_APIS {
        assert_eq!(group_for_migrated_api(api), Some("docs"), "{api}");
    }
    for api in TODO_APIS {
        assert_eq!(group_for_migrated_api(api), None, "{api}");
    }
    for api in GLOBAL_APIS {
        assert_eq!(group_for_migrated_api(api), Some("global"), "{api}");
    }
    assert_eq!(group_for_migrated_api("not_a_tool"), None);
    for api in RETIRED_APIS {
        assert_eq!(group_for_migrated_api(api), None, "{api} is retired");
    }
}

#[test]
fn runtime_builds_enabled_routes_from_factory_only() {
    let enabled = GroupedEnabledCatalog {
        docs: vec!["list_projects".into(), "create_doc".into(), "get_doc_file".into()],
        global: vec!["search".into()],
    };
    let routes = build_routes_for_channel("spark", "spark", &enabled);
    let names: Vec<_> = routes.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"list_projects"));
    assert!(names.contains(&"create_doc"));
    assert!(!names.contains(&"list_todo_tasks"));
    assert!(!names.contains(&"create_todo_task"));
    assert!(names.contains(&"get_doc_file"));
    assert!(names.contains(&"search"));
    assert!(!names.contains(&"create_note"));
}

#[test]
fn mobile_create_doc_uses_content_invoke() {
    let route = build("docs", "create_doc", "mobile").expect("mobile create_doc");
    assert!(invoke_eq(route.invoke, create_doc_from_content));
    assert!(!invoke_eq(route.invoke, create_doc_from_source));
    assert!(route.input_schema["properties"].get("asset_paths").is_none());
    assert!(route.input_schema["properties"].get("workspace").is_some());
}

#[test]
fn desktop_create_doc_schema_includes_asset_paths() {
    let route = build("docs", "create_doc", "spark").expect("spark create_doc");
    assert!(invoke_eq(route.invoke, create_doc_from_source));
    assert!(route.input_schema["properties"].get("asset_paths").is_some());
    assert!(route.input_schema["properties"].get("source_path").is_some());
    assert!(route.input_schema["properties"].get("workspace").is_some());
    assert!(route.input_schema["properties"].get("project").is_some());
}

#[test]
fn mobile_update_doc_uses_content_invoke() {
    let route = build("docs", "update_doc", "mobile").expect("mobile update_doc");
    assert!(invoke_eq(route.invoke, update_doc_from_content));
    assert!(!invoke_eq(route.invoke, update_doc_from_source));
}

#[test]
fn spark_update_doc_uses_source_invoke() {
    let route = build("docs", "update_doc", "spark").expect("spark update_doc");
    assert!(invoke_eq(route.invoke, update_doc_from_source));
    assert!(!invoke_eq(route.invoke, update_doc_from_content));
}

#[test]
fn create_and_update_doc_descriptions_omit_internal_paths() {
    for (api, channel) in [
        ("create_doc", "spark"),
        ("create_doc", "mobile"),
        ("update_doc", "spark"),
        ("update_doc", "mobile"),
    ] {
        let route = build("docs", api, channel).expect(api);
        assert!(
            !route.description.contains("common_path"),
            "{api} {channel}: {}",
            route.description
        );
        assert!(
            !route.description.contains("{project}/{theme}"),
            "{api} {channel}: {}",
            route.description
        );
        assert!(
            route.description.contains("{ok, id"),
            "{api} {channel}: {}",
            route.description
        );
    }
}

#[test]
fn get_doc_content_is_mobile_only() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        assert!(build("docs", "get_doc_content", channel).is_none(), "{channel}");
    }
    let mobile = build("docs", "get_doc_content", "mobile").expect("mobile");
    assert!(invoke_eq(mobile.invoke, docs_service::get_path));
    assert!(mobile.description.contains("absolute file path"));
    assert!(!mobile.description.contains("Stage"));
}

#[test]
fn get_doc_file_is_desktop_only_and_requires_dest_dir() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        let route = build("docs", "get_doc_file", channel).expect(channel);
        assert!(invoke_eq(route.invoke, docs_service::export_file), "{channel}");
        assert_eq!(route.input_schema["required"], serde_json::json!(["id", "dest_dir"]));
        assert!(route.input_schema["properties"]["dest_dir"]["description"]
            .as_str()
            .unwrap()
            .contains("read and write"));
        assert!(route.input_schema["properties"].get("workspace").is_some());
    }
    assert!(build("docs", "get_doc_file", "mobile").is_none());
}

#[test]
fn delete_doc_only_available_on_spark() {
    assert!(build("docs", "delete_doc", "spark").is_some());
    assert!(build("docs", "delete_doc", "cursor_ide").is_none());
    assert!(build("docs", "delete_doc", "mobile").is_none());
}

#[test]
fn factory_unknown_group_returns_none() {
    assert!(build("unknown", "list_projects", "spark").is_none());
    assert!(build("notes", "create_doc", "spark").is_none());
    assert!(build("knowledge", "get_doc_file", "spark").is_none());
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
fn search_points_each_channel_at_its_own_follow_up_tools() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        let route = build("global", "search", channel).expect(channel);
        assert!(route.description.contains("own library"), "{channel}");
        assert!(route.description.contains("workspace"), "{channel}");
        assert!(!route.description.to_lowercase().contains("read later"), "{channel}");
        assert!(!route.description.contains("read-later"), "{channel}");
        assert!(route.description.contains("get_doc_file"), "{channel}");
        assert!(!route.description.contains("get_doc_content"), "{channel}");
        assert!(route.input_schema["properties"].get("workspace").is_some(), "{channel}");
    }
    let mobile = build("global", "search", "mobile").expect("mobile");
    assert!(mobile.description.contains("own library"));
    assert!(!mobile.description.to_lowercase().contains("read later"));
    assert!(mobile.description.contains("get_doc_content"));
    assert!(!mobile.description.contains("get_doc_file"));
}

#[test]
fn settings_snapshot_keeps_mobile_only_tools() {
    let names: Vec<String> = catalog_groups()
        .into_iter()
        .flat_map(|(_, tools)| tools.into_iter().map(|t| t.name))
        .collect();
    assert!(names.contains(&"get_doc_content".to_string()));
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
