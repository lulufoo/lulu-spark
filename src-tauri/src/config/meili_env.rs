//! Single source for `meili.env` parsing (parity with `scripts/workbench_config.py`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_KB_ROOT: &str = "/Users/lulu/Code";
const DEFAULT_CORPUS_GITHUB: &str = "https://github.com/lulufoo/lulu-workbench-knowledge/blob/main";

/// Parse `repo_root/meili.env` (same rules as `workbench_config.load_meili_env`).
pub fn meili_kv(repo_root: &Path) -> HashMap<String, String> {
    let mut m = HashMap::new();
    let p = repo_root.join("meili.env");
    let Ok(text) = fs::read_to_string(p) else {
        return m;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        m.insert(k.trim().to_string(), v.trim().to_string());
    }
    m
}

pub fn corpus_root_path(repo_root: &Path) -> PathBuf {
    let m = meili_kv(repo_root);
    m.get("KNOWLEDGE_CORPUS_DIR")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.to_path_buf())
}

pub fn kb_root_string(repo_root: &Path) -> String {
    let m = meili_kv(repo_root);
    m.get("KNOWLEDGE_BASE_DIR")
        .cloned()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_KB_ROOT.to_string())
}

pub fn meili_url(repo_root: &Path) -> String {
    meili_kv(repo_root)
        .get("MEILI_URL")
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| "http://localhost:7700".to_string())
}

pub fn meili_master_key(repo_root: &Path) -> String {
    meili_kv(repo_root)
        .get("MEILI_MASTER_KEY")
        .cloned()
        .unwrap_or_default()
}

pub fn corpus_github_string(repo_root: &Path) -> String {
    let m = meili_kv(repo_root);
    m.get("KNOWLEDGE_CORPUS_GITHUB")
        .map(|s| s.trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_CORPUS_GITHUB.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn corpus_root_uses_knowledge_corpus_dir_from_meili_env() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(&corpus).expect("mkdir");
        let mut f = fs::File::create(dir.path().join("meili.env")).expect("env");
        writeln!(f, "KNOWLEDGE_CORPUS_DIR={}", corpus.display()).expect("w");
        assert_eq!(corpus_root_path(dir.path()), corpus);
    }

    #[test]
    fn corpus_root_falls_back_to_repo_root_without_meili_env() {
        let dir = tempfile::tempdir().expect("tmp");
        assert_eq!(corpus_root_path(dir.path()), dir.path());
    }

    #[test]
    fn meili_kv_ignores_comments_empty_lines_and_lines_without_equals() {
        let dir = tempfile::tempdir().expect("tmp");
        fs::write(
            dir.path().join("meili.env"),
            "# comment\n\nNOEQUALS\nKNOWLEDGE_CORPUS_DIR=/tmp/corpus\n",
        )
        .expect("w");
        let m = meili_kv(dir.path());
        assert_eq!(m.get("KNOWLEDGE_CORPUS_DIR").map(String::as_str), Some("/tmp/corpus"));
        assert!(!m.contains_key("NOEQUALS"));
    }

    #[test]
    fn kb_root_falls_back_to_default_without_meili_env() {
        let dir = tempfile::tempdir().expect("tmp");
        assert_eq!(kb_root_string(dir.path()), DEFAULT_KB_ROOT);
    }

    #[test]
    fn kb_root_reads_knowledge_base_dir_from_meili_env() {
        let dir = tempfile::tempdir().expect("tmp");
        let kb = dir.path().join("my-kb");
        fs::write(
            dir.path().join("meili.env"),
            format!("KNOWLEDGE_BASE_DIR={}\n", kb.display()),
        )
        .expect("w");
        assert_eq!(kb_root_string(dir.path()), kb.display().to_string());
    }

    #[test]
    fn corpus_github_default_when_unset() {
        let dir = tempfile::tempdir().expect("tmp");
        assert_eq!(corpus_github_string(dir.path()), DEFAULT_CORPUS_GITHUB);
    }

    #[test]
    fn meili_url_defaults_without_meili_env() {
        let dir = tempfile::tempdir().expect("tmp");
        assert_eq!(meili_url(dir.path()), "http://localhost:7700");
    }

    #[test]
    fn meili_url_reads_meili_url_from_meili_env() {
        let dir = tempfile::tempdir().expect("tmp");
        fs::write(dir.path().join("meili.env"), "MEILI_URL=http://127.0.0.1:7701\n").expect("w");
        assert_eq!(meili_url(dir.path()), "http://127.0.0.1:7701");
    }

    #[test]
    fn meili_master_key_empty_without_env() {
        let dir = tempfile::tempdir().expect("tmp");
        assert_eq!(meili_master_key(dir.path()), "");
    }
}
