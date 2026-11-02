use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use super::move_project;
use crate::config::paths;
use crate::services::docs::{create_doc_content, create_project, create_workspace, disable_workspace, search};
use crate::services::search_index;
use crate::services::spark_data::{self, find_doc, new_doc_id, DocRow};
use crate::test_support::TestSandbox;

const A: &str = "notes/202601011200-alpha1.md";
const B: &str = "other/202601021300-beta22.md";
const WORD: &str = "迁项目关键词";

struct Setup {
    sandbox: TestSandbox,
    src: String,
    dest: String,
    source: String,
    target: String,
}

fn setup() -> Setup {
    let sandbox = TestSandbox::new();
    let dest = crate::test_support::default_workspace().id;
    let src = create_workspace(&json!({ "workspace_name": "Work" }))["workspace_id"]
        .as_str()
        .expect("ws")
        .to_string();
    let source = create_project(&json!({ "workspace_id": src, "project_name": "Plans" }))["project_id"]
        .as_str()
        .expect("source")
        .to_string();
    let target = create_project(&json!({ "workspace_id": dest, "project_name": "Shelf" }))["project_id"]
        .as_str()
        .expect("target")
        .to_string();
    Setup { sandbox, src, dest, source, target }
}

impl Setup {
    fn root(&self, workspace: &str) -> PathBuf {
        self.sandbox.data_dir().join("workspaces").join(workspace)
    }

    fn file(&self, workspace: &str, layer: &str, project: &str, rel: &str) -> PathBuf {
        self.root(workspace).join(layer).join(project).join(rel)
    }

