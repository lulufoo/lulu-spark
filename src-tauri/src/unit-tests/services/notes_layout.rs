use super::*;
use std::fs;

#[test]
fn ensure_notes_layout_moves_legacy_root_into_notes_and_drops_dead_layers() {
    let dir = tempfile::tempdir().expect("tmp");
    let wb = dir.path();
    fs::create_dir_all(wb.join("raw")).expect("raw");
    fs::write(wb.join("raw/a.md"), "hi").expect("md");
    fs::create_dir_all(wb.join("distilled")).expect("dead");
    fs::write(wb.join("distilled/x.md"), "gone").expect("dead file");
    fs::write(
        wb.join("index.json"),
        r#"{"entries":{"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":{"layers":["raw","distilled","digest"]}}}"#,
    )
    .expect("index");

    ensure_notes_layout(wb).expect("migrate");

    assert!(wb.join("notes/index.json").is_file());
    assert!(wb.join("notes/raw/a.md").is_file());
    assert!(!wb.join("index.json").exists());
    assert!(!wb.join("raw").exists());
    assert!(!wb.join("distilled").exists());
    let index: Value =
        serde_json::from_str(&fs::read_to_string(wb.join("notes/index.json")).unwrap()).unwrap();
    assert_eq!(
        index["entries"]["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]["layers"],
        serde_json::json!(["raw", "digest"])
    );
}

#[test]
fn ensure_notes_layout_is_noop_when_notes_index_exists() {
    let dir = tempfile::tempdir().expect("tmp");
    let wb = dir.path();
    fs::create_dir_all(wb.join("notes")).expect("notes");
    fs::write(wb.join("notes/index.json"), r#"{"entries":{}}"#).expect("index");
    fs::write(wb.join("index.json"), r#"{"entries":{"stale":{}}}"#).expect("legacy");

    ensure_notes_layout(wb).expect("noop");

    assert!(wb.join("index.json").is_file(), "do not move when already migrated");
}
