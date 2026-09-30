//! Business groups registered with the catalog factory.

pub mod global;
pub mod knowledge;
pub mod notes;

pub const ALL_GROUP_IDS: &[&str] = &[
    notes::GROUP_ID,
    knowledge::GROUP_ID,
    global::GROUP_ID,
];
