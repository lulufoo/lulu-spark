use super::*;
use crate::test_support::TestSandbox;
use std::fs;
use std::sync::{Mutex, OnceLock};

static ENTRY_ADMIN_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn entry_admin_test_guard() -> std::sync::MutexGuard<'static, ()> {
    ENTRY_ADMIN_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("entry_admin test lock")
}

fn with_corpus_fixture<F: FnOnce(&std::path::Path, &std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let wb = sandbox.workbench_knowledge_root();
    setup(cfg_dir, wb.as_path());
    f(cfg_dir, wb.as_path());
}

#[test]
fn delete_entry_removes_file_and_index() {
    let _guard = entry_admin_test_guard();
    with_corpus_fixture(
        |_, corpus| {
            fs::create_dir_all(corpus.join("raw/proj")).expect("mkdir");
            fs::write(corpus.join("raw/proj/a.md"), "# x").expect("w");
            let id = "a".repeat(32);
            let index = json!({
                "entries": {
                    id.clone(): { "common_path": "proj/a.md" }
                }
            });
            fs::write(
                corpus.join("index.json"),
                serde_json::to_string_pretty(&index).unwrap(),
            )
            .expect("idx");
        },
        |_, corpus| {
            let id = "a".repeat(32);
            let v = delete_entry(&json!({ "id": id }));
            assert_eq!(v["ok"], true);
            assert!(!corpus.join("raw/proj/a.md").exists());
        },
    );
}

#[test]
fn move_entry_project_moves_zh_translation() {
    let _guard = entry_admin_test_guard();
    with_corpus_fixture(
        |_, corpus| {
            let old_zh = "inbox/topic/slug-zh.md";
            fs::create_dir_all(corpus.join("raw/inbox/topic")).expect("mkdir");
            fs::write(corpus.join("raw/inbox/topic/slug.md"), "# main").expect("w main");
            fs::write(corpus.join("raw").join(old_zh), "# zh").expect("w zh");
            let id = "b".repeat(32);
            let index = json!({
                "entries": {
                    id.clone(): {
                        "common_path": "inbox/topic/slug.md",
                        "translations": { "zh": old_zh }
                    }
                }
            });
            fs::write(
                corpus.join("index.json"),
                serde_json::to_string_pretty(&index).unwrap(),
            )
            .expect("idx");
        },
        |_, corpus| {
            let id = "b".repeat(32);
            let old_zh = "inbox/topic/slug-zh.md";
            let v = move_entry_project(&json!({ "id": id, "new_project": "learning-ai-llm" }));
            assert_eq!(v["ok"], true);
            let new_zh = "learning-ai-llm/topic/slug-zh.md";
            assert!(corpus.join("raw/learning-ai-llm/topic/slug.md").exists());
            assert!(corpus.join("raw").join(new_zh).exists());
            assert!(!corpus.join("raw/inbox/topic/slug.md").exists());
            assert!(!corpus.join("raw").join(old_zh).exists());
            let idx: Value =
                serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
            assert_eq!(
                idx["entries"][&id]["translations"]["zh"],
                json!(new_zh)
            );
        },
    );
}
