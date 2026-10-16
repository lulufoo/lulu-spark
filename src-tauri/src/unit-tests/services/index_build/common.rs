use super::*;

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
fn should_skip_md_translation_suffixes() {
    assert!(should_skip_md("202606191700-waymo-interview-zh.md"));
    assert!(should_skip_md("202606191700-waymo-interview-fr.md"));
    assert!(!should_skip_md("202606191700-waymo-interview.md"));
    assert!(!should_skip_md("guide.md"));
}
