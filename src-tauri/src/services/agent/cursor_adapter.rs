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
//! Only [`CursorAgentProcessManager`](crate::services::agent::process_manager::CursorAgentProcessManager)
//! may construct / hold [`ProcessCursorRunnerClient`] (Phase-Cutover).
//!
//! T-VerifyAgent: KEEP surface locked to `request` / `concurrent_jsonl` / `close` /
//! `force_kill` / `terminate_hook` / `on_spawned_with_api_key` (no typed create/turn/cancel).
//!
//! ## Tests
//!
//! Inject [`FakeCursorRunnerClient`] via manager factory. Legacy
//! [`CursorSessionRuntime`] remains `cfg(test)`-only for older adapter/e2e doubles.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::cursor_agent_runner_launch_path;
use crate::services::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::services::agent::process_manager::{
    ClientAccess, CursorAgentProcessManager, EnsureError, RequestError,
};
use crate::services::agent::session_cwd;
use crate::services::mcp_endpoint_readiness::{self, ReadyMcpTransports};

/// A1 confirmed: per-session `local.cwd` combined with injected `mcpServers`.
pub const CURSOR_LOCAL_A1_CWD_MCPSERVERS_COMBO: bool = true;

/// A2: SDK embed via replaceable port / process client.
pub const CURSOR_SDK_A2_REPLACEABLE_PORT: bool = true;

/// Host LLM generic upstream message — Cursor errors must never collapse to this.
pub const HOST_GENERIC_UPSTREAM_UNAVAILABLE: &str = "上游服务暂时不可用，请稍后重试。";

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
        // Coalesced create never executed — recoverable, not Cancelled, not process death.
        "coalesced" | "recoverable_failure" | "recoverable" => {
            CursorErrorCode::RecoverableFailure
        }
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
        Err(e) => match error_track(e) {
            // Cancelled / RecoverableFailure never become Session turns.
            ErrorTrack::Cancelled | ErrorTrack::RecoverableFailure => false,
            ErrorTrack::Other => e.code != CursorErrorCode::Busy,
        },
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

/// Concurrent JSONL submit: wait for response without holding the client mutex.
pub trait ConcurrentJsonl: Send + Sync {
    fn submit(
        &self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError>;
}

pub trait CursorRunnerClient: Send {
    fn close(&mut self) -> Result<(), CursorError>;
    fn force_kill(&mut self);
    /// JSONL request with caller-provided `request_id` (pending map / wire id).
    /// Client must not invent a different id.
    fn request(&mut self, request_id: &str, method: &str, params: Option<Value>) -> Result<Value, CursorError>;
    /// When present, [`ClientAccess::request`] submits via this handle and does not
    /// hold the client mutex across the response wait (multi-request demux).
    fn concurrent_jsonl(&self) -> Option<Arc<dyn ConcurrentJsonl>> {
        None
    }
    /// Hook usable without locking the client mutex (cancel-timeout / hang path).
    fn terminate_hook(&self) -> Arc<dyn Fn() + Send + Sync> {
        let _ = self;
        Arc::new(|| {})
    }
    fn on_spawned_with_api_key(&mut self, _api_key: &str) {}
}

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

    /// After manager reclaim/`force_kill`, shared cancel flags stick; clear on respawn.
    pub fn reset_lifecycle_flags_for_tests(&self) {
        self.shared.force_killed.store(false, Ordering::SeqCst);
        self.shared.cancel_requested.store(false, Ordering::SeqCst);
        self.shared.in_flight.store(false, Ordering::SeqCst);
    }

