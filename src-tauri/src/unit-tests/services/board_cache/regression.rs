use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::fixture::*;

#[test]
fn discarded_corruption_is_not_restored_by_a_failed_remote_read() {
    for broken in ["{\"bmd\":", "{\"schema_version\":1,\"version\":7}"] {
        let h = Harness::new(|_, _| error("network"));
        h.seed(URL, ID, 7);
        h.seed(OTHER, "b_89abcdef", 4);
        let unrelated = fs::read(h.cache.disk.path(OTHER)).expect("unrelated source");
        fs::write(h.cache.disk.path(URL), broken).expect("damage fixture");

        let cached = h.service.read_cached(URL);
        assert_eq!(cached.status, "cache_miss");
        assert!(cached.content_b64.is_none());
        assert!(cached.version.is_none());
        assert!(!h.cache.disk.path(URL).exists());
        let remote = h.service.read_remote(URL);
        assert_eq!(remote.status, "network");
        assert!(remote.content_b64.is_none());
        assert!(remote.version.is_none());
        assert_eq!(h.service.read_cached(URL).status, "cache_miss");
        assert!(!h.cache.disk.path(URL).exists());
        assert_eq!(fs::read(h.cache.disk.path(OTHER)).expect("unrelated retained"), unrelated);
        assert_eq!(h.fetch.calls.load(Ordering::SeqCst), 1);
        assert_eq!(h.cache.stores.load(Ordering::SeqCst), 0);
        assert!(h.events.lock().unwrap().is_empty());
    }
}

#[test]
fn one_disk_entry_follows_sketch_to_boxes_and_back_through_the_host() {
    let versions = Arc::new(AtomicUsize::new(0));
    let next = versions.clone();
    let sources = [
        (7, "sketch\ncontent PGgxPkZpcnN0PC9oMT4=", "sketch"),
        (8, "boxes\nbox A", "not_sketch"),
        (9, "sketch\ncontent PGgxPk5leHQ8L2gxPg==", "sketch"),
    ];
    let h = Harness::new(move |_, _| {
        let index = next.fetch_add(1, Ordering::SeqCst);
        document(sources[index].0, sources[index].1)
    });
    let original_path = h.cache.disk.path(URL);
    for (version, bmd, status) in sources {
        let remote = h.service.read_remote(URL);
        assert_eq!(remote.status, status);
        assert_eq!(remote.version, Some(version));
        assert_eq!(remote.content_b64.is_some(), status == "sketch");
        let persisted = h.cache.disk.read(URL, SCOPE).expect("read source").expect("present");
        assert_eq!(persisted.version, version as u32);
        assert_eq!(persisted.bmd, bmd);
        assert_eq!(h.cache.disk.path(URL), original_path);
        let cached = h.service.read_cached(URL);
        assert_eq!(cached.status, status);
        assert_eq!(cached.version, Some(version));
    }
    assert_eq!(versions.load(Ordering::SeqCst), 3);
    assert_eq!(h.cache.stores.load(Ordering::SeqCst), 3);
    assert!(h.events.lock().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn unreadable_cleanup_never_returns_a_malformed_record_or_blocks_remote_success() {
    use std::os::unix::fs::PermissionsExt;

    let h = Harness::new(|_, _| sketch(7));
    h.seed(URL, ID, 4);
    h.seed(OTHER, "b_89abcdef", 4);
    let path = h.cache.disk.path(URL);
    fs::write(&path, "{\"version\":4,\"bmd\":").expect("malformed source");
    let unrelated = fs::read(h.cache.disk.path(OTHER)).expect("unrelated source");
    let directory = h.cache.disk.directory();
    let permissions = fs::metadata(directory).expect("directory").permissions();
    fs::set_permissions(directory, fs::Permissions::from_mode(0o500)).expect("deny unlink");

    // Restore permissions before assertions so a failed check cannot strand the
    // temporary directory. Only this fixture's files are affected.
    let raw = h.cache.disk.read(URL, SCOPE);
    let cached = h.service.read_cached(URL);
    let remote = h.service.read_remote(URL);
    let malformed_remains = fs::read(&path);
    fs::set_permissions(directory, permissions).expect("restore directory");

    assert!(raw.is_err(), "removing malformed JSON must fail in the read-only directory");
    assert_eq!(cached.status, "cache_miss");
    assert!(cached.content_b64.is_none());
    assert!(cached.version.is_none());
    assert_eq!(remote.status, "sketch");
    assert_eq!(remote.version, Some(7));
    assert_eq!(malformed_remains.expect("failed unlink retains file"), b"{\"version\":4,\"bmd\":");
    assert_eq!(fs::read(h.cache.disk.path(OTHER)).expect("unrelated retained"), unrelated);
    assert_eq!(h.service.read_cached(URL).status, "cache_miss");
    assert!(!path.exists());
}
