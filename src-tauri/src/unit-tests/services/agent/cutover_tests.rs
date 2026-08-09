//! T-Cutover / Phase-Cutover: cut CursorSessionRuntime per-session spawn and
//! bypass ProcessCursorRunnerClient construction. Done When = code search
//! (migration cutover is outside VERIFY runtime packages).

use std::fs;
use std::path::PathBuf;

fn agent_services_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services/agent")
}

fn read_agent_rs(name: &str) -> String {
    fs::read_to_string(agent_services_dir().join(name)).unwrap_or_else(|e| {
        panic!("read services/agent/{name}: {e}");
    })
}

/// Production service sources under `services/agent/` (`.rs` only).
fn agent_service_sources() -> Vec<(String, String)> {
    let dir = agent_services_dir();
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).expect("read services/agent") {
        let entry = entry.expect("dirent");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let src = fs::read_to_string(&path).expect("read rs");
        out.push((name, src));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

// ── Done When: only manager constructs / holds ProcessCursorRunnerClient ─────

#[test]
fn t7_only_process_manager_calls_process_cursor_runner_client_spawn() {
    let mut call_sites: Vec<String> = Vec::new();
    for (name, src) in agent_service_sources() {
        if src.contains("ProcessCursorRunnerClient::spawn") {
            call_sites.push(name);
        }
    }
    assert_eq!(
        call_sites,
        vec!["process_manager.rs".to_string()],
        "Phase-Cutover: only CursorAgentProcessManager may construct ProcessCursorRunnerClient via spawn; found in {call_sites:?}"
    );
    let pm = read_agent_rs("process_manager.rs");
    assert!(
        pm.contains("ProcessCursorRunnerClient::spawn"),
        "manager must remain the unique spawn call site"
    );
}

#[test]
fn t7_business_and_session_entry_points_must_not_construct_process_client() {
    for label in [
        "runtime.rs",
        "session.rs",
        "loop.rs",
        "llm.rs",
        "host_startup.rs",
        "engine_router.rs",
    ] {
        let src = read_agent_rs(label);
        assert!(
            !src.contains("ProcessCursorRunnerClient::spawn")
                && !src.contains("ProcessCursorRunnerClient::new"),
            "{label} must not bypass manager to construct ProcessCursorRunnerClient"
        );
    }
    let adapter = read_agent_rs("cursor_adapter.rs");
    // Method definition may live in cursor_adapter; call-site `::spawn` must not
    // (covered by t7_only_process_manager…). Also forbid a `::new` bypass.
    assert!(
        !adapter.contains("ProcessCursorRunnerClient::new"),
        "cursor_adapter must not expose alternate ProcessCursorRunnerClient::new construction"
    );
}

#[test]
fn t7_process_cursor_runner_client_spawn_is_crate_private() {
    let adapter = read_agent_rs("cursor_adapter.rs");
    let spawn_idx = adapter
        .find("impl ProcessCursorRunnerClient")
        .expect("ProcessCursorRunnerClient impl");
    let impl_src = &adapter[spawn_idx..];
    let fn_spawn = impl_src
        .find("fn spawn(")
        .expect("spawn method on ProcessCursorRunnerClient");
    let region_start = fn_spawn.saturating_sub(80);
    let region = &impl_src[region_start..fn_spawn + 24];
    assert!(
        region.contains("pub(super) fn spawn"),
        "ProcessCursorRunnerClient::spawn must be pub(super) (agent-module only); near spawn: {region:?}"
    );
    assert!(
        !region.contains("pub fn spawn") && !region.contains("pub(crate) fn spawn"),
        "ProcessCursorRunnerClient::spawn must not be pub/pub(crate) after T9 encapsulation"
    );
}

// ── Done When: CursorSessionRuntime per-session factory / ownership cut ──────

#[test]
fn t7_cursor_session_runtime_production_factory_spawn_removed() {
    let adapter = read_agent_rs("cursor_adapter.rs");
    let rt_idx = adapter
        .find("pub struct CursorSessionRuntime")
        .expect("CursorSessionRuntime still present for test doubles");
    let rt_src = &adapter[rt_idx..];
    assert!(
        !rt_src.contains("fn production("),
        "CursorSessionRuntime must not expose production() after Phase-Cutover"
    );
    assert!(
        !rt_src.contains("ProcessCursorRunnerClient::spawn"),
        "CursorSessionRuntime must not spawn ProcessCursorRunnerClient (per-session factory cut)"
    );
    assert!(
        !adapter.contains("per-session factory spawn) remains until cutover"),
        "module docs must drop pre-cutover 'remains until cutover' note"
    );
}

#[test]
fn t7_runtime_drops_prod_per_session_cursor_session_runtime_owner_path() {
    let runtime = read_agent_rs("runtime.rs");
    assert!(
        !runtime.contains("CursorSessionRuntime::production"),
        "runtime must not construct CursorSessionRuntime::production (per-session owner path)"
    );
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        adapter.contains("#[cfg(test)]\npub struct CursorSessionRuntime")
            || adapter
                .lines()
                .collect::<Vec<_>>()
                .windows(2)
                .any(|w| w[0].contains("#[cfg(test)]") && w[1].contains("pub struct CursorSessionRuntime")),
        "CursorSessionRuntime must be cfg(test)-only after isolation"
    );
    assert!(
        !runtime.contains("PROD_CURSOR_RT") && !runtime.contains("fn cursor_runtime("),
        "runtime must drop dead prod CursorSessionRuntime slot / cursor_runtime() factory (t7 cutover)"
    );
    assert!(
        runtime.contains("CursorLlmEngine"),
        "runtime production Cursor path must remain CursorLlmEngine after cutover"
    );
}

#[test]
fn t7_no_dual_path_or_rollback_feature_flag_for_session_held_client() {
    for label in ["cursor_adapter.rs", "runtime.rs", "process_manager.rs"] {
        let src = read_agent_rs(label);
        for needle in [
            "legacy_per_session",
            "per_session_client",
            "dual_path_spawn",
            "CUTOVER_COMPAT",
            "session_held_client",
            "allow_session_spawn",
            "feature = \"per-session-runner\"",
            "feature = \"legacy_cursor_runtime\"",
        ] {
            assert!(
                !src.contains(needle),
                "{label} must not retain dual-path / rollback flag `{needle}` (no compat window)"
            );
        }
    }
}
