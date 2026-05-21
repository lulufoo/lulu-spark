use super::*;
use std::fs;

use crate::test_support::with_test_config_dir;

fn with_cache<F: FnOnce()>(f: F) {
    with_test_config_dir(|cfg| {
        let cache = cfg.join("cache");
        fs::write(
            cfg.join("config.toml"),
            format!(r#"cache_dir = "{}""#, cache.display()),
        )
        .expect("write");
        f();
    });
}

#[test]
fn save_draft_writes_under_cache_dir() {
    with_cache(|| {
        let v = save_comment_draft(&json!({
            "common_path": "proj/note.md",
            "content": "hello"
        }));
        assert_eq!(v["ok"], true);
        let p = paths::draft_path("proj/note.md").expect("path");
        assert_eq!(fs::read_to_string(&p).expect("read"), "hello");
    });
}
