//! Per-workspace document tables: name rule, DDL template, creation.
//!
//! The table name is spliced into SQL, so every name passes
//! [`validate_doc_table_name`] first. Names are never taken from callers:
//! they are `doc_` + a Host-generated workspace id.

use rusqlite::Connection;

use super::db::doc_table_columns;

pub const DOC_TABLE_PREFIX: &str = "doc_";
const MAX_ID_LEN: usize = 48;

fn is_id_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
}

/// Workspace ids are `[a-z0-9_]`, non-empty, short.
pub fn validate_workspace_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > MAX_ID_LEN || !id.chars().all(is_id_char) {
        return Err(format!("invalid workspace id: {id:?}"));
    }
    Ok(())
}

/// `doc_` + a valid workspace id.
pub fn validate_doc_table_name(table: &str) -> Result<(), String> {
    match table.strip_prefix(DOC_TABLE_PREFIX) {
        Some(id) if validate_workspace_id(id).is_ok() => Ok(()),
        _ => Err(format!("invalid document table name: {table:?}")),
    }
}

pub fn doc_table_for_id(workspace_id: &str) -> Result<String, String> {
    validate_workspace_id(workspace_id)?;
    Ok(format!("{DOC_TABLE_PREFIX}{workspace_id}"))
}

/// Runtime form of the template used by the migration step.
fn doc_table_ddl(table: &str) -> Result<String, String> {
    validate_doc_table_name(table)?;
    Ok(format!(
        "CREATE TABLE {table} ({});\nCREATE INDEX {table}_raw_path ON {table}(raw_path);\n",
        doc_table_columns!()
    ))
}

/// Create the (empty) document table for a workspace. Caller owns the transaction.
pub fn create_doc_table(conn: &Connection, table: &str) -> Result<(), String> {
    conn.execute_batch(&doc_table_ddl(table)?)
        .map_err(|e| e.to_string())
}
