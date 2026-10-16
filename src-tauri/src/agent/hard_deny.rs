//! Shared hard-deny path list. User allow and Stage grants cannot override it.
//! Seatbelt can reuse these roots later; this module does not talk to the OS sandbox.

use std::path::{Path, PathBuf};

use crate::agent::fence::{in_any, stored_path};
use crate::config::paths;

#[derive(Debug, Clone)]
pub struct HardDenyList {
    roots: Vec<PathBuf>,
}

impl HardDenyList {
    /// Spark v1 roots: `SPARK_DATA_DIR` and the workbench `.git`.
    pub fn spark_defaults() -> Option<Self> {
        let wb = stored_path(paths::spark_root().ok()?);
        let data = stored_path(paths::runtime_data_dir().ok()?);
        let roots = vec![data, wb.join(".git")];
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
