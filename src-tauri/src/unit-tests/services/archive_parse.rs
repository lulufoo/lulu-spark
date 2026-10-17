use super::*;

#[test]
fn ts_from_common_path_extracts_prefix() {
    assert_eq!(
        ts_from_common_path("t/202606191430-foo.md").as_deref(),
        Some("202606191430")
    );
}

#[test]
fn expected_zh_common_path_appends_suffix() {
    assert_eq!(
        expected_zh_common_path("t/202606191430-foo.md").as_deref(),
        Some("t/202606191430-foo-zh.md")
    );
}

#[test]
fn expected_lang_common_path_appends_suffix() {
    assert_eq!(
        expected_lang_common_path("t/202606191430-foo.md", "zh").as_deref(),
        Some("t/202606191430-foo-zh.md")
    );
    assert_eq!(
        expected_lang_common_path("t/202606191430-foo.md", "fr").as_deref(),
        Some("t/202606191430-foo-fr.md")
    );
}

#[test]
fn expected_lang_common_path_rejects_bad_primary() {
    assert_eq!(expected_lang_common_path("t/foo", "zh"), None);
}

#[test]
fn is_valid_common_path_accepts_ts_rand_filename() {
    assert!(is_valid_common_path("notes/202608301000-a3f9k2.md"));
    assert!(is_valid_common_path(
        "notes/202608301000-a3f9k2-source.md"
    ));
    assert!(!is_valid_common_path("notes/notimestamp.md"));
}

#[test]
fn is_valid_common_path_allows_one_to_three_folder_levels() {
    assert!(is_valid_common_path("a/b/202608301000-a3f9k2.md"));
    assert!(is_valid_common_path("a/b/c/202608301000-a3f9k2.md"));
    assert!(!is_valid_common_path("a/b/c/d/202608301000-a3f9k2.md"));
    assert!(!is_valid_common_path("a//202608301000-a3f9k2.md"));
    assert!(!is_valid_common_path("a/../202608301000-a3f9k2.md"));
    assert!(!is_valid_common_path("202608301000-a3f9k2.md"));
}
