use super::*;
use std::fs;

#[test]
fn save_entry_updates_file() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    let md = corpus.join("digest").join("foo.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "old").expect("w");
    crate::config::settings::write_test_config(dir.path(), &corpus, None);

    let v = save_entry(
        dir.path(),
        "digest".into(),
        "foo.md".into(),
        "new content".into(),
    );
    assert_eq!(v["ok"], json!(true));
    assert_eq!(fs::read_to_string(&md).expect("read"), "new content");
    crate::config::settings::set_test_config_dir(None);
}
