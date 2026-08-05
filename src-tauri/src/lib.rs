pub mod commands;
pub mod config;
pub mod integrations;
pub mod repositories;
pub mod services;

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(not(test))]
use tauri::webview::{NewWindowResponse, WebviewWindowBuilder};
#[cfg(not(test))]
use tauri::{Manager, Url, WebviewUrl, WindowEvent};
#[cfg(not(test))]
use tauri_plugin_opener::OpenerExt;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
const RETRY_INTERVAL: Duration = Duration::from_millis(500);
pub const DEFAULT_MCP_PORT: u16 = 9876;
pub const READ_LATER_ASSISTANT_LABEL: &str = "read-later-assistant";
pub const AI_ASSISTANT_LABEL: &str = "ai-assistant";
pub const PLAN_ATTACHMENT_DIALOG_EXTENSIONS: &[&str] = &["md"];

/// Launch path for the Cursor Agent SDK Node sidecar (system Node, like knowledge-mcp).
pub fn cursor_agent_runner_launch_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join("packages/cursor-agent-runner/dist/index.js")
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReadLaterAssistantWindowSpec {
    pub label: &'static str,
    pub entry: &'static str,
    pub always_on_top: bool,
}

pub fn read_later_assistant_entry_path() -> &'static str {
    "read-later-assistant.html"
}

pub fn read_later_assistant_spec() -> ReadLaterAssistantWindowSpec {
    ReadLaterAssistantWindowSpec {
        label: READ_LATER_ASSISTANT_LABEL,
        entry: read_later_assistant_entry_path(),
        always_on_top: true,
    }
}

fn localhost_addrs(port: u16) -> Vec<SocketAddr> {
    format!("localhost:{port}")
        .to_socket_addrs()
        .expect("resolve localhost")
        .collect()
}

pub fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let addrs = localhost_addrs(port);

    loop {
        if addrs
            .iter()
            .any(|addr| TcpStream::connect_timeout(addr, CONNECT_TIMEOUT).is_ok())
        {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(RETRY_INTERVAL);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpawnDecision {
    Skip,
    Spawn,
}

pub fn decide_spawn(port: u16, probe_timeout: Duration) -> SpawnDecision {
    if wait_for_port(port, probe_timeout) {
        SpawnDecision::Skip
    } else {
        SpawnDecision::Spawn
    }
}

#[cfg(not(test))]
#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[cfg(test)]
fn ping() -> &'static str {
    "pong"
}

#[cfg(not(test))]
fn is_in_app_navigation(url: &Url) -> bool {
    matches!(url.scheme(), "tauri" | "asset" | "file")
}

#[cfg(not(test))]
fn is_localhost_http_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    )
}

#[cfg(not(test))]
fn open_external_url(app: &tauri::AppHandle, url: &Url) {
    if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("[nav] Failed to open external URL {}: {error}", url);
    }
}

/// Holds the spawned Meilisearch child process so we can kill it on app exit.
#[cfg(not(test))]
pub struct MeiliProcess(Mutex<Option<std::process::Child>>);

#[cfg(not(test))]
impl MeiliProcess {
    fn new(child: Option<std::process::Child>) -> Self {
        Self(Mutex::new(child))
    }

    pub fn kill(&self) {
        if let Ok(mut guard) = self.0.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

/// Holds the embedded Host MCP runtime (independent OS thread + tokio).
/// Replaces `KnowledgeMcpProcess` child lifecycle for start/stop/join.
pub struct EmbeddedMcpRuntime {
    handle: Mutex<Option<services::mcp_protocol_adapter::McpRuntimeHandle>>,
}

impl EmbeddedMcpRuntime {
    pub fn new(handle: Option<services::mcp_protocol_adapter::McpRuntimeHandle>) -> Self {
        Self {
            handle: Mutex::new(handle),
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                let _ = services::mcp_protocol_adapter::stop_embedded_mcp_runtime(handle);
            }
        }
    }
}

/// Config retained so a dead sidecar on `mcp_port` can be respawned without App restart.
#[derive(Clone, Debug)]
pub struct KnowledgeMcpSpawnCfg {
    pub repo_root: PathBuf,
    pub http_port: u16,
    pub mcp_port: u16,
}

/// Holds the spawned knowledge-mcp sidecar so we can kill it on app exit
/// and respawn if the listen port drops while HTTP is still ready.
pub struct KnowledgeMcpProcess {
    child: Mutex<Option<std::process::Child>>,
    spawn_cfg: Mutex<Option<KnowledgeMcpSpawnCfg>>,
}

impl KnowledgeMcpProcess {
    pub fn new(child: Option<std::process::Child>) -> Self {
        Self {
            child: Mutex::new(child),
            spawn_cfg: Mutex::new(None),
        }
    }

