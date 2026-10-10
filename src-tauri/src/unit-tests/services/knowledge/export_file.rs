use std::fs;

use crate::services::knowledge::{doc_map, get_knowledge_file};
use crate::test_support::TestSandbox;

fn remember(sandbox: &TestSandbox, name: &str, body: &str) -> String {
    let file = sandbox.cache_dir().join("kb-src").join(name);
    fs::create_dir_all(file.parent().unwrap()).expect("mkdir");
    fs::write(&file, body).expect("write");
    doc_map::remember_path(&file).expect("remember")
}

#[test]
fn get_knowledge_file_copies_with_source_extension() {
    let sandbox = TestSandbox::new();
    let id = remember(&sandbox, "guide.md", "# guide");
    let dest = sandbox.cache_dir().join("ws/k1");
    let out = get_knowledge_file(&id, dest.to_str().unwrap());
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["id"], id);
    let path = out["path"].as_str().expect("path");
    assert!(path.ends_with(&format!("{id}.md")));
    assert_eq!(fs::read_to_string(path).unwrap(), "# guide");
    assert!(out.get("content").is_none());
}

#[test]
fn get_knowledge_file_keeps_other_extensions_and_none() {
    let sandbox = TestSandbox::new();
    let txt = remember(&sandbox, "notes.txt", "t");
    let bare = remember(&sandbox, "LICENSE", "l");
    let dest = sandbox.cache_dir().join("ws/k2");
    let dest = dest.to_str().unwrap();
    assert!(get_knowledge_file(&txt, dest)["path"].as_str().unwrap().ends_with(&format!("{txt}.txt")));
    assert!(get_knowledge_file(&bare, dest)["path"].as_str().unwrap().ends_with(&bare));
}

#[test]
fn get_knowledge_file_bumps_version_instead_of_overwriting() {
    let sandbox = TestSandbox::new();
    let id = remember(&sandbox, "guide.md", "body");
    let dest = sandbox.cache_dir().join("ws/k3");
    let dest = dest.to_str().unwrap();
    let first = get_knowledge_file(&id, dest);
    let second = get_knowledge_file(&id, dest);
    assert!(second["path"].as_str().unwrap().ends_with(&format!("{id}-v1.md")));
    assert_ne!(first["path"], second["path"]);
}

#[test]
fn get_knowledge_file_errors_for_unknown_id_and_bad_dest() {
    let sandbox = TestSandbox::new();
    let id = remember(&sandbox, "guide.md", "body");
    let dest = sandbox.cache_dir().join("ws/k4");
    let unknown = get_knowledge_file("not-issued", dest.to_str().unwrap());
    assert_eq!(unknown["ok"], false);
    assert!(!dest.exists(), "unknown id creates no dir");
    let relative = get_knowledge_file(&id, "relative/dir");
    assert_eq!(relative["ok"], false);
    assert_eq!(relative["_status"], 400);
    let in_data = sandbox.data_dir().join("knowledge/copies");
    let denied = get_knowledge_file(&id, in_data.to_str().unwrap());
    assert_eq!(denied["ok"], false);
    assert_eq!(denied["_status"], 403);
    assert!(!in_data.exists());
}
