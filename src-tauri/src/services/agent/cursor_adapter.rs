//! Cursor Local adapter — Agent SDK Local via Node JSONL runner.
//!
//! Isolated from Host Loop (`loop.rs` / `llm.rs` must not import this module).
//! Cursor failures stay on the SDK path (L09-I #7 — no fallback to
//! process-local business tool dispatch).
//!
//! ## Production path
//!
//! [`CursorLlmEngine`] obtains temporary access via
//! [`CursorAgentProcessManager::ensure_client`](crate::services::agent::process_manager::CursorAgentProcessManager::ensure_client)
//! and submits JSONL with caller-provided `request_id` through
//! [`ClientAccess::request`](crate::services::agent::process_manager::ClientAccess::request).
//! The managed [`ProcessCursorRunnerClient`] speaks `create` / `turn` / `cancel` /
//! `close`; API key is injected only via child env `CURSOR_API_KEY` (never JSONL).
//!
//! [`CursorSessionRuntime`] (per-session factory spawn) remains until cutover (t7).
//!
//! ## Tests
//!
//! Inject [`FakeCursorRunnerClient`] via manager factory / [`CursorSessionRuntime::with_client_factory`].
//! Legacy [`CursorAgentSdk`] capturing doubles remain for older shape tests.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use crate::cursor_agent_runner_launch_path;
use crate::services::agent::engine_router::TurnInput;
use crate::services::agent::process_manager::{
    CursorAgentProcessManager, EnsureError, RequestError,
};
use crate::services::agent::r#loop;
use crate::services::agent::session_cwd;
use crate::services::mcp_endpoint_readiness::{self, ReadyMcpTransports};
use crate::services::mcp_server_registry::McpServerConfig;

/// A1 confirmed: per-session `local.cwd` combined with injected `mcpServers`.
pub const CURSOR_LOCAL_A1_CWD_MCPSERVERS_COMBO: bool = true;

/// A2: SDK embed via replaceable port / process client.
pub const CURSOR_SDK_A2_REPLACEABLE_PORT: bool = true;

/// Host LLM generic upstream message — Cursor errors must never collapse to this.
pub const HOST_GENERIC_UPSTREAM_UNAVAILABLE: &str = "上游服务暂时不可用，请稍后重试。";

/// Default inline mcpServers entry name for the session capability config.
pub const DEFAULT_MCP_SERVER_NAME: &str = "workbench";

const DEFAULT_CANCEL_TIMEOUT: Duration = Duration::from_secs(5);

// ── Legacy shape (t4/t5 capturing doubles) ───────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpServerInlineEntry {
    pub capability_description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpServersInline {
    #[serde(flatten)]
    pub servers: BTreeMap<String, McpServerInlineEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCreateParams {
    pub mcp_servers: McpServersInline,
    pub local_cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorAdapterError {
    MissingMcpConfig,
    Cwd(String),
    Sdk(String),
}

impl std::fmt::Display for CursorAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CursorAdapterError::MissingMcpConfig => {
                write!(f, "session capability MCP config is not loaded")
            }
            CursorAdapterError::Cwd(msg) => write!(f, "session cwd error: {msg}"),
            CursorAdapterError::Sdk(msg) => write!(f, "cursor sdk error: {msg}"),
        }
    }
}

impl std::error::Error for CursorAdapterError {}

pub trait CursorAgentSdk {
    fn create(&mut self, params: AgentCreateParams) -> Result<(), CursorAdapterError>;
    fn run_turn(&mut self, message: &str) -> Result<String, CursorAdapterError>;
}

pub fn map_mcp_config_to_mcp_servers(config: &McpServerConfig) -> McpServersInline {
    let mut servers = BTreeMap::new();
    servers.insert(
        DEFAULT_MCP_SERVER_NAME.to_string(),
        McpServerInlineEntry {
            capability_description: config.capability_description.clone(),
        },
    );
    McpServersInline { servers }
}

pub fn create_agent<S: CursorAgentSdk>(
    sdk: &mut S,
    mcp_config: &McpServerConfig,
    cwd: PathBuf,
) -> Result<(), CursorAdapterError> {
    let params = AgentCreateParams {
        mcp_servers: map_mcp_config_to_mcp_servers(mcp_config),
        local_cwd: cwd,
    };
    sdk.create(params)
}

