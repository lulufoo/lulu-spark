//! Engine-aware chat turn orchestration (busy / gate / persist / ChatTurnResult).
//!
//! Sibling to Host `loop` and Cursor `cursor_adapter`. Formal chat enters here;
//! settings select Host (`run_loop`) or Cursor (adapter + Host MCP readiness via
//! `GET /health` on `DEFAULT_MCP_PORT`). Cursor failures never fall back to Host
//! LLM or process-local business tool handlers.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Value};

use crate::config::settings;
use crate::services::agent::cursor_adapter::{
    self, CursorError, CursorErrorCode, CursorLlmEngine, CursorSessionRuntime,
    TurnOutcome as CursorTurnOutcome, TurnRequest,
};
use crate::services::agent::engine_router::{self, AdapterKind, EngineRouteError, TurnInput};
use crate::services::agent::llm;
use crate::services::agent::r#loop::{self, ChatTurnResult, Terminal, TurnOutcome};
use crate::services::agent::session::{self, Turn};
use crate::services::local_http::DEFAULT_HTTP_PORT;
use crate::services::mcp_endpoint_readiness::{self, ReadyMcpTransports};
use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
use crate::DEFAULT_MCP_PORT;

fn cursor_code_str(code: CursorErrorCode) -> &'static str {
    match code {
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

fn cursor_code_from_str(code: &str) -> CursorErrorCode {
    match code {
        "credential" => CursorErrorCode::Credential,
        "sdk_config" => CursorErrorCode::SdkConfig,
        "mcp_unavailable" => CursorErrorCode::McpUnavailable,
        "cwd" => CursorErrorCode::Cwd,
        "runner" => CursorErrorCode::Runner,
        "sdk_run" => CursorErrorCode::SdkRun,
        "cancelled" => CursorErrorCode::Cancelled,
        "busy" => CursorErrorCode::Busy,
        "recoverable_failure" => CursorErrorCode::RecoverableFailure,
        _ => CursorErrorCode::Runner,
    }
}

fn cursor_err_routed(err: CursorError) -> RoutedTurn {
    RoutedTurn {
        reply_text: err.message.clone(),
        terminal: Terminal::Error,
        wrote: false,
        cursor_error: Some(err),
    }
}

static TEST_CURSOR_RT: OnceLock<Mutex<Option<Arc<CursorSessionRuntime>>>> = OnceLock::new();
static TEST_READY_MCP: OnceLock<Mutex<Option<ReadyMcpTransports>>> = OnceLock::new();
static TEST_CURSOR_INSTALLED: AtomicBool = AtomicBool::new(false);

fn test_cursor_slot() -> &'static Mutex<Option<Arc<CursorSessionRuntime>>> {
    TEST_CURSOR_RT.get_or_init(|| Mutex::new(None))
}

fn test_ready_slot() -> &'static Mutex<Option<ReadyMcpTransports>> {
    TEST_READY_MCP.get_or_init(|| Mutex::new(None))
}

pub fn reset_for_tests() {
    *test_cursor_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
    *test_ready_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
    TEST_CURSOR_INSTALLED.store(false, Ordering::SeqCst);
}

pub fn set_cursor_runtime_for_tests(rt: Option<Arc<CursorSessionRuntime>>) {
    TEST_CURSOR_INSTALLED.store(rt.is_some(), Ordering::SeqCst);
    *test_cursor_slot().lock().unwrap_or_else(|e| e.into_inner()) = rt;
}

pub fn set_ready_mcp_for_tests(ready: Option<ReadyMcpTransports>) {
    *test_ready_slot().lock().unwrap_or_else(|e| e.into_inner()) = ready;
}

pub fn cursor_runtime_installed_for_tests() -> bool {
    TEST_CURSOR_INSTALLED.load(Ordering::SeqCst)
}

/// Production Cursor path: CursorLlmEngine (ensure_client → request). Test doubles
/// may still inject [`CursorSessionRuntime`] via `set_cursor_runtime_for_tests`.
fn cursor_run_turn(req: &TurnRequest) -> Result<CursorTurnOutcome, CursorError> {
    if let Some(rt) = test_cursor_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        return rt.run_turn(req);
    }
    CursorLlmEngine::global().run_turn(req)
}

