//! Daily snapshot of `spark-data.sqlite`.
//!
//! The database is the only copy of comments, highlights, tags and the knowledge
//! registry, so the app keeps a few dated snapshots next to it. `VACUUM INTO` writes a
//! consistent copy while the database is open (a plain file copy is not safe in WAL
//! mode). Restore by hand: quit the app and replace `spark-data.sqlite` with a snapshot.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use rusqlite::Connection;

use crate::config::paths;

use super::db::open;

pub const BACKUP_DIR: &str = "_backup";
const KEEP: usize = 7;
const PREFIX: &str = "spark-data-";
const SUFFIX: &str = ".sqlite";

pub fn backup_dir() -> Result<PathBuf, String> {
    Ok(paths::runtime_data_dir().map_err(|e| format!("{e:?}"))?.join(BACKUP_DIR))
}

fn snapshot_name(day: &str) -> String {
    format!("{PREFIX}{day}{SUFFIX}")
}

fn is_snapshot_name(name: &str) -> bool {
    name.strip_prefix(PREFIX)
        .and_then(|rest| rest.strip_suffix(SUFFIX))
        .is_some_and(|day| day.len() == 8 && day.bytes().all(|b| b.is_ascii_digit()))
}

/// Delete the oldest snapshots beyond `keep`. Day names sort in date order.
fn prune_snapshots(dir: &Path, keep: usize) -> Result<(), String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_snapshot_name(name))
        .collect();
    names.sort();
    let excess = names.len().saturating_sub(keep);
    for name in names.into_iter().take(excess) {
        fs::remove_file(dir.join(name)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Write `spark-data-<day>.sqlite` into `dir` unless that day already has one, then
/// keep only the newest `keep`. Returns the new snapshot's path.
pub fn backup_for_day(
    conn: &Connection,
    dir: &Path,
    day: &str,
    keep: usize,
) -> Result<Option<PathBuf>, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let target = dir.join(snapshot_name(day));
    if target.exists() {
        return Ok(None);
    }
    // Written under a temp name first: a half-written file must never look like a snapshot.
    let partial = dir.join(format!("{}.partial", snapshot_name(day)));
    let _ = fs::remove_file(&partial);
    conn.execute("VACUUM INTO ?1", [partial.to_string_lossy().as_ref()])
        .map_err(|e| e.to_string())?;
    fs::rename(&partial, &target).map_err(|e| e.to_string())?;
    prune_snapshots(dir, keep)?;
    Ok(Some(target))
}

/// Today's snapshot of the production database (local date), once per day.
pub fn backup_daily() -> Result<Option<PathBuf>, String> {
    let day = Local::now().format("%Y%m%d").to_string();
    backup_for_day(&open()?, &backup_dir()?, &day, KEEP)
}

#[cfg(test)]
#[path = "../../unit-tests/services/spark_data_backup.rs"]
mod tests;
