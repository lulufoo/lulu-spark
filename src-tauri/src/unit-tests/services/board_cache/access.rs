use std::sync::atomic::{AtomicUsize,Ordering};
use crate::services::board_sketch::BoardInvalidationScope;
use super::*;

#[test]
fn unauthorized_retires_the_whole_scope_even_if_file_deletion_fails() {
    let h=Harness::new(|_,_|error("unauthorized"));
    h.seed(URL,ID,1);
    h.seed(OTHER,"b_89abcdef",2);
    h.cache.fail_delete.store(true,Ordering::SeqCst);
    let reply=h.service.read_remote(URL);
    assert_eq!(reply.status,"unauthorized");
    assert_eq!(reply.invalidation_scope,Some(BoardInvalidationScope::Credential));
    assert!(h.credentials.current.lock().unwrap().as_ref().unwrap().revoked);
    assert!(h.cache.disk.path(URL).exists(),"injected deletion failure leaves bytes");
    assert_eq!(h.restart().read_cached(URL).status,"unauthorized");
    assert_eq!(h.restart().read_cached(OTHER).status,"unauthorized");
    let event=h.events.lock().unwrap()[0].clone();
    assert_eq!(event.credential_scope,SCOPE);
    assert_eq!(event.diagram_id,None);
    assert_eq!(event.reason,"unauthorized");
}

#[test]
fn forbidden_or_missing_diagram_persists_denial_without_clearing_unrelated_diagrams() {
    for status in ["forbidden","not_found"] {
        let h=Harness::new(move|_,_|error(status));
        h.seed(URL,ID,1); h.seed(ALIAS,ID,1); h.seed(OTHER,"b_89abcdef",2);
        let reply=h.service.read_remote(URL);
        assert_eq!(reply.status,status);
        assert!(!h.cache.disk.path(URL).exists());
        assert!(!h.cache.disk.path(ALIAS).exists());
        assert_eq!(h.service.read_cached(OTHER).version,Some(2));
        let event=h.events.lock().unwrap()[0].clone();
        assert_eq!(event.diagram_id.as_deref(),Some(ID));
        assert_eq!(event.diagram_generation,Some(2));
        assert_eq!(event.reason,status);
        assert_eq!(h.restart().read_cached(URL).status,"forbidden");
    }
}

#[test]
fn denied_diagram_stays_unreadable_across_restart_after_delete_failure() {
    let h=Harness::new(|_,_|error("forbidden"));
    h.seed(URL,ID,1); h.seed(OTHER,"b_89abcdef",2);
    h.cache.fail_delete.store(true,Ordering::SeqCst);
    h.service.read_remote(URL);
    assert!(h.cache.disk.path(URL).exists());
    assert_eq!(h.restart().read_cached(URL).status,"forbidden");
    assert_eq!(h.restart().read_cached(OTHER).version,Some(2));
}

#[test]
fn a_new_remote_success_clears_durable_diagram_denial_before_caching() {
    let answer=Arc::new(AtomicUsize::new(0)); let next=answer.clone();
    let h=Harness::new(move|_,_|if next.load(Ordering::SeqCst)==0 {error("forbidden")} else {sketch(9)});
    assert_eq!(h.service.read_remote(URL).status,"forbidden");
    assert!(h.credentials.current.lock().unwrap().as_ref().unwrap().denied_diagrams.contains(ID));
    answer.store(1,Ordering::SeqCst);
    assert_eq!(h.service.read_remote(URL).version,Some(9));
    assert!(!h.credentials.current.lock().unwrap().as_ref().unwrap().denied_diagrams.contains(ID));
    assert_eq!(h.restart().read_cached(URL).version,Some(9));
}

#[test]
fn credential_replacement_protects_restart_even_if_old_files_cannot_be_removed() {
    let h=Harness::new(|_,_|sketch(1));
    h.seed(URL,ID,1);
    h.cache.fail_delete.store(true,Ordering::SeqCst);
    h.service.set_pat("new-credential").unwrap();
    assert!(h.cache.disk.path(URL).exists());
    assert_eq!(h.restart().read_cached(URL).status,"cache_miss");
    h.service.delete_pat().unwrap();
    assert_eq!(h.restart().read_cached(URL).status,"unconfigured");
}

#[test]
fn simultaneous_access_marker_and_file_removal_failure_still_fails_closed_in_process() {
    for status in ["unauthorized","forbidden"] {
        let h=Harness::new(move|_,_|error(status));
        h.seed(URL,ID,1);
        h.credentials.fail_write.store(true,Ordering::SeqCst);
        h.cache.fail_delete.store(true,Ordering::SeqCst);
        assert_eq!(h.service.read_remote(URL).status,status);
        let cached=h.service.read_cached(URL);
        assert_eq!(cached.status,status);
        assert_eq!(cached.version,None);
        assert!(h.cache.disk.path(URL).exists());
        // Both persistence channels were unavailable, so this assertion is
        // intentionally about the live process, not a durable restart promise.
    }
}
