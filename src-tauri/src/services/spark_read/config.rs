use std::path::Path;
use serde_json::{json, Value};

use crate::config::secrets;
use crate::config::settings;

pub fn check_spark_root(path: &str) -> Value {
    let p = settings::expand_user_path(path);
    if path.trim().is_empty() {
        return json!({ "ok": false, "error": "路径为空" });
    }
    if !p.is_dir() {
        return json!({
            "ok": false,
            "error": format!("目录不存在：{}", p.display())
        });
    }
    json!({ "ok": true })
}

pub fn get_config(_repo_root: &Path) -> Value {
    let _ = _repo_root;
    let s = settings::load().unwrap_or_default();
    settings::to_config_json(&s, &secrets::list_saved_engines())
        .unwrap_or_else(|e| json!({ "ok": false, "error": format!("{e}") }))
}
