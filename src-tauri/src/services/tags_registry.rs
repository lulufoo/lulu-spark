//! Tags registry: the `tag` table with reference counts counted from the notes.

use serde_json::{json, Value};

use crate::services::spark_data::{self, read_tag_registry};

pub fn empty_registry() -> Value {
    json!({ "keys": {} })
}

/// `{ "keys": { key: { value, refs } } }`; empty when the database cannot be read.
pub fn read_registry() -> Value {
    spark_data::open()
        .and_then(|conn| read_tag_registry(&conn))
        .unwrap_or_else(|_| empty_registry())
}
