use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use super::*;
use crate::services::archive_parse::expected_lang_common_path;
use crate::services::spark_data::{self, find_doc, new_doc_id, DocRow};
use crate::test_support::TestSandbox;

const A: &str = "notes/202601011200-alpha1.md";
const B: &str = "other/202601021300-beta22.md";

struct Setup {
    sandbox: TestSandbox,
    ws: String,
    inbox: String,
    project: String,
}

fn setup() -> Setup {
    let sandbox = TestSandbox::new();
    let ws = create_workspace(&json!({ "workspace_name": "Work" }))["workspace_id"].as_str().expect("ws").to_string();
    let conn = spark_data::open().expect("db");
    let inbox = spark_data::get_inbox_project(&conn, &ws).expect("lookup").expect("inbox").id;
    let made = create_project(&json!({ "workspace_id": ws, "project_name": "Plans" }));
    let project = made["project_id"].as_str().expect("project").to_string();
    Setup { sandbox, ws, inbox, project }
}

impl Setup {
    fn root(&self) -> PathBuf {
        self.sandbox.data_dir().join("workspaces").join(&self.ws)
    }

    fn file(&self, layer: &str, project: &str, rel: &str) -> PathBuf {
        self.root().join(layer).join(project).join(rel)
    }

    fn put(&self, layer: &str, project: &str, rel: &str, tag: &str) {
        let path = self.file(layer, project, rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, format!("{tag}:{project}:{rel}")).expect("write");
    }

    fn text(&self, layer: &str, project: &str, rel: &str) -> String {
        fs::read_to_string(self.file(layer, project, rel)).unwrap_or_else(|_| panic!("missing {layer}/{project}/{rel}"))
    }

    /// A document row and its files: raw, a digest when asked, one file per translation language.
    fn seed(&self, project: &str, rel: &str, digest: bool, langs: &[&str]) -> String {
        let conn = spark_data::open().expect("db");
        let id = new_doc_id();
        let mut row = DocRow::new(&id, rel, project);
        row.digest_path = digest.then(|| rel.to_string());
        row.translations = json!(langs);
        spark_data::insert_doc(&conn, &row).expect("insert");
        self.put("raw", project, rel, "raw");
        if digest {
            self.put("digest", project, rel, "digest");
        }
        for lang in langs {
            self.put("raw", project, &expected_lang_common_path(rel, lang).expect("path"), lang);
        }
        id
    }

    fn row(&self, id: &str) -> DocRow {
        find_doc(&spark_data::open().expect("db"), id).expect("find").expect("doc").doc
    }

    fn delete(&self, project: &str) -> Value {
        delete_project(&json!({ "workspace_id": self.ws, "project_id": project }))
    }

    fn has_project(&self, project: &str) -> bool {
        spark_data::get_project(&spark_data::open().expect("db"), project).expect("get").is_some()
    }
}

fn status(value: &Value) -> u64 {
    value.get("_status").and_then(Value::as_u64).unwrap_or_else(|| panic!("expected an error, got {value}"))
}

#[test]
fn every_document_moves_to_the_inbox_and_the_old_folders_go() {
    let s = setup();
    let kept = s.seed(&s.inbox, B, false, &[]);
    let one = s.seed(&s.project, A, true, &["zh"]);
    let two = s.seed(&s.project, B.replace("beta22", "gamma3").as_str(), false, &[]);

    let done = s.delete(&s.project);
    assert_eq!(done["ok"], json!(true), "{done}");
    assert_eq!(
        (done["moved"].clone(), done["renamed"].clone(), done["target_project_id"].clone(), done["target_project_name"].clone()),
        (json!(2), json!(0), json!(s.inbox), json!("Inbox"))
    );

    assert!(!s.has_project(&s.project));
    for id in [&one, &two] {
        assert_eq!(s.row(id).project_id, s.inbox);
    }
    assert_eq!(s.row(&kept).raw_path, B);
    let zh = expected_lang_common_path(A, "zh").expect("zh");
    let zh_old = zh.clone();
    assert_eq!(s.text("raw", &s.inbox, A), format!("raw:{}:{A}", s.project));
    assert_eq!(s.text("digest", &s.inbox, A), format!("digest:{}:{A}", s.project));
    assert_eq!(s.text("raw", &s.inbox, &zh), format!("zh:{}:{zh_old}", s.project));
    assert!(!s.root().join("raw").join(&s.project).exists());
    assert!(!s.root().join("digest").join(&s.project).exists());
    let read = get_file(&json!({ "workspace_id": s.ws, "id": one, "lang": "zh" }));
    assert_eq!(read["content"], json!(format!("zh:{}:{zh_old}", s.project)));
}

