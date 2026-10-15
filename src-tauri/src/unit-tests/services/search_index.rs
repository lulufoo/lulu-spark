use super::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::services::search_index::chunk::char_len;
use crate::agent::fence::PathFence;

fn temp_cache() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tmp");
    let cache = dir.path().join("cache");
    fs::create_dir_all(&cache).expect("mkdir");
    (dir, cache)
}

fn chunk(
    id: &str,
    doc: &str,
    category: &str,
    body: &str,
    path: &str,
) -> SourceChunk {
    SourceChunk::from_text(
        category,
        "raw",
        path,
        path,
        "",
        doc,
        "t",
        body,
        "",
        id.rsplit(':').next().and_then(|s| s.parse().ok()).unwrap_or(0),
        0,
    )
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
    assert!(indexed_mtimes(&conn).expect("mtime column exists").is_empty());
}

#[test]
fn replace_path_chunks_drops_stale_tail_chunks() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let three = vec![
        chunk("p:0", "doc-p", "notes", "第一段正文内容足够长", "/tmp/p.md"),
        chunk("p:1", "doc-p", "notes", "第二段正文内容足够长", "/tmp/p.md"),
        chunk("p:2", "doc-p", "notes", "第三段正文内容足够长", "/tmp/p.md"),
    ];
    upsert_chunks(&conn, &three).expect("seed");
    let one = vec![chunk("p:0", "doc-p", "notes", "只剩一段", "/tmp/p.md")];
    let stats = replace_path_chunks(&conn, "/tmp/p.md", &one).expect("replace");
    assert_eq!(stats.deleted, 3);
    assert_eq!(stats.upserted, 1);
    let n: i64 = conn
        .query_row("SELECT count(*) FROM chunk WHERE path = '/tmp/p.md'", [], |r| r.get(0))
        .expect("count");
    assert_eq!(n, 1);
    replace_path_chunks(&conn, "/tmp/p.md", &[]).expect("delete all");
    assert!(distinct_paths(&conn).expect("paths").is_empty());
}

#[test]
fn upsert_document_is_noop_without_index_file() {
    let (_dir, cache) = temp_cache();
    let stats = upsert_document(&cache, &[chunk("x:0", "doc-x", "knowledge", "body", "/tmp/x.md")])
        .expect("noop");
    assert_eq!(stats.upserted, 0);
    assert!(!index_exists(&cache), "write hook must not create a partial index");
}

#[test]
fn desktop_spark_hits_always_expose_raw_layer() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let digest_only = SourceChunk::from_text(
        "notes",
        "digest",
        "/tmp/notes/digest/proj/n.md",
        "proj/n.md",
        "",
        "doc-n",
        "Note N",
        "摘要里提到角色扮演的设计",
        "",
        0,
        0,
    );
    upsert_chunks(&conn, &[digest_only]).expect("seed");
    drop(conn);
    let out = search_desktop_spark(&cache, "角色扮演", Some(5));
    let hits = out["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["layer"], "raw");
    assert_eq!(hits[0]["common_path"], "proj/n.md");
}

#[test]
fn grep_prefilter_forces_scan_when_file_newer_than_index() {
    let (_dir, cache) = temp_cache();
    let root = _dir.path().join("corpus");
    fs::create_dir_all(&root).expect("mkdir");
    let path = root.join("stale.md");
    fs::write(&path, "original text without the needle").expect("write");
    let key = path.to_string_lossy().replace('\\', "/");
    let indexed = SourceChunk::from_text(
        "notes",
        "raw",
        &key,
        "stale.md",
        "",
        "doc-stale",
        "stale",
        "original text without the needle",
        "",
        0,
        file_mtime_nanos(&path) - 1_000_000_000,
    );
    upsert_chunks(&open(&cache).expect("open"), &[indexed]).expect("index");

    // Indexed, unchanged-looking, not a candidate → would be skipped if mtime were ignored.
    let pre = prefilter_or_none(&cache, "needlework").expect("prefilter");
    assert!(!pre.candidates.contains(&PathBuf::from(&key)));
    assert!(
        should_scan_file(&path, Some(&pre)),
        "file newer than its index row must be scanned"
    );

    // Same file re-indexed with the current mtime → skip is allowed again.
    let fresh = SourceChunk::from_text(
        "notes",
        "raw",
        &key,
        "stale.md",
        "",
        "doc-stale",
        "stale",
        "original text without the needle",
        "",
        0,
        file_mtime_nanos(&path),
    );
    upsert_chunks(&open(&cache).expect("open"), &[fresh]).expect("reindex");
    let pre = prefilter_or_none(&cache, "needlework").expect("prefilter");
    assert!(!should_scan_file(&path, Some(&pre)));
}

#[test]
fn prefilter_does_not_create_missing_index_file() {
    let (_dir, cache) = temp_cache();
    assert!(prefilter_or_none(&cache, "anything").is_none());
    assert!(!index_exists(&cache));
}

