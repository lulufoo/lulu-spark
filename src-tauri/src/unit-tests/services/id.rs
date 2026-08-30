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

#[test]
fn random_entry_id_returns_32_lower_hex() {
    let s = crate::services::id::random_entry_id();
    assert_eq!(s.len(), 32);
    assert!(s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
}

#[test]
fn random_alnum6_is_six_lowercase_alnum() {
    let s = crate::services::id::random_alnum6();
    assert_eq!(s.len(), 6);
    assert!(
        s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
        "expected [a-z0-9]{{6}}, got {s:?}"
    );
}
