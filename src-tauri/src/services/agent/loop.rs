//! Agent Loop + Host open/chat-turn core (single-flight, terminals, history caps).

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use serde_json::{json, Value};

use crate::services::agent::llm::{self, AssistantMessage, LlmConfig, LlmError};
use crate::services::agent::session::{self, Session, Turn};
use crate::services::agent::tools;
use crate::services::agent::PLAN_ASSISTANT_SYSTEM_PROMPT;
use crate::services::todo_task;

pub use crate::services::agent::session::{
    validate_binding, Binding, BindingStateSummary, SetError,
};

pub const EVENT_TURN_COMPLETED: &str = "ai-assistant:turn-completed";
pub const WINDOW_LABEL: &str = "ai-assistant";
pub const MAX_TOOL_ROUNDS: u32 = 8;
pub const MAX_CLARIFY_ROUNDS: u32 = 5;
pub const MAX_HISTORY_MESSAGES: usize = 20;
pub const MAX_USER_TURNS: usize = 8;

const WRITE_TOOLS: &[&str] = &["add_sub_task", "update_sub_title", "update_master_title"];
const WHITELIST: &[&str] = &[
    "get_plan",
    "list_sub_tasks",
    "add_sub_task",
    "update_sub_title",
    "update_master_title",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    None,
    Business,
    Error,
}

impl Terminal {
    pub fn as_str(self) -> &'static str {
        match self {
            Terminal::None => "none",
            Terminal::Business => "business",
            Terminal::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnOutcome {
    pub reply_text: String,
    pub terminal: Terminal,
    pub wrote: bool,
}

#[derive(Debug, Clone)]
pub struct ChatTurnResult {
    pub body: Value,
    pub emit_turn_completed: Option<Value>,
}

/// Binding Contract lifecycle callback payload (E1): event name + optional result category only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleEvent {
    pub event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'static str>,
}

/// Outcome of a gated Binding Contract execute (tools/prompt from current Binding only).
#[derive(Debug, Clone)]
pub struct ExecuteOutcome {
    pub applied_tools: Value,
    pub applied_prompt: Value,
}

/// Stable execute / gate failure categories (L11-AR).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecError {
    code: &'static str,
}

impl ExecError {
    pub fn as_code(&self) -> &'static str {
        self.code
    }

    fn rejected_unbound() -> Self {
        Self {
            code: "rejected_unbound",
        }
    }

    fn reset_cancelled() -> Self {
        Self {
            code: "reset_cancelled",
        }
    }
}

type LifecycleListener = Box<dyn Fn(&LifecycleEvent) + Send + 'static>;

#[derive(Default)]
struct Runtime {
    busy: bool,
    current_session_id: Option<String>,
    bound_master_task_id: Option<String>,
    bound_title: Option<String>,
    /// Current generic Binding (tools+prompt+callbacks); None ⇒ unbound.
    current_binding: Option<session::Binding>,
    /// Live generation while bound; None when unbound.
    current_generation: Option<u64>,
    /// Monotonic counter for Set/replace generations.
    generation_seq: u64,
    /// True while a Binding Contract execute is in flight.
    executing: bool,
    /// Set by Reset (strategy A) while `executing` — cancels the in-flight execute.
    execute_cancelled: bool,
    clarify_counts: HashMap<String, u32>,
}

fn runtime() -> &'static Mutex<Runtime> {
    static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(Runtime::default()))
}

fn lifecycle_log() -> &'static Mutex<Vec<LifecycleEvent>> {
    static LOG: OnceLock<Mutex<Vec<LifecycleEvent>>> = OnceLock::new();
    LOG.get_or_init(|| Mutex::new(Vec::new()))
}

fn lifecycle_listener() -> &'static Mutex<Option<LifecycleListener>> {
    static LISTENER: OnceLock<Mutex<Option<LifecycleListener>>> = OnceLock::new();
    LISTENER.get_or_init(|| Mutex::new(None))
}

pub fn reset_runtime_for_tests() {
    let mut rt = runtime().lock().unwrap();
    *rt = Runtime::default();
    drop(rt);
    lifecycle_log().lock().unwrap().clear();
    *lifecycle_listener().lock().unwrap() = None;
}

pub fn set_busy_for_tests(busy: bool) {
    runtime().lock().unwrap().busy = busy;
}

pub fn clear_lifecycle_events_for_tests() {
    lifecycle_log().lock().unwrap().clear();
}

