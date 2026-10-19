use super::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::services::search_index::chunk::char_len;

fn temp_cache() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = dir.path().join("cache");
    fs::create_dir_all(&cache).expect("mkdir");
    (dir, cache)
}

fn chunk(id: &str, doc: &str, category: &str, body: &str) -> SourceChunk {
    SourceChunk::from_text(
        category,
        "raw",
        doc,
        "t",
        body,
        "",
        id.rsplit(':').next().and_then(|s| s.parse().ok()).unwrap_or(0),
    )
}

/// A chunk of a document of the default workspace.
fn doc_chunk(id: &str, doc: &str, body: &str) -> SourceChunk {
    chunk(id, doc, "docs", body).in_workspace(&crate::test_support::default_workspace().id)
}

#[test]
fn open_drops_tables_from_older_schema_version() {
    let (_dir, cache) = temp_cache();
    {
        let conn = rusqlite::Connection::open(cache.join(INDEX_FILE)).expect("raw open");
        conn.execute_batch(
            "CREATE VIRTUAL TABLE docs_tri USING fts5(chunk_id UNINDEXED, body);
             INSERT INTO docs_tri(chunk_id, body) VALUES ('legacy:0', 'legacy body text');",
        )
        .expect("legacy schema");
    }
    let conn = open(&cache).expect("open migrates");
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("version");
    assert_eq!(version, SCHEMA_VERSION);
    assert!(indexed_doc_ids(&conn).expect("new schema is readable").is_empty());
}

#[test]
fn replace_doc_chunks_drops_stale_tail_chunks() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let three = vec![
        chunk("p:0", "doc-p", "notes", "第一段正文内容足够长"),
        chunk("p:1", "doc-p", "notes", "第二段正文内容足够长"),
        chunk("p:2", "doc-p", "notes", "第三段正文内容足够长"),
    ];
    upsert_chunks(&conn, &three).expect("seed");
    let one = vec![chunk("p:0", "doc-p", "notes", "只剩一段")];
    let stats = replace_doc_chunks(&conn, "doc-p", &one).expect("replace");
    assert_eq!(stats.deleted, 3);
    assert_eq!(stats.upserted, 1);
    let n: i64 = conn
        .query_row("SELECT count(*) FROM chunk WHERE doc_id = 'doc-p'", [], |r| r.get(0))
        .expect("count");
    assert_eq!(n, 1);
    replace_doc_chunks(&conn, "doc-p", &[]).expect("delete all");
    assert!(indexed_doc_ids(&conn).expect("ids").is_empty());
}

#[test]
fn sync_workspace_docs_is_noop_without_index_file() {
    let _sandbox = crate::test_support::TestSandbox::new();
    let n = sync_workspace_docs(&crate::test_support::default_workspace().id, &["x"]).expect("noop");
    assert_eq!(n, 0);
    assert!(
        !index_exists(&cache_dir_or_err().expect("cache")),
        "write hook must not create a partial index"
    );
}

#[test]
fn desktop_spark_hits_always_expose_raw_layer() {
    crate::test_support::with_sandbox_notes(|_, _| {
        crate::test_support::seed_note_entries(&serde_json::json!({
            "doc-n": { "common_path": "proj/n.md" }
        }));
        let cache = cache_dir_or_err().expect("cache");
        let conn = open(&cache).expect("open");
        let digest_only = SourceChunk::from_text(
            "docs",
            "digest",
            "doc-n",
            "Note N",
            "摘要里提到角色扮演的设计",
            "",
            0,
        )
        .in_workspace(&crate::test_support::default_workspace().id);
        upsert_chunks(&conn, &[digest_only]).expect("seed");
        drop(conn);
        let out = search_desktop_docs(&cache, "角色扮演", Some(5), None);
        let hits = out["hits"].as_array().expect("hits");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["layer"], "raw");
        assert_eq!(hits[0]["id"], "doc-n");
        assert!(hits[0].get("common_path").is_none());
        assert_eq!(hits[0]["project_id"], "seed-proj");
    });
}