    fn push_log(&self, method: &str, params: Option<Value>) {
        self.shared
            .log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(FakeLogEntry {
                request_id: None,
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
        fake_shared_request(&self.shared, request_id, method, params)
    }

    fn concurrent_jsonl(&self) -> Option<Arc<dyn ConcurrentJsonl>> {
        Some(Arc::new(FakeConcurrent(self.shared.clone())) as Arc<dyn ConcurrentJsonl>)
    }
}

struct FakeConcurrent(Arc<FakeShared>);

impl ConcurrentJsonl for FakeConcurrent {
    fn submit(
        &self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        fake_shared_request(&self.0, request_id, method, params)
    }
}

fn fake_shared_request(
    shared: &FakeShared,
    request_id: &str,
    method: &str,
    params: Option<Value>,
) -> Result<Value, CursorError> {
    shared.log.lock().unwrap_or_else(|e| e.into_inner()).push(FakeLogEntry {
        request_id: Some(request_id.into()),
        method: method.into(),
        params: params.clone(),
    });
    match method {
        "create" => Ok(json!({})),
        "turn" => {
            shared.in_flight.store(true, Ordering::SeqCst);
            if let Some(d) = *shared.turn_block.lock().unwrap_or_else(|e| e.into_inner()) {
                let start = Instant::now();
                while start.elapsed() < d {
                    if shared.cancel_requested.load(Ordering::SeqCst) {
                        shared.in_flight.store(false, Ordering::SeqCst);
                        return Err(map_runner_error("cancelled", "run cancelled"));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
            if shared.cancel_requested.load(Ordering::SeqCst) {
                shared.in_flight.store(false, Ordering::SeqCst);
                return Err(map_runner_error("cancelled", "run cancelled"));
            }
            shared.in_flight.store(false, Ordering::SeqCst);
            Ok(json!({ "text": "fake-ok" }))
        }
        "cancel" => {
            shared.cancel_requested.store(true, Ordering::SeqCst);
            if shared.cancel_hang.load(Ordering::SeqCst) {
                // Hang until force_kill flips the flag / clears hang.
                let start = Instant::now();
                while start.elapsed() < Duration::from_secs(30) {
                    if shared.force_killed.load(Ordering::SeqCst) {
                        shared.in_flight.store(false, Ordering::SeqCst);
                        return Ok(json!({}));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
            shared.in_flight.store(false, Ordering::SeqCst);
            Ok(json!({}))
        }
        "close" => {
            shared.dispose_awaited.store(true, Ordering::SeqCst);
            Ok(json!({}))
        }
        _ => Err(map_runner_error("runner", "unknown method")),
    }
}

// ── Production process client (pending-map demux) ────────────────────────────

/// Shared JSONL transport: short stdin write lock + `request_id` pending map +
/// dedicated stdout reader thread.
struct JsonlBridge {
    stdin: Mutex<Option<ChildStdin>>,
    pending: Mutex<HashMap<String, SyncSender<Result<Value, CursorError>>>>,
    dead: AtomicBool,
}

impl JsonlBridge {
    fn new(stdin: ChildStdin) -> Arc<Self> {
        Arc::new(Self {
            stdin: Mutex::new(Some(stdin)),
            pending: Mutex::new(HashMap::new()),
            dead: AtomicBool::new(false),
        })
    }

    fn start_reader(self: &Arc<Self>, stdout: ChildStdout) {
        let bridge = Arc::clone(self);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut response_line = String::new();
                match reader.read_line(&mut response_line) {
                    Ok(0) => {
                        bridge.fail_all();
                        break;
                    }
                    Ok(_) => {
                        if let Err(()) = bridge.dispatch_line(response_line.trim()) {
                            bridge.fail_all();
                            break;
                        }
                    }
                    Err(_) => {
                        bridge.fail_all();
                        break;
                    }
                }
            }
        });
    }

    fn fail_all(&self) {
        self.dead.store(true, Ordering::SeqCst);
        if let Ok(mut stdin) = self.stdin.lock() {
            *stdin = None;
        }
        let drained: Vec<_> = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain()
            .map(|(_, tx)| tx)
            .collect();
        let err = CursorError::new(
            CursorErrorCode::Runner,
            frontend_message_for(CursorErrorCode::Runner),
        );
        for tx in drained {
            let _ = tx.send(Err(err.clone()));
        }
    }

    fn dispatch_line(&self, trimmed: &str) -> Result<(), ()> {
        if trimmed.is_empty() {
            return Err(());
        }
        let v: Value = serde_json::from_str(trimmed).map_err(|_| ())?;
        let id = v
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or(())?
            .to_string();
        let tx = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        let Some(tx) = tx else {
            // Unknown id — ignore (do not tear down the bridge).
            return Ok(());
        };
        let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
        let result = if ok {
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
        };
        let _ = tx.send(result);
        Ok(())
    }

    fn exchange_with_id(
        &self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        if self.dead.load(Ordering::SeqCst) {
            return Err(CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            ));
        }
        let (tx, rx) = mpsc::sync_channel(1);
        {
            let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            if self.dead.load(Ordering::SeqCst) {
                return Err(CursorError::new(
                    CursorErrorCode::Runner,
                    frontend_message_for(CursorErrorCode::Runner),
                ));
            }
            if pending.contains_key(request_id) {
                return Err(CursorError::new(
                    CursorErrorCode::Runner,
                    frontend_message_for(CursorErrorCode::Runner),
                ));
            }
            pending.insert(request_id.to_string(), tx);
        }

        let mut req = json!({ "id": request_id, "method": method });
        if let Some(p) = params {
            req["params"] = p;
        }
        let line = serde_json::to_string(&req).map_err(|_| {
            self.abort_pending(request_id);
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;

        {
            let mut stdin_g = self.stdin.lock().unwrap_or_else(|e| e.into_inner());
            let stdin = stdin_g.as_mut().ok_or_else(|| {
                self.abort_pending(request_id);
                CursorError::new(
                    CursorErrorCode::Runner,
                    frontend_message_for(CursorErrorCode::Runner),
                )
            })?;
            if writeln!(stdin, "{line}").is_err() || stdin.flush().is_err() {
                self.abort_pending(request_id);
                self.fail_all();
                return Err(CursorError::new(
                    CursorErrorCode::Runner,
                    frontend_message_for(CursorErrorCode::Runner),
                ));
            }
        }

        match rx.recv() {
            Ok(r) => r,
            Err(_) => Err(CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )),
        }
    }

    fn abort_pending(&self, request_id: &str) {
        let _ = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(request_id);
    }
}

impl ConcurrentJsonl for JsonlBridge {
    fn submit(
        &self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        self.exchange_with_id(request_id, method, params)
    }
}

pub struct ProcessCursorRunnerClient {
    child: Option<Child>,
    pid: Arc<Mutex<Option<u32>>>,
    bridge: Arc<JsonlBridge>,
    next_id: u64,
}

impl ProcessCursorRunnerClient {
    /// Spawn Node runner with `CURSOR_API_KEY` in child env only.
    /// `pub(super)`: visible inside `services::agent` only (not the wider crate).
    /// Sole production caller: [`super::process_manager::CursorAgentProcessManager`].
    pub(super) fn spawn(repo_root: &Path, api_key: &str) -> Result<Self, CursorError> {
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
        let stdin = child.stdin.take().ok_or_else(|| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            CursorError::new(
                CursorErrorCode::Runner,
                frontend_message_for(CursorErrorCode::Runner),
            )
        })?;
        let bridge = JsonlBridge::new(stdin);
        bridge.start_reader(stdout);
        Ok(Self {
            child: Some(child),
            pid,
            bridge,
            next_id: 1,
        })
    }

    fn exchange(&mut self, method: &str, params: Option<Value>) -> Result<Value, CursorError> {
        let id = format!("r{}", self.next_id);
        self.next_id += 1;
        self.bridge.exchange_with_id(&id, method, params)
    }
}

impl CursorRunnerClient for ProcessCursorRunnerClient {
    fn close(&mut self) -> Result<(), CursorError> {
        let _ = self.exchange("close", None);
        self.bridge.fail_all();
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
        Ok(())
    }

    fn force_kill(&mut self) {
        self.bridge.fail_all();
        (self.terminate_hook())();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn terminate_hook(&self) -> Arc<dyn Fn() + Send + Sync> {
        let pid = self.pid.clone();
        let bridge = self.bridge.clone();
        Arc::new(move || {
            bridge.fail_all();
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
        self.bridge.exchange_with_id(request_id, method, params)
    }

    fn concurrent_jsonl(&self) -> Option<Arc<dyn ConcurrentJsonl>> {
        Some(self.bridge.clone() as Arc<dyn ConcurrentJsonl>)
    }
}

impl Drop for ProcessCursorRunnerClient {
    fn drop(&mut self) {
        self.force_kill();
    }
}

// ── CursorLlmEngine (Phase-Ensure) ───────────────────────────────────────────

/// Foreground SDK Agent binding: session id + process generation that created it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ForegroundBind {
    session_id: String,
    generation: u64,
}

/// Cursor engine communication path: `ensure_client` → `request(request_id, …)`.
/// Does not own/spawn `ProcessCursorRunnerClient` (manager does).
pub struct CursorLlmEngine {
    manager: Option<Arc<CursorAgentProcessManager>>,
    next_request_id: AtomicU64,
    foreground: Mutex<Option<ForegroundBind>>,
}

impl CursorLlmEngine {
    pub fn production() -> Self {
        Self {
            manager: None,
            next_request_id: AtomicU64::new(1),
            foreground: Mutex::new(None),
        }
    }

    pub fn with_manager(manager: CursorAgentProcessManager) -> Self {
        Self::with_shared_manager(Arc::new(manager))
    }

    pub fn with_shared_manager(manager: Arc<CursorAgentProcessManager>) -> Self {
        Self {
            manager: Some(manager),
            next_request_id: AtomicU64::new(1),
            foreground: Mutex::new(None),
        }
    }

    pub fn global() -> &'static CursorLlmEngine {
        static GLOBAL: OnceLock<CursorLlmEngine> = OnceLock::new();
        GLOBAL.get_or_init(Self::production)
    }

    fn manager(&self) -> &CursorAgentProcessManager {
        self.manager
            .as_deref()
            .unwrap_or_else(|| CursorAgentProcessManager::global())
    }

    fn next_request_id(&self) -> String {
        let n = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        format!("eng-{n}")
    }

    /// JSONL request via managed access; invalidate process on Runner/SdkRun (not Cancelled/Stale).
    fn request_managed(
        &self,
        access: &ClientAccess,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        match access.request(request_id, method, params) {
            Ok(v) => Ok(v),
            Err(err) => {
                let mapped = map_request_error(err);
                if matches!(
                    mapped.code,
                    CursorErrorCode::Runner | CursorErrorCode::SdkRun
                ) {
                    self.manager().invalidate();
                }
                Err(mapped)
            }
        }
    }

    /// ILlmEngine-shaped turn entry: ensure managed client, then JSONL request(s).
    pub fn run_turn(&self, req: &TurnRequest) -> Result<TurnOutcome, CursorError> {
        self.run_turn_inner(req, None)
    }

    pub(crate) fn run_turn_with_trace(
        &self,
        req: &TurnRequest,
        trace_id: &TraceId,
    ) -> Result<TurnOutcome, CursorError> {
        self.run_turn_inner(req, Some(trace_id))
    }

    fn run_turn_inner(
        &self,
        req: &TurnRequest,
        trace_id: Option<&TraceId>,
    ) -> Result<TurnOutcome, CursorError> {
        let run_started = Instant::now();
        let cwd_started = Instant::now();
        let cwd = match session_cwd::create_session_cwd(&req.session_id) {
            Ok(path) => {
                log_cursor_timing(trace_id, "cursor.session_cwd.completed", cwd_started, "ok");
                path
            }
            Err(_) => {
                log_cursor_timing(
                    trace_id,
                    "cursor.session_cwd.completed",
                    cwd_started,
                    "error",
                );
                return Err(CursorError::new(
                    CursorErrorCode::Cwd,
                    frontend_message_for(CursorErrorCode::Cwd),
                ));
            }
        };

        let mcp_servers = match serde_json::to_value(
            mcp_endpoint_readiness::map_to_sdk_mcp_servers(&req.ready_mcp),
        ) {
            Ok(value) => value,
            Err(_) => {
                return Err(CursorError::new(
                    CursorErrorCode::Runner,
                    frontend_message_for(CursorErrorCode::Runner),
                ));
            }
        };

        let ensure_started = Instant::now();
        let access = match self.manager().ensure_client() {
            Ok(access) => {
                log_cursor_timing(
                    trace_id,
                    "cursor.client_ensure.completed",
                    ensure_started,
                    "ok",
                );
                access
            }
            Err(err) => {
                log_cursor_timing(
                    trace_id,
                    "cursor.client_ensure.completed",
                    ensure_started,
                    "error",
                );
                return Err(map_ensure_error(err));
            }
        };

        let need_create = {
            let fg = self.foreground.lock().unwrap_or_else(|e| e.into_inner());
            match fg.as_ref() {
                Some(bind)
                    if bind.session_id == req.session_id
                        && bind.generation == access.generation() =>
                {
                    false
                }
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
            let create_started = Instant::now();
            let create_result = match self.request_managed(&access, &rid, "create", Some(params)) {
                Ok(result) => {
                    log_cursor_timing(
                        trace_id,
                        "cursor.create.completed",
                        create_started,
                        "ok",
                    );
                    result
                }
                Err(err) => {
                    log_cursor_timing(
                        trace_id,
                        "cursor.create.completed",
                        create_started,
                        "error",
                    );
                    return Err(err);
                }
            };
            // Defense: coalesced must never look like a successful create (bind/turn).
            if create_result
                .get("coalesced")
                .and_then(|v| v.as_bool())
                == Some(true)
            {
                return Err(CursorError::new(
                    CursorErrorCode::RecoverableFailure,
                    frontend_message_for(CursorErrorCode::RecoverableFailure),
                ));
            }
            // Bind foreground only after successful replace/create (session + process gen).
            {
                let mut fg = self.foreground.lock().unwrap_or_else(|e| e.into_inner());
                *fg = Some(ForegroundBind {
                    session_id: req.session_id.clone(),
                    generation: access.generation(),
                });
            }
            // Host cleans old session cwd only after runner confirms dispose via replaced_session_id.
            if let Some(replaced) = create_result
                .get("replaced_session_id")
                .and_then(|x| x.as_str())
            {
                if let Err(e) = session_cwd::cleanup_session_cwd(replaced) {
                    // Cleanup after successful replace is recoverable — never Session-turn persist.
                    let _ = e;
                    return Err(CursorError::new(
                        CursorErrorCode::RecoverableFailure,
                        frontend_message_for(CursorErrorCode::RecoverableFailure),
                    ));
                }
            }
        }

        let rid = self.next_request_id();
        let turn_started = Instant::now();
        let result = match self.request_managed(
            &access,
            &rid,
            "turn",
            Some(json!({ "prompt": req.prompt })),
        ) {
            Ok(result) => {
                log_cursor_timing(trace_id, "cursor.turn.completed", turn_started, "ok");
                result
            }
            Err(err) => {
                log_cursor_timing(trace_id, "cursor.turn.completed", turn_started, "error");
                return Err(err);
            }
        };
        let text = result
            .get("text")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        log_cursor_timing(trace_id, "cursor.run_turn.completed", run_started, "ok");
        Ok(TurnOutcome {
            text,
            should_persist: true,
        })
    }
}

fn log_cursor_timing(
    trace_id: Option<&TraceId>,
    event: &'static str,
    started: Instant,
    outcome: &'static str,
) {
    let Some(trace_id) = trace_id else {
        return;
    };
    let _ = diagnostics::log(
        DiagnosticEvent::timing("assistant.cursor", event, trace_id, started.elapsed())
            .with_static_field("outcome", outcome),
    );
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

// ── Session runtime (test doubles only; production uses CursorLlmEngine) ─────

#[cfg(test)]
const DEFAULT_CANCEL_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(test)]
type ClientFactory =
    Box<dyn FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send>;

#[cfg(test)]
struct SessionState {
    client: Arc<Mutex<Box<dyn CursorRunnerClient>>>,
    terminate: Arc<dyn Fn() + Send + Sync>,
    created: bool,
    in_flight: bool,
}

/// Legacy per-session runtime — compiled only for tests. Production path is
/// [`CursorLlmEngine`] + [`CursorAgentProcessManager`].
#[cfg(test)]
pub struct CursorSessionRuntime {
    sessions: Mutex<HashMap<String, SessionState>>,
    factory: Mutex<ClientFactory>,
    cancel_timeout: Mutex<Duration>,
    cancel_gen: AtomicU64,
}

#[cfg(test)]
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

        let result: Result<String, CursorError> = (|| {
            {
                let mut guard = client.lock().unwrap_or_else(|e| e.into_inner());
                if need_create {
                    // Carry opaque session_id (replace-create / cwd cleanup contract).
                    guard.request(
                        "sess-create",
                        "create",
                        Some(json!({
                            "session_id": req.session_id,
                            "model": req.model,
                            "cwd": cwd.to_string_lossy(),
                            "mcpServers": mcp_servers,
                        })),
                    )?;
                }
            }
            if need_create {
                let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(state) = sessions.get_mut(&req.session_id) {
                    state.created = true;
                }
            }
            let mut guard = client.lock().unwrap_or_else(|e| e.into_inner());
            let turn_result = guard.request(
                "sess-turn",
                "turn",
                Some(json!({ "prompt": req.prompt })),
            )?;
            Ok(turn_result
                .get("text")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string())
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
            let _ = guard.request("sess-cancel", "cancel", None);
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
