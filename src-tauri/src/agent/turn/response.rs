//! Copy the response-level fields of one LLM response onto the `Step` that carries it.

use crate::agent::llm::AssistantMessage;
use crate::agent::session::Step;

/// Fill `reasoning_content`, `finish_reason`, `model` and `usage` from `msg`.
///
/// Call it for the `Step` that carries a response's tool calls or text. When a
/// response has neither (empty body), the synthetic fallback `Step` stands in
/// for it: its text is ours, but these fields belong to the model's response.
pub(super) fn with_response(mut step: Step, msg: &AssistantMessage) -> Step {
    step.finish_reason = msg.finish_reason.clone();
    step.model = msg.model.clone();
    step.usage = msg.usage.clone();
    step.reasoning_content = msg.reasoning_content.clone().filter(|text| !text.is_empty());
    step
}

/// Reply for a response that ended with no text and no tool calls.
///
/// `wrote` is true once a mutating tool succeeded in this Turn; `tool_rounds` counts
/// the tool rounds already run. A model that stops silently after writing must not be
/// reported as "nothing was written".
pub(super) fn empty_reply_text(wrote: bool, tool_rounds: usize) -> &'static str {
    if wrote {
        "已执行写入，但模型没有给出最终回复。"
    } else if tool_rounds > 0 {
        "已执行工具，未写入，但模型没有给出最终回复。"
    } else {
        "模型响应为空，未执行任何写入。"
    }
}
