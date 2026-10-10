use super::*;
use std::fs;

use crate::config::paths;
use crate::test_support::TestSandbox;

#[test]
fn remember_reuses_id_for_same_path() {
    let sandbox = TestSandbox::new();
    let file = sandbox.cache_dir().join("doc.md");
    fs::write(&file, "body").expect("write");
    let first = remember_path(&file).expect("first");
    let second = remember_path(&file).expect("second");
    assert_eq!(first, second);
    assert_eq!(first.len(), 32);
    assert!(first.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')), "{first}");
    let map = paths::knowledge_doc_map_path().expect("map path");
    sandbox.assert_not_prod_path(&map).expect("sandbox map");
    assert!(map.starts_with(sandbox.cache_dir()));
}

#[test]
fn lookup_unknown_id_is_none() {
    let _sandbox = TestSandbox::new();
    assert_eq!(lookup_path("missing").expect("lookup"), None);
    assert_eq!(lookup_path("").expect("empty"), None);
}

#[test]
fn lookup_returns_remembered_path() {
    let sandbox = TestSandbox::new();
    let file = sandbox.cache_dir().join("kept.md");
    fs::write(&file, "x").expect("write");
    let id = remember_path(&file).expect("remember");
    let got = lookup_path(&id).expect("lookup").expect("some");
    assert_eq!(got, file.canonicalize().unwrap_or(file));
}

#[test]
fn existing_short_ids_are_kept_and_still_resolve() {
    let sandbox = TestSandbox::new();
    let file = sandbox.cache_dir().join("legacy.md");
    fs::write(&file, "x").expect("write");
    let canon = file.canonicalize().expect("canon");
    let map = paths::knowledge_doc_map_path().expect("map path");
    fs::create_dir_all(map.parent().unwrap()).expect("mkdir");
    fs::write(
        &map,
        serde_json::json!({"version": 1, "entries": [{"id": "0123456789ab", "path": canon.to_string_lossy()}]})
            .to_string(),
    )
    .expect("legacy map");
    assert_eq!(remember_path(&file).expect("reuse"), "0123456789ab");
    assert_eq!(lookup_path("0123456789ab").expect("lookup"), Some(canon));
    let other = sandbox.cache_dir().join("fresh.md");
    fs::write(&other, "y").expect("write");
    assert_eq!(remember_path(&other).expect("new").len(), 32);
}
