#!/usr/bin/env python3
"""
Build (or rebuild) the Meilisearch workbench index from local .md files.

Scans: raw/, distilled/, digest/, diagnose/

Usage:
    python3 scripts/build_workbench_index.py        # full rebuild (always wipes)
    python3 scripts/build_workbench_index.py --wipe # explicit wipe (same)
"""

import argparse
import json
import re
import sys
import time
import urllib.request
from pathlib import Path

# ── Config ────────────────────────────────────────────────────────────────────
REPO_ROOT  = Path(__file__).parent.parent.resolve()
CACHE_DIR  = REPO_ROOT / '.cache'
ENV_FILE   = CACHE_DIR / 'meili.env'

MEILI_URL  = 'http://localhost:7700'
MEILI_KEY  = ''

SCAN_LAYERS = ['raw', 'distilled', 'digest', 'diagnose']
SKIP_FILES  = {'_index.md', 'README.md', 'readme.md'}
BATCH_SIZE  = 100


def _load_env():
    global MEILI_URL, MEILI_KEY
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


def _wait_for_task(task_uid, max_wait=60):
    """Poll Meilisearch task until done or timeout."""
    for _ in range(max_wait):
        r = _req('GET', f'/tasks/{task_uid}')
        if r.get('status') in ('succeeded', 'failed', 'canceled'):
            return r
        time.sleep(1)
    return None


def _wipe_index():
    try:
        task = _req('DELETE', '/indexes/workbench')
        print('  Wiping existing index…')
        uid = task.get('taskUid', task.get('uid', 0))
        _wait_for_task(uid, max_wait=30)
        print('  Index wiped.')
    except Exception:
        pass  # Index may not exist yet


def _ensure_index():
    """Create index if it doesn't exist; configure searchable + filterable attributes."""
    try:
        _req('GET', '/indexes/workbench')
    except Exception:
        task = _req('POST', '/indexes', {'uid': 'workbench', 'primaryKey': 'id'})
        _wait_for_task(task.get('taskUid', task.get('uid', 0)))

    task = _req('PUT', '/indexes/workbench/settings/searchable-attributes',
                ['title', 'body', 'topic'])
    _wait_for_task(task.get('taskUid', task.get('uid', 0)))

    task = _req('PUT', '/indexes/workbench/settings/filterable-attributes',
                ['layer', 'common_path'])
    _wait_for_task(task.get('taskUid', task.get('uid', 0)))


def _extract_title(content, filename):
    """Extract first # heading from content; fall back to filename slug."""
    for line in content.splitlines():
        line = line.strip()
        if line.startswith('# '):
            return line[2:].strip()
    stem = Path(filename).stem
    return re.sub(r'^\d{8,14}-', '', stem).replace('-', ' ')


def _make_id(layer, common_path):
    raw_id = f'{layer}__{common_path}'
    return re.sub(r'[^a-zA-Z0-9\-_]', '_', raw_id)[:511]


def _collect_docs():
    docs = []
    for layer in SCAN_LAYERS:
        layer_dir = REPO_ROOT / layer
        if not layer_dir.is_dir():
            print(f'  {layer}: (directory not found, skipped)')
            continue
        count = 0
        for md_file in sorted(layer_dir.rglob('*.md')):
            if md_file.name in SKIP_FILES:
                continue
            common_path = str(md_file.relative_to(layer_dir))
            m = re.match(r'(\d{8})', md_file.name)
            date = m.group(1) if m else ''
            parts = common_path.split('/')
            topic = parts[0] if len(parts) > 1 else ''
            try:
                body = md_file.read_text(encoding='utf-8', errors='replace')
            except Exception:
                continue
            title = _extract_title(body, md_file.name)
            docs.append({
                'id':          _make_id(layer, common_path),
                'layer':       layer,
                'common_path': common_path,
                'title':       title,
                'date':        date,
                'topic':       topic,
                'body':        body,
            })
            count += 1
        print(f'  {layer}: {count} files')
    return docs


def _upsert_docs(docs):
    total = 0
    for i in range(0, len(docs), BATCH_SIZE):
        batch = docs[i:i + BATCH_SIZE]
        task = _req('POST', '/indexes/workbench/documents', batch)
        _wait_for_task(task.get('taskUid', task.get('uid', 0)), max_wait=120)
        total += len(batch)
    return total


def main():
    _load_env()
    parser = argparse.ArgumentParser(description='Build Meilisearch workbench index')
    parser.add_argument('--wipe', action='store_true', help='Wipe index before rebuild')
    args = parser.parse_args()

    print(f'Meilisearch: {MEILI_URL}')
    print(f'Repo root:   {REPO_ROOT}')
    print(f'Scan layers: {SCAN_LAYERS}')

    _wipe_index()
    _ensure_index()

    print('Collecting documents…')
    docs = _collect_docs()
    print(f'Total files found: {len(docs)}')

    if not docs:
        print('No documents found.')
        sys.exit(0)

    print('Uploading to Meilisearch…')
    total = _upsert_docs(docs)
    print(f'Total indexed: {total} documents')


if __name__ == '__main__':
    main()
