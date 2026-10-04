//! Tests for `crate::test_support` helpers.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Barrier, mpsc};
use std::time::Duration;
use std::time::SystemTime;

use crate::config::settings::{
    self, DEFAULT_PROD_HTTP_PORT, DEFAULT_PROD_MCP_PORT, DEFAULT_SANDBOX_HTTP_PORT,
    DEFAULT_SANDBOX_MCP_PORT, default_cache_dir, load_prod_settings, prod_config_file_path,
};
use crate::test_support::{
    CONFIG_TEST_SERIAL, TestConfigEnv, TestSandbox, install_env_restore_probe,
    install_test_config_env_construction_probe, with_sandbox_notes,
};

fn prod_cache_dir_mtime() -> Option<SystemTime> {
    let path = default_cache_dir();
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn prod_config_mtime() -> Option<SystemTime> {
    fs::metadata(prod_config_file_path())
        .ok()
        .and_then(|m| m.modified().ok())
}

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
                .join(".config/lulu-spark-sandbox-api")
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
        let machine_config_path = settings::config_file_path().expect("machine config");
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

        assert_eq!(
            settings::config_file_path().expect("machine config restored"),
            machine_config_path
        );
        assert_eq!(std::env::var("HOME").ok(), original_home);
        assert_eq!(std::env::var("TestSandbox").ok(), original_sandbox);
        assert_eq!(std::env::var("TestSandboxId").ok(), original_sandbox_id);
    });
}

#[test]
fn test_sandbox_new_creates_isolated_three_roots() {
    let sandbox = TestSandbox::new();
    let cfg = settings::load().expect("load");
    let base = sandbox.config_dir();
    assert!(cfg.spark_root.starts_with(base));
    assert!(cfg.knowledge_root.starts_with(base));
    assert!(cfg.cache_dir.starts_with(base));
    assert_ne!(cfg.cache_dir, default_cache_dir());
    assert_eq!(
        sandbox.config_file_path(),
        settings::config_file_path().expect("config path")
    );
    assert_eq!(
        sandbox.ports(),
        (DEFAULT_SANDBOX_HTTP_PORT, DEFAULT_SANDBOX_MCP_PORT)
    );
}

#[test]
fn write_test_config_persists_to_sandbox_config_toml() {
    let sandbox = TestSandbox::new();
    let path = settings::config_file_path().expect("cfg path");
    let text = fs::read_to_string(&path).expect("read config");
    assert!(text.contains("spark_root"));
    assert!(text.contains("knowledge_root"));
    assert!(text.contains("cache_dir"));
    assert!(path.ends_with("config.toml"));
    let _ = sandbox;
}

#[test]
fn test_config_writer_uses_explicit_target_path() {
    let sandbox = TestSandbox::new();
    let expected = sandbox.config_file_path();
    let (http_port, mcp_port) = sandbox.ports();
    settings::write_test_config_with_cache(
        &expected,
        sandbox.config_dir(),
        Some(sandbox.config_dir()),
        Some(&sandbox.config_dir().join("cache")),
        http_port,
        mcp_port,
    )
    .expect("write explicit sandbox config");
    assert!(expected.is_file());

    let text = fs::read_to_string(expected).expect("read explicit config");
    assert!(text.contains("cache_dir"));
    assert!(text.contains(&format!("http_port = {http_port}")));
    assert!(text.contains(&format!("mcp_port = {mcp_port}")));
}

#[test]
fn config_write_guard_rejects_dot_dot_and_symlink_aliases() {
    let temp_machine_home = tempfile::tempdir().expect("temp machine home");
    let protected = temp_machine_home
        .path()
        .join(".config/lulu-spark/config.toml");
    fs::create_dir_all(protected.parent().expect("protected parent")).expect("mkdir");
    fs::write(&protected, "cache_dir = \"safe\"").expect("create machine config");

    let alias_parent = protected.parent().expect("protected parent").join("alias");
    fs::create_dir_all(&alias_parent).expect("create alias parent");
    let dot_dot_alias = alias_parent
        .join("..")
        .join(protected.file_name().expect("config file name"));
    assert!(settings::reject_write_to_protected_config(&dot_dot_alias, &protected).is_err());

    #[cfg(unix)]
    {
        let symlink_alias = temp_machine_home.path().join("config.toml");
        std::os::unix::fs::symlink(&protected, &symlink_alias).expect("create symlink");
        assert!(settings::reject_write_to_protected_config(&symlink_alias, &protected).is_err());
    }
}

