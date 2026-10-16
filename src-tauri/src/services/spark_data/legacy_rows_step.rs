//! Migration step: copy the legacy `note` / `knowledge_doc` rows into the workspace tables.
//!
//! A copy, not a move: the old tables stay until the cleanup step drops them, so
//! nothing is lost if a later step fails. Notes land in `Default`, knowledge in
//! `Knowledge`. Knowledge project ids get a `knowledge:` prefix because `project.id`
//! is global and knowledge directory ids are plain folder names that could equal a
//! notes category id. Knowledge rows marked `missing` are not copied (no record
//! without a file). The legacy `category` group becomes text in the project
//! description.

pub(super) const LEGACY_ROWS_STEP: &str = r#"
INSERT INTO project (id, workspace_id, folder, title, description)
SELECT id, 'default', folder, title, description FROM note_category;

INSERT INTO project (id, workspace_id, folder, title, description)
SELECT 'knowledge:' || id, 'knowledge', folder, folder,
       CASE WHEN category = 'Uncategorized' THEN description
            WHEN description = '' THEN '分组：' || category
            ELSE '分组：' || category || char(10) || description END
FROM knowledge_directory;

INSERT INTO doc_default (
  doc_id, raw_path, digest_path, source_type, created_at, importance, done,
  links, comments, tags, highlights, translations, title, project_id
)
SELECT
  doc_id, raw_path, digest_path, source_type, created_at, importance, done,
  links, comments, tags, highlights, translations, title, category_id
FROM note;

INSERT INTO doc_knowledge (doc_id, raw_path, links, comments, highlights, project_id)
SELECT doc_id, path, links, comments, highlights, 'knowledge:' || directory_id
FROM knowledge_doc
WHERE status = 'active';
"#;
