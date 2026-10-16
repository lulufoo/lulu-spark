//! Permission service: type + resource → allow / ask / deny.
//! v1 is `fs` only. `Always` scope is stored and ignored.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::agent::fence::{stored_path, PathFence};
use crate::agent::hard_deny::HardDenyList;

const GRANTS_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS permission_grants (
  grant_id TEXT PRIMARY KEY,
  perm_type TEXT NOT NULL,
  scope TEXT NOT NULL,
  session_id TEXT NOT NULL,
  resource TEXT
);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    Fs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionScope {
    Session,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    pub kind: PermissionKind,
    pub path: PathBuf,
    pub session_id: String,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRecord {
    pub kind: PermissionKind,
    pub scope: PermissionScope,
    pub session_id: String,
    pub resource: Option<PathBuf>,
}

impl PermissionKind {
    fn as_str(self) -> &'static str {
        "fs"
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "fs" => Some(Self::Fs),
            _ => None,
        }
    }
}

impl PermissionScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Always => "always",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "session" => Some(Self::Session),
            "always" => Some(Self::Always),
            _ => None,
        }
    }
}

/// Hard deny → deny. Workspace ∪ Stage → allow. Session-allow (not Always) →
/// allow unless this path was unstaged. Otherwise ask.
pub fn decide(
    req: &PermissionRequest,
    fence: &PathFence,
    records: &[PermissionRecord],
) -> PermissionDecision {
    if HardDenyList::spark_defaults().is_some_and(|list| list.contains(&req.path)) {
        return PermissionDecision::Deny;
    }
    if fence.allows_write(&req.path) {
        return PermissionDecision::Allow;
    }
    if path_is_revoked(req, records) {
        return PermissionDecision::Ask;
    }
    if has_session_allow(req, records) {
        return PermissionDecision::Allow;
    }
    PermissionDecision::Ask
}

pub fn load_records(db_path: &Path) -> Result<Vec<PermissionRecord>, String> {
    let conn = open_grants(db_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT perm_type, scope, session_id, resource FROM permission_grants ORDER BY rowid ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut records = Vec::new();
    for row in rows {
        let (kind, scope, session_id, resource) = row.map_err(|e| e.to_string())?;
        let Some(kind) = PermissionKind::parse(&kind) else {
            continue;
        };
        let Some(scope) = PermissionScope::parse(&scope) else {
            continue;
        };
        let resource = resource
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        records.push(PermissionRecord {
            kind,
            scope,
            session_id,
            resource,
        });
    }
    Ok(records)
}

pub fn store_record(db_path: &Path, record: &PermissionRecord) -> Result<(), String> {
    let conn = open_grants(db_path)?;
    conn.execute(
        "INSERT INTO permission_grants (grant_id, perm_type, scope, session_id, resource)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(grant_id) DO UPDATE SET
           perm_type = excluded.perm_type,
           scope = excluded.scope,
           session_id = excluded.session_id,
           resource = excluded.resource",
        params![
            record_id(record),
            record.kind.as_str(),
            record.scope.as_str(),
            record.session_id,
            record
                .resource
                .as_ref()
                .map(|p| stored_path(p.clone()).to_string_lossy().into_owned()),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn store_session_allow(db_path: &Path, session_id: &str) -> Result<(), String> {
    store_record(
        db_path,
        &PermissionRecord {
            kind: PermissionKind::Fs,
            scope: PermissionScope::Session,
            session_id: session_id.to_string(),
            resource: None,
        },
    )
}

pub fn store_revoke(db_path: &Path, session_id: &str, path: &Path) -> Result<(), String> {
    store_record(
        db_path,
        &PermissionRecord {
            kind: PermissionKind::Fs,
            scope: PermissionScope::Session,
            session_id: session_id.to_string(),
            resource: Some(path.to_path_buf()),
        },
    )
}

fn open_grants(db_path: &Path) -> Result<Connection, String> {
    if !db_path.is_file() {
        return Err("Session not found".into());
    }
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(GRANTS_SQL)
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

fn record_id(record: &PermissionRecord) -> String {
    let resource = record
        .resource
        .as_ref()
        .map(|p| stored_path(p.clone()).to_string_lossy().into_owned())
        .unwrap_or_default();
    format!(
        "{}:{}:{}:{resource}",
        record.kind.as_str(),
        record.scope.as_str(),
        record.session_id
    )
}

fn path_is_revoked(req: &PermissionRequest, records: &[PermissionRecord]) -> bool {
    records.iter().any(|record| {
        record.scope == PermissionScope::Session
            && record.kind == req.kind
            && record.session_id == req.session_id
            && record
                .resource
                .as_ref()
                .is_some_and(|path| same_resource(path, &req.path))
    })
}

fn has_session_allow(req: &PermissionRequest, records: &[PermissionRecord]) -> bool {
    records.iter().any(|record| {
        record.scope == PermissionScope::Session
            && record.kind == req.kind
            && record.session_id == req.session_id
            && record.resource.is_none()
    })
}

fn same_resource(stored: &Path, path: &Path) -> bool {
    stored_path(stored.to_path_buf()) == stored_path(path.to_path_buf())
}

#[cfg(test)]
#[path = "../unit-tests/agent/permission_tests.rs"]
mod tests;
