use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use super::*;
use crate::services::local_http;
use crate::mcp_host::{
    observe_dual_listen, start_embedded_mcp_runtime, stop_embedded_mcp_runtime, McpRuntimeConfig,
    McpStartError,
};

fn lib_rs_source() -> String {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = std::fs::read_to_string(src.join("lib.rs")).expect("read src-tauri/src/lib.rs");
    out.push('\n');
    out.push_str(&crate::test_support::read_rs_dir(src.join("host")));
    out
}

fn this_test_source() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("unit-tests")
            .join("lib")
            .join("knowledge_mcp_tests.rs"),
    )
    .expect("read knowledge_mcp_tests.rs")
}

fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

#[test]
fn default_mcp_port_is_9876() {
    assert_eq!(DEFAULT_MCP_PORT, 9876);
}

/// T5: Host cannot bind `127.0.0.1:9876` → fail-closed; MUST NOT fall back to Node spawn.
#[test]
fn host_setup_must_not_fall_back_to_node_spawn_on_embed_bind_failure() {
    let lib_rs = lib_rs_source();

    assert!(
        !lib_rs.contains("falling back to Node spawn"),
        "T5: on embedded MCP bind failure Host must fail-closed — Node knowledge-mcp spawn fallback is forbidden"
    );
    assert!(
        !lib_rs.contains("until T5 fail-closed"),
        "T5: temporary Err→try_spawn_knowledge_mcp fallback comment/path must be removed"
    );

    let start_idx = lib_rs
        .find("start_embedded_mcp_runtime")
        .expect("Host setup must call start_embedded_mcp_runtime");
    let setup_slice = &lib_rs[start_idx..];
    let end_idx = setup_slice
        .find("seed_defaults")
        .unwrap_or(setup_slice.len().min(2500));
    let host_mcp_boot = &setup_slice[..end_idx];
    assert!(
        !host_mcp_boot.contains("try_spawn_knowledge_mcp"),
        "T5: Host MCP bind-failure path must not call try_spawn_knowledge_mcp / packages/knowledge-mcp/index.mjs"
    );
}

/// T7 / V1: spawn symbol definitions must leave the Host executable path.
#[test]
fn t7_hard_cut_removes_try_spawn_and_knowledge_mcp_process_symbols() {
    let lib_rs = lib_rs_source();
    for needle in [
        "fn try_spawn_knowledge_mcp(",
        "fn try_spawn_knowledge_mcp_with_port(",
        "fn try_spawn_knowledge_mcp_with_port_and_node(",
        "struct KnowledgeMcpProcess",
        "struct KnowledgeMcpSpawnCfg",
        "impl KnowledgeMcpProcess",
    ] {
        assert!(
            !lib_rs.contains(needle),
            "T7 hard-cut: `{needle}` must not remain in src-tauri/src/lib.rs"
        );
    }
}

/// T7 / V1: Host must have no code path that spawns packages/knowledge-mcp/index.mjs.
#[test]
fn t7_host_must_not_spawn_knowledge_mcp_index_mjs() {
    let lib_rs = lib_rs_source();
    assert!(
        !lib_rs.contains("packages/knowledge-mcp/index.mjs"),
        "T7/V1: Host lib.rs must not reference packages/knowledge-mcp/index.mjs (spawn path)"
    );
}

/// T7: setup uses embedded MCP runtime start; exit uses stop (not KnowledgeMcpProcess kill).
#[test]
fn t7_setup_and_teardown_use_embedded_mcp_runtime_only() {
    let lib_rs = lib_rs_source();
    assert!(
        lib_rs.contains("start_embedded_mcp_runtime"),
        "T7: setup must start embedded MCP runtime"
    );
    assert!(
        lib_rs.contains("struct EmbeddedMcpRuntime"),
        "T7: Host must hold EmbeddedMcpRuntime for lifecycle"
    );
    assert!(
        lib_rs.contains("embedded.stop()")
            || lib_rs.contains("stop_embedded_mcp_runtime"),
        "T7: exit/teardown must stop/join embedded MCP runtime"
    );
    assert!(
        !lib_rs.contains("KnowledgeMcpProcess::new"),
        "T7: setup must not construct KnowledgeMcpProcess"
    );
    assert!(
        !lib_rs.contains("app.manage(knowledge)"),
        "T7: setup must not manage KnowledgeMcpProcess state"
    );
    assert!(
        !lib_rs.contains("knowledge.kill()"),
        "T7: exit must not kill KnowledgeMcpProcess child"
    );
}

