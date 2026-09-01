//! Host-only chat turn orchestration.
//!
//! Agent Loop owns the business Binding and tools. This module only applies
//! the single Host/GLM runtime route, persists completed turns, and exposes
//! the engine-opaque command surface.

use std::time::Instant;

use serde_json::{json, Value};

use crate::config::settings;
use crate::services::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::services::agent::engine_router::{self, AdapterKind, EngineRouteError, TurnInput};
use crate::services::agent::llm;
use crate::services::agent::progress::{self, ProgressSink};
use crate::services::agent::r#loop::{self, ChatTurnResult, Terminal, TurnOutcome};
use crate::services::agent::session::{self, Turn};

/// Reset the current Binding. Agent Loop owns all live context cleanup.
pub fn reset_binding() -> Result<(), String> {
    r#loop::reset_binding().map_err(|_| "reset_binding_failed".to_string())
}

/// Cancel the current business turn without disposing any runtime resource.
pub fn cancel_from_binding() -> Result<(), String> {
    r#loop::cancel_from_binding(|_| Ok(()))
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

fn encode_host_outcome(outcome: TurnOutcome) -> String {
    json!({
        "reply_text": outcome.reply_text,
        "terminal": outcome.terminal.as_str(),
        "wrote": outcome.wrote,
    })
    .to_string()
}

fn decode_host_outcome(value: &str) -> Result<TurnOutcome, String> {
    let value: Value = serde_json::from_str(value).map_err(|error| error.to_string())?;
    let terminal = match value
        .get("terminal")
        .and_then(Value::as_str)
        .unwrap_or("none")
    {
        "business" => Terminal::Business,
        "error" => Terminal::Error,
        _ => Terminal::None,
    };
    Ok(TurnOutcome {
        reply_text: value
            .get("reply_text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        terminal,
        wrote: value.get("wrote").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn host_adapter_turn(
    session_id: &str,
    message: &str,
    trace_id: &TraceId,
    sink: &ProgressSink,
) -> Result<String, String> {
    let started = Instant::now();
    let mut session = session::load_session(session_id)?;
    let outcome = match llm::load_llm_config() {
        Ok(config) => {
            r#loop::run_loop_with_progress(&mut session, message, &config, trace_id, Some(sink))
        }
        Err(error) => {
            let outcome = r#loop::map_llm_error(&error);
            append_user_assistant(&mut session, message, &outcome.reply_text);
            outcome
        }
    };
    log_timing(
        trace_id,
        session_id,
        "assistant.host",
        "turn.completed",
        started,
        "ok",
    );
    Ok(encode_host_outcome(outcome))
}

fn log_timing(
    trace_id: &TraceId,
    session_id: &str,
    component: &'static str,
    event: &'static str,
    started: Instant,
    outcome: &'static str,
) {
    let _ = diagnostics::log(
        DiagnosticEvent::timing(component, event, trace_id, started.elapsed())
            .with_session_id(session_id)
            .with_static_field("outcome", outcome),
    );
}

/// Facade-facing chat turn. Engine selection is read from settings only.
pub fn chat_turn(
    session_id: &str,
    message: &str,
    master_task_id: Option<&str>,
) -> Result<ChatTurnResult, String> {
    let trace_id = TraceId::new();
    chat_turn_with_trace(
        session_id,
        message,
        master_task_id,
        &trace_id,
        progress::noop_sink(),
    )
}

pub(crate) fn chat_turn_with_trace(
    session_id: &str,
    message: &str,
    _master_task_id: Option<&str>,
    trace_id: &TraceId,
    sink: ProgressSink,
) -> Result<ChatTurnResult, String> {
    let turn_started = Instant::now();
    let _ = diagnostics::log(
        DiagnosticEvent::point("assistant.runtime", "turn.started", trace_id)
            .with_session_id(session_id),
    );

    if let Err(early) = r#loop::try_begin_chat_turn(session_id, trace_id, sink.clone()) {
        log_timing(
            trace_id,
            session_id,
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
                session_id,
                "assistant.runtime",
                "session_load.completed",
                session_load_started,
                "ok",
            );
            session
        }
        Err(error) => {
            log_timing(
                trace_id,
                session_id,
                "assistant.runtime",
                "session_load.completed",
                session_load_started,
                "error",
            );
            r#loop::end_chat_turn(session_id);
            return Err(error);
        }
    };

    // Binding is checked before settings or credentials.
    if !r#loop::has_active_binding() {
        let reply = "Unbound — no active Binding Contract; chat cannot run.".to_string();
        append_user_assistant(&mut session, message, &reply);
        r#loop::end_chat_turn(session_id);
        log_timing(
            trace_id,
            session_id,
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
        Ok(settings) => settings,
        Err(error) => {
            r#loop::end_chat_turn(session_id);
            log_timing(
                trace_id,
                session_id,
                "assistant.runtime",
                "turn.completed",
                turn_started,
                "settings_error",
            );
            return Err(error.to_string());
        }
    };

    let input = TurnInput {
        session_id: session_id.to_string(),
        message: message.to_string(),
    };
    let session_id_for_adapter = session_id.to_string();
    let message_for_adapter = message.to_string();
    let sink_for_adapter = sink.clone();
    let dispatch_started = Instant::now();
    let routed = engine_router::dispatch_from_settings(
        &settings,
        &input,
        || {
            host_adapter_turn(
                &session_id_for_adapter,
                &message_for_adapter,
                trace_id,
                &sink_for_adapter,
            )
        },
    );

    r#loop::end_chat_turn(session_id);

    match routed {
        Ok(route) => {
            log_timing(
                trace_id,
                session_id,
                "assistant.runtime",
                "engine_dispatch.completed",
                dispatch_started,
                "ok",
            );
            let outcome = decode_host_outcome(&route.body)?;
            let result = pack_outcome(session_id, outcome);
            let result_outcome = if result.body["terminal"] == "error" {
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
                .with_session_id(session_id)
                .with_static_field("outcome", result_outcome)
                .with_static_field("adapter", adapter_name(route.adapter)),
            );
            Ok(result)
        }
        Err(EngineRouteError::Unconfigured) => {
            log_timing(
                trace_id,
                session_id,
                "assistant.runtime",
                "engine_dispatch.completed",
                dispatch_started,
                "unconfigured",
            );
            log_timing(
                trace_id,
                session_id,
                "assistant.runtime",
                "turn.completed",
                turn_started,
                "unconfigured",
            );
            Ok(pack_outcome(
                session_id,
                TurnOutcome {
                    reply_text:
                        "GLM is not configured. Configure Host / GLM in Settings before starting a chat."
                            .into(),
                    terminal: Terminal::Error,
                    wrote: false,
                },
            ))
        }
        Err(EngineRouteError::Adapter(message)) => {
            log_timing(
                trace_id,
                session_id,
                "assistant.runtime",
                "engine_dispatch.completed",
                dispatch_started,
                "adapter_error",
            );
            log_timing(
                trace_id,
                session_id,
                "assistant.runtime",
                "turn.completed",
                turn_started,
                "error",
            );
            Ok(pack_outcome(
                session_id,
                TurnOutcome {
                    reply_text: message,
                    terminal: Terminal::Error,
                    wrote: false,
                },
            ))
        }
    }
}

fn adapter_name(adapter: AdapterKind) -> &'static str {
    match adapter {
        AdapterKind::Host => "host",
    }
}
