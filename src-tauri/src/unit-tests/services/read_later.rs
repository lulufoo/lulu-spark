use super::*;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use crate::config::paths;
use crate::test_support::with_test_config_dir;

static READ_LATER_TEST_LOCK: Mutex<()> = Mutex::new(());

fn with_read_later_cache<F: FnOnce(&Path)>(f: F) {
    let _guard = READ_LATER_TEST_LOCK.lock().expect("read_later test lock");
    with_test_config_dir(|cfg| {
        let cache = cfg.join("cache");
        fs::write(
            cfg.join("config.toml"),
            format!(r#"cache_dir = "{}""#, cache.display()),
        )
        .expect("write config");
        f(&cache);
    });
}

fn is_iso8601(s: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(s).is_ok()
}

#[test]
fn read_later_path_is_under_cache_dir() {
    with_read_later_cache(|cache| {
        let path = read_later_path().expect("path");
        assert_eq!(path, cache.join("read_later.json"));
        let corpus = paths::workbench_knowledge_root().expect("corpus root");
        assert!(
            !path.starts_with(&corpus),
            "read_later.json must not live under corpus tree"
        );
    });
}

#[test]
fn list_entries_empty_store_returns_empty_array() {
    with_read_later_cache(|_| {
        let v = list_entries();
        assert_eq!(v.as_array().expect("array").len(), 0);
        assert!(v.get("_status").is_none());
    });
}

#[test]
fn create_entry_returns_id_saved_at_and_unread() {
    with_read_later_cache(|_| {
        let v = create_entry("https://example.com/a", Some("Example"));
        assert_eq!(v["_status"], 201);
        let entry = &v["entry"];
        assert_eq!(entry["url"], "https://example.com/a");
        assert_eq!(entry["title"], "Example");
        assert_eq!(entry["read"], false);
        let id = entry["id"].as_str().expect("id");
        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        let saved_at = entry["saved_at"].as_str().expect("saved_at");
        assert!(is_iso8601(saved_at));
    });
}

#[test]
fn create_entry_allows_missing_or_empty_title() {
    with_read_later_cache(|_| {
        let no_title = create_entry("https://example.com/b", None);
        assert_eq!(no_title["_status"], 201);
        assert_eq!(no_title["entry"]["title"], "");

        let empty_title = create_entry("https://example.com/c", Some(""));
        assert_eq!(empty_title["_status"], 201);
        assert_eq!(empty_title["entry"]["title"], "");
    });
}

#[test]
fn create_entry_rejects_missing_url() {
    with_read_later_cache(|_| {
        let v = create_entry("", None);
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn list_entries_sorted_by_saved_at_desc() {
    with_read_later_cache(|_| {
        create_entry("https://example.com/1", Some("first"));
        thread::sleep(Duration::from_millis(5));
        create_entry("https://example.com/2", Some("second"));
        thread::sleep(Duration::from_millis(5));
        create_entry("https://example.com/3", Some("third"));

        let list = list_entries().as_array().expect("array").clone();
        assert_eq!(list.len(), 3);
        let times: Vec<&str> = list
            .iter()
            .map(|e| e["saved_at"].as_str().expect("saved_at"))
            .collect();
        assert!(times[0] >= times[1]);
        assert!(times[1] >= times[2]);
        assert_eq!(list[0]["url"], "https://example.com/3");
    });
}

#[test]
fn mark_read_updates_existing_entry() {
    with_read_later_cache(|_| {
        let created = create_entry("https://example.com/x", Some("x"));
        let id = created["entry"]["id"].as_str().expect("id").to_string();

        let updated = mark_read(&id, true);
        assert_eq!(updated["_status"], 200);
        assert_eq!(updated["entry"]["read"], true);
        assert_eq!(updated["entry"]["id"], id);

        let list_val = list_entries();
        let list = list_val.as_array().expect("array");
        assert_eq!(list[0]["read"], true);
    });
}

#[test]
fn mark_read_unknown_id_returns_404() {
    with_read_later_cache(|_| {
        let v = mark_read("00000000000000000000000000000000", true);
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn create_and_mark_read_persists_valid_json() {
    with_read_later_cache(|cache| {
        let created = create_entry("https://example.com/persist", Some("persist"));
        let id = created["entry"]["id"].as_str().expect("id").to_string();
        let marked = mark_read(&id, true);
        assert_eq!(marked["_status"], 200);

        let path = cache.join("read_later.json");
        assert!(path.is_file());
        let text = fs::read_to_string(&path).expect("read json");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["version"], 1);
        let entries = parsed["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["read"], true);
    });
}

#[test]
fn corrupt_json_recovers_to_empty_and_rebuilds() {
    with_read_later_cache(|cache| {
        let path = cache.join("read_later.json");
        fs::create_dir_all(cache).expect("mkdir");
        fs::write(&path, "{not valid json").expect("write corrupt");

        let list = list_entries();
        assert_eq!(list.as_array().expect("array").len(), 0);

        let created = create_entry("https://example.com/recover", None);
        assert_eq!(created["_status"], 201);

        let text = fs::read_to_string(&path).expect("read rebuilt");
        serde_json::from_str::<serde_json::Value>(&text).expect("rebuilt json valid");
        assert_eq!(list_entries().as_array().expect("array").len(), 1);
    });
}