    pub fn set_spawn_cfg(&self, cfg: KnowledgeMcpSpawnCfg) {
        if let Ok(mut guard) = self.spawn_cfg.lock() {
            *guard = Some(cfg);
        }
    }

    pub fn kill(&self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    /// If `mcp_port` is not listening and HTTP is ready, clear a dead child and spawn again.
    /// No-op when the port is already up (this process or an external listener).
    pub fn ensure_running(&self, http_ready: bool) {
        let cfg = match self.spawn_cfg.lock() {
            Ok(g) => g.clone(),
            Err(_) => return,
        };
        let Some(cfg) = cfg else {
            return;
        };
        if wait_for_port(cfg.mcp_port, Duration::from_millis(0)) {
            return;
        }
        if !http_ready {
            return;
        }
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                match child.try_wait() {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    Err(_) => {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }
            if wait_for_port(cfg.mcp_port, Duration::from_millis(0)) {
                return;
            }
            eprintln!(
                "[knowledge-mcp] port {} down — respawning sidecar",
                cfg.mcp_port
            );
            *guard = try_spawn_knowledge_mcp_with_port(
                true,
                &cfg.repo_root,
                cfg.http_port,
                cfg.mcp_port,
            );
        }
    }
}

pub fn try_spawn_knowledge_mcp(
    http_ready: bool,
    repo_root: &Path,
    http_port: u16,
) -> Option<std::process::Child> {
    try_spawn_knowledge_mcp_with_port_and_node(
        http_ready,
        repo_root,
        http_port,
        DEFAULT_MCP_PORT,
        Path::new("node"),
    )
}

#[doc(hidden)]
pub fn try_spawn_knowledge_mcp_with_port(
    http_ready: bool,
    repo_root: &Path,
    http_port: u16,
    mcp_port: u16,
) -> Option<std::process::Child> {
    try_spawn_knowledge_mcp_with_port_and_node(http_ready, repo_root, http_port, mcp_port, Path::new("node"))
}

#[doc(hidden)]
pub fn try_spawn_knowledge_mcp_with_port_and_node(
    http_ready: bool,
    repo_root: &Path,
    http_port: u16,
    mcp_port: u16,
    node: &Path,
) -> Option<std::process::Child> {
    if !http_ready {
        eprintln!("[knowledge-mcp] HTTP unavailable, MCP sidecar skipped");
        return None;
    }

    let script = repo_root.join("packages/knowledge-mcp/index.mjs");
    if !script.is_file() {
        eprintln!(
            "[knowledge-mcp] sidecar script not found: {}",
            script.display()
        );
        return None;
    }

    if decide_spawn(mcp_port, Duration::from_millis(0)) == SpawnDecision::Skip {
        eprintln!("[knowledge-mcp] port {mcp_port} already in use — spawn failed");
        return None;
    }

    let workbench_url = format!("http://127.0.0.1:{http_port}");

    match std::process::Command::new(node)
        .arg(&script)
        .env("WORKBENCH_HTTP_URL", &workbench_url)
        .env("MCP_PORT", mcp_port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            eprintln!("[knowledge-mcp] spawned pid={}", child.id());
            if wait_for_port(mcp_port, Duration::from_secs(5)) {
                eprintln!("[knowledge-mcp] ready on port {mcp_port}");
                Some(child)
            } else {
                eprintln!("[knowledge-mcp] warn: port {mcp_port} not ready after 5 s");
                let _ = child.kill();
                let _ = child.wait();
                None
            }
        }
        Err(e) => {
            eprintln!("[knowledge-mcp] spawn failed ({e})");
            None
        }
    }
}

/// Try to start the system `meilisearch` binary if the configured port is not
/// already listening. Silently skips if the binary is not installed or the port
/// is already up (e.g. user runs Meilisearch as a service).
#[cfg(not(test))]
fn try_autostart_meilisearch() -> Option<std::process::Child> {
    use crate::config::{secrets, settings};

    let cfg = settings::load().unwrap_or_default();

    // Parse port from meili_url (e.g. "http://localhost:7700")
    let port: u16 = cfg
        .meili_url
        .trim_end_matches('/')
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7700);

