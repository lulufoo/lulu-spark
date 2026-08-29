use super::*;
use std::fs;

#[test]
fn ensure_knowledge_registry_layout_renames_sediment_kb() {
    let dir = tempfile::tempdir().expect("tmp");
    let wb = dir.path();
    fs::create_dir_all(wb.join("sediment-kb")).expect("old");
    fs::write(wb.join("sediment-kb/repos.json"), "{}").expect("repos");

    ensure_knowledge_registry_layout(wb).expect("migrate");

    assert!(wb.join("knowledge/repos.json").is_file());
    assert!(!wb.join("sediment-kb").exists());
}

#[test]
fn ensure_knowledge_registry_layout_is_noop_when_knowledge_exists() {
    let dir = tempfile::tempdir().expect("tmp");
    let wb = dir.path();
    fs::create_dir_all(wb.join("knowledge")).expect("new");
    fs::write(wb.join("knowledge/repos.json"), "{\"ok\":1}").expect("repos");
    fs::create_dir_all(wb.join("sediment-kb")).expect("stale");
    fs::write(wb.join("sediment-kb/repos.json"), "{\"stale\":1}").expect("stale");

    ensure_knowledge_registry_layout(wb).expect("noop");

    assert_eq!(
        fs::read_to_string(wb.join("knowledge/repos.json")).unwrap(),
        "{\"ok\":1}"
    );
    assert!(wb.join("sediment-kb").is_dir());
}