/// T7: respawn watchdog path must be deleted.
#[test]
fn t7_respawn_watchdog_path_deleted() {
    let lib_rs = lib_rs_source();
    assert!(
        !lib_rs.contains("respawning sidecar"),
        "T7: respawn watchdog log/path must be removed"
    );
    assert!(
        !lib_rs.contains("fn ensure_running"),
        "T7: KnowledgeMcpProcess::ensure_running must be removed"
    );
    assert!(
        !lib_rs.contains("Watchdog: if MCP listen port drops"),
        "T7: MCP respawn watchdog thread must be removed"
    );
    assert!(
        !lib_rs.contains("try_state::<KnowledgeMcpProcess>"),
        "T7: no Host path may hold/watch KnowledgeMcpProcess"
    );
}

/// Exception / V1: residual Node spawn lifecycle blocks P2 acceptance.
#[test]
fn t7_residual_spawn_lifecycle_blocks_p2() {
    let lib_rs = lib_rs_source();
    let residual = [
        lib_rs.contains("fn try_spawn_knowledge_mcp"),
        lib_rs.contains("struct KnowledgeMcpProcess"),
        lib_rs.contains("packages/knowledge-mcp/index.mjs"),
        lib_rs.contains("respawning sidecar"),
    ];
    assert!(
        residual.iter().all(|hit| !*hit),
        "T7/V1 failure: residual Node MCP spawn lifecycle still present in Host lib.rs"
    );
}

/// T9 / Normal: same-process Host dual listen is observable (Sidecar `:8765` + MCP `:9876` contract).
/// Ephemeral ports isolate the harness; default constants lock the production shape.
#[test]
fn embedded_mcp_dual_listen_observable() {
    assert_eq!(DEFAULT_MCP_PORT, 9876);
    assert_eq!(local_http::DEFAULT_HTTP_PORT, 8765);

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let sidecar_port = ephemeral_port();
    let http_handle = local_http::start(repo_root, sidecar_port).expect("start Sidecar tiny_http");
    thread::sleep(Duration::from_millis(50));

    let mcp_port = ephemeral_port();
    let mcp_handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: SocketAddr::from(([127, 0, 0, 1], mcp_port)),
    })
    .expect("start embedded MCP");

    let observed = observe_dual_listen(mcp_port, sidecar_port)
        .expect("T9: same-process Sidecar + MCP dual listen must be observable");
    assert!(
        observed,
        "T9: observe_dual_listen must report dual listen observed"
    );

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP");
    local_http::stop(http_handle);
}

/// T9 / Exception: occupied MCP port → bind fail-closed; no Node knowledge-mcp spawn fallback.
#[test]
fn embedded_mcp_bind_fail_closed() {
    let port = ephemeral_port();
    let bind_addr = SocketAddr::from(([127, 0, 0, 1], port));
    let _holder = TcpListener::bind(bind_addr).expect("occupy MCP port");

    let err = match start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }) {
        Ok(_) => panic!("T9: busy MCP port must fail closed (no silent success)"),
        Err(e) => e,
    };
    assert!(
        matches!(err, McpStartError::BindFailed(_)),
        "T9: bind conflict must be BindFailed, got {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("MCP bind failed"),
        "T9: fail-closed error must be clear/prefixed, got {msg:?}"
    );

    // Source gate: Host boot path must not resurrect Node spawn on bind failure.
    let lib_rs = lib_rs_source();
    assert!(
        !lib_rs.contains("packages/knowledge-mcp/index.mjs"),
        "T9: bind fail-closed must not fall back to Node knowledge-mcp"
    );
    assert!(
        !lib_rs.contains("falling back to Node spawn"),
        "T9: Node spawn fallback string must stay absent"
    );
}

/// T9 / Exception: this suite must not revive Node knowledge-mcp spawn as a pass condition.
#[test]
fn t9_suite_must_not_require_node_knowledge_mcp_spawn() {
    let src = this_test_source();
    // Split literals so this gate does not match its own source text.
    // Call-site / script patterns only (not the T7 negative string needles).
    let forbidden = [
        format!("{}{}", "todo-task-mcp-", "e2e"),
        format!("{}{}", "Command::new(", "\"node\")"),
        format!("{}{}", "KnowledgeMcpProcess::", "new(child)"),
        format!("{}{}", "process.", "ensure_running("),
        format!("{}{}", "create_todo_task_mcp_tool_", "e2e_with_local_http"),
    ];
    for needle in &forbidden {
        assert!(
            !src.contains(needle),
            "T9/V1: knowledge_mcp_tests.rs must not require Node spawn pass condition `{needle}`"
        );
    }
}
