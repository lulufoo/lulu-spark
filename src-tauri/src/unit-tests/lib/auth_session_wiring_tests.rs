use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn source(rel: &str) -> String {
    fs::read_to_string(manifest_dir().join(rel)).unwrap_or_else(|err| panic!("{rel}: {err}"))
}

fn lib_rs_source() -> String {
    source("src/lib.rs")
}

fn parse_toml_allow(text: &str, identifier: &str) -> BTreeSet<String> {
    let marker = format!("identifier = \"{identifier}\"");
    let block = text
        .split(&marker)
        .nth(1)
        .and_then(|s| s.split("commands.allow = [").nth(1))
        .and_then(|s| s.split(']').next())
        .unwrap_or_else(|| panic!("{identifier} commands.allow block"));
    block
        .split('"')
        .enumerate()
        .filter_map(|(i, s)| if i % 2 == 1 { Some(s.to_string()) } else { None })
        .collect()
}

fn parse_acl_allow(text: &str, identifier: &str) -> BTreeSet<String> {
    let v: serde_json::Value = serde_json::from_str(text).expect("acl json");
    let arr = v["__app-acl__"]["permissions"][identifier]["commands"]["allow"]
        .as_array()
        .unwrap_or_else(|| panic!("{identifier} allow array"));
    arr.iter()
        .map(|x| x.as_str().expect("cmd string").to_string())
        .collect()
}

fn opened_arm(src: &str) -> &str {
    let start = src
        .find("RunEvent::Opened")
        .expect("lib.rs must handle RunEvent::Opened");
    let rest = &src[start..];
    let end = rest.find("RunEvent::Exit").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn generate_handler_registers_auth_session_commands() {
    let src = lib_rs_source();
    for cmd in [
        "commands::auth_session::get_auth_session",
        "commands::auth_session::set_auth_session",
        "commands::auth_session::delete_auth_session",
    ] {
        assert!(src.contains(cmd), "generate_handler must register {cmd}");
    }
}

#[test]
fn commands_mod_exposes_auth_session() {
    let src = source("src/commands/mod.rs");
    assert!(
        src.contains("pub mod auth_session"),
        "commands/mod.rs must expose auth_session"
    );
}

#[test]
fn auth_session_commands_wrap_t2_vault_api_without_provider_tokens() {
    let src = source("src/commands/auth_session.rs");
    assert!(src.contains("fn get_auth_session"));
    assert!(src.contains("fn set_auth_session"));
    assert!(src.contains("fn delete_auth_session"));
    assert!(src.contains("fn emit_opened_scheme"));
    assert!(src.contains("crate::config::vault::get_auth_session"));
    assert!(src.contains("crate::config::vault::set_auth_session"));
    assert!(src.contains("crate::config::vault::delete_auth_session"));
    assert!(src.contains("AppHandle"));
    assert!(
        src.contains("spark-scheme:opened"),
        "emit_opened_scheme must emit spark-scheme:opened"
    );
    for forbidden in [
        "linkIdentity",
        "link_identity",
        "provider_token",
        "provider_refresh_token",
        "exchangeCodeForSession",
        "localStorage",
    ] {
        assert!(
            !src.contains(forbidden),
            "auth_session.rs must not contain {forbidden}"
        );
    }
}

#[test]
fn opened_run_event_forwards_scheme_to_frontend_without_exchanging_session() {
    let src = lib_rs_source();
    assert!(
        src.contains("emit_opened_scheme"),
        "Opened must hand the scheme to emit_opened_scheme"
    );
    let arm = opened_arm(&src);
    assert!(
        arm.contains("emit_opened_scheme"),
        "RunEvent::Opened must call emit_opened_scheme"
    );
    assert!(
        arm.contains("is_auth_login_scheme") && arm.contains("present_main_window"),
        "auth-login Opened must present the main window: {arm}"
    );
    for forbidden in [
        "set_auth_session",
        "delete_auth_session",
        "exchange",
        "provider_token",
        "linkIdentity",
    ] {
        assert!(
            !arm.contains(forbidden),
            "Opened must not {forbidden} in Rust"
        );
    }
}

#[test]
fn auth_session_acl_allows_get_set_delete() {
    let read_toml = source("permissions/read-api.toml");
    let write_toml = source("permissions/write-api.toml");
    let acl = source("gen/schemas/acl-manifests.json");
    let read_allow = parse_toml_allow(&read_toml, "read-api");
    let write_allow = parse_toml_allow(&write_toml, "write-api");
    assert!(
        read_allow.contains("get_auth_session"),
        "get_auth_session must be in read-api.toml"
    );
    assert!(
        write_allow.contains("set_auth_session"),
        "set_auth_session must be in write-api.toml"
    );
    assert!(
        write_allow.contains("delete_auth_session"),
        "delete_auth_session must be in write-api.toml"
    );
    assert_eq!(
        read_allow,
        parse_acl_allow(&acl, "read-api"),
        "read-api.toml vs acl-manifests.json"
    );
    assert_eq!(
        write_allow,
        parse_acl_allow(&acl, "write-api"),
        "write-api.toml vs acl-manifests.json"
    );
}

#[test]
fn vault_auth_session_type_has_no_provider_token_field() {
    let src = source("src/config/vault/types.rs");
    assert!(src.contains("struct AuthSession"));
    assert!(src.contains("struct AuthUser"));
    assert!(!src.contains("provider_token"));
    assert!(!src.contains("provider_refresh_token"));
}
