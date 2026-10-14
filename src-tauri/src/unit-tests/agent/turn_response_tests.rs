//! LW-171: the empty-reply copy must not claim "nothing written" after a write.

use crate::agent::language::{describe_empty_reply, Language};

fn text(wrote: bool, tool_rounds: usize) -> &'static str {
    describe_empty_reply(Language::English, wrote, tool_rounds)
}

#[test]
fn empty_reply_tells_apart_written_read_only_and_no_tools() {
    let wrote = text(true, 2);
    let read_only = text(false, 1);
    let none = text(false, 0);
    assert!(wrote.contains("Writes were made"));
    assert!(read_only.contains("nothing was written") && read_only.contains("Tools ran"));
    assert_eq!(none, "The model's response was empty. Nothing was written.");
    assert!(!wrote.contains("Nothing was written"));
    assert!(wrote != read_only && read_only != none);
}

#[test]
fn a_write_wins_over_the_round_count() {
    assert_eq!(text(true, 0), text(true, 5));
}
