use super::*;
use crate::services::knowledge::{
    kb_reorder_comments, kb_update_comments, kb_update_highlights, kb_update_links,
};
use crate::services::spark_data::{self, register_knowledge_doc};
use crate::test_support::TestSandbox;
use serde_json::json;
use std::fs;

fn with_doc<F: FnOnce(&str)>(f: F) {
    let sandbox = TestSandbox::new();
    let kb = sandbox.knowledge_root();
    fs::create_dir_all(kb.join("myrepo/docs")).expect("mkdir");
    fs::write(kb.join("myrepo/docs/a.md"), "hello").expect("write");
    let id = register_knowledge_doc(&spark_data::open().expect("db"), "myrepo", "docs/a.md")
        .expect("register");
    f(&id);
}

fn comment(id: &str, body: Value) -> Value {
    kb_update_comments(id.into(), body, "t1".into())
}

fn annotation(id: &str) -> Value {
    kb_annotation_json(id)
}

#[test]
fn a_document_without_comments_reads_as_empty() {
    with_doc(|id| {
        assert_eq!(annotation(id), json!({}));
    });
}

#[test]
fn comments_add_edit_delete_and_reorder_on_the_table() {
    with_doc(|id| {
        let first = comment(id, json!({ "text": "one" }));
        let second = comment(id, json!({ "text": "two" }));
        let (id1, id2) = (first["id"].as_str().unwrap(), second["id"].as_str().unwrap());
        assert_ne!(id1, id2);
        assert_eq!(annotation(id)["comments"].as_array().unwrap().len(), 2);

        comment(id, json!({ "id": id1, "text": "one edited" }));
        assert_eq!(annotation(id)["comments"][0]["text"], "one edited");

        let reordered = kb_reorder_comments(id.into(), vec![id2.to_string(), id1.to_string()]);
        assert_eq!(reordered["ok"], true);
        assert_eq!(annotation(id)["comments"][0]["id"], id2);

        let missing = kb_reorder_comments(id.into(), vec!["nope".into()]);
        assert_eq!(missing["_status"], 404);

        comment(id, json!({ "id": id1, "text": "" }));
        assert_eq!(annotation(id)["comments"].as_array().unwrap().len(), 1);
        comment(id, json!({ "id": id2, "text": "" }));
        assert_eq!(annotation(id), json!({}), "empty fields are not reported");
    });
}

#[test]
fn highlights_and_links_round_trip_without_a_layer() {
    with_doc(|id| {
        let added = kb_update_highlights(
            id.into(),
            json!({ "text": "hello", "occurrence": 2 }),
            "t1".into(),
        );
        let hid = added["id"].as_str().unwrap().to_string();
        let ann = annotation(id);
        assert_eq!(ann["highlights"][0]["text"], "hello");
        assert_eq!(ann["highlights"][0]["occurrence"], 2);
        assert!(ann["highlights"][0].get("layer").is_none());

        let ok = kb_update_links(id.into(), json!([{ "url": "https://example.com" }]));
        assert_eq!(ok["ok"], true);
        assert_eq!(annotation(id)["links"][0]["url"], "https://example.com");
        let bad = kb_update_links(id.into(), json!([{ "url": "ftp://x" }]));
        assert_eq!(bad["_status"], 400);

        kb_update_highlights(id.into(), json!({ "id": hid }), "t2".into());
        assert!(annotation(id).get("highlights").is_none());
        kb_update_links(id.into(), json!([]));
        assert_eq!(annotation(id), json!({}));
    });
}

#[test]
fn writes_need_a_known_id_and_an_existing_file() {
    with_doc(|id| {
        let unknown = kb_update_comments(
            "ffffffffffffffffffffffffffffffff".into(),
            json!({ "text": "x" }),
            "t".into(),
        );
        assert_eq!(unknown["error"], "Unknown document id");
        let file = crate::config::paths::knowledge_root()
            .expect("root")
            .join("myrepo/docs/a.md");
        fs::remove_file(&file).expect("remove");
        let gone = kb_update_comments(id.into(), json!({ "text": "x" }), "t".into());
        assert_eq!(gone["error"], "File not found");
    });
}
