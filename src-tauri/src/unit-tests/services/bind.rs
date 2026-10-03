use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use crate::services::mcp_oauth::{list_devices, verify_device_token, verify_for_slot, Slot};
use crate::test_support::TestSandbox;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs()
}

fn with_bind<F: FnOnce()>(test: F) {
    let _sandbox = TestSandbox::new();
    test_reset_bind_keychain();
    test_clear_session();
    test();
    test_clear_session();
}

fn draw(ip: Ipv4Addr, port: u16, tls: &str) -> BindPayload {
    create_bind_payload(ip, port, tls).expect("create_bind_payload")
}

#[test]
fn create_bind_payload_returns_object_with_temp_pub_exp_tls_and_sig() {
    with_bind(|| {
        let ip = Ipv4Addr::new(192, 168, 1, 8);
        let payload = draw(ip, 7654, "ab".repeat(32).as_str());
        assert_eq!(payload.ip, ip);
        assert_eq!(payload.port, 7654);
        assert_eq!(payload.tls_fingerprint, "ab".repeat(32));
        assert_eq!(payload.temp_pub.len(), 64);
        assert!(
            payload
                .temp_pub
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        assert_eq!(payload.sig.len(), 128);
        let now = now_secs();
        assert!(
            payload.exp > now && payload.exp <= now + 180 + 2,
            "temp session must expire in 3 minutes, got exp={} now={now}",
            payload.exp
        );
        let canon = canonical_bind_string(&payload);
        assert_eq!(
            canon,
            format!(
                "v1|{}|{}|{}|{}|{}",
                payload.ip, payload.port, payload.temp_pub, payload.tls_fingerprint, payload.exp
            )
        );
        assert!(
            !canon.contains(&payload.sig),
            "sig must not enter the canonical string"
        );
        verify_bind_signature(&payload).expect("Ed25519 signing key must sign the payload");
    });
}

#[test]
fn create_bind_payload_replaces_in_memory_session() {
    with_bind(|| {
        let first = draw(Ipv4Addr::new(10, 0, 0, 1), 7654, "aa");
        let sealed_first =
            seal_bind_request(&first.temp_pub, "phone-old", None).expect("seal first");
        let second = draw(Ipv4Addr::new(10, 0, 0, 1), 7654, "aa");
        assert_ne!(first.temp_pub, second.temp_pub);
        assert_eq!(
            complete_bind(&sealed_first).expect_err("old session"),
            BindError::session_unavailable
        );
        let sealed_second =
            seal_bind_request(&second.temp_pub, "phone-new", Some("label")).expect("seal second");
        let done = complete_bind(&sealed_second).expect("new session");
        assert_eq!(
            verify_device_token(&done.device_mcp_token).expect("issued"),
            "phone-new"
        );
    });
}

#[test]
fn complete_bind_issues_device_ticket_and_returns_binding_public_key() {
    with_bind(|| {
        let payload = draw(Ipv4Addr::new(10, 0, 0, 2), 17654, "ff");
        let sealed =
            seal_bind_request(&payload.temp_pub, "phone-bind", Some("Pixel")).expect("seal");
        let done = complete_bind(&sealed).expect("complete");
        assert_eq!(done.device_mcp_token.len(), 64);
        assert_eq!(
            verify_device_token(&done.device_mcp_token).expect("oauth device ticket"),
            "phone-bind"
        );
        assert_eq!(done.binding_public_key, binding_public_key_hex().expect("pk"));
        assert_eq!(done.binding_public_key.len(), 64);
        assert_ne!(done.binding_public_key, done.device_mcp_token);
        assert!(
            verify_device_token(&done.binding_public_key).is_err(),
            "binding_public_key must not participate in MCP ticket verify"
        );
        let listed = list_devices().expect("list");
        assert_eq!(listed[0].device_id, "phone-bind");
        assert_eq!(listed[0].device_label.as_deref(), Some("Pixel"));
        assert_eq!(
            verify_for_slot(
                Slot::Spark,
                crate::services::mcp_oauth::TicketHandle::from_secret(
                    done.device_mcp_token.clone()
                )
            )
            .expect_err("device ticket is not a slot ticket"),
            crate::services::mcp_oauth::OAuthError::rejected
        );
    });
}

#[test]
fn complete_bind_consumes_session_and_rejects_replay() {
    with_bind(|| {
        let payload = draw(Ipv4Addr::new(10, 0, 0, 3), 7654, "cc");
        let sealed = seal_bind_request(&payload.temp_pub, "phone-once", None).expect("seal");
        complete_bind(&sealed).expect("first use");
        assert_eq!(
            complete_bind(&sealed).expect_err("replay"),
            BindError::consumed
        );
        let replay = seal_bind_request(&payload.temp_pub, "phone-once-2", None);
        assert!(
            replay.is_err()
                || complete_bind(&replay.expect("sealed replay")).is_err(),
            "repeat challenge / used session cannot bind again"
        );
    });
}

#[test]
fn complete_bind_rejects_expired_session() {
    with_bind(|| {
        let payload = draw(Ipv4Addr::new(10, 0, 0, 4), 7654, "dd");
        let sealed = seal_bind_request(&payload.temp_pub, "phone-exp", None).expect("seal");
        test_expire_current_session();
        assert_eq!(
            complete_bind(&sealed).expect_err("expired"),
            BindError::expired
        );
    });
}

#[test]
fn complete_bind_request_omits_temp_pub_and_requires_v1_device_id() {
    with_bind(|| {
        let payload = draw(Ipv4Addr::new(10, 0, 0, 5), 7654, "ee");
        let missing = seal_bind_request(&payload.temp_pub, "", None).expect("seal empty");
        assert_eq!(
            complete_bind(&missing).expect_err("empty device_id"),
            BindError::invalid_request
        );
        let ok = seal_bind_request(&payload.temp_pub, "phone-v1", None).expect("seal");
        complete_bind(&ok).expect("v=1 and device_id");
    });
}

#[test]
fn keychain_bind_service_creates_binding_key_when_missing() {
    with_bind(|| {
        test_reset_bind_keychain();
        let before = binding_public_key_hex();
        assert!(
            before.is_err(),
            "reset must remove the long-term Binding key"
        );
        let payload = draw(Ipv4Addr::new(10, 0, 0, 6), 7654, "11");
        let sealed = seal_bind_request(&payload.temp_pub, "phone-kc", None).expect("seal");
        let done = complete_bind(&sealed).expect("complete creates key");
        assert_eq!(done.binding_public_key, binding_public_key_hex().expect("pk"));
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/services/bind/mod.rs"
        ));
        assert!(
            src.contains("lulu-spark-bind"),
            "Keychain service must be lulu-spark-bind"
        );
        assert!(
            !src.contains("lulu-spark-mcp-oauth"),
            "bind must not write the oauth keychain service"
        );
    });
}