pub fn run_turn<S: CursorAgentSdk>(
    sdk: &mut S,
    input: &TurnInput,
    cwd: PathBuf,
) -> Result<String, CursorAdapterError> {
    let Some(mcp) = r#loop::session_capability_mcp_config() else {
        return Err(CursorAdapterError::MissingMcpConfig);
    };
    create_agent(sdk, &mcp, cwd)?;
    sdk.run_turn(&input.message)
}

pub fn run_turn_for_session<S: CursorAgentSdk>(
    sdk: &mut S,
    input: &TurnInput,
) -> Result<String, CursorAdapterError> {
    let cwd = session_cwd::create_session_cwd(&input.session_id)
        .map_err(|e| CursorAdapterError::Cwd(e.to_string()))?;
    run_turn(sdk, input, cwd)
}

// ── Production typed errors ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorErrorCode {
    Credential,
    SdkConfig,
    McpUnavailable,
    Cwd,
    Runner,
    SdkRun,
    Cancelled,
    Busy,
    /// Process / JSONL / generation-stale communication failure (not foreground Cancelled).
    RecoverableFailure,
}

/// Caller-facing error track: Cancelled (foreground interrupt) vs RecoverableFailure
/// (process/JSONL/replace invalidate). Both must not persist as Session turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorTrack {
    Cancelled,
    RecoverableFailure,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorError {
    pub code: CursorErrorCode,
    pub message: String,
}

pub fn error_track(err: &CursorError) -> ErrorTrack {
    match err.code {
        CursorErrorCode::Cancelled => ErrorTrack::Cancelled,
        CursorErrorCode::RecoverableFailure
        | CursorErrorCode::Runner
        | CursorErrorCode::SdkRun => ErrorTrack::RecoverableFailure,
        _ => ErrorTrack::Other,
    }
}

impl std::fmt::Display for CursorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code_str(), self.message)
    }
}

impl std::error::Error for CursorError {}

impl CursorError {
    pub fn new(code: CursorErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn code_str(&self) -> &'static str {
        match self.code {
            CursorErrorCode::Credential => "credential",
            CursorErrorCode::SdkConfig => "sdk_config",
            CursorErrorCode::McpUnavailable => "mcp_unavailable",
            CursorErrorCode::Cwd => "cwd",
            CursorErrorCode::Runner => "runner",
            CursorErrorCode::SdkRun => "sdk_run",
            CursorErrorCode::Cancelled => "cancelled",
            CursorErrorCode::Busy => "busy",
            CursorErrorCode::RecoverableFailure => "recoverable_failure",
        }
    }
}

pub fn frontend_message_for(code: CursorErrorCode) -> &'static str {
    match code {
        CursorErrorCode::Credential => "Cursor 鉴权失败，请检查 API Key 配置。",
        CursorErrorCode::SdkConfig => "Cursor SDK 配置无效或当前环境不支持。",
        CursorErrorCode::McpUnavailable => "Workbench MCP 未就绪，无法启动 Cursor 引擎。",
        CursorErrorCode::Cwd => "Cursor 会话工作目录不可用。",
        CursorErrorCode::Runner => "Cursor 运行进程异常。",
        CursorErrorCode::SdkRun => "Cursor Agent 执行失败。",
        CursorErrorCode::Cancelled => "Cursor 回合已取消。",
        CursorErrorCode::Busy => "当前会话已有进行中的 Cursor 回合。",
        CursorErrorCode::RecoverableFailure => "Cursor 通信失败，请重试。",
    }
}