#[test]
fn a_name_clash_renames_the_whole_document_and_overwrites_nothing() {
    let s = setup();
    s.seed(&s.inbox, A, true, &[]);
    let id = s.seed(&s.project, A, true, &["zh"]);

    let done = s.delete(&s.project);
    assert_eq!((done["moved"].clone(), done["renamed"].clone()), (json!(1), json!(1)), "{done}");

    let row = s.row(&id);
    let stem = row.raw_path.strip_suffix(".md").expect("md");
    assert!(stem.starts_with("notes/202601011200-alpha1-") && stem.len() == "notes/202601011200-alpha1-".len() + 6, "{stem}");
    assert_eq!(row.digest_path.as_deref(), Some(row.raw_path.as_str()));
    let zh = expected_lang_common_path(&row.raw_path, "zh").expect("zh");
    let zh_old = expected_lang_common_path(A, "zh").expect("zh");
    assert_eq!(s.text("raw", &s.inbox, &row.raw_path), format!("raw:{}:{A}", s.project));
    assert_eq!(s.text("digest", &s.inbox, &row.raw_path), format!("digest:{}:{A}", s.project));
    assert_eq!(s.text("raw", &s.inbox, &zh), format!("zh:{}:{zh_old}", s.project));
    assert_eq!(s.text("raw", &s.inbox, A), format!("raw:{}:{A}", s.inbox), "the inbox file is untouched");
    assert_eq!(s.text("digest", &s.inbox, A), format!("digest:{}:{A}", s.inbox));
}

#[test]
fn the_built_in_project_cannot_be_deleted() {
    let s = setup();
    let id = s.seed(&s.inbox, A, false, &[]);
    let refused = s.delete(&s.inbox);
    assert_eq!((status(&refused), refused["error"].clone()), (400, json!("Cannot delete the built-in project")));
    assert!(s.has_project(&s.inbox));
    assert_eq!(s.row(&id).project_id, s.inbox);
}

#[test]
fn bad_requests_and_a_disabled_workspace_are_refused() {
    let s = setup();
    assert_eq!(status(&delete_project(&json!({ "workspace_id": s.ws }))), 400);
    assert_eq!(status(&s.delete("nope")), 400);
    let other = create_workspace(&json!({ "workspace_name": "Other" }))["workspace_id"].as_str().expect("ws").to_string();
    assert_eq!(status(&delete_project(&json!({ "workspace_id": other, "project_id": s.project }))), 400);
    disable_workspace(&json!({ "workspace_id": s.ws }));
    assert_eq!(status(&s.delete(&s.project)), 409);
    assert!(s.has_project(&s.project));
}

#[test]
fn an_empty_project_is_simply_deleted() {
    let s = setup();
    let done = s.delete(&s.project);
    assert_eq!((done["moved"].clone(), done["renamed"].clone()), (json!(0), json!(0)), "{done}");
    assert!(!s.has_project(&s.project));
}

#[test]
fn a_failed_file_move_puts_everything_back() {
    let s = setup();
    let id = s.seed(&s.project, A, true, &["zh"]);
    // The digest folder `notes` cannot be made in the inbox: a file sits there. The raw files
    // have moved by then.
    let blocker = s.file("digest", &s.inbox, "notes");
    fs::create_dir_all(blocker.parent().expect("parent")).expect("dirs");
    fs::write(&blocker, "in the way").expect("blocker");

    let failed = s.delete(&s.project);
    assert_eq!(status(&failed), 500, "{failed}");
    assert!(s.has_project(&s.project));
    assert_eq!(s.row(&id).project_id, s.project);
    assert_eq!(s.text("raw", &s.project, A), format!("raw:{}:{A}", s.project));
    assert_eq!(s.text("digest", &s.project, A), format!("digest:{}:{A}", s.project));
    assert!(s.file("raw", &s.project, &expected_lang_common_path(A, "zh").expect("zh")).is_file());
    assert!(!s.file("raw", &s.inbox, "notes").exists(), "the folder made for the move is gone again");
    assert!(blocker.is_file());
}

#[test]
fn a_failed_database_write_puts_the_files_and_rows_back() {
    let s = setup();
    s.seed(&s.inbox, A, true, &[]);
    let id = s.seed(&s.project, A, true, &["zh"]);
    spark_data::open()
        .expect("db")
        .execute_batch("CREATE TRIGGER no_project_delete BEFORE DELETE ON project BEGIN SELECT RAISE(ABORT, 'boom'); END;")
        .expect("trigger");

    let failed = s.delete(&s.project);
    assert_eq!(status(&failed), 500, "{failed}");
    assert!(s.has_project(&s.project));
    let row = s.row(&id);
    assert_eq!((row.project_id.as_str(), row.raw_path.as_str()), (s.project.as_str(), A));
    assert_eq!(s.text("raw", &s.project, A), format!("raw:{}:{A}", s.project));
    assert_eq!(s.text("digest", &s.project, A), format!("digest:{}:{A}", s.project));
    assert_eq!(s.text("raw", &s.inbox, A), format!("raw:{}:{A}", s.inbox));
    let renamed = fs::read_dir(s.file("raw", &s.inbox, "notes")).expect("dir").count();
    assert_eq!(renamed, 1, "only the inbox's own file is left there");
}