pub fn drain_lifecycle_events() -> Vec<LifecycleEvent> {
    std::mem::take(&mut *lifecycle_log().lock().unwrap())
}

pub fn set_lifecycle_listener_for_tests(listener: Option<LifecycleListener>) {
    *lifecycle_listener().lock().unwrap() = listener;
}

fn emit_lifecycle(event: &'static str, category: Option<&'static str>) {
    emit_lifecycle_inner(event, category, true);
}

fn emit_lifecycle_inner(event: &'static str, category: Option<&'static str>, invoke_listener: bool) {
    let ev = LifecycleEvent { event, category };
    lifecycle_log().lock().unwrap().push(ev.clone());
    if !invoke_listener {
        return;
    }
    let listener = {
        let mut slot = lifecycle_listener().lock().unwrap();
        slot.take()
    };
    let Some(listener) = listener else {
        return;
    };
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| listener(&ev)));
    *lifecycle_listener().lock().unwrap() = Some(listener);
    if result.is_err() {
        // Record callback_failed without re-entering the panicking listener.
        emit_lifecycle_inner("onError", Some("callback_failed"), false);
    }
}

/// Set Binding after B1 validation. Failure returns `set_invalid` and leaves state unchanged.
/// Legal Set on bound atomically replaces and invalidates the previous generation.
/// Emits onBound; replace also emits onUnbound → onBound (D1).
pub fn set_binding(binding: session::Binding) -> Result<(), SetError> {
    session::validate_binding(&binding)?;
    let was_bound = {
        let mut rt = runtime().lock().unwrap();
        let was = rt.current_binding.is_some();
        rt.generation_seq = rt.generation_seq.saturating_add(1);
        rt.current_generation = Some(rt.generation_seq);
        rt.current_binding = Some(binding);
        was
    };
    if was_bound {
        emit_lifecycle("onUnbound", None);
    }
    emit_lifecycle("onBound", None);
    Ok(())
}

/// JSON Set entry: require tools/prompt/callbacks keys present; never fill from business fields.
pub fn try_set_binding_json(v: &Value) -> Result<(), SetError> {
    set_binding(session::binding_from_json(v)?)
}

/// Reset: discard current Binding → unbound. Idempotent when already unbound.
/// Bound→unbound emits onUnbound; mid-execute Reset (strategy A) cancels in-flight execute.
pub fn reset_binding() -> Result<(), ()> {
    let was_bound = {
        let mut rt = runtime().lock().unwrap();
        let was = rt.current_binding.is_some();
        rt.current_binding = None;
        rt.current_generation = None;
        if rt.executing {
            rt.execute_cancelled = true;
        }
        was
    };
    if was_bound {
        emit_lifecycle("onUnbound", None);
    }
    Ok(())
}

/// Binding Contract execute: only when bound; applies current Binding tools/prompt as sole config.
pub fn execute_binding() -> Result<ExecuteOutcome, ExecError> {
    execute_binding_during(|| {})
}

/// Like `execute_binding`, but invokes `mid` while the execute is marked in-flight
/// (for Strategy A mid-execute Reset observation).
pub fn execute_binding_during<F: FnOnce()>(mid: F) -> Result<ExecuteOutcome, ExecError> {
    let snapshot = {
        let mut rt = runtime().lock().unwrap();
        let Some(binding) = rt.current_binding.clone() else {
            drop(rt);
            emit_lifecycle("onError", Some("rejected_unbound"));
            return Err(ExecError::rejected_unbound());
        };
        rt.executing = true;
        rt.execute_cancelled = false;
        binding
    };

    mid();

    let cancelled = {
        let mut rt = runtime().lock().unwrap();
        let cancelled = rt.execute_cancelled;
        rt.executing = false;
        cancelled
    };

    if cancelled {
        emit_lifecycle("onError", Some("reset_cancelled"));
        return Err(ExecError::reset_cancelled());
    }

    Ok(ExecuteOutcome {
        applied_tools: snapshot.tools,
        applied_prompt: snapshot.prompt,
    })
}

/// Read-only query: state∈{unbound,bound}; bound includes generation only (no tools/prompt/business IDs).
pub fn query_binding() -> BindingStateSummary {
    query_binding_snapshot().0
}

