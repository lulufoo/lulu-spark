//! On-disk layout of one TestSandbox: `<HOME>/.config/lulu-spark-sandbox/<id>/`
//! (`config.toml` + `cache/`), plus fake prod roots kept outside it.

use std::path::{Path, PathBuf};

use crate::config::settings;

/// Create `cache/{spark-knowledge,knowledge-clones}` under one sandbox dir.
pub(super) fn prepare_sandbox_roots(sandbox_dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let cache = sandbox_dir.join("cache");
    let wb = cache.join("spark-knowledge");
    let clones = cache.join("knowledge-clones");
    std::fs::create_dir_all(&wb).expect("mkdir wb");
    std::fs::create_dir_all(&clones).expect("mkdir knowledge-clones");
    std::fs::create_dir_all(&cache).expect("mkdir cache");
    (wb, clones, cache)
}

pub(super) fn write_sandbox_config(
    config_path: &Path,
    wb: &Path,
    clones: &Path,
    cache: &Path,
    ports: (u16, u16),
) {
    settings::write_test_config_with_cache(
        config_path,
        wb,
        Some(clones),
        Some(cache),
        ports.0,
        ports.1,
    )
    .expect("write sandbox config");
}

fn path_under_prod_root(path: &Path, root: &Path) -> bool {
    if path == root {
        return true;
    }
    path.starts_with(root.join(""))
}

/// Error returned when a test write targets a prod root path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProdPathGuardError {
    pub path: PathBuf,
    pub prod_root: PathBuf,
}

impl std::fmt::Display for ProdPathGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "test path {} must not touch prod root {}",
            self.path.display(),
            self.prod_root.display()
        )
    }
}

impl std::error::Error for ProdPathGuardError {}

pub(super) fn assert_path_not_under_prod_roots(
    path: &Path,
    prod_wb: &Path,
    prod_clones: &Path,
    prod_cache: &Path,
) -> Result<(), ProdPathGuardError> {
    for root in [prod_wb, prod_clones, prod_cache] {
        if path_under_prod_root(path, root) {
            return Err(ProdPathGuardError {
                path: path.to_path_buf(),
                prod_root: root.to_path_buf(),
            });
        }
    }
    Ok(())
}

/// Fake prod config + roots at `<HOME>/prod-*`, outside every sandbox dir.
pub(super) fn write_fake_prod_config(home: &Path) {
    let prod_config_path = home
        .join(".config")
        .join("lulu-spark")
        .join(settings::PROD_CONFIG_FILE_NAME);
    let prod_wb = home.join("prod-wb");
    let prod_clones = home.join("prod-knowledge-clones");
    let prod_cache = home.join("prod-cache");
    std::fs::create_dir_all(&prod_wb).expect("mkdir prod wb");
    std::fs::create_dir_all(&prod_clones).expect("mkdir prod knowledge-clones");
    std::fs::create_dir_all(&prod_cache).expect("mkdir prod cache");
    settings::write_test_config_with_cache(
        &prod_config_path,
        &prod_wb,
        Some(&prod_clones),
        Some(&prod_cache),
        settings::DEFAULT_PROD_HTTP_PORT,
        settings::DEFAULT_PROD_MCP_PORT,
    )
    .expect("write fake prod config");
}

#[path = "../unit-tests/test_support/sandbox_layout.rs"]
mod tests;
