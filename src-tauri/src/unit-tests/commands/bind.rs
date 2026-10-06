use std::fs;
use std::net::Ipv4Addr;
use std::path::PathBuf;

use serde_json::Value;

use crate::services::bind::{
    bind_session_state, complete_bind, create_bind_payload, seal_bind_request, test_clear_session,
    test_expire_current_session, test_reset_bind_keychain, BindError, BindSessionState,
};
use crate::gateway::{start, GatewayConfig, GatewayListen, GatewayState};
use crate::host::lan_ip::{test_override_nics, NicIpv4};
use crate::test_support::TestSandbox;

fn nic(name: &str, addr: &str) -> NicIpv4 {
    NicIpv4 {
        name: name.to_string(),
        addr: addr.parse().expect("ipv4"),
        up: true,
    }
}

fn with_nics(nics: Option<Vec<NicIpv4>>, test: impl FnOnce()) {
    test_override_nics(nics);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
    test_override_nics(None);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn with_cmd(test: impl FnOnce()) {
    let _sandbox = TestSandbox::new();
    test_reset_bind_keychain();
    test_clear_session();
    test_override_nics(None);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
    test_override_nics(None);
    test_clear_session();
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn source(rel: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|err| panic!("{rel}: {err}"))
}

fn lan_nics() -> Vec<NicIpv4> {
    vec![nic("en0", "10.0.0.4")]
}

fn start_gateway_state() -> (GatewayState, u16, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("gateway cert dir");
    let handle = start(GatewayConfig {
        listen_port: 0,
        mcp_port: 9,
        sidecar_port: 9,
        config_dir: dir.path().to_path_buf(),
    })
    .expect("start gateway");
    let port = handle.local_addr().port();
    let tls_fingerprint = handle.tls_fingerprint().to_string();
    let state = GatewayState::new();
    state.set(handle);
    (state, port, tls_fingerprint, dir)
}

fn assert_payload_object(value: &Value) -> &serde_json::Map<String, Value> {
    let obj = value
        .as_object()
        .unwrap_or_else(|| panic!("success body must be a draw object, got {value}"));
    for key in [
        "ip",
        "port",
        "temp_pub",
        "tls_fingerprint",
        "exp",
        "sign_pub",
        "sig",
    ] {
        assert!(
            obj.contains_key(key),
            "draw object must include {key}, got {value}"
        );
    }
    obj
}

fn assert_no_draw_object(result: Result<Value, String>) -> String {
    match result {
        Ok(value) => panic!("failure must not return a draw object, got {value}"),
        Err(err) => err,
    }
}

#[test]
fn issue_bind_reads_lan_and_gateway_then_calls_create_bind_payload_without_frontend_address() {
    let _: fn(&GatewayState) -> Result<Value, String> = super::issue_bind_with;
    let src = source("src/commands/bind.rs");
    assert!(
        src.contains("current_lan_ipv4"),
        "issue command must take current_lan_ipv4 itself"
    );
    assert!(
        src.contains("create_bind_payload"),
        "issue command must call create_bind_payload"
    );
    assert!(
        !src.contains("ip: Ipv4Addr")
            && !src.contains("ip: String")
            && !src.contains("port: u16")
            && !src.contains("tls_fingerprint: String")
            && !src.contains("tls_fingerprint: &str"),
        "frontend must not pass address arguments"
    );
    let listen = src
        .find(".current(")
        .or_else(|| src.find("tls_fingerprint"))
        .expect("issue command must read Gateway listen");
    let create = src
        .find("create_bind_payload")
        .expect("issue command must call create_bind_payload");
    assert!(
        listen < create,
        "Gateway port and tls_fingerprint must be read before create_bind_payload"
    );
}

#[test]
fn issue_bind_logs_critical_path_with_bind_mobile_business_id() {
    let bind_src = source("src/services/bind/mod.rs");
    assert!(
        bind_src.contains("BIND_MOBILE_BUSINESS_ID: &str = \"Bind_Mobile\""),
        "Bind logs must carry the Bind_Mobile business ID"
    );
    for event in [
        "keychain.read",
        "keychain.write",
        "keychain.ensure",
        "payload.create",
    ] {
        assert!(
            bind_src.contains(event),
            "Bind service must log critical event {event}"
        );
    }

    let command_src = source("src/commands/bind.rs");
    for outcome in ["started", "no_lan", "no_gateway", "payload_failed", "succeeded"] {
        assert!(
            command_src.contains(&format!("\"{outcome}\"")),
            "issue_bind must log outcome {outcome}"
        );
    }
}

#[test]
fn issue_bind_succeeds_with_lan_ip_and_running_gateway_handle() {
    with_cmd(|| {
        with_nics(Some(lan_nics()), || {
            let (state, port, tls_fingerprint, _dir) = start_gateway_state();
            let value = super::issue_bind_with(&state).expect("issue");
            let obj = assert_payload_object(&value);
            assert_eq!(obj["ip"], "10.0.0.4");
            assert_eq!(obj["port"].as_u64(), Some(u64::from(port)));
            assert_eq!(obj["tls_fingerprint"], tls_fingerprint);
            assert!(obj["temp_pub"].as_str().map(|s| s.len() == 64).unwrap_or(false));
            assert!(obj["sign_pub"].as_str().map(|s| s.len() == 64).unwrap_or(false));
            assert!(obj["sig"].as_str().map(|s| s.len() == 128).unwrap_or(false));
            assert!(obj["exp"].as_u64().is_some());
            assert_eq!(bind_session_state(), BindSessionState::live);
            state.stop();
        });
    });
}

#[test]
fn read_bind_session_forwards_bind_four_states_without_redefining_them() {
    with_cmd(|| {
        assert_eq!(super::read_bind_session(), "idle");
        assert_eq!(
            super::read_bind_session(),
            format!("{:?}", bind_session_state())
        );

        create_bind_payload(Ipv4Addr::new(10, 0, 0, 8), 7654, "aa").expect("create");
        assert_eq!(super::read_bind_session(), "live");
        assert_eq!(
            super::read_bind_session(),
            format!("{:?}", bind_session_state())
        );

        test_expire_current_session();
        assert_eq!(super::read_bind_session(), "expired");
        assert_eq!(
            super::read_bind_session(),
            format!("{:?}", bind_session_state())
        );

        test_clear_session();
        let payload = create_bind_payload(Ipv4Addr::new(10, 0, 0, 9), 7654, "bb").expect("create");
        let sealed = seal_bind_request(&payload.temp_pub, "phone-cmd", None).expect("seal");
        complete_bind(&sealed).expect("complete");
        assert_eq!(super::read_bind_session(), "consumed");
        assert_eq!(
            super::read_bind_session(),
            format!("{:?}", bind_session_state())
        );
    });

    let src = source("src/commands/bind.rs");
    assert!(
        src.contains("bind_session_state"),
        "read command must forward Bind four-state read"
    );
    assert!(
        !src.contains("now_secs") && !src.contains("BIND_TTL_SECS") && !src.contains("SESSION"),
        "read command must not redefine Bind four-state rules"
    );
}

#[test]
fn bind_commands_are_host_only_and_not_mounted_on_gateway() {
    let cmd = source("src/commands/bind.rs");
    for needle in [
        "build_router",
        "bind_complete",
        "0.0.0.0",
        "advertised_address",
        "/bind/complete",
        "/mcp/mobile",
    ] {
        assert!(
            !cmd.contains(needle),
            "host bind commands must stay local; found {needle}"
        );
    }

    let gateway = source("src/gateway/mod.rs");
    assert!(
        !gateway.contains("issue_bind") && !gateway.contains("read_bind_session"),
        "bind commands must not be mounted on Gateway"
    );
    assert!(
        gateway.contains(".route(\"/bind/complete\"")
            && gateway.contains(".route(\"/mcp/mobile\"")
            && gateway.contains(".route(\"/health\""),
        "named Gateway routes must stay unchanged"
    );
    assert!(
        !gateway.contains("fn complete_bind"),
        "complete_bind ticket algorithm stays in Bind, not Gateway"
    );
}

#[test]
fn gateway_state_current_reads_handle_port_and_tls_fingerprint() {
    let empty = GatewayState::new();
    assert!(
        empty.current().is_none(),
        "no handle must yield empty listen"
    );

    let dir = tempfile::tempdir().expect("gateway cert dir");
    let handle = start(GatewayConfig {
        listen_port: 0,
        mcp_port: 9,
        sidecar_port: 9,
        config_dir: dir.path().to_path_buf(),
    })
    .expect("start gateway");
    let port = handle.local_addr().port();
    let tls_fingerprint = handle.tls_fingerprint().to_string();
    let state = GatewayState::new();
    state.set(handle);
    let listen: GatewayListen = state.current().expect("handle listen");
    assert_eq!(listen.port, port);
    assert_eq!(listen.tls_fingerprint, tls_fingerprint);
    state.stop();
    assert!(
        state.current().is_none(),
        "stop must clear the readable handle"
    );

    let gateway = source("src/gateway/mod.rs");
    let start = gateway
        .find("pub fn current")
        .expect("GatewayState must expose a read of the current handle");
    let rest = &gateway[start..];
    let end = rest.find("\n    pub fn ").unwrap_or(rest.len());
    let read_fn = &rest[..end];
    assert!(
        !read_fn.contains("self.set(") && !read_fn.contains("self.stop("),
        "GatewayState read must not set or stop"
    );
}

#[test]
fn issue_bind_requires_gateway_listen_before_create_bind_payload() {
    with_cmd(|| {
        with_nics(Some(lan_nics()), || {
            let (state, port, tls_fingerprint, _dir) = start_gateway_state();
            let listen = state.current().expect("readable listen");
            assert_eq!(listen.port, port);
            assert_eq!(listen.tls_fingerprint, tls_fingerprint);
            let value = super::issue_bind_with(&state).expect("issue after listen");
            let obj = assert_payload_object(&value);
            assert_eq!(obj["port"].as_u64(), Some(u64::from(listen.port)));
            assert_eq!(obj["tls_fingerprint"], listen.tls_fingerprint);
            state.stop();
        });
    });
}

#[test]
fn read_bind_session_does_not_return_secret() {
    with_cmd(|| {
        create_bind_payload(Ipv4Addr::new(10, 0, 0, 13), 7654, "cc").expect("create");
        let payload = create_bind_payload(Ipv4Addr::new(10, 0, 0, 13), 7654, "cc").expect("replace");
        let shown = super::read_bind_session();
        assert_eq!(shown, "live");
        assert!(!shown.contains(&payload.temp_pub));
        assert!(!shown.contains(&payload.sig));
        assert!(!shown.contains("secret"));
        assert!(!shown.contains("device_mcp_token"));
    });
}

#[test]
fn issue_bind_returns_no_lan_without_draw_object_when_lan_ip_is_missing() {
    with_cmd(|| {
        with_nics(Some(vec![nic("lo0", "127.0.0.1")]), || {
            let (state, _port, _fp, _dir) = start_gateway_state();
            let err = assert_no_draw_object(super::issue_bind_with(&state));
            assert_eq!(err, "no_lan");
            state.stop();
        });
    });
}

#[test]
fn issue_bind_returns_no_gateway_without_draw_object_when_handle_is_missing() {
    with_cmd(|| {
        with_nics(Some(lan_nics()), || {
            let state = GatewayState::new();
            let err = assert_no_draw_object(super::issue_bind_with(&state));
            assert_eq!(err, "no_gateway");
            assert!(
                state.current().is_none(),
                "LAN IP must not imply a Gateway handle"
            );
        });
    });
}

#[test]
fn issue_bind_propagates_bind_error_without_folding_to_no_lan_or_no_gateway() {
    for err in [
        BindError::session_unavailable,
        BindError::expired,
        BindError::consumed,
        BindError::decrypt_failed,
        BindError::invalid_request,
        BindError::keychain_unavailable,
        BindError::rejected,
    ] {
        let mapped = super::bind_error_to_command_error(err);
        assert_eq!(mapped, format!("{err:?}"));
        assert_ne!(mapped, "no_lan");
        assert_ne!(mapped, "no_gateway");
    }

    let src = source("src/commands/bind.rs");
    assert!(
        src.contains("bind_error_to_command_error") && src.contains("create_bind_payload"),
        "create_bind_payload errors must pass through bind_error_to_command_error"
    );
}

#[test]
fn lan_ip_and_gateway_listen_are_independent() {
    with_cmd(|| {
        with_nics(Some(lan_nics()), || {
            assert_eq!(
                crate::host::lan_ip::current_lan_ipv4(),
                Some(Ipv4Addr::new(10, 0, 0, 4))
            );
            let state = GatewayState::new();
            assert!(state.current().is_none());
            let err = assert_no_draw_object(super::issue_bind_with(&state));
            assert_eq!(err, "no_gateway");
        });
    });
}

#[test]
fn bind_commands_are_declared_and_registered_in_generate_handler() {
    let commands_mod = source("src/commands/mod.rs");
    assert!(
        commands_mod.contains("pub mod bind"),
        "commands/mod.rs must declare bind"
    );

    let lib = source("src/lib.rs");
    for name in [
        "commands::bind::issue_bind",
        "commands::bind::read_bind_session",
    ] {
        assert!(
            lib.contains(name),
            "lib.rs generate_handler must register {name}"
        );
    }

    let write_acl = source("permissions/write-api.toml");
    assert!(
        write_acl.contains("\"issue_bind\""),
        "issue_bind must be allowed by write-api ACL"
    );
    let read_acl = source("permissions/read-api.toml");
    assert!(
        read_acl.contains("\"read_bind_session\""),
        "read_bind_session must be allowed by read-api ACL"
    );
}