/// Atomic snapshot of query summary + current Binding slot count (0 or 1).
pub fn query_binding_snapshot() -> (BindingStateSummary, usize) {
    let rt = runtime().lock().unwrap();
    match rt.current_binding.as_ref() {
        Some(_) => (
            BindingStateSummary {
                state: "bound",
                generation: rt.current_generation,
            },
            1,
        ),
        None => (
            BindingStateSummary {
                state: "unbound",
                generation: None,
            },
            0,
        ),
    }
}

/// Current binding gate: `unbound` | `bound` (generic Binding present).
pub fn binding_state() -> &'static str {
    query_binding().state
}

/// How many current Binding slots are held (0 or 1). Used to prove single-binding serialization.
pub fn current_binding_slot_count() -> usize {
    query_binding_snapshot().1
}

/// Whether `generation` is still the live current Binding (false after Reset or replace).
pub fn is_binding_generation_current(generation: u64) -> bool {
    runtime().lock().unwrap().current_generation == Some(generation)
}

/// Binding Contract ops only (L03-SC / L08-AR). Present/Open are shell surface, not ops.
pub const BINDING_CONTRACT_OPS: &[&str] = &["set", "reset", "query", "execute", "callbacks"];

/// Stated Binding Contract ops list (excludes Present / Open).
pub fn binding_contract_ops() -> &'static [&'static str] {
    BINDING_CONTRACT_OPS
}

/// Outcome of Host Present (shell open/focus). Not a Binding Contract result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentOutcome {
    pub surface: &'static str,
    pub window_label: &'static str,
}

/// Present: open/focus assistant shell. Does not Set; does not change binding state.
/// Maps to existing `create_or_focus_ai_assistant_window` semantics (window label).
pub fn present_ai_assistant_core() -> Result<PresentOutcome, String> {
    Ok(PresentOutcome {
        surface: "Present",
        window_label: WINDOW_LABEL,
    })
}

/// Shell close (关壳). Not Reset — leaves Binding Contract state unchanged; no onUnbound.
pub fn shell_close_core() -> Result<(), String> {
    Ok(())
}

pub fn clarify_count(session: &Session) -> u32 {
    runtime()
        .lock()
        .unwrap()
        .clarify_counts
        .get(&session.session_id)
        .copied()
        .unwrap_or(0)
}

fn is_clarify_text(content: &str) -> bool {
    let t = content.trim();
    t.contains('？') || t.contains('?')
}

fn turn_to_message(turn: &Turn) -> Value {
    let mut m = json!({ "role": turn.role });
    if let Some(c) = &turn.content {
        m["content"] = json!(c);
    } else if turn.tool_calls.is_none() {
        m["content"] = json!("");
    }
    if let Some(id) = &turn.tool_call_id {
        m["tool_call_id"] = json!(id);
    }
    if let Some(name) = &turn.name {
        m["name"] = json!(name);
    }
    if let Some(tc) = &turn.tool_calls {
        m["tool_calls"] = tc.clone();
    }
    m
}

/// Drop oldest complete user rounds until dual hard caps hold.
pub fn truncate_turns(turns: &[Turn]) -> Vec<Turn> {
    let mut kept = turns.to_vec();
    loop {
        let user_count = kept.iter().filter(|t| t.role == "user").count();
        if kept.len() <= MAX_HISTORY_MESSAGES && user_count <= MAX_USER_TURNS {
            break;
        }
        // Find first user index; drop through next user (exclusive) or end.
        let Some(start) = kept.iter().position(|t| t.role == "user") else {
            break;
        };
        let end = kept[start + 1..]
            .iter()
            .position(|t| t.role == "user")
            .map(|i| start + 1 + i)
            .unwrap_or(kept.len());
        if end == start {
            break;
        }
        kept.drain(start..end);
    }
    kept
}

pub fn build_llm_messages_from_turns(turns: &[Turn]) -> Vec<Value> {
    let mut messages = vec![json!({
        "role": "system",
        "content": PLAN_ASSISTANT_SYSTEM_PROMPT,
    })];
    for turn in truncate_turns(turns) {
        messages.push(turn_to_message(&turn));
    }
    messages
}