    // Skip if already running
    if decide_spawn(port, Duration::from_millis(0)) == SpawnDecision::Skip {
        eprintln!("[meili] port {port} already listening — skip autostart");
        return None;
    }

    let master_key = secrets::get_secret(secrets::KEY_MEILI_MASTER)
        .ok()
        .flatten()
        .unwrap_or_else(|| "lulu-workbench-local".to_string());

    let db_path = cfg.cache_dir.join(".meilisearch");
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match std::process::Command::new("meilisearch")
        .args([
            "--no-analytics",
            "--db-path",
            &db_path.to_string_lossy(),
            "--master-key",
            &master_key,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => {
            eprintln!("[meili] spawned pid={}", child.id());
            if wait_for_port(port, Duration::from_secs(5)) {
                eprintln!("[meili] ready on port {port}");
            } else {
                eprintln!("[meili] warn: port {port} not ready after 5 s");
            }
            Some(child)
        }
        Err(e) => {
            // Not installed or not in PATH — degrade gracefully
            eprintln!("[meili] autostart skipped ({e}); install with: brew install meilisearch");
            None
        }
    }
}

#[cfg(not(test))]
fn create_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .or_else(|| app.config().app.windows.first())
        .ok_or("tauri.conf.json must define at least one window")?;

    let app_handle = app.handle().clone();
    let window = WebviewWindowBuilder::from_config(app, window_config)?
        .on_navigation({
            let app_handle = app_handle.clone();
            move |url| {
                if is_in_app_navigation(url) || is_localhost_http_url(url) {
                    return true;
                }
                if url.scheme() == "http" || url.scheme() == "https" {
                    open_external_url(&app_handle, url);
                    return false;
                }
                true
            }
        })
        .on_new_window({
            let app_handle = app_handle.clone();
            move |url, _features| {
                if is_in_app_navigation(&url) || is_localhost_http_url(&url) {
                    return NewWindowResponse::Deny;
                }
                open_external_url(&app_handle, &url);
                NewWindowResponse::Deny
            }
        })
        .build()?;

    let hide_target = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hide_target.hide();
        }
    });
    Ok(())
}

#[cfg(not(test))]
fn create_read_later_assistant_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if app.get_webview_window(READ_LATER_ASSISTANT_LABEL).is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(
        app,
        "read-later-assistant",
        WebviewUrl::App("read-later-assistant.html".into()),
    )
    .always_on_top(true)
    .build()?;
    Ok(())
}

