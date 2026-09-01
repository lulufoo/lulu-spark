use super::*;
use crate::test_support::TestSandbox;
use std::fs;

fn with_notes_fixture<F: FnOnce(&std::path::Path, &std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let notes = sandbox.workbench_root().join("notes");
    fs::create_dir_all(&notes).expect("notes");
    setup(cfg_dir, notes.as_path());
    f(cfg_dir, notes.as_path());
}

#[test]
fn delete_entry_removes_file_and_index() {
    with_notes_fixture(
        |_, notes| {
            fs::create_dir_all(notes.join("raw/proj")).expect("mkdir");
            fs::write(notes.join("raw/proj/a.md"), "# x").expect("w");
            let id = "a".repeat(32);
            let index = json!({
                "entries": {
                    id.clone(): { "common_path": "proj/a.md" }
                }
            });
            fs::write(
                notes.join("index.json"),
                serde_json::to_string_pretty(&index).unwrap(),
            )
            .expect("idx");
        },
        |_, notes| {
            let id = "a".repeat(32);
            let v = delete_entry(&json!({ "id": id }));
            assert_eq!(v["ok"], true);
            assert!(!notes.join("raw/proj/a.md").exists());
        },
    );
}

#[test]
fn move_entry_project_moves_zh_translation() {
    with_notes_fixture(
        |_, notes| {
            crate::services::notes::create_notes_category("learning-ai-llm", "LLM", "")
                .expect("notes category");

            let old_zh = "inbox/topic/slug-zh.md";
            fs::create_dir_all(notes.join("raw/inbox/topic")).expect("mkdir");
            fs::write(notes.join("raw/inbox/topic/slug.md"), "# main").expect("w main");
            fs::write(notes.join("raw").join(old_zh), "# zh").expect("w zh");
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
                notes.join("index.json"),
                serde_json::to_string_pretty(&index).unwrap(),
            )
            .expect("idx");
        },
        |_, notes| {
            let id = "b".repeat(32);
            let old_zh = "inbox/topic/slug-zh.md";
            let v = move_entry_project(&json!({ "id": id, "new_project": "learning-ai-llm" }));
            assert_eq!(v["ok"], true);
            let new_zh = "learning-ai-llm/topic/slug-zh.md";
            assert!(notes.join("raw/learning-ai-llm/topic/slug.md").exists());
            assert!(notes.join("raw").join(new_zh).exists());
            assert!(!notes.join("raw/inbox/topic/slug.md").exists());
            assert!(!notes.join("raw").join(old_zh).exists());
            let idx: Value =
                serde_json::from_str(&fs::read_to_string(notes.join("index.json")).unwrap()).unwrap();
            assert_eq!(
                idx["entries"][&id]["translations"]["zh"],
                json!(new_zh)
            );
        },
    );
}
