//! OTH-205: three catalog tools leave MCP.

use std::path::PathBuf;

use crate::mcp_host::catalog::groups::{knowledge, notes};
use crate::mcp_host::catalog::{build, group_for_migrated_api};

const RETIRED_NOTES_APIS: &[&str] = &[
    "get_latest_digest_per_catalog",
    "get_notes_by_catalog",
];
const RETIRED_KNOWLEDGE_APIS: &[&str] = &["list_knowledge_repos"];

#[test]
fn retired_catalog_tools_do_not_build() {
    for channel in ["spark", "cursor_ide", "mobile"] {
        for api in RETIRED_NOTES_APIS {
            assert!(!notes::contains(api), "{api} must leave notes registry");
            assert!(
                build("notes", api, channel).is_none(),
                "{api} must not build on {channel}"
            );
            assert_eq!(group_for_migrated_api(api), None, "{api}");
        }
        for api in RETIRED_KNOWLEDGE_APIS {
            assert!(
                !knowledge::contains(api),
                "{api} must leave knowledge registry"
            );
            assert!(
                build("knowledge", api, channel).is_none(),
                "{api} must not build on {channel}"
            );
            assert_eq!(group_for_migrated_api(api), None, "{api}");
        }
    }
}

#[test]
fn retired_catalog_tool_modules_are_gone() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for rel in [
        "mcp_host/catalog/groups/notes/get_latest_digest_per_catalog.rs",
        "mcp_host/catalog/groups/notes/get_notes_by_catalog.rs",
        "mcp_host/catalog/groups/knowledge/list_knowledge_repos.rs",
    ] {
        assert!(!src.join(rel).exists(), "{rel} must be deleted");
    }
}
