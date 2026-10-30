use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use serde_json::{json, Value};

use super::delete_workspace;
use crate::config::paths;
use crate::services::docs::{create_doc_content, create_project, create_workspace, disable_workspace, search};
use crate::services::search_index;
use crate::services::spark_data::{self, find_doc, new_doc_id, DocRow};
use crate::test_support::TestSandbox;

const A: &str = "notes/202601011200-alpha1.md";
const B: &str = "other/202601021300-beta22.md";
const WORD: &str = "删仓关键词";

struct Setup {
    sandbox: TestSandbox,
    src: String,
    dest: String,
    dest_inbox: String,
    project: String,
}

fn setup() -> Setup {
    let sandbox = TestSandbox::new();
    let dest = crate::test_support::default_workspace().id;
    let dest_inbox = crate::test_support::default_inbox_id();
    let src = create_workspace(&json!({ "workspace_name": "Work" }))["workspace_id"]
        .as_str()
        .expect("ws")
        .to_string();
    let project = create_project(&json!({ "workspace_id": src, "project_name": "Plans" }))["project_id"]
        .as_str()
        .expect("project")
        .to_string();
    Setup { sandbox, src, dest, dest_inbox, project }
}

impl Setup {
    fn src_root(&self) -> PathBuf {
        self.sandbox.data_dir().join("workspaces").join(&self.src)
    }

    fn dest_root(&self) -> PathBuf {
        self.sandbox.data_dir().join("workspaces").join(&self.dest)
    }

    fn file(&self, root: &PathBuf, layer: &str, project: &str, rel: &str) -> PathBuf {
        root.join(layer).join(project).join(rel)
    }

    fn put(&self, root: &PathBuf, layer: &str, project: &str, rel: &str, tag: &str) {
        let path = self.file(root, layer, project, rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, format!("{tag}:{project}:{rel}")).expect("write");
    }

    fn seed(&self, project: &str, rel: &str) -> String {
        let conn = spark_data::open().expect("db");
        let id = new_doc_id();
        spark_data::insert_doc(&conn, &DocRow::new(&id, rel, project)).expect("insert");
        self.put(&self.src_root(), "raw", project, rel, "raw");
        id
    }

    fn write_cache(&self) {
        for make in [paths::workspace_hide_patterns_path, paths::workspace_viewer_state_path] {
            let path = make(&self.src).expect("cache path");
            fs::create_dir_all(path.parent().expect("dir")).expect("dirs");
            fs::write(path, "{}").expect("write");
        }
    }

    fn delete(&self) -> Value {
        delete_workspace(&json!({ "workspace_id": self.src }))
    }

