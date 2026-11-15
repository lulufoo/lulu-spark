use super::*;

use serde_json::json;

pub(super) const URL: &str = "https://luluboard.app/#b:b_0123abcd";
pub(super) const SCOPE: &str = "credential-generation-a";

pub(super) fn record(url: &str, id: &str, scope: &str) -> BoardCacheRecord {
    BoardCacheRecord::new(
        url.into(),
        id.into(),
        1,
        "sketch \"原始内容\"\ncontent abc=\r\n".into(),
        "原始内容".into(),
        scope.into(),
    )
}

pub(super) fn seed(cache: &BoardCache, url: &str, value: &Value) {
    fs::create_dir_all(cache.directory()).expect("mkdir");
    fs::write(cache.path(url), serde_json::to_vec(value).expect("json")).expect("seed");
}

#[test]
fn runtime_uses_the_active_runtime_root_and_only_the_board_subdirectory() {
    let sandbox = crate::test_support::TestSandbox::new();
    let cache = BoardCache::runtime().expect("runtime cache");
    assert_eq!(cache.directory(), sandbox.sandbox_dir().join("cache/board"));
    let cfg = settings::load().expect("config");
    let mut changed = cfg.clone();
    changed.cache_dir = sandbox.sandbox_dir().join("another-configured-cache");
    settings::save(&changed).expect("change config cache only");
    assert_eq!(
        BoardCache::runtime().expect("same plane").directory(),
        cache.directory()
    );
}

#[test]
fn url_utf8_bytes_make_the_filename_without_id_or_type_substitution() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    assert_eq!(cache_key(URL), "7489458e528a132d07195e849a2ddaa6");
    assert_eq!(
        cache_key("https://luluboard.app/view?title=中文#b:b_0123abcd"),
        "b86c8f17d0eaa321f0390ec05f0b1e66"
    );
    assert_eq!(
        cache.path(URL),
        dir.path().join("board/7489458e528a132d07195e849a2ddaa6.json")
    );
    assert_ne!(cache.path(URL), cache.path(&(URL.to_owned() + "&alias=1")));
}

#[test]
fn source_and_metadata_survive_a_new_cache_handle_without_content_digest() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    let original = record(URL, "b_0123abcd", SCOPE);
    assert_eq!(cache.store(&original).expect("store"), StoreOutcome::Stored);
    let persisted: Value = serde_json::from_slice(&fs::read(cache.path(URL)).expect("file"))
        .expect("JSON");
    assert!(persisted.get("bmd_md5").is_none());
    assert_eq!(persisted["bmd"], original.bmd);
    let reopened = BoardCache::new(dir.path());
    assert_eq!(reopened.read(URL, SCOPE).expect("read"), Some(original));
    assert!(!cache.path(URL).with_extension("json.tmp").exists());
}

#[test]
fn opaque_sources_of_other_types_and_empty_source_use_the_same_record_format() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    for source in ["boxes\nbox A", "flowchart LR\nA-->B", "not currently understood", ""] {
        let mut value = record(URL, "b_0123abcd", SCOPE);
        value.bmd = source.into();
        cache.store(&value).expect("store opaque source");
        assert_eq!(cache.read(URL, SCOPE).expect("read"), Some(value));
    }
}

#[test]
fn legitimate_json_reformatting_and_valid_content_changes_are_not_integrity_checked() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    let mut value = record(URL, "b_0123abcd", SCOPE);
    seed(&cache, URL, &serde_json::to_value(&value).expect("json"));
    assert_eq!(cache.read(URL, SCOPE).expect("compact JSON"), Some(value.clone()));
    value.bmd = "changed valid string\n没有摘要校验".into();
    seed(&cache, URL, &serde_json::to_value(&value).expect("json"));
    assert_eq!(cache.read(URL, SCOPE).expect("read"), Some(value));
}