#[test]
fn hits_of_a_deleted_doc_are_dropped() {
    crate::test_support::with_sandbox_notes(|_, _| {
        crate::test_support::seed_note_entries(&serde_json::json!({
            "doc-live": { "common_path": "proj/live.md" }
        }));
        let cache = cache_dir_or_err().expect("cache");
        let conn = open(&cache).expect("open");
        upsert_chunks(&conn, &[
            doc_chunk("a:0", "doc-live", "共同关键词出现在笔记里"),
            doc_chunk("b:0", "doc-gone", "共同关键词出现在已删除笔记里"),
        ])
        .expect("seed");
        let all = search_in_cache(&cache, "共同关键词", &SearchFilter::default(), 10).expect("search");
        let ids: Vec<&str> = all.iter().map(|h| h.doc_id.as_str()).collect();
        assert_eq!(ids, vec!["doc-live"], "the deleted doc has no row, so no hit");
        assert_eq!(all[0].common_path, "live.md");
        assert_eq!(all[0].topic, "seed-proj");
    });
}

#[test]
fn desktop_search_reports_not_indexed_when_sqlite_missing() {
    let (_dir, cache) = temp_cache();
    let out = search_desktop_docs(&cache, "角色扮演", Some(5), None);
    assert_eq!(
        out.get("error").and_then(|v| v.as_str()),
        Some("not_indexed")
    );
}

#[test]
fn classify_routes_by_term_length() {
    assert_eq!(classify_query("的"), QueryRoute::Empty);
    assert_eq!(
        classify_query("角色"),
        QueryRoute::Bi {
            terms: vec!["角色".into()]
        }
    );
    assert_eq!(
        classify_query("角色扮演"),
        QueryRoute::Tri {
            match_terms: vec!["角色扮演".into()],
            like_terms: vec![]
        }
    );
    assert_eq!(
        classify_query("search 角色"),
        QueryRoute::Tri {
            match_terms: vec!["search".into()],
            like_terms: vec!["角色".into()]
        }
    );
}

#[test]
fn short_title_merges_into_next_chunk() {
    let text = "## 短标题\n\n这是正文段落，足够长。\n\n## 第二节\n\n另一段。";
    let chunks = chunk_markdown(text);
    assert!(
        chunks[0].contains("短标题") && chunks[0].contains("这是正文"),
        "{chunks:?}"
    );
    assert!(char_len(&chunks[0]) > SHORT_TITLE_MERGE_CHARS || chunks[0].contains("正文"));
}

#[test]
fn no_heading_is_one_chunk_until_cap() {
    let chunks = chunk_markdown("plain paragraph without heading");
    assert_eq!(chunks, vec!["plain paragraph without heading".to_string()]);
}

#[test]
fn content_hash_skips_unchanged_and_ghost_delete_removes_stale() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let first = vec![
        chunk("a:0", "doc-a", "notes", "角色扮演与叙事"),
        chunk("b:0", "doc-b", "notes", "另一个文件正文足够长"),
    ];
    let s1 = upsert_chunks(&conn, &first).expect("up1");
    assert_eq!(s1.upserted, 2);
    let s2 = upsert_chunks(&conn, &first).expect("up2");
    assert_eq!(s2.skipped, 2);
    assert_eq!(s2.upserted, 0);

    let live: HashSet<ChunkKey> = first[..1].iter().map(|c| c.to_key()).collect();
    let deleted = delete_ghosts(&conn, &live, Some("notes"), &HashSet::new()).expect("ghost");
    assert_eq!(deleted, 1);
    assert_eq!(indexed_doc_ids(&conn).expect("ids").len(), 1);
}

