use super::*;
use std::fs;

#[test]
fn collect_docs_from_sample_corpus() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path();
    let md = corpus.join("raw/proj/note.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "# Hello\n\nbody").expect("write");
    let docs = collect_documents(corpus);
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["id"], super::super::common::workbench_doc_id("raw", "proj/note.md"));
    assert_eq!(docs[0]["title"], "Hello");
    assert_eq!(docs[0]["topic"], "proj");
}
