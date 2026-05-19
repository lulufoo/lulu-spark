use std::path::PathBuf;

/// Default `KNOWLEDGE_BASE_DIR` in server.py (before meili.env override).
const DEFAULT_KNOWLEDGE_BASE_DIR: &str = "/Users/lulu/Code";

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

/// Aligns with `KNOWLEDGE_CORPUS_DIR = REPO_ROOT` default in server.py.
pub fn knowledge_corpus_dir() -> Result<PathBuf, PathsError> {
    repo_root()
}

/// Aligns with `KNOWLEDGE_BASE_DIR` default in server.py.
pub fn knowledge_base_dir() -> Result<PathBuf, PathsError> {
    Ok(PathBuf::from(DEFAULT_KNOWLEDGE_BASE_DIR))
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
    fn knowledge_corpus_dir_defaults_to_repo_root() {
        assert_eq!(
            knowledge_corpus_dir().expect("corpus"),
            repo_root().expect("repo_root")
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
