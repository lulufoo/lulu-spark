//! The `workspace` registry: one row per workspace, pointing at its document table.
//!
//! `id` and `doc_table` never change after creation (a trigger enforces it);
//! only `name`, `view_mode`, `icon`, `sort_index` and `disabled_at` are editable. A workspace is never deleted:
//! it is disabled (hidden, data kept) and can be enabled again. The default workspace
//! cannot be disabled.

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::doc_table::{create_doc_table, doc_table_for_id, validate_doc_table_name};
use crate::services::id::random_alnum6;

/// Id of the built-in default workspace (the former Notes).
pub const DEFAULT_WORKSPACE_ID: &str = "default";
/// Id of the seeded workspace that took over the former Knowledge.
pub const KNOWLEDGE_WORKSPACE_ID: &str = "knowledge";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Directory,
    Timeline,
}

impl ViewMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ViewMode::Directory => "directory",
            ViewMode::Timeline => "timeline",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "directory" => Ok(ViewMode::Directory),
            "timeline" => Ok(ViewMode::Timeline),
            other => Err(format!("unknown view_mode: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRow {
    pub id: String,
    pub name: String,
    pub doc_table: String,
    pub view_mode: ViewMode,
    pub is_default: bool,
    pub created_at: Option<String>,
    /// `None` = active; a timestamp = disabled (hidden from lists and search, data kept).
    pub disabled_at: Option<String>,
    /// Id of a built-in icon; `None` = the default icon. The frontend owns the icon list.
    pub icon: Option<String>,
    /// Position in the user's order; ignored on insert (new rows go last).
    pub sort_index: i64,
}

const COLUMNS: &str = "id, name, doc_table, view_mode, is_default, created_at, disabled_at, icon, sort_index";
const INSERT_COLUMNS: &str = "id, name, doc_table, view_mode, is_default, created_at";

fn map_row(row: &Row<'_>) -> rusqlite::Result<WorkspaceRow> {
    let mode: String = row.get(3)?;
    Ok(WorkspaceRow {
        id: row.get(0)?,
        name: row.get(1)?,
        doc_table: row.get(2)?,
        view_mode: ViewMode::parse(&mode).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, e.into())
        })?,
        is_default: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        disabled_at: row.get(6)?,
        icon: row.get(7)?,
        sort_index: row.get(8)?,
    })
}

fn clean_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("workspace name must not be empty".to_string());
    }
    Ok(name)
}

fn mint_id(conn: &Connection) -> Result<String, String> {
    for _ in 0..8 {
        let id = random_alnum6();
        if get_workspace(conn, &id)?.is_none() {
            return Ok(id);
        }
    }
    Err("could not mint a unique workspace id".to_string())
}

/// Register a workspace and create its document table in one transaction.
/// The caller supplies only a display name; `id` and table name are Host-made.
pub fn create_workspace(
    conn: &Connection,
    name: &str,
    view_mode: Option<ViewMode>,
) -> Result<WorkspaceRow, String> {
    let name = clean_name(name)?;
    let mode = view_mode.unwrap_or(ViewMode::Directory);
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let id = mint_id(&tx)?;
    let doc_table = doc_table_for_id(&id)?;
    tx.execute(
        &format!(
            "INSERT INTO workspace({INSERT_COLUMNS}, sort_index)
             VALUES (?1, ?2, ?3, ?4, 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                     (SELECT coalesce(max(sort_index) + 1, 0) FROM workspace))"
        ),
        params![id, name, doc_table, mode.as_str()],
    )
    .map_err(|e| e.to_string())?;
    create_doc_table(&tx, &doc_table)?;
    let row = get_workspace(&tx, &id)?.ok_or("workspace row missing after insert")?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(row)
}

pub fn get_workspace(conn: &Connection, id: &str) -> Result<Option<WorkspaceRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM workspace WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// The workspace named `name`, disabled or not (the name stays taken after disabling).
pub fn get_workspace_by_name(conn: &Connection, name: &str) -> Result<Option<WorkspaceRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM workspace WHERE name = ?1"),
        params![name],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn list_where(conn: &Connection, condition: &str) -> Result<Vec<WorkspaceRow>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM workspace WHERE {condition} ORDER BY sort_index, id"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Active workspaces in the user's order (`sort_index`).
pub fn list_workspaces(conn: &Connection) -> Result<Vec<WorkspaceRow>, String> {
    list_where(conn, "disabled_at IS NULL")
}