fn map_llm_error(err: &LlmError) -> TurnOutcome {
    let reply_text = match err {
        LlmError::MissingConfig => {
            "LLM 配置不完整，请到应用「设置」中填写 api_key、base_url 与 model。".into()
        }
        LlmError::Unauthorized => "鉴权失败，请检查 API Key 配置。".into(),
        LlmError::BadRequest(_) => "请求不被上游接受，请稍后重试或检查配置。".into(),
        LlmError::RateLimited => "请求过于频繁，请稍后重试。".into(),
        LlmError::Server(_) | LlmError::Network(_) => {
            "上游服务暂时不可用，请稍后重试。".into()
        }
        LlmError::Timeout => "调用超时，请稍后重试。".into(),
        LlmError::Truncated => "回复被截断，请重试或缩短请求。".into(),
        LlmError::UnsupportedToolCalls => {
            "当前上游不支持工具调用（tool_calls），无法继续。".into()
        }
        LlmError::InvalidResponse(_) => "模型响应格式异常，未执行任何写入。".into(),
    };
    TurnOutcome {
        reply_text,
        terminal: Terminal::Error,
        wrote: false,
    }
}

fn tool_calls_json(msg: &AssistantMessage) -> Value {
    let arr: Vec<Value> = msg
        .tool_calls
        .iter()
        .map(|tc| {
            json!({
                "id": tc.id,
                "type": "function",
                "function": {
                    "name": tc.name,
                    "arguments": tc.arguments,
                }
            })
        })
        .collect();
    Value::Array(arr)
}

fn persist(session: &Session) {
    let _ = session::save_session(session);
}

