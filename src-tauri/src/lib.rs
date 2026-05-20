pub mod commands;
pub mod config;
pub mod integrations;
pub mod repositories;
pub mod services;

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
#[cfg(not(test))]
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(not(test))]
use tauri::webview::{NewWindowResponse, WebviewWindowBuilder};
#[cfg(not(test))]
use tauri::{Manager, Url};
#[cfg(not(test))]
use tauri_plugin_opener::OpenerExt;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
const RETRY_INTERVAL: Duration = Duration::from_millis(500);

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
    WebviewWindowBuilder::from_config(app, window_config)?
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
    Ok(())
}

#[cfg(not(test))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::read::search_knowledge,
            commands::read::search_workbench,
            commands::read::get_topics,
            commands::read::get_annotations,
            commands::read::get_annotation,
            commands::read::get_draft,
            commands::read::get_config,
            commands::config_cmd::set_config,
            commands::sync::save_comment_draft,
            commands::sync::corpus_git_commit,
            commands::sync::corpus_git_pull,
            commands::sync::kb_git_commit,
            commands::sync::kb_git_revert,
            commands::sync::delete_entry,
            commands::sync::move_entry_project,
            commands::sync::gh_move_assets,
            commands::sync::settle_entry,
            commands::sync::open_kb_in_iterm,
            commands::read::get_status,
            commands::read::kb_read,
            commands::read::kb_annotation,
            commands::read::kb_status,
            commands::read::get_repo_list,
            commands::read::get_repo_list_status,
            commands::read::get_repo_dirs,
            commands::read::check_file,
            commands::read::fetch_link_title,
            commands::read::get_corpus_index,
            commands::read::get_corpus_file,
            commands::read::get_kb_corpus_status,
            commands::search::reindex_knowledge,
            commands::search::reindex_workbench,
            commands::search::reindex_kb_repo,
            commands::search::sync_knowledge_corpus,
            commands::search::sync_workbench_repo,
            commands::search::sync_workbench_corpus,
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
        ])
        .setup(|app| {
            // Auto-start Meilisearch if not already running
            let meili_child = try_autostart_meilisearch();
            app.manage(MeiliProcess::new(meili_child));
            create_main_window(app)?;
            app.manage(services::reindex::ReindexState::new());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(meili) = app_handle.try_state::<MeiliProcess>() {
                    meili.kill();
                }
            }
        });
}

#[cfg(test)]
mod wait_for_port_tests {
    use super::*;
    use std::net::TcpListener;
    use std::time::Instant;

    fn listen_on_ephemeral() -> (TcpListener, u16) {
        let listener = TcpListener::bind("localhost:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local addr").port();
        (listener, port)
    }

    #[test]
    fn returns_true_when_port_already_listening() {
        let (_listener, port) = listen_on_ephemeral();
        let start = Instant::now();
        assert!(wait_for_port(port, Duration::from_secs(2)));
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn returns_true_when_port_listens_after_delay() {
        let port = {
            let listener = TcpListener::bind("localhost:0").expect("bind");
            let port = listener.local_addr().expect("local addr").port();
            drop(listener);
            port
        };
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            let _listener = TcpListener::bind(format!("localhost:{port}")).expect("bind delayed");
            thread::sleep(Duration::from_secs(5));
        });
        let start = Instant::now();
        assert!(wait_for_port(port, Duration::from_secs(2)));
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(500));
        assert!(elapsed < Duration::from_secs(2));
    }

    #[test]
    fn zero_timeout_on_closed_port_returns_false_immediately() {
        let port = {
            let listener = TcpListener::bind("localhost:0").expect("bind");
            let port = listener.local_addr().expect("local addr").port();
            drop(listener);
            port
        };
        let start = Instant::now();
        assert!(!wait_for_port(port, Duration::from_millis(0)));
        assert!(start.elapsed() < Duration::from_millis(200));
    }

    #[test]
    fn zero_timeout_on_open_port_returns_true() {
        let (_listener, port) = listen_on_ephemeral();
        assert!(wait_for_port(port, Duration::from_millis(0)));
    }

    #[test]
    fn returns_false_after_timeout_when_never_listening() {
        let port = {
            let listener = TcpListener::bind("localhost:0").expect("bind");
            let port = listener.local_addr().expect("local addr").port();
            drop(listener);
            port
        };
        let start = Instant::now();
        assert!(!wait_for_port(port, Duration::from_millis(800)));
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(600));
        assert!(elapsed <= Duration::from_millis(1200));
    }

    #[test]
    fn port_65535_closed_returns_false_without_panic() {
        assert!(!wait_for_port(65535, Duration::from_secs(1)));
    }
}

#[cfg(test)]
mod ping_tests {
    use super::*;

    #[test]
    fn ping_returns_pong() {
        assert_eq!(ping(), "pong");
    }

    #[test]
    fn ping_is_stateless() {
        assert_eq!(ping(), "pong");
        assert_eq!(ping(), "pong");
    }
}

#[cfg(test)]
mod spawn_decision_tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn decide_spawn_skips_when_port_is_listening() {
        let listener = TcpListener::bind("localhost:0").expect("bind");
        let port = listener.local_addr().expect("local addr").port();
        assert_eq!(
            decide_spawn(port, Duration::from_millis(200)),
            SpawnDecision::Skip
        );
    }

    #[test]
    fn decide_spawn_spawns_when_port_is_closed() {
        let listener = TcpListener::bind("localhost:0").expect("bind");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);
        assert_eq!(
            decide_spawn(port, Duration::from_millis(0)),
            SpawnDecision::Spawn
        );
    }
}
