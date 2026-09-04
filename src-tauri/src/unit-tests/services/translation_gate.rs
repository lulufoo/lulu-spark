use super::is_full_english;

#[test]
fn empty_body_is_not_full_english() {
    assert!(!is_full_english("# T\n\n---\n\n"));
}

#[test]
fn latin_without_cjk_is_full_english() {
    let doc = "# T\n\n> 创建时间：2026年1月1日 12:00\n\n---\n\nHello world.\n";
    assert!(is_full_english(doc));
}

#[test]
fn mixed_cjk_is_not_full_english() {
    let doc = "# T\n\n---\n\n摘要 Summary body.\n";
    assert!(!is_full_english(doc));
}

#[test]
fn author_chrome_and_html_comment_do_not_count_as_cjk() {
    let doc = "# T\n\n---\n\n作者 | Jane\n<!-- 忽略 -->\nOnly English remains.\n";
    assert!(is_full_english(doc));
}
