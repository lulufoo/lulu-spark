//! Tests for `crate::test_support` helpers.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Barrier, mpsc};
use std::time::Duration;

use crate::config::settings::{
    self, DEFAULT_PROD_HTTP_PORT, DEFAULT_PROD_MCP_PORT, DEFAULT_SANDBOX_HTTP_PORT,
    DEFAULT_SANDBOX_MCP_PORT, prod_config_file_path,
};
use crate::test_support::{
    CONFIG_TEST_SERIAL, TestConfigEnv, TestSandbox, install_env_restore_probe,
    install_test_config_env_construction_probe,
};

fn file_content_digest(path: &std::path::Path) -> Option<u64> {
    match fs::read(path) {
        Ok(content) => {
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            Some(hasher.finish())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("read {} for digest: {error}", path.display()),
    }
}

#[test]
fn test_config_env_drop_restores_original_environment() {
    let original_home = std::env::var("HOME").ok();
    let original_sandbox = std::env::var("TestSandbox").ok();
    let original_sandbox_id = std::env::var("TestSandboxId").ok();
    let dir = tempfile::tempdir().expect("tmp");
    {
        let _env = TestConfigEnv::sandbox(dir.path(), "restore_order");
        assert_eq!(std::env::var("HOME").ok().as_deref(), dir.path().to_str());
        assert_eq!(std::env::var("TestSandbox").as_deref(), Ok("true"));
        assert_eq!(
            std::env::var("TestSandboxId").as_deref(),
            Ok("restore_order")
        );
    }
    assert_eq!(std::env::var("HOME").ok(), original_home);
    assert_eq!(std::env::var("TestSandbox").ok(), original_sandbox);
    assert_eq!(std::env::var("TestSandboxId").ok(), original_sandbox_id);
}

#[test]
fn restore_runs_while_config_lock_is_still_held() {
    let _probe = install_env_restore_probe(|| {
        assert!(matches!(
            CONFIG_TEST_SERIAL.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        ));
    });
    let dir = tempfile::tempdir().expect("tmp");
    drop(TestConfigEnv::sandbox(dir.path(), "drop_order"));
}

#[test]
fn probe_panic_still_completes_config_env_cleanup() {
    let original_home = std::env::var("HOME").ok();
    let original_sandbox = std::env::var("TestSandbox").ok();
    let original_sandbox_id = std::env::var("TestSandboxId").ok();
    let first_dir = tempfile::tempdir().expect("first tmp");

    {
        let _probe = install_env_restore_probe(|| panic!("probe panic"));
        let result = std::panic::catch_unwind(|| {
            drop(TestConfigEnv::sandbox(first_dir.path(), "probe_panic"));
        });
        assert!(result.is_err());
    }

    assert_eq!(std::env::var("HOME").ok(), original_home);
    assert_eq!(std::env::var("TestSandbox").ok(), original_sandbox);
    assert_eq!(std::env::var("TestSandboxId").ok(), original_sandbox_id);

    let second_dir = tempfile::tempdir().expect("second tmp");
    let _probe = install_env_restore_probe(|| {
        assert!(matches!(
            CONFIG_TEST_SERIAL.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        ));
    });
    drop(TestConfigEnv::sandbox(
        second_dir.path(),
        "after_probe_panic",
    ));
}

#[test]
fn construction_panic_restores_environment_and_releases_lock() {
    let original_home = std::env::var("HOME").ok();
    let original_sandbox = std::env::var("TestSandbox").ok();
    let original_sandbox_id = std::env::var("TestSandboxId").ok();
    let first_dir = tempfile::tempdir().expect("first tmp");

    let result = {
        let _restore_probe =
            install_env_restore_probe(|| panic!("construction restore probe panic"));
        let _construction_probe =
            install_test_config_env_construction_probe(|| panic!("construction hook panic"));
        std::panic::catch_unwind(|| {
            let _env = TestConfigEnv::sandbox(first_dir.path(), "construction_panic");
        })
    };

    let payload = result.expect_err("construction hook must panic");
    assert_eq!(
        payload.downcast_ref::<&str>(),
        Some(&"construction hook panic")
    );
    assert_eq!(std::env::var("HOME").ok(), original_home);
    assert_eq!(std::env::var("TestSandbox").ok(), original_sandbox);
    assert_eq!(std::env::var("TestSandboxId").ok(), original_sandbox_id);

    let second_dir = tempfile::tempdir().expect("second tmp");
    let _probe = install_env_restore_probe(|| {
        assert!(matches!(
            CONFIG_TEST_SERIAL.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        ));
    });
    drop(TestConfigEnv::sandbox(
        second_dir.path(),
        "after_construction_panic",
    ));
}

#[test]
fn competing_config_env_waits_until_environment_is_restored() {
    let original_home = std::env::var("HOME").ok();
    let first_dir = tempfile::tempdir().expect("first tmp");
    let second_dir = tempfile::tempdir().expect("second tmp");
    let first_path = first_dir.path().to_path_buf();
    let second_path = second_dir.path().to_path_buf();
    let restore_started = Arc::new(Barrier::new(2));
    let (first_ready_tx, first_ready_rx) = mpsc::channel();
    let (second_attempt_tx, second_attempt_rx) = mpsc::channel::<bool>();
    let (second_acquired_tx, second_acquired_rx) = mpsc::channel();
    let (second_restored_tx, second_restored_rx) = mpsc::channel();

    let first_barrier = Arc::clone(&restore_started);
    let first = std::thread::spawn(move || {
        let env = TestConfigEnv::sandbox(&first_path, "first");
        first_ready_tx.send(()).expect("signal first ready");
        let _probe = install_env_restore_probe(move || {
            first_barrier.wait();
            let second_was_blocked = second_attempt_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("wait for second lock outcome");
            assert!(second_was_blocked);
            assert!(matches!(
                second_acquired_rx.recv_timeout(Duration::from_millis(100)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ));
        });
        drop(env);
    });

    first_ready_rx.recv().expect("wait for first env");
    let second_barrier = Arc::clone(&restore_started);
    let second = std::thread::spawn(move || {
        second_barrier.wait();
        let was_blocked = matches!(
            CONFIG_TEST_SERIAL.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        );
        second_attempt_tx
            .send(was_blocked)
            .expect("signal second lock outcome");
        assert!(was_blocked);
        let env = TestConfigEnv::sandbox(&second_path, "second");
        let _ = second_acquired_tx.send(());
        drop(env);
        second_restored_tx
            .send(std::env::var("HOME").ok())
            .expect("record restored HOME");
    });

    first.join().expect("first thread");
    second.join().expect("second thread");
    assert_eq!(
        second_restored_rx.recv().expect("restored HOME"),
        original_home
    );
}

#[test]
fn test_config_env_reports_plane_paths_and_ports() {
    let home = tempfile::tempdir().expect("tmp");
    {
        let env = TestConfigEnv::prod(home.path());
        assert_eq!(
            env.config_file_path(),
            home.path()
                .join(".config/lulu-spark")
                .join("config.toml")
        );
        assert_eq!(
            env.ports(),
            (DEFAULT_PROD_HTTP_PORT, DEFAULT_PROD_MCP_PORT)
        );
        assert_eq!(std::env::var("HOME").ok().as_deref(), home.path().to_str());
        assert!(std::env::var("TestSandbox").is_err());
        assert!(std::env::var("TestSandboxId").is_err());
    }
    {
        let env = TestConfigEnv::sandbox(home.path(), "api");
        assert_eq!(
            env.config_file_path(),
            home.path()
                .join(".config/lulu-spark-sandbox/api")
                .join("config.toml")
        );
        assert_eq!(
            env.ports(),
            (DEFAULT_SANDBOX_HTTP_PORT, DEFAULT_SANDBOX_MCP_PORT)
        );
    }
}

#[test]
fn sandbox_config_writes_leave_machine_config_content_unchanged() {
    crate::test_support::with_config_test_serial(|| {
        let machine_config_path = prod_config_file_path();
        let before = file_content_digest(&machine_config_path);

        {
            let sandbox = TestSandbox::new();
            let sandbox_config_path = sandbox.config_file_path();
            let (http_port, mcp_port) = sandbox.ports();
            settings::write_test_config_with_cache(
                &sandbox_config_path,
                sandbox.config_dir(),
                Some(sandbox.config_dir()),
                Some(&sandbox.config_dir().join("cache")),
                http_port,
                mcp_port,
            )
            .expect("write explicit sandbox config");
        }

        assert_eq!(file_content_digest(&machine_config_path), before);
    });
}

#[test]
fn nested_test_config_env_restores_outer_then_machine_context() {
    crate::test_support::with_config_test_serial(|| {
        let original_home = std::env::var("HOME").ok();
        let original_sandbox = std::env::var("TestSandbox").ok();
        let original_sandbox_id = std::env::var("TestSandboxId").ok();
        let machine_config_path = settings::prod_config_file_path();
        let outer_dir = tempfile::tempdir().expect("outer tmp");
        let inner_dir = tempfile::tempdir().expect("inner tmp");

        let outer = TestConfigEnv::sandbox(outer_dir.path(), "outer");
        assert_eq!(
            settings::config_file_path().expect("outer config"),
            outer.config_file_path()
        );
        {
            let inner = TestConfigEnv::sandbox(inner_dir.path(), "inner");
            assert_eq!(
                settings::config_file_path().expect("inner config"),
                inner.config_file_path()
            );
        }
        assert_eq!(
            settings::config_file_path().expect("outer config restored"),
            outer.config_file_path()
        );
        drop(outer);

        // Real-user HOME Prod must not adopt a machine plane in lib tests.
        assert!(
            settings::runtime_mode().is_err(),
            "machine restore clears the runtime plane"
        );
        assert_eq!(settings::prod_config_file_path(), machine_config_path);
        assert_eq!(std::env::var("HOME").ok(), original_home);
        assert_eq!(std::env::var("TestSandbox").ok(), original_sandbox);
        assert_eq!(std::env::var("TestSandboxId").ok(), original_sandbox_id);
    });
}
