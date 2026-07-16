//! Host MVP Agent: Session / Tools / LLM (Loop arrives in t4).

pub mod llm;
pub mod session;
pub mod tools;

/// Mount point for the plan-assistant system prompt (body filled in t4).
pub const PLAN_ASSISTANT_SYSTEM_PROMPT: &str = "";

#[cfg(test)]
#[path = "../../unit-tests/services/agent/mod.rs"]
mod tests;
