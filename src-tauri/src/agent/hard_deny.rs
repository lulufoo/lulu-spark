//! Shared hard-deny path list. User allow and Stage grants cannot override it.
//! Seatbelt can reuse these roots later; this module does not talk to the OS sandbox.

use std::path::{Path, PathBuf};

use crate::agent::fence::{in_any, stored_path};
use crate::config::paths;
use crate::services::sediment_kb;

#[derive(Debug, Clone)]
pub struct HardDenyList {
    roots: Vec<PathBuf>,
}

impl HardDenyList {
    /// Spark v1 roots: `SPARK_DATA_DIR`, workbench `.git`, and each knowledge clone `.git`.
    pub fn spark_defaults() -> Option<Self> {
        let wb = stored_path(paths::spark_root().ok()?);
        let data = stored_path(paths::runtime_data_dir().ok()?);
        let knowledge = stored_path(paths::knowledge_root().ok()?);
        let mut roots = vec![data, wb.join(".git")];
        if let Ok(repos) = sediment_kb::load_repos() {
            for repo in repos.repos {
                if repo.full_name.is_empty() {
                    continue;
                }
                let dir = stored_path(knowledge.join(&repo.full_name));
                roots.push(dir.join(".git"));
            }
        }
        Some(Self { roots })
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// True when `path` is under a hard-deny root after resolving `../` and symlinks.
    pub fn contains(&self, path: &Path) -> bool {
        in_any(path, &self.roots)
    }
}

#[cfg(test)]
#[path = "../unit-tests/agent/hard_deny_tests.rs"]
mod tests;
