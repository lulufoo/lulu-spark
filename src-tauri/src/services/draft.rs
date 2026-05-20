//! Comment draft cache under `{cache_dir}/drafts/`.

use std::fs;

use serde_json::{json, Value};

use crate::config::paths;

pub fn save_comment_draft(payload: &Value) -> Value {
    let common_path = payload
        .get("common_path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }
    let draft_file = match paths::draft_path(common_path) {
        Ok(p) => p,
        Err(_) => return json!({ "error": "Path traversal not allowed", "_status": 400 }),
    };
    if !content.is_empty() {
        if let Some(parent) = draft_file.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return json!({ "error": e.to_string(), "_status": 500 });
            }
        }
        if let Err(e) = fs::write(&draft_file, content) {
            return json!({ "error": e.to_string(), "_status": 500 });
        }
    } else if draft_file.exists() {
        let _ = fs::remove_file(&draft_file);
    }
    json!({ "ok": true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_cache<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock");
        let dir = tempfile::tempdir().expect("tmp");
        let cfg = dir.path().join("cfg");
        fs::create_dir_all(&cfg).expect("mkdir");
        let cache = dir.path().join("cache");
        fs::write(
            cfg.join("config.toml"),
            format!(r#"cache_dir = "{}""#, cache.display()),
        )
        .expect("write");
        crate::config::settings::set_test_config_dir(Some(cfg.clone()));
        f();
        crate::config::settings::set_test_config_dir(None);
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
}
