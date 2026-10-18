use rusqlite::{params, Connection};

use crate::agent::session;
use crate::commands::ai_assistant::search_chat_sessions_json;
use crate::test_support::TestSandbox;

/// A chat with this catalog title and `updated_at`. Search reads only the catalog.
fn seed_titled(sandbox: &TestSandbox, title: &str, updated_at: i64) -> String {
    let id = session::create_session().expect("create").session_id;
    let path = sandbox
        .cache_dir()
        .join("agent-exec")
        .join("sessions-catalog.sqlite");
    Connection::open(path)
        .expect("open catalog")
        .execute(
            "UPDATE sessions SET title = ?2, updated_at = ?3 WHERE session_id = ?1",
            params![id, title, updated_at],
        )
        .expect("set title");
    id
}

fn search_ids(query: &str, limit: usize) -> Vec<String> {
    session::search_session_titles(query, limit)
        .expect("search")
        .iter()
        .map(|hit| hit["session_id"].as_str().expect("id").to_string())
        .collect()
}

#[test]
fn finds_a_title_by_a_substring_and_ignores_the_case_of_letters() {
    let sandbox = TestSandbox::new();
    let hit = seed_titled(&sandbox, "Fix the Login redirect", 100);
    seed_titled(&sandbox, "Plan the trip", 200);

    assert_eq!(search_ids("login", 10), vec![hit.clone()]);
    assert_eq!(search_ids("LOGIN RED", 10), vec![hit.clone()]);
    assert_eq!(search_ids("  login  ", 10), vec![hit], "the query is trimmed");
    assert!(search_ids("nothing like this", 10).is_empty());
}

#[test]
fn finds_a_chinese_title() {
    let sandbox = TestSandbox::new();
    let hit = seed_titled(&sandbox, "帮我整理这周的会议纪要", 100);
    seed_titled(&sandbox, "Plan the trip", 200);
    assert_eq!(search_ids("会议", 10), vec![hit]);
}

#[test]
fn percent_underscore_and_backslash_are_plain_characters() {
    let sandbox = TestSandbox::new();
    let percent = seed_titled(&sandbox, "Raise it by 50% today", 100);
    let underscore = seed_titled(&sandbox, "Rename snake_case fields", 200);
    let backslash = seed_titled(&sandbox, r"Open C:\temp\notes", 300);
    seed_titled(&sandbox, "Sixty percent of the snakeXcase", 400);

    assert_eq!(search_ids("50%", 10), vec![percent]);
    assert_eq!(search_ids("snake_case", 10), vec![underscore]);
    assert_eq!(search_ids(r"C:\temp", 10), vec![backslash]);
    assert!(search_ids("%", 10).len() == 1, "a lone % matches only the title that has a %");
    assert!(search_ids("_", 10).len() == 1, "a lone _ matches only the title that has a _");
}

#[test]
fn a_blank_query_or_a_zero_limit_finds_nothing() {
    let sandbox = TestSandbox::new();
    seed_titled(&sandbox, "Anything", 100);
    assert!(search_ids("", 10).is_empty());
    assert!(search_ids("   ", 10).is_empty());
    assert!(search_ids("any", 0).is_empty());
}

#[test]
fn hits_are_newest_first_and_capped_by_the_limit() {
    let sandbox = TestSandbox::new();
    let oldest = seed_titled(&sandbox, "topic one", 100);
    let newest = seed_titled(&sandbox, "topic three", 300);
    let middle = seed_titled(&sandbox, "topic two", 200);

    assert_eq!(search_ids("topic", 10), vec![newest.clone(), middle.clone(), oldest]);
    assert_eq!(search_ids("topic", 2), vec![newest, middle]);
}

#[test]
fn searches_all_history_not_only_the_recent_days() {
    let sandbox = TestSandbox::new();
    let ancient = seed_titled(&sandbox, "Old design review", 1_000_000_000);
    seed_titled(&sandbox, "Today", 1_700_000_000);
    seed_titled(&sandbox, "Yesterday", 1_699_900_000);
    session::reset_live_for_tests();
    assert!(session::init_recent_cutoff().expect("cutoff") > 1_000_000_000);
    assert!(
        !session::list_session_summaries()
            .expect("list")
            .iter()
            .any(|s| s["session_id"] == ancient.as_str()),
        "the old chat is outside the Home list"
    );

    assert_eq!(search_ids("design", 10), vec![ancient]);
    session::reset_live_for_tests();
}

#[test]
fn a_hit_carries_id_title_and_time_through_the_command() {
    let sandbox = TestSandbox::new();
    let id = seed_titled(&sandbox, "Release checklist", 1_700_000_000);

    let value = search_chat_sessions_json("check", 20).expect("command");
    let hits = value["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["session_id"], id.as_str());
    assert_eq!(hits[0]["title"], "Release checklist");
    assert_eq!(hits[0]["updated_at"], 1_700_000_000);
    assert!(hits[0].get("llm").is_none());
}

#[test]
fn searching_before_any_chat_exists_finds_nothing() {
    let _sandbox = TestSandbox::new();
    assert!(session::search_session_titles("x", 10).expect("search").is_empty());
}
