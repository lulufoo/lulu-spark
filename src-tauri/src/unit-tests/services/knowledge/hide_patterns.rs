use super::*;
use crate::test_support::TestSandbox;
use std::fs;

#[test]
fn load_or_seed_writes_seed_when_missing() {
    let sandbox = TestSandbox::new();
    let rows = load_or_seed();
    let patterns: Vec<_> = rows.iter().map(|r| r.pattern.as_str()).collect();
    assert_eq!(patterns, SEED_PATTERNS);
    let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["seed00", "seed01", "seed02", "seed03"]);
    let path = sandbox.cache_dir().join("knowledge-hide-patterns.json");
    assert!(path.is_file(), "seed must write cache file");
}

#[test]
fn name_is_hidden_is_or() {
    let git = Regex::new(r"^\.git$").unwrap();
    let cache = Regex::new(r"^\.cache$").unwrap();
    let repo_type = Regex::new(r"^\.repository-type\.json$").unwrap();
    let hide = [git, cache, repo_type];
    assert!(name_is_hidden(".git", &hide));
    assert!(name_is_hidden(".cache", &hide));
    assert!(name_is_hidden(".repository-type.json", &hide));
    assert!(!name_is_hidden("git.md", &hide));
    assert!(!name_is_hidden("readme.md", &hide));
}

#[test]
fn add_rejects_invalid_and_duplicate() {
    let _sandbox = TestSandbox::new();
    let bad = add("[").expect("add");
    assert!(bad.get("error").is_some());
    let ok = add("^draft$").expect("add draft");
    assert!(ok.get("error").is_none());
    let dup = add("^draft$").expect("dup");
    assert_eq!(dup["error"], "duplicate pattern");
}

#[test]
fn update_and_remove_by_id() {
    let _sandbox = TestSandbox::new();
    let listed = add(r"^\.xxx$").expect("add");
    let rows = listed["patterns"].as_array().expect("arr");
    let added = rows
        .iter()
        .find(|r| r["pattern"] == r"^\.xxx$")
        .expect("row");
    let id = added["id"].as_str().expect("id").to_string();
    let updated = update(&id, r"^\.yyy$").expect("update");
    assert!(updated.get("error").is_none());
    let patterns: Vec<_> = updated["patterns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["pattern"].as_str().unwrap())
        .collect();
    assert!(patterns.contains(&r"^\.yyy$"));
    assert!(!patterns.contains(&r"^\.xxx$"));
    let removed = remove(&id).expect("remove");
    let left: Vec<_> = removed["patterns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert!(!left.contains(&id.as_str()));
}

#[test]
fn collect_notes_skips_seed_hidden_dir() {
    let sandbox = TestSandbox::new();
    let notes = sandbox.workbench_root().join("notes").join("raw");
    fs::create_dir_all(notes.join(".cache")).expect("mkdir");
    fs::write(notes.join("keep.md"), "# k").expect("w");
    fs::write(notes.join(".cache/hidden.md"), "# h").expect("w");
    let chunks = crate::services::keyword_index::collect_notes(sandbox.config_dir());
    let paths: Vec<_> = chunks.iter().map(|c| c.common_path.as_str()).collect();
    assert!(paths.contains(&"keep.md"));
    assert!(!paths.iter().any(|p| p.contains(".cache")));
}

#[test]
fn corrupt_file_reads_empty_without_overwrite() {
    let sandbox = TestSandbox::new();
    let path = sandbox.cache_dir().join("knowledge-hide-patterns.json");
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, "{not-json").expect("write");
    let rows = load_or_seed();
    assert!(rows.is_empty());
    let text = fs::read_to_string(&path).expect("read");
    assert_eq!(text, "{not-json");
}
