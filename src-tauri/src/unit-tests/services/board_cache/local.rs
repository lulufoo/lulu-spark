use std::sync::atomic::Ordering;
use std::fs;
use super::*;

#[test]
fn cold_remote_read_stores_source_and_cache_only_survives_restart_without_network() {
    let h=Harness::new(|_,_|sketch(7));
    assert_eq!(h.service.read_cached(URL).status,"cache_miss");
    assert_eq!(h.fetch.calls.load(Ordering::SeqCst),0);
    let reply=h.service.read_remote(URL);
    assert_eq!(reply.status,"sketch");
    assert_eq!(reply.version,Some(7));
    assert_eq!(reply.identity.as_ref().unwrap().credential_scope,SCOPE);
    assert!(!serde_json::to_string(&reply).unwrap().contains("secret-a"));
    let persisted=h.cache.disk.read(URL,SCOPE).unwrap().unwrap();
    assert_eq!(persisted.bmd,"sketch\ncontent PGgxPkhpPC9oMT4=");
    assert!(!fs::read_to_string(h.cache.disk.path(URL)).unwrap().contains("secret-a"));
    assert_eq!(h.restart().read_cached(URL).version,Some(7));
    assert_eq!(h.fetch.calls.load(Ordering::SeqCst),1);
}

#[test]
fn invalid_cached_database_version_or_wrong_diagram_identity_is_deleted() {
    let h=Harness::new(|_,_|sketch(1));
    h.seed(URL,ID,i32::MAX as u32+1);
    assert_eq!(h.service.read_cached(URL).status,"cache_miss");
    assert!(!h.cache.disk.path(URL).exists());
    h.seed(URL,"b_89abcdef",1);
    assert_eq!(h.service.read_cached(URL).status,"cache_miss");
    assert!(!h.cache.disk.path(URL).exists());
}

#[test]
fn same_version_keeps_valid_cache_but_corrupt_same_version_cache_is_rebuilt() {
    let h=Harness::new(|_,_|sketch(7));
    h.seed(URL,ID,7);
    let before=fs::read(h.cache.disk.path(URL)).unwrap();
    assert_eq!(h.service.read_remote(URL).version,Some(7));
    assert_eq!(h.cache.stores.load(Ordering::SeqCst),0);
    assert_eq!(fs::read(h.cache.disk.path(URL)).unwrap(),before);
    fs::write(h.cache.disk.path(URL),"broken JSON").unwrap();
    assert_eq!(h.service.read_cached(URL).status,"cache_miss");
    assert!(!h.cache.disk.path(URL).exists());
    assert_eq!(h.service.read_remote(URL).version,Some(7));
    assert_eq!(h.cache.stores.load(Ordering::SeqCst),1);
    assert!(h.cache.disk.read(URL,SCOPE).unwrap().is_some());
}

#[test]
fn generic_sources_and_bad_sketch_projections_still_cache_the_original_document() {
    for (source,status) in [("boxes\nbox a", "not_sketch"),
        ("sketch\ncontent !!!invalid-base64!!!", "invalid_source")] {
        let h=Harness::new(move|_,_|document(3,source));
        assert_eq!(h.service.read_remote(URL).status,status);
        assert_eq!(h.cache.disk.read(URL,SCOPE).unwrap().unwrap().bmd,source);
        assert_eq!(h.service.read_cached(URL).status,status);
    }
}

#[test]
fn network_or_invalid_remote_document_preserves_valid_cache() {
    for status in ["network","invalid_document"] {
        let h=Harness::new(move|_,_|error(status));
        h.seed(URL,ID,4);
        assert_eq!(h.service.read_remote(URL).status,status);
        assert_eq!(h.service.read_cached(URL).version,Some(4));
        assert!(h.events.lock().unwrap().is_empty());
    }
}

#[test]
fn cache_io_errors_and_store_failures_do_not_fail_successful_remote_reads() {
    let h=Harness::new(|_,_|sketch(4));
    fs::write(h.cache.disk.directory(),"not a directory").unwrap();
    assert_eq!(h.service.read_cached(URL).status,"cache_miss");
    assert_eq!(h.service.read_remote(URL).status,"sketch");
    let h=Harness::new(|_,_|sketch(4));
    h.cache.fail_store.store(true,Ordering::SeqCst);
    assert_eq!(h.service.read_remote(URL).version,Some(4));
    assert_eq!(h.service.read_cached(URL).status,"cache_miss");
}

#[test]
fn credential_save_or_delete_failure_does_not_clear_cache_advance_generation_or_emit() {
    let h=Harness::new(|_,_|sketch(2));
    h.seed(URL,ID,2);
    let before=h.service.read_cached(URL).identity.unwrap();
    h.credentials.fail_write.store(true,Ordering::SeqCst);
    assert!(h.service.set_pat("new-value").is_err());
    assert!(h.service.delete_pat().is_err());
    let after=h.service.read_cached(URL);
    assert_eq!(after.identity,Some(before));
    assert_eq!(after.version,Some(2));
    assert!(h.events.lock().unwrap().is_empty());
}

#[test]
fn replacing_or_removing_credentials_clears_cache_and_persists_scope_isolation() {
    let h=Harness::new(|_,_|sketch(1));
    h.seed(URL,ID,4);
    let old=h.service.read_cached(URL).identity.unwrap();
    h.service.set_pat("new-value").unwrap();
    assert_eq!(h.service.read_cached(URL).status,"cache_miss");
    assert!(!h.cache.disk.path(URL).exists());
    let new=h.service.read_remote(URL).identity.unwrap();
    assert_ne!(new.credential_scope,old.credential_scope);
    assert!(new.generation>old.generation);
    h.service.delete_pat().unwrap();
    assert_eq!(h.restart().read_cached(URL).status,"unconfigured");
    assert_eq!(h.events.lock().unwrap().len(),2);
}
