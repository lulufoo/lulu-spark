use crate::mcp_host::build_channel_tool_table;
use crate::mcp_host::invoke_eq;
use crate::mcp_host::catalog::groups::document::{
    self, create_doc_from_content, create_doc_from_source, update_doc_from_content,
    update_doc_from_source,
};
use crate::mcp_host::catalog::groups::{global, project, read_later, workspace, ALL_GROUP_IDS};
use crate::mcp_host::catalog::{build, build_routes_for_channel, catalog_groups, group_for_migrated_api};
use crate::services::docs as docs_service;
use crate::services::docs::{DocCategory, save_doc_categories};
use crate::services::settings::mcp_catalog::GroupedEnabledCatalog;
use crate::test_support::TestSandbox;

const WORKSPACE_APIS: &[&str] = &["list_workspaces", "create_workspace", "rename_workspace"];

const PROJECT_APIS: &[&str] = &["list_projects", "create_project", "update_project"];

const DOCUMENT_APIS: &[&str] = &[
    "get_doc_digest",
    "get_doc_content",
    "get_doc_file",
    "list_doc_categories",
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

const GLOBAL_APIS: &[&str] = &["search"];

const READ_LATER_APIS: &[&str] = &["get_read_later"];

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
fn every_group_registry_covers_all_snapshot_tools() {
    let cases: [(&str, Vec<_>, &[&str], fn(&str) -> bool); 5] = [
        ("global", global::catalog_snapshot_routes(), GLOBAL_APIS, global::contains),
        ("workspace", workspace::catalog_snapshot_routes(), WORKSPACE_APIS, workspace::contains),
        ("project", project::catalog_snapshot_routes(), PROJECT_APIS, project::contains),
        ("document", document::catalog_snapshot_routes(), DOCUMENT_APIS, document::contains),
        ("read_later", read_later::catalog_snapshot_routes(), READ_LATER_APIS, read_later::contains),
    ];
    for (group, snapshot, apis, contains) in cases {
        assert_eq!(snapshot.len(), apis.len(), "{group}");
        for api in apis {
            assert!(contains(api), "missing {group} registry entry for {api}");
        }
    }
}

#[test]
fn catalog_groups_are_the_five_groups_in_settings_order() {
    let ids: Vec<&str> = catalog_groups().into_iter().map(|(id, _)| id).collect();
    assert_eq!(
        ids,
        vec!["global", "workspace", "project", "document", "read_later"]
    );
    assert_eq!(ALL_GROUP_IDS, ids.as_slice());
}

#[test]
fn no_tool_deletes_disables_or_enables_a_workspace_or_project() {
    for (group, routes) in catalog_groups() {
        for route in routes {
            let name = route.name.as_str();
            let on_container = name.contains("workspace") || name.contains("project");
            assert!(
                !(on_container && ["delete", "disable", "enable"].iter().any(|verb| name.starts_with(verb))),
                "{group} offers {name}: a person does that in Settings"
            );
        }
    }
}

#[test]
fn every_tool_belongs_to_exactly_one_group() {
    let mut seen = std::collections::HashSet::new();
    for (group, routes) in catalog_groups() {
        for route in routes {
            assert!(
                seen.insert(route.name.clone()),
                "{} appears in more than one group (again in {group})",
                route.name
            );
        }
    }
    let expected = GLOBAL_APIS.len()
        + WORKSPACE_APIS.len()
        + PROJECT_APIS.len()
        + DOCUMENT_APIS.len()
        + READ_LATER_APIS.len();
    assert_eq!(seen.len(), expected);
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
fn factory_builds_all_apis_with_channel_parity() {
    assert_channel_parity("global", GLOBAL_APIS);
    assert_channel_parity("workspace", WORKSPACE_APIS);
    assert_channel_parity("project", PROJECT_APIS);
    assert_channel_parity("document", DOCUMENT_APIS);
    assert_channel_parity("read_later", READ_LATER_APIS);
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
    for (group, apis) in [
        ("workspace", WORKSPACE_APIS),
        ("project", PROJECT_APIS),
        ("document", DOCUMENT_APIS),
        ("read_later", READ_LATER_APIS),
        ("global", GLOBAL_APIS),
    ] {
        for api in apis {
            assert_eq!(group_for_migrated_api(api), Some(group), "{api}");
        }
    }
    for api in TODO_APIS {
        assert_eq!(group_for_migrated_api(api), None, "{api}");
    }
    assert_eq!(group_for_migrated_api("not_a_tool"), None);
    for api in RETIRED_APIS {
        assert_eq!(group_for_migrated_api(api), None, "{api} is retired");
    }
}

#[test]
fn runtime_builds_enabled_routes_from_factory_only() {
    let enabled = GroupedEnabledCatalog {
        project: vec!["list_projects".into()],
        document: vec!["create_doc".into(), "get_doc_file".into()],
        global: vec!["search".into()],
        ..Default::default()
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
fn workspace_and_project_tools_take_only_the_id_and_name_keys() {
    let keys = |group: &str, tool: &str| -> Vec<String> {
        let route = build(group, tool, "spark").unwrap_or_else(|| panic!("{tool}"));
        let mut keys: Vec<String> = route.input_schema["properties"].as_object().expect("properties").keys().cloned().collect();
        keys.sort();
        keys
    };
    assert_eq!(keys("workspace", "create_workspace"), vec!["description", "view_mode", "workspace_name"]);
    assert_eq!(keys("workspace", "rename_workspace"), vec!["workspace_id", "workspace_name"]);
    assert_eq!(keys("project", "list_projects"), vec!["workspace_id"]);
    assert_eq!(keys("project", "create_project"), vec!["description", "project_name", "workspace_id"]);
    assert_eq!(keys("project", "update_project"), vec!["description", "project_id", "project_name"]);
}

#[test]
fn mobile_create_doc_uses_content_invoke() {
    let route = build("document", "create_doc", "mobile").expect("mobile create_doc");
    assert!(invoke_eq(route.invoke, create_doc_from_content));
    assert!(!invoke_eq(route.invoke, create_doc_from_source));
    assert!(route.input_schema["properties"].get("asset_paths").is_none());
    assert!(route.input_schema["properties"].get("workspace_id").is_some());
}

#[test]
fn desktop_create_doc_schema_includes_asset_paths() {
    let route = build("document", "create_doc", "spark").expect("spark create_doc");
    assert!(invoke_eq(route.invoke, create_doc_from_source));
    assert!(route.input_schema["properties"].get("asset_paths").is_some());
    assert!(route.input_schema["properties"].get("source_path").is_some());
    assert!(route.input_schema["properties"].get("workspace_id").is_some());
    assert!(route.input_schema["properties"].get("project_id").is_some());
    assert!(route.input_schema["properties"].get("workspace").is_none());
    assert!(route.input_schema["properties"].get("project").is_none());
}

#[test]
fn mobile_update_doc_uses_content_invoke() {
    let route = build("document", "update_doc", "mobile").expect("mobile update_doc");
    assert!(invoke_eq(route.invoke, update_doc_from_content));
    assert!(!invoke_eq(route.invoke, update_doc_from_source));
    assert!(route.input_schema["properties"].get("workspace_id").is_none());
}

#[test]
fn spark_update_doc_uses_source_invoke() {
    let route = build("document", "update_doc", "spark").expect("spark update_doc");
    assert!(invoke_eq(route.invoke, update_doc_from_source));
    assert!(!invoke_eq(route.invoke, update_doc_from_content));
    assert!(route.input_schema["properties"].get("workspace_id").is_none());
}

#[test]
fn create_and_update_doc_descriptions_omit_internal_paths() {
    for (api, channel) in [
        ("create_doc", "spark"),
        ("create_doc", "mobile"),
        ("update_doc", "spark"),
        ("update_doc", "mobile"),
    ] {
        let route = build("document", api, channel).expect(api);
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
        assert!(build("document", "get_doc_content", channel).is_none(), "{channel}");
    }
    let mobile = build("document", "get_doc_content", "mobile").expect("mobile");
    assert!(invoke_eq(mobile.invoke, docs_service::get_path));
    assert!(mobile.description.contains("absolute file path"));
    assert!(!mobile.description.contains("Stage"));
}

#[test]
fn get_doc_file_is_desktop_only_and_requires_dest_dir() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        let route = build("document", "get_doc_file", channel).expect(channel);
        assert!(invoke_eq(route.invoke, docs_service::export_file), "{channel}");
        assert_eq!(route.input_schema["required"], serde_json::json!(["id", "dest_dir"]));
        assert!(route.input_schema["properties"]["dest_dir"]["description"]
            .as_str()
            .unwrap()
            .contains("read and write"));
        assert!(route.input_schema["properties"].get("workspace_id").is_some());
    }
    assert!(build("document", "get_doc_file", "mobile").is_none());
}

#[test]
fn delete_doc_only_available_on_spark() {
    assert!(build("document", "delete_doc", "spark").is_some());
    assert!(build("document", "delete_doc", "cursor_ide").is_none());
    assert!(build("document", "delete_doc", "mobile").is_none());
}

#[test]
fn factory_unknown_group_returns_none() {
    assert!(build("unknown", "list_projects", "spark").is_none());
    assert!(build("notes", "create_doc", "spark").is_none());
    assert!(build("knowledge", "get_doc_file", "spark").is_none());
}

#[test]
fn get_read_later_is_spark_only() {
    assert!(build("read_later", "get_read_later", "spark").is_some());
    for channel in ["cursor", "cursor_ide", "codex", "claude", "mobile"] {
        assert!(
            build("read_later", "get_read_later", channel).is_none(),
            "{channel}"
        );
    }
}

#[test]
fn search_points_each_channel_at_its_own_follow_up_tools() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude"] {
        let route = build("global", "search", channel).expect(channel);
        assert!(route.description.contains("own library"), "{channel}");
        assert!(route.description.contains("workspace_id"), "{channel}");
        assert!(!route.description.to_lowercase().contains("read later"), "{channel}");
        assert!(!route.description.contains("read-later"), "{channel}");
        assert!(route.description.contains("get_doc_file"), "{channel}");
        assert!(!route.description.contains("get_doc_content"), "{channel}");
        assert!(route.input_schema["properties"].get("workspace_id").is_some(), "{channel}");
    }
    let mobile = build("global", "search", "mobile").expect("mobile");
    assert!(mobile.description.contains("own library"));
    assert!(!mobile.description.to_lowercase().contains("read later"));
    assert!(mobile.description.contains("get_doc_content"));
    assert!(!mobile.description.contains("get_doc_file"));
}

