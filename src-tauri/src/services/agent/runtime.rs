//! Engine-aware chat turn orchestration (busy / gate / persist / ChatTurnResult).
//!
//! Sibling to Host `loop` and Cursor `cursor_adapter`. Formal chat enters here;
//! settings select Host (`run_loop`) or Cursor (Profile-backed adapter). Cursor
//! failures never fall back to Host LLM or process-local business tool handlers.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use serde_json::{json, Value};

use crate::config::settings;
use crate::services::agent::cursor_adapter::{
    self, CreateRequest, CursorError, CursorErrorCode, CursorLlmEngine,
    TurnOutcome as CursorTurnOutcome, TurnRequest,
};
use crate::services::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::services::agent::profile::{self, BusinessProfileSnapshot};
#[cfg(test)]
use crate::services::agent::cursor_adapter::CursorSessionRuntime;
use crate::services::agent::engine_router::{self, AdapterKind, EngineRouteError, TurnInput};
use crate::services::agent::llm;
use crate::services::agent::r#loop::{self, ChatTurnResult, Terminal, TurnOutcome};
use crate::services::agent::session::{self, Turn};

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

#[cfg(test)]
static TEST_CURSOR_RT: OnceLock<Mutex<Option<Arc<CursorSessionRuntime>>>> = OnceLock::new();
#[cfg(test)]
static TEST_CURSOR_INSTALLED: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static TEST_PROFILE: OnceLock<Mutex<Option<BusinessProfileSnapshot>>> = OnceLock::new();

#[cfg(test)]
fn test_cursor_slot() -> &'static Mutex<Option<Arc<CursorSessionRuntime>>> {
    TEST_CURSOR_RT.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn test_profile_slot() -> &'static Mutex<Option<BusinessProfileSnapshot>> {
    TEST_PROFILE.get_or_init(|| Mutex::new(None))
}

pub fn reset_for_tests() {
    #[cfg(test)]
    {
        *test_cursor_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
        TEST_CURSOR_INSTALLED.store(false, Ordering::SeqCst);
    }
    #[cfg(test)]
    {
        *test_profile_slot().lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// UI session lifecycle is presentation-only; it must not dispose an Agent.
pub fn on_ui_session_close(_session_id: &str) {}

/// Cancel the current business turn without disposing its Agent slot.
pub fn cancel_from_binding() -> Result<(), String> {
    r#loop::cancel_from_binding(|business_id| {
        CursorLlmEngine::global()
            .cancel(business_id)
            .map_err(|error| error.message)
    })
}

#[cfg(test)]
pub fn set_cursor_runtime_for_tests(rt: Option<Arc<CursorSessionRuntime>>) {
    TEST_CURSOR_INSTALLED.store(rt.is_some(), Ordering::SeqCst);
    *test_cursor_slot().lock().unwrap_or_else(|e| e.into_inner()) = rt;
}

#[cfg(test)]
pub fn set_profile_for_tests(profile: Option<BusinessProfileSnapshot>) {
    *test_profile_slot().lock().unwrap_or_else(|e| e.into_inner()) = profile;
}

#[cfg(test)]
pub fn cursor_runtime_installed_for_tests() -> bool {
    TEST_CURSOR_INSTALLED.load(Ordering::SeqCst)
}

/// Production Cursor path: CursorLlmEngine (ensure_client → request).
fn cursor_run_turn(
    req: &TurnRequest,
    trace_id: &TraceId,
) -> Result<CursorTurnOutcome, CursorError> {
    #[cfg(test)]
    if let Some(rt) = test_cursor_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        // request-only CSR: typed create/turn/cancel removed; CSR.run_turn uses request.
        return rt.run_turn(req);
    }
    CursorLlmEngine::global().run_turn_with_trace(req, trace_id)
}

fn cursor_create_with_trace(
    req: &CreateRequest,
    trace_id: &TraceId,
) -> Result<(), CursorError> {
    #[cfg(test)]
    if test_cursor_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
    {
        // The test-only session runtime keeps its legacy create+turn seam inside
        // `run_turn`; production always uses the explicit engine create below.
        return Ok(());
    }
    CursorLlmEngine::global().create_with_trace(req, trace_id)
}

fn profile_for_business(key: &str) -> Result<BusinessProfileSnapshot, CursorError> {
    #[cfg(test)]
    if let Some(profile) = test_profile_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        if profile.business_id == key {
            return Ok(profile);
        }
    }
    profile::query_business_profile(key).map_err(cursor_adapter::map_profile_error)
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

fn host_adapter_turn(
    session_id: &str,
    message: &str,
    trace_id: &TraceId,
) -> Result<String, String> {
    let started = Instant::now();
    let mut session = session::load_session(session_id)?;
    let outcome = match llm::load_llm_config() {
        Ok(cfg) => r#loop::run_loop(&mut session, message, &cfg),
        Err(e) => {
            let out = r#loop::map_llm_error(&e);
            append_user_assistant(&mut session, message, &out.reply_text);
            out
        }
    };
    log_timing(trace_id, "assistant.host", "turn.completed", started, "ok");
    Ok(encode_routed(&RoutedTurn {
        reply_text: outcome.reply_text,
        terminal: outcome.terminal,
        wrote: outcome.wrote,
        cursor_error: None,
    }))
}

