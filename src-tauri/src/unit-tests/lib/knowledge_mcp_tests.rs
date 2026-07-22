use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use super::*;
use crate::services::local_http;
use crate::test_support::TestSandbox;

fn repo_root_with_sidecar() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

fn setup_http(repo_root: PathBuf) -> (u16, local_http::LocalHttpHandle) {
    let port = ephemeral_port();
    let handle = local_http::start(repo_root, port).expect("start http");
    thread::sleep(Duration::from_millis(50));
    (port, handle)
}

#[test]
fn try_spawn_knowledge_mcp_skips_when_http_not_ready() {
    let repo_root = repo_root_with_sidecar();
    let child = try_spawn_knowledge_mcp(false, &repo_root, 8765);
    assert!(child.is_none());
}

#[test]
fn try_spawn_knowledge_mcp_returns_none_when_sidecar_script_missing() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let repo_root = dir.path().to_path_buf();
    let child = try_spawn_knowledge_mcp(true, &repo_root, 8765);
    assert!(child.is_none());
}

#[test]
fn try_spawn_knowledge_mcp_spawns_and_listens_when_http_ready() {
    let repo_root = repo_root_with_sidecar();
    let script = repo_root.join("packages/knowledge-mcp/index.mjs");
    if !script.is_file() {
        eprintln!("skip: sidecar script missing at {}", script.display());
        return;
    }

    let (http_port, http_handle) = setup_http(repo_root.clone());
    let mcp_port = ephemeral_port();

    let child = try_spawn_knowledge_mcp_with_port(true, &repo_root, http_port, mcp_port);
    assert!(child.is_some(), "expected sidecar spawn");

    assert!(
        wait_for_port(mcp_port, Duration::from_secs(5)),
        "MCP port should become ready"
    );

    let process = KnowledgeMcpProcess::new(child);
    process.kill();
    local_http::stop(http_handle);
    assert!(
        !wait_for_port(mcp_port, Duration::from_millis(300)),
        "MCP port should close after kill"
    );
}

#[test]
fn knowledge_mcp_process_kill_leaves_no_child() {
    let repo_root = repo_root_with_sidecar();
    let script = repo_root.join("packages/knowledge-mcp/index.mjs");
    if !script.is_file() {
        return;
    }

    let (http_port, http_handle) = setup_http(repo_root.clone());
    let mcp_port = ephemeral_port();
    let child = try_spawn_knowledge_mcp_with_port(true, &repo_root, http_port, mcp_port);
    assert!(child.is_some());
    assert!(wait_for_port(mcp_port, Duration::from_secs(5)));

    let pid = child.as_ref().map(|c| c.id()).expect("pid");
    let process = KnowledgeMcpProcess::new(child);
    process.kill();
    local_http::stop(http_handle);

    thread::sleep(Duration::from_millis(200));
    assert!(
        !wait_for_port(mcp_port, Duration::from_millis(300)),
        "port should be closed"
    );
    assert!(
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| !s.success())
            .unwrap_or(true),
        "child process should be terminated"
    );
}

#[test]
fn try_spawn_knowledge_mcp_skips_when_mcp_port_in_use() {
    let repo_root = repo_root_with_sidecar();
    let script = repo_root.join("packages/knowledge-mcp/index.mjs");
    if !script.is_file() {
        return;
    }

    let mcp_port = ephemeral_port();
    let _guard = TcpListener::bind(format!("127.0.0.1:{mcp_port}")).expect("occupy mcp port");
    let (http_port, http_handle) = setup_http(repo_root.clone());

    let child = try_spawn_knowledge_mcp_with_port(true, &repo_root, http_port, mcp_port);
    assert!(child.is_none(), "spawn should fail when MCP port is occupied");

    local_http::stop(http_handle);
}

#[test]
fn try_spawn_knowledge_mcp_degrades_when_node_missing() {
    let repo_root = repo_root_with_sidecar();
    let script = repo_root.join("packages/knowledge-mcp/index.mjs");
    if !script.is_file() {
        return;
    }

    let (http_port, http_handle) = setup_http(repo_root.clone());
    let mcp_port = ephemeral_port();

    let child = try_spawn_knowledge_mcp_with_port_and_node(
        true,
        &repo_root,
        http_port,
        mcp_port,
        Path::new("/nonexistent-node-binary"),
    );
    assert!(child.is_none(), "spawn should degrade when node is unavailable");

    local_http::stop(http_handle);
}

#[test]
fn default_mcp_port_is_9876() {
    assert_eq!(DEFAULT_MCP_PORT, 9876);
}

#[test]
fn create_todo_task_mcp_tool_e2e_with_local_http() {
    let repo_root = repo_root_with_sidecar();
    let script = repo_root.join("packages/knowledge-mcp/index.mjs");
    let e2e = repo_root.join("packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs");
    if !script.is_file() || !e2e.is_file() {
        eprintln!("skip: knowledge-mcp scripts missing");
        return;
    }

    let _sandbox = TestSandbox::new();
    let wb = _sandbox.workbench_knowledge_root();
    std::fs::create_dir_all(&wb).expect("mkdir corpus");
    let todo_tasks_tasks_dir = wb.join("todo_tasks").join("tasks");
    std::fs::create_dir_all(&todo_tasks_tasks_dir).expect("mkdir todo_tasks/tasks");
    // Host todo HTTP requires durable migration gate (t5) before serving todo_* routes.
    std::fs::write(
        wb.join("todo_tasks").join(".migration_gate_passed"),
        b"ok\n",
    )
    .expect("write migration gate");
    let config_root = _sandbox.config_dir().to_path_buf();

    let (http_port, http_handle) = setup_http(config_root);
    let mcp_port = ephemeral_port();

    let child = try_spawn_knowledge_mcp_with_port(true, &repo_root, http_port, mcp_port);
    assert!(child.is_some(), "expected sidecar spawn");
    assert!(
        wait_for_port(mcp_port, Duration::from_secs(5)),
        "MCP port should become ready"
    );

    let status = Command::new("node")
        .arg(&e2e)
        .env("MCP_PORT", mcp_port.to_string())
        .env(
            "E2E_PLAN_TASKS_TASKS_DIR",
            todo_tasks_tasks_dir.to_string_lossy().as_ref(),
        )
        .status()
        .expect("run todo-task-mcp-e2e");
    assert!(status.success(), "todo-task-mcp-e2e should pass");

    let process = KnowledgeMcpProcess::new(child);
    process.kill();
    local_http::stop(http_handle);
}