/// Probe Host MCP readiness (`GET http://127.0.0.1:{DEFAULT_MCP_PORT}/health`)
/// and return Binding transports (`/mcp/<slot>`). Health contract only.
fn ready_mcp_for_binding_key(key: &str) -> Result<ReadyMcpTransports, CursorError> {
    if let Some(ready) = test_ready_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        return Ok(ready);
    }
    mcp_endpoint_readiness::ready_transports_for_business_key(
        key,
        DEFAULT_HTTP_PORT,
        DEFAULT_MCP_PORT,
    )
    .map_err(|_| {
        CursorError::new(
            CursorErrorCode::McpUnavailable,
            cursor_adapter::frontend_message_for(CursorErrorCode::McpUnavailable),
        )
    })
}

fn append_user_assistant(session: &mut session::Session, user: &str, assistant: &str) {
    session.turns.push(Turn {
        role: "user".into(),
        content: Some(user.to_string()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    });
    session.turns.push(Turn {
        role: "assistant".into(),
        content: Some(assistant.to_string()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    });
    r#loop::persist(session);
}

fn pack_outcome(session_id: &str, outcome: TurnOutcome) -> ChatTurnResult {
    let terminal = outcome.terminal.as_str();
    let body = json!({
        "reply_text": outcome.reply_text,
        "terminal": terminal,
        "wrote": outcome.wrote,
        "busy": false,
        "session_id": session_id,
    });
    let emit = r#loop::turn_completed_emit(session_id, outcome.wrote, terminal);
    ChatTurnResult {
        body,
        emit_turn_completed: Some(emit),
    }
}

fn pack_cursor_error(session_id: &str, err: &CursorError) -> ChatTurnResult {
    let body = json!({
        "reply_text": err.message,
        "terminal": Terminal::Error.as_str(),
        "wrote": false,
        "busy": false,
        "session_id": session_id,
        "code": format!("cursor_{}", cursor_code_str(err.code)),
    });
    let emit = r#loop::turn_completed_emit(session_id, false, Terminal::Error.as_str());
    ChatTurnResult {
        body,
        emit_turn_completed: Some(emit),
    }
}

#[derive(Debug, Clone)]
struct RoutedTurn {
    reply_text: String,
    terminal: Terminal,
    wrote: bool,
    cursor_error: Option<CursorError>,
}

fn encode_routed(r: &RoutedTurn) -> String {
    // Closures for engine_router return String; pack fields as JSON.
    json!({
        "reply_text": r.reply_text,
        "terminal": r.terminal.as_str(),
        "wrote": r.wrote,
        "cursor_error_code": r.cursor_error.as_ref().map(|e| cursor_code_str(e.code)),
        "cursor_error_message": r.cursor_error.as_ref().map(|e| e.message.clone()),
    })
    .to_string()
}

fn decode_routed(s: &str) -> Result<RoutedTurn, String> {
    let v: Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
    let reply_text = v
        .get("reply_text")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let terminal = match v.get("terminal").and_then(|x| x.as_str()).unwrap_or("none") {
        "business" => Terminal::Business,
        "error" => Terminal::Error,
        _ => Terminal::None,
    };
    let wrote = v.get("wrote").and_then(|x| x.as_bool()).unwrap_or(false);
    let cursor_error = match (
        v.get("cursor_error_code").and_then(|x| x.as_str()),
        v.get("cursor_error_message").and_then(|x| x.as_str()),
    ) {
        (Some(code), Some(msg)) => Some(CursorError::new(cursor_code_from_str(code), msg)),
        _ => None,
    };
    Ok(RoutedTurn {
        reply_text,
        terminal,
        wrote,
        cursor_error,
    })
}

fn host_adapter_turn(session_id: &str, message: &str) -> Result<String, String> {
    let mut session = session::load_session(session_id)?;
    let outcome = match llm::load_llm_config() {
        Ok(cfg) => r#loop::run_loop(&mut session, message, &cfg),
        Err(e) => {
            let out = r#loop::map_llm_error(&e);
            append_user_assistant(&mut session, message, &out.reply_text);
            out
        }
    };
    Ok(encode_routed(&RoutedTurn {
        reply_text: outcome.reply_text,
        terminal: outcome.terminal,
        wrote: outcome.wrote,
        cursor_error: None,
    }))
}

fn cursor_adapter_turn(session_id: &str, message: &str) -> Result<String, String> {
    // L2-A: consume AIAssistantSession execution-context snapshot (sole live owner).
    let live_ctx = session::live_context_owner().execution_context_snapshot();
    // Same-session MCP config face (L2) — required before readiness/adapter.
    if live_ctx.loaded_mcp_server.is_none() {
        let err = CursorError::new(
            CursorErrorCode::McpUnavailable,
            cursor_adapter::frontend_message_for(CursorErrorCode::McpUnavailable),
        );
        return Ok(encode_routed(&cursor_err_routed(err)));
    }

    let settings = settings::load().map_err(|e| e.to_string())?;
    let cfg = engine_router::read_engine_runtime_config(&settings).map_err(|e| e.to_string())?;
    let Some(api_key) = cfg.credential.filter(|s| !s.is_empty()) else {
        let err = CursorError::new(
            CursorErrorCode::Credential,
            cursor_adapter::frontend_message_for(CursorErrorCode::Credential),
        );
        return Ok(encode_routed(&cursor_err_routed(err)));
    };

    let key = live_ctx
        .binding
        .as_ref()
        .and_then(session::binding_business_key)
        .unwrap_or_else(|| SEEDED_BUSINESS_KEY.to_string());

    let ready_mcp = match ready_mcp_for_binding_key(&key) {
        Ok(r) => r,
        Err(err) => return Ok(encode_routed(&cursor_err_routed(err))),
    };

    let req = TurnRequest {
        session_id: session_id.to_string(),
        prompt: message.to_string(),
        model: cfg.model,
        api_key,
        ready_mcp,
    };

    let result = cursor_run_turn(&req);
    let should_persist = cursor_adapter::should_persist_cursor_result(&result);

    match result {
        Ok(out) => {
            if should_persist {
                let mut session = session::load_session(session_id)?;
                append_user_assistant(&mut session, message, &out.text);
            }
            Ok(encode_routed(&RoutedTurn {
                reply_text: out.text,
                terminal: Terminal::None,
                wrote: false,
                cursor_error: None,
            }))
        }
        Err(err) => {
            if should_persist {
                let mut session = session::load_session(session_id)?;
                append_user_assistant(&mut session, message, &err.message);
            }
            Ok(encode_routed(&cursor_err_routed(err)))
        }
    }
}

/// Facade-facing chat turn: engine-opaque signature; routes Host/Cursor by settings.
pub fn chat_turn(
    session_id: &str,
    message: &str,
    _master_task_id: Option<&str>,
) -> Result<ChatTurnResult, String> {
    if let Err(early) = r#loop::try_begin_chat_turn(session_id) {
        return Ok(early);
    }

    let mut session = session::load_session(session_id)?;

    // Gate Binding before any engine/config — unbound must not depend on secrets.
    if !r#loop::has_active_binding() {
        let reply = "Unbound — no active Binding Contract; chat cannot run.".to_string();
        append_user_assistant(&mut session, message, &reply);
        r#loop::end_chat_turn_busy();
        return Ok(pack_outcome(
            session_id,
            TurnOutcome {
                reply_text: reply,
                terminal: Terminal::Business,
                wrote: false,
            },
        ));
    }

    let settings = match settings::load() {
        Ok(s) => s,
        Err(e) => {
            r#loop::end_chat_turn_busy();
            return Err(e.to_string());
        }
    };

    let input = TurnInput {
        session_id: session_id.to_string(),
        message: message.to_string(),
    };

    let sid = session_id.to_string();
    let msg = message.to_string();
    let routed = engine_router::dispatch_from_settings(
        &settings,
        &input,
        || host_adapter_turn(&sid, &msg),
        || cursor_adapter_turn(&sid, &msg),
    );

    r#loop::end_chat_turn_busy();

    match routed {
        Ok(route) => {
            let decoded = decode_routed(&route.body)?;
            if let Some(err) = decoded.cursor_error {
                // Typed Cursor surface — never Host generic upstream collapse.
                if matches!(route.adapter, AdapterKind::Cursor) {
                    return Ok(pack_cursor_error(session_id, &err));
                }
            }
            Ok(pack_outcome(
                session_id,
                TurnOutcome {
                    reply_text: decoded.reply_text,
                    terminal: decoded.terminal,
                    wrote: decoded.wrote,
                },
            ))
        }
        Err(EngineRouteError::InvalidEngine(v)) => Ok(pack_outcome(
            session_id,
            TurnOutcome {
                reply_text: format!("invalid assistant_engine value: {v}"),
                terminal: Terminal::Error,
                wrote: false,
            },
        )),
        Err(EngineRouteError::Adapter(msg)) => {
            // Adapter string errors on Cursor must not become Host upstream text.
            let err = CursorError::new(CursorErrorCode::Runner, msg);
            if engine_router::resolve_engine(&settings).ok()
                == Some(engine_router::EngineKind::Cursor)
            {
                Ok(pack_cursor_error(session_id, &err))
            } else {
                Ok(pack_outcome(
                    session_id,
                    TurnOutcome {
                        reply_text: err.message,
                        terminal: Terminal::Error,
                        wrote: false,
                    },
                ))
            }
        }
    }
}
