use super::*;
use crate::services::spark_data::{get_note, open};
use crate::test_support::{seed_note_entries, with_sandbox_notes};

/// Seed one note at `cp` and return its row id.
fn seed(cp: &str) -> String {
    let id = format!("{:0>32}", cp.len());
    seed_note_entries(&json!({ id.clone(): { "common_path": cp } }));
    id
}

fn row(id: &str) -> crate::services::spark_data::NoteRow {
    get_note(&open().expect("db"), id).expect("get").expect("row")
}

#[test]
fn set_done_true_marks_the_note_and_false_clears_it() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-done.md";
        let id = seed(cp);
        assert_eq!(set_done(dir, cp, true)["ok"], json!(true));
        assert!(row(&id).done);
        assert_eq!(read_annotation_object(cp)["done"], json!(true));
        set_done(dir, cp, false);
        assert!(!row(&id).done);
        assert!(read_annotation_object(cp).get("done").is_none());
    });
}

#[test]
fn set_importance_high_then_cleared() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-importance.md";
        let id = seed(cp);
        assert_eq!(set_importance(dir, cp, Some("high".into()))["ok"], json!(true));
        assert_eq!(row(&id).importance.as_deref(), Some("high"));
        assert_eq!(set_importance(dir, cp, Some("urgent".into()))["_status"], json!(400));
        set_importance(dir, cp, None);
        assert_eq!(row(&id).importance, None);
    });
}

#[test]
fn an_annotation_write_on_an_unknown_path_is_404() {
    with_sandbox_notes(true, |dir, _| {
        let v = set_done(dir, "ai/nothing-here.md", true);
        assert_eq!(v["error"], "Entry not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn two_notes_sharing_a_path_refuse_the_write() {
    with_sandbox_notes(true, |dir, _| {
        seed_note_entries(&json!({
            "a".repeat(32): { "common_path": "ai/shared.md" },
            "b".repeat(32): { "common_path": "ai/shared.md" },
        }));
        let v = set_done(dir, "ai/shared.md", true);
        assert_eq!(v["_status"], 409);
    });
}

#[test]
fn update_comments_new_returns_id_and_stores_it_on_the_row() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-comments.md";
        let id = seed(cp);
        let v = update_comments(dir, cp, "raw", json!({ "text": "hi" }), "202601011200".into());
        assert_eq!(v["ok"], json!(true));
        let cid = v["id"].as_str().expect("id").to_string();
        assert_eq!(cid.len(), 12);
        assert_eq!(row(&id).comments, json!([{ "id": cid, "text": "hi", "ts": "202601011200" }]));
        assert_eq!(read_annotation_object(cp)["raw"]["comments"][0]["text"], "hi");

        let edited = update_comments(dir, cp, "raw", json!({ "id": cid, "text": "edited" }), String::new());
        assert_eq!(edited["id"], json!(cid));
        assert_eq!(row(&id).comments[0]["text"], "edited");
        assert_eq!(row(&id).comments[0]["ts"], "202601011200", "an empty ts keeps the old one");

        let removed = update_comments(dir, cp, "raw", json!({ "id": cid, "text": "" }), String::new());
        assert_eq!(removed["id"], json!(""));
        assert_eq!(row(&id).comments, json!([]));
        assert!(read_annotation_object(cp).get("raw").is_none());
    });
}

#[test]
fn comments_and_highlights_only_exist_on_the_raw_layer() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-layers.md";
        let id = seed(cp);
        let c = update_comments(dir, cp, "digest", json!({ "text": "x" }), String::new());
        assert_eq!(c["error"], "Invalid layer: digest");
        assert_eq!(c["_status"], 400);
        let h = update_highlights(dir, cp, "digest", json!({ "text": "x" }), String::new());
        assert_eq!(h["error"], "Invalid layer: digest");
        let r = reorder_comments(dir, cp, "digest", vec![]);
        assert_eq!(r["error"], "Invalid layer: digest");
        assert_eq!(row(&id).comments, json!([]));
        assert_eq!(row(&id).highlights, json!([]));
    });
}

