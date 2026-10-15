//! JSON-array TEXT columns: stored as JSON text, handled as `serde_json::Value`.

use rusqlite::types::Type;
use rusqlite::Row;
use serde_json::Value;

pub fn write_array(value: &Value) -> Result<String, String> {
    if !value.is_array() {
        return Err(format!("JSON column expects an array, got {value}"));
    }
    serde_json::to_string(value).map_err(|e| e.to_string())
}

pub fn read_array(row: &Row<'_>, idx: usize) -> rusqlite::Result<Value> {
    let text: String = row.get(idx)?;
    serde_json::from_str(&text)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(idx, Type::Text, Box::new(e)))
}