#[cfg(not(test))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::read::search_knowledge,
            commands::read::search_workbench,
            commands::read::get_topics,
            commands::read::get_annotations,
            commands::read::get_tags_registry,
            commands::read::get_annotation,
            commands::read::get_draft,
            commands::read::get_note_draft,
            commands::read::get_config,
            commands::read::infer_github_user_url,
            commands::read::check_workbench_knowledge_root,
            commands::config_cmd::set_config,
            commands::sync::save_comment_draft,
            commands::sync::save_note_draft,
            commands::sync::clear_note_draft,
            commands::sync::corpus_git_commit,
            commands::sync::corpus_git_pull,
            commands::sync::corpus_git_revert,
            commands::sync::kb_git_commit,
            commands::sync::kb_git_revert,
            commands::sync::delete_entry,
            commands::sync::move_entry_project,
            commands::sync::gh_move_assets,
            commands::sync::gh_delete_assets,
            commands::sync::settle_entry,
            commands::sync::open_kb_in_iterm,
            commands::read::get_status,
            commands::read::kb_read,
            commands::read::kb_list,
            commands::read::kb_doc_count,
            commands::read::kb_annotation,
            commands::read::kb_status,
            commands::read::get_repo_dirs,
            commands::read::check_file,
            commands::read::fetch_link_title,
            commands::read::get_corpus_index,
            commands::read::get_corpus_file,
            commands::read::get_corpus_asset,
            commands::read::get_kb_diff_status,
            commands::read::get_sediment_kb_categories,
            commands::read::get_sediment_kb_repos,
            commands::read_later::create_read_later,
            commands::read_later::get_read_later,
            commands::read_later::mark_read_later,
            commands::read_later::delete_read_later,
            commands::todo_task::get_todo_tasks,
            commands::todo_task::create_todo_task,
            commands::todo_task::delete_todo_task,
            commands::todo_task::add_todo_sub,
            commands::todo_task::delete_todo_sub,
            commands::todo_task::read_todo_md,
            commands::todo_task::update_todo_md,
            commands::todo_task::complete_todo,
            commands::todo_task::abandon_todo_sub,
            commands::todo_task::update_todo_sub,
            commands::todo_task::update_todo_master_title,
            commands::todo_task::set_todo_master_status,
            commands::todo_task::stage_todo_attachment_source,
            commands::todo_task::add_todo_attachment,
            commands::todo_task::list_todo_attachments,
            commands::todo_task::read_todo_attachment,
            commands::todo_task::save_todo_attachment,
            commands::todo_task::delete_todo_attachment,
            commands::todo_task::list_todo_comments,
            commands::todo_task::add_todo_comment,
            commands::todo_task::update_todo_comment,
            commands::todo_task::delete_todo_comment,
            commands::todo_task::list_todo_categories,
            commands::todo_task::create_todo_category,
            commands::todo_task::delete_todo_category,
            commands::todo_task::set_todo_category,
            commands::ai_assistant::open_ai_assistant,
            commands::ai_assistant::present_ai_assistant,
            commands::ai_assistant::ensure_ai_assistant_session,
            commands::ai_assistant::shell_close_ai_assistant,
            commands::ai_assistant::get_ai_assistant_binding,
            commands::ai_assistant::set_binding,
            commands::ai_assistant::reset_binding,
            commands::ai_assistant::defensive_unbound,
            commands::ai_assistant::query_binding,
            commands::ai_assistant::execute_binding,
            commands::ai_assistant::agent_chat_turn,
            commands::search::reindex_knowledge,
            commands::search::reindex_workbench,
            commands::search::reindex_kb_repo,
            commands::search::sync_knowledge_corpus,
            commands::search::get_reindex_status,
            commands::search::get_reindex_workbench_status,
            commands::write::set_done,
            commands::write::set_importance,
            commands::write::update_links,
            commands::write::update_comments,
            commands::write::reorder_comments,
            commands::write::update_highlights,
            commands::write::save_entry,
            commands::write::kb_save,
            commands::write::kb_update_comments,
            commands::write::kb_reorder_comments,
            commands::write::kb_update_highlights,
            commands::write::kb_update_links,
            commands::write::tag_attach,
            commands::write::tag_detach,
            commands::write::tag_update_value,
            commands::write::sediment_kb_add_repo,
            commands::write::sediment_kb_remove_repo,
            commands::write::sediment_kb_update_repo_category,
            commands::write::sediment_kb_add_category,
            commands::write::sediment_kb_rename_category,
            commands::write::sediment_kb_remove_category,
            commands::write::archive_document,
        ])
        .setup(|app| {
            // Auto-start Meilisearch if not already running
            let meili_child = try_autostart_meilisearch();
            app.manage(MeiliProcess::new(meili_child));

            let local_http = services::local_http::LocalHttpState::new();
            let mut knowledge_child = None;
            let mut knowledge_cfg = None;
            let mut embedded_mcp_handle = None;
            if let Ok(repo_root) = crate::config::paths::repo_root() {
                let http_port = services::local_http::DEFAULT_HTTP_PORT;
                local_http.try_start(repo_root.clone(), http_port);
                // Prefer embedded MCP runtime (dual-listen with Sidecar). On bind failure,
                // fall back to Node spawn until T5 fail-closed / T7 hard-cut remove the path.
                let mcp_bind = SocketAddr::from(([127, 0, 0, 1], DEFAULT_MCP_PORT));
                match services::mcp_protocol_adapter::start_embedded_mcp_runtime(
                    services::mcp_protocol_adapter::McpRuntimeConfig {
                        bind_addr: mcp_bind,
                    },
                ) {
                    Ok(handle) => {
                        embedded_mcp_handle = Some(handle);
                    }
                    Err(err) => {
                        eprintln!("[mcp-runtime] embedded start failed: {err}; falling back to Node spawn");
                        knowledge_child = try_spawn_knowledge_mcp(
                            local_http.is_ready(),
                            &repo_root,
                            http_port,
                        );
                        knowledge_cfg = Some(KnowledgeMcpSpawnCfg {
                            repo_root,
                            http_port,
                            mcp_port: DEFAULT_MCP_PORT,
                        });
                    }
                }
            }
            // L2 Host key→MCP registry: seed L1 internal MCP business surface before Binding Set.
            services::mcp_server_registry::seed_defaults();
            let knowledge = KnowledgeMcpProcess::new(knowledge_child);
            if let Some(cfg) = knowledge_cfg {
                knowledge.set_spawn_cfg(cfg);
            }
            app.manage(EmbeddedMcpRuntime::new(embedded_mcp_handle));
            app.manage(knowledge);
            app.manage(local_http);

            create_main_window(app)?;
            app.manage(services::reindex::ReindexState::new());

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let Ok(repo_root) = crate::config::paths::repo_root() else {
                    return;
                };
                let corpus =
                    crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
                if !corpus.join("index.json").is_file() {
                    return;
                }
                if services::tags_registry::reconcile_tags(&repo_root).is_none() {
                    use tauri::Emitter;
                    let _ = app_handle.emit("tags:reconciled", ());
                }
            });

            // Watchdog: if MCP listen port drops while HTTP is ready, respawn sidecar.
            let watchdog = app.handle().clone();
            std::thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(3));
                let Some(knowledge) = watchdog.try_state::<KnowledgeMcpProcess>() else {
                    continue;
                };
                let http_ready = watchdog
                    .try_state::<services::local_http::LocalHttpState>()
                    .map(|s| s.is_ready())
                    .unwrap_or(false);
                knowledge.ensure_running(http_ready);
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(meili) = app_handle.try_state::<MeiliProcess>() {
                    meili.kill();
                }
                if let Some(embedded) = app_handle.try_state::<EmbeddedMcpRuntime>() {
                    embedded.stop();
                }
                if let Some(knowledge) = app_handle.try_state::<KnowledgeMcpProcess>() {
                    knowledge.kill();
                }
                if let Some(local_http) = app_handle.try_state::<services::local_http::LocalHttpState>()
                {
                    local_http.stop();
                }
            }
        });
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
#[path = "unit-tests/test_support.rs"]
mod test_support_tests;

#[cfg(test)]
#[path = "unit-tests/lib/wait_for_port_tests.rs"]
mod wait_for_port_tests;

#[cfg(test)]
#[path = "unit-tests/lib/ping_tests.rs"]
mod ping_tests;

#[cfg(test)]
#[path = "unit-tests/lib/spawn_decision_tests.rs"]
mod spawn_decision_tests;

#[cfg(test)]
#[path = "unit-tests/lib/knowledge_mcp_tests.rs"]
mod knowledge_mcp_tests;

#[cfg(test)]
#[path = "unit-tests/lib/cursor_agent_runner_tests.rs"]
mod cursor_agent_runner_tests;

#[cfg(test)]
#[path = "unit-tests/lib/read_later_assistant_window_tests.rs"]
mod read_later_assistant_window_tests;

#[cfg(test)]
#[path = "unit-tests/lib/ai_assistant_window_tests.rs"]
mod ai_assistant_window_tests;
