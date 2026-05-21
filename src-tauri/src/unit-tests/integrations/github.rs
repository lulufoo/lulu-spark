use super::*;

#[test]
fn require_token_fails_when_empty() {
    crate::config::secrets::test_secrets_clear();
    let err = require_token().expect_err("token");
    assert!(err.message.contains("GitHub Token"));
}

#[test]
fn decode_contents_payload_roundtrip() {
    let raw = "hello";
    let b64 = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
    let v = serde_json::json!({ "content": format!("{b64}\n") });
    let text = decode_contents_payload(&v).expect("decode");
    assert_eq!(text, "hello");
}