    fn has_workspace(&self) -> bool {
        spark_data::get_workspace(&spark_data::open().expect("db"), &self.src)
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
fn documents_and_attachments_move_and_the_workspace_is_gone() {
    let s = setup();
    let one = s.seed(&s.project, A);
    let attached = s
        .file(&s.src_root(), "raw", &s.project, A)
        .parent()
        .expect("md")
        .join(".attached")
        .join(&one)
        .join("pic.png");
    fs::create_dir_all(attached.parent().expect("attached")).expect("dirs");
    fs::write(&attached, b"png").expect("pic");
    s.write_cache();

    let done = s.delete();
    assert_eq!(done["ok"], json!(true), "{done}");
    assert_eq!(
        (done["moved"].clone(), done["projects_removed"].clone(), done["folder_removed"].clone()),
        (json!(1), json!(2), json!(true))
    );
    assert!(!s.has_workspace());
    assert_eq!(s.row(&one).project_id, s.dest_inbox);
    assert_eq!(
        fs::read(s.file(&s.dest_root(), "raw", &s.dest_inbox, A).parent().unwrap().join(".attached").join(&one).join("pic.png"))
            .expect("attached"),
        b"png"
    );
    assert!(!s.src_root().exists());
    assert!(!paths::workspace_hide_patterns_path(&s.src).expect("hide").exists());
    assert!(!paths::workspace_viewer_state_path(&s.src).expect("viewer").exists());
}

#[test]
fn a_name_clash_renames_and_empty_projects_go() {
    let s = setup();
    let dest_file = s.file(&s.dest_root(), "raw", &s.dest_inbox, A);
    fs::create_dir_all(dest_file.parent().expect("parent")).expect("dirs");
    fs::write(&dest_file, b"occupied").expect("taken");
    let id = s.seed(&s.project, A);

    let done = s.delete();
    assert_eq!(done["ok"], json!(true), "{done}");
    let row = s.row(&id);
    assert_eq!(row.project_id, s.dest_inbox);
    assert_ne!(row.raw_path, A);
    assert_eq!(fs::read(&dest_file).expect("kept"), b"occupied");
    assert!(!s.has_workspace());
}

#[test]
fn an_empty_workspace_is_simply_deleted() {
    let s = setup();
    let done = s.delete();
    assert_eq!((done["ok"].clone(), done["moved"].clone(), done["folder_removed"].clone()), (json!(true), json!(0), json!(true)), "{done}");
    assert!(!s.has_workspace());
}

#[test]
fn default_and_disabled_workspaces_are_refused() {
    let s = setup();
    assert_eq!(status(&delete_workspace(&json!({}))), 400);
    let refused = delete_workspace(&json!({ "workspace_id": s.dest }));
    assert_eq!((status(&refused), refused["error"].clone()), (400, json!("Cannot delete the default workspace")));
    disable_workspace(&json!({ "workspace_id": s.src }));
    assert_eq!(status(&s.delete()), 409);
    assert!(s.has_workspace());
}

#[test]
fn a_failed_document_stops_and_a_retry_finishes() {
    let s = setup();
    let first = s.seed(&s.project, A);
    let second = s.seed(&s.project, B);
    let blocker = s.dest_root().join("raw").join(&s.dest_inbox).join("other");
    fs::create_dir_all(blocker.parent().expect("parent")).expect("dirs");
    fs::write(&blocker, "in the way").expect("blocker");

    let failed = s.delete();
    assert_eq!(status(&failed), 500, "{failed}");
    assert!(s.has_workspace());
    assert_eq!(failed["failed_id"], json!(second));
    assert_eq!(s.row(&second).project_id, s.project);

    fs::remove_file(&blocker).expect("unblock");
    let done = s.delete();
    assert_eq!(done["ok"], json!(true), "{done}");
    assert_eq!(s.row(&first).project_id, s.dest_inbox);
    assert_eq!(s.row(&second).project_id, s.dest_inbox);
    assert!(!s.has_workspace());
}

#[test]
fn leftover_files_keep_the_folder() {
    let s = setup();
    let stray = s.src_root().join("raw").join("keep-me.txt");
    fs::create_dir_all(stray.parent().expect("parent")).expect("dirs");
    fs::write(&stray, "manual").expect("stray");
    let done = s.delete();
    assert_eq!((done["ok"].clone(), done["folder_removed"].clone()), (json!(true), json!(false)), "{done}");
    assert!(!s.has_workspace());
    assert_eq!(fs::read_to_string(&stray).expect("kept"), "manual");
}

#[test]
fn the_index_follows_the_document_and_leaves_no_source_chunks() {
    let s = setup();
    let created = create_doc_content(&json!({
        "workspace_id": s.src,
        "project_id": s.project,
        "title": "T",
        "content": format!("# t\n\n{WORD}\n"),
        "digest": "never",
        "source_type": "summary",
    }));
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().expect("id").to_string();
    crate::services::index_build::rebuild_spark_index(&paths::repo_root().expect("root")).expect("index");

    let started = Instant::now();
    let done = s.delete();
    eprintln!("delete_workspace of 1 document took {:?}", started.elapsed());
    assert_eq!(done["ok"], json!(true), "{done}");

    let dest_hits = search(&json!({ "q": WORD, "workspace_id": s.dest }));
    let dest_ids: Vec<String> = dest_hits["items"].as_array().expect("items").iter().map(|i| i["id"].as_str().unwrap().to_string()).collect();
    assert_eq!(dest_ids, vec![id.clone()], "{dest_hits}");

    let cache = s.sandbox.cache_dir();
    let conn = search_index::open(&cache).expect("index");
    let leftover: i64 = conn
        .query_row("SELECT count(*) FROM chunk WHERE workspace_id = ?1", [&s.src], |r| r.get(0))
        .expect("count");
    assert_eq!(leftover, 0, "source workspace chunks stayed");
    let dest_chunks: i64 = conn
        .query_row("SELECT count(*) FROM chunk WHERE doc_id = ?1 AND workspace_id = ?2", [&id, &s.dest], |r| r.get(0))
        .expect("dest");
    assert!(dest_chunks > 0, "moved document is indexed in the dest workspace");
}