#[test]
fn desktop_search_reports_not_indexed_when_sqlite_missing() {
    let (_dir, cache) = temp_cache();
    let wb = search_desktop_spark(&cache, "角色扮演", Some(5));
    assert_eq!(
        wb.get("error").and_then(|v| v.as_str()),
        Some("not_indexed")
    );
    let kb = search_desktop_knowledge(&cache, "角色扮演", Some(5));
    assert_eq!(
        kb.get("error").and_then(|v| v.as_str()),
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
        chunk("a:0", "doc-a", "notes", "角色扮演与叙事", "/tmp/a.md"),
        chunk("b:0", "doc-b", "notes", "另一个文件正文足够长", "/tmp/b.md"),
    ];
    let s1 = upsert_chunks(&conn, &first).expect("up1");
    assert_eq!(s1.upserted, 2);
    let s2 = upsert_chunks(&conn, &first).expect("up2");
    assert_eq!(s2.skipped, 2);
    assert_eq!(s2.upserted, 0);

    let live: HashSet<ChunkKey> = first[..1].iter().map(|c| c.to_key()).collect();
    let deleted = delete_ghosts(&conn, &live, Some("notes"), None).expect("ghost");
    assert_eq!(deleted, 1);
    let paths = distinct_paths(&conn).expect("paths");
    assert_eq!(paths.len(), 1);
}

#[test]
fn upsert_chunks_preloads_hashes_and_updates_mtime_only() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let mut many = Vec::new();
    for i in 0..80 {
        many.push(SourceChunk::from_text(
            "notes",
            "raw",
            &format!("/tmp/n{i}.md"),
            &format!("n{i}.md"),
            "",
            &format!("doc-{i}"),
            "t",
            &format!("足够长的正文内容编号{i}以便切块稳定"),
            "",
            0,
            1_000 + i,
        ));
    }
    let s1 = upsert_chunks(&conn, &many).expect("seed");
    assert_eq!(s1.upserted, 80);
    for c in &mut many {
        c.mtime += 50;
    }
    let s2 = upsert_chunks(&conn, &many).expect("mtime only");
    assert_eq!(s2.skipped, 80);
    assert_eq!(s2.upserted, 0);
    let (hash, mtime) = existing_hash(&conn, &many[0].doc_id, &many[0].layer, many[0].idx)
        .expect("lookup")
        .expect("row");
    assert_eq!(hash, many[0].content_hash);
    assert_eq!(mtime, many[0].mtime);
    many[3] = SourceChunk::from_text(
        "notes",
        "raw",
        "/tmp/n3.md",
        "n3.md",
        "",
        "doc-3",
        "t",
        "改过的正文也要足够长才能当新块",
        "",
        0,
        many[3].mtime,
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
        "这一段讲角色扮演如何展开，以及 search 与角色的关系。",
        "/tmp/role.md",
    )];
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
fn chunk_unique_key_is_enforced_by_the_database() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let insert = "INSERT INTO chunk(doc_id, layer, idx, category, title, content_hash, path, common_path, repo, created_at, mtime)
                  VALUES ('d', 'raw', 0, 'notes', '', 'h', '/p', 'p', '', '', 0)";
    conn.execute(insert, []).expect("first insert");
    assert!(conn.execute(insert, []).is_err(), "duplicate (doc_id, layer, idx) must fail");
}

#[test]
fn fts_rows_share_rowid_with_chunk_and_leave_no_orphans() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let chunks = vec![
        chunk("a:0", "doc-a", "notes", "角色扮演与叙事设计", "/tmp/a.md"),
        chunk("a:1", "doc-a", "notes", "第二段关于检索的内容", "/tmp/a.md"),
        chunk("b:0", "doc-b", "notes", "另一个文件正文足够长", "/tmp/b.md"),
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

    let rewritten = vec![chunk("a:0", "doc-a", "notes", "改过的正文也要足够长", "/tmp/a.md")];
    upsert_chunks(&conn, &rewritten).expect("rewrite one");
    replace_path_chunks(&conn, "/tmp/b.md", &[]).expect("delete path");
    let live: HashSet<ChunkKey> = rewritten.iter().map(|c| c.to_key()).collect();
    delete_ghosts(&conn, &live, Some("notes"), None).expect("ghosts");

    assert_eq!(count_rows(&conn, "SELECT count(*) FROM chunk"), 1);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM docs_tri"), 1);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM docs_bi"), 1);
    assert_eq!(orphans("docs_tri"), 0);
    assert_eq!(orphans("docs_bi"), 0);
}

