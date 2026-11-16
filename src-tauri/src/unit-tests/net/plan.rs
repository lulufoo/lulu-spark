//! Plan resolution: precedence, dropped sources, merged exclusions.

use super::*;
use crate::net::plan::{resolve, summary_json, LOOPBACK_HOSTS, PlanInputs};

#[test]
fn no_env_or_system_resolves_direct_with_implicit_loopback() {
    let resolved = resolve_with_env(false, snapshot(None, None, &[]), &[]);
    assert_eq!(resolved.plan.http, None);
    assert_eq!(resolved.plan.https, None);
    assert_eq!(resolved.plan.no_proxy, LOOPBACK_HOSTS.join(","));
    assert!(resolved.warnings.is_empty());
}

#[test]
fn specific_env_beats_all_proxy_beats_system() {
    let resolved = resolve_with_env(
        true,
        snapshot(Some(("sys.http", Some(9000))), Some(("sys.https", Some(9001))), &[]),
        &[("HTTP_PROXY", "http://env.http:3128"), ("ALL_PROXY", "http://env.all:3129")],
    );
    let http = resolved.plan.http.expect("http proxy from specific env");
    assert_eq!(http.display, "env.http:3128");
    assert_eq!(http.source, crate::net::ProxySource::Env);
    let https = resolved.plan.https.expect("https proxy from ALL_PROXY");
    assert_eq!(https.display, "env.all:3129");

    // No env at all: the system source fills both schemes.
    let resolved = resolve_with_env(
        true,
        snapshot(Some(("sys.http", Some(9000))), Some(("sys.https", Some(9001))), &[]),
        &[],
    );
    assert_eq!(resolved.plan.http.as_ref().expect("system http").display, "sys.http:9000");
    assert_eq!(
        resolved.plan.https.as_ref().expect("system https").source,
        crate::net::ProxySource::System
    );
}

#[test]
fn toggle_off_ignores_system_but_keeps_env() {
    let resolved = resolve_with_env(
        false,
        snapshot(Some(("sys.http", Some(9000))), None, &[]),
        &[("HTTP_PROXY", "http://env.http:3128")],
    );
    assert_eq!(resolved.plan.http.as_ref().expect("env kept").display, "env.http:3128");

    let resolved = resolve_with_env(false, snapshot(Some(("sys.http", Some(9000))), None, &[]), &[]);
    assert_eq!(resolved.plan.http, None, "system source must be ignored while off");
}

#[test]
fn invalid_env_value_drops_to_next_source() {
    let resolved = resolve_with_env(
        true,
        snapshot(Some(("sys.http", Some(9000))), None, &[]),
        &[("HTTP_PROXY", "not a proxy url")],
    );
    assert_eq!(
        resolved.plan.http.as_ref().expect("system takes over").source,
        crate::net::ProxySource::System
    );
    assert!(
        resolved.warnings.iter().any(|warning| warning.contains("HTTP_PROXY")),
        "invalid source must be logged: {:?}",
        resolved.warnings
    );
}

#[test]
fn socks_env_value_is_out_of_scope_and_dropped() {
    let resolved = resolve_with_env(
        false,
        snapshot(None, None, &[]),
        &[("HTTPS_PROXY", "socks5://127.0.0.1:1080")],
    );
    assert_eq!(resolved.plan.https, None, "socks is not a supported source");
    assert!(!resolved.warnings.is_empty());
}

#[test]
fn system_exception_list_merges_into_no_proxy_with_dedupe() {
    let resolved = resolve_with_env(
        true,
        snapshot(None, None, &["*.internal", "localhost", " board.corp "]),
        &[("NO_PROXY", "a.com, b.com,localhost")],
    );
    let parts: Vec<&str> = resolved.plan.no_proxy.split(',').collect();
    assert_eq!(&parts[..3], &LOOPBACK_HOSTS, "loopback entries come first");
    assert!(parts.contains(&"a.com") && parts.contains(&"b.com"));
    assert!(parts.contains(&"*.internal") && parts.contains(&"board.corp"));
    let localhost_count = parts.iter().filter(|part| **part == "localhost").count();
    assert_eq!(localhost_count, 1, "duplicates collapse: {parts:?}");
}

#[test]
fn cgi_environment_ignores_env_sources() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    set_env("REQUEST_METHOD", "GET");
    set_env("HTTP_PROXY", "http://env.http:3128");
    let resolved = resolve(&PlanInputs {
        use_system_proxy: true,
        system: snapshot(Some(("sys.http", Some(9000))), None, &[]),
    });
    assert_eq!(
        resolved.plan.http.as_ref().expect("system wins under CGI").source,
        crate::net::ProxySource::System
    );
}

#[test]
fn credentials_stay_in_url_but_not_in_display_or_summary() {
    let resolved = resolve_with_env(
        false,
        snapshot(None, None, &[]),
        &[("HTTP_PROXY", "http://user:pass@proxy.corp:3128")],
    );
    let proxy = resolved.plan.http.as_ref().expect("credentialed proxy");
    assert_eq!(proxy.url, "http://user:pass@proxy.corp:3128");
    assert_eq!(proxy.display, "proxy.corp:3128");
    let summary = summary_json(&resolved.plan).to_string();
    assert!(!summary.contains("pass"), "summary must not leak credentials: {summary}");
    assert!(summary.contains("proxy.corp:3128"));
}

#[test]
fn bare_host_port_value_becomes_http_proxy_url() {
    let resolved = resolve_with_env(false, snapshot(None, None, &[]), &[("HTTP_PROXY", "proxy.local:8080")]);
    let proxy = resolved.plan.http.expect("bare host:port");
    assert_eq!(proxy.url, "http://proxy.local:8080");
    assert_eq!(proxy.display, "proxy.local:8080");
}

fn resolve_with_env(
    use_system_proxy: bool,
    system: crate::host::SystemProxySnapshot,
    env: &[(&str, &str)],
) -> crate::net::plan::Resolved {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    for (name, value) in env {
        set_env(name, value);
    }
    resolve(&PlanInputs {
        use_system_proxy,
        system,
    })
}
