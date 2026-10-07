use super::*;

#[test]
fn pass_id_reads_encoded_bag_and_ignores_hash() {
    let id = "trace_12345678abcd";
    let bag_json = format!(r#"{{"id":"{id}"}}"#);
    let bag = urlencoding::encode(&bag_json);
    let scheme = format!(
        "spark://auth-login/callback?pass={bag}#access_token=secret&refresh_token=r"
    );
    assert_eq!(pass_id_from_scheme(&scheme).as_deref(), Some(id));
    assert!(is_auth_login_scheme(&scheme));
}

#[test]
fn pass_id_missing_on_notes_or_bare_callback() {
    assert!(!is_auth_login_scheme(
        "spark://notes/open?id=a&path=p.md&trace=trace_12345678"
    ));
    assert!(pass_id_from_scheme("spark://auth-login/callback").is_none());
    assert!(
        pass_id_from_scheme("spark://auth-login/callback#access_token=trace_12345678abcd")
            .is_none()
    );
}

#[test]
fn scheme_for_log_drops_hash_tokens() {
    let bag = urlencoding::encode(r#"{"id":"trace_12345678abcd"}"#);
    let scheme = format!("spark://auth-login/callback?pass={bag}#access_token=secret");
    let clipped = scheme_for_log(&scheme);
    assert!(clipped.starts_with("spark://auth-login/callback?pass="));
    assert!(!clipped.contains("access_token"));
    assert!(!clipped.contains("secret"));
}

#[test]
fn production_module_writes_login_business() {
    let prod = include_str!("../../services/login_hop.rs");
    assert!(prod.contains("#[path = \"../unit-tests/services/login_hop.rs\"]"));
    assert!(!prod.contains("#[test]"));
    assert!(prod.contains("LOGIN_BUSINESS"));
    assert!(prod.contains("\"login\""));
    assert!(prod.contains("LOGIN_EVENT_SCHEME_OPEN"));
    assert!(prod.contains("app_log::log"));
    assert!(!prod.contains("os-notify"));
}
