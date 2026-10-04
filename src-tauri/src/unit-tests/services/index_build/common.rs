use super::*;

#[test]
fn spark_doc_id_matches_python_rules() {
    assert_eq!(
        spark_doc_id("raw", "proj/note.md"),
        sanitize_doc_id("raw__proj/note.md")
    );
    assert_eq!(spark_doc_id("raw", "a/b/c.md"), "raw__a_b_c_md");
}

#[test]
fn spark_title_from_heading_or_slug() {
    assert_eq!(
        extract_spark_title("# My Title\n\nx", "20250101-note.md"),
        "My Title"
    );
    assert_eq!(
        extract_spark_title("no heading", "20250101-my-slug.md"),
        "my slug"
    );
}

#[test]
fn knowledge_doc_id_matches_legacy_id_rules() {
    assert_eq!(
        knowledge_doc_id("o/repo", "docs/a.md"),
        sanitize_doc_id("repo__docs__a.md")
    );
}

#[test]
fn should_skip_md_translation_suffixes() {
    assert!(should_skip_md("202606191700-waymo-interview-zh.md"));
    assert!(should_skip_md("202606191700-waymo-interview-fr.md"));
    assert!(!should_skip_md("202606191700-waymo-interview.md"));
    assert!(!should_skip_md("guide.md"));
}
