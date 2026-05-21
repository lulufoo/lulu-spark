use super::*;

#[test]
fn ping_returns_pong() {
    assert_eq!(ping(), "pong");
}

#[test]
fn ping_is_stateless() {
    assert_eq!(ping(), "pong");
    assert_eq!(ping(), "pong");
}
