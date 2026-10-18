use std::collections::HashSet;

use serde_json::{json, Value};

use super::*;
use crate::services::docs::{create_doc_content, create_workspace, delete_doc, update_doc_content};
use crate::test_support::TestSandbox;

const WORD: &str = "角色扮演";

fn new_workspace(name: &str) -> String {
    let made = create_workspace(&json!({ "workspace_name": name }));
    made["workspace_id"].as_str().expect("workspace id").to_string()
}

fn write(workspace: Option<&str>, body: &str) -> String {
    let mut payload = json!({ "title": "T", "content": body, "digest": "never", "source_type": "summary" });
    if let Some(w) = workspace {
        payload["workspace_id"] = json!(w);
    }
    let created = create_doc_content(&payload);
    created["id"].as_str().unwrap_or_else(|| panic!("create failed: {created}")).to_string()
}

fn only_workspace(id: &str) -> SearchFilter {
    SearchFilter { workspace: Some(id.to_string()), ..SearchFilter::default() }
}

fn ids(hits: &[QueryHit]) -> Vec<&str> {
    hits.iter().map(|h| h.doc_id.as_str()).collect()
}

#[test]
fn searching_with_a_workspace_only_hits_that_workspace() {
    let sandbox = TestSandbox::new();
    let other = new_workspace("Memory");
    let in_default = write(None, &format!("# 默认\n\n这篇讲{WORD}的做法。\n"));
    let in_other = write(Some(&other), &format!("# 记忆\n\n这篇也讲{WORD}的做法。\n"));

    let chunks = collect_workspace_docs().expect("collect");
    assert!(chunks.iter().all(|c| c.category == DOCS_CATEGORY));
    rebuild(&sandbox.cache_dir(), &chunks, None, &HashSet::new()).expect("rebuild");

    let cache = sandbox.cache_dir();
    let everywhere = search_in_cache(&cache, WORD, &SearchFilter::default(), 10).expect("global");
    assert_eq!(everywhere.len(), 2);

    let scoped = search_in_cache(&cache, WORD, &only_workspace(&other), 10).expect("scoped");
    assert_eq!(ids(&scoped), vec![in_other.as_str()]);
    assert_eq!(scoped[0].workspace_id, other);
    let inbox = crate::services::spark_data::get_inbox_project(
        &crate::services::spark_data::open().expect("db"),
        &other,
    )
    .expect("lookup")
    .expect("inbox")
    .id;
    assert_eq!(scoped[0].topic, inbox, "the project id comes from the registry");
    assert!(scoped[0].common_path.ends_with(".md"));

    let default_only = search_in_cache(&cache, WORD, &only_workspace("default"), 10).expect("default");
    assert_eq!(ids(&default_only), vec![in_default.as_str()]);
    assert!(search_in_cache(&cache, WORD, &only_workspace("nope"), 10).expect("none").is_empty());
}

#[test]
fn writes_through_the_doc_service_keep_the_index_in_step() {
    let sandbox = TestSandbox::new();
    let ws = new_workspace("Live");
    drop(open(&sandbox.cache_dir()).expect("create index"));

    let id = write(Some(&ws), "# 一\n\n最初没有那个词。\n");
    let cache = sandbox.cache_dir();
    assert!(search_in_cache(&cache, WORD, &only_workspace(&ws), 10).expect("before").is_empty());

    let updated = update_doc_content(&json!({
        "workspace_id": ws, "id": id, "content": format!("# 一\n\n现在有{WORD}了。\n"), "digest": "never"
    }));
    assert_eq!(updated["ok"], json!(true), "{updated}");
    let found = search_in_cache(&cache, WORD, &only_workspace(&ws), 10).expect("after update");
    assert_eq!(ids(&found), vec![id.as_str()]);

    let removed: Value = delete_doc(&json!({ "workspace_id": ws, "id": id }));
    assert_eq!(removed["ok"], json!(true));
    assert!(search_in_cache(&cache, WORD, &only_workspace(&ws), 10).expect("after delete").is_empty());
}

#[test]
fn a_chunk_has_no_workspace_until_it_is_placed_in_one() {
    let chunk = SourceChunk::from_text(DOCS_CATEGORY, "raw", "d1", "t", "正文", "", 0);
    assert_eq!(chunk.workspace_id, "");
    assert_eq!(chunk.in_workspace("abc123").workspace_id, "abc123");
}