pub fn map_runner_error(runner_type: &str, _detail: &str) -> CursorError {
    let code = match runner_type {
        "credential" => CursorErrorCode::Credential,
        "sdk_config" => CursorErrorCode::SdkConfig,
        "mcp_unavailable" => CursorErrorCode::McpUnavailable,
        "cwd" => CursorErrorCode::Cwd,
        "runner" => CursorErrorCode::Runner,
        "sdk_run" => CursorErrorCode::SdkRun,
        "cancelled" => CursorErrorCode::Cancelled,
        "busy" => CursorErrorCode::Busy,
        "recoverable_failure" | "recoverable" => CursorErrorCode::RecoverableFailure,
        _ => CursorErrorCode::Runner,
    };
    CursorError::new(code, frontend_message_for(code))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutcome {
    pub text: String,
    pub should_persist: bool,
}

pub fn should_persist_cursor_result(result: &Result<TurnOutcome, CursorError>) -> bool {
    match result {
        Ok(o) => o.should_persist,
        Err(e) => !matches!(
            error_track(e),
            ErrorTrack::Cancelled | ErrorTrack::RecoverableFailure
        ) && e.code != CursorErrorCode::Busy,
    }
}

#[derive(Debug, Clone)]
pub struct TurnRequest {
    pub session_id: String,
    pub prompt: String,
    pub model: String,
    pub api_key: String,
    pub ready_mcp: ReadyMcpTransports,
}

// ── Runner client port ───────────────────────────────────────────────────────

pub trait CursorRunnerClient: Send {
    fn create(
        &mut self,
        model: &str,
        cwd: &Path,
        mcp_servers: &Value,
    ) -> Result<(), CursorError>;
    fn turn(&mut self, prompt: &str) -> Result<String, CursorError>;
    fn cancel(&mut self) -> Result<(), CursorError>;
    fn close(&mut self) -> Result<(), CursorError>;
    fn force_kill(&mut self);
    /// JSONL request with caller-provided `request_id` (pending map / wire id).
    /// Client must not invent a different id.
    fn request(&mut self, request_id: &str, method: &str, params: Option<Value>) -> Result<Value, CursorError>;
    /// Hook usable without locking the client mutex (cancel-timeout / hang path).
    fn terminate_hook(&self) -> Arc<dyn Fn() + Send + Sync> {
        let _ = self;
        Arc::new(|| {})
    }
    fn on_spawned_with_api_key(&mut self, _api_key: &str) {}
}

type ClientFactory =
    Box<dyn FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send>;

// ── Fake client (unit tests) ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FakeLogEntry {
    pub request_id: Option<String>,
    pub method: String,
    pub params: Option<Value>,
}

struct FakeShared {
    log: Arc<Mutex<Vec<FakeLogEntry>>>,
    force_killed: Arc<AtomicBool>,
    dispose_awaited: Arc<AtomicBool>,
    last_spawn_api_key: Arc<Mutex<Option<String>>>,
    turn_block: Mutex<Option<Duration>>,
    cancel_hang: AtomicBool,
    in_flight: AtomicBool,
    cancel_requested: AtomicBool,
}

fn fake_registry() -> &'static Mutex<HashMap<usize, Arc<FakeShared>>> {
    static REG: OnceLock<Mutex<HashMap<usize, Arc<FakeShared>>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct FakeCursorRunnerClient {
    shared: Arc<FakeShared>,
    pub log: Arc<Mutex<Vec<FakeLogEntry>>>,
    pub force_killed: Arc<AtomicBool>,
    pub dispose_awaited: Arc<AtomicBool>,
    pub last_spawn_api_key: Arc<Mutex<Option<String>>>,
}

impl FakeCursorRunnerClient {
    pub fn new() -> Self {
        let log = Arc::new(Mutex::new(Vec::new()));
        let force_killed = Arc::new(AtomicBool::new(false));
        let dispose_awaited = Arc::new(AtomicBool::new(false));
        let last_spawn_api_key = Arc::new(Mutex::new(None));
        let shared = Arc::new(FakeShared {
            log: log.clone(),
            force_killed: force_killed.clone(),
            dispose_awaited: dispose_awaited.clone(),
            last_spawn_api_key: last_spawn_api_key.clone(),
            turn_block: Mutex::new(None),
            cancel_hang: AtomicBool::new(false),
            in_flight: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),
        });
        let key = Arc::as_ptr(&log) as usize;
        fake_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, shared.clone());
        Self {
            shared,
            log,
            force_killed,
            dispose_awaited,
            last_spawn_api_key,
        }
    }

    pub fn from_shared(log: Arc<Mutex<Vec<FakeLogEntry>>>) -> Self {
        let key = Arc::as_ptr(&log) as usize;
        let shared = fake_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key)
            .cloned()
            .expect(
                "FakeCursorRunnerClient::from_shared requires log from FakeCursorRunnerClient::new",
            );
        Self {
            log: shared.log.clone(),
            force_killed: shared.force_killed.clone(),
            dispose_awaited: shared.dispose_awaited.clone(),
            last_spawn_api_key: shared.last_spawn_api_key.clone(),
            shared,
        }
    }

    pub fn set_turn_block(&self, d: Duration) {
        *self
            .shared
            .turn_block
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(d);
    }

    pub fn set_cancel_hang(&self, hang: bool) {
        self.shared.cancel_hang.store(hang, Ordering::SeqCst);
    }

    pub fn mark_in_flight(&self, v: bool) {
        self.shared.in_flight.store(v, Ordering::SeqCst);
    }

    pub fn on_spawned_with_api_key(&mut self, api_key: &str) {
        *self
            .shared
            .last_spawn_api_key
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(api_key.to_string());
    }

    fn push_log(&self, method: &str, params: Option<Value>) {
        self.push_log_with_id(None, method, params);
    }

    fn push_log_with_id(&self, request_id: Option<String>, method: &str, params: Option<Value>) {
        self.shared
            .log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(FakeLogEntry {
                request_id,
                method: method.into(),
                params,
            });
    }
}

