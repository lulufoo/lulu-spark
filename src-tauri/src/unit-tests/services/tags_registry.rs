use super::*;
use std::fs;

use serde_json::json;

use crate::repositories::annotation_paths::annotation_json_path;
use crate::repositories::atomic_json;
use crate::services::annotation::read_annotation_object;
use crate::test_support::with_sandbox_corpus;

#[test]
fn registry_path_points_under_corpus_tags() {
    with_sandbox_corpus(false, |_dir, corpus| {
        assert_eq!(
            registry_path(&corpus),
            corpus.join("tags/registry.json")
        );
    });
}

#[test]
fn read_registry_missing_file_returns_empty_keys() {
    with_sandbox_corpus(false, |_dir, corpus| {
        let reg = read_registry(&corpus);
        assert_eq!(reg["keys"], json!({}));
    });
}

#[test]
fn save_registry_creates_tags_dir() {
    with_sandbox_corpus(false, |_dir, corpus| {
        let mut reg = empty_registry();
        adjust_refs(
            &mut reg,
            &["k1".to_string()],
            1,
        );
        reg["keys"]["k1"]["value"] = json!("demo");
        assert!(save_registry(&corpus, &reg).is_none());
        assert!(registry_path(&corpus).is_file());
    });
}

#[test]
fn adjust_refs_increments_and_removes_at_zero() {
    let mut reg = empty_registry();
    reg["keys"]["k1"] = json!({ "value": "a", "refs": 0 });
    adjust_refs(&mut reg, &["k1".to_string()], 2);
    assert_eq!(reg["keys"]["k1"]["refs"], 2);
    adjust_refs(&mut reg, &["k1".to_string()], -2);
    assert!(reg["keys"].get("k1").is_none());
}

#[test]
fn adjust_refs_empty_keys_no_panic() {
    let mut reg = empty_registry();
    adjust_refs(&mut reg, &[], 1);
    assert_eq!(reg["keys"], json!({}));
}

#[test]
fn reconcile_drops_registry_key_with_zero_refs() {
    with_sandbox_corpus(true, |dir, corpus| {
        let cp1 = "ai/a.md";
        let cp2 = "ai/b.md";
        fs::write(corpus.join("index.json"), index_two_entries(cp1, cp2)).expect("index");
        write_annotation(&corpus, cp1, json!({ "tag_keys": ["tagabc123456"] }));
        write_annotation(&corpus, cp2, json!({ "tag_keys": ["tagabc123456"] }));

        let mut reg = empty_registry();
        reg["keys"]["tagabc123456"] = json!({ "value": "x", "refs": 99 });
        assert!(save_registry(&corpus, &reg).is_none());

        assert!(reconcile_tags(dir).is_none());
        let after = read_registry(&corpus);
        assert_eq!(after["keys"]["tagabc123456"]["refs"], 2);

        write_annotation(&corpus, cp1, json!({}));
        write_annotation(&corpus, cp2, json!({}));
        assert!(reconcile_tags(dir).is_none());
        let final_reg = read_registry(&corpus);
        assert!(final_reg["keys"].get("tagabc123456").is_none());
    });
}

#[test]
fn reconcile_removes_stale_tag_keys_from_annotation() {
    with_sandbox_corpus(true, |dir, corpus| {
        let cp = "ai/stale.md";
        fs::write(corpus.join("index.json"), index_one_entry(cp)).expect("index");
        write_annotation(
            &corpus,
            cp,
            json!({ "tag_keys": ["ghostkey123456", "realkey12345678"] }),
        );
        let mut reg = empty_registry();
        reg["keys"]["realkey12345678"] = json!({ "value": "ok", "refs": 0 });
        assert!(save_registry(&corpus, &reg).is_none());

        assert!(reconcile_tags(dir).is_none());

        let ann = read_annotation_object(&corpus, cp);
        let keys = ann["tag_keys"].as_array().expect("tag_keys");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "realkey12345678");

        let reg = read_registry(&corpus);
        assert_eq!(reg["keys"]["realkey12345678"]["refs"], 1);
        assert!(reg["keys"].get("ghostkey123456").is_none());
    });
}

fn index_one_entry(common_path: &str) -> String {
    serde_json::to_string(&json!({
        "entries": { "e1": { "common_path": common_path } }
    }))
    .expect("index json")
}

fn index_two_entries(cp1: &str, cp2: &str) -> String {
    serde_json::to_string(&json!({
        "entries": {
            "e1": { "common_path": cp1 },
            "e2": { "common_path": cp2 }
        }
    }))
    .expect("index json")
}

fn write_annotation(corpus: &std::path::Path, common_path: &str, ann: serde_json::Value) {
    let path = annotation_json_path(corpus, common_path).expect("ann path");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    atomic_json::write_json(&path, &ann).expect("write ann");
}