    fn seed(&self, project: &str, rel: &str) -> String {
        let conn = spark_data::open().expect("db");
        let id = new_doc_id();
        spark_data::insert_doc(&conn, &DocRow::new(&id, rel, project)).expect("insert");
        let path = self.file(&self.src, "raw", project, rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(&path, format!("raw:{project}:{rel}")).expect("write");
        id
    }

    fn move_to(&self, target_workspace: &str, target_project: &str) -> Value {
        move_project(&json!({
            "workspace_id": self.src,
            "project_id": self.source,
            "target_workspace_id": target_workspace,
            "target_project_id": target_project,
        }))
    }

    fn has_project(&self, id: &str) -> bool {
        spark_data::get_project(&spark_data::open().expect("db"), id)
            .expect("get")
            .is_some()
    }

    fn row(&self, id: &str) -> DocRow {
        find_doc(&spark_data::open().expect("db"), id).expect("find").expect("doc").doc
    }
}

fn status(value: &Value) -> u64 {
    value.get("_status").and_then(Value::as_u64).unwrap_or_else(|| panic!("expected an error, got {value}"))
}

#[test]
fn documents_and_attachments_move_and_the_source_project_is_gone() {
    let s = setup();
    let one = s.seed(&s.source, A);
    let attached = s
        .file(&s.src, "raw", &s.source, A)
        .parent()
        .expect("md")
        .join(".attached")
        .join(&one)
        .join("pic.png");
    fs::create_dir_all(attached.parent().expect("attached")).expect("dirs");
    fs::write(&attached, b"png").expect("pic");

    let done = s.move_to(&s.dest, &s.target);
    assert_eq!(done["ok"], json!(true), "{done}");
    assert_eq!(done["moved"], json!(1));
    assert_eq!(done["target_project_id"], json!(s.target));
    assert!(!s.has_project(&s.source));
    let row = s.row(&one);
    assert_eq!(row.doc_id, one);
    assert_eq!(row.project_id, s.target);
    let picture = s
        .file(&s.dest, "raw", &s.target, A)
        .parent()
        .unwrap()
        .join(".attached")
        .join(&one)
        .join("pic.png");
    assert_eq!(fs::read(picture).expect("attached"), b"png");
    assert!(!s.file(&s.src, "raw", &s.source, A).exists());
    assert!(!s.root(&s.src).join("raw").join(&s.source).exists());
}

#[test]
fn a_project_merges_into_another_project_in_the_same_workspace() {
    let s = setup();
    let shelf = create_project(&json!({ "workspace_id": s.src, "project_name": "Shelf" }))["project_id"]
        .as_str()
        .expect("shelf")
        .to_string();
    let id = s.seed(&s.source, A);

    let done = s.move_to(&s.src, &shelf);
    assert_eq!(done["ok"], json!(true), "{done}");
    assert_eq!(s.row(&id).project_id, shelf);
    assert_eq!(s.row(&id).doc_id, id);
    assert!(!s.has_project(&s.source));
}

#[test]
fn an_empty_project_is_deleted() {
    let s = setup();
    let done = s.move_to(&s.dest, &s.target);
    assert_eq!((done["ok"].clone(), done["moved"].clone()), (json!(true), json!(0)), "{done}");
    assert!(!s.has_project(&s.source));
    assert!(s.has_project(&s.target));
}

#[test]
fn the_built_in_project_and_a_move_onto_itself_are_refused() {
    let s = setup();
    let inbox = crate::test_support::default_inbox_id();
    let built_in = move_project(&json!({
        "workspace_id": s.dest,
        "project_id": inbox,
        "target_workspace_id": s.src,
        "target_project_id": s.source,
    }));
    assert_eq!(
        (status(&built_in), built_in["error"].clone()),
        (400, json!("Cannot move the built-in project"))
    );
    let same = s.move_to(&s.src, &s.source);
    assert_eq!((status(&same), same["error"].clone()), (400, json!("Already in this project")));
    assert!(s.has_project(&s.source));
}

#[test]
fn a_disabled_target_workspace_is_refused() {
    let s = setup();
    let other = create_workspace(&json!({ "workspace_name": "Away" }))["workspace_id"]
        .as_str()
        .expect("other")
        .to_string();
    let shelf = create_project(&json!({ "workspace_id": other, "project_name": "Shelf" }))["project_id"]
        .as_str()
        .expect("shelf")
        .to_string();
    let disabled = disable_workspace(&json!({ "workspace_id": other }));
    assert_eq!(disabled.get("ok"), Some(&json!(true)), "{disabled}");
    assert_eq!(status(&s.move_to(&other, &shelf)), 409);
    assert!(s.has_project(&s.source));
}

#[test]
fn a_failed_document_stops_and_a_retry_finishes() {
    let s = setup();
    let first = s.seed(&s.source, A);
    let second = s.seed(&s.source, B);
    let blocker = s.root(&s.dest).join("raw").join(&s.target).join("other");
    fs::create_dir_all(blocker.parent().expect("parent")).expect("dirs");
    fs::write(&blocker, "in the way").expect("blocker");

    let failed = s.move_to(&s.dest, &s.target);
    assert_eq!(status(&failed), 500, "{failed}");
    assert!(s.has_project(&s.source));
    assert_eq!(failed["failed_id"], json!(second));
    assert_eq!(s.row(&second).project_id, s.source);

    fs::remove_file(&blocker).expect("unblock");
    let done = s.move_to(&s.dest, &s.target);
    assert_eq!(done["ok"], json!(true), "{done}");
    assert_eq!(s.row(&first).project_id, s.target);
    assert_eq!(s.row(&second).project_id, s.target);
    assert!(!s.has_project(&s.source));
}

#[test]
fn the_index_follows_the_document() {
    let s = setup();
    let created = create_doc_content(&json!({
        "workspace_id": s.src,
        "project_id": s.source,
        "title": "T",
        "content": format!("# t\n\n{WORD}\n"),
        "digest": "never",
        "source_type": "summary",
    }));
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().expect("id").to_string();
    crate::services::index_build::rebuild_spark_index(&paths::repo_root().expect("root")).expect("index");

    let done = s.move_to(&s.dest, &s.target);
    assert_eq!(done["ok"], json!(true), "{done}");

    let hits = search(&json!({ "q": WORD, "workspace_id": s.dest }));
    let ids: Vec<String> = hits["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, vec![id.clone()], "{hits}");

    let conn = search_index::open(&s.sandbox.cache_dir()).expect("index");
    let leftover: i64 = conn
        .query_row("SELECT count(*) FROM chunk WHERE workspace_id = ?1", [&s.src], |row| row.get(0))
        .expect("count");
    assert_eq!(leftover, 0);
}

#[test]
fn cancel_applies_to_the_run_that_is_going_on_and_not_the_next_one() {
    use std::sync::atomic::Ordering;
    let run = super::RUN.fetch_add(1, Ordering::SeqCst) + 1;
    assert!(!super::run_is_cancelled(run));
    super::cancel_move_project();
    assert!(super::run_is_cancelled(run));
    let next = super::RUN.fetch_add(1, Ordering::SeqCst) + 1;
    assert!(!super::run_is_cancelled(next));
}
