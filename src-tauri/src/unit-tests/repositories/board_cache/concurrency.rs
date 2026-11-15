use super::*;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn independent_handles_serialize_two_writers_reads_and_removal() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    fs::create_dir_all(cache.directory()).expect("mkdir");
    fs::write(cache.path(URL), "broken initial JSON").expect("bad seed");
    let barrier = Arc::new(Barrier::new(4));
    let reader_cache = cache.clone();
    let reader_start = barrier.clone();
    let reader = thread::spawn(move || {
        reader_start.wait();
        for _ in 0..100 {
            if let Some(value) = reader_cache.read(URL, SCOPE).expect("serialized read") {
                assert_eq!(value.bmd, format!("revision {}", value.version));
            }
        }
    });
    let writer_cache = BoardCache::new(dir.path());
    let writer_start = barrier.clone();
    let writer = thread::spawn(move || {
        writer_start.wait();
        for version in 1..=100 {
            let mut value = record(URL, "b_0123abcd", SCOPE);
            value.version = version;
            value.bmd = format!("revision {version}");
            writer_cache.store(&value).expect("atomic replacement");
        }
    });
    let other_writer_cache = BoardCache::new(dir.path());
    let other_writer_start = barrier.clone();
    let other_writer = thread::spawn(move || {
        other_writer_start.wait();
        for version in 200..=299 {
            let mut value = record(URL, "b_0123abcd", SCOPE);
            value.version = version;
            value.bmd = format!("revision {version}");
            other_writer_cache.store(&value).expect("second writer");
        }
    });
    barrier.wait();
    for _ in 0..50 {
        cache.remove_url(URL).expect("serialized remove");
    }
    reader.join().expect("reader");
    writer.join().expect("writer");
    other_writer.join().expect("other writer");
    let mut final_value = record(URL, "b_0123abcd", SCOPE);
    final_value.version = 101;
    final_value.bmd = "revision 101".into();
    cache.store(&final_value).expect("final write");
    assert_eq!(cache.read(URL, SCOPE).expect("final read"), Some(final_value));
    assert!(!cache.path(URL).with_extension("json.tmp").exists());
}
