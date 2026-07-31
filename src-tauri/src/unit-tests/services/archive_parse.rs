use super::*;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30
> 来源：theme-summary
> 导航：[digest](../../../digest/inbox/test-topic/202606191430-test-slug.md)

---

Summary body here.
"#;

#[test]
fn parse_valid_document() {
    let parsed = parse_archive_document(SAMPLE_DOC).expect("parse");
    assert_eq!(
        parsed.common_path,
        "inbox/test-topic/202606191430-test-slug.md"
    );
    assert_eq!(parsed.created_at, "202606191430");
}

#[test]
fn parse_requires_digest_nav_only() {
    let doc = "# T\n\n> 创建时间：x\n> 导航：[digest](../../../digest/inbox/a/202606191430-x.md)\n\n---\n\nbody";
    assert!(parse_archive_document(doc).is_ok());
}

#[test]
fn parse_missing_title_fails() {
    let doc = "no title\n\n> 创建时间：x\n> 导航：[digest](x/inbox/a/b/202606191430-x.md)\n\n---\n\nbody";
    assert_eq!(
        parse_archive_document(doc).err(),
        Some(ParseError::MissingTitle)
    );
}

#[test]
fn ts_from_common_path_extracts_prefix() {
    assert_eq!(
        ts_from_common_path("inbox/t/202606191430-foo.md").as_deref(),
        Some("202606191430")
    );
}

#[test]
fn is_valid_entry_id_requires_32_lower_hex() {
    assert!(is_valid_entry_id("a1b2c3d4e5f6789012345678901234ab"));
    assert!(!is_valid_entry_id("SHORT"));
    assert!(!is_valid_entry_id("A1B2C3D4E5F6789012345678901234AB"));
}

#[test]
fn expected_zh_common_path_appends_suffix() {
    assert_eq!(
        expected_zh_common_path("inbox/t/202606191430-foo.md").as_deref(),
        Some("inbox/t/202606191430-foo-zh.md")
    );
}

#[test]
fn expected_lang_common_path_appends_suffix() {
    assert_eq!(
        expected_lang_common_path("inbox/t/202606191430-foo.md", "zh").as_deref(),
        Some("inbox/t/202606191430-foo-zh.md")
    );
    assert_eq!(
        expected_lang_common_path("inbox/t/202606191430-foo.md", "fr").as_deref(),
        Some("inbox/t/202606191430-foo-fr.md")
    );
}

#[test]
fn expected_lang_common_path_rejects_bad_primary() {
    assert_eq!(expected_lang_common_path("inbox/t/foo", "zh"), None);
}