/// Disabled workspaces, same order. Only the Settings "Disabled" section reads them.
pub fn list_disabled_workspaces(conn: &Connection) -> Result<Vec<WorkspaceRow>, String> {
    list_where(conn, "disabled_at IS NOT NULL")
}

/// Every workspace, disabled ones included. For lookups by document id, which must keep
/// working for a disabled workspace.
pub fn list_all_workspaces(conn: &Connection) -> Result<Vec<WorkspaceRow>, String> {
    list_where(conn, "1 = 1")
}

/// `false` when the workspace does not exist.
pub fn rename_workspace(conn: &Connection, id: &str, name: &str) -> Result<bool, String> {
    let name = clean_name(name)?;
    conn.execute("UPDATE workspace SET name = ?2 WHERE id = ?1", params![id, name])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}

/// `false` when the workspace does not exist.
pub fn set_workspace_view_mode(
    conn: &Connection,
    id: &str,
    view_mode: ViewMode,
) -> Result<bool, String> {
    conn.execute(
        "UPDATE workspace SET view_mode = ?2 WHERE id = ?1",
        params![id, view_mode.as_str()],
    )
    .map(|changed| changed > 0)
    .map_err(|e| e.to_string())
}

/// Set the icon id, or clear it with `None` (back to the default icon).
/// `false` when the workspace does not exist.
pub fn set_workspace_icon(conn: &Connection, id: &str, icon: Option<&str>) -> Result<bool, String> {
    conn.execute("UPDATE workspace SET icon = ?2 WHERE id = ?1", params![id, icon])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}

/// Disable a workspace: it disappears from lists and search, its documents and table stay.
/// Disabling twice keeps the first timestamp. Refuses the default workspace.
/// `false` when it does not exist.
pub fn disable_workspace(conn: &Connection, id: &str) -> Result<bool, String> {
    let Some(row) = get_workspace(conn, id)? else {
        return Ok(false);
    };
    if row.is_default {
        return Err("the default workspace cannot be disabled".to_string());
    }
    conn.execute(
        "UPDATE workspace SET disabled_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ?1 AND disabled_at IS NULL",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(true)
}

/// Enable a disabled workspace again. `false` when it does not exist.
pub fn enable_workspace(conn: &Connection, id: &str) -> Result<bool, String> {
    conn.execute("UPDATE workspace SET disabled_at = NULL WHERE id = ?1", params![id])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}

/// Undo a `create_workspace` whose follow-up steps failed: drops the (still empty) projects,
/// the document table and the registry row together. Not a user operation: refuses the
/// default workspace and any workspace that holds documents. `false` when it did not exist.
pub fn discard_new_workspace(conn: &Connection, id: &str) -> Result<bool, String> {
    let Some(row) = get_workspace(conn, id)? else {
        return Ok(false);
    };
    if row.is_default {
        return Err("the default workspace cannot be discarded".to_string());
    }
    // `doc_table` came out of the registry, whose CHECK ties it to a valid id.
    validate_doc_table_name(&row.doc_table)?;
    let docs: i64 = conn
        .query_row(&format!("SELECT count(*) FROM {}", row.doc_table), [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if docs > 0 {
        return Err(format!("workspace {id} still has {docs} document(s)"));
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM project WHERE workspace_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.execute_batch(&format!("DROP TABLE {}", row.doc_table))
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM workspace WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Put every workspace in the order of `ids`. `ids` must name every workspace (disabled ones
/// included) exactly once; anything else is refused and nothing changes.
pub fn reorder_workspaces(conn: &Connection, ids: &[String]) -> Result<(), String> {
    let known: Vec<String> = list_all_workspaces(conn)?.into_iter().map(|w| w.id).collect();
    ensure_same_set(&known, ids, "workspace")?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE workspace SET sort_index = ?2 WHERE id = ?1",
            params![id, index as i64],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

/// `Err` unless `given` holds exactly the entries of `known`, each once.
pub(super) fn ensure_same_set(known: &[String], given: &[String], what: &str) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for item in given {
        if !seen.insert(item.as_str()) {
            return Err(format!("duplicate {what} in order: {item}"));
        }
        if !known.iter().any(|k| k == item) {
            return Err(format!("unknown {what} in order: {item}"));
        }
    }
    if let Some(missing) = known.iter().find(|k| !seen.contains(k.as_str())) {
        return Err(format!("order is missing {what}: {missing}"));
    }
    Ok(())
}
