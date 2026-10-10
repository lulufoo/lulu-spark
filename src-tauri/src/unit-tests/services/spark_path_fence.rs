use std::fs;
use std::path::Path;

use serde_json::json;

use crate::config::paths;
use crate::agent::r#loop;
use crate::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
use crate::services::path_fence::{stored_path, validate_stage_file};
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
    let kb = sandbox.data_dir().join("knowledge");
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

        let data = paths::runtime_data_dir();
        assert!(
            !fence.read_allow.iter().any(|p| same_path(p, &data)),
            "SPARK_DATA_DIR must not be a read root"
        );
        assert!(fence.read_allow.is_empty(), "no default read roots: {:?}", fence.read_allow);
        assert!(fence.read_deny.iter().any(|p| same_path(p, &data)));
        assert!(
            !fence.read_allow.iter().any(|p| same_path(p, &wb)),
            "spark_root must not be a default read root"
        );
        assert!(
            !fence.read_allow.iter().any(|p| same_path(p, &clone)),
            "clone must not be its own allow root"
        );
        assert!(!fence
            .read_allow
            .iter()
            .any(|p| same_path(p, &paths::knowledge_root().expect("knowledge_root"))));
        assert!(!fence.allows_read(&clone), "SPARK_DATA_DIR is closed to the LLM");
        assert!(!fence.allows_read(&wb.join("readable.md")));
        assert!(fence
            .read_deny
            .contains(&stored_path(wb.clone()).join(".git")));
        assert!(fence
            .read_deny
            .contains(&stored_path(clone.clone()).join(".git")));
        assert!(fence.write_allow.is_empty());
        let scratch_parent = fence.scratch_parent.as_ref().expect("scratch_parent");
        assert_eq!(
            scratch_parent.file_name().and_then(|n| n.to_str()),
            Some("agent-workspace")
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
            !fence
                .read_allow
                .iter()
                .any(|p| same_path(p, &sandbox.data_dir())),
            "A1 must not include SPARK_DATA_DIR, got {:?}",
            fence.read_allow
        );
        assert!(
            !fence
                .read_allow
                .iter()
                .any(|p| same_path(p, &sandbox.spark_root())),
            "A1 must not include spark_root, got {:?}",
            fence.read_allow
        );
        assert!(fence.write_allow.is_empty(), "B1 stays empty until a turn");
        r#loop::reset_binding().expect("Reset");
        assert!(r#loop::loaded_path_fence().is_none());
    });
}

#[test]
fn staged_exact_spark_root_file_is_readable_and_writable() {
    with_sandbox(|sandbox| {
        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY).expect("spark fence");
        let outside = sandbox.spark_root().join("only-if-staged.md");
        fs::write(&outside, "secret").expect("write");
        assert!(!fence.allows_read(&outside));
        let mut granted = fence
            .with_session_scratch("spark_chat_abc")
            .expect("scratch");
        granted.grant_staged_file(outside.clone());
        assert!(granted.allows_read(&outside));
        assert!(granted.allows_write(&outside));
        assert!(!granted.allows_write(&sandbox.spark_root().join("sibling.md")));
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

#[test]
fn data_dir_is_closed_to_read_and_write_even_when_granted() {
    with_sandbox(|sandbox| {
        let note = sandbox.data_dir().join("notes").join("n1.md");
        fs::create_dir_all(note.parent().unwrap()).expect("notes dir");
        fs::write(&note, "body").expect("note");
        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY).expect("spark fence");
        assert!(fence.is_read_denied(&note));
        assert!(!fence.allows_read(&note));
        assert!(!fence.allows_write(&note));

        let mut granted = fence.with_session_scratch("spark_chat_abc").expect("scratch");
        granted.grant_staged_file(stored_path(note.clone()));
        assert!(!granted.allows_read(&note), "a legacy staged Data file stays closed");
        assert!(!granted.allows_write(&note));
    });
}

#[test]
fn session_scratch_is_outside_the_closed_data_dir() {
    with_sandbox(|sandbox| {
        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY).expect("spark fence");
        let granted = fence.with_session_scratch("spark_chat_abc").expect("scratch");
        let scratch = granted
            .session_scratch_root("spark_chat_abc")
            .expect("root")
            .expect("scratch parent");
        let pad = scratch.join("pad.md");
        assert!(granted.allows_write(&pad), "scratch must stay writable");
        assert!(granted.allows_read(&pad));
        assert!(!granted.allows_write(&sandbox.data_dir().join("pad.md")));
    });
}

#[test]
fn stage_rejects_data_files_and_accepts_external_files() {
    with_sandbox(|sandbox| {
        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY).expect("spark fence");
        let inside = sandbox.data_dir().join("notes").join("n2.md");
        fs::create_dir_all(inside.parent().unwrap()).expect("notes dir");
        fs::write(&inside, "body").expect("note");
        let err = validate_stage_file(inside.to_str().unwrap(), &fence).expect_err("Data");
        assert!(err.contains("outside the read fence"), "{err}");

        let outside = sandbox.config_dir().join("external").join("doc.md");
        fs::create_dir_all(outside.parent().unwrap()).expect("external dir");
        fs::write(&outside, "body").expect("external");
        let ok = validate_stage_file(outside.to_str().unwrap(), &fence).expect("external");
        assert_eq!(ok, stored_path(outside));
    });
}