#[test]
fn crate_exports_bind_and_registers_module() {
    let _: fn(Ipv4Addr, u16, &str) -> Result<BindPayload, BindError> = create_bind_payload;
    let _: fn(&[u8]) -> Result<BindResult, BindError> = complete_bind;
    let _: fn() -> BindSessionState = bind_session_state;
    let _: fn(&str, Option<&str>) -> Result<crate::services::mcp_oauth::TicketHandle, _> =
        crate::services::mcp_oauth::issue_for_device;
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mod.rs"
    ));
    assert!(src.contains("pub mod bind"), "services/mod.rs must register bind");
    let bind_src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/bind/mod.rs"
    ));
    assert!(
        !bind_src.contains("/mcp/mobile"),
        "T1 bind must not mount /mcp/mobile"
    );
}

#[test]
fn bind_does_not_listen_and_does_not_store_tokens() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/bind/mod.rs"
    ));
    assert!(
        !src.contains("TcpListener") && !src.contains("0.0.0.0"),
        "Bind is in-process and must not listen"
    );
    with_bind(|| {
        let payload = draw(Ipv4Addr::new(10, 0, 0, 7), 7654, "22");
        let sealed = seal_bind_request(&payload.temp_pub, "phone-store", None).expect("seal");
        let done = complete_bind(&sealed).expect("complete");
        assert!(
            !src.contains(&done.device_mcp_token),
            "source must not hardcode tokens"
        );
    });
}

#[test]
fn bind_session_state_is_idle_when_memory_has_no_session() {
    with_bind(|| {
        assert_eq!(bind_session_state(), BindSessionState::idle);
    });
}

#[test]
fn bind_session_state_is_live_after_create_while_now_at_or_before_exp() {
    with_bind(|| {
        let payload = draw(Ipv4Addr::new(10, 0, 0, 8), 7654, "33");
        assert_eq!(bind_session_state(), BindSessionState::live);
        let now = now_secs();
        assert!(
            payload.exp >= now,
            "countdown material is payload.exp, got exp={} now={now}",
            payload.exp
        );
    });
}

