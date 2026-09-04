//! `run_loop` / `run_loop_with_trace` / `run_loop_with_progress` only.

use std::time::Instant;

use serde_json::{json, Value};

use crate::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::agent::llm::{self, LlmConfig};
use crate::agent::progress::{self, ProgressSink};
use crate::agent::session::{Session, Turn};
use crate::agent::tools::{self, InvokeOutcome, PrepareError};

use crate::agent::binding::{
    current_binding_generation_snapshot, current_binding_snapshot, loaded_path_fence,
    session_capability_mcp_config,
};
use super::flights::{chat_turn_interrupted, runtime};
use super::history::{
    build_llm_messages_from_turns, cancelled_turn_outcome, is_clarify_text, map_llm_error, persist,
    prompt_text_from_binding,
};
use super::types::{Terminal, TurnOutcome, MAX_CLARIFY_ROUNDS, MAX_MCP_TOOL_CALLS, MAX_MCP_TOOL_ROUNDS};

pub fn run_loop(session: &mut Session, user_message: &str, config: &LlmConfig) -> TurnOutcome {
    run_loop_with_trace(session, user_message, config, &TraceId::new())
}

pub(crate) fn run_loop_with_trace(
    session: &mut Session,
    user_message: &str,
    config: &LlmConfig,
    trace_id: &TraceId,
) -> TurnOutcome {
    run_loop_with_progress(session, user_message, config, trace_id, None)
}