#[test]
fn list_doc_categories_is_on_every_channel_and_read_only() {
    for channel in ["spark", "cursor", "cursor_ide", "codex", "claude", "mobile"] {
        let route = build("document", "list_doc_categories", channel).expect(channel);
        assert!(route.read_only, "{channel}");
        assert!(!route.destructive, "{channel}");
        assert_eq!(route.input_schema["required"], serde_json::Value::Null);
        assert_eq!(route.input_schema["properties"], serde_json::json!({}));
    }
}

#[test]
fn list_doc_categories_returns_exposed_items_without_jot() {
    let _sandbox = TestSandbox::new();
    let route = build("document", "list_doc_categories", "spark").expect("list");
    let value = (route.invoke)(&serde_json::json!({}));
    assert_eq!(value["ok"], serde_json::json!(true));
    let ids: Vec<&str> = value["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .map(|c| {
            assert!(c.get("exposed").is_none(), "{c}");
            c["id"].as_str().expect("id")
        })
        .collect();
    assert_eq!(ids, vec!["summary", "dialogue", "video", "article"]);
    assert!(!ids.contains(&"jot"));
    assert!(!ids.contains(&"theme-line"));
    assert!(!ids.contains(&"transcript"));
}

#[test]
fn create_doc_source_type_enum_follows_exposed_list() {
    let _sandbox = TestSandbox::new();
    let route = build("document", "create_doc", "spark").expect("create_doc");
    assert_eq!(
        route.input_schema["properties"]["source_type"]["enum"],
        serde_json::json!(["summary", "dialogue", "video", "article"])
    );

    save_doc_categories(&[
        DocCategory {
            id: "summary".into(),
            label: "Summary".into(),
            exposed: true,
        },
        DocCategory {
            id: "jot".into(),
            label: "Jot".into(),
            exposed: true,
        },
    ])
    .expect("expose jot");
    let listed = (build("document", "list_doc_categories", "spark")
        .expect("list")
        .invoke)(&serde_json::json!({}));
    let listed_ids: Vec<&str> = listed["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .map(|c| c["id"].as_str().expect("id"))
        .collect();
    assert_eq!(listed_ids, vec!["summary", "jot"]);
    let again = build("document", "create_doc", "spark").expect("create_doc");
    assert_eq!(
        again.input_schema["properties"]["source_type"]["enum"],
        serde_json::json!(["summary", "jot"])
    );
}

#[test]
fn mcp_create_doc_requires_exposed_source_type() {
    let _sandbox = TestSandbox::new();
    let invoke = build("document", "create_doc", "mobile")
        .expect("mobile create")
        .invoke;
    let missing = invoke(&serde_json::json!({
        "content": "# 标题\n\n正文\n",
        "title": "T",
        "digest": "never",
    }));
    assert_eq!(missing["_status"], serde_json::json!(400), "{missing}");
    assert!(
        missing["error"].as_str().unwrap_or("").contains("source_type"),
        "{missing}"
    );
    for bad in ["jot", "theme-line", "transcript"] {
        let rejected = invoke(&serde_json::json!({
            "content": "# 标题\n\n正文\n",
            "title": "T",
            "digest": "never",
            "source_type": bad,
        }));
        assert_eq!(rejected["_status"], serde_json::json!(400), "{bad}: {rejected}");
    }
    let ok = invoke(&serde_json::json!({
        "workspace_id": crate::test_support::default_workspace().id,
        "content": "# 标题\n\n这是足够长的正文内容。\n",
        "title": "T",
        "digest": "never",
        "source_type": "dialogue",
    }));
    assert_eq!(ok["ok"], serde_json::json!(true), "{ok}");
}

#[test]
fn create_doc_schema_requires_source_type() {
    for channel in ["spark", "mobile"] {
        let route = build("document", "create_doc", channel).expect(channel);
        let required = route.input_schema["required"].as_array().expect("required");
        assert!(
            required.iter().any(|v| v.as_str() == Some("source_type")),
            "{channel}"
        );
        assert!(
            required.iter().any(|v| v.as_str() == Some("workspace_id")),
            "{channel}"
        );
    }
}

#[test]
fn catalog_has_no_write_tool_for_categories() {
    for (group, routes) in catalog_groups() {
        for route in routes {
            assert_ne!(route.name, "create_doc_categories", "{group}");
            assert_ne!(route.name, "update_doc_categories", "{group}");
            assert_ne!(route.name, "delete_doc_categories", "{group}");
            assert_ne!(route.name, "save_doc_categories", "{group}");
        }
    }
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

#[path = "naming_smoke.rs"]
mod naming_smoke;

#[path = "todo_group_removed.rs"]
mod todo_group_removed;

#[path = "todo_commands_removed.rs"]
mod todo_commands_removed;

#[path = "todo_service_removed.rs"]
mod todo_service_removed;

#[path = "retired_catalog.rs"]
mod retired_catalog;
