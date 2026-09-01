//! Concatenated Agent Loop owner sources for scan-style tests.
//! Owners live at binding / shell / turn; `loop/` is a thin re-export facade.

pub const LOOP_SRC: &str = concat!(
    include_str!("../../../services/agent/loop/mod.rs"),
    include_str!("../../../services/agent/binding/mod.rs"),
    include_str!("../../../services/agent/shell/mod.rs"),
    include_str!("../../../services/agent/turn/mod.rs"),
    include_str!("../../../services/agent/turn/types.rs"),
    include_str!("../../../services/agent/turn/flights.rs"),
    include_str!("../../../services/agent/turn/history.rs"),
    include_str!("../../../services/agent/turn/run.rs"),
);