#[test]
fn missing_files_and_mismatched_scopes_never_return_source() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    assert_eq!(cache.read(URL, SCOPE).expect("missing"), None);
    let value = record(URL, "b_0123abcd", SCOPE);
    cache.store(&value).expect("store");
    assert_eq!(cache.read(URL, "different-credential").expect("scope"), None);
    assert!(cache.path(URL).is_file());
    assert_eq!(cache.read(URL, SCOPE).expect("original scope"), Some(value));
}

#[test]
fn invalid_json_is_deleted_before_returning_a_miss() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    fs::create_dir_all(cache.directory()).expect("mkdir");
    fs::write(cache.path(URL), b"{\"bmd\":").expect("broken JSON");
    assert_eq!(cache.read(URL, SCOPE).expect("miss"), None);
    assert!(!cache.path(URL).exists());
}

#[test]
fn malformed_required_fields_are_deleted_instead_of_becoming_a_version_baseline() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    let original = serde_json::to_value(record(URL, "b_0123abcd", SCOPE)).expect("json");
    for key in [
        "schema_version",
        "url",
        "diagram_id",
        "version",
        "bmd",
        "title",
        "credential_scope",
        "cached_at",
    ] {
        let mut value = original.clone();
        value.as_object_mut().expect("object").remove(key);
        seed(&cache, URL, &value);
        assert_eq!(cache.read(URL, SCOPE).expect("miss"), None, "missing {key}");
        assert!(!cache.path(URL).exists(), "delete missing {key}");
    }
    for (key, bad) in [
        ("version", json!(0)),
        ("version", json!(-1)),
        ("version", json!(1.5)),
        ("bmd", json!(false)),
        ("diagram_id", json!("../other")),
        ("credential_scope", json!("")),
        ("cached_at", json!("invalid date")),
        ("schema_version", json!("1")),
        ("schema_version", json!(0)),
        ("url", json!("")),
    ] {
        let mut value = original.clone();
        value[key] = bad;
        seed(&cache, URL, &value);
        assert_eq!(cache.read(URL, SCOPE).expect("miss"), None, "bad {key}");
        assert!(!cache.path(URL).exists(), "delete bad {key}");
    }
}

#[test]
fn unsupported_format_is_a_miss_without_deleting_an_unknown_record() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    seed(&cache, URL, &json!({ "schema_version": SCHEMA_VERSION + 1 }));
    assert_eq!(cache.read(URL, SCOPE).expect("miss"), None);
    assert!(cache.path(URL).is_file());
}

#[test]
fn record_under_the_wrong_url_key_is_discarded() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    let value = record(&(URL.to_owned() + "&alias=1"), "b_0123abcd", SCOPE);
    seed(&cache, URL, &serde_json::to_value(value).expect("json"));
    assert_eq!(cache.read(URL, SCOPE).expect("miss"), None);
    assert!(!cache.path(URL).exists());
}

#[test]
fn lower_versions_can_be_written_after_a_service_accepts_a_recreated_diagram() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    let mut value = record(URL, "b_0123abcd", SCOPE);
    value.version = 12;
    cache.store(&value).expect("old document");
    value.version = 1;
    value.bmd = "replacement document".into();
    cache.store(&value).expect("accepted new generation");
    assert_eq!(cache.read(URL, SCOPE).expect("read"), Some(value));
}

#[test]
fn write_errors_return_to_the_caller_without_a_partial_valid_record() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    fs::write(cache.directory(), "directory unavailable").expect("block cache directory");
    assert!(cache.store(&record(URL, "b_0123abcd", SCOPE)).is_err());
    assert!(!cache.path(URL).exists());
}

#[test]
fn read_io_failure_never_returns_content_or_a_version() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    fs::create_dir_all(cache.path(URL)).expect("directory at file path");
    assert!(cache.read(URL, SCOPE).is_err());
}

#[test]
fn invalid_records_are_not_written() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    let mut value = record(URL, "b_0123abcd", SCOPE);
    value.version = 0;
    assert!(cache.store(&value).is_err());
    assert!(!cache.directory().exists());
}

#[path = "board_cache/removal.rs"]
mod removal;

#[path = "board_cache/concurrency.rs"]
mod concurrency;
