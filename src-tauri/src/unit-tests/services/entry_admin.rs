use super::*;
use crate::config::settings;
use std::fs;

#[test]
fn delete_entry_removes_file_and_index() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
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
    settings::write_test_config(dir.path(), &corpus, None);
    let v = delete_entry(&json!({ "id": id }));
    assert_eq!(v["ok"], true);
    assert!(!corpus.join("raw/proj/a.md").exists());
    settings::set_test_config_dir(None);
}
