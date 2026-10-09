//! LW-171: the empty-reply copy must not claim "nothing written" after a write.

use super::response::empty_reply_text;

#[test]
fn empty_reply_tells_apart_written_read_only_and_no_tools() {
    let wrote = empty_reply_text(true, 2);
    let read_only = empty_reply_text(false, 1);
    let none = empty_reply_text(false, 0);
    assert!(wrote.contains("已执行写入"));
    assert!(read_only.contains("未写入") && read_only.contains("已执行工具"));
    assert_eq!(none, "模型响应为空，未执行任何写入。");
    assert!(!wrote.contains("未执行任何写入"));
    assert!(wrote != read_only && read_only != none);
}

#[test]
fn a_write_wins_over_the_round_count() {
    assert_eq!(empty_reply_text(true, 0), empty_reply_text(true, 5));
}