#[test]
fn atomic_config_write_replaces_hard_link_without_mutating_protected_file() {
    let temp_machine_home = tempfile::tempdir().expect("temp machine home");
    let protected = temp_machine_home
        .path()
        .join(".config/lulu-spark/config.toml");
    fs::create_dir_all(protected.parent().expect("protected parent")).expect("mkdir");
    fs::write(&protected, "cache_dir = \"safe\"").expect("create machine config");
    let protected_before = fs::read_to_string(&protected).expect("read protected");
    let hard_link_alias = protected
        .parent()
        .expect("protected parent")
        .join("hard-link-config.toml");
    fs::hard_link(&protected, &hard_link_alias).expect("create hard link");

    settings::atomic_write_test_config(&hard_link_alias, "cache_dir = \"replacement\"")
        .expect("atomic replacement");

    assert_eq!(
        fs::read_to_string(&protected).expect("read protected"),
        protected_before
    );
    assert_ne!(
        fs::read_to_string(&hard_link_alias).expect("read replacement"),
        protected_before
    );
}

#[test]
fn nested_test_sandbox_serializes_distinct_roots() {
    crate::test_support::with_config_test_serial(|| {
        let _outer = TestSandbox::new();
        let outer_wb = settings::load().expect("load").spark_root;
        {
            let inner = TestSandbox::new();
            let inner_wb = settings::load().expect("load").spark_root;
            assert_ne!(outer_wb, inner_wb);
            assert!(settings::config_file_path()
                .expect("cfg")
                .ends_with("config.toml"));
            drop(inner);
        }
        assert_eq!(
            settings::load().expect("load").spark_root,
            outer_wb
        );
    });
}

#[test]
fn lib_tests_do_not_touch_prod_cache_dir_mtime() {
    let before = prod_cache_dir_mtime();
    {
        let _sandbox = TestSandbox::new();
        with_sandbox_notes(true, |_dir, notes| {
            let _ = fs::create_dir_all(notes.join("annotations/ai"));
        });
    }
    let after = prod_cache_dir_mtime();
    assert_eq!(before, after);
}

#[test]
fn lib_tests_do_not_touch_prod_config_mtime() {
    let before = prod_config_mtime();
    {
        let _sandbox = TestSandbox::new();
    }
    let after = prod_config_mtime();
    assert_eq!(before, after);
}

#[test]
fn test_sandbox_records_prod_three_roots_at_new() {
    let sandbox = TestSandbox::new();
    // Prod roots are the fake prod under the sandbox HOME, not the machine prod.
    assert_eq!(
        sandbox.prod_spark_root(),
        sandbox.config_dir().join("prod-wb")
    );
    assert_eq!(
        sandbox.prod_knowledge_root(),
        sandbox.config_dir().join("prod-knowledge-clones")
    );
    assert_eq!(sandbox.prod_cache_dir(), sandbox.config_dir().join("prod-cache"));
    let _ = load_prod_settings();
}

#[test]
fn test_sandbox_write_paths_not_equal_prod_roots() {
    let sandbox = TestSandbox::new();
    let cfg = settings::load().expect("load");
    assert_ne!(
        cfg.spark_root,
        sandbox.prod_spark_root()
    );
    assert_ne!(cfg.cache_dir, sandbox.prod_cache_dir());
    assert_ne!(
        cfg.knowledge_root,
        sandbox.prod_knowledge_root()
    );
}

#[test]
fn assert_not_prod_path_rejects_prod_spark_root() {
    let sandbox = TestSandbox::new();
    let prod = sandbox.prod_spark_root();
    assert!(sandbox.assert_not_prod_path(&prod).is_err());
    assert!(sandbox
        .assert_not_prod_path(&prod.join("knowledge"))
        .is_err());
}

#[test]
fn assert_not_prod_path_rejects_prod_cache_dir() {
    let sandbox = TestSandbox::new();
    let prod = sandbox.prod_cache_dir();
    assert!(sandbox.assert_not_prod_path(&prod).is_err());
    assert!(sandbox
        .assert_not_prod_path(&prod.join("read_later.json"))
        .is_err());
}
