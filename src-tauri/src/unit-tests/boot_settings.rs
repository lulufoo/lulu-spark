//! Tests for `boot_settings`: startup resolves the runtime mode before config.

use super::resolve_boot_settings;
use crate::config::settings::RuntimeMode;
use crate::test_support::{TestConfigEnv, TestSandbox};

#[test]
fn resolve_boot_settings_uses_sandbox_mode_and_ports() {
    let sandbox = TestSandbox::new();
    let boot = resolve_boot_settings().expect("boot");
    assert!(matches!(boot.mode, RuntimeMode::Sandbox { .. }));
    assert_eq!((boot.http_port, boot.mcp_port), sandbox.ports());
    assert_eq!(boot.settings.cache_dir, sandbox.sandbox_dir().join("cache"));
}

#[test]
fn resolve_boot_settings_refuses_unreadable_mode() {
    let home = tempfile::tempdir().expect("tmp");
    for (flag, id) in [(Some("yes"), Some("abc")), (Some("1"), None)] {
        let _env = TestConfigEnv::raw(home.path(), flag, id);
        assert!(resolve_boot_settings().is_err());
    }
}

#[test]
fn resolve_boot_settings_refuses_sandbox_without_config() {
    let home = tempfile::tempdir().expect("tmp");
    let _env = TestConfigEnv::sandbox(home.path(), "no-config");
    let err = resolve_boot_settings().err().expect("must refuse");
    assert!(err.contains("sandbox settings"), "{err}");
}
