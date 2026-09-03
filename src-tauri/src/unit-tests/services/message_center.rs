use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::Value;

use crate::config::paths;
use crate::test_support::{read_rs_dir, TestSandbox};

fn with_message_center_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    set_changed_handler(None);
    f();
    set_changed_handler(None);
}

fn is_iso8601(s: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(s).is_ok()
}

fn persist_value() -> Value {
    let path = paths::message_center_path().expect("message_center_path");
    let text = fs::read_to_string(&path).expect("read persist file");
    serde_json::from_str(&text).expect("parse persist file")
}

fn walk_objects(value: &Value, visit: &mut impl FnMut(&serde_json::Map<String, Value>)) {
    match value {
        Value::Object(map) => {
            visit(map);
            for child in map.values() {
                walk_objects(child, visit);
            }
        }
        Value::Array(items) => {
            for child in items {
                walk_objects(child, visit);
            }
        }
        _ => {}
    }
}

fn is_min_record(map: &serde_json::Map<String, Value>) -> bool {
    map.contains_key("id")
        && map.contains_key("channel")
        && map.contains_key("unread")
        && map.contains_key("created_at")
}

fn collect_min_records(value: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    walk_objects(value, &mut |map| {
        if is_min_record(map) {
            out.push(Value::Object(map.clone()));
        }
    });
    out
}

fn install_notify_counter() -> Arc<AtomicUsize> {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_cb = Arc::clone(&hits);
    set_changed_handler(Some(Arc::new(move || {
        hits_cb.fetch_add(1, Ordering::SeqCst);
    })));
    hits
}

#[test]
fn produce_notes_marks_channel_unread() {
    with_message_center_sandbox(|| {
        assert!(!channel_unread("notes"));
        let record = produce("notes").expect("produce notes");
        assert_eq!(record.channel, "notes");
        assert!(record.unread);
        assert!(channel_unread("notes"));
    });
}

#[test]
fn produce_notifies_once_after_persist() {
    with_message_center_sandbox(|| {
        let hits = install_notify_counter();
        produce("notes").expect("produce notes");
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(paths::message_center_path()
            .expect("message_center_path")
            .is_file());
    });
}

#[test]
fn mark_channel_read_clears_unread() {
    with_message_center_sandbox(|| {
        produce("notes").expect("produce notes");
        assert!(channel_unread("notes"));
        mark_channel_read("notes").expect("mark notes read");
        assert!(!channel_unread("notes"));
    });
}

#[test]
fn only_notes_read_later_todos_channels_and_unread_is_independent() {
    with_message_center_sandbox(|| {
        assert!(produce("knowledge").is_err());
        assert!(produce("").is_err());
        assert!(produce("Notes").is_err());
        assert!(mark_channel_read("knowledge").is_err());
        assert!(!channel_unread("knowledge"));

        produce("notes").expect("produce notes");
        produce("read_later").expect("produce read_later");
        produce("todos").expect("produce todos");
        assert!(channel_unread("notes"));
        assert!(channel_unread("read_later"));
        assert!(channel_unread("todos"));

        mark_channel_read("notes").expect("mark notes read");
        assert!(!channel_unread("notes"));
        assert!(channel_unread("read_later"));
        assert!(channel_unread("todos"));
    });
}

#[test]
fn one_produce_one_record_with_only_min_fields() {
    with_message_center_sandbox(|| {
        let first = produce("notes").expect("first produce");
        let second = produce("notes").expect("second produce");
        assert_ne!(first.id, second.id);
        assert!(first.unread && second.unread);
        assert!(is_iso8601(&first.created_at));
        assert!(is_iso8601(&second.created_at));

        for record in [&first, &second] {
            let value = serde_json::to_value(record).expect("serialize record");
            let obj = value.as_object().expect("record object");
            assert_eq!(obj.len(), 4);
            assert!(obj.contains_key("id"));
            assert!(obj.contains_key("channel"));
            assert!(obj.contains_key("unread"));
            assert!(obj.contains_key("created_at"));
            assert!(!obj.contains_key("body"));
            assert!(!obj.contains_key("title"));
            assert!(!obj.contains_key("content"));
            assert!(!obj.contains_key("url"));
        }

        let stored = persist_value();
        let records = collect_min_records(&stored);
        assert_eq!(records.len(), 2);
        let ids: Vec<&str> = records
            .iter()
            .map(|r| r["id"].as_str().expect("id"))
            .collect();
        assert!(ids.contains(&first.id.as_str()));
        assert!(ids.contains(&second.id.as_str()));
        walk_objects(&stored, &mut |map| {
            if is_min_record(map) {
                assert!(!map.contains_key("body"));
                assert!(!map.contains_key("title"));
                assert!(!map.contains_key("content"));
                assert!(!map.contains_key("url"));
                assert!(!map.contains_key("text"));
            }
        });
    });
}

#[test]
fn missing_store_means_all_channels_read() {
    with_message_center_sandbox(|| {
        let path = paths::message_center_path().expect("message_center_path");
        assert!(!path.exists());
        assert!(!channel_unread("notes"));
        assert!(!channel_unread("read_later"));
        assert!(!channel_unread("todos"));
    });
}

#[test]
fn persist_failure_does_not_notify() {
    with_message_center_sandbox(|| {
        let hits = install_notify_counter();
        let path = paths::message_center_path().expect("message_center_path");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("persist parent");
        }
        fs::create_dir_all(&path).expect("occupy persist path as directory");

        assert!(produce("notes").is_err());
        assert_eq!(hits.load(Ordering::SeqCst), 0);
        assert!(!channel_unread("notes"));
    });
}

#[test]
fn production_module_declares_tests_without_test_bodies() {
    let prod = read_rs_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/message_center"
    ));
    assert!(
        prod.contains("#[cfg(test)]")
            && prod.contains("#[path = \"../../unit-tests/services/message_center.rs\"]"),
        "production module must declare unit-tests path"
    );
    assert!(
        !prod.contains("#[test]"),
        "test bodies must not live in production files"
    );
}
