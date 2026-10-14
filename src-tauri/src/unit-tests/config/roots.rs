//! Tests for `config::roots`: notes/knowledge read the process plane only.

use std::path::Path;

use super::{knowledge_root_string, notes_root_path};
use crate::config::settings::default_cache_dir;
use crate::test_support::{TestConfigEnv, TestSandbox};

#[test]
fn notes_root_path_uses_sandbox_plane_not_prod_cache() {
    let sandbox = TestSandbox::new();
    let notes = notes_root_path(Path::new("."));
    assert_eq!(notes, sandbox.data_dir().join("notes"));
    assert!(notes.starts_with(sandbox.sandbox_dir()));
    assert!(!notes.starts_with(default_cache_dir()));
}

#[test]
fn knowledge_root_string_uses_sandbox_plane_not_prod_cache() {
    let sandbox = TestSandbox::new();
    let knowledge = knowledge_root_string(Path::new("."));
    let expected = sandbox.data_dir().join("knowledge");
    assert_eq!(knowledge, expected.to_string_lossy());
    assert!(!knowledge.starts_with(&*default_cache_dir().to_string_lossy()));
}

#[test]
#[should_panic(expected = "runtime plane not initialized")]
fn notes_root_path_panics_when_plane_missing_after_invalid_mode() {
    let home = tempfile::tempdir().expect("tmp");
    let _env = TestConfigEnv::raw(home.path(), Some("yes"), Some("abc"));
    let prod_notes = default_cache_dir().join("data").join("notes");
    let got = notes_root_path(Path::new("."));
    // Must not reach here; if it did, falling back to prod would be the bug.
    assert_ne!(got, prod_notes);
}
