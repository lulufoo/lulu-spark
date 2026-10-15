//! Host MVP Agent: Session / Tools / LLM / Binding / Shell / Turn.

use std::path::Path;

pub mod binding;
pub mod context;
pub mod diagnostics;
pub mod engine_router;
pub mod fence;
pub mod language;
pub mod llm;
pub mod mcp;
pub mod progress;
pub mod prompt;
pub mod r#loop;
pub mod runtime;
pub mod session;
pub mod shell;
pub mod tools;
pub mod turn;
pub mod workspace;

pub use prompt::SPARK_HOST_SYSTEM_PROMPT;

pub const SESSION_WORKSPACE_DIR_PLACEHOLDER: &str = "{session_workspace_dir}";

/// Fill the Host file-directory placeholder for one turn. Scratch stays a
/// placeholder when this turn has no session scratch.
pub fn fill_host_file_dirs(prompt: &str, scratch: Option<&Path>) -> String {
    match scratch {
        Some(scratch) => prompt.replace(
            SESSION_WORKSPACE_DIR_PLACEHOLDER,
            scratch.to_string_lossy().as_ref(),
        ),
        None => prompt.to_string(),
    }
}

#[cfg(test)]
#[path = "../unit-tests/agent/mod.rs"]
mod tests;
