use std::sync::atomic::{AtomicUsize, Ordering};
use super::*;
use crate::services::board_cache::tests::fixture::*;

#[test]
fn a_slow_alias_of_an_older_flight_cannot_overwrite_a_newer_committed_flight() {
    for (old_version,new_version) in [(1,2),(9,1)] {
        let calls=Arc::new(AtomicUsize::new(0)); let next=calls.clone();
        let h=Harness::new(move|_,_|sketch(if next.fetch_add(1,Ordering::SeqCst)==0 {
            old_version
        } else {new_version}));
        let first=h.service.read_remote(URL);
        // A follower has received this completed source but has not yet stored
        // its original alias. A second flight is then allowed to complete.
        let late_alias=CompletedRead { result:sketch(old_version).result,
            identity:first.identity.unwrap(),bytes:None };
        let newer=h.service.read_remote(ALIAS);
        assert_eq!(newer.version,Some(new_version));
        assert_eq!(h.service.commit_alias(ALIAS,&late_alias).status,"superseded");
        assert_eq!(h.cache.disk.read(ALIAS,SCOPE).unwrap().unwrap().version,new_version as u32);
        assert!(newer.identity.unwrap().diagram_generation>late_alias.identity.diagram_generation);
    }
}

#[test]
fn restored_permission_orders_a_same_version_success_after_old_denial_results_and_events() {
    let calls=Arc::new(AtomicUsize::new(0)); let next=calls.clone();
    let h=Harness::new(move|_,_|if next.fetch_add(1,Ordering::SeqCst)==0 {
        error("forbidden")
    } else {sketch(1)});
    h.seed(URL,ID,1);
    let denied=h.service.read_remote(URL);
    let old_event=h.events.lock().unwrap()[0].clone();
    let late_denial=CompletedRead { result:Err(denied.clone()),
        identity:denied.identity.unwrap(),bytes:None };
    let restored=h.service.read_remote(ALIAS);
    assert_eq!(restored.version,Some(1));
    let restored_identity=restored.identity.unwrap();
    assert!(restored_identity.diagram_generation>old_event.diagram_generation.unwrap());
    assert_eq!(h.service.commit_alias(ALIAS,&late_denial).status,"superseded");
    assert_eq!(h.service.read_cached(ALIAS).version,Some(1));
}

#[test]
fn a_failed_retry_keeps_the_known_denial_at_the_new_request_generation() {
    let calls=Arc::new(AtomicUsize::new(0)); let next=calls.clone();
    let h=Harness::new(move|_,_|if next.fetch_add(1,Ordering::SeqCst)==0 {
        error("forbidden")
    } else {error("network")});
    h.seed(URL,ID,1);
    let first=h.service.read_remote(URL);
    let retry=h.service.read_remote(ALIAS);
    assert_eq!(retry.status,"forbidden");
    assert!(retry.identity.as_ref().unwrap().diagram_generation>
        first.identity.as_ref().unwrap().diagram_generation);
    let events=h.events.lock().unwrap();
    assert_eq!(events.len(),2);
    assert_eq!(events[1].diagram_generation,Some(retry.identity.unwrap().diagram_generation));
    assert_eq!(h.service.read_cached(URL).status,"forbidden");
}