#[test]
fn upsert_chunks_preloads_hashes_and_skips_unchanged_bodies() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let mut many = Vec::new();
    for i in 0..80 {
        many.push(SourceChunk::from_text(
            "notes",
            "raw",
            &format!("doc-{i}"),
            "t",
            &format!("足够长的正文内容编号{i}以便切块稳定"),
            "",
            0,
        ));
    }
    let s1 = upsert_chunks(&conn, &many).expect("seed");
    assert_eq!(s1.upserted, 80);
    let s2 = upsert_chunks(&conn, &many).expect("unchanged");
    assert_eq!(s2.skipped, 80);
    assert_eq!(s2.upserted, 0);
    let hash = existing_hash(&conn, &many[0].doc_id, &many[0].layer, many[0].idx)
        .expect("lookup")
        .expect("row");
    assert_eq!(hash, many[0].content_hash);
    many[3] = SourceChunk::from_text(
        "notes",
        "raw",
        "doc-3",
        "t",
        "改过的正文也要足够长才能当新块",
        "",
        0,
    );
    let s3 = upsert_chunks(&conn, &many).expect("one rewrite");
    assert_eq!(s3.skipped, 79);
    assert_eq!(s3.upserted, 1);
}

#[test]
fn two_char_and_mixed_queries_return_snippets() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let chunks = vec![chunk(
        "n:0",
        "doc-role",
        "notes",
        "这一段讲角色扮演如何展开，以及 search 与角色的关系。")];
    upsert_chunks(&conn, &chunks).expect("up");

    let bi = search(
        &conn,
        "角色",
        &SearchFilter {
            category: Some("notes".into()),
            ..SearchFilter::default()
        },
        10,
    )
    .expect("bi");
    assert!(!bi.is_empty(), "pure two-char should hit");
    assert!(!bi[0].snippet.is_empty());

    let mixed = search(
        &conn,
        "search 角色",
        &SearchFilter {
            category: Some("notes".into()),
            ..SearchFilter::default()
        },
        10,
    )
    .expect("mixed");
    assert!(!mixed.is_empty(), "mixed should hit via trigram + like");
    assert!(!mixed[0].snippet.is_empty());
}

fn count_rows(conn: &rusqlite::Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

#[test]
fn chunk_table_has_no_location_columns() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let cols: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('chunk')")
        .expect("prepare")
        .query_map([], |r| r.get::<_, String>(0))
        .expect("cols")
        .collect::<Result<_, _>>()
        .expect("rows");
    for gone in ["path", "common_path", "repo", "mtime"] {
        assert!(!cols.iter().any(|c| c == gone), "{gone} must not be stored: {cols:?}");
    }
}

#[test]
fn chunk_unique_key_is_enforced_by_the_database() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let insert = "INSERT INTO chunk(doc_id, layer, idx, category, workspace_id, title, content_hash, created_at)
                  VALUES ('d', 'raw', 0, 'notes', 'default', '', 'h', '')";
    conn.execute(insert, []).expect("first insert");
    assert!(conn.execute(insert, []).is_err(), "duplicate (doc_id, layer, idx) must fail");
}

#[test]
fn fts_rows_share_rowid_with_chunk_and_leave_no_orphans() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let chunks = vec![
        chunk("a:0", "doc-a", "notes", "角色扮演与叙事设计"),
        chunk("a:1", "doc-a", "notes", "第二段关于检索的内容"),
        chunk("b:0", "doc-b", "notes", "另一个文件正文足够长"),
    ];
    upsert_chunks(&conn, &chunks).expect("seed");
    let orphans = |table: &str| {
        count_rows(
            &conn,
            &format!("SELECT count(*) FROM {table} WHERE rowid NOT IN (SELECT id FROM chunk)"),
        )
    };
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM docs_tri"), 3);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM docs_bi"), 3);

    let rewritten = vec![chunk("a:0", "doc-a", "notes", "改过的正文也要足够长")];
    upsert_chunks(&conn, &rewritten).expect("rewrite one");
    replace_doc_chunks(&conn, "doc-b", &[]).expect("delete doc");
    let live: HashSet<ChunkKey> = rewritten.iter().map(|c| c.to_key()).collect();
    delete_ghosts(&conn, &live, Some("notes"), &HashSet::new()).expect("ghosts");

    assert_eq!(count_rows(&conn, "SELECT count(*) FROM chunk"), 1);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM docs_tri"), 1);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM docs_bi"), 1);
    assert_eq!(orphans("docs_tri"), 0);
    assert_eq!(orphans("docs_bi"), 0);
}

