use crate::services::id::random_hex12;

#[test]
fn random_hex12_returns_twelve_lowercase_hex_chars() {
    let s = random_hex12();
    assert_eq!(s.len(), 12);
    assert!(
        s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "expected [0-9a-f]{{12}}, got {s:?}"
    );
}

#[test]
fn random_hex12_consecutive_calls_differ() {
    let a = random_hex12();
    let b = random_hex12();
    assert_ne!(a, b, "two calls should not return the same id");
}
