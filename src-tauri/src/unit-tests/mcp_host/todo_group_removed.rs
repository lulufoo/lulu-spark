//! t5 red gate: the MCP todo group and its wiring are gone.
//! These checks compile whether or not `groups::todo` still exists.

use std::fs;
use std::path::PathBuf;

fn src(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

fn text(rel: &str) -> String {
    fs::read_to_string(src(rel)).unwrap_or_else(|err| panic!("read {rel}: {err}"))
}

#[test]
fn todo_group_directory_is_gone() {
    let dir = src("mcp_host/catalog/groups/todo");
    assert!(
        !dir.exists(),
        "mcp_host/catalog/groups/todo must be deleted"
    );
}

#[test]
fn wiring_files_do_not_reference_todo_group() {
    let forbidden = [
        ("mcp_host/catalog/groups/mod.rs", "pub mod todo"),
        ("mcp_host/catalog/mod.rs", "groups::todo"),
        ("mcp_host/catalog/factory.rs", "todo::GROUP_ID"),
        ("mcp_host/server/slot.rs", "todo::routes_for_channel"),
        ("mcp_host/catalog/runtime.rs", "enabled.todo"),
        ("mcp_host/catalog/route_util.rs", "fn todo_wire"),
        ("services/settings/mcp_channel_tools.rs", "pub todo:"),
        ("services/settings/mcp_catalog.rs", "pub todo:"),
        ("services/settings/mcp_channel_classify.rs", "groups.todo"),
    ];
    for (rel, marker) in forbidden {
        let body = text(rel);
        assert!(
            !body.contains(marker),
            "{rel} still contains `{marker}`"
        );
    }
}
