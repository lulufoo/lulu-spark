//! Real-process fixture restoration; this does not claim native App restart coverage.

use std::collections::BTreeSet;
use std::sync::atomic::Ordering;

use serde_json::Value;

use crate::config::vault;
use crate::test_support::{run_stage_child, TestSubprocess};
use super::fixture::{ALIAS, ID, OTHER, URL};
use super::process_fixture::{original_scope, scenario, ProcessHarness, PAT, SOURCES};

fn child_test(stage: &str) -> String {
    format!("services::board_cache::tests::process::{stage}_child")
}

fn run_case(scenario: &str) {
    let fixture = TestSubprocess::new();
    std::fs::write(fixture.directory().join("scenario.txt"), scenario).unwrap();
    let receipts = ["seed", "transition", "verify"].map(|stage| fixture.run_stage(&child_test(stage), stage));
    let pids: BTreeSet<_> = receipts.iter().map(|receipt| receipt.pid).collect();
    assert_eq!(pids.len(), 3, "all stages must execute in distinct real processes");
    for receipt in &receipts {
        assert_eq!(receipt.result["scenario"], scenario);
        assert_eq!(receipt.result["source_files"], 3, "original cache files must survive");
    }
    let seed_scope = &receipts[0].result["scope"];
    let changed = matches!(scenario, "same_pat" | "delete_restore");
    assert_eq!(receipts[1].result["scope"] != *seed_scope, changed);
    assert_eq!(receipts[2].result["scope"], receipts[1].result["scope"]);
    assert_eq!(receipts[0].result["fetch_calls"], 3);
    assert_eq!(receipts[2].result["fetch_calls"], 0);
    println!("fixture restoration verified: scenario={scenario}, distinct child PIDs={pids:?}");
}

#[test]
fn fixture_process_restart_preserves_cached_source_without_remote_access() { run_case("normal"); }

#[test]
fn fixture_process_restart_rejects_old_scope_after_saving_the_same_pat() { run_case("same_pat"); }

#[test]
fn fixture_process_restart_rejects_old_scope_after_delete_and_restore() { run_case("delete_restore"); }

#[test]
fn fixture_process_restart_preserves_401_revocation_when_old_files_remain() { run_case("unauthorized"); }

#[test]
fn fixture_process_restart_preserves_403_denial_when_old_files_remain() { run_case("forbidden"); }

#[test]
fn fixture_process_restart_preserves_404_denial_when_old_files_remain() { run_case("not_found"); }

#[test]
#[ignore = "launched only by the shared subprocess fixture"]
fn seed_child() {
    run_stage_child(&child_test("seed"), "seed", |root| {
        let scenario = scenario(root);
        let h = ProcessHarness::open(root, Some("sketch"), false);
        assert!(vault::get_board_credentials().unwrap().is_none());
        h.service.set_pat(PAT).expect("configure fake PAT using actual Vault backend");
        for url in SOURCES {
            assert_eq!(h.service.read_remote(url).version, Some(7));
        }
        h.assert_original_sources(&h.credentials().cache_scope);
        h.checkpoint();
        h.receipt(&scenario)
    });
}

#[test]
#[ignore = "launched only by the shared subprocess fixture"]
fn transition_child() {
    run_stage_child(&child_test("transition"), "transition", |root| {
        let scenario = scenario(root);
        let status = match scenario.as_str() {
            "unauthorized" => Some("unauthorized"),
            "forbidden" => Some("forbidden"),
            "not_found" => Some("not_found"),
            _ => None,
        };
        let h = ProcessHarness::open(root, status, true);
        let old_scope = original_scope(root);
        assert_eq!(h.credentials().cache_scope, old_scope);
        h.assert_all_cached();
        let before = h.source_bytes();
        match scenario.as_str() {
            "normal" => {}
            "same_pat" => h.service.set_pat(PAT).expect("same PAT replaces scope"),
            "delete_restore" => {
                h.service.delete_pat().expect("remove fake PAT");
                assert!(vault::get_board_credentials().unwrap().is_none());
                h.service.set_pat(PAT).expect("restore the same fake PAT");
            }
            _ => assert_eq!(h.service.read_remote(URL).status, status.unwrap()),
        }
        assert_eq!(h.source_bytes(), before, "injected deletion failure preserves original bytes");
        h.assert_original_sources(&old_scope);
        h.checkpoint();
        h.receipt(&scenario)
    });
}

#[test]
#[ignore = "launched only by the shared subprocess fixture"]
fn verify_child() {
    run_stage_child(&child_test("verify"), "verify", |root| {
        let scenario = scenario(root);
        let h = ProcessHarness::open(root, None, false);
        let old_scope = original_scope(root);
        h.assert_original_sources(&old_scope);
        let current = h.credentials();
        match scenario.as_str() {
            "normal" => {
                assert_eq!(current.cache_scope, old_scope);
                h.assert_all_cached();
            }
            "same_pat" | "delete_restore" => {
                assert_ne!(current.cache_scope, old_scope);
                for url in SOURCES {
                    assert_eq!(h.service.read_cached(url).status, "cache_miss");
                }
            }
            "unauthorized" => {
                assert_eq!(current.cache_scope, old_scope);
                assert!(current.revoked, "credential-wide revocation must come from disk");
                for url in SOURCES {
                    assert_eq!(h.service.read_cached(url).status, "unauthorized");
                }
            }
            _ => {
                assert_eq!(current.cache_scope, old_scope);
                assert!(!current.revoked);
                assert!(current.denied_diagrams.contains(ID), "diagram denial must come from disk");
                for url in [URL, ALIAS] {
                    assert_eq!(h.service.read_cached(url).status, "forbidden");
                }
                assert_eq!(h.service.read_cached(OTHER).version, Some(7));
            }
        }
        assert_eq!(h.fetch.calls.load(Ordering::SeqCst), 0);
        let mut result: Value = h.receipt(&scenario);
        result["restored_from_disk"] = Value::Bool(true);
        result
    });
}