#[test]
fn bind_session_state_is_consumed_after_complete_bind() {
    with_bind(|| {
        let payload = draw(Ipv4Addr::new(10, 0, 0, 9), 7654, "44");
        let sealed = seal_bind_request(&payload.temp_pub, "phone-ro", None).expect("seal");
        complete_bind(&sealed).expect("complete");
        assert_eq!(bind_session_state(), BindSessionState::consumed);
    });
}

#[test]
fn bind_session_state_is_expired_when_live_and_now_after_exp() {
    with_bind(|| {
        draw(Ipv4Addr::new(10, 0, 0, 10), 7654, "55");
        test_expire_current_session();
        assert_eq!(bind_session_state(), BindSessionState::expired);
    });
}

#[test]
fn bind_session_state_stays_live_when_now_equals_exp() {
    with_bind(|| {
        draw(Ipv4Addr::new(10, 0, 0, 11), 7654, "66");
        loop {
            let now = now_secs();
            test_set_current_session_exp(now);
            let state = bind_session_state();
            if now_secs() == now {
                assert_eq!(state, BindSessionState::live);
                break;
            }
        }
    });
}

#[test]
fn bind_session_state_expired_read_leaves_live_session_in_place() {
    with_bind(|| {
        let payload = draw(Ipv4Addr::new(10, 0, 0, 12), 7654, "77");
        let sealed = seal_bind_request(&payload.temp_pub, "phone-ro-exp", None).expect("seal");
        test_expire_current_session();
        let before = test_session_bytes();
        assert_eq!(bind_session_state(), BindSessionState::expired);
        assert_eq!(
            test_session_bytes(),
            before,
            "expired is computed from time; read must not clear SESSION"
        );
        assert_eq!(
            complete_bind(&sealed).expect_err("still Live+expired"),
            BindError::expired
        );
    });
}

#[test]
fn bind_session_state_does_not_return_secret() {
    with_bind(|| {
        draw(Ipv4Addr::new(10, 0, 0, 13), 7654, "88");
        let state = bind_session_state();
        assert_eq!(state, BindSessionState::live);
        let debug = format!("{state:?}");
        assert_eq!(debug, "live");
        let bind_src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/services/bind/mod.rs"
        ));
        assert!(
            !bind_src.contains("BindSessionState {")
                && !bind_src.contains("BindSessionState::live {"),
            "four-state tag must not carry secret"
        );
    });
}

#[test]
fn bind_session_state_is_read_only_across_two_calls() {
    with_bind(|| {
        draw(Ipv4Addr::new(10, 0, 0, 14), 7654, "99");
        let before = test_session_bytes();
        assert_eq!(bind_session_state(), BindSessionState::live);
        assert_eq!(bind_session_state(), BindSessionState::live);
        assert_eq!(
            test_session_bytes(),
            before,
            "two reads must leave SESSION byte-identical"
        );
    });
}

#[test]
fn bind_session_state_does_not_invent_a_clock_or_change_create_complete() {
    let bind_src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/bind/mod.rs"
    ));
    let start = bind_src
        .find("pub fn bind_session_state")
        .expect("public read-only mouth");
    let rest = &bind_src[start..];
    let end = rest.find("\npub fn ").unwrap_or(rest.len());
    let read_fn = &rest[..end];
    assert!(
        !read_fn.contains("BIND_TTL_SECS") && !read_fn.contains("now_secs() +"),
        "countdown stays on create_bind_payload.exp; read must not mint a clock"
    );
    with_bind(|| {
        let first = draw(Ipv4Addr::new(10, 0, 0, 15), 7654, "aa");
        let sealed_first =
            seal_bind_request(&first.temp_pub, "phone-ro-keep", None).expect("seal first");
        assert_eq!(bind_session_state(), BindSessionState::live);
        let second = draw(Ipv4Addr::new(10, 0, 0, 15), 7654, "aa");
        assert_ne!(first.temp_pub, second.temp_pub);
        assert_eq!(
            complete_bind(&sealed_first).expect_err("create still replaces SESSION"),
            BindError::session_unavailable
        );
        let sealed_second =
            seal_bind_request(&second.temp_pub, "phone-ro-keep-2", None).expect("seal second");
        complete_bind(&sealed_second).expect("complete_bind contract unchanged");
        assert_eq!(bind_session_state(), BindSessionState::consumed);
        assert_eq!(
            complete_bind(&sealed_second).expect_err("replay still consumed"),
            BindError::consumed
        );
    });
}
