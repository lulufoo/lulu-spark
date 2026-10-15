use super::{check_zh_parity, is_full_english};

#[test]
fn empty_or_whitespace_is_not_full_english() {
    assert!(!is_full_english(""));
    assert!(!is_full_english("   \n"));
}

#[test]
fn latin_without_cjk_is_full_english() {
    let doc = "# T\n\nHello world.\n";
    assert!(is_full_english(doc));
}

#[test]
fn mixed_cjk_is_not_full_english() {
    let doc = "# T\n\n摘要 Summary body.\n";
    assert!(!is_full_english(doc));
}

#[test]
fn author_chrome_and_html_comment_do_not_count_as_cjk() {
    let doc = "# T\n\n作者 | Jane\n<!-- 忽略 -->\nOnly English remains.\n";
    assert!(is_full_english(doc));
}

#[test]
fn created_at_line_counts_as_cjk() {
    let doc = "# T\n\n> 创建时间：2026年1月1日 12:00\n\nHello world.\n";
    assert!(!is_full_english(doc));
}

#[test]
fn zh_parity_accepts_documents_without_separator() {
    let en = "# Title\n\n## A\nHost: hello\n";
    let zh = "# 题\n\n## 甲\n主持：你好\n";
    assert!(check_zh_parity(en, zh).is_ok());
}
