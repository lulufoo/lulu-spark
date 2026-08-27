//! Concatenated Agent Loop owner sources for scan-style tests.

pub const LOOP_SRC: &str = concat!(
    include_str!("../../../services/agent/loop/mod.rs"),
    include_str!("../../../services/agent/loop/types.rs"),
    include_str!("../../../services/agent/loop/flights.rs"),
    include_str!("../../../services/agent/loop/binding.rs"),
    include_str!("../../../services/agent/loop/history.rs"),
    include_str!("../../../services/agent/loop/turn.rs"),
    include_str!("../../../services/agent/loop/shell.rs"),
);
