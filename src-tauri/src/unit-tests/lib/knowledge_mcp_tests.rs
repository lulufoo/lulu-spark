use std::path::PathBuf;

use super::*;

fn lib_rs_source() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("read src-tauri/src/lib.rs")
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

/// Former Node-spawn e2e fixture. T7 removes Host spawn paths; T9 rewrites this suite.
#[test]
fn create_todo_task_mcp_tool_e2e_with_local_http() {
    eprintln!(
        "skip: Node knowledge-mcp spawn e2e retired by T7 hard-cut; Host MCP e2e deferred to T9"
    );
}
