use super::*;

#[test]
fn missing_repo_returns_400() {
    let v = open_kb_in_iterm(&json!({}));
    assert_eq!(v["_status"], 400);
}