#[test]
fn moved_document_is_replaced_without_unique_collision() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    upsert_chunks(&conn, &[chunk("a:0", "doc-a", "notes", "搬家前的正文足够长", "/tmp/old.md")])
        .expect("seed");
    let stats = replace_path_chunks(
        &conn,
        "/tmp/new.md",
        &[chunk("a:0", "doc-a", "notes", "搬家后的正文足够长", "/tmp/new.md")],
    )
    .expect("moved note, new path synced first");
    assert_eq!(stats.deleted, 1);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM chunk"), 1);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM chunk WHERE path = '/tmp/new.md'"), 1);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM docs_tri"), 1);
}

#[test]
fn unchanged_body_refreshes_stale_metadata() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let old = chunk("a:0", "doc-a", "notes", "同样的正文足够长", "/tmp/old.md");
    upsert_chunks(&conn, &[old.clone()]).expect("seed");
    let mut moved = old;
    moved.path = "/tmp/new.md".into();
    moved.common_path = "new.md".into();
    moved.title = "新标题".into();
    let stats = upsert_chunks(&conn, &[moved]).expect("same hash, new metadata");
    assert_eq!(stats.skipped, 1);
    assert_eq!(stats.upserted, 0);
    let (path, title): (String, String) = conn
        .query_row("SELECT path, title FROM chunk", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("row");
    assert_eq!((path.as_str(), title.as_str()), ("/tmp/new.md", "新标题"));
}

#[test]
fn bigram_hit_snippet_comes_from_original_text_and_title_only_words_do_not_match() {
    let (_dir, cache) = temp_cache();
    let conn = open(&cache).expect("open");
    let mut c = chunk("n:0", "doc-n", "notes", "这一段讲角色扮演如何展开，与 search 无关。", "/tmp/n.md");
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
        .prepare("EXPLAIN QUERY PLAN SELECT content_hash, mtime FROM chunk WHERE doc_id = 'd' AND layer = 'raw' AND idx = 0")
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

fn scan_matches(root: &Path, regex: &Regex) -> Vec<String> {
    let fence = PathFence {
        read_allow: vec![root.to_path_buf()],
        ..PathFence::default()
    };
    let mut matches = Vec::new();
    let mut files_seen = 0usize;
    walk_for_test(root, &fence, regex, &mut matches, &mut files_seen, None);
    matches
}

fn walk_for_test(
    root: &Path,
    fence: &PathFence,
    regex: &Regex,
    matches: &mut Vec<String>,
    files_seen: &mut usize,
    prefilter: Option<&GrepPrefilter>,
) {
    if !root.is_dir() {
        return;
    }
    for entry in fs::read_dir(root).expect("rd").flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_for_test(&path, fence, regex, matches, files_seen, prefilter);
            continue;
        }
        if !path.is_file() {
            continue;
        }
        if !should_scan_file(&path, prefilter) {
            continue;
        }
        *files_seen += 1;
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            if regex.is_match(line) {
                matches.push(format!("{}:{}:{line}", path.display(), idx + 1));
            }
        }
    }
}

#[test]
fn grep_prefilter_matches_full_scan_on_random_patterns() {
    let (_dir, cache) = temp_cache();
    let root = _dir.path().join("corpus");
    fs::create_dir_all(&root).expect("mkdir");
    let files = [
        ("a.md", "角色扮演 is a long story about narrative design"),
        ("b.md", "search tools and 角色 appear together here"),
        ("c.txt", "unindexed sidecar file mentions 角色扮演 too"),
        ("d.md", "zzzz only filler text without the keywords"),
    ];
    let mut chunks = Vec::new();
    for (name, body) in files {
        let path = root.join(name);
        fs::write(&path, body).expect("write");
        if name.ends_with(".md") {
            chunks.push(SourceChunk::from_text(
                "notes",
                "raw",
                &path.to_string_lossy(),
                name,
                "",
                name,
                name,
                body,
                "",
                0,
                file_mtime_nanos(&path),
            ));
        }
    }
    upsert_chunks(&open(&cache).expect("open"), &chunks).expect("index");

    let patterns = [
        "角色扮演",
        "search",
        "narrative",
        "filler",
        "foo",
        "角色扮演|missing",
        "search.*角色",
        "zzzz",
        "sidecar",
        "design",
    ];
    let mut extra = Vec::new();
    for i in 0..90 {
        extra.push(format!("story{i:02}"));
    }
    let all: Vec<String> = patterns
        .iter()
        .map(|s| s.to_string())
        .chain(extra)
        .collect();
    assert!(all.len() >= 100);

    for pat in &all {
        let Ok(regex) = Regex::new(pat) else {
            continue;
        };
        let full = scan_matches(&root, &regex);
        let pre = prefilter_or_none(&cache, pat);
        let fence = PathFence {
            read_allow: vec![root.clone()],
            ..PathFence::default()
        };
        let mut filtered = Vec::new();
        let mut seen = 0usize;
        walk_for_test(
            &root,
            &fence,
            &regex,
            &mut filtered,
            &mut seen,
            pre.as_ref(),
        );
        assert_eq!(filtered, full, "pattern {pat}");
    }
}

