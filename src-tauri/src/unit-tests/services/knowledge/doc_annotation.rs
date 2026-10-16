use super::*;
use crate::services::knowledge::{
    kb_reorder_comments, kb_update_comments, kb_update_highlights, kb_update_links,
};
use crate::services::spark_data;
use crate::test_support::TestSandbox;
use serde_json::json;
use std::fs;

fn with_doc<F: FnOnce(&Path)>(f: F) {
    let sandbox = TestSandbox::new();
    let kb = sandbox.knowledge_root();
    fs::create_dir_all(kb.join("myrepo/docs")).expect("mkdir");
    fs::write(kb.join("myrepo/docs/a.md"), "hello").expect("write");
    f(sandbox.config_dir());
}

fn comment(cfg: &Path, body: Value) -> Value {
    kb_update_comments(cfg, "acme/myrepo".into(), "docs/a.md".into(), body, "t1".into())
}

fn annotation(cfg: &Path) -> Value {
    kb_annotation_json(cfg, "acme/myrepo", "docs/a.md")
}

#[test]
fn a_document_without_a_record_reads_as_empty_and_reading_creates_nothing() {
    with_doc(|cfg| {
        assert_eq!(annotation(cfg), json!({}));
        let conn = spark_data::open().expect("db");
        assert!(spark_data::find_knowledge_doc_id(&conn, "myrepo", "docs/a.md")
            .expect("find")
            .is_none());
    });
}

#[test]
fn comments_add_edit_delete_and_reorder_on_the_table() {
    with_doc(|cfg| {
        let first = comment(cfg, json!({ "text": "one" }));
        let second = comment(cfg, json!({ "text": "two" }));
        let (id1, id2) = (first["id"].as_str().unwrap(), second["id"].as_str().unwrap());
        assert_ne!(id1, id2);
        assert_eq!(annotation(cfg)["comments"].as_array().unwrap().len(), 2);

        comment(cfg, json!({ "id": id1, "text": "one edited" }));
        assert_eq!(annotation(cfg)["comments"][0]["text"], "one edited");

        let reordered = kb_reorder_comments(
            cfg,
            "acme/myrepo".into(),
            "docs/a.md".into(),
            vec![id2.to_string(), id1.to_string()],
        );
        assert_eq!(reordered["ok"], true);
        assert_eq!(annotation(cfg)["comments"][0]["id"], id2);

        let missing = kb_reorder_comments(cfg, "acme/myrepo".into(), "docs/a.md".into(), vec!["nope".into()]);
        assert_eq!(missing["_status"], 404);

        comment(cfg, json!({ "id": id1, "text": "" }));
        assert_eq!(annotation(cfg)["comments"].as_array().unwrap().len(), 1);
        comment(cfg, json!({ "id": id2, "text": "" }));
        assert_eq!(annotation(cfg), json!({}), "empty fields are not reported");
    });
}

#[test]
fn highlights_and_links_round_trip_without_a_layer() {
    with_doc(|cfg| {
        let added = kb_update_highlights(
            cfg,
            "acme/myrepo".into(),
            "docs/a.md".into(),
            json!({ "text": "hello", "occurrence": 2 }),
            "t1".into(),
        );
        let hid = added["id"].as_str().unwrap().to_string();
        let ann = annotation(cfg);
        assert_eq!(ann["highlights"][0]["text"], "hello");
        assert_eq!(ann["highlights"][0]["occurrence"], 2);
        assert!(ann["highlights"][0].get("layer").is_none());

        let ok = kb_update_links(
            cfg,
            "acme/myrepo".into(),
            "docs/a.md".into(),
            json!([{ "url": "https://example.com" }]),
        );
        assert_eq!(ok["ok"], true);
        assert_eq!(annotation(cfg)["links"][0]["url"], "https://example.com");
        let bad = kb_update_links(cfg, "acme/myrepo".into(), "docs/a.md".into(), json!([{ "url": "ftp://x" }]));
        assert_eq!(bad["_status"], 400);

        kb_update_highlights(cfg, "acme/myrepo".into(), "docs/a.md".into(), json!({ "id": hid }), "t2".into());
        assert!(annotation(cfg).get("highlights").is_none());
        kb_update_links(cfg, "acme/myrepo".into(), "docs/a.md".into(), json!([]));
        assert_eq!(annotation(cfg), json!({}));
    });
}

#[test]
fn the_same_record_is_reached_by_dir_name_or_owner_slash_name() {
    with_doc(|cfg| {
        comment(cfg, json!({ "text": "via owner/name" }));
        assert_eq!(
            kb_annotation_json(cfg, "myrepo", "docs/a.md")["comments"][0]["text"],
            "via owner/name"
        );
        let conn = spark_data::open().expect("db");
        let count: i64 = conn
            .query_row("SELECT count(*) FROM knowledge_doc", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 1);
    });
}

#[test]
fn writes_need_an_existing_file_and_a_safe_path() {
    with_doc(|cfg| {
        let no_file = kb_update_comments(cfg, "acme/myrepo".into(), "docs/none.md".into(), json!({ "text": "x" }), "t".into());
        assert_eq!(no_file["_status"], 404);
        let escape = kb_update_comments(cfg, "acme/myrepo".into(), "../a.md".into(), json!({ "text": "x" }), "t".into());
        assert_eq!(escape["_status"], 400);
        let conn = spark_data::open().expect("db");
        let count: i64 = conn
            .query_row("SELECT count(*) FROM knowledge_doc", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 0, "rejected writes leave no record behind");
    });
}
