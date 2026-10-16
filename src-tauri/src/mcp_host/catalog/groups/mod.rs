//! Business groups registered with the catalog factory.

pub mod docs;
pub mod global;

pub const ALL_GROUP_IDS: &[&str] = &[docs::GROUP_ID, global::GROUP_ID];
