//! Thin facade: Binding / Shell / Turn live at top-level agent modules.
//! Prefer `crate::services::agent::{binding, shell, turn}` for new code.
//! Existing `r#loop::` call sites keep compiling through these re-exports.

pub use crate::services::agent::session::{
    validate_binding, Binding, BindingStateSummary, SetError,
};

pub use crate::services::agent::binding::*;
pub use crate::services::agent::shell::*;
pub use crate::services::agent::turn::*;
