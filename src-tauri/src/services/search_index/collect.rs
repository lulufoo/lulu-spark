//! The chunk type of the search index, and the markdown-file → chunks step.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::services::index_build::common::extract_spark_title;

use super::chunk::{chunk_markdown, first_heading_title};

/// `(doc_id, layer, idx)`: the unique key of a chunk in the index.
pub type ChunkKey = (String, String, usize);

#[derive(Debug, Clone)]
pub struct SourceChunk {
    pub doc_id: String,
    pub category: String,
    /// The workspace the document lives in.
    pub workspace_id: String,
    pub layer: String,
    pub title: String,
    pub body: String,
    pub content_hash: String,
    pub created_at: String,
    /// Position of this chunk within its file.
    pub idx: usize,
}

/// `category` of chunks collected from the `document` table.
pub const DOCS_CATEGORY: &str = "docs";

impl SourceChunk {
    /// A chunk with no workspace yet; place it with [`SourceChunk::in_workspace`].
    #[allow(clippy::too_many_arguments)]
    pub fn from_text(
        category: &str,
        layer: &str,
        doc_id: &str,
        title: &str,
        body: &str,
        created_at: &str,
        idx: usize,
    ) -> Self {
        Self {
            doc_id: doc_id.to_string(),
            category: category.to_string(),
            workspace_id: String::new(),
            layer: layer.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            content_hash: content_hash(body),
            created_at: created_at.to_string(),
            idx,
        }
    }

    pub fn in_workspace(mut self, workspace_id: &str) -> Self {
        self.workspace_id = workspace_id.to_string();
        self
    }

    pub fn to_key(&self) -> ChunkKey {
        (self.doc_id.clone(), self.layer.clone(), self.idx)
    }
}

pub fn content_hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub(super) fn chunks_from_file(
    category: &str,
    layer: &str,
    abs: &Path,
    doc_id: &str,
    created_at: &str,
) -> Vec<SourceChunk> {
    let Ok(body) = fs::read_to_string(abs) else {
        return Vec::new();
    };
    let name = abs.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let title = first_heading_title(&body).unwrap_or_else(|| extract_spark_title(&body, name));
    chunk_markdown(&body)
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            SourceChunk::from_text(category, layer, doc_id, &title, &text, created_at, idx)
        })
        .collect()
}