fn cursor_adapter_turn(
    _session_id: &str,
    message: &str,
    trace_id: &TraceId,
) -> Result<String, String> {
    let started = Instant::now();
    // L2-A: consume AIAssistantSession execution-context snapshot (sole live owner).
    let live_ctx = session::live_context_owner().execution_context_snapshot();

    let settings = settings::load().map_err(|e| e.to_string())?;
    let cfg = engine_router::read_engine_runtime_config(&settings).map_err(|e| e.to_string())?;
    let Some(api_key) = cfg.credential.filter(|s| !s.is_empty()) else {
        let err = CursorError::new(
            CursorErrorCode::Credential,
            cursor_adapter::frontend_message_for(CursorErrorCode::Credential),
        );
        return Ok(encode_routed(&cursor_err_routed(err)));
    };

    let Some(key) = live_ctx
        .binding
        .as_ref()
        .and_then(session::binding_business_id)
    else {
        let err = CursorError::new(
            CursorErrorCode::SdkConfig,
            cursor_adapter::frontend_message_for(CursorErrorCode::SdkConfig),
        );
        return Ok(encode_routed(&cursor_err_routed(err)));
    };

    let profile = match profile_for_business(&key) {
        Ok(profile) => profile,
        Err(err) => return Ok(encode_routed(&cursor_err_routed(err))),
    };

    let profile_for_create = profile.clone();
    let profile_for_turn = profile;
    let api_key_for_create = api_key.clone();
    let result = r#loop::ensure_create_then_turn_with_error(
        message,
        CursorError::new(
            CursorErrorCode::SdkConfig,
            cursor_adapter::frontend_message_for(CursorErrorCode::SdkConfig),
        ),
        CursorError::new(
            CursorErrorCode::SdkConfig,
            cursor_adapter::frontend_message_for(CursorErrorCode::SdkConfig),
        ),
        CursorError::new(
            CursorErrorCode::RecoverableFailure,
            cursor_adapter::frontend_message_for(CursorErrorCode::RecoverableFailure),
        ),
        move |business_id, create_session_id| {
            cursor_create_with_trace(
                &CreateRequest {
                    business_id: business_id.to_string(),
                    session_id: create_session_id.to_string(),
                    api_key: api_key_for_create.clone(),
                    profile: profile_for_create.clone(),
                },
                trace_id,
            )
        },
        move |business_id, turn_session_id, prompt| {
            if profile_for_turn.business_id != business_id {
                return Err(CursorError::new(
                    CursorErrorCode::SdkConfig,
                    cursor_adapter::frontend_message_for(CursorErrorCode::SdkConfig),
                ));
            }
            let req = TurnRequest {
                session_id: turn_session_id.to_string(),
                prompt: prompt.to_string(),
                api_key: api_key.clone(),
                profile: profile_for_turn.clone(),
            };
            let turn_result = cursor_run_turn(&req, trace_id);
            let should_persist = cursor_adapter::should_persist_cursor_result(&turn_result);

            match turn_result {
                Ok(out) => {
                    if should_persist {
                        let persist_started = Instant::now();
                        let mut session = session::load_session(turn_session_id)
                            .map_err(|error| CursorError::new(CursorErrorCode::Runner, error))?;
                        append_user_assistant(&mut session, prompt, &out.text);
                        log_timing(
                            trace_id,
                            "assistant.cursor",
                            "cursor.session_persist.completed",
                            persist_started,
                            "ok",
                        );
                    }
                    log_timing(
                        trace_id,
                        "assistant.cursor",
                        "cursor.adapter.completed",
                        started,
                        "ok",
                    );
                    Ok(RoutedTurn {
                        reply_text: out.text,
                        terminal: Terminal::None,
                        wrote: false,
                        cursor_error: None,
                    })
                }
                Err(err) => {
                    if should_persist {
                        let persist_started = Instant::now();
                        let mut session = session::load_session(turn_session_id)
                            .map_err(|error| CursorError::new(CursorErrorCode::Runner, error))?;
                        append_user_assistant(&mut session, prompt, &err.message);
                        log_timing(
                            trace_id,
                            "assistant.cursor",
                            "cursor.session_persist.completed",
                            persist_started,
                            "ok",
                        );
                    }
                    log_timing(
                        trace_id,
                        "assistant.cursor",
                        "cursor.adapter.completed",
                        started,
                        "error",
                    );
                    Err(err)
                }
            }
        },
    );

    match result {
        Ok(routed) => Ok(encode_routed(&routed)),
        Err(err) => Ok(encode_routed(&cursor_err_routed(err))),
    }
}

fn log_timing(
    trace_id: &TraceId,
    component: &'static str,
    event: &'static str,
    started: Instant,
    outcome: &'static str,
) {
    let _ = diagnostics::log(
        DiagnosticEvent::timing(component, event, trace_id, started.elapsed())
            .with_static_field("outcome", outcome),
    );
}

