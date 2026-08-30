use super::*;

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

#[test]
fn is_valid_common_path_accepts_ts_rand_filename() {
    assert!(is_valid_common_path("inbox/notes/202608301000-a3f9k2.md"));
    assert!(is_valid_common_path(
        "inbox/notes/202608301000-a3f9k2-source.md"
    ));
    assert!(!is_valid_common_path("inbox/notes/notimestamp.md"));
}
