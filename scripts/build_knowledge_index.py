#!/usr/bin/env python3
"""
Build (or rebuild) the Meilisearch knowledge index from all layer-3 repos.

Usage:
    python3 scripts/build_knowledge_index.py          # upsert (incremental)
    python3 scripts/build_knowledge_index.py --wipe   # delete index then rebuild
"""

import argparse
import json
import re
import subprocess
import sys
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

# ── Config ────────────────────────────────────────────────────────────────────
REPO_ROOT = Path(__file__).parent.parent.resolve()
CACHE_DIR = REPO_ROOT / '.cache'
ENV_FILE  = CACHE_DIR / 'meili.env'
META_FILE = CACHE_DIR / 'meili-meta.json'
COMMIT_CACHE_FILE = CACHE_DIR / 'repo-commits.json'
TOPICS_FILE = REPO_ROOT / 'topics.json'

MEILI_URL = 'http://localhost:7700'
MEILI_KEY = ''
KNOWLEDGE_BASE_DIR = Path('/Users/lulu/Code')

SKIP_FILES = {'_index.md', 'README.md', 'readme.md'}
BATCH_SIZE = 100


def _load_env():
    global MEILI_URL, MEILI_KEY, KNOWLEDGE_BASE_DIR
    if not ENV_FILE.exists():
        return
    for line in ENV_FILE.read_text(encoding='utf-8').splitlines():
        line = line.strip()
        if not line or line.startswith('#') or '=' not in line:
            continue
        key, _, val = line.partition('=')
        key, val = key.strip(), val.strip()
        if key == 'MEILI_MASTER_KEY':
            MEILI_KEY = val
        elif key == 'MEILI_URL':
            MEILI_URL = val
        elif key == 'KNOWLEDGE_BASE_DIR':
            KNOWLEDGE_BASE_DIR = Path(val)


# Bypass system proxy for localhost Meilisearch requests
_no_proxy_opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def _req(method, path, body=None):
    """Raw HTTP request to Meilisearch. Raises on error."""
    url = MEILI_URL.rstrip('/') + path
    data = json.dumps(body).encode('utf-8') if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header('Authorization', f'Bearer {MEILI_KEY}')
    req.add_header('Content-Type', 'application/json')
    with _no_proxy_opener.open(req, timeout=15) as resp:
        return json.loads(resp.read().decode('utf-8'))


def _wait_for_task(task_uid, max_wait=30):
    """Poll Meilisearch task until done or timeout."""
    import time
    for _ in range(max_wait):
        r = _req('GET', f'/tasks/{task_uid}')
        if r.get('status') in ('succeeded', 'failed', 'canceled'):
            return r
        time.sleep(1)
    return None


def _ensure_index():
    """Create index if it doesn't exist, configure searchable + filterable attributes."""
    try:
        _req('GET', '/indexes/knowledge')
    except Exception:
        task = _req('POST', '/indexes', {'uid': 'knowledge', 'primaryKey': 'id'})
        _wait_for_task(task.get('taskUid', task.get('uid', 0)))

    # Set searchable attributes with priority order
    task = _req('PUT', '/indexes/knowledge/settings/searchable-attributes',
                ['title', 'topic_desc', 'body'])
    _wait_for_task(task.get('taskUid', task.get('uid', 0)))

    # Set filterable attributes (required for per-repo delete filter)
    task = _req('PUT', '/indexes/knowledge/settings/filterable-attributes', ['repo'])
    _wait_for_task(task.get('taskUid', task.get('uid', 0)))


def _wipe_index():
    try:
        task = _req('DELETE', '/indexes/knowledge')
        print('  Wiping existing index…')
        _wait_for_task(task.get('taskUid', task.get('uid', 0)))
    except Exception:
        pass  # index may not exist yet


def _get_repo_head(local_dir):
    """Return HEAD commit hash for local git repo, or None on error."""
    r = subprocess.run(
        ['git', '-C', str(local_dir), 'rev-parse', 'HEAD'],
        capture_output=True, text=True
    )
    return r.stdout.strip() if r.returncode == 0 else None


def _load_commit_cache():
    """Return {repo_name: commit_hash} dict from cache file."""
    if not COMMIT_CACHE_FILE.exists():
        return {}
    try:
        return json.loads(COMMIT_CACHE_FILE.read_text(encoding='utf-8'))
    except Exception:
        return {}


def _save_commit_cache(cache):
    CACHE_DIR.mkdir(exist_ok=True)
    COMMIT_CACHE_FILE.write_text(json.dumps(cache, indent=2), encoding='utf-8')