pub fn run_loop(session: &mut Session, user_message: &str, config: &LlmConfig) -> TurnOutcome {
    // Unbound → business, no LLM.
    let bound = session
        .bound_master_task_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let bound_ok = match bound {
        Some(id) => {
            let got = todo_task::get_by_id(id);
            got.get("master_task_id").and_then(|v| v.as_str()).is_some()
                && !got
                    .get("_status")
                    .and_then(|s| s.as_u64())
                    .is_some_and(|s| s >= 400)
        }
        None => false,
    };
    if !bound_ok {
        session.turns.push(Turn {
            role: "user".into(),
            content: Some(user_message.to_string()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        let reply = "当前没有可操作的计划（无计划），请从计划页打开助手并绑定计划。".to_string();
        session.turns.push(Turn {
            role: "assistant".into(),
            content: Some(reply.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        persist(session);
        return TurnOutcome {
            reply_text: reply,
            terminal: Terminal::Business,
            wrote: false,
        };
    }

    session.turns.push(Turn {
        role: "user".into(),
        content: Some(user_message.to_string()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    });
    persist(session);

    let mut wrote = false;
    let mut tool_rounds: u32 = 0;
    let tools_defs = tools::openai_tool_definitions();

    loop {
        let messages = build_llm_messages_from_turns(&session.turns);
        let msg = match llm::chat_completions(&messages, &tools_defs, config) {
            Ok(m) => m,
            Err(e) => {
                let out = map_llm_error(&e);
                session.turns.push(Turn {
                    role: "assistant".into(),
                    content: Some(out.reply_text.clone()),
                    tool_call_id: None,
                    tool_calls: None,
                    name: None,
                });
                persist(session);
                return out;
            }
        };

        if !msg.tool_calls.is_empty() {
            // Unknown / malformed tool names → error, no write.
            for tc in &msg.tool_calls {
                if tc.id.trim().is_empty() || !WHITELIST.contains(&tc.name.as_str()) {
                    let reply = "模型响应异常或请求了不支持的工具，未执行任何写入。".to_string();
                    session.turns.push(Turn {
                        role: "assistant".into(),
                        content: Some(reply.clone()),
                        tool_call_id: None,
                        tool_calls: Some(tool_calls_json(&msg)),
                        name: None,
                    });
                    persist(session);
                    return TurnOutcome {
                        reply_text: reply,
                        terminal: Terminal::Error,
                        wrote: false,
                    };
                }
            }

            if tool_rounds >= MAX_TOOL_ROUNDS {
                let reply = "工具调用次数已达上限，已停止本回合。".to_string();
                session.turns.push(Turn {
                    role: "assistant".into(),
                    content: Some(reply.clone()),
                    tool_call_id: None,
                    tool_calls: None,
                    name: None,
                });
                persist(session);
                return TurnOutcome {
                    reply_text: reply,
                    terminal: Terminal::Error,
                    wrote,
                };
            }
            tool_rounds += 1;

            // Same-message content is NOT final; store assistant tool_calls turn (content kept in history but ignored as UI final).
            session.turns.push(Turn {
                role: "assistant".into(),
                content: msg.content.clone(),
                tool_call_id: None,
                tool_calls: Some(tool_calls_json(&msg)),
                name: None,
            });

            for tc in &msg.tool_calls {
                let args: Value = serde_json::from_str(&tc.arguments).unwrap_or(json!({}));
                let result = tools::dispatch(
                    &tc.name,
                    &args,
                    session.bound_master_task_id.as_deref(),
                );
                if WRITE_TOOLS.contains(&tc.name.as_str()) && result["ok"] == true {
                    wrote = true;
                }
                session.turns.push(Turn {
                    role: "tool".into(),
                    content: Some(result.to_string()),
                    tool_call_id: Some(tc.id.clone()),
                    tool_calls: None,
                    name: Some(tc.name.clone()),
                });
            }
            persist(session);
            continue;
        }

        // Content-only: clarify or final.
        let content = msg
            .content
            .clone()
            .unwrap_or_else(|| "".to_string());
        if content.trim().is_empty() {
            let reply = "模型响应为空，未执行任何写入。".to_string();
            session.turns.push(Turn {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            persist(session);
            return TurnOutcome {
                reply_text: reply,
                terminal: Terminal::Error,
                wrote,
            };
        }

        if is_clarify_text(&content) {
            let mut rt = runtime().lock().unwrap();
            let count = rt
                .clarify_counts
                .entry(session.session_id.clone())
                .or_insert(0);
            if *count >= MAX_CLARIFY_ROUNDS {
                drop(rt);
                let reply = "澄清次数已达上限，请换种方式说明需求或稍后重试。".to_string();
                session.turns.push(Turn {
                    role: "assistant".into(),
                    content: Some(reply.clone()),
                    tool_call_id: None,
                    tool_calls: None,
                    name: None,
                });
                persist(session);
                return TurnOutcome {
                    reply_text: reply,
                    terminal: Terminal::Error,
                    wrote,
                };
            }
            *count += 1;
            drop(rt);
            session.turns.push(Turn {
                role: "assistant".into(),
                content: Some(content.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            persist(session);
            return TurnOutcome {
                reply_text: content,
                terminal: Terminal::None,
                wrote,
            };
        }

        // Business cue from model text
        if content.contains("目前不支持") {
            session.turns.push(Turn {
                role: "assistant".into(),
                content: Some(content.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            persist(session);
            return TurnOutcome {
                reply_text: content,
                terminal: Terminal::Business,
                wrote,
            };
        }

        session.turns.push(Turn {
            role: "assistant".into(),
            content: Some(content.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        persist(session);
        return TurnOutcome {
            reply_text: content,
            terminal: Terminal::None,
            wrote,
        };
    }
}

fn plan_title(master_task_id: &str) -> Option<String> {
    let got = todo_task::get_by_id(master_task_id);
    if got
        .get("_status")
        .and_then(|s| s.as_u64())
        .is_some_and(|s| s >= 400)
    {
        return None;
    }
    got.get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Current open binding for the assistant window (may be empty if never opened).
pub fn get_ai_assistant_binding_core() -> Value {
    let rt = runtime().lock().unwrap();
    json!({
        "session_id": rt.current_session_id.clone().unwrap_or_default(),
        "bound_master_task_id": rt.bound_master_task_id.clone().unwrap_or_default(),
        "bound_title": rt.bound_title.clone().unwrap_or_default(),
        "window_label": WINDOW_LABEL,
        "busy": rt.busy,
    })
}

pub fn open_ai_assistant_core(master_task_id: &str) -> Result<Value, String> {
    let id = master_task_id.trim();
    if id.is_empty() {
        return Err("Missing master_task_id".into());
    }
    let title = plan_title(id).ok_or_else(|| "Plan not found".to_string())?;

    let mut rt = runtime().lock().unwrap();
    if rt.busy {
        return Ok(json!({
            "session_id": rt.current_session_id,
            "bound_master_task_id": rt.bound_master_task_id,
            "bound_title": rt.bound_title,
            "window_label": WINDOW_LABEL,
            "busy": true,
            "reply_text": "处理中，暂不可切换绑定计划。",
        }));
    }

    let sess = session::create_session(Some(id), Some(&title))?;
    rt.current_session_id = Some(sess.session_id.clone());
    rt.bound_master_task_id = Some(id.to_string());
    rt.bound_title = Some(title.clone());
    rt.clarify_counts.insert(sess.session_id.clone(), 0);

    Ok(json!({
        "session_id": sess.session_id,
        "bound_master_task_id": id,
        "bound_title": title,
        "window_label": WINDOW_LABEL,
        "busy": false,
    }))
}

fn emit_payload(
    session_id: &str,
    bound_master_task_id: Option<&str>,
    bound_title: Option<&str>,
    wrote: bool,
    terminal: &str,
) -> Value {
    json!({
        "event": EVENT_TURN_COMPLETED,
        "payload": {
            "session_id": session_id,
            "bound_master_task_id": bound_master_task_id,
            "bound_title": bound_title,
            "wrote": wrote,
            "terminal": terminal,
        }
    })
}

pub fn agent_chat_turn_core(
    session_id: &str,
    message: &str,
    master_task_id: Option<&str>,
) -> Result<ChatTurnResult, String> {
    {
        let rt = runtime().lock().unwrap();
        if rt.busy {
            return Ok(ChatTurnResult {
                body: json!({
                    "reply_text": "处理中，请稍候。",
                    "terminal": "none",
                    "wrote": false,
                    "bound_master_task_id": rt.bound_master_task_id,
                    "bound_title": rt.bound_title,
                    "busy": true,
                    "session_id": session_id,
                }),
                emit_turn_completed: None,
            });
        }
    }

    let mut session = session::load_session(session_id)?;
    let bound_id = session.bound_master_task_id.clone();
    let bound_title = session.bound_title.clone();

    if let Some(req) = master_task_id.map(str::trim).filter(|s| !s.is_empty()) {
        if bound_id.as_deref() != Some(req) {
            let reply = "请求的计划与当前窗口绑定不一致。".to_string();
            let body = json!({
                "reply_text": reply,
                "terminal": "business",
                "wrote": false,
                "bound_master_task_id": bound_id,
                "bound_title": bound_title,
                "busy": false,
                "session_id": session_id,
            });
            let emit = emit_payload(
                session_id,
                bound_id.as_deref(),
                bound_title.as_deref(),
                false,
                "business",
            );
            return Ok(ChatTurnResult {
                body,
                emit_turn_completed: Some(emit),
            });
        }
    }

    {
        let mut rt = runtime().lock().unwrap();
        rt.busy = true;
        rt.current_session_id = Some(session_id.to_string());
        rt.bound_master_task_id = bound_id.clone();
        rt.bound_title = bound_title.clone();
    }

    let outcome = match llm::load_llm_config() {
        Ok(cfg) => run_loop(&mut session, message, &cfg),
        Err(e) => {
            let out = map_llm_error(&e);
            // Still record user message path via run_loop? load failed before — append briefly.
            session.turns.push(Turn {
                role: "user".into(),
                content: Some(message.to_string()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            session.turns.push(Turn {
                role: "assistant".into(),
                content: Some(out.reply_text.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            persist(&session);
            out
        }
    };

    {
        let mut rt = runtime().lock().unwrap();
        rt.busy = false;
    }

    // Refresh title if wrote
    let bound_title = session.bound_title.clone().or_else(|| {
        session
            .bound_master_task_id
            .as_deref()
            .and_then(plan_title)
    });
    if let Some(ref t) = bound_title {
        if session.bound_title.as_deref() != Some(t.as_str()) {
            session.bound_title = Some(t.clone());
            persist(&session);
        }
    }
    if outcome.wrote {
        if let Some(id) = session.bound_master_task_id.as_deref() {
            if let Some(t) = plan_title(id) {
                session.bound_title = Some(t.clone());
                persist(&session);
                let mut rt = runtime().lock().unwrap();
                rt.bound_title = Some(t);
            }
        }
    }

    let terminal = outcome.terminal.as_str();
    let body = json!({
        "reply_text": outcome.reply_text,
        "terminal": terminal,
        "wrote": outcome.wrote,
        "bound_master_task_id": session.bound_master_task_id,
        "bound_title": session.bound_title,
        "busy": false,
        "session_id": session_id,
    });
    let emit = emit_payload(
        session_id,
        session.bound_master_task_id.as_deref(),
        session.bound_title.as_deref(),
        outcome.wrote,
        terminal,
    );
    Ok(ChatTurnResult {
        body,
        emit_turn_completed: Some(emit),
    })
}
