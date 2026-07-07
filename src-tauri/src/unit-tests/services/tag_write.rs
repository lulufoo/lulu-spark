use super::*;
use std::fs;

use serde_json::json;

use crate::services::annotation::read_annotation_object;
use crate::services::tags_registry::{read_registry, registry_path};
use crate::test_support::with_sandbox_corpus;

#[test]
fn tag_attach_new_key_writes_registry_and_annotation() {
    with_sandbox_corpus(true, |dir, corpus| {
        let cp = "ai/tagged.md";
        let v = tag_attach(
            dir,
            cp,
            &json!({ "value": "my-tag" }),
        );
        assert_eq!(v["ok"], true);
        let key = v["key"].as_str().expect("key");
        assert_eq!(key.len(), 12);

        let reg = read_registry(&corpus);
        assert_eq!(reg["keys"][key]["value"], "my-tag");
        assert_eq!(reg["keys"][key]["refs"], 1);

        let ann = read_annotation_object(&corpus, cp);
        let keys = ann["tag_keys"].as_array().expect("tag_keys");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);
    });
}

#[test]
fn tag_attach_same_key_twice_is_idempotent() {
    with_sandbox_corpus(true, |dir, corpus| {
        let cp = "ai/dup.md";
        let first = tag_attach(dir, cp, &json!({ "value": "x" }));
        let key = first["key"].as_str().unwrap().to_string();
        let second = tag_attach(dir, cp, &json!({ "key": key, "value": "x" }));
        assert_eq!(second["idempotent"], true);

        let reg = read_registry(&corpus);
        assert_eq!(reg["keys"][&key]["refs"], 1);
    });
}

#[test]
fn tag_detach_removes_key_at_zero_refs() {
    with_sandbox_corpus(true, |dir, corpus| {
        let cp = "ai/detach.md";
        let attached = tag_attach(dir, cp, &json!({ "value": "gone" }));
        let key = attached["key"].as_str().unwrap().to_string();
        let v = tag_detach(dir, cp, &key);
        assert_eq!(v["ok"], true);

        let reg = read_registry(&corpus);
        assert!(reg["keys"].get(&key).is_none());
        let ann = read_annotation_object(&corpus, cp);
        assert!(ann.get("tag_keys").is_none());
    });
}

#[test]
fn tag_update_value_only_changes_registry() {
    with_sandbox_corpus(true, |dir, corpus| {
        let cp = "ai/update-val.md";
        let attached = tag_attach(dir, cp, &json!({ "value": "old" }));
        let key = attached["key"].as_str().unwrap().to_string();
        let v = tag_update_value(dir, &key, "new");
        assert_eq!(v["ok"], true);

        let reg = read_registry(&corpus);
        assert_eq!(reg["keys"][&key]["value"], "new");
        let ann = read_annotation_object(&corpus, cp);
        assert_eq!(ann["tag_keys"][0], key);
    });
}

#[test]
fn tag_attach_reuses_existing_key_for_same_value() {
    with_sandbox_corpus(true, |dir, corpus| {
        let cp1 = "ai/first.md";
        let cp2 = "ai/second.md";
        let first = tag_attach(dir, cp1, &json!({ "value": "Obsidian" }));
        let key = first["key"].as_str().expect("key").to_string();

        let second = tag_attach(dir, cp2, &json!({ "value": "Obsidian" }));
        assert_eq!(second["ok"], true);
        assert_eq!(second["key"].as_str(), Some(key.as_str()));

        let reg = read_registry(&corpus);
        assert_eq!(reg["keys"].as_object().unwrap().len(), 1);
        assert_eq!(reg["keys"][&key]["refs"], 2);
    });
}

#[test]
fn tag_attach_rejects_empty_value() {
    with_sandbox_corpus(true, |dir, _| {
        let v = tag_attach(dir, "ai/bad.md", &json!({ "value": "   " }));
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn delete_entry_decrements_refs() {
    with_sandbox_corpus(true, |dir, corpus| {
        let cp = "ai/del.md";
        let entry_id = "a1b2c3d4e5f6789012345678901234ab";
        fs::write(
            corpus.join("index.json"),
            serde_json::to_string(&json!({
                "entries": { entry_id: { "common_path": cp } }
            }))
            .unwrap(),
        )
        .expect("index");
        fs::create_dir_all(corpus.join("raw/ai")).expect("mkdir");
        fs::write(corpus.join("raw/ai/del.md"), "# x").expect("md");

        let attached = tag_attach(dir, cp, &json!({ "value": "t" }));
        let key = attached["key"].as_str().unwrap().to_string();

        let v = crate::services::entry_admin::delete_entry(&json!({ "id": entry_id }));
        assert_eq!(v["ok"], true);

        let reg = read_registry(&corpus);
        assert!(reg["keys"].get(&key).is_none());
        assert!(!registry_path(&corpus).exists() || reg["keys"].as_object().unwrap().is_empty());
    });
}
