use super::*;
use crate::integrations::search::build_knowledge_document;

#[test]
fn workbench_doc_id_matches_python_rules() {
    assert_eq!(
        workbench_doc_id("raw", "proj/note.md"),
        sanitize_doc_id("raw__proj/note.md")
    );
    assert_eq!(workbench_doc_id("raw", "a/b/c.md"), "raw__a_b_c_md");
}

#[test]
fn workbench_title_from_heading_or_slug() {
    assert_eq!(
        extract_workbench_title("# My Title\n\nx", "20250101-note.md"),
        "My Title"
    );
    assert_eq!(
        extract_workbench_title("no heading", "20250101-my-slug.md"),
        "my slug"
    );
}

#[test]
fn knowledge_doc_id_matches_build_knowledge_document() {
    let doc = build_knowledge_document("o/repo", "docs/a.md", "# T\n", "desc");
    let expected = sanitize_doc_id("repo__docs__a.md");
    assert_eq!(doc["id"], expected);
}

#[test]
fn should_skip_md_translation_suffixes() {
    assert!(should_skip_md("202606191700-waymo-interview-zh.md"));
    assert!(should_skip_md("202606191700-waymo-interview-fr.md"));
    assert!(!should_skip_md("202606191700-waymo-interview.md"));
    assert!(!should_skip_md("guide.md"));
}
