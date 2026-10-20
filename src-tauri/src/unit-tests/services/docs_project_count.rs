use std::collections::HashMap;

use serde_json::{json, Value};

use super::*;
use crate::services::spark_data::{self, new_doc_id, DocRow};
use crate::test_support::TestSandbox;

fn counts(listed: &Value) -> HashMap<String, i64> {
    listed["projects"]
        .as_array()
        .expect("projects")
        .iter()
        .map(|p| (p["project_name"].as_str().expect("name").to_string(), p["doc_count"].as_i64().expect("doc_count")))
        .collect()
}

fn add_doc(project: &str, rel: &str) {
    let conn = spark_data::open().expect("db");
    let row = DocRow::new(&new_doc_id(), rel, project);
    spark_data::insert_doc(&conn, &row).expect("insert");
}

fn new_project(ws: &str, name: &str) -> String {
    create_project(&json!({ "workspace_id": ws, "project_name": name }))["project_id"].as_str().expect("id").to_string()
}

#[test]
fn list_projects_counts_documents_per_project() {
    let _sandbox = TestSandbox::new();
    let ws = create_workspace(&json!({ "workspace_name": "Work" }))["workspace_id"].as_str().expect("ws").to_string();
    let inbox = spark_data::get_default_project(&spark_data::open().expect("db"), &ws).expect("get").expect("inbox").id;
    let plans = new_project(&ws, "Plans");
    new_project(&ws, "Empty");
    add_doc(&inbox, "notes/202601011200-aaaaaa.md");
    add_doc(&plans, "notes/202601011200-bbbbbb.md");
    add_doc(&plans, "notes/202601011200-cccccc.md");

    let listed = list_projects(&json!({ "workspace_id": ws }));
    let got = counts(&listed);
    assert_eq!((got["Inbox"], got["Plans"], got["Empty"]), (1, 2, 0), "{listed}");

    let other = create_workspace(&json!({ "workspace_name": "Other" }))["workspace_id"].as_str().expect("ws").to_string();
    assert_eq!(counts(&list_projects(&json!({ "workspace_id": other })))["Inbox"], 0, "another workspace counts its own documents");
}

#[test]
fn create_update_and_reorder_report_the_count_too() {
    let _sandbox = TestSandbox::new();
    let default = crate::test_support::default_workspace().id;
    let created = create_project(&json!({ "workspace_id": default, "project_name": "Plans" }));
    assert_eq!(created["doc_count"], json!(0), "{created}");
    let plans = created["project_id"].as_str().expect("id").to_string();
    add_doc(&plans, "notes/202601011200-aaaaaa.md");

    let updated = update_project(&json!({ "project_id": plans, "project_name": "Roadmaps" }));
    assert_eq!(updated["doc_count"], json!(1), "{updated}");

    let ids: Vec<String> = list_projects(&json!({ "workspace_id": default }))["projects"]
        .as_array()
        .expect("projects")
        .iter()
        .rev()
        .map(|p| p["project_id"].as_str().expect("id").to_string())
        .collect();
    let reordered = reorder_projects(&json!({ "ids": ids }));
    assert_eq!(counts(&reordered)["Roadmaps"], 1, "{reordered}");
}
