use std::fs;
use std::path::Path;

use serde_json::json;

use crate::config::paths;
use crate::agent::r#loop;
use crate::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
use crate::services::spark_path_fence::expand_for_business_key;
use crate::test_support::TestSandbox;

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn with_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    registry::clear_for_tests();
    registry::seed_defaults();
    f(&sandbox);
}

fn plant_demo_repo(sandbox: &TestSandbox) {
    let kb = sandbox.spark_root().join("knowledge");
    fs::create_dir_all(&kb).expect("knowledge");
    fs::write(
        kb.join("repos.json"),
        r#"{"version":1,"repos":[{"full_name":"acme/demo","category_id":"uncategorized"}]}"#,
    )
    .expect("repos.json");
    fs::create_dir_all(sandbox.knowledge_root().join("demo")).expect("clone dir");
}

#[test]
fn expand_spark_includes_knowledge_root_and_listed_clone() {
    with_sandbox(|sandbox| {
        plant_demo_repo(sandbox);
        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY).expect("spark fence");
        let wb = paths::spark_root().expect("wb");
        let clone = paths::knowledge_root().expect("knowledge_root").join("demo");
        let cache = paths::cache_dir().expect("cache");

        let fence_wb = fence
            .read_allow
            .iter()
            .find(|p| same_path(p, &wb))
            .expect("$WB in A1");
        let fence_clone = fence
            .read_allow
            .iter()
            .find(|p| same_path(p, &clone))
            .expect("clone in A1");
        assert!(!fence
            .read_allow
            .iter()
            .any(|p| same_path(p, &paths::knowledge_root().expect("knowledge_root"))));
        assert!(fence.read_deny.iter().any(|p| p == &fence_wb.join(".git")));
        assert!(fence
            .read_deny
            .iter()
            .any(|p| p == &fence_clone.join(".git")));
        assert!(fence.write_allow.is_empty());
        let scratch_parent = fence.scratch_parent.as_ref().expect("scratch_parent");
        assert_eq!(
            scratch_parent.file_name().and_then(|n| n.to_str()),
            Some("agent-scratch")
        );
        assert!(
            scratch_parent
                .parent()
                .is_some_and(|p| same_path(p, &cache)),
            "scratch_parent={scratch_parent:?} cache={cache:?}"
        );
        assert!(!fence.allows_write(&wb.join("todo.md")));
        assert!(expand_for_business_key("other").is_none());
    });
}

#[test]
fn public_set_attaches_fence_and_reset_clears_it() {
    with_sandbox(|sandbox| {
        plant_demo_repo(sandbox);
        assert!(r#loop::loaded_path_fence().is_none());
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let fence = r#loop::loaded_path_fence().expect("fence after Set");
        assert!(
            fence
                .read_allow
                .iter()
                .any(|p| same_path(p, &sandbox.spark_root())),
            "A1 must include $WB, got {:?}",
            fence.read_allow
        );
        assert!(fence.write_allow.is_empty(), "B1 stays empty until a turn");
        r#loop::reset_binding().expect("Reset");
        assert!(r#loop::loaded_path_fence().is_none());
    });
}

#[test]
fn typed_set_does_not_attach_a_fence() {
    with_sandbox(|_| {
        r#loop::set_binding(r#loop::Binding {
            tools: json!([]),
            prompt: json!("typed prompt"),
            callbacks: json!({}),
        })
        .expect("typed Set");
        assert!(r#loop::loaded_path_fence().is_none());
    });
}
