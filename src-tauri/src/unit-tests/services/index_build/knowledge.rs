use super::*;
use std::fs;

#[test]
fn collect_repo_docs_builds_ids() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("myrepo");
    let md = kb.join("docs/a.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "# Title\n\nx").expect("write");
    let docs = collect_repo_documents(dir.path(), "o/myrepo", "desc");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["title"], "Title");
    assert_eq!(docs[0]["repo"], "o/myrepo");
}