def _delete_repo_docs(repo_full):
    """Delete all Meilisearch documents for a repo using filter."""
    task = _req('POST', '/indexes/knowledge/documents/delete',
                {'filter': f"repo = '{repo_full}'"})
    _wait_for_task(task.get('taskUid', task.get('uid', 0)), max_wait=60)


def _extract_title(content, filename):
    for line in content.splitlines():
        m = re.match(r'^#\s+(.+)', line)
        if m:
            return m.group(1).strip()
    return re.sub(r'\.md$', '', filename)


def _build_docs(repos_meta):
    """Yield document dicts from all repos."""
    for repo_full, topic_desc in repos_meta:
        repo_name = repo_full.split('/')[-1]
        local_dir = KNOWLEDGE_BASE_DIR / repo_name
        if not local_dir.is_dir():
            print(f'  [skip] {repo_name}: local dir not found at {local_dir}')
            continue
        count = 0
        for md_file in sorted(local_dir.rglob('*.md')):
            if md_file.name in SKIP_FILES:
                continue
            try:
                rel_path = md_file.relative_to(local_dir).as_posix()
                content = md_file.read_text(encoding='utf-8', errors='replace')
                title = _extract_title(content, md_file.name)
                raw_id = f"{repo_name}__{rel_path.replace('/', '__')}"
                doc_id = re.sub(r'[^a-zA-Z0-9\-_]', '_', raw_id)[:511]
                url = f'https://github.com/{repo_full}/blob/main/{rel_path}'
                yield {
                    'id': doc_id,
                    'title': title,
                    'body': content,
                    'repo': repo_full,
                    'path': rel_path,
                    'url': url,
                    'topic_desc': topic_desc,
                }
                count += 1
            except Exception as e:
                print(f'  [warn] {md_file}: {e}')
        print(f'  {repo_name}: {count} files')


def _send_batch(docs):
    task = _req('PUT', '/indexes/knowledge/documents', docs)
    _wait_for_task(task.get('taskUid', task.get('uid', 0)))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--wipe', action='store_true', help='Delete index before rebuilding')
    args = parser.parse_args()

    _load_env()

    # Verify Meilisearch is reachable
    try:
        health = _req('GET', '/health')
        if health.get('status') != 'available':
            print(f'ERROR: Meilisearch not available: {health}')
            sys.exit(1)
    except Exception as e:
        print(f'ERROR: Cannot reach Meilisearch at {MEILI_URL}: {e}')
        sys.exit(1)

    print(f'Meilisearch: {MEILI_URL}')
    print(f'Knowledge base: {KNOWLEDGE_BASE_DIR}')

    if args.wipe:
        _wipe_index()
        COMMIT_CACHE_FILE.unlink(missing_ok=True)
        print('  Commit cache cleared.')

    _ensure_index()

    # Load topics
    topics_data = json.loads(TOPICS_FILE.read_text(encoding='utf-8'))
    repos_meta = [
        (t['repo'], t.get('description', ''))
        for t in topics_data.get('topics', [])
        if 'repo' in t
    ]
    print(f'Repos to index: {len(repos_meta)}')

    commit_cache = _load_commit_cache()
    new_cache = {}
    total = 0

    for repo_full, topic_desc in repos_meta:
        repo_name = repo_full.split('/')[-1]
        local_dir = KNOWLEDGE_BASE_DIR / repo_name
        if not local_dir.is_dir():
            print(f'  [skip] {repo_name}: local dir not found')
            continue

        head = _get_repo_head(local_dir)
        cached_head = commit_cache.get(repo_name)

        if not args.wipe and head and cached_head == head:
            print(f'  [skip] {repo_name}: commit unchanged ({head[:8]})')
            new_cache[repo_name] = head
            continue

        old_short = cached_head[:8] if cached_head else 'new'
        new_short = head[:8] if head else '?'
        print(f'  [index] {repo_name}: {old_short} → {new_short}')

        # Delete existing docs for this repo, then upsert fresh
        _delete_repo_docs(repo_full)

        batch = []
        count = 0
        for doc in _build_docs([(repo_full, topic_desc)]):
            batch.append(doc)
            count += 1
            total += 1
            if len(batch) >= BATCH_SIZE:
                _send_batch(batch)
                batch = []
        if batch:
            _send_batch(batch)
        print(f'    {count} files indexed')

        if head:
            new_cache[repo_name] = head

    _save_commit_cache(new_cache)
    print(f'Total indexed this run: {total} documents')

    # Write meta
    CACHE_DIR.mkdir(exist_ok=True)
    META_FILE.write_text(
        json.dumps({
            'last_indexed_at': datetime.now(timezone.utc).isoformat(),
            'doc_count': total,
        }, indent=2),
        encoding='utf-8'
    )
    print(f'Meta written to {META_FILE}')


if __name__ == '__main__':
    main()