#[test]
fn replace_doc_chunks_drops_a_layer_the_note_no_longer_has() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let mut digest = chunk("a:0", "doc-a", "notes", "摘要层的正文足够长");
    digest.layer = "digest".into();
    upsert_chunks(&conn, &[chunk("a:0", "doc-a", "notes", "原文层的正文足够长"), digest])
        .expect("seed");
    let stats = replace_doc_chunks(&conn, "doc-a", &[chunk("a:0", "doc-a", "notes", "只剩原文层的正文")])
        .expect("raw only");
    assert_eq!(stats.deleted, 2);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM chunk"), 1);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM chunk WHERE layer = 'digest'"), 0);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM docs_tri"), 1);
}

#[test]
fn delete_ghosts_spares_documents_of_other_repos() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    upsert_chunks(&conn, &[
        chunk("a:0", "kb-a", "knowledge", "仓库甲的正文足够长"),
        chunk("b:0", "kb-b", "knowledge", "仓库乙的正文足够长"),
        chunk("c:0", "kb-orphan", "knowledge", "没有记录的正文足够长"),
    ])
    .expect("seed");
    let keep: HashSet<String> = ["kb-b".to_string()].into();
    let deleted = delete_ghosts(&conn, &HashSet::new(), Some("knowledge"), &keep).expect("ghosts");
    assert_eq!(deleted, 2, "kb-a is a ghost of the scoped repo, kb-orphan has no record");
    assert_eq!(indexed_doc_ids(&conn).expect("ids"), keep);
}

#[test]
fn unchanged_body_refreshes_stale_metadata() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let old = chunk("a:0", "doc-a", "notes", "同样的正文足够长");
    upsert_chunks(&conn, &[old.clone()]).expect("seed");
    let mut retitled = old;
    retitled.title = "新标题".into();
    retitled.created_at = "202601011200".into();
    let stats = upsert_chunks(&conn, &[retitled]).expect("same hash, new metadata");
    assert_eq!(stats.skipped, 1);
    assert_eq!(stats.upserted, 0);
    let (title, created): (String, String) = conn
        .query_row("SELECT title, created_at FROM chunk", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("row");
    assert_eq!((title.as_str(), created.as_str()), ("新标题", "202601011200"));
}

#[test]
fn bigram_hit_snippet_comes_from_original_text_and_title_only_words_do_not_match() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let mut c = chunk("n:0", "doc-n", "notes", "这一段讲角色扮演如何展开，与 search 无关。");
    c.title = "专属标题词".into();
    upsert_chunks(&conn, &[c]).expect("seed");
    let filter = SearchFilter::default();

    let hits = search(&conn, "角色", &filter, 10).expect("bi");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].snippet.contains("角色扮演"), "snippet must be original text, not bigrams");
    assert_eq!(hits[0].title, "专属标题词", "title is read from chunk");

    assert!(
        search(&conn, "专属标题词", &filter, 10).expect("title-only").is_empty(),
        "title is not indexed (decision 27)"
    );
}

#[test]
fn doc_id_lookup_uses_the_unique_index() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let plan: Vec<String> = conn
        .prepare("EXPLAIN QUERY PLAN SELECT content_hash, created_at FROM chunk WHERE doc_id = 'd' AND layer = 'raw' AND idx = 0")
        .expect("prepare")
        .query_map([], |r| r.get::<_, String>(3))
        .expect("plan")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert!(
        plan.iter().all(|step| step.contains("USING INDEX") || step.contains("USING COVERING INDEX")),
        "lookup must not scan: {plan:?}"
    );
}
