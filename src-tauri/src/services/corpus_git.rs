//! Workbench knowledge archive git commit / pull (`settings.workbench_knowledge_root`).

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::config::paths;
use crate::integrations::git::{self, GitError};

fn workbench_knowledge_root() -> Result<PathBuf, Value> {
    paths::workbench_knowledge_root().map_err(|e| json!({ "error": format!("{e:?}") }))
}

pub fn corpus_git_commit(payload: &Value) -> Value {
    let corpus = match workbench_knowledge_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !corpus.join(".git").exists() {
        return json!({ "error": format!("corpus is not a git repository: {}", corpus.display()) });
    }
    let message = payload
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let message = if message.is_empty() {
        "update: edit via viewer"
    } else {
        message
    };
    if let Some(files) = payload.get("files").and_then(|v| v.as_array()) {
        let paths: Vec<String> = files
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if let Err(e) = git::add_paths(&corpus, &paths) {
            return git_error_value(&e, None);
        }
    } else if let Err(e) = git::add_all(&corpus) {
        return git_error_value(&e, None);
    }

    let (nothing, commit_out) = match git::commit(&corpus, message) {
        Ok(v) => v,
        Err(e) => return git_error_value(&e, None),
    };

    if nothing {
        match git::push(&corpus) {
            Ok(o) if o.success => {
                return json!({ "ok": true, "info": "nothing to commit" });
            }
            Ok(o) => {
                return json!({
                    "error": "git push failed",
                    "stderr": o.stderr
                });
            }
            Err(e) => return git_error_value(&e, None),
        }
    }

    if let Err(e) = git::pull_rebase_in_repo(&corpus) {
        return git_error_value(&e, None);
    }
    match git::push(&corpus) {
        Ok(o) if o.success => json!({ "ok": true, "info": commit_out }),
        Ok(o) => json!({ "error": "git push failed", "stderr": o.stderr }),
        Err(e) => git_error_value(&e, None),
    }
}

pub fn corpus_git_pull(_payload: &Value) -> Value {
    let corpus = match workbench_knowledge_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !corpus.join(".git").exists() {
        return json!({ "error": format!("corpus is not a git repository: {}", corpus.display()) });
    }
    match git::pull_rebase(&corpus) {
        Ok(o) if o.success => json!({ "ok": true, "info": o.stdout.trim() }),
        Ok(o) => json!({
            "error": "git pull --rebase failed",
            "stderr": o.stderr,
            "stdout": o.stdout
        }),
        Err(e) => git_error_value(&e, None),
    }
}

fn git_error_value(err: &GitError, step: Option<&str>) -> Value {
    let mut v = git::git_error_json(err, step);
    v["_status"] = json!(500);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_corpus<F: FnOnce(&std::path::Path)>(f: F) {
        let _g = LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock");
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(&corpus).expect("mkdir");
        git::exec(&corpus, &["init"]).expect("init");
        git::exec(&corpus, &["config", "user.email", "t@t.com"]).expect("e");
        git::exec(&corpus, &["config", "user.name", "t"]).expect("n");
        fs::write(corpus.join("f.md"), "a").expect("w");
        settings::write_test_config(dir.path(), &corpus, None);
        f(dir.path());
        settings::set_test_config_dir(None);
    }

    #[test]
    fn corpus_commit_errors_when_corpus_not_git() {
        let _g = LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock");
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(&corpus).expect("mkdir");
        settings::write_test_config(dir.path(), &corpus, None);
        let v = corpus_git_commit(&json!({}));
        assert!(v.get("error").is_some());
        settings::set_test_config_dir(None);
    }
}
