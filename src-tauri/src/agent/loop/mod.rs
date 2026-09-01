//! Thin facade: Binding / Shell / Turn live at top-level agent modules.
//! Prefer `crate::agent::{binding, shell, turn}` for new code.
//! Existing `r#loop::` call sites keep compiling through these re-exports.

pub use crate::agent::session::{
    validate_binding, Binding, BindingStateSummary, SetError,
};

pub use crate::agent::binding::*;
pub use crate::agent::shell::*;
pub use crate::agent::turn::*;
