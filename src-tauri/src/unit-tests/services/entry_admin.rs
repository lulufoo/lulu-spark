use super::*;
use crate::test_support::{block_note_writes, read_note_entries, seed_note_entries, TestSandbox};
use std::fs;

fn with_notes_fixture<F: FnOnce(&std::path::Path, &std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let notes = sandbox.data_dir().join("notes");
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
            seed_note_entries(&index["entries"]);
        },
        |_, notes| {
            let id = "a".repeat(32);
            let v = delete_entry(&json!({ "id": id }));
            assert_eq!(v["ok"], true);
            assert_eq!(v["id"], json!(id));
            assert_eq!(v["removed"], json!(1));
            assert!(v.get("deleted").is_none());
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
            seed_note_entries(&index["entries"]);
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
                read_note_entries();
            assert_eq!(
                idx["entries"][&id]["translations"]["zh"],
                json!(new_zh)
            );
        },
    );
}

#[test]
fn delete_entry_removes_the_files_before_the_row() {
    with_notes_fixture(
        |_, notes| {
            fs::create_dir_all(notes.join("raw/proj")).expect("mkdir");
            fs::write(notes.join("raw/proj/a.md"), "# x").expect("w");
            seed_note_entries(&json!({ "a".repeat(32): { "common_path": "proj/a.md" } }));
            block_note_writes("DELETE");
        },
        |_, notes| {
            let id = "a".repeat(32);
            let v = delete_entry(&json!({ "id": id }));
            assert_eq!(v["_status"], 500, "{v}");
            assert!(!notes.join("raw/proj/a.md").exists(), "the file step already ran");
            assert!(read_note_entries()["entries"].get(&id).is_some(), "the row is still there");
        },
    );
}

#[test]
fn move_entry_project_moves_the_files_before_the_row() {
    with_notes_fixture(
        |_, notes| {
            crate::services::notes::create_notes_category("learning-ai-llm", "LLM", "")
                .expect("notes category");
            fs::create_dir_all(notes.join("raw/inbox/topic")).expect("mkdir");
            fs::write(notes.join("raw/inbox/topic/slug.md"), "# x").expect("w");
            seed_note_entries(&json!({ "c".repeat(32): { "common_path": "inbox/topic/slug.md" } }));
            block_note_writes("UPDATE");
        },
        |_, notes| {
            let id = "c".repeat(32);
            let v = move_entry_project(&json!({ "id": id, "new_project": "learning-ai-llm" }));
            assert_eq!(v["_status"], 500, "{v}");
            assert!(notes.join("raw/learning-ai-llm/topic/slug.md").exists(), "the file step already ran");
            assert_eq!(
                read_note_entries()["entries"][&id]["common_path"],
                json!("inbox/topic/slug.md")
            );
        },
    );
}

#[test]
fn move_entry_project_moves_every_translation_and_the_digest_path() {
    with_notes_fixture(
        |_, notes| {
            crate::services::notes::create_notes_category("learning-ai-llm", "LLM", "")
                .expect("notes category");
            fs::create_dir_all(notes.join("raw/inbox/topic")).expect("mkdir");
            fs::create_dir_all(notes.join("digest/inbox/topic")).expect("mkdir");
            for name in ["slug.md", "slug-zh.md", "slug-ja.md"] {
                fs::write(notes.join("raw/inbox/topic").join(name), "# x").expect("w");
            }
            fs::write(notes.join("digest/inbox/topic/slug.md"), "# d").expect("w");
            seed_note_entries(&json!({ "d".repeat(32): {
                "common_path": "inbox/topic/slug.md", "layers": ["raw", "digest"],
                "translations": { "zh": "inbox/topic/slug-zh.md", "ja": "inbox/topic/slug-ja.md" }
            }}));
        },
        |_, notes| {
            let id = "d".repeat(32);
            let v = move_entry_project(&json!({ "id": id, "new_project": "learning-ai-llm" }));
            assert_eq!(v["ok"], true, "{v}");
            for name in ["slug.md", "slug-zh.md", "slug-ja.md"] {
                assert!(notes.join("raw/learning-ai-llm/topic").join(name).exists(), "{name}");
            }
            assert!(notes.join("digest/learning-ai-llm/topic/slug.md").exists());
            let entry = read_note_entries()["entries"][&id].clone();
            assert_eq!(entry["common_path"], json!("learning-ai-llm/topic/slug.md"));
            assert_eq!(entry["layers"], json!(["raw", "digest"]));
            assert_eq!(entry["translations"]["ja"], json!("learning-ai-llm/topic/slug-ja.md"));
        },
    );
}
