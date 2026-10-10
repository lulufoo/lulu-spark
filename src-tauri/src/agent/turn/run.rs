//! `run_loop` / `run_loop_with_trace` / `run_loop_with_progress` only.

use std::time::Instant;

use serde_json::{json, Value};

use crate::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::agent::llm::{self, LlmConfig};
use crate::agent::progress::{self, ProgressSink};
use crate::agent::session::{self, read_unix_millis, Session, Step, StepClock};
use crate::agent::tools::{self, InvokeOutcome, PrepareError};

use crate::agent::binding::{
    current_binding_generation_snapshot, current_binding_snapshot, loaded_path_fence,
    session_capability_mcp_config,
};
use super::flights::{chat_turn_interrupted, runtime};
use super::history::{
    build_llm_messages, cancelled_turn_outcome, is_clarify_text, map_llm_error,     persist_thinking,
    prompt_text_from_binding,
};
use super::response::{empty_reply_text, with_response};
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
    let mut thinking = progress::TurnThinking::default();
    let mut steps_checkpoint = session.steps.len();
    // Executable turns require Binding Contract bound.
    let Some((binding, generation)) = current_binding_generation_snapshot() else {
        session.steps.push(Step {
            role: "user".into(),
            content: Some(user_message.to_string()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            finish_reason: None,
            model: None,
            usage: None,
            reasoning_content: None,
            clock: StepClock::stamp_now(),
        });
        // Distinguish never/fully unbound from a corrupted bound-without-generation slot.
        if current_binding_snapshot().is_none() {
            let reply =
                "Unbound — no active Binding Contract; chat cannot run.".to_string();
            session.steps.push(Step {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
                clock: StepClock::stamp_now(),
            });
            persist_thinking(session, thinking.persist_ms());
            return TurnOutcome {
                reply_text: reply,
                terminal: Terminal::Business,
                wrote: false,
            };
        }
        return cancelled_turn_outcome(session, steps_checkpoint);
    };

    let system_prompt = prompt_text_from_binding(&binding.prompt);
    if system_prompt.trim().is_empty() {
        session.steps.push(Step {
            role: "user".into(),
            content: Some(user_message.to_string()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            finish_reason: None,
            model: None,
            usage: None,
            reasoning_content: None,
            clock: StepClock::stamp_now(),
        });
        let reply = "Binding prompt is empty; cannot run chat.".to_string();
        session.steps.push(Step {
            role: "assistant".into(),
            content: Some(reply.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            finish_reason: None,
            model: None,
            usage: None,
            reasoning_content: None,
            clock: StepClock::stamp_now(),
        });
        persist_thinking(session, thinking.persist_ms());
        return TurnOutcome {
            reply_text: reply,
            terminal: Terminal::Error,
            wrote: false,
        };
    }

    crate::agent::context::maybe_compress(session, config);
    steps_checkpoint = session.steps.len();

    session.steps.push(Step {
        role: "user".into(),
        content: Some(user_message.to_string()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
        finish_reason: None,
        model: None,
        usage: None,
        reasoning_content: None,
        clock: StepClock::stamp_now(),
    });
    persist_thinking(session, thinking.persist_ms());

    // A key-only Binding resolves this connection at Set time. Typed/internal
    // bindings retain their historical text-only behavior when they carry no
    // MCP capability config.
    if chat_turn_interrupted(&session.session_id, generation) {
        return cancelled_turn_outcome(session, steps_checkpoint);
    }

    let mut turn_fence = turn_fence_for_session(session);
    let mcp_config = session_capability_mcp_config();
    let turn_tools = match tools::discover_and_merge(mcp_config.as_ref(), turn_fence.is_some()) {
        Ok(tools) => tools,
        Err(PrepareError::EmptyMcpTools) => {
            let reply = "当前场景的 MCP 服务未暴露任何工具，无法继续。".to_string();
            session.steps.push(Step {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
                clock: StepClock::stamp_now(),
            });
            persist_thinking(session, thinking.persist_ms());
            return TurnOutcome {
                reply_text: reply,
                terminal: Terminal::Error,
                wrote: false,
            };
        }
        Err(PrepareError::Mcp(error)) => {
            let reply = format!("当前场景的 MCP 服务不可用：{error}");
            session.steps.push(Step {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
                clock: StepClock::stamp_now(),
            });
            persist_thinking(session, thinking.persist_ms());
            return TurnOutcome {
                reply_text: reply,
                terminal: Terminal::Error,
                wrote: false,
            };
        }
    };
    let scratch = turn_fence.as_ref().and_then(|fence| {
        fence
            .session_scratch_root(&session.session_id)
            .ok()
            .flatten()
    });
    let system_prompt = crate::agent::fill_host_file_dirs(&system_prompt, scratch.as_deref());
    if chat_turn_interrupted(&session.session_id, generation) {
        return cancelled_turn_outcome(session, steps_checkpoint);
    }

    let mut wrote = false;
    let mut tool_rounds = 0usize;
    let mut tool_call_count = 0usize;
    let (msg, llm_start, llm_end) = loop {
        if chat_turn_interrupted(&session.session_id, generation) {
            return cancelled_turn_outcome(session, steps_checkpoint);
        }
        let summaries =
            session::load_summary_bodies(&session.session_id).unwrap_or_default();
        let messages = build_llm_messages(&session.steps, &system_prompt, &summaries);
        let tool_defs = turn_tools
            .catalog
            .as_ref()
            .map(|catalog| catalog.definitions.as_slice())
            .unwrap_or(&[]);
        let (thinking_text, thinking_ms) = thinking.view(None);
        progress::emit_progress_state(
            sink,
            &session.session_id,
            trace_id,
            "Requesting…",
            thinking_text,
            thinking_ms,
        );
        crate::agent::context::record_sent_prompt(
            &session.session_id,
            &messages,
            tool_defs,
            &tools::mcp_names(&turn_tools),
        );
        let session_id = session.session_id.clone();
        let mut on_delta = |hint: &str, reasoning: &str| {
            let (text, ms) = thinking.tick(reasoning);
            progress::emit_progress_state(sink, &session_id, trace_id, hint, text, ms);
        };
        let ctx = llm::CallContext {
            session_id: session.session_id.clone(),
            trace_id: Some(trace_id.as_str().to_string()),
            purpose: "turn",
            round: tool_rounds + 1,
            step_index: session.steps.len(),
        };
        let llm_start = read_unix_millis();
        let call = llm::chat_completions_recorded(
            &messages,
            tool_defs,
            config,
            llm::DEFAULT_TIMEOUT,
            Some(&mut on_delta),
            Some(&ctx),
        );
        let llm_end = read_unix_millis();
        // Kept even when the call is rejected: the thinking and `finish_reason` that
        // did arrive go onto the error Step below.
        let arrived = call.snapshot.clone();
        thinking.finish_call(arrived.as_ref().and_then(|message| message.reasoning_content.as_deref()));
        let msg = match call.into_result() {
            Ok(message) => message,
            Err(error) => {
                if chat_turn_interrupted(&session.session_id, generation) {
                    return cancelled_turn_outcome(session, steps_checkpoint);
                }
                let mut out = map_llm_error(&error);
                out.wrote = wrote;
                // The text is our error copy, not model output; the fields are the model's.
                let step = Step {
                    role: "assistant".into(),
                    content: Some(out.reply_text.clone()),
                    tool_call_id: None,
                    tool_calls: None,
                    name: None,
                    finish_reason: None,
                    model: None,
                    usage: None,
                    reasoning_content: None,
                    clock: StepClock::stamp_span(llm_start, llm_end),
                };
                session.steps.push(match &arrived {
                    Some(message) => with_response(step, message),
                    None => step,
                });
                persist_thinking(session, thinking.persist_ms());
                return out;
            }
        };

        // Round-trip gate: cancel / generation may have been raised during LLM.
        if chat_turn_interrupted(&session.session_id, generation) {
            return cancelled_turn_outcome(session, steps_checkpoint);
        }

        if msg.tool_calls.is_empty() {
            break (msg, llm_start, llm_end);
        }

        if turn_tools.catalog.is_none() {
            // Typed/internal Binding without an MCP server keeps the old
            // text-only contract and never dispatches in-process tools.
            let reply = "模型响应异常或请求了不支持的工具，未执行任何写入。".to_string();
            session.steps.push(Step {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
                clock: StepClock::stamp_now(),
            });
            persist_thinking(session, thinking.persist_ms());
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
            session.steps.push(Step {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
                clock: StepClock::stamp_now(),
            });
            persist_thinking(session, thinking.persist_ms());
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
        session.steps.push(with_response(Step {
            role: "assistant".into(),
            content: msg.content.clone(),
            tool_call_id: None,
            tool_calls: Some(tool_calls),
            name: None,
            finish_reason: None,
            model: None,
            usage: None,
            reasoning_content: None,
            clock: StepClock::stamp_span(llm_start, llm_end),
        }, &msg));
        persist_thinking(session, thinking.persist_ms());

        for call in msg.tool_calls {
            if chat_turn_interrupted(&session.session_id, generation) {
                return cancelled_turn_outcome(session, steps_checkpoint);
            }
            let (thinking_text, thinking_ms) = thinking.view(None);
            progress::emit_progress_state(
                sink,
                &session.session_id,
                trace_id,
                format!("Calling {}…", call.name),
                thinking_text,
                thinking_ms,
            );

            let tool_started = Instant::now();
            let tool_start_ts = read_unix_millis();
            if chat_turn_interrupted(&session.session_id, generation) {
                return cancelled_turn_outcome(session, steps_checkpoint);
            }
            let (result, use_host) = match tools::invoke(
                &turn_tools,
                &call.name,
                &call.arguments,
                turn_fence.as_ref(),
                session,
                trace_id,
            ) {
                InvokeOutcome::Done { result, host } => (result, host),
                InvokeOutcome::Abort(reply) => {
                    session.steps.push(Step {
                        role: "assistant".into(),
                        content: Some(reply.clone()),
                        tool_call_id: None,
                        tool_calls: None,
                        name: None,
                        finish_reason: None,
                        model: None,
                        usage: None,
                        reasoning_content: None,
                        clock: StepClock::stamp_now(),
                    });
                    persist_thinking(session, thinking.persist_ms());
                    return TurnOutcome {
                        reply_text: reply,
                        terminal: Terminal::Error,
                        wrote,
                    };
                }
            };
            let tool_end_ts = read_unix_millis();

            if chat_turn_interrupted(&session.session_id, generation) {
                return cancelled_turn_outcome(session, steps_checkpoint);
            }
            let catalog = turn_tools.catalog.as_ref().expect("catalog present");
            if !result.is_error && catalog.is_mutating(&call.name) {
                wrote = true;
            }
            if !result.is_error && call.name == "stage" {
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
            session.steps.push(Step {
                role: "tool".into(),
                content: Some(if result.content.trim().is_empty() {
                    empty_copy.into()
                } else {
                    result.content
                }),
                tool_call_id: Some(call.id),
                tool_calls: None,
                name: Some(call.name),
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
                clock: StepClock::stamp_span(tool_start_ts, tool_end_ts),
            });
        }
        persist_thinking(session, thinking.persist_ms());
    };

    let content = msg.content.clone().unwrap_or_default();
    if content.trim().is_empty() {
        let reply = empty_reply_text(wrote, tool_rounds).to_string();
        session.steps.push(with_response(Step {
            role: "assistant".into(),
            content: Some(reply.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            finish_reason: None,
            model: None,
            usage: None,
            reasoning_content: None,
            clock: StepClock::stamp_span(llm_start, llm_end),
        }, &msg));
        persist_thinking(session, thinking.persist_ms());
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
            session.steps.push(Step {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
                clock: StepClock::stamp_now(),
            });
            persist_thinking(session, thinking.persist_ms());
            return TurnOutcome {
                reply_text: reply,
                terminal: Terminal::Error,
                wrote,
            };
        }
        *count += 1;
        drop(rt);
        session.steps.push(with_response(Step {
            role: "assistant".into(),
            content: Some(content.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            finish_reason: None,
            model: None,
            usage: None,
            reasoning_content: None,
            clock: StepClock::stamp_span(llm_start, llm_end),
        }, &msg));
        persist_thinking(session, thinking.persist_ms());
        return TurnOutcome {
            reply_text: content,
            terminal: Terminal::None,
            wrote,
        };
    }

    if content.contains("目前不支持") {
        session.steps.push(with_response(Step {
            role: "assistant".into(),
            content: Some(content.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            finish_reason: None,
            model: None,
            usage: None,
            reasoning_content: None,
            clock: StepClock::stamp_span(llm_start, llm_end),
        }, &msg));
        persist_thinking(session, thinking.persist_ms());
        return TurnOutcome {
            reply_text: content,
            terminal: Terminal::Business,
            wrote,
        };
    }

    session.steps.push(with_response(Step {
        role: "assistant".into(),
        content: Some(content.clone()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
        finish_reason: None,
        model: None,
        usage: None,
        reasoning_content: None,
        clock: StepClock::stamp_span(llm_start, llm_end),
    }, &msg));
    persist_thinking(session, thinking.persist_ms());
    TurnOutcome {
        reply_text: content,
        terminal: Terminal::None,
        wrote,
    }
}

fn turn_fence_for_session(session: &Session) -> Option<crate::services::path_fence::PathFence> {
    loaded_path_fence().map(|base| {
        let mut fence = base
            .with_session_scratch(&session.session_id)
            .unwrap_or(base);
        for entry in session.list_staged() {
            fence.grant_staged_file(std::path::PathBuf::from(&entry.path));
        }
        fence
    })
}
