//! The last LLM call of a Turn: wrap-up prompt, and what is kept when it still asks for tools.

use serde_json::Value;

use crate::agent::language::{describe_no_wrap_up, Language};
use crate::agent::llm::AssistantMessage;
use crate::agent::prompt::{read_skipped_tool_text, MAX_STEPS_PROMPT};
use crate::agent::session::{Session, Step, StepClock, EXTRA_APPEND_CONTENT, EXTRA_SKIPPED};

use super::types::{Terminal, TurnOutcome};

/// Attach the wrap-up prompt to the newest `Step`, so it is sent after it and stored with it.
pub(super) fn attach_wrap_up_prompt(session: &mut Session) {
    if let Some(step) = session.steps.last_mut() {
        step.extra
            .insert(EXTRA_APPEND_CONTENT.into(), Value::String(MAX_STEPS_PROMPT.into()));
    }
}

/// End the Turn after the last call. `msg`'s assistant `Step` is already pushed.
///
/// Tool calls in `msg` are not run: each gets a `tool` `Step` marked skipped, so the
/// history stays a valid call/result sequence. The Turn ends normally when the model
/// wrote text; with no text it ends as an error.
pub(super) fn end_turn_at_call_limit(
    session: &mut Session,
    msg: &AssistantMessage,
    wrote: bool,
) -> TurnOutcome {
    for call in &msg.tool_calls {
        let mut step = Step {
            role: "tool".into(),
            content: Some(read_skipped_tool_text().into()),
            tool_call_id: Some(call.id.clone()),
            tool_calls: None,
            name: Some(call.name.clone()),
            finish_reason: None,
            model: None,
            usage: None,
            reasoning_content: None,
            clock: StepClock::stamp_now(),
            extra: Default::default(),
        };
        step.extra.insert(EXTRA_SKIPPED.into(), Value::Bool(true));
        session.steps.push(step);
    }
    let text = msg.content.clone().unwrap_or_default();
    if !text.trim().is_empty() {
        return TurnOutcome {
            reply_text: text,
            terminal: Terminal::None,
            wrote,
        };
    }
    let reply = describe_no_wrap_up(Language::current());
    session.steps.push(Step {
        role: "assistant".into(),
        content: Some(reply.into()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
        finish_reason: None,
        model: None,
        usage: None,
        reasoning_content: None,
        clock: StepClock::stamp_now(),
        extra: Default::default(),
    });
    TurnOutcome {
        reply_text: reply.into(),
        terminal: Terminal::Error,
        wrote,
    }
}
