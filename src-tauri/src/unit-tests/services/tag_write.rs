use super::*;
use std::fs;

use serde_json::json;

use crate::services::annotation::read_annotation_object;
use crate::services::spark_data::{get_note, open};
use crate::services::tags_registry::read_registry;
use crate::test_support::{seed_note_entries, with_sandbox_notes};

fn seed(cps: &[&str]) {
    let entries: serde_json::Map<String, Value> = cps
        .iter()
        .enumerate()
        .map(|(i, cp)| (format!("{:0>32}", i + 1), json!({ "common_path": cp })))
        .collect();
    seed_note_entries(&Value::Object(entries));
}

#[test]
fn tag_attach_new_key_writes_the_tag_and_the_note_row() {
    with_sandbox_notes(|dir, _| {
        let cp = "ai/tagged.md";
        seed(&[cp]);
        let v = tag_attach(dir, cp, &json!({ "value": "my-tag" }));
        assert_eq!(v["ok"], true);
        let key = v["key"].as_str().expect("key");
        assert_eq!(key.len(), 12);

        let reg = read_registry();
        assert_eq!(reg["keys"][key]["value"], "my-tag");
        assert_eq!(reg["keys"][key]["refs"], 1);

        let ann = read_annotation_object(cp);
        assert_eq!(ann["tag_keys"], json!([key]));
    });
}

#[test]
fn tag_attach_same_key_twice_is_idempotent() {
    with_sandbox_notes(|dir, _| {
        let cp = "ai/dup.md";
        seed(&[cp]);
        let first = tag_attach(dir, cp, &json!({ "value": "x" }));
        let key = first["key"].as_str().unwrap().to_string();
        let second = tag_attach(dir, cp, &json!({ "key": key, "value": "x" }));
        assert_eq!(second["idempotent"], true);
        assert_eq!(read_registry()["keys"][&key]["refs"], 1);
    });
}

#[test]
fn tag_attach_unknown_key_is_400() {
    with_sandbox_notes(|dir, _| {
        seed(&["ai/unknown.md"]);
        let v = tag_attach(dir, "ai/unknown.md", &json!({ "key": "ghost0000000" }));
        assert_eq!(v["error"], "Unknown tag key");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn tag_detach_drops_the_tag_nobody_references() {
    with_sandbox_notes(|dir, _| {
        let cp = "ai/detach.md";
        seed(&[cp]);
        let attached = tag_attach(dir, cp, &json!({ "value": "gone" }));
        let key = attached["key"].as_str().unwrap().to_string();
        assert_eq!(tag_detach(dir, cp, &key)["ok"], true);

        assert!(read_registry()["keys"].get(&key).is_none());
        assert!(read_annotation_object(cp).get("tag_keys").is_none());
    });
}

#[test]
fn tag_detach_keeps_a_tag_another_note_still_has() {
    with_sandbox_notes(|dir, _| {
        seed(&["ai/one.md", "ai/two.md"]);
        let key = tag_attach(dir, "ai/one.md", &json!({ "value": "shared" }))["key"]
            .as_str()
            .unwrap()
            .to_string();
        tag_attach(dir, "ai/two.md", &json!({ "key": key }));
        assert_eq!(read_registry()["keys"][&key]["refs"], 2);
        tag_detach(dir, "ai/one.md", &key);
        assert_eq!(read_registry()["keys"][&key]["refs"], 1);
    });
}

#[test]
fn tag_update_value_renames_without_touching_the_notes() {
    with_sandbox_notes(|dir, _| {
        let cp = "ai/update-val.md";
        seed(&[cp]);
        let attached = tag_attach(dir, cp, &json!({ "value": "old" }));
        let key = attached["key"].as_str().unwrap().to_string();
        assert_eq!(tag_update_value(dir, &key, "new")["ok"], true);

        assert_eq!(read_registry()["keys"][&key]["value"], "new");
        assert_eq!(read_annotation_object(cp)["tag_keys"][0], key);
        assert_eq!(tag_update_value(dir, "ghost0000000", "x")["_status"], 400);
    });
}

#[test]
fn tag_attach_reuses_existing_key_for_same_value() {
    with_sandbox_notes(|dir, _| {
        seed(&["ai/first.md", "ai/second.md"]);
        let first = tag_attach(dir, "ai/first.md", &json!({ "value": "Obsidian" }));
        let key = first["key"].as_str().expect("key").to_string();

        let second = tag_attach(dir, "ai/second.md", &json!({ "value": "Obsidian" }));
        assert_eq!(second["key"].as_str(), Some(key.as_str()));

        let reg = read_registry();
        assert_eq!(reg["keys"].as_object().unwrap().len(), 1);
        assert_eq!(reg["keys"][&key]["refs"], 2);
    });
}

#[test]
fn tag_attach_rejects_empty_value() {
    with_sandbox_notes(|dir, _| {
        seed(&["ai/bad.md"]);
        let v = tag_attach(dir, "ai/bad.md", &json!({ "value": "   " }));
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn registry_shape_is_keys_with_value_and_refs() {
    with_sandbox_notes(|dir, _| {
        assert_eq!(read_registry(), json!({ "keys": {} }));
        seed(&["ai/shape.md"]);
        let key = tag_attach(dir, "ai/shape.md", &json!({ "value": "a" }))["key"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(read_registry(), json!({ "keys": { key: { "value": "a", "refs": 1 } } }));
    });
}

#[test]
fn delete_entry_prunes_the_tag_it_was_the_last_to_use() {
    with_sandbox_notes(|dir, notes| {
        let cp = "ai/del.md";
        let entry_id = "a1b2c3d4e5f6789012345678901234ab";
        seed_note_entries(&json!({ entry_id: { "common_path": cp } }));
        fs::create_dir_all(notes.join("raw/ai")).expect("mkdir");
        fs::write(notes.join("raw/ai/del.md"), "# x").expect("md");

        let attached = tag_attach(dir, cp, &json!({ "value": "t" }));
        let key = attached["key"].as_str().unwrap().to_string();

        let v = crate::services::entry_admin::delete_entry(&json!({ "id": entry_id }));
        assert_eq!(v["ok"], true);

        assert!(read_registry()["keys"].get(&key).is_none());
        assert!(get_note(&open().expect("db"), entry_id).expect("get").is_none());
    });
}