#[test]
fn update_comments_rejects_an_empty_new_comment_and_an_unknown_id() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-comment-errors.md";
        seed(cp);
        assert_eq!(update_comments(dir, cp, "raw", json!({ "text": " " }), String::new())["_status"], 400);
        let v = update_comments(dir, cp, "raw", json!({ "id": "ghost", "text": "x" }), String::new());
        assert_eq!(v["error"], "Comment not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn highlights_add_and_remove_by_id() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-highlights.md";
        let id = seed(cp);
        let v = update_highlights(dir, cp, "raw", json!({ "text": "quote", "occurrence": 2 }), "ts1".into());
        let hid = v["id"].as_str().expect("id").to_string();
        assert_eq!(row(&id).highlights[0]["occurrence"], json!(2));
        assert_eq!(row(&id).highlights[0]["text"], json!("quote"));
        update_highlights(dir, cp, "raw", json!({ "id": hid }), String::new());
        assert_eq!(row(&id).highlights, json!([]));
        let empty = update_highlights(dir, cp, "raw", json!({ "text": "" }), String::new());
        assert_eq!(empty["_status"], 400);
    });
}

#[test]
fn update_links_invalid_url() {
    with_sandbox_notes(true, |dir, _| {
        seed("ai/note-links-bad.md");
        let v = update_links(dir, "ai/note-links-bad.md", json!([{ "url": "ftp://x" }]));
        assert_eq!(v["_status"], json!(400));
        assert_eq!(update_links(dir, "ai/note-links-bad.md", json!({}))["error"], "links must be array");
    });
}

#[test]
fn update_links_invalid_common_path() {
    with_sandbox_notes(true, |dir, _| {
        let v = update_links(dir, "../evil.md", json!([{ "url": "https://github.com/foo/bar" }]));
        assert_eq!(v["error"], "Invalid common_path");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn update_links_persists_and_an_empty_array_clears_them() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-links.md";
        let id = seed(cp);
        let links = json!([{ "url": "https://github.com/foo/bar" }]);
        assert_eq!(update_links(dir, cp, links.clone())["ok"], json!(true));
        assert_eq!(row(&id).links, links);
        assert_eq!(read_annotation_object(cp)["links"], links);
        update_links(dir, cp, json!([]));
        assert!(read_annotation_object(cp).get("links").is_none());
    });
}

#[test]
fn reorder_comments_persists_new_order() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-reorder.md";
        let id = seed(cp);
        let ids: Vec<String> = ["first", "second", "third"]
            .iter()
            .map(|t| update_comments(dir, cp, "raw", json!({ "text": t }), String::new())["id"].as_str().expect("id").to_string())
            .collect();
        let want = vec![ids[2].clone(), ids[0].clone(), ids[1].clone()];
        assert_eq!(reorder_comments(dir, cp, "raw", want.clone())["ok"], json!(true));
        let got: Vec<String> = row(&id)
            .comments
            .as_array()
            .expect("comments")
            .iter()
            .filter_map(|c| c["id"].as_str().map(String::from))
            .collect();
        assert_eq!(got, want);
    });
}

#[test]
fn reorder_comments_rejects_unknown_id_and_keeps_the_order() {
    with_sandbox_notes(true, |dir, _| {
        let cp = "ai/note-reorder-bad.md";
        let id = seed(cp);
        let first = update_comments(dir, cp, "raw", json!({ "text": "only" }), String::new());
        let cid = first["id"].as_str().expect("id").to_string();
        let v = reorder_comments(dir, cp, "raw", vec![cid.clone(), "nope00000000".into()]);
        assert_eq!(v["error"], "Comment id not found");
        assert_eq!(v["_status"], 404);
        assert_eq!(row(&id).comments[0]["id"], json!(cid));
    });
}

#[test]
fn reorder_invalid_common_path_returns_400() {
    with_sandbox_notes(true, |dir, _| {
        let v = reorder_comments(dir, "../evil.md", "raw", vec!["x".into()]);
        assert_eq!(v["error"], "Invalid common_path");
        assert_eq!(v["_status"], 400);
    });
}
