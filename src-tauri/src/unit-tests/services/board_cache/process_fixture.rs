//! Explicit disk checkpoints bridge the test-only memory Vault between real PIDs.
//! This exercises production writer/codec composition, not native app startup.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};

use serde_json::{json, Value};

use crate::config::vault::{self, BoardCredentialSnapshot};
use crate::repositories::board_cache::BoardCache;
use crate::test_support::{reset_vault, TestSandbox};
use super::super::{backend, Coordinator};
use super::fixture::{error, sketch, Cache, Fetch, ALIAS, OTHER, URL};

pub const PAT: &str = "lin_pat_process_fixture_only";
pub const SOURCES: [&str; 3] = [URL, ALIAS, OTHER];

pub struct ProcessHarness {
    _sandbox: TestSandbox,
    root: PathBuf,
    pub cache: Arc<Cache>,
    pub fetch: Arc<Fetch>,
    pub service: Coordinator,
}

impl ProcessHarness {
    pub fn open(root: &Path, remote_status: Option<&'static str>, fail_delete: bool) -> Self {
        let sandbox = TestSandbox::new();
        reset_vault();
        let path = root.join("vault-fixture.toml");
        if path.exists() {
            let restored = vault::vault_from_toml(&fs::read_to_string(path).expect("read fixture Vault"))
                .expect("production TOML decoder restores the saved Vault");
            vault::update_vault(|current| *current = restored).expect("restore only this process's test Vault");
        }
        let cache = Arc::new(Cache {
            disk: BoardCache::new(root),
            fail_delete: AtomicBool::new(fail_delete),
            fail_store: AtomicBool::new(false),
            stores: AtomicUsize::new(0),
            on_store: Mutex::new(None),
        });
        let fetch = Arc::new(Fetch {
            calls: AtomicUsize::new(0),
            run: Box::new(move |credential, _| {
                assert_eq!(credential, PAT, "only the fake test credential may reach the controlled fetcher");
                match remote_status {
                    Some("sketch") => sketch(7),
                    Some(status) => error(status),
                    None => panic!("this cache-only stage must never fetch remotely"),
                }
            }),
        });
        let service = Coordinator::new(Arc::new(backend::VaultCredentials), cache.clone(), fetch.clone());
        Self { _sandbox: sandbox, root: root.into(), cache, fetch, service }
    }

    pub fn credentials(&self) -> BoardCredentialSnapshot {
        vault::get_board_credentials().expect("read actual Vault credentials").expect("configured")
    }

    pub fn checkpoint(&self) {
        let vault = vault::read_vault().expect("read actual Vault tree");
        let path = self.root.join("vault-fixture.toml");
        vault::write_debug_vault_fixture(&path, &vault).expect("production debug Vault writer checkpoints fixture");
        let restored = vault::vault_from_toml(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(restored, vault);
        assert!(!path.with_extension("toml.tmp").exists());
    }

    pub fn source_bytes(&self) -> Vec<Vec<u8>> {
        SOURCES.iter().map(|url| fs::read(self.cache.disk.path(url)).expect("original source file remains"))
            .collect()
    }

    pub fn assert_original_sources(&self, original_scope: &str) {
        for url in SOURCES {
            let record = self.cache.disk.read(url, original_scope).unwrap().expect("original on-disk source remains");
            assert_eq!(record.version, 7);
            assert_eq!(record.credential_scope, original_scope);
            assert!(!fs::read_to_string(self.cache.disk.path(url)).unwrap().contains(PAT));
        }
    }

    pub fn assert_all_cached(&self) {
        for url in SOURCES {
            let reply = self.service.read_cached(url);
            assert_eq!(reply.status, "sketch");
            assert_eq!(reply.version, Some(7));
        }
        assert_eq!(self.fetch.calls.load(Ordering::SeqCst), 0);
    }

    pub fn receipt(&self, scenario: &str) -> Value {
        let current = self.credentials();
        json!({
            "scenario": scenario, "scope": current.cache_scope, "revoked": current.revoked,
            "denied_diagrams": current.denied_diagrams,
            "fetch_calls": self.fetch.calls.load(Ordering::SeqCst),
            "source_files": SOURCES.iter().filter(|url| self.cache.disk.path(url).is_file()).count(),
        })
    }
}

pub fn scenario(root: &Path) -> String {
    let value = fs::read_to_string(root.join("scenario.txt")).expect("scenario supplied by parent fixture");
    assert!(matches!(value.as_str(), "normal" | "same_pat" | "delete_restore" |
        "unauthorized" | "forbidden" | "not_found"));
    value
}

pub fn original_scope(root: &Path) -> String {
    let receipt: Value = serde_json::from_slice(&fs::read(root.join("receipt-seed.json")).unwrap()).unwrap();
    receipt["result"]["scope"].as_str().expect("seed receipt scope").into()
}
