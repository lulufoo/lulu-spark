use std::fs;
use std::path::{Path, PathBuf};

use lulu_spark_lib::services::spark_data::{
    find_knowledge_doc_id, get_knowledge_doc, get_note, open_at,
};
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::migration::run_migration;

fn write_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, serde_json::to_string_pretty(value).expect("json")).expect("write");
}

fn db_path(dir: &Path) -> PathBuf {
    dir.join("spark-data.sqlite")
}

/// Three notes: one with a digest, translation and annotation; one with a title and
/// stray fields; one bare. Two tags. One annotated knowledge document.
fn build_fixture(registry_refs: i64) -> TempDir {
    let tmp = TempDir::new().expect("tmp");
    let notes = tmp.path().join("notes");
    write_json(
        &notes.join("index.json"),
        &json!({ "entries": {
            "id-a": { "common_path": "t/a/20260101-a.md", "created_at": "202601010000", "layers": ["raw", "digest"],
                      "source_type": "article", "translations": { "zh": "t/a/20260101-a-zh.md" } },
            "id-b": { "common_path": "t/b/20260102-b.md", "created_at": "202601020000", "layers": ["raw"],
                      "source_type": "summary", "title": "Bee", "fetch": { "url": "https://example.com" } },
            "id-c": { "common_path": "t/c/20260103-c.md", "created_at": "202601030000", "layers": ["raw"],
                      "source_type": "dialogue" },
        }}),
    );
    write_json(
        &notes.join("annotations/t/a/20260101-a.json"),
        &json!({
            "tag_keys": ["k1", "k2"], "importance": "high", "done": true, "links": [{ "url": "https://x" }],
            "raw": { "comments": [{ "id": "c1", "text": "hello", "ts": "202601011200" }],
                     "highlights": [{ "id": "h1", "text": "quote" }] },
            "digest": { "comments": [] },
        }),
    );
    write_json(
        &notes.join("tags/registry.json"),
        &json!({ "keys": { "k1": { "refs": registry_refs, "value": "One" }, "k2": { "refs": 1, "value": "Two" } } }),
    );
    let kb = tmp.path().join("knowledge/repo");
    fs::create_dir_all(kb.join("docs")).expect("mkdir");
    fs::write(kb.join("docs/guide.md"), "# guide").expect("write");
    write_json(
        &kb.join(".knowledge_annotations/docs/guide.json"),
        &json!({ "comments": [{ "id": "kc", "text": "note" }], "links": [] }),
    );
    tmp
}

#[test]
fn imports_reconciles_and_deletes_the_old_files() {
    let tmp = build_fixture(1);
    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.report.notes, 3);
    assert_eq!(outcome.report.comments, 1);
    assert_eq!(outcome.report.tag_refs, 2);
    assert_eq!(outcome.files_deleted, 4);
    assert_eq!(outcome.dropped_fields, 1);

    let conn = open_at(&db_path(tmp.path())).expect("db");
    let a = get_note(&conn, "id-a").expect("get").expect("a");
    assert_eq!(a.raw_path, "t/a/20260101-a.md");
    assert_eq!(a.digest_path.as_deref(), Some("t/a/20260101-a.md"));
    assert_eq!(a.translations, json!(["zh"]));
    assert_eq!(a.importance.as_deref(), Some("high"));
    assert!(a.done);
    assert_eq!(a.tags, json!(["k1", "k2"]));
    assert_eq!(a.comments, json!([{ "id": "c1", "text": "hello", "ts": "202601011200" }]));
    let b = get_note(&conn, "id-b").expect("get").expect("b");
    assert_eq!((b.title.as_deref(), b.digest_path), (Some("Bee"), None));
    let id = find_knowledge_doc_id(&conn, "repo", "docs/guide.md").expect("find").expect("doc");
    let doc = get_knowledge_doc(&conn, &id).expect("get").expect("row");
    assert_eq!(doc.comments, json!([{ "id": "kc", "text": "note" }]));

    assert!(!tmp.path().join("notes/index.json").exists());
    assert!(!tmp.path().join("notes/annotations").exists());
    assert!(!tmp.path().join("notes/tags").exists());
    assert!(!tmp.path().join("knowledge/repo/.knowledge_annotations").exists());
    assert!(tmp.path().join("knowledge/repo/docs/guide.md").is_file(), "documents stay");
    let dropped: Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join("migration-dropped-fields.json")).expect("read"))
            .expect("json");
    assert_eq!(dropped["id-b"]["fetch"]["url"], "https://example.com");
}

fn all_old_files_exist(dir: &Path) -> bool {
    ["notes/index.json", "notes/annotations/t/a/20260101-a.json", "notes/tags/registry.json",
     "knowledge/repo/.knowledge_annotations/docs/guide.json"]
        .iter()
        .all(|rel| dir.join(rel).is_file())
}

fn db_is_empty(dir: &Path) -> bool {
    let conn = open_at(&db_path(dir)).expect("db");
    ["note", "tag", "knowledge_doc"].iter().all(|table| {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get::<_, i64>(0)).expect("count") == 0
    })
}

#[test]
fn a_difference_deletes_nothing_and_writes_nothing() {
    let tmp = build_fixture(5);
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("tag k1"), "{err}");
    assert!(all_old_files_exist(tmp.path()));
    assert!(db_is_empty(tmp.path()));
}

#[test]
fn comments_on_the_digest_layer_stop_the_run() {
    let tmp = build_fixture(1);
    let path = tmp.path().join("notes/annotations/t/a/20260101-a.json");
    let mut ann: Value = serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    ann["digest"] = json!({ "comments": [{ "id": "d1", "text": "wrong layer" }] });
    write_json(&path, &ann);
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("digest layer"), "{err}");
    assert!(all_old_files_exist(tmp.path()));
    assert!(!db_path(tmp.path()).exists() || db_is_empty(tmp.path()));
}

#[test]
fn an_annotation_without_an_entry_stops_the_run() {
    let tmp = build_fixture(1);
    write_json(&tmp.path().join("notes/annotations/t/zzz/orphan.json"), &json!({ "done": true }));
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("no entry in index.json"), "{err}");
    assert!(all_old_files_exist(tmp.path()));
}

#[test]
fn a_translation_path_the_new_model_cannot_derive_stops_the_run() {
    let tmp = build_fixture(1);
    let path = tmp.path().join("notes/index.json");
    let mut index: Value = serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    index["entries"]["id-a"]["translations"]["zh"] = json!("elsewhere/a-zh.md");
    write_json(&path, &index);
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("translation `zh`"), "{err}");
    assert!(all_old_files_exist(tmp.path()));
}

#[test]
fn an_unknown_annotation_field_stops_the_run() {
    let tmp = build_fixture(1);
    let path = tmp.path().join("notes/annotations/t/a/20260101-a.json");
    let mut ann: Value = serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    ann["mystery"] = json!(1);
    write_json(&path, &ann);
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("unknown field `mystery`"), "{err}");
}

#[test]
fn a_directory_without_old_files_is_refused() {
    let tmp = TempDir::new().expect("tmp");
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("no old metadata files"), "{err}");
}