impl Default for FakeCursorRunnerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorRunnerClient for FakeCursorRunnerClient {
    fn create(
        &mut self,
        model: &str,
        cwd: &Path,
        mcp_servers: &Value,
    ) -> Result<(), CursorError> {
        self.push_log(
            "create",
            Some(json!({
                "model": model,
                "cwd": cwd.to_string_lossy(),
                "mcpServers": mcp_servers,
            })),
        );
        Ok(())
    }

    fn turn(&mut self, prompt: &str) -> Result<String, CursorError> {
        self.shared.in_flight.store(true, Ordering::SeqCst);
        self.push_log("turn", Some(json!({ "prompt": prompt })));
        if let Some(d) = *self
            .shared
            .turn_block
            .lock()
            .unwrap_or_else(|e| e.into_inner())
        {
            let start = Instant::now();
            while start.elapsed() < d {
                if self.shared.cancel_requested.load(Ordering::SeqCst) {
                    self.shared.in_flight.store(false, Ordering::SeqCst);
                    return Err(map_runner_error("cancelled", "run cancelled"));
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
        if self.shared.cancel_requested.load(Ordering::SeqCst) {
            self.shared.in_flight.store(false, Ordering::SeqCst);
            return Err(map_runner_error("cancelled", "run cancelled"));
        }
        self.shared.in_flight.store(false, Ordering::SeqCst);
        Ok("fake-ok".into())
    }

    fn cancel(&mut self) -> Result<(), CursorError> {
        self.push_log("cancel", None);
        self.shared.cancel_requested.store(true, Ordering::SeqCst);
        if self.shared.cancel_hang.load(Ordering::SeqCst) {
            // Hang until force_kill flips the flag / clears hang.
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(30) {
                if self.shared.force_killed.load(Ordering::SeqCst) {
                    self.shared.in_flight.store(false, Ordering::SeqCst);
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
        self.shared.in_flight.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn close(&mut self) -> Result<(), CursorError> {
        // Node side awaits agent[Symbol.asyncDispose] before responding.
        self.shared.dispose_awaited.store(true, Ordering::SeqCst);
        self.push_log("close", None);
        Ok(())
    }

    fn force_kill(&mut self) {
        (self.terminate_hook())();
    }

    fn terminate_hook(&self) -> Arc<dyn Fn() + Send + Sync> {
        let s = self.shared.clone();
        Arc::new(move || {
            s.force_killed.store(true, Ordering::SeqCst);
            s.in_flight.store(false, Ordering::SeqCst);
            s.cancel_requested.store(true, Ordering::SeqCst);
        })
    }

    fn on_spawned_with_api_key(&mut self, api_key: &str) {
        Self::on_spawned_with_api_key(self, api_key);
    }

    fn request(
        &mut self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        match method {
            "create" => {
                self.push_log_with_id(Some(request_id.into()), "create", params);
                Ok(json!({}))
            }
            "turn" => {
                self.shared.in_flight.store(true, Ordering::SeqCst);
                self.push_log_with_id(Some(request_id.into()), "turn", params.clone());
                if let Some(d) = *self
                    .shared
                    .turn_block
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                {
                    let start = Instant::now();
                    while start.elapsed() < d {
                        if self.shared.cancel_requested.load(Ordering::SeqCst) {
                            self.shared.in_flight.store(false, Ordering::SeqCst);
                            return Err(map_runner_error("cancelled", "run cancelled"));
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                }
                if self.shared.cancel_requested.load(Ordering::SeqCst) {
                    self.shared.in_flight.store(false, Ordering::SeqCst);
                    return Err(map_runner_error("cancelled", "run cancelled"));
                }
                self.shared.in_flight.store(false, Ordering::SeqCst);
                Ok(json!({ "text": "fake-ok" }))
            }
            "cancel" => {
                self.push_log_with_id(Some(request_id.into()), "cancel", None);
                self.shared.cancel_requested.store(true, Ordering::SeqCst);
                self.shared.in_flight.store(false, Ordering::SeqCst);
                Ok(json!({}))
            }
            "close" => {
                self.shared.dispose_awaited.store(true, Ordering::SeqCst);
                self.push_log_with_id(Some(request_id.into()), "close", None);
                Ok(json!({}))
            }
            _ => Err(map_runner_error("runner", "unknown method")),
        }
    }
}

// ── Production process client ────────────────────────────────────────────────

pub struct ProcessCursorRunnerClient {
    child: Option<Child>,
    pid: Arc<Mutex<Option<u32>>>,
    stdin: Option<std::process::ChildStdin>,
    stdout: Option<BufReader<std::process::ChildStdout>>,
    next_id: u64,
}

impl ProcessCursorRunnerClient {
    /// Spawn Node runner with `CURSOR_API_KEY` in child env only.
    pub fn spawn(repo_root: &Path, api_key: &str) -> Result<Self, CursorError> {
        let script = cursor_agent_runner_launch_path(repo_root);
        if !script.is_file() {
            return Err(CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            ));
        }
        let mut child = Command::new("node")
            .arg(&script)
            .current_dir(repo_root)
            .env("CURSOR_API_KEY", api_key)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| {
                CursorError::new(
                    CursorErrorCode::Runner,
                    frontend_message_for(CursorErrorCode::Runner),
                )
            })?;
        let pid = Arc::new(Mutex::new(Some(child.id())));
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().map(BufReader::new);
        Ok(Self {
            child: Some(child),
            pid,
            stdin,
            stdout,
            next_id: 1,
        })
    }

    fn exchange(&mut self, method: &str, params: Option<Value>) -> Result<Value, CursorError> {
        let id = format!("r{}", self.next_id);
        self.next_id += 1;
        self.exchange_with_id(&id, method, params)
    }

    fn exchange_with_id(
        &mut self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        let mut req = json!({ "id": request_id, "method": method });
        if let Some(p) = params {
            req["params"] = p;
        }
        let line = serde_json::to_string(&req).map_err(|_| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;
        writeln!(stdin, "{line}").map_err(|_| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;
        stdin.flush().map_err(|_| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;

        let stdout = self.stdout.as_mut().ok_or_else(|| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;
        let mut response_line = String::new();
        stdout.read_line(&mut response_line).map_err(|_| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;
        let trimmed = response_line.trim();
        if trimmed.is_empty() {
            return Err(CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            ));
        }
        let v: Value = serde_json::from_str(trimmed).map_err(|_| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;
        let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
        if ok {
            Ok(v.get("result").cloned().unwrap_or(json!({})))
        } else {
            let err_type = v
                .pointer("/error/type")
                .and_then(|x| x.as_str())
                .unwrap_or("runner");
            let detail = v
                .pointer("/error/message")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            Err(map_runner_error(err_type, detail))
        }
    }
}

impl CursorRunnerClient for ProcessCursorRunnerClient {
    fn create(
        &mut self,
        model: &str,
        cwd: &Path,
        mcp_servers: &Value,
    ) -> Result<(), CursorError> {
        let params = json!({
            "model": model,
            "cwd": cwd.to_string_lossy(),
            "mcpServers": mcp_servers,
        });
        self.exchange("create", Some(params))?;
        Ok(())
    }

    fn turn(&mut self, prompt: &str) -> Result<String, CursorError> {
        let result = self.exchange("turn", Some(json!({ "prompt": prompt })))?;
        let text = result
            .get("text")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        Ok(text)
    }

    fn cancel(&mut self) -> Result<(), CursorError> {
        self.exchange("cancel", None)?;
        Ok(())
    }

    fn close(&mut self) -> Result<(), CursorError> {
        let _ = self.exchange("close", None)?;
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
        self.stdin = None;
        self.stdout = None;
        Ok(())
    }

    fn force_kill(&mut self) {
        (self.terminate_hook())();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stdin = None;
        self.stdout = None;
    }

    fn terminate_hook(&self) -> Arc<dyn Fn() + Send + Sync> {
        let pid = self.pid.clone();
        Arc::new(move || {
            if let Some(p) = *pid.lock().unwrap_or_else(|e| e.into_inner()) {
                let _ = Command::new("kill").arg("-9").arg(p.to_string()).status();
                *pid.lock().unwrap_or_else(|e| e.into_inner()) = None;
            }
        })
    }

    fn request(
        &mut self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        self.exchange_with_id(request_id, method, params)
    }
}

impl Drop for ProcessCursorRunnerClient {
    fn drop(&mut self) {
        self.force_kill();
    }
}

// ── CursorLlmEngine (Phase-Ensure) ───────────────────────────────────────────

/// Cursor engine communication path: `ensure_client` → `request(request_id, …)`.
/// Does not own/spawn `ProcessCursorRunnerClient` (manager does).
pub struct CursorLlmEngine {
    manager: Option<CursorAgentProcessManager>,
    next_request_id: AtomicU64,
    foreground_session: Mutex<Option<String>>,
}

impl CursorLlmEngine {
    pub fn production() -> Self {
        Self {
            manager: None,
            next_request_id: AtomicU64::new(1),
            foreground_session: Mutex::new(None),
        }
    }

    pub fn with_manager(manager: CursorAgentProcessManager) -> Self {
        Self {
            manager: Some(manager),
            next_request_id: AtomicU64::new(1),
            foreground_session: Mutex::new(None),
        }
    }

    pub fn global() -> &'static CursorLlmEngine {
        static GLOBAL: OnceLock<CursorLlmEngine> = OnceLock::new();
        GLOBAL.get_or_init(Self::production)
    }

    fn manager(&self) -> &CursorAgentProcessManager {
        self.manager
            .as_ref()
            .unwrap_or_else(|| CursorAgentProcessManager::global())
    }

    fn next_request_id(&self) -> String {
        let n = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        format!("eng-{n}")
    }

    /// ILlmEngine-shaped turn entry: ensure managed client, then JSONL request(s).
    pub fn run_turn(&self, req: &TurnRequest) -> Result<TurnOutcome, CursorError> {
        let cwd = match session_cwd::create_session_cwd(&req.session_id) {
            Ok(p) => p,
            Err(_) => {
                return Err(CursorError::new(
                    CursorErrorCode::Cwd,
                    frontend_message_for(CursorErrorCode::Cwd),
                ));
            }
        };
        let mcp_servers =
            serde_json::to_value(mcp_endpoint_readiness::map_to_sdk_mcp_servers(&req.ready_mcp))
                .map_err(|_| {
                    CursorError::new(
                        CursorErrorCode::Runner,
                        frontend_message_for(CursorErrorCode::Runner),
                    )
                })?;

        let access = self
            .manager()
            .ensure_client()
            .map_err(map_ensure_error)?;

        let need_create = {
            let fg = self
                .foreground_session
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            match fg.as_deref() {
                Some(id) if id == req.session_id => false,
                _ => true,
            }
        };

        if need_create {
            let rid = self.next_request_id();
            let params = json!({
                "session_id": req.session_id,
                "model": req.model,
                "cwd": cwd.to_string_lossy(),
                "mcpServers": mcp_servers,
            });
            let create_result = access
                .request(&rid, "create", Some(params))
                .map_err(map_request_error)?;
            // Bind foreground only after successful replace/create.
            {
                let mut fg = self
                    .foreground_session
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                *fg = Some(req.session_id.clone());
            }
            // Host cleans old session cwd only after runner confirms dispose via replaced_session_id.
            if let Some(replaced) = create_result
                .get("replaced_session_id")
                .and_then(|x| x.as_str())
            {
                if let Err(e) = session_cwd::cleanup_session_cwd(replaced) {
                    return Err(CursorError::new(
                        CursorErrorCode::Cwd,
                        format!("session cwd cleanup recoverable failure: {e}"),
                    ));
                }
            }
        }

        let rid = self.next_request_id();
        let result = access
            .request(&rid, "turn", Some(json!({ "prompt": req.prompt })))
            .map_err(map_request_error)?;
        let text = result
            .get("text")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        Ok(TurnOutcome {
            text,
            should_persist: true,
        })
    }
}

fn map_ensure_error(err: EnsureError) -> CursorError {
    match err {
        EnsureError::Unavailable => CursorError::new(
            CursorErrorCode::Credential,
            frontend_message_for(CursorErrorCode::Credential),
        ),
        EnsureError::Config(_) => CursorError::new(
            CursorErrorCode::SdkConfig,
            frontend_message_for(CursorErrorCode::SdkConfig),
        ),
        // Spawn stays Runner (engine ensure surface); Stale → RecoverableFailure track.
        EnsureError::Spawn => CursorError::new(
            CursorErrorCode::Runner,
            frontend_message_for(CursorErrorCode::Runner),
        ),
        EnsureError::Stale => CursorError::new(
            CursorErrorCode::RecoverableFailure,
            frontend_message_for(CursorErrorCode::RecoverableFailure),
        ),
    }
}

/// Public mapping for `ClientAccess::request` / generation-stale failures.
pub fn map_client_request_error(err: RequestError) -> CursorError {
    map_request_error(err)
}

fn map_request_error(err: RequestError) -> CursorError {
    match err {
        RequestError::Stale => CursorError::new(
            CursorErrorCode::RecoverableFailure,
            frontend_message_for(CursorErrorCode::RecoverableFailure),
        ),
        // Preserve Cancelled (foreground interrupt); leave other runner codes intact
        // so `error_track` can classify Runner/SdkRun as RecoverableFailure.
        RequestError::Runner(e) => e,
    }
}

// ── Session runtime ──────────────────────────────────────────────────────────

struct SessionState {
    client: Arc<Mutex<Box<dyn CursorRunnerClient>>>,
    terminate: Arc<dyn Fn() + Send + Sync>,
    created: bool,
    in_flight: bool,
}

pub struct CursorSessionRuntime {
    sessions: Mutex<HashMap<String, SessionState>>,
    factory: Mutex<ClientFactory>,
    cancel_timeout: Mutex<Duration>,
    cancel_gen: AtomicU64,
}

impl CursorSessionRuntime {
    pub fn with_client_factory<F>(factory: F) -> Self
    where
        F: FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send + 'static,
    {
        Self {
            sessions: Mutex::new(HashMap::new()),
            factory: Mutex::new(Box::new(factory)),
            cancel_timeout: Mutex::new(DEFAULT_CANCEL_TIMEOUT),
            cancel_gen: AtomicU64::new(0),
        }
    }

    /// Production factory: spawn [`ProcessCursorRunnerClient`] with env key.
    pub fn production(repo_root: PathBuf) -> Self {
        Self::with_client_factory(move |api_key| {
            let mut client = ProcessCursorRunnerClient::spawn(&repo_root, api_key)?;
            client.on_spawned_with_api_key(api_key);
            Ok(Box::new(client))
        })
    }

    pub fn set_cancel_timeout(&self, d: Duration) {
        *self
            .cancel_timeout
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = d;
    }

    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(session_id)
    }

    pub fn run_turn(&self, req: &TurnRequest) -> Result<TurnOutcome, CursorError> {
        let cwd = match session_cwd::create_session_cwd(&req.session_id) {
            Ok(p) => p,
            Err(_) => {
                return Err(CursorError::new(
                    CursorErrorCode::Cwd,
                    frontend_message_for(CursorErrorCode::Cwd),
                ));
            }
        };

        // Inject Host MCP Binding transports as SDK mcpServers (URL shape
        // http://127.0.0.1:9876/mcp/<slot>; readiness already probed via /health).
        let mcp_servers =
            serde_json::to_value(mcp_endpoint_readiness::map_to_sdk_mcp_servers(&req.ready_mcp))
                .map_err(|_| {
                    CursorError::new(
                        CursorErrorCode::Runner,
                        frontend_message_for(CursorErrorCode::Runner),
                    )
                })?;

        let cancel_gen_at_start = self.cancel_gen.load(Ordering::SeqCst);

        let client = {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(state) = sessions.get(&req.session_id) {
                if state.in_flight {
                    return Err(CursorError::new(
                        CursorErrorCode::Busy,
                        frontend_message_for(CursorErrorCode::Busy),
                    ));
                }
            }
            if !sessions.contains_key(&req.session_id) {
                let mut client = {
                    let mut factory = self.factory.lock().unwrap_or_else(|e| e.into_inner());
                    factory(&req.api_key)?
                };
                client.on_spawned_with_api_key(&req.api_key);
                let terminate = client.terminate_hook();
                sessions.insert(
                    req.session_id.clone(),
                    SessionState {
                        client: Arc::new(Mutex::new(client)),
                        terminate,
                        created: false,
                        in_flight: false,
                    },
                );
            }
            let state = sessions.get_mut(&req.session_id).unwrap();
            state.in_flight = true;
            state.client.clone()
        };

        let need_create = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions
                .get(&req.session_id)
                .map(|s| !s.created)
                .unwrap_or(true)
        };

        let result = (|| {
            {
                let mut guard = client.lock().unwrap_or_else(|e| e.into_inner());
                if need_create {
                    guard.create(&req.model, &cwd, &mcp_servers)?;
                }
            }
            if need_create {
                let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(state) = sessions.get_mut(&req.session_id) {
                    state.created = true;
                }
            }
            let mut guard = client.lock().unwrap_or_else(|e| e.into_inner());
            guard.turn(&req.prompt)
        })();

        {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(state) = sessions.get_mut(&req.session_id) {
                state.in_flight = false;
            }
        }

        let cancelled_after = self.cancel_gen.load(Ordering::SeqCst) != cancel_gen_at_start;
        match result {
            Ok(text) if cancelled_after => Ok(TurnOutcome {
                text,
                should_persist: false,
            }),
            Ok(text) => Ok(TurnOutcome {
                text,
                should_persist: true,
            }),
            Err(e) if e.code == CursorErrorCode::Cancelled || cancelled_after => Err(
                CursorError::new(
                    CursorErrorCode::Cancelled,
                    frontend_message_for(CursorErrorCode::Cancelled),
                ),
            ),
            Err(e) => Err(e),
        }
    }

    /// Reset / replacement Set / defensive unbound: cancel in-flight, wait terminal.
    pub fn cancel_for_binding_cut(&self, session_id: &str) -> Result<(), CursorError> {
        self.cancel_gen.fetch_add(1, Ordering::SeqCst);
        let timeout = *self
            .cancel_timeout
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let (client, terminate, was_in_flight) = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            match sessions.get(session_id) {
                Some(state) if state.in_flight || state.created => (
                    state.client.clone(),
                    state.terminate.clone(),
                    state.in_flight,
                ),
                _ => return Ok(()),
            }
        };

        // Signal interrupt immediately (does not need client mutex) so an in-flight
        // turn can observe cancellation even while it holds the client lock.
        if was_in_flight {
            terminate();
        }

        let done = Arc::new(AtomicBool::new(false));
        let done_flag = done.clone();
        let client_for_cancel = client.clone();
        let handle = thread::spawn(move || {
            let mut guard = client_for_cancel
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let _ = guard.cancel();
            done_flag.store(true, Ordering::SeqCst);
        });

        let start = Instant::now();
        while start.elapsed() < timeout && !done.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(10));
        }
        if !done.load(Ordering::SeqCst) {
            terminate();
            let _ = handle.join();
        } else {
            let _ = handle.join();
        }

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = sessions.get_mut(session_id) {
            state.in_flight = false;
        }
        Ok(())
    }

    /// Shell close: session continues — do not cancel/close/cleanup.
    pub fn on_shell_close(&self, _session_id: &str) {
        // Intentional no-op (existing shell-close semantics).
    }

    /// Close after Node awaited `agent[Symbol.asyncDispose]`, then clean cwd.
    pub fn close_session(&self, session_id: &str) -> Result<(), CursorError> {
        let client = {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            match sessions.remove(session_id) {
                Some(state) => state.client,
                None => {
                    let _ = session_cwd::cleanup_session_cwd(session_id);
                    return Ok(());
                }
            }
        };
        {
            let mut guard = client.lock().unwrap_or_else(|e| e.into_inner());
            guard.close()?;
        }
        let _ = session_cwd::cleanup_session_cwd(session_id);
        Ok(())
    }

    pub fn close_all_for_app_exit(&self) -> Result<(), CursorError> {
        let ids: Vec<String> = self
            .sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect();
        for id in ids {
            let _ = self.cancel_for_binding_cut(&id);
            self.close_session(&id)?;
        }
        Ok(())
    }
}
