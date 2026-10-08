use std::fs;
use std::path::PathBuf;

fn source(rel: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|err| panic!("{rel}: {err}"))
}

#[test]
fn command_returns_host_snapshot() {
    let snap = super::get_package_snapshot();
    assert_eq!(snap, crate::host::snapshot());
}

#[test]
fn command_is_declared_and_registered() {
    assert!(
        source("src/commands/mod.rs").contains("pub mod package"),
        "commands/mod.rs must declare package"
    );
    assert!(
        source("src/lib.rs").contains("commands::package::get_package_snapshot"),
        "lib.rs generate_handler must register get_package_snapshot"
    );
}

#[test]
fn production_module_declares_the_test_tree() {
    let prod = include_str!("../../commands/package.rs");
    assert!(
        prod.contains("#[path = \"../unit-tests/commands/package.rs\"]"),
        "production file only declares that tests exist"
    );
    assert!(!prod.contains("#[test]"), "tests belong in the test tree");
}
