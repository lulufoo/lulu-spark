use std::path::PathBuf;

fn lib_rs_source() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("read src-tauri/src/lib.rs")
}

#[test]
fn lib_starts_gateway_only_when_lan_ip_is_present() {
    let src = lib_rs_source();
    assert!(
        src.contains("current_lan_ipv4"),
        "startup must consult current_lan_ipv4() before Gateway"
    );
    assert!(
        src.contains("gateway::boot") || src.contains("services::gateway::boot"),
        "lib.rs must start the in-process Gateway module"
    );
    assert!(
        src.contains("effective_gateway_port"),
        "lib.rs must use the settings Gateway port"
    );
}

#[test]
fn lib_keeps_host_http_and_mcp_on_loopback() {
    let src = lib_rs_source();
    assert!(
        src.contains("SocketAddr::from(([127, 0, 0, 1], mcp_port))"),
        "MCP must stay on 127.0.0.1"
    );
    assert!(
        src.contains("main_host.try_start(repo_root.clone(), http_port)"),
        "Main Host startup must remain the existing loopback listen"
    );
    assert!(
        !src.contains("0.0.0.0") || src.contains("gateway"),
        "0.0.0.0 listen belongs to Gateway, not Main Host/MCP Host"
    );
}
