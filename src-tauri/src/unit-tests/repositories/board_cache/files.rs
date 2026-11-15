use super::*;
use crate::repositories::board_cache::tests::{record, SCOPE, URL};
use crate::repositories::board_cache::StoreOutcome;
use std::time::{Duration, UNIX_EPOCH};

#[test]
fn capacity_evicts_oldest_write_and_never_the_fresh_target() {
    let dir = tempfile::tempdir().expect("tmp");
    let values = [
        record("https://luluboard.app/#b:b_0123abcd", "b_0123abcd", SCOPE),
        record("https://luluboard.app/#b:b_0123abce", "b_0123abce", SCOPE),
        record("https://luluboard.app/#b:b_0123abcf", "b_0123abcf", SCOPE),
    ];
    let entry_size = serde_json::to_vec_pretty(&serde_json::to_value(&values[0]).expect("value"))
        .expect("json").len() as u64;
    let cache = BoardCache::with_capacity(dir.path(), entry_size * 2);
    for (index, value) in values.iter().enumerate() {
        cache.store(value).expect("store");
        let modified = UNIX_EPOCH + Duration::from_secs(index as u64 + 1);
        fs::OpenOptions::new()
            .write(true)
            .open(cache.path(&value.url))
            .expect("file")
            .set_times(fs::FileTimes::new().set_modified(modified))
            .expect("order writes");
    }
    assert!(!cache.path(&values[0].url).exists());
    assert!(cache.path(&values[1].url).exists());
    assert!(cache.path(&values[2].url).exists());
    let total: u64 = cache.entries().expect("files").iter().map(|f| f.bytes).sum();
    assert!(total <= entry_size * 2);
}

#[test]
fn oversized_single_write_is_skipped_and_keeps_the_previous_record() {
    let dir = tempfile::tempdir().expect("tmp");
    let value = record(URL, "b_0123abcd", SCOPE);
    let size = serde_json::to_vec_pretty(&serde_json::to_value(&value).expect("value"))
        .expect("json").len() as u64;
    let cache = BoardCache::with_capacity(dir.path(), size);
    assert_eq!(cache.store(&value).expect("write"), StoreOutcome::Stored);
    assert_eq!(fs::metadata(cache.path(URL)).expect("actual file").len(), size);
    let mut too_large = value.clone();
    too_large.bmd.push('x');
    assert_eq!(cache.store(&too_large).expect("skip"), StoreOutcome::SkippedTooLarge);
    assert_eq!(cache.read(URL, SCOPE).expect("old record"), Some(value));
}

#[test]
fn store_cleans_crash_leftover_temps_without_deleting_other_files() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    fs::create_dir_all(cache.directory()).expect("mkdir");
    let interrupted = cache.path(URL).with_extension("json.tmp");
    let another = cache.path("https://luluboard.app/#b:b_89abcdef").with_extension("json.tmp");
    fs::write(&interrupted, "interrupted cache write").expect("temp");
    fs::write(&another, "another interrupted cache write").expect("temp");
    let unrelated = cache.directory().join("unrelated.json.tmp");
    fs::write(&unrelated, "unrelated").expect("other file");
    cache.store(&record(URL, "b_0123abcd", SCOPE)).expect("store");
    assert!(!interrupted.exists());
    assert!(!another.exists());
    assert!(unrelated.is_file());
    assert!(cache.path(URL).is_file());
}