fn adapter_name(adapter: AdapterKind) -> &'static str {
    match adapter {
        AdapterKind::Host => "host",
        AdapterKind::Cursor => "cursor",
    }
}

/// Facade-facing chat turn: engine-opaque signature; routes Host/Cursor by settings.
pub fn chat_turn(
    session_id: &str,
    message: &str,
    _master_task_id: Option<&str>,
) -> Result<ChatTurnResult, String> {
    let trace_id = TraceId::new();
    chat_turn_with_trace(session_id, message, _master_task_id, &trace_id)
}

pub(crate) fn chat_turn_with_trace(
    session_id: &str,
    message: &str,
    _master_task_id: Option<&str>,
    trace_id: &TraceId,
) -> Result<ChatTurnResult, String> {
    let turn_started = Instant::now();
    let _ = diagnostics::log(DiagnosticEvent::point(
        "assistant.runtime",
        "turn.started",
        trace_id,
    ));
    if let Err(early) = r#loop::try_begin_chat_turn(session_id) {
        log_timing(
            trace_id,
            "assistant.runtime",
            "turn.completed",
            turn_started,
            "busy",
        );
        return Ok(early);
    }

    let session_load_started = Instant::now();
    let mut session = match session::load_session(session_id) {
        Ok(session) => {
            log_timing(
                trace_id,
                "assistant.runtime",
                "session_load.completed",
                session_load_started,
                "ok",
            );
            session
        }
        Err(err) => {
            log_timing(
                trace_id,
                "assistant.runtime",
                "session_load.completed",
                session_load_started,
                "error",
            );
            return Err(err);
        }
    };

    // Gate Binding before any engine/config — unbound must not depend on secrets.
    if !r#loop::has_active_binding() {
        let reply = "Unbound — no active Binding Contract; chat cannot run.".to_string();
        append_user_assistant(&mut session, message, &reply);
        r#loop::end_chat_turn_busy();
        log_timing(
            trace_id,
            "assistant.runtime",
            "turn.completed",
            turn_started,
            "unbound",
        );
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
            log_timing(
                trace_id,
                "assistant.runtime",
                "turn.completed",
                turn_started,
                "settings_error",
            );
            return Err(e.to_string());
        }
    };

    let input = TurnInput {
        session_id: session_id.to_string(),
        message: message.to_string(),
    };

    let sid = session_id.to_string();
    let msg = message.to_string();
    let dispatch_started = Instant::now();
    let routed = engine_router::dispatch_from_settings(
        &settings,
        &input,
        || host_adapter_turn(&sid, &msg, trace_id),
        || cursor_adapter_turn(&sid, &msg, trace_id),
    );

    r#loop::end_chat_turn_busy();

    match routed {
        Ok(route) => {
            let adapter = adapter_name(route.adapter);
            log_timing(
                trace_id,
                "assistant.runtime",
                "engine_dispatch.completed",
                dispatch_started,
                "ok",
            );
            let decoded = decode_routed(&route.body)?;
            if let Some(err) = decoded.cursor_error {
                // Typed Cursor surface — never Host generic upstream collapse.
                if matches!(route.adapter, AdapterKind::Cursor) {
                    log_timing(
                        trace_id,
                        "assistant.runtime",
                        "turn.completed",
                        turn_started,
                        "error",
                    );
                    return Ok(pack_cursor_error(session_id, &err));
                }
            }
            let result = pack_outcome(
                session_id,
                TurnOutcome {
                    reply_text: decoded.reply_text,
                    terminal: decoded.terminal,
                    wrote: decoded.wrote,
                },
            );
            let outcome = if result.body["terminal"] == "error" {
                "error"
            } else {
                "ok"
            };
            let _ = diagnostics::log(
                DiagnosticEvent::timing(
                    "assistant.runtime",
                    "turn.completed",
                    trace_id,
                    turn_started.elapsed(),
                )
                .with_static_field("outcome", outcome)
                .with_static_field("adapter", adapter),
            );
            Ok(result)
        }
        Err(EngineRouteError::InvalidEngine(v)) => {
            log_timing(
                trace_id,
                "assistant.runtime",
                "engine_dispatch.completed",
                dispatch_started,
                "invalid_engine",
            );
            log_timing(
                trace_id,
                "assistant.runtime",
                "turn.completed",
                turn_started,
                "error",
            );
            Ok(pack_outcome(
                session_id,
                TurnOutcome {
                    reply_text: format!("invalid assistant_engine value: {v}"),
                    terminal: Terminal::Error,
                    wrote: false,
                },
            ))
        }
        Err(EngineRouteError::Adapter(msg)) => {
            log_timing(
                trace_id,
                "assistant.runtime",
                "engine_dispatch.completed",
                dispatch_started,
                "adapter_error",
            );
            // Adapter string errors on Cursor must not become Host upstream text.
            let err = CursorError::new(CursorErrorCode::Runner, msg);
            log_timing(
                trace_id,
                "assistant.runtime",
                "turn.completed",
                turn_started,
                "error",
            );
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
