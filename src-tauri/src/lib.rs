use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(not(test))]
use std::path::PathBuf;

#[cfg(not(test))]
use tauri::webview::{NewWindowResponse, WebviewWindowBuilder};
#[cfg(not(test))]
use tauri::{Manager, Url};
#[cfg(not(test))]
use tauri_plugin_shell::process::CommandChild;
#[cfg(not(test))]
use tauri_plugin_shell::ShellExt;

#[cfg(test)]
type CommandChild = ();

const CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
const RETRY_INTERVAL: Duration = Duration::from_millis(500);
#[cfg(not(test))]
const PYTHON_PORT: u16 = 8765;
#[cfg(not(test))]
const PROBE_TIMEOUT: Duration = Duration::from_millis(500);
#[cfg(not(test))]
const SERVER_READY_TIMEOUT: Duration = Duration::from_secs(10);

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

pub struct PythonProcess(pub Arc<Mutex<Option<CommandChild>>>);

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
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must have a parent directory")
        .to_path_buf()
}

#[cfg(not(test))]
fn spawn_python_server(app: &tauri::App, child_holder: Arc<Mutex<Option<CommandChild>>>) {
    let project_root = project_root();
    let server_script = project_root.join("server.py");

    match decide_spawn(PYTHON_PORT, PROBE_TIMEOUT) {
        SpawnDecision::Skip => {
            eprintln!(
                "[P0] Port 8765 already listening; skipping python3 spawn (likely beforeDevCommand)."
            );
        }
        SpawnDecision::Spawn => match app
            .shell()
            .command("python3")
            .args([server_script.to_string_lossy().as_ref()])
            .current_dir(&project_root)
            .spawn()
        {
            Ok((mut rx, child)) => {
                *child_holder.lock().expect("python child mutex") = Some(child);
                tauri::async_runtime::spawn(async move {
                    while rx.recv().await.is_some() {}
                });
                if !wait_for_port(PYTHON_PORT, SERVER_READY_TIMEOUT) {
                    eprintln!("[P0] Warning: server.py did not bind port 8765 within 10s");
                }
            }
            Err(error) => {
                eprintln!("[P0] Failed to spawn python3: {error}");
            }
        },
    }
}

#[cfg(not(test))]
fn is_in_app_navigation(url: &Url) -> bool {
    match url.scheme() {
        "tauri" | "asset" | "file" => true,
        "http" | "https" if cfg!(debug_assertions) => {
            url.host_str() == Some("localhost") && url.port().unwrap_or(80) == PYTHON_PORT
        }
        _ => false,
    }
}

#[cfg(not(test))]
fn open_external_url(app: &tauri::AppHandle, url: &Url) {
    if let Err(error) = app.shell().open(url.as_str(), None) {
        eprintln!("[P0] Failed to open external URL {}: {error}", url);
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
                if is_in_app_navigation(url) {
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
                open_external_url(&app_handle, &url);
                NewWindowResponse::Deny
            }
        })
        .build()?;
    Ok(())
}

#[cfg(not(test))]
fn kill_python_child(app_handle: &tauri::AppHandle) {
    if let Some(child) = app_handle
        .state::<PythonProcess>()
        .0
        .lock()
        .expect("python child mutex")
        .take()
    {
        if let Err(error) = child.kill() {
            eprintln!("[P0] Failed to kill python3: {error}");
        }
    }
}

#[cfg(not(test))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![ping])
        .setup(|app| {
            create_main_window(app)?;
            let child_holder = Arc::new(Mutex::new(None::<CommandChild>));
            spawn_python_server(app, Arc::clone(&child_holder));
            app.manage(PythonProcess(child_holder));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                kill_python_child(app_handle);
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
mod python_lifecycle_tests {
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

    #[test]
    fn python_process_mutex_take_twice() {
        let holder = Arc::new(Mutex::new(None::<CommandChild>));
        assert!(holder.lock().expect("mutex").take().is_none());
        assert!(holder.lock().expect("mutex").take().is_none());
    }

    #[test]
    fn python_process_construct_with_none_holder() {
        let holder = Arc::new(Mutex::new(None::<CommandChild>));
        let process = PythonProcess(Arc::clone(&holder));
        assert!(process.0.lock().expect("mutex").is_none());
    }
}
