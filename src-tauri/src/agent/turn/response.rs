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
