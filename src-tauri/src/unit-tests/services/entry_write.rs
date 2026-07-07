use super::*;
use std::fs;

use crate::test_support::TestSandbox;

#[test]
fn save_entry_updates_file() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let md = wb.join("digest").join("foo.md");
    fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
    fs::write(&md, "old").expect("w");

    let v = save_entry(
        sandbox.config_dir(),
        "digest".into(),
        "foo.md".into(),
        "new content".into(),
    );
    assert_eq!(v["ok"], json!(true));
    assert_eq!(fs::read_to_string(&md).expect("read"), "new content");
}
