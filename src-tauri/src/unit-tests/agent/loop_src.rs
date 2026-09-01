//! Concatenated Agent Loop owner sources for scan-style tests.
//! Owners live at binding / shell / turn; `loop/` is a thin re-export facade.

pub const LOOP_SRC: &str = concat!(
    include_str!("../../agent/loop/mod.rs"),
    include_str!("../../agent/binding/mod.rs"),
    include_str!("../../agent/shell/mod.rs"),
    include_str!("../../agent/turn/mod.rs"),
    include_str!("../../agent/turn/types.rs"),
    include_str!("../../agent/turn/flights.rs"),
    include_str!("../../agent/turn/history.rs"),
    include_str!("../../agent/turn/run.rs"),
);
