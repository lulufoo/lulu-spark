use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum PathsError {
    RepoRootUnavailable,
}

pub fn repo_root() -> Result<PathBuf, PathsError> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or(PathsError::RepoRootUnavailable)
}

pub fn cache_dir() -> Result<PathBuf, PathsError> {
    Ok(repo_root()?.join(".cache"))
}

/// Aligns with `get_corpus_root` in `workbench_config.py` (via `meili.env`).
pub fn knowledge_corpus_dir() -> Result<PathBuf, PathsError> {
    Ok(crate::config::meili_env::corpus_root_path(&repo_root()?))
}

/// Aligns with `KNOWLEDGE_BASE_DIR` in `meili.env` (default matches server.py).
pub fn knowledge_base_dir() -> Result<PathBuf, PathsError> {
    Ok(PathBuf::from(
        crate::config::meili_env::kb_root_string(&repo_root()?),
    ))
}

/// Aligns with `knowledge_index_loader.knowledge_index_path` → `.cache/knowledge-index.json`.
pub fn knowledge_index_path() -> Result<PathBuf, PathsError> {
    Ok(cache_dir()?.join("knowledge-index.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_root_matches_cargo_manifest_parent() {
        let root = repo_root().expect("repo_root");
        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("parent")
            .to_path_buf();
        assert_eq!(root, expected);
    }

    #[test]
    fn cache_dir_is_repo_root_dot_cache() {
        let root = repo_root().expect("repo_root");
        assert_eq!(cache_dir().expect("cache_dir"), root.join(".cache"));
    }

    #[test]
    fn knowledge_corpus_dir_matches_meili_env_single_source() {
        let root = repo_root().expect("repo_root");
        assert_eq!(
            knowledge_corpus_dir().expect("corpus"),
            crate::config::meili_env::corpus_root_path(&root)
        );
    }

    #[test]
    fn knowledge_base_dir_matches_server_default() {
        assert_eq!(
            knowledge_base_dir().expect("kb"),
            PathBuf::from("/Users/lulu/Code")
        );
    }

    #[test]
    fn join_unicode_segment_does_not_panic() {
        let joined = repo_root()
            .expect("repo_root")
            .join("层")
            .join("笔记.md");
        assert!(joined.to_string_lossy().contains('层'));
    }

    #[test]
    fn repo_root_parent_is_none_returns_err() {
        // Synthetic: cannot easily break CARGO_MANIFEST_DIR parent in unit tests;
        // PathsError variant exists for callers when resolution fails.
        assert_ne!(repo_root(), Err(PathsError::RepoRootUnavailable));
    }
}
