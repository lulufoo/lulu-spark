use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use crate::commands::read_later::{
    create_read_later_json, delete_read_later_json, get_read_later_json, mark_read_later_json,
};
use crate::test_support::TestSandbox;

static READ_LATER_CMD_TEST_LOCK: Mutex<()> = Mutex::new(());

fn with_read_later_sandbox<F: FnOnce()>(f: F) {
    let _guard = READ_LATER_CMD_TEST_LOCK
        .lock()
        .expect("read_later command test lock");
    let _sandbox = TestSandbox::new();
    f();
}

#[test]
fn create_read_later_json_returns_entry_without_status() {
    with_read_later_sandbox(|| {
        let v = create_read_later_json("https://example.com/a", Some("Example"))
            .expect("create");
        assert!(v.get("_status").is_none());
        let entry = &v["entry"];
        assert_eq!(entry["url"], "https://example.com/a");
        assert_eq!(entry["title"], "Example");
        assert_eq!(entry["read"], false);
        assert!(entry.get("id").and_then(|x| x.as_str()).is_some());
        assert!(entry.get("saved_at").and_then(|x| x.as_str()).is_some());
    });
}

#[test]
fn get_read_later_json_returns_desc_sorted_array() {
    with_read_later_sandbox(|| {
        create_read_later_json("https://example.com/1", Some("first")).expect("first");
        thread::sleep(Duration::from_millis(5));
        create_read_later_json("https://example.com/2", Some("second")).expect("second");

        let list = get_read_later_json().expect("list");
        let items = list.as_array().expect("array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["url"], "https://example.com/2");
        assert_eq!(items[1]["url"], "https://example.com/1");
    });
}

#[test]
fn mark_read_later_json_updates_entry_and_list() {
    with_read_later_sandbox(|| {
        let created =
            create_read_later_json("https://example.com/x", Some("x")).expect("create");
        let id = created["entry"]["id"].as_str().expect("id").to_string();

        let updated = mark_read_later_json(&id, true).expect("mark");
        assert!(updated.get("_status").is_none());
        assert_eq!(updated["entry"]["read"], true);
        assert_eq!(updated["entry"]["id"], id);

        let list = get_read_later_json().expect("list");
        assert_eq!(list[0]["read"], true);
    });
}

#[test]
fn create_read_later_json_empty_url_returns_400_class() {
    with_read_later_sandbox(|| {
        let v = create_read_later_json("", None).expect("invoke");
        assert_eq!(v["error"], "Missing url");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn mark_read_later_json_unknown_id_returns_404_class() {
    with_read_later_sandbox(|| {
        let v = mark_read_later_json("00000000000000000000000000000000", true)
            .expect("invoke");
        assert_eq!(v["error"], "Not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn delete_read_later_json_removes_entry() {
    with_read_later_sandbox(|| {
        let created = create_read_later_json("https://example.com/del", Some("del"))
            .expect("create");
        let id = created["entry"]["id"].as_str().expect("id").to_string();
        let deleted = delete_read_later_json(&id).expect("delete");
        assert_eq!(deleted["entry"]["id"], id);
        let list = get_read_later_json().expect("list");
        assert_eq!(list.as_array().expect("array").len(), 0);
    });
}
