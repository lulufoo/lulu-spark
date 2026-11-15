use super::*;

#[test]
fn remove_diagram_removes_url_aliases_only_in_its_scope() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    let alias = URL.to_owned() + "&alias=1";
    let foreign_scope_url = URL.to_owned() + "&alias=2";
    let other_url = "https://luluboard.app/#b:b_89abcdef";
    for value in [
        record(URL, "b_0123abcd", SCOPE),
        record(&alias, "b_0123abcd", SCOPE),
        record(&foreign_scope_url, "b_0123abcd", "other"),
        record(other_url, "b_89abcdef", SCOPE),
    ] {
        cache.store(&value).expect("store");
    }
    assert_eq!(cache.remove_diagram("b_0123abcd", SCOPE).expect("remove"), 2);
    assert!(!cache.path(URL).exists());
    assert!(!cache.path(&alias).exists());
    assert!(cache.path(&foreign_scope_url).exists());
    assert!(cache.path(other_url).exists());
    assert_eq!(cache.remove_scope(SCOPE).expect("remove scope"), 1);
    assert!(cache.path(&foreign_scope_url).exists());
}

#[test]
fn url_and_all_cache_removal_leave_other_runtime_data_untouched() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    fs::write(dir.path().join("unrelated.json"), "unrelated").expect("other cache");
    let value = record(URL, "b_0123abcd", SCOPE);
    cache.store(&value).expect("store");
    cache.remove_url(URL).expect("remove URL");
    cache.remove_url(URL).expect("already absent");
    assert!(!cache.path(URL).exists());
    cache.store(&value).expect("store again");
    fs::write(cache.path(URL).with_extension("json.tmp"), "interrupted").expect("temp");
    cache.clear().expect("clear Board cache");
    cache.clear().expect("already absent");
    assert!(!cache.directory().exists());
    assert_eq!(fs::read_to_string(dir.path().join("unrelated.json")).expect("other"), "unrelated");
}

#[test]
fn corrupt_metadata_found_during_scoped_cleanup_is_discarded() {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = BoardCache::new(dir.path());
    seed(&cache, URL, &json!({ "schema_version": 1, "bmd": "missing metadata" }));
    assert_eq!(cache.remove_scope(SCOPE).expect("scope clear"), 0);
    assert!(!cache.path(URL).exists());
}
