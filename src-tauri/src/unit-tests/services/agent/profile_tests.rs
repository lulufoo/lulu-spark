use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::config::settings;
use crate::services::agent::profile::query_business_profile;
use crate::services::mcp_server_registry;
use crate::test_support::TestSandbox;

struct JsonStub {
    port: u16,
    stop: Arc<AtomicBool>,
}

impl Drop for JsonStub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn spawn_json_stub(body: String) -> JsonStub {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let port = listener.local_addr().expect("stub address").port();
    listener
        .set_nonblocking(true)
        .expect("nonblocking stub listener");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);
    thread::spawn(move || {
        while !stop_flag.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = serve_json(&mut stream, &body);
                }
                Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });
    thread::sleep(Duration::from_millis(20));

    JsonStub { port, stop }
}

fn serve_json(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
    let mut request = [0u8; 2048];
    let _ = stream.read(&mut request);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

fn configure_profile_settings(model: Option<&str>, http_port: u16, mcp_port: u16) {
    let mut config = settings::load().expect("load sandbox settings");
    config.http_port = Some(http_port);
    config.mcp_port = Some(mcp_port);
    config.llm.clear();
    if let Some(model) = model {
        settings::upsert_llm_entry(
            &mut config.llm,
            "cursor",
            &settings::LlmSettings {
                platform: "cursor_agent".into(),
                base_url: "https://cursor.invalid".into(),
                model: model.into(),
            },
        )
        .expect("configure cursor settings");
    }
    settings::save(&config).expect("save sandbox settings");
}

fn unused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind unused port");
    listener.local_addr().expect("unused address").port()
}

#[test]
fn valid_business_profile_is_complete_and_stable() {
    let _sandbox = TestSandbox::new();
    mcp_server_registry::clear_for_tests();

    let http = spawn_json_stub(r#"{"ok":true,"http_port":1}"#.into());
    let mcp = spawn_json_stub(r#"{"ok":true,"mcp":"http://127.0.0.1/mcp"}"#.into());
    configure_profile_settings(Some("cursor-model"), http.port, mcp.port);
    mcp_server_registry::seed_defaults();

    let first = query_business_profile("todo_task").expect("valid profile");
    let second = query_business_profile("todo_task").expect("stable profile");

    assert_eq!(first.business_id, "todo_task");
    assert_eq!(first.model, "cursor-model");
    assert_eq!(first.cwd, second.cwd);
    assert!(first.cwd.is_dir(), "profile cwd must be a directory");
    assert!(
        !first.mcp_servers.is_empty(),
        "healthy registry candidate must produce mcpServers"
    );
    let server = first
        .mcp_servers
        .get("workbench")
        .expect("workbench mcp server");
    assert_eq!(
        server.url,
        format!("http://127.0.0.1:{}/mcp/todo_task", mcp.port)
    );

    let encoded = serde_json::to_value(&first).expect("serialize profile");
    assert!(encoded.get("session_id").is_none());
    assert!(encoded.get("masterTaskId").is_none());
    assert!(encoded.get("master_task_id").is_none());

    mcp_server_registry::clear_for_tests();
}

#[test]
fn registry_candidate_without_readiness_is_an_explicit_failure() {
    let _sandbox = TestSandbox::new();
    mcp_server_registry::clear_for_tests();
    configure_profile_settings(Some("cursor-model"), unused_port(), unused_port());
    mcp_server_registry::seed_defaults();

    let result = query_business_profile("todo_task");
    assert!(
        result.is_err(),
        "registry presence without healthy endpoints must fail explicitly: {result:?}"
    );

    mcp_server_registry::clear_for_tests();
}

#[test]
fn unknown_or_invalid_business_id_does_not_fall_back_to_todo_task() {
    let _sandbox = TestSandbox::new();
    mcp_server_registry::clear_for_tests();

    let http = spawn_json_stub(r#"{"ok":true,"http_port":1}"#.into());
    let mcp = spawn_json_stub(r#"{"ok":true,"mcp":"http://127.0.0.1/mcp"}"#.into());
    configure_profile_settings(Some("cursor-model"), http.port, mcp.port);
    mcp_server_registry::seed_defaults();

    assert!(query_business_profile("unknown_business").is_err());
    assert!(query_business_profile("").is_err());
    assert!(query_business_profile("../escape").is_err());

    mcp_server_registry::clear_for_tests();
}

#[test]
fn missing_cursor_model_is_an_explicit_profile_failure() {
    let _sandbox = TestSandbox::new();
    mcp_server_registry::clear_for_tests();

    let http = spawn_json_stub(r#"{"ok":true,"http_port":1}"#.into());
    let mcp = spawn_json_stub(r#"{"ok":true,"mcp":"http://127.0.0.1/mcp"}"#.into());
    configure_profile_settings(None, http.port, mcp.port);
    mcp_server_registry::seed_defaults();

    assert!(query_business_profile("todo_task").is_err());

    mcp_server_registry::clear_for_tests();
}