pub(crate) fn run_loop_with_progress(
    session: &mut Session,
    user_message: &str,
    config: &LlmConfig,
    trace_id: &TraceId,
    sink: Option<&ProgressSink>,
) -> TurnOutcome {
    let turns_checkpoint = session.turns.len();
    // Executable turns require Binding Contract bound.
    let Some((binding, generation)) = current_binding_generation_snapshot() else {
        session.turns.push(Turn {
            role: "user".into(),
            content: Some(user_message.to_string()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        // Distinguish never/fully unbound from a corrupted bound-without-generation slot.
        if current_binding_snapshot().is_none() {
            let reply =
                "Unbound — no active Binding Contract; chat cannot run.".to_string();
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
        return cancelled_turn_outcome(session, turns_checkpoint);
    };

    let system_prompt = prompt_text_from_binding(&binding.prompt);
    if system_prompt.trim().is_empty() {
        session.turns.push(Turn {
            role: "user".into(),
            content: Some(user_message.to_string()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        let reply = "Binding prompt is empty; cannot run chat.".to_string();
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

    // A key-only Binding resolves this connection at Set time. Typed/internal
    // bindings retain their historical text-only behavior when they carry no
    // MCP capability config.
    if chat_turn_interrupted(&session.session_id, generation) {
        return cancelled_turn_outcome(session, turns_checkpoint);
    }

    let mut turn_fence = turn_fence_for_session(session);
    let mcp_config = session_capability_mcp_config();
    let turn_tools = match tools::discover_and_merge(mcp_config.as_ref(), turn_fence.is_some()) {
        Ok(tools) => tools,
        Err(PrepareError::EmptyMcpTools) => {
            let reply = "当前场景的 MCP 服务未暴露任何工具，无法继续。".to_string();
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
                wrote: false,
            };
        }
        Err(PrepareError::Mcp(error)) => {
            let reply = format!("当前场景的 MCP 服务不可用：{error}");
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
                wrote: false,
            };
        }
    };
    if chat_turn_interrupted(&session.session_id, generation) {
        return cancelled_turn_outcome(session, turns_checkpoint);
    }

    let mut wrote = false;
    let mut tool_rounds = 0usize;
    let mut tool_call_count = 0usize;
    let msg = loop {
        if chat_turn_interrupted(&session.session_id, generation) {
            return cancelled_turn_outcome(session, turns_checkpoint);
        }
        let messages = build_llm_messages_from_turns(&session.turns, &system_prompt);
        let tool_defs = turn_tools
            .catalog
            .as_ref()
            .map(|catalog| catalog.definitions.as_slice())
            .unwrap_or(&[]);
        progress::emit_progress(
            sink,
            &session.session_id,
            trace_id,
            "Requesting…",
        );
        let msg = match llm::chat_completions(&messages, tool_defs, config) {
            Ok(message) => message,
            Err(error) => {
                if chat_turn_interrupted(&session.session_id, generation) {
                    return cancelled_turn_outcome(session, turns_checkpoint);
                }
                let mut out = map_llm_error(&error);
                out.wrote = wrote;
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

        // Round-trip gate: cancel / generation may have been raised during LLM.
        if chat_turn_interrupted(&session.session_id, generation) {
            return cancelled_turn_outcome(session, turns_checkpoint);
        }

        if msg.tool_calls.is_empty() {
            break msg;
        }

        if turn_tools.catalog.is_none() {
            // Typed/internal Binding without an MCP server keeps the old
            // text-only contract and never dispatches in-process tools.
            let reply = "模型响应异常或请求了不支持的工具，未执行任何写入。".to_string();
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

        if tool_rounds >= MAX_MCP_TOOL_ROUNDS
            || tool_call_count.saturating_add(msg.tool_calls.len()) > MAX_MCP_TOOL_CALLS
        {
            let reply = "工具调用次数已达上限，未继续执行。".to_string();
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
        tool_call_count += msg.tool_calls.len();

        let tool_calls = Value::Array(
            msg.tool_calls
                .iter()
                .map(|call| {
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": call.arguments,
                        }
                    })
                })
                .collect(),
        );
        session.turns.push(Turn {
            role: "assistant".into(),
            content: msg.content.clone(),
            tool_call_id: None,
            tool_calls: Some(tool_calls),
            name: None,
        });
        persist(session);

        for call in msg.tool_calls {
            if chat_turn_interrupted(&session.session_id, generation) {
                return cancelled_turn_outcome(session, turns_checkpoint);
            }
            progress::emit_progress(
                sink,
                &session.session_id,
                trace_id,
                format!("Calling {}…", call.name),
            );

            let tool_started = Instant::now();
            if chat_turn_interrupted(&session.session_id, generation) {
                return cancelled_turn_outcome(session, turns_checkpoint);
            }
            let (result, use_host, staged_note) = match tools::invoke(
                &turn_tools,
                &call.name,
                &call.arguments,
                turn_fence.as_ref(),
                session,
                trace_id,
            ) {
                InvokeOutcome::Done {
                    result,
                    host,
                    staged_note,
                } => (result, host, staged_note),
                InvokeOutcome::Abort(reply) => {
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
            };

            if chat_turn_interrupted(&session.session_id, generation) {
                return cancelled_turn_outcome(session, turns_checkpoint);
            }
            let catalog = turn_tools.catalog.as_ref().expect("catalog present");
            if !result.is_error && (catalog.is_mutating(&call.name) || staged_note) {
                wrote = true;
            }
            if !result.is_error && (call.name == "stage" || staged_note) {
                turn_fence = turn_fence_for_session(session);
            }
            let _ = diagnostics::log(
                DiagnosticEvent::timing(
                    "assistant.tools",
                    "tool.completed",
                    trace_id,
                    tool_started.elapsed(),
                )
                .with_session_id(&session.session_id)
                .with_bounded_text_field("tool_name", &call.name)
                .with_static_field("source", if use_host { "host" } else { "mcp" })
                .with_bool_field("is_error", result.is_error),
            );
            let empty_copy = if use_host {
                if result.is_error {
                    "Host tool failed without an error message."
                } else {
                    "Host tool completed without text output."
                }
            } else if result.is_error {
                "MCP tool failed without an error message."
            } else {
                "MCP tool completed without text output."
            };
            session.turns.push(Turn {
                role: "tool".into(),
                content: Some(if result.content.trim().is_empty() {
                    empty_copy.into()
                } else {
                    result.content
                }),
                tool_call_id: Some(call.id),
                tool_calls: None,
                name: Some(call.name),
            });
        }
        persist(session);
    };

    let content = msg.content.clone().unwrap_or_default();
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
    TurnOutcome {
        reply_text: content,
        terminal: Terminal::None,
        wrote,
    }
}

fn turn_fence_for_session(session: &Session) -> Option<crate::services::path_fence::PathFence> {
    loaded_path_fence().map(|base| {
        base.with_session_writes(
            &session.session_id,
            session.staged.iter().map(|entry| entry.path.as_str()),
        )
        .unwrap_or_else(|_| {
            base.with_session_scratch(&session.session_id)
                .unwrap_or(base)
        })
    })
}
