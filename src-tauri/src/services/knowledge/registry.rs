//! Filesystem side of the knowledge registry: walk clones and reconcile `knowledge_doc`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use regex::Regex;

use crate::config::roots::knowledge_root_string;
use crate::services::sediment_kb;
use crate::services::spark_data::{self, directory_id_for_folder, ReconcileStats};

use super::name_is_hidden;

fn walk_md(dir: &Path, base: &Path, hide: &[Regex], out: &mut HashSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name_is_hidden(&name, hide) {
            continue;
        }
        if path.is_dir() {
            walk_md(&path, base, hide, out);
        } else if path.is_file() && name.ends_with(".md") {
            if let Ok(rel) = path.strip_prefix(base) {
                out.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

/// Reconcile one clone directory (`kb_root/<repo_dir>`) with its records.
///
/// A clone directory that does not exist right now changes nothing: it may be a
/// clone in progress or an unmounted disk, not proof that every file was deleted.
/// Files and folders matching a hide rule are neither registered nor marked missing.
pub fn scan_repo_dir(
    conn: &rusqlite::Connection,
    kb_root: &Path,
    repo_dir: &str,
    hide: &[Regex],
) -> Result<ReconcileStats, String> {
    let local = kb_root.join(repo_dir);
    if !local.is_dir() {
        return Ok(ReconcileStats::default());
    }
    let mut found = HashSet::new();
    walk_md(&local, &local, hide, &mut found);
    let directory_id = directory_id_for_folder(conn, repo_dir)?;
    spark_data::reconcile_knowledge_directory(
        conn,
        &directory_id,
        &found,
        |rel| local.join(rel).is_file(),
        |rel| rel.split('/').any(|part| name_is_hidden(part, hide)),
    )
}

/// Scan every registered knowledge repo (or only `force_repo`, an `owner/name`).
pub fn scan_registered_repos(repo_root: &Path, force_repo: Option<&str>) -> Result<(), String> {
    let kb_root = std::path::PathBuf::from(knowledge_root_string(repo_root));
    let repos = sediment_kb::load_repos().map_err(|e| e.to_string())?;
    let hide = super::compiled_hide_regexes();
    let conn = spark_data::open()?;
    for repo in repos.repos {
        let folder = repo.full_name.as_str();
        if force_repo.is_some_and(|force| force != folder && force.split('/').next_back() != Some(folder))
        {
            continue;
        }
        scan_repo_dir(&conn, &kb_root, folder, &hide)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/registry.rs"]
mod tests;
