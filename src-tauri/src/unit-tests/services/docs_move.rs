use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::services::docs::{
    create_doc, create_doc_content, create_project, create_workspace, disable_workspace, move_doc,
    search,
};
use crate::services::spark_data;
use crate::test_support::TestSandbox;

const BODY: &str = "# 标题\n\n这是正文，足够写入。\n";
const PNG: &[u8] = b"\x89PNG\r\n";
const WORD: &str = "迁仓关键词";

fn setup() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    fs::create_dir_all(notes.join("raw")).expect("raw");
    fs::create_dir_all(notes.join("digest")).expect("digest");
    sandbox
}

fn default_id() -> String {
    crate::test_support::default_workspace().id
}

fn new_workspace(name: &str) -> String {
    create_workspace(&json!({ "workspace_name": name }))["workspace_id"]
        .as_str()
        .expect("workspace_id")
        .to_string()
}

fn new_project(workspace: &str, title: &str) -> String {
    create_project(&json!({ "workspace_id": workspace, "project_name": title }))["project_id"]
        .as_str()
        .expect("project_id")
        .to_string()
}

fn inbox_of(workspace: &str) -> String {
    spark_data::get_default_project(&spark_data::open().expect("db"), workspace)
        .expect("lookup")
        .expect("inbox")
        .id
}

fn workspaces_dir(sandbox: &TestSandbox) -> PathBuf {
    sandbox.data_dir().join("workspaces")
}

fn write(workspace: &str, project: Option<&str>, body: &str) -> serde_json::Value {
    let mut payload = json!({
        "workspace_id": workspace,
        "title": "T",
        "content": body,
        "digest": "never",
        "source_type": "summary",
    });
    if let Some(p) = project {
        payload["project_id"] = json!(p);
    }
    create_doc_content(&payload)
}

fn stage_bytes(sandbox: &TestSandbox, name: &str, bytes: &[u8]) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage");
    let p = dir.join(name);
    fs::write(&p, bytes).expect("write");
    p.canonicalize().expect("canon")
}

#[test]
fn move_doc_cross_workspace_moves_attachments_and_the_body() {
    let sandbox = setup();
    let dest_ws = new_workspace("Dest");
    let dest_proj = new_project(&dest_ws, "Shelf");
    let png = stage_bytes(&sandbox, "pic.png", PNG);
    let source = stage_bytes(
        &sandbox,
        "source.md",
        format!("{BODY}\n![x](pic.png)\n").as_bytes(),
    );
    let created = create_doc(&json!({
            "workspace_id": default_id(),
            "source_path": source.to_str().unwrap(),
            "title": "T",
            "digest": "never",
            "source_type": "summary",
            "asset_paths": [png.to_str().unwrap()],
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    let moved = move_doc(&json!({
            "id": id,
            "project_id": dest_proj,
            "target_workspace_id": dest_ws,
        }),
    );
    assert_eq!(moved.get("ok"), Some(&json!(true)), "{moved}");
    assert_eq!(moved["workspace_id"], json!(dest_ws));
    assert_eq!(moved["project_id"], json!(dest_proj));
    let rel = spark_data::get_doc(&spark_data::open().expect("db"), id)
        .expect("get")
        .expect("row")
        .raw_path;
    let dest_file = workspaces_dir(&sandbox)
        .join(&dest_ws)
        .join("raw")
        .join(&dest_proj)
        .join(&rel);
    assert!(dest_file.is_file(), "{}", dest_file.display());
    let attached = dest_file.parent().unwrap().join(".attached").join(id).join("pic.png");
    assert!(attached.is_file(), "{}", attached.display());
    let src_root = workspaces_dir(&sandbox).join(default_id()).join("raw");
    let leftover = walkdir_has(&src_root, id);
    assert!(!leftover, "source workspace still holds {id}");
}

#[test]
fn move_doc_renames_when_the_dest_path_is_taken() {
    let sandbox = setup();
    let dest_ws = new_workspace("Dest");
    let dest_proj = new_project(&dest_ws, "Shelf");
    let created = write(&default_id(), None, BODY);
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    let rel = spark_data::get_doc(&spark_data::open().expect("db"), id)
        .expect("get")
        .expect("row")
        .raw_path;
    let taken = workspaces_dir(&sandbox)
        .join(&dest_ws)
        .join("raw")
        .join(&dest_proj)
        .join(&rel);
    fs::create_dir_all(taken.parent().unwrap()).expect("parent");
    fs::write(&taken, b"occupied").expect("taken");
    let moved = move_doc(&json!({
            "id": id,
            "project_id": dest_proj,
            "target_workspace_id": dest_ws,
        }),
    );
    assert_eq!(moved.get("ok"), Some(&json!(true)), "{moved}");
    let new_rel = spark_data::get_doc(&spark_data::open().expect("db"), id)
        .expect("get")
        .expect("row")
        .raw_path;
    assert_ne!(new_rel, rel);
    assert_eq!(fs::read(&taken).unwrap(), b"occupied");
}

#[test]
fn move_doc_refuses_a_disabled_target_workspace() {
    let _sandbox = setup();
    let dest_ws = new_workspace("Dest");
    let dest_proj = new_project(&dest_ws, "Shelf");
    disable_workspace(&json!({ "workspace_id": dest_ws }));
    let created = write(&default_id(), None, BODY);
    let id = created["id"].as_str().unwrap();
    let moved = move_doc(&json!({
            "id": id,
            "project_id": dest_proj,
            "target_workspace_id": dest_ws,
        }),
    );
    assert_eq!(moved.get("_status"), Some(&json!(409)), "{moved}");
}

fn walkdir_has(root: &std::path::Path, needle: &str) -> bool {
    fn walk(dir: &std::path::Path, needle: &str) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == needle)
            {
                return true;
            }
            if path.is_dir() && walk(&path, needle) {
                return true;
            }
        }
        false
    }
    walk(root, needle)
}

#[test]
fn move_doc_reindexes_only_the_target_workspace() {
    let _sandbox = setup();
    let dest_ws = new_workspace("Dest");
    let dest_inbox = inbox_of(&dest_ws);
    let created = write(&default_id(), None, &format!("{BODY}\n{WORD}\n"));
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap().to_string();
    crate::services::index_build::rebuild_spark_index(&crate::config::paths::repo_root().expect("root"))
        .expect("index");
    let moved = move_doc(&json!({
            "id": id,
            "project_id": dest_inbox,
            "target_workspace_id": dest_ws,
        }),
    );
    assert_eq!(moved.get("ok"), Some(&json!(true)), "{moved}");
    let dest_hits = search(&json!({ "q": WORD, "workspace_id": dest_ws }));
    let dest_ids: Vec<String> = dest_hits["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(dest_ids, vec![id.clone()], "{dest_hits}");
    let src_hits = search(&json!({ "q": WORD, "workspace_id": default_id() }));
    let src_ids: Vec<String> = src_hits["items"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|i| i["id"].as_str().map(str::to_string))
        .collect();
    assert!(!src_ids.contains(&id), "{src_hits}");
}
