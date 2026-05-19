#!/usr/bin/env python3
"""
Local dev server for lulu-workbench viewer.
Serves static files + provides API for file editing and git commit/push.

Usage:
    python3 server.py
    # open http://localhost:8765
"""

import http.server
import json
import os
import re
import shutil
import subprocess
import sys
import urllib.parse
import base64
import threading
import time
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone, timedelta
from pathlib import Path

REPO_ROOT = Path(__file__).parent.resolve()
CACHE_DIR = REPO_ROOT / '.cache'
REPO_LIST_CACHE_FILE = CACHE_DIR / 'repo-list.json'
TOPICS_CACHE_FILE = CACHE_DIR / 'topics.json'
EDITABLE_LAYERS = {'raw', 'distilled', 'digest', 'trace'}
PORT = 8765

_SCRIPTS = REPO_ROOT / 'scripts'
if str(_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS))
from knowledge_index_loader import (  # noqa: E402
    knowledge_index_path,
    load_knowledge_index,
    load_topics,
)

# ── Meilisearch config ────────────────────────────────────────────────────────
MEILI_URL = 'http://localhost:7700'
MEILI_KEY = ''
KNOWLEDGE_BASE_DIR = Path('/Users/lulu/Code')
KNOWLEDGE_CORPUS_DIR = REPO_ROOT
KNOWLEDGE_CORPUS_GITHUB = 'https://github.com/lulufoo/lulu-workbench-knowledge/blob/main'

from workbench_config import (  # noqa: E402
    get_corpus_github as _get_corpus_github,
    load_meili_env as _load_meili_env_dict,
)

_CORPUS_LAYER_NAMES = frozenset({'raw', 'distilled', 'digest', 'trace', 'diagnose', 'annotations'})


def _load_meili_config():
    global MEILI_URL, MEILI_KEY, KNOWLEDGE_BASE_DIR, KNOWLEDGE_CORPUS_DIR, KNOWLEDGE_CORPUS_GITHUB
    env = _load_meili_env_dict(REPO_ROOT)
    if 'MEILI_MASTER_KEY' in env:
        MEILI_KEY = env['MEILI_MASTER_KEY']
    if 'MEILI_URL' in env:
        MEILI_URL = env['MEILI_URL']
    if 'KNOWLEDGE_BASE_DIR' in env:
        KNOWLEDGE_BASE_DIR = Path(env['KNOWLEDGE_BASE_DIR'])
    if 'KNOWLEDGE_CORPUS_DIR' in env:
        KNOWLEDGE_CORPUS_DIR = Path(env['KNOWLEDGE_CORPUS_DIR'])
    KNOWLEDGE_CORPUS_GITHUB = _get_corpus_github(REPO_ROOT)


_load_meili_config()
if not KNOWLEDGE_CORPUS_DIR.exists():
    print(f'Warning: KNOWLEDGE_CORPUS_DIR does not exist: {KNOWLEDGE_CORPUS_DIR}')


def _corpus_git_root() -> Path:
    """Git working tree for sync menu (commit / pull / status)."""
    root = KNOWLEDGE_CORPUS_DIR.resolve()
    if not (root / '.git').exists():
        raise ValueError(f'corpus is not a git repository: {root}')
    return root


# ── Meilisearch state ─────────────────────────────────────────────────────────
_reindex_job    = {'status': 'idle', 'started_at': None, 'finished_at': None, 'log': ''}
_reindex_wb_job = {'status': 'idle', 'started_at': None, 'finished_at': None, 'log': ''}


import urllib.request as _ureq
# Bypass system proxy for localhost Meilisearch requests
_meili_opener = _ureq.build_opener(_ureq.ProxyHandler({}))


def _meili_request(method, path, body=None):
    """Send a request to Meilisearch. Returns parsed dict or None on connection failure."""
    import urllib.error as _uerr
    try:
        url = MEILI_URL.rstrip('/') + path
        data = json.dumps(body).encode('utf-8') if body is not None else None
        req = _ureq.Request(url, data=data, method=method)
        req.add_header('Authorization', f'Bearer {MEILI_KEY}')
        req.add_header('Content-Type', 'application/json')
        with _meili_opener.open(req, timeout=5) as resp:
            return json.loads(resp.read().decode('utf-8'))
    except _uerr.HTTPError as e:
        # HTTP error (e.g. 404 index not found): return parsed body, not None
        try:
            return json.loads(e.read().decode('utf-8'))
        except Exception:
            return {'_http_status': e.code}
    except Exception:
        return None


def _meili_upsert_doc(repo, path, content, topic_desc=''):
    """Build a Meilisearch document from file content and upsert it."""
    repo_name = repo.split('/')[-1]
    raw_id = f"{repo_name}__{path.replace('/', '__')}"
    doc_id = re.sub(r'[^a-zA-Z0-9\-_]', '_', raw_id)[:511]
    title = ''
    for line in content.splitlines():
        m = re.match(r'^#\s+(.+)', line)
        if m:
            title = m.group(1).strip()
            break
    if not title:
        title = re.sub(r'\.md$', '', path.split('/')[-1])
    url = f'https://github.com/{repo}/blob/main/{path}'
    doc = {
        'id': doc_id,
        'title': title,
        'body': content,
        'repo': repo,
        'path': path,
        'url': url,
        'topic_desc': topic_desc,
    }
    return _meili_request('PUT', '/indexes/knowledge/documents', [doc])


class Handler(http.server.SimpleHTTPRequestHandler):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(REPO_ROOT), **kwargs)

    def translate_path(self, path):
        fs_path = super().translate_path(path)
        try:
            rel = Path(fs_path).relative_to(REPO_ROOT)
        except ValueError:
            return fs_path
        if rel.as_posix() == 'index.json' or (rel.parts and rel.parts[0] in _CORPUS_LAYER_NAMES):
            return str((KNOWLEDGE_CORPUS_DIR / rel).resolve())
        return fs_path

    # ── Routing ──────────────────────────────────────────────────────────────

    def do_POST(self):
        if self.path == '/api/save':
            self._handle_save()
        elif self.path == '/api/commit':
            self._handle_commit()
        elif self.path == '/api/pull':
            self._handle_pull()
        elif self.path == '/api/update-links':
            self._handle_update_links()
        elif self.path == '/api/update-comments':
            self._handle_update_comments()
        elif self.path == '/api/reorder-comments':
            self._handle_reorder_comments()
        elif self.path == '/api/update-highlights':
            self._handle_update_highlights()
        elif self.path == '/api/delete':
            self._handle_delete()
        elif self.path == '/api/gh-move':
            self._handle_gh_move()
        elif self.path == '/api/set-done':
            self._handle_set_done()
        elif self.path == '/api/set-importance':
            self._handle_set_importance()
        elif self.path == '/api/move-project':
            self._handle_move_project()
        elif self.path == '/api/settle':
            self._handle_settle()
        elif self.path == '/api/update-topics':
            self._handle_update_topics()
        elif self.path == '/api/reindex-knowledge':
            self._handle_reindex_knowledge()
        elif self.path == '/api/reindex-workbench':
            self._handle_reindex_workbench()
        elif self.path == '/api/draft':
            self._handle_save_draft()
        elif self.path == '/api/kb/save':
            self._handle_kb_save()
        elif self.path == '/api/kb/commit':
            self._handle_kb_commit()
        elif self.path == '/api/kb/revert':
            self._handle_kb_revert()
        elif self.path == '/api/kb/reindex':
            self._handle_kb_reindex()
        elif self.path == '/api/kb/update-comments':
            self._handle_kb_update_comments()
        elif self.path == '/api/kb/reorder-comments':
            self._handle_kb_reorder_comments()
        elif self.path == '/api/kb/update-highlights':
            self._handle_kb_update_highlights()
        elif self.path == '/api/kb/update-links':
            self._handle_kb_update_links()
        elif self.path == '/api/open-iterm':
            self._handle_open_iterm()
        else:
            self.send_error(404)

    def do_GET(self):
        parsed_path = urllib.parse.urlparse(self.path).path
        if parsed_path.startswith('/api/fetch-title'):
            self._handle_fetch_title()
        elif parsed_path == '/api/reindex-status':
            self._handle_reindex_status()
        elif parsed_path == '/api/reindex-workbench-status':
            self._handle_reindex_workbench_status()
        else:
            super().do_GET()

    def end_headers(self):
        # Disable caching for JS/CSS/HTML so edits take effect immediately
        ext = self.path.split('?')[0].rsplit('.', 1)[-1].lower()
        if ext in ('js', 'css', 'html'):
            self.send_header('Cache-Control', 'no-store')
        super().end_headers()

    def do_OPTIONS(self):
        self.send_response(204)
        self._cors_headers()
        self.end_headers()

    # ── Helpers ───────────────────────────────────────────────────────────────

    def _read_json(self):
        length = int(self.headers.get('Content-Length', 0))
        return json.loads(self.rfile.read(length))

    def _json_response(self, data, status=200):
        body = json.dumps(data, ensure_ascii=False).encode('utf-8')
        self.send_response(status)
        self._cors_headers()
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _cors_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')

    def log_message(self, fmt, *args):
        print(f'  [{self.address_string()}] {fmt % args}')

    # ── API: save file ────────────────────────────────────────────────────────

    def _handle_save(self):
        try:
            data = self._read_json()
            layer = data.get('layer', '').strip()
            common_path = data.get('common_path', '').strip()
            content = data.get('content', '')

            if layer not in EDITABLE_LAYERS:
                self._json_response({'error': f'Invalid layer: {layer}'}, 400)
                return

            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid path'}, 400)
                return

            # Resolve and verify path stays inside repo root (prevent traversal)
            target = (KNOWLEDGE_CORPUS_DIR / layer / common_path).resolve()
            if not str(target).startswith(str(KNOWLEDGE_CORPUS_DIR) + os.sep):
                self._json_response({'error': 'Path traversal not allowed'}, 400)
                return

            if not target.exists():
                self._json_response({'error': f'File not found: {layer}/{common_path}'}, 404)
                return

            target.write_text(content, encoding='utf-8')
            print(f'  [save] {layer}/{common_path}')
            self._json_response({'ok': True})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: git commit + push ─────────────────────────────────────────────────

    def _handle_commit(self):
        try:
            try:
                git_root = _corpus_git_root()
            except ValueError as e:
                self._json_response({'error': str(e)}, 400)
                return

            data = self._read_json()
            message = data.get('message', '').strip() or 'update: edit via viewer'

            def run(cmd):
                return subprocess.run(
                    cmd, cwd=str(git_root),
                    capture_output=True, text=True
                )

            files = data.get('files')  # optional list of specific file paths
            if files and isinstance(files, list):
                add = run(['git', 'add', '--'] + [f for f in files if f])
            else:
                add = run(['git', 'add', '-A'])
            if add.returncode != 0:
                self._json_response({
                    'error': 'git add failed',
                    'stderr': add.stderr
                }, 500)
                return

            commit = run(['git', 'commit', '-m', message])
            nothing_to_commit = (
                'nothing to commit' in commit.stdout or
                'nothing to commit' in commit.stderr
            )
            if commit.returncode != 0 and not nothing_to_commit:
                self._json_response({
                    'error': 'git commit failed',
                    'stderr': commit.stderr,
                    'stdout': commit.stdout
                }, 500)
                return

            if nothing_to_commit:
                # Still push in case there are local commits not yet pushed
                push = run(['git', 'push'])
                if push.returncode != 0:
                    self._json_response({
                        'error': 'git push failed',
                        'stderr': push.stderr
                    }, 500)
                    return
                self._json_response({'ok': True, 'info': 'nothing to commit'})
                return

            pull = run(['git', 'pull', '--rebase'])
            if pull.returncode != 0:
                self._json_response({
                    'error': 'git pull --rebase failed',
                    'stderr': pull.stderr,
                    'stdout': pull.stdout
                }, 500)
                return

            push = run(['git', 'push'])
            if push.returncode != 0:
                self._json_response({
                    'error': 'git push failed',
                    'stderr': push.stderr
                }, 500)
                return

            print(f'  [commit+push] {message}')
            self._json_response({'ok': True, 'info': commit.stdout.strip()})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: git status ────────────────────────────────────────────────────

    def _handle_status(self):
        try:
            try:
                git_root = _corpus_git_root()
            except ValueError as e:
                self._json_response({'error': str(e)}, 400)
                return

            result = subprocess.run(
                ['git', 'status', '--porcelain'],
                cwd=str(git_root), capture_output=True, text=True
            )
            categories = {'new': [], 'modified': [], 'deleted': [], 'renamed': [], 'conflicted': []}
            for line in result.stdout.splitlines():
                if not line.strip():
                    continue
                xy = line[:2]
                path = line[3:]
                x, y = xy[0], xy[1]
                # Merge conflicts
                if xy in ('UU', 'AA', 'DD', 'AU', 'UA', 'DU', 'UD'):
                    categories['conflicted'].append(path.strip())
                # Renamed (staged or work tree)
                elif x == 'R' or y == 'R':
                    if ' -> ' in path:
                        old, new = path.split(' -> ', 1)
                        categories['renamed'].append(f'{old.strip()} → {new.strip()}')
                    else:
                        categories['renamed'].append(path.strip())
                # Deleted
                elif x == 'D' or y == 'D':
                    categories['deleted'].append(path.strip())
                # New file (staged: A) or untracked (??)
                elif x == 'A' or xy == '??':
                    categories['new'].append(path.strip())
                # Modified
                elif x == 'M' or y == 'M':
                    categories['modified'].append(path.strip())
                # Catch-all
                elif xy.strip():
                    categories['modified'].append(path.strip())
            total = sum(len(v) for v in categories.values())
            # Check local commits not yet pushed
            ahead_r = subprocess.run(
                ['git', 'rev-list', '--count', 'HEAD...@{u}'],
                cwd=str(git_root), capture_output=True, text=True
            )
            ahead = int(ahead_r.stdout.strip()) if ahead_r.returncode == 0 and ahead_r.stdout.strip().isdigit() else 0
            self._json_response({
                **categories,
                'total': total,
                'ahead': ahead,
                'archive_root': str(KNOWLEDGE_CORPUS_DIR),
            })
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: git pull --rebase ──────────────────────────────────────────────

    def _handle_knowledge_index(self, force: bool = False):
        del force  # reserved for API contract; always read from disk
        try:
            index_path = knowledge_index_path(REPO_ROOT)
            entries = load_knowledge_index(index_path)
        except FileNotFoundError:
            self._json_response({'error': 'knowledge-index.json not found'}, 404)
            return
        except ValueError as e:
            self._json_response({'error': str(e)}, 500)
            return
        except Exception as e:
            self._json_response({'error': f'invalid knowledge-index: {e}'}, 500)
            return
        if not entries:
            self._json_response({'error': 'no valid entries in knowledge-index'}, 500)
            return
        self._json_response({
            'entries': entries,
            'cached_at': datetime.now(timezone.utc).isoformat(),
        })

    def _handle_topics(self):
        try:
            topics_data = load_topics(REPO_ROOT)
        except FileNotFoundError:
            self._json_response({
                'error': (
                    'topics.json not found; run ⊙ 全量同步 or '
                    'update_topics_from_github.py'
                ),
            }, 404)
            return
        except Exception as e:
            self._json_response({'error': str(e)}, 500)
            return
        payload = dict(topics_data)
        if TOPICS_CACHE_FILE.is_file():
            mtime = TOPICS_CACHE_FILE.stat().st_mtime
            payload['cached_at'] = datetime.fromtimestamp(
                mtime, tz=timezone.utc
            ).isoformat()
        self._json_response(payload)

    def _handle_update_topics(self):
        try:
            self._read_json()  # consume body (ignored)
            try:
                index_path = knowledge_index_path(REPO_ROOT)
            except FileNotFoundError:
                self._json_response({'error': 'knowledge-index.json not found'}, 404)
                return
            script = REPO_ROOT / 'scripts' / 'update_topics_from_github.py'
            cmd = [
                sys.executable,
                str(script),
                '-o', str(TOPICS_CACHE_FILE),
                '--index-path', str(index_path),
            ]
            print('  [update-topics] full sync from knowledge-index.json')
            result = subprocess.run(
                cmd, cwd=REPO_ROOT, capture_output=True, text=True, timeout=300
            )
            print(f'  [update-topics] returncode={result.returncode}')
            if result.stdout.strip():
                print(f'  [update-topics] stdout: {result.stdout.strip()}')
            if result.stderr.strip():
                print(f'  [update-topics] stderr: {result.stderr.strip()}')
            if result.returncode != 0:
                self._json_response({
                    'error': 'update_topics_from_github.py failed',
                    'stderr': result.stderr,
                    'stdout': result.stdout
                }, 500)
                return
            print('  [update-topics] done')
            self._json_response({'ok': True, 'info': result.stderr.strip()})
        except subprocess.TimeoutExpired:
            self._json_response({'error': 'update_topics timed out after 300s'}, 500)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    def _handle_pull(self):
        try:
            try:
                git_root = _corpus_git_root()
            except ValueError as e:
                self._json_response({'error': str(e)}, 400)
                return

            self._read_json()  # consume body
            result = subprocess.run(
                ['git', 'pull', '--rebase'],
                cwd=str(git_root), capture_output=True, text=True
            )
            if result.returncode != 0:
                self._json_response({
                    'error': 'git pull --rebase failed',
                    'stderr': result.stderr,
                    'stdout': result.stdout
                }, 500)
                return
            print(f'  [pull] {result.stdout.strip()}')
            self._json_response({'ok': True, 'info': result.stdout.strip()})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)


    # ── API: fetch title from GitHub URL ──────────────────────────────────────

    def _handle_fetch_title(self):
        try:
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            url = params.get('url', [''])[0]

            if not url:
                self._json_response({'error': 'missing url'}, 400)
                return

            title = self._resolve_title(url)
            self._json_response({'title': title})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    def _resolve_title(self, url):
        # SSRF guard: reject non-https and private/localhost targets
        p = urllib.parse.urlparse(url)
        if p.scheme != 'https':
            return self._fallback_title(url)
        host = p.hostname or ''
        if host in ('localhost', '127.0.0.1', '::1') or host.startswith('192.168.') \
                or host.startswith('10.') or host.startswith('172.'):
            return self._fallback_title(url)

        # GitHub blob URL: https://github.com/{owner}/{repo}/blob/{ref}/{path}
        m = re.match(
            r'https://github\.com/([^/]+)/([^/]+)/blob/([^/]+)/(.+)',
            url
        )
        if not m:
            return self._fallback_title(url)

        owner, repo, ref, path = m.group(1), m.group(2), m.group(3), m.group(4)
        api_path = f'repos/{owner}/{repo}/contents/{path}?ref={ref}'
        try:
            result = subprocess.run(
                ['gh', 'api', api_path, '--jq', '.content'],
                capture_output=True, text=True, timeout=10
            )
            if result.returncode != 0:
                return self._fallback_title(url)
            content = base64.b64decode(result.stdout.strip()).decode('utf-8', errors='replace')
            for line in content.splitlines():
                m2 = re.match(r'^#\s+(.+)', line)
                if m2:
                    return m2.group(1).strip()
            return self._fallback_title(url)
        except Exception:
            return self._fallback_title(url)

    def _fallback_title(self, url):
        try:
            parts = urllib.parse.urlparse(url).path.rstrip('/').split('/')
            name = urllib.parse.unquote(parts[-1]) if parts else url
            return re.sub(r'\.md$', '', name)
        except Exception:
            return url

    # ── Annotation helpers ─────────────────────────────────────────────────────

    def _annotation_path(self, common_path):
        """Return resolved Path for annotation file, or None if invalid."""
        if not common_path or '..' in common_path:
            return None
        rel = (common_path[:-3] + '.json') if common_path.endswith('.md') else (common_path + '.json')
        target = (KNOWLEDGE_CORPUS_DIR / 'annotations' / rel).resolve()
        if not str(target).startswith(str(KNOWLEDGE_CORPUS_DIR) + os.sep):
            return None
        return target

    def _read_annotation(self, common_path):
        """Read annotation file; return {} if missing or invalid."""
        p = self._annotation_path(common_path)
        if p is None or not p.exists():
            return {}
        try:
            return json.loads(p.read_text(encoding='utf-8'))
        except Exception:
            return {}

    def _write_annotation(self, common_path, data):
        """Atomic write of annotation file. Deletes file when data is empty."""
        p = self._annotation_path(common_path)
        if p is None:
            raise ValueError('Invalid common_path')
        # Clean up empty layer dicts before deciding whether to persist
        for key in list(data.keys()):
            if isinstance(data[key], dict) and not data[key]:
                del data[key]
        if not data:
            if p.exists():
                p.unlink()
            return
        p.parent.mkdir(parents=True, exist_ok=True)
        tmp = p.with_suffix('.json.tmp')
        tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding='utf-8')
        tmp.replace(p)

    # ── API: GET /api/annotations — batch summary for list view ───────────────

    def _handle_get_annotations(self):
        try:
            index_path = KNOWLEDGE_CORPUS_DIR / 'index.json'
            index_data = json.loads(index_path.read_text(encoding='utf-8'))
            entries = index_data.get('entries', index_data)
            result = {}
            for entry in entries.values():
                common_path = entry.get('common_path', '')
                if not common_path:
                    continue
                ann = self._read_annotation(common_path)
                if not ann:
                    continue
                summary = {}
                if ann.get('done'):
                    summary['done'] = True
                if ann.get('importance') in ('high', 'medium', 'low'):
                    summary['importance'] = ann['importance']
                if ann.get('links'):
                    summary['links'] = ann['links']
                for layer in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
                    layer_data = ann.get(layer)
                    if not layer_data:
                        continue
                    comments = layer_data.get('comments', [])
                    if comments:
                        summary.setdefault('comment_counts', {})[layer] = len(comments)
                if summary:
                    result[common_path] = summary
            self._json_response(result)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/annotation?path=<common_path> — single full annotation ──

    def _handle_get_annotation(self):
        try:
            params = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
            common_path = params.get('path', [''])[0].strip()
            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid path'}, 400)
                return
            ann = self._read_annotation(common_path)
            self._json_response(ann)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: update links in annotation file ──────────────────────────────────

    def _handle_update_links(self):
        try:
            data = self._read_json()
            common_path = data.get('common_path', '').strip()
            links = data.get('links', [])

            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return

            # Validate each link
            for link in links:
                url = link.get('url', '')
                p = urllib.parse.urlparse(url)
                if p.scheme not in ('http', 'https') or not p.netloc:
                    self._json_response({'error': f'Invalid url: {url}'}, 400)
                    return

            ann = self._read_annotation(common_path)
            if links:
                ann['links'] = links
            else:
                ann.pop('links', None)
            self._write_annotation(common_path, ann)

            print(f'  [update-links] {common_path} → {len(links)} link(s)')
            self._json_response({'ok': True})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)


    # ── API: set done flag on annotation file ────────────────────────────────

    def _handle_set_done(self):
        try:
            data = self._read_json()
            common_path = data.get('common_path', '').strip()
            done = bool(data.get('done', False))

            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return

            ann = self._read_annotation(common_path)
            if done:
                ann['done'] = True
            else:
                ann.pop('done', None)
            self._write_annotation(common_path, ann)

            print(f'  [set-done] {common_path} → done={done}')
            self._json_response({'ok': True})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: set importance in annotation file ─────────────────────────────────

    def _handle_set_importance(self):
        try:
            data = self._read_json()
            common_path = data.get('common_path', '').strip()
            importance = data.get('importance')  # 'high' | 'medium' | 'low' | None

            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return
            if importance is not None and importance not in ('high', 'medium', 'low'):
                self._json_response({'error': f'Invalid importance: {importance}'}, 400)
                return

            ann = self._read_annotation(common_path)
            if importance:
                ann['importance'] = importance
            else:
                ann.pop('importance', None)
            self._write_annotation(common_path, ann)

            print(f'  [set-importance] {common_path} → {importance}')
            self._json_response({'ok': True})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: update comments in annotation file ────────────────────────────────

    def _handle_update_comments(self):
        try:
            data = self._read_json()
            common_path = data.get('common_path', '').strip()
            layer = data.get('layer', '').strip()
            comment = data.get('comment', {})

            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return
            if layer not in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
                self._json_response({'error': f'Invalid layer: {layer}'}, 400)
                return

            cid = str(comment.get('id') or '').strip()
            text = str(comment.get('text') or '').strip()
            ts = str(data.get('ts') or '').strip()

            ann = self._read_annotation(common_path)
            layer_data = ann.setdefault(layer, {})
            comments = layer_data.setdefault('comments', [])

            if cid:
                # Update or delete existing comment
                idx = next((i for i, c in enumerate(comments) if c.get('id') == cid), None)
                if idx is None:
                    self._json_response({'error': 'Comment not found'}, 404)
                    return
                if text:
                    comments[idx]['text'] = text
                    if ts:
                        comments[idx]['ts'] = ts
                else:
                    comments.pop(idx)  # empty text = delete
                    cid = ''
            else:
                # New comment
                if not text:
                    self._json_response({'error': 'text required for new comment'}, 400)
                    return
                cid = uuid.uuid4().hex[:12]
                comments.append({'id': cid, 'text': text, 'ts': ts})

            # Clean up empty structures
            if not layer_data.get('comments'):
                layer_data.pop('comments', None)
            self._write_annotation(common_path, ann)

            print(f'  [update-comments] {common_path} [{layer}] id={cid or "deleted"}')
            self._json_response({'ok': True, 'id': cid})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: reorder comments in annotation file ───────────────────────────────

    def _handle_reorder_comments(self):
        try:
            data = self._read_json()
            common_path = data.get('common_path', '').strip()
            layer = data.get('layer', '').strip()
            ids = data.get('ids', [])

            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return
            if layer not in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
                self._json_response({'error': f'Invalid layer: {layer}'}, 400)
                return

            ann = self._read_annotation(common_path)
            layer_data = ann.get(layer, {})
            comments = layer_data.get('comments', [])
            id_map = {c['id']: c for c in comments}
            reordered = [id_map[i] for i in ids if i in id_map]
            layer_data['comments'] = reordered
            ann[layer] = layer_data
            self._write_annotation(common_path, ann)

            print(f'  [reorder-comments] {common_path} [{layer}]')
            self._json_response({'ok': True})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: update highlights in annotation file ──────────────────────────────

    def _handle_update_highlights(self):
        try:
            data = self._read_json()
            common_path = data.get('common_path', '').strip()
            layer = data.get('layer', '').strip()
            highlight = data.get('highlight', {})

            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return
            if layer not in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
                self._json_response({'error': f'Invalid layer: {layer}'}, 400)
                return

            hid        = str(highlight.get('id') or '').strip()
            text       = str(highlight.get('text') or '').strip()
            occurrence = highlight.get('occurrence')
            ts         = str(data.get('ts') or '').strip()

            ann = self._read_annotation(common_path)
            layer_data = ann.setdefault(layer, {})
            highlights = layer_data.setdefault('highlights', [])

            if hid:
                # Delete existing highlight
                layer_data['highlights'] = [h for h in highlights if h.get('id') != hid]
                if not layer_data['highlights']:
                    del layer_data['highlights']
                hid = ''
            else:
                # New highlight
                if not text:
                    self._json_response({'error': 'text required for new highlight'}, 400)
                    return
                hid = uuid.uuid4().hex[:12]
                entry = {'id': hid, 'text': text, 'ts': ts}
                if occurrence is not None:
                    try:
                        entry['occurrence'] = int(occurrence)
                    except (TypeError, ValueError):
                        pass
                highlights.append(entry)

            # Clean up empty structures
            if not layer_data.get('highlights') and not layer_data.get('comments'):
                ann.pop(layer, None)
            self._write_annotation(common_path, ann)

            print(f'  [update-highlights] {common_path} [{layer}] id={hid or "deleted"}')
            self._json_response({'ok': True, 'id': hid})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)


    # ── API: gh move (copy file/dir to new repo/dir, delete source) ────────

    def _gh_move_single_file(self, src_owner, src_repo, src_ref, src_path,
                              dst_owner, dst_repo, dst_path):
        """Move one file via GitHub Contents API. Returns dict with ok/error/warn."""
        get_r = subprocess.run(
            ['gh', 'api', f'repos/{src_owner}/{src_repo}/contents/{src_path}?ref={src_ref}'],
            capture_output=True, text=True, timeout=20
        )
        if get_r.returncode != 0:
            return {'error': f'获取源文件失败：{get_r.stderr.strip() or get_r.stdout.strip()}'}
        src_info = json.loads(get_r.stdout)
        file_content_b64 = src_info.get('content', '').replace('\n', '')
        src_sha = src_info.get('sha', '')
        if 'content' not in src_info or not src_sha:
            return {'error': '源文件内容无法读取'}

        put_payload = json.dumps({
            'message': f'move: {src_repo}/{src_path} → {dst_repo}/{dst_path}',
            'content': file_content_b64
        })
        put_r = subprocess.run(
            ['gh', 'api', f'repos/{dst_owner}/{dst_repo}/contents/{dst_path}',
             '-X', 'PUT', '--input', '-'],
            input=put_payload, capture_output=True, text=True, timeout=20
        )
        if put_r.returncode != 0:
            err_msg = put_r.stderr.strip() or put_r.stdout.strip()
            return {'error': f'创建目标文件失败：{err_msg}'}

        del_payload = json.dumps({
            'message': f'move: remove {src_repo}/{src_path} (moved to {dst_repo}/{dst_path})',
            'sha': src_sha
        })
        del_r = subprocess.run(
            ['gh', 'api', f'repos/{src_owner}/{src_repo}/contents/{src_path}',
             '-X', 'DELETE', '--input', '-'],
            input=del_payload, capture_output=True, text=True, timeout=20
        )
        if del_r.returncode != 0:
            err_msg = del_r.stderr.strip() or del_r.stdout.strip()
            return {
                'ok': True,
                'warn': f'目标文件已创建，但删除源文件失败：{err_msg}',
                'dst_path': dst_path
            }
        return {'ok': True, 'dst_path': dst_path}

    def _handle_gh_move(self):
        try:
            data = self._read_json()
            src_url = (data.get('src_url') or '').strip()
            dst_dir_url = (data.get('dst_dir_url') or '').strip()

            # Parse destination directory URL (flexible)
            # Accepts: tree/ref/dir, plain /owner/repo, /owner/repo/dir, /owner/repo/tree/ref/dir
            dst_m = re.match(r'https://github\.com/([^/]+)/([^/]+)(/.*)?$', dst_dir_url)
            if not dst_m:
                self._json_response({'error': '目标目录 URL 格式无效'}, 400)
                return
            dst_owner, dst_repo = dst_m.group(1), dst_m.group(2)
            raw_tail = (dst_m.group(3) or '').strip('/')
            # Strip tree/{ref}/ prefix if present
            raw_tail = re.sub(r'^tree/[^/]+/?', '', raw_tail)
            dst_dir = raw_tail.strip('/')

            # Detect source type: blob (single file) or tree (directory)
            blob_m = re.match(
                r'https://github\.com/([^/]+)/([^/]+)/blob/([^/]+)/(.+)',
                src_url
            )
            tree_m = re.match(
                r'https://github\.com/([^/]+)/([^/]+)/tree/([^/]+)/(.+)',
                src_url
            )

            if blob_m:
                # ── Single file move ──────────────────────────────────────
                src_owner, src_repo, src_ref, src_path = (
                    blob_m.group(1), blob_m.group(2), blob_m.group(3), blob_m.group(4)
                )
                filename = src_path.split('/')[-1]
                dst_path = f'{dst_dir}/{filename}' if dst_dir else filename
                result = self._gh_move_single_file(
                    src_owner, src_repo, src_ref, src_path,
                    dst_owner, dst_repo, dst_path
                )
                if result.get('ok'):
                    print(f'  [gh-move] {src_repo}/{src_path} → {dst_repo}/{dst_path}')
                self._json_response(result)

            elif tree_m:
                # ── Directory move ────────────────────────────────────────
                src_owner, src_repo, src_ref, src_dir_path = (
                    tree_m.group(1), tree_m.group(2), tree_m.group(3),
                    tree_m.group(4).rstrip('/')
                )
                dir_name = src_dir_path.split('/')[-1]
                dst_base = f'{dst_dir}/{dir_name}' if dst_dir else dir_name

                # Fetch all blobs recursively (one API call)
                tree_r = subprocess.run(
                    ['gh', 'api',
                     f'repos/{src_owner}/{src_repo}/git/trees/{src_ref}?recursive=1'],
                    capture_output=True, text=True, timeout=30
                )
                if tree_r.returncode != 0:
                    self._json_response({
                        'error': f'获取目录内容失败：{tree_r.stderr.strip() or tree_r.stdout.strip()}'
                    }, 500)
                    return
                tree_data = json.loads(tree_r.stdout)
                prefix = src_dir_path + '/'
                files = [
                    item['path'] for item in tree_data.get('tree', [])
                    if item.get('type') == 'blob' and item['path'].startswith(prefix)
                    and item['path'].split('/')[-1] != '.gitkeep'
                ]
                if not files:
                    self._json_response(
                        {'error': f'目录为空或不存在：{src_dir_path}'}, 400
                    )
                    return

                moved, failed = [], []
                for file_path in files:
                    rel = file_path[len(prefix):]
                    dst_path = f'{dst_base}/{rel}'
                    result = self._gh_move_single_file(
                        src_owner, src_repo, src_ref, file_path,
                        dst_owner, dst_repo, dst_path
                    )
                    if result.get('ok'):
                        print(f'  [gh-move] {src_repo}/{file_path} → {dst_repo}/{dst_path}')
                        moved.append(dst_path)
                    else:
                        failed.append({'src': file_path, 'error': result.get('error', 'unknown')})

                if not moved and failed:
                    self._json_response(
                        {'error': f'所有文件移动失败，第一个错误：{failed[0]["error"]}'}, 500
                    )
                    return

                self._json_response({
                    'ok': True,
                    'dst_path': dst_base,
                    'moved': len(moved),
                    'failed': failed,
                    'warn': (f'{len(failed)} 个文件移动失败' if failed else None)
                })

            else:
                self._json_response({
                    'error': '源 URL 格式无效，需为 GitHub blob（文件）或 tree（目录）链接'
                }, 400)

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except subprocess.TimeoutExpired:
            self._json_response({'error': 'gh 命令超时'}, 500)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: move entry to a different project ──────────────────────────────

    def _handle_move_project(self):
        try:
            data = self._read_json()
            entry_id = str(data.get('id') or '').strip()
            new_project = str(data.get('new_project') or '').strip()

            # 基本校验
            if not re.fullmatch(r'[0-9a-f]{32}', entry_id):
                self._json_response({'error': 'Invalid id'}, 400)
                return
            if not new_project or '..' in new_project or '/' in new_project:
                self._json_response({'error': 'Invalid new_project'}, 400)
                return

            # 校验 new_project 在 topics.json 中
            topics_data = load_topics(REPO_ROOT)
            valid_projects = set()
            for t in topics_data.get('topics', []):
                if 'dir' in t:
                    valid_projects.add(t['dir'])
                elif 'repo' in t:
                    valid_projects.add(t['repo'].split('/')[-1])
            if new_project not in valid_projects:
                self._json_response({'error': f'Unknown project: {new_project}'}, 400)
                return

            # 读取 index.json
            index_path = KNOWLEDGE_CORPUS_DIR / 'index.json'
            index_data = json.loads(index_path.read_text(encoding='utf-8'))
            entries = index_data.get('entries', index_data)

            if entry_id not in entries:
                self._json_response({'error': 'Entry not found'}, 404)
                return

            entry = entries[entry_id]
            old_cp = entry.get('common_path', '')
            if not old_cp or '..' in old_cp:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return

            # 计算新 common_path（仅替换第一个路径段）
            old_parts = old_cp.split('/')
            old_parts[0] = new_project
            new_cp = '/'.join(old_parts)

            if old_cp == new_cp:
                self._json_response({'error': 'Already in this project'}, 400)
                return

            # 收集所有 src → dst 路径对
            moves = []
            for layer in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
                src = (KNOWLEDGE_CORPUS_DIR / layer / old_cp).resolve()
                dst = (KNOWLEDGE_CORPUS_DIR / layer / new_cp).resolve()
                if not str(src).startswith(str(KNOWLEDGE_CORPUS_DIR) + os.sep):
                    continue
                if src.exists():
                    moves.append((src, dst))

            # annotation 文件
            ann_src = self._annotation_path(old_cp)
            if ann_src and ann_src.exists():
                new_ann_rel = (new_cp[:-3] + '.json') if new_cp.endswith('.md') else (new_cp + '.json')
                ann_dst = (KNOWLEDGE_CORPUS_DIR / 'annotations' / new_ann_rel).resolve()
                moves.append((ann_src, ann_dst))

            # zh 翻译文件
            old_zh = entry.get('translations', {}).get('zh')
            new_zh = None
            if old_zh:
                zh_parts = old_zh.split('/')
                new_cp_parts = new_cp.split('/')
                zh_parts[0] = new_project
                # 同步 doc-theme（zh 目录名应始终与 common_path 保持一致）
                if len(zh_parts) >= 2 and len(new_cp_parts) >= 2:
                    zh_parts[1] = new_cp_parts[1]
                new_zh = '/'.join(zh_parts)
                # 源文件：先尝试 old_project + new_doc_theme（rename_themes 已重命名目录后的位置）
                old_cp_parts = old_cp.split('/')
                corrected_old_zh_parts = list(zh_parts)
                corrected_old_zh_parts[0] = old_cp_parts[0]
                zh_src = (KNOWLEDGE_CORPUS_DIR / 'raw' / '/'.join(corrected_old_zh_parts)).resolve()
                if not zh_src.exists():
                    zh_src = (KNOWLEDGE_CORPUS_DIR / 'raw' / old_zh).resolve()
                zh_dst = (KNOWLEDGE_CORPUS_DIR / 'raw' / new_zh).resolve()
                if zh_src.exists():
                    moves.append((zh_src, zh_dst))

            # 原子性 mv：逐一执行，失败则 rollback
            completed = []
            try:
                for src, dst in moves:
                    dst.parent.mkdir(parents=True, exist_ok=True)
                    src.rename(dst)
                    completed.append((src, dst))
                    print(f'  [move-project] {src.relative_to(KNOWLEDGE_CORPUS_DIR)} → {dst.relative_to(KNOWLEDGE_CORPUS_DIR)}')
            except Exception as mv_err:
                for src_r, dst_r in reversed(completed):
                    try:
                        dst_r.rename(src_r)
                    except Exception:
                        pass
                self._json_response({'error': f'Move failed, rolled back: {mv_err}'}, 500)
                return

            # 更新文件内导航链接
            for src, dst in completed:
                if dst.suffix != '.md':
                    continue
                try:
                    content = dst.read_text(encoding='utf-8')
                    for layer in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
                        content = content.replace(f'{layer}/{old_cp}', f'{layer}/{new_cp}')
                        content = content.replace(f'`{layer}/{old_cp}`', f'`{layer}/{new_cp}`')
                    if old_zh and new_zh:
                        content = content.replace(f'raw/{old_zh}', f'raw/{new_zh}')
                    dst.write_text(content, encoding='utf-8')
                except Exception:
                    pass

            # 写 index.json（最后执行）
            entry['common_path'] = new_cp
            if new_zh:
                entry.setdefault('translations', {})['zh'] = new_zh
            tmp_path = index_path.with_suffix('.json.tmp')
            tmp_path.write_text(
                json.dumps(index_data, ensure_ascii=False, indent=2),
                encoding='utf-8'
            )
            tmp_path.replace(index_path)

            print(f'  [move-project] entry {entry_id}: {old_cp} → {new_cp}')
            self._json_response({'ok': True, 'new_common_path': new_cp})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: delete entry + all associated files ──────────────────────────────

    def _handle_delete(self):
        try:
            data = self._read_json()
            entry_id = str(data.get('id') or '').strip()

            if not re.fullmatch(r'[0-9a-f]{32}', entry_id):
                self._json_response({'error': 'Invalid id'}, 400)
                return

            index_path = KNOWLEDGE_CORPUS_DIR / 'index.json'
            index_data = json.loads(index_path.read_text(encoding='utf-8'))
            entries = index_data.get('entries', index_data)

            if entry_id not in entries:
                self._json_response({'error': 'Entry not found'}, 404)
                return

            entry = entries[entry_id]
            common_path = entry.get('common_path', '')
            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return

            # Delete files in all layers that exist
            deleted = []
            for layer in ('raw', 'distilled', 'trace', 'digest', 'diagnose'):
                target = (KNOWLEDGE_CORPUS_DIR / layer / common_path).resolve()
                if not str(target).startswith(str(KNOWLEDGE_CORPUS_DIR) + os.sep):
                    continue
                if target.exists():
                    target.unlink()
                    deleted.append(f'{layer}/{common_path}')
                    print(f'  [delete] {layer}/{common_path}')

            # Delete annotation file if exists
            ann_path = self._annotation_path(common_path)
            if ann_path and ann_path.exists():
                ann_path.unlink()
                deleted.append(f'annotations/{common_path[:-3]}.json')
                print(f'  [delete] annotations/{common_path}')

            # Remove from index.json
            del entries[entry_id]
            tmp_path = index_path.with_suffix('.json.tmp')
            tmp_path.write_text(
                json.dumps(index_data, ensure_ascii=False, indent=2),
                encoding='utf-8'
            )
            tmp_path.replace(index_path)

            print(f'  [delete] index entry {entry_id} removed')
            self._json_response({'ok': True, 'deleted': deleted})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── Internal: write/read file via gh api ──────────────────────────────────

    def _gh_put_file(self, owner, repo, path, content_str, commit_message, sha=None):
        """Create or update a file in a GitHub repo. Returns (response_data, error_str)."""
        content_b64 = base64.b64encode(content_str.encode('utf-8')).decode('ascii')
        payload = {'message': commit_message, 'content': content_b64}
        if sha:
            payload['sha'] = sha
        r = subprocess.run(
            ['gh', 'api', f'repos/{owner}/{repo}/contents/{path}',
             '-X', 'PUT', '--input', '-'],
            input=json.dumps(payload), capture_output=True, text=True, timeout=30
        )
        if r.returncode != 0:
            return None, r.stderr.strip() or r.stdout.strip()
        return json.loads(r.stdout), None

    def _gh_get_file(self, owner, repo, path, ref='main'):
        """Read a file from a GitHub repo. Returns ({content, sha}, error_str)."""
        r = subprocess.run(
            ['gh', 'api', f'repos/{owner}/{repo}/contents/{path}?ref={ref}'],
            capture_output=True, text=True, timeout=20
        )
        if r.returncode != 0:
            return None, r.stderr.strip() or r.stdout.strip()
        data = json.loads(r.stdout)
        content_b64 = data.get('content', '').replace('\n', '')
        content = base64.b64decode(content_b64).decode('utf-8', errors='replace')
        return {'content': content, 'sha': data.get('sha', '')}, None

    # ── API: GET /api/check-file ──────────────────────────────────────────────

    def _handle_check_file(self):
        try:
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            repo = params.get('repo', [''])[0].strip()
            path = params.get('path', [''])[0].strip()
            if not repo or '/' not in repo or not path:
                self._json_response({'error': 'repo and path required'}, 400)
                return

            # Security: only allow repos listed in topics.json
            topics_data = load_topics(REPO_ROOT)
            valid_repos = {t['repo'] for t in topics_data.get('topics', []) if 'repo' in t}
            if repo not in valid_repos:
                self._json_response({'error': f'Unknown repo: {repo}'}, 400)
                return

            owner, repo_name = repo.split('/', 1)
            result = subprocess.run(
                ['gh', 'api', f'repos/{owner}/{repo_name}/contents/{path}'],
                capture_output=True, text=True
            )
            self._json_response({'exists': result.returncode == 0})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/repo-list ───────────────────────────────────────────────

    def _handle_repo_list(self):
        def _fetch_type_meta(repo):
            """Fetch .repository-type.json for one repo. Returns (repo, type_str, description)."""
            full_name = repo.get('full_name', '')
            default_branch = repo.get('default_branch', 'main') or 'main'
            path = f"repos/{full_name}/contents/.repository-type.json?ref={urllib.parse.quote(default_branch)}"
            r = subprocess.run(
                ['gh', 'api', '-H', 'Accept: application/vnd.github+json', path],
                capture_output=True, text=True, check=False, timeout=15
            )
            if r.returncode != 0:
                return (full_name, None, None)
            try:
                meta = json.loads(r.stdout)
                raw_bytes = base64.b64decode((meta.get('content') or '').replace('\n', ''))
                doc = json.loads(raw_bytes.decode('utf-8'))
                return (
                    full_name,
                    doc.get('type'),
                    doc.get('description'),
                )
            except Exception:
                return (full_name, None, None)

        try:
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            force = params.get('force', [''])[0] == '1'

            # Return cached data if available and not forced
            if not force and REPO_LIST_CACHE_FILE.exists():
                try:
                    cached = json.loads(REPO_LIST_CACHE_FILE.read_text(encoding='utf-8'))
                    self._json_response(cached)
                    return
                except Exception:
                    pass  # cache corrupted, fall through to re-fetch

            r = subprocess.run(
                ['gh', 'api', '-H', 'Accept: application/vnd.github+json',
                 'user/repos?per_page=100&affiliation=owner', '--paginate'],
                capture_output=True, text=True, check=False, timeout=60
            )
            if r.returncode != 0:
                self._json_response({'error': r.stderr.strip() or r.stdout.strip()}, 500)
                return

            # gh --paginate returns concatenated JSON arrays; parse them all
            raw = r.stdout.strip()
            all_repos = []
            decoder = json.JSONDecoder()
            idx = 0
            while idx < len(raw):
                while idx < len(raw) and raw[idx] in ' \t\n\r':
                    idx += 1
                if idx >= len(raw):
                    break
                obj, end = decoder.raw_decode(raw, idx)
                all_repos.extend(obj)
                idx = end

            # Concurrently fetch .repository-type.json for each repo
            type_map = {}   # full_name -> (type, description)
            with ThreadPoolExecutor(max_workers=10) as executor:
                futures = {executor.submit(_fetch_type_meta, repo): repo for repo in all_repos}
                for future in as_completed(futures):
                    try:
                        full_name, repo_type, description = future.result()
                        type_map[full_name] = (repo_type, description)
                    except Exception:
                        pass

            repos_out = []
            for repo in all_repos:
                full_name = repo.get('full_name', '')
                repo_type, description = type_map.get(full_name, (None, None))
                repos_out.append({
                    'name': repo.get('name', ''),
                    'full_name': full_name,
                    'type': repo_type,
                    'description': description,
                })

            cached_at = datetime.now(timezone(timedelta(hours=8))).strftime('%m月%d日 %H:%M')
            payload = {'repos': repos_out, 'cached_at': cached_at}

            # Write to .cache/repo-list.json
            CACHE_DIR.mkdir(exist_ok=True)
            REPO_LIST_CACHE_FILE.write_text(
                json.dumps(payload, ensure_ascii=False, indent=2), encoding='utf-8'
            )

            self._json_response(payload)
        except subprocess.TimeoutExpired:
            self._json_response({'error': 'repo-list timed out'}, 500)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/repo-dirs ───────────────────────────────────────────────

    def _handle_repo_dirs(self):
        try:
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            repo = params.get('repo', [''])[0].strip()
            if not repo or '/' not in repo:
                self._json_response({'error': 'missing or invalid repo'}, 400)
                return

            # Security: only allow repos listed in topics.json
            topics_data = load_topics(REPO_ROOT)
            valid_repos = {t['repo'] for t in topics_data.get('topics', []) if 'repo' in t}
            if repo not in valid_repos:
                self._json_response({'error': f'Unknown repo: {repo}'}, 400)
                return

            owner, repo_name = repo.split('/', 1)
            r = subprocess.run(
                ['gh', 'api', f'repos/{owner}/{repo_name}/contents/'],
                capture_output=True, text=True, timeout=20
            )
            if r.returncode != 0:
                self._json_response({'error': r.stderr.strip() or r.stdout.strip()}, 500)
                return

            items = json.loads(r.stdout)
            dirs = sorted(
                item['name'] for item in items
                if item.get('type') == 'dir'
                and not item['name'].startswith('.')
                and not item['name'].startswith('_')
            )
            self._json_response({'dirs': dirs})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/settle ─────────────────────────────────────────────────

    def _handle_settle(self):
        try:
            data = self._read_json()
            common_path = (data.get('common_path') or '').strip()
            comment_id  = (data.get('comment_id') or '').strip()
            layer       = (data.get('layer') or 'raw').strip()
            doc_theme   = (data.get('doc_theme') or '').strip()
            slug        = (data.get('slug') or '').strip()
            content     = (data.get('content') or '').strip()

            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid common_path'}, 400)
                return
            if not comment_id:
                self._json_response({'error': 'comment_id required'}, 400)
                return
            if layer not in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
                self._json_response({'error': f'Invalid layer: {layer}'}, 400)
                return
            if not doc_theme or (doc_theme != '.' and ('..' in doc_theme or '/' in doc_theme)):
                self._json_response({'error': 'Invalid doc_theme'}, 400)
                return
            if not slug or not re.fullmatch(r'[a-z0-9][a-z0-9\-]*', slug):
                self._json_response({'error': 'slug must be lowercase letters/digits/hyphens, not starting with hyphen'}, 400)
                return
            if not content:
                self._json_response({'error': 'content required'}, 400)
                return

            # Derive target_repo from common_path's first segment
            project_dir = common_path.split('/')[0]
            topics_data = load_topics(REPO_ROOT)
            target_repo = None
            for t in topics_data.get('topics', []):
                if 'repo' in t:
                    repo_name_t = t['repo'].split('/')[-1]
                    if repo_name_t == project_dir or t.get('dir') == project_dir:
                        target_repo = t['repo']
                        break
            if not target_repo:
                self._json_response({'error': f'No GitHub repo found for project: {project_dir}'}, 400)
                return

            owner, repo_name = target_repo.split('/', 1)

            # Build timestamp (UTC+8)
            tz8  = timezone(timedelta(hours=8))
            now8 = datetime.now(tz8)
            ts   = now8.strftime('%Y%m%d%H%M')
            date_str = f'{now8.year}年{now8.month}月{now8.day}日'
            filename = f'{ts}-{slug}.md'
            dst_path = filename if doc_theme == '.' else f'{doc_theme}/{filename}'
            dst_url  = f'https://github.com/{owner}/{repo_name}/blob/main/{dst_path}'
            src_url  = f'{KNOWLEDGE_CORPUS_GITHUB}/raw/{common_path}'
            full_content = f'> 来源：[Entry]({src_url})\n> 沉淀时间：{date_str}\n\n{content}'

            # Step 1: Push file to target repo (critical — abort on failure)
            _, err = self._gh_put_file(
                owner, repo_name, dst_path, full_content,
                f'settle: add {doc_theme}/{filename}'
            )
            if err:
                self._json_response({'error': f'写入目标仓库失败：{err}'}, 500)
                return

            warns = []

            # Step 1b: Upsert doc into Meilisearch (best-effort, non-blocking)
            try:
                topic_desc = ''
                for t in topics_data.get('topics', []):
                    if t.get('repo') == target_repo:
                        topic_desc = t.get('description', '')
                        break
                _meili_upsert_doc(target_repo, dst_path, full_content, topic_desc)
            except Exception as e1b:
                warns.append(f'Meilisearch upsert 失败：{e1b}')

            # Step 2: Append URL to annotation.links
            try:
                ann = self._read_annotation(common_path)
                links = ann.get('links', [])
                links.append({'url': dst_url})
                ann['links'] = links
                self._write_annotation(common_path, ann)
            except Exception as e2:
                warns.append(f'更新 annotation.links 失败：{e2}')

            # Step 3: Append row to target repo _index.md (create if missing)
            try:
                new_row = f'| {doc_theme} | （待补充） | {dst_url} |'
                file_info, _ = self._gh_get_file(owner, repo_name, '_index.md')
                if file_info:
                    idx_content = file_info['content']
                    if not idx_content.endswith('\n'):
                        idx_content += '\n'
                    idx_content += new_row + '\n'
                    _, put_err = self._gh_put_file(
                        owner, repo_name, '_index.md', idx_content,
                        f'settle: update _index.md for {doc_theme}',
                        sha=file_info['sha']
                    )
                    if put_err:
                        warns.append(f'更新 _index.md 失败：{put_err}')
                else:
                    idx_content = (
                        f'# {repo_name} 知识索引\n\n'
                        '| 主题 | 一句话描述 | GitHub URL |\n'
                        '|------|-----------|------------|\n'
                        f'{new_row}\n'
                    )
                    _, put_err = self._gh_put_file(
                        owner, repo_name, '_index.md', idx_content,
                        'settle: create _index.md'
                    )
                    if put_err:
                        warns.append(f'创建 _index.md 失败：{put_err}')
            except Exception as e3:
                warns.append(f'更新 _index.md 失败：{e3}')

            # Step 4: Delete comment (after file confirmed pushed)
            try:
                ann = self._read_annotation(common_path)
                ld  = ann.get(layer, {})
                ld['comments'] = [c for c in ld.get('comments', []) if c.get('id') != comment_id]
                if not ld.get('comments'):
                    ld.pop('comments', None)
                if ld:
                    ann[layer] = ld
                elif layer in ann:
                    del ann[layer]
                self._write_annotation(common_path, ann)
            except Exception as e4:
                warns.append(f'删除 comment 失败：{e4}')

            print(f'  [settle] {common_path} → {owner}/{repo_name}/{dst_path}')
            resp = {'ok': True, 'url': dst_url}
            if warns:
                resp['warn'] = warns
            self._json_response(resp)

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except subprocess.TimeoutExpired:
            self._json_response({'error': 'gh 命令超时'}, 500)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/search-knowledge ────────────────────────────────────────

    def _handle_search_knowledge(self):
        try:
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            q = params.get('q', [''])[0].strip()
            if not q:
                self._json_response({'error': 'q parameter required'}, 400)
                return
            try:
                limit = min(int(params.get('limit', ['10'])[0]), 50)
            except ValueError:
                limit = 10

            result = _meili_request('POST', '/indexes/knowledge/search', {
                'q': q,
                'limit': limit,
                'attributesToCrop': ['body'],
                'cropLength': 80,
                'attributesToHighlight': ['body'],
            })
            if result is None:
                self._json_response({'hits': [], 'error': 'unavailable'})
                return
            if result.get('code') == 'index_not_found':
                self._json_response({'hits': [], 'error': 'not_indexed'})
                return

            hits = result.get('hits', [])
            self._json_response({'hits': hits})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/reindex-knowledge ─────────────────────────────────────

    def _handle_reindex_knowledge(self):
        try:
            global _reindex_job
            if _reindex_job['status'] == 'running':
                self._json_response({'error': 'already running'}, 409)
                return

            now_iso = datetime.now(timezone.utc).isoformat()
            _reindex_job = {
                'status': 'running',
                'started_at': now_iso,
                'finished_at': None,
                'log': '启动中…',
            }

            def _sync_repo(repo):
                """Clone missing repo or pull existing one. Returns (repo_name, status, err)."""
                repo_name = repo.split('/')[-1]
                local_dir = KNOWLEDGE_BASE_DIR / repo_name
                if not local_dir.is_dir():
                    r = subprocess.run(
                        ['gh', 'repo', 'clone', repo, str(local_dir)],
                        capture_output=True, text=True, timeout=300
                    )
                    if r.returncode != 0:
                        shutil.rmtree(str(local_dir), ignore_errors=True)
                        return (repo_name, 'clone_failed', r.stderr.strip() or r.stdout.strip())
                    return (repo_name, 'cloned', None)
                else:
                    r = subprocess.run(
                        ['git', '-C', str(local_dir), 'pull', '--rebase'],
                        capture_output=True, text=True, timeout=60
                    )
                    if r.returncode != 0:
                        return (repo_name, 'pull_failed', r.stderr.strip() or r.stdout.strip())
                    return (repo_name, 'pulled', None)

            def _do_reindex():
                global _reindex_job
                try:
                    topics_data = load_topics(REPO_ROOT)
                    repos = [t['repo'] for t in topics_data.get('topics', []) if 'repo' in t]

                    # Phase D: parallel sync
                    _reindex_job['log'] = '同步仓库（并行）…'
                    failed_repos = []
                    with ThreadPoolExecutor(max_workers=5) as executor:
                        futures = {executor.submit(_sync_repo, repo): repo for repo in repos}
                        for future in as_completed(futures):
                            name, status, err = future.result()
                            if 'failed' in status:
                                failed_repos.append(f'{name}: {err}')

                    _reindex_job['log'] = '重建索引…'
                    script = REPO_ROOT / 'scripts' / 'build_knowledge_index.py'
                    r = subprocess.run(
                        ['python3', str(script)],
                        cwd=str(REPO_ROOT), capture_output=True, text=True, timeout=600
                    )
                    if r.returncode != 0:
                        _reindex_job['status'] = 'error'
                        _reindex_job['log'] = r.stderr.strip() or r.stdout.strip() or '未知错误'
                    else:
                        log = r.stdout.strip() or '完成'
                        if failed_repos:
                            log += '\n⚠ 同步失败：' + '；'.join(failed_repos)
                        _reindex_job['status'] = 'done'
                        _reindex_job['log'] = log
                except Exception as e:
                    _reindex_job['status'] = 'error'
                    _reindex_job['log'] = str(e)
                finally:
                    _reindex_job['finished_at'] = datetime.now(timezone.utc).isoformat()

            threading.Thread(target=_do_reindex, daemon=True).start()
            self._json_response({'status': 'running'})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/reindex-status ──────────────────────────────────────────

    def _handle_reindex_status(self):
        self._json_response(_reindex_job)

    # ── API: KB helpers ───────────────────────────────────────────────────────

    def _kb_safe_path(self, repo, rel_path):
        """Derive and validate local KB file path.
        Returns (resolved_Path, None) on success, or (None, error_str) on failure."""
        if not repo or '/' not in repo:
            return None, 'invalid repo format'
        repo_name = repo.split('/')[-1]
        if not rel_path or '..' in rel_path:
            return None, 'invalid path'
        kb_root = KNOWLEDGE_BASE_DIR.resolve()
        local_dir = KNOWLEDGE_BASE_DIR / repo_name
        if not local_dir.is_dir():
            return None, f'repo not cloned locally: {repo_name}'
        target = (local_dir / rel_path).resolve()
        if not str(target).startswith(str(kb_root) + os.sep) and str(target) != str(kb_root):
            return None, 'path traversal not allowed'
        return target, None

    # ── API: KB annotation helpers ───────────────────────────────────────────

    def _kb_annotation_path(self, repo, rel_path):
        """Derive annotation file path for a KB document.
        Maps {repo}/{rel_path}.md → KNOWLEDGE_BASE_DIR/{repo_name}/.knowledge_annotations/{rel_path}.json
        Returns (Path, None) on success or (None, error_str)."""
        target, err = self._kb_safe_path(repo, rel_path)
        if err:
            return None, err
        # Strip leading repo_name segment from rel_path (rel_path is relative to repo root)
        repo_name = repo.split('/')[-1]
        ann_rel = rel_path
        # Replace .md extension with .json; keep other extensions as-is with .json appended
        if ann_rel.endswith('.md'):
            ann_rel = ann_rel[:-3] + '.json'
        else:
            ann_rel = ann_rel + '.json'
        ann_path = KNOWLEDGE_BASE_DIR / repo_name / '.knowledge_annotations' / ann_rel
        # Path traversal check
        kb_root = KNOWLEDGE_BASE_DIR.resolve()
        if not str(ann_path.resolve()).startswith(str(kb_root) + os.sep):
            return None, 'path traversal not allowed'
        return ann_path, None

    def _kb_annotation_read(self, ann_path):
        """Read annotation JSON from ann_path; return {} if not found."""
        if not ann_path.exists():
            return {}
        try:
            return json.loads(ann_path.read_text(encoding='utf-8'))
        except Exception:
            return {}

    def _kb_annotation_write(self, ann_path, data):
        """Write annotation JSON; lazily create parent directories."""
        ann_path.parent.mkdir(parents=True, exist_ok=True)
        ann_path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding='utf-8')

    # ── API: GET /api/kb/annotation ───────────────────────────────────────────

    def _handle_kb_annotation_get(self):
        try:
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            repo = params.get('repo', [''])[0].strip()
            path = params.get('path', [''])[0].strip()
            ann_path, err = self._kb_annotation_path(repo, path)
            if err:
                status = 400 if ('invalid' in err or 'traversal' in err) else 404
                self._json_response({'error': err}, status)
                return
            self._json_response(self._kb_annotation_read(ann_path))
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/kb/update-comments ────────────────────────────────────

    def _handle_kb_update_comments(self):
        try:
            data = self._read_json()
            repo = data.get('repo', '').strip()
            path = data.get('path', '').strip()
            comment = data.get('comment', {})
            ts = data.get('ts', '')
            ann_path, err = self._kb_annotation_path(repo, path)
            if err:
                status = 400 if ('invalid' in err or 'traversal' in err) else 404
                self._json_response({'error': err}, status)
                return
            ann = self._kb_annotation_read(ann_path)
            comments = ann.get('comments', [])
            cid = comment.get('id', '').strip()
            text = comment.get('text', '')
            if not cid:
                # add
                cid = uuid.uuid4().hex[:12]
                comments.append({'id': cid, 'text': text, 'ts': ts})
                ann['comments'] = comments
                self._kb_annotation_write(ann_path, ann)
                print(f'  [kb/annotation] add comment {cid} in {repo}/{path}')
                self._json_response({'ok': True, 'id': cid})
            elif text:
                # edit
                for c in comments:
                    if c['id'] == cid:
                        c['text'] = text
                        c['ts'] = ts
                        break
                ann['comments'] = comments
                self._kb_annotation_write(ann_path, ann)
                print(f'  [kb/annotation] edit comment {cid} in {repo}/{path}')
                self._json_response({'ok': True})
            else:
                # delete
                ann['comments'] = [c for c in comments if c['id'] != cid]
                self._kb_annotation_write(ann_path, ann)
                print(f'  [kb/annotation] delete comment {cid} in {repo}/{path}')
                self._json_response({'ok': True})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/kb/reorder-comments ───────────────────────────────────

    def _handle_kb_reorder_comments(self):
        try:
            data = self._read_json()
            repo = data.get('repo', '').strip()
            path = data.get('path', '').strip()
            ids = data.get('ids', [])
            ann_path, err = self._kb_annotation_path(repo, path)
            if err:
                status = 400 if ('invalid' in err or 'traversal' in err) else 404
                self._json_response({'error': err}, status)
                return
            ann = self._kb_annotation_read(ann_path)
            comments = ann.get('comments', [])
            id_map = {c['id']: c for c in comments}
            reordered = [id_map[i] for i in ids if i in id_map]
            ann['comments'] = reordered
            self._kb_annotation_write(ann_path, ann)
            print(f'  [kb/annotation] reorder comments in {repo}/{path}')
            self._json_response({'ok': True})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/kb/update-highlights ──────────────────────────────────

    def _handle_kb_update_highlights(self):
        try:
            data = self._read_json()
            repo = data.get('repo', '').strip()
            path = data.get('path', '').strip()
            highlight = data.get('highlight', {})
            ts = data.get('ts', '')
            ann_path, err = self._kb_annotation_path(repo, path)
            if err:
                status = 400 if ('invalid' in err or 'traversal' in err) else 404
                self._json_response({'error': err}, status)
                return
            ann = self._kb_annotation_read(ann_path)
            highlights = ann.get('highlights', [])
            hid = highlight.get('id', '').strip()
            if not hid:
                # add
                hid = uuid.uuid4().hex[:12]
                highlights.append({
                    'id': hid,
                    'text': highlight.get('text', ''),
                    'occurrence': highlight.get('occurrence', 0),
                    'ts': ts,
                })
                ann['highlights'] = highlights
                self._kb_annotation_write(ann_path, ann)
                print(f'  [kb/annotation] add highlight {hid} in {repo}/{path}')
                self._json_response({'ok': True, 'id': hid})
            else:
                # delete
                ann['highlights'] = [h for h in highlights if h['id'] != hid]
                self._kb_annotation_write(ann_path, ann)
                print(f'  [kb/annotation] delete highlight {hid} in {repo}/{path}')
                self._json_response({'ok': True})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/kb/update-links ─────────────────────────────────────────

    def _handle_kb_update_links(self):
        try:
            data = self._read_json()
            repo = data.get('repo', '').strip()
            path = data.get('path', '').strip()
            links = data.get('links', [])
            ann_path, err = self._kb_annotation_path(repo, path)
            if err:
                status = 400 if ('invalid' in err or 'traversal' in err) else 404
                self._json_response({'error': err}, status)
                return
            for link in links:
                url = link.get('url', '')
                p = urllib.parse.urlparse(url)
                if p.scheme not in ('http', 'https') or not p.netloc:
                    self._json_response({'error': f'Invalid url: {url}'}, 400)
                    return
            ann = self._kb_annotation_read(ann_path)
            if links:
                ann['links'] = links
            else:
                ann.pop('links', None)
            self._kb_annotation_write(ann_path, ann)
            print(f'  [kb/annotation] update-links {repo}/{path} → {len(links)} link(s)')
            self._json_response({'ok': True})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/kb/read ─────────────────────────────────────────────────

    def _handle_kb_read(self):
        try:
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            repo = params.get('repo', [''])[0].strip()
            path = params.get('path', [''])[0].strip()
            target, err = self._kb_safe_path(repo, path)
            if err:
                status = 400 if ('invalid' in err or 'traversal' in err) else 404
                self._json_response({'error': err}, status)
                return
            if not target.exists():
                self._json_response({'error': f'file not found: {path}'}, 404)
                return
            content = target.read_text(encoding='utf-8')
            self._json_response({'content': content})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/kb/save ────────────────────────────────────────────────

    def _handle_kb_save(self):
        try:
            data = self._read_json()
            repo = data.get('repo', '').strip()
            path = data.get('path', '').strip()
            content = data.get('content', '')
            target, err = self._kb_safe_path(repo, path)
            if err:
                status = 400 if ('invalid' in err or 'traversal' in err) else 404
                self._json_response({'error': err}, status)
                return
            if not target.exists():
                self._json_response({'error': f'file not found: {path}'}, 404)
                return
            target.write_text(content, encoding='utf-8')
            print(f'  [kb/save] {repo}/{path}')
            self._json_response({'ok': True})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/kb/status ───────────────────────────────────────────────

    def _handle_kb_status(self):
        try:
            from urllib.parse import urlparse, parse_qs
            qs = parse_qs(urlparse(self.path).query)
            repo = (qs.get('repo') or [''])[0].strip()
            if not repo or '/' not in repo:
                self._json_response({'error': 'repo required'}, 400)
                return
            repo_name = repo.split('/')[-1]
            local_dir = KNOWLEDGE_BASE_DIR / repo_name
            if not local_dir.is_dir():
                self._json_response({'error': f'repo not cloned: {repo_name}'}, 404)
                return

            def run(cmd):
                return subprocess.run(cmd, cwd=str(local_dir), capture_output=True, text=True)

            result = run(['git', 'status', '--porcelain'])
            categories = {'new': [], 'modified': [], 'deleted': [], 'renamed': [], 'conflicted': []}
            for line in result.stdout.splitlines():
                if not line.strip():
                    continue
                xy = line[:2]
                fpath = line[3:]
                x, y = xy[0], xy[1]
                if xy in ('UU', 'AA', 'DD', 'AU', 'UA', 'DU', 'UD'):
                    categories['conflicted'].append(fpath.strip())
                elif x == 'R' or y == 'R':
                    if ' -> ' in fpath:
                        old, new = fpath.split(' -> ', 1)
                        categories['renamed'].append(f'{old.strip()} → {new.strip()}')
                    else:
                        categories['renamed'].append(fpath.strip())
                elif x == 'D' or y == 'D':
                    categories['deleted'].append(fpath.strip())
                elif x == 'A' or xy == '??':
                    categories['new'].append(fpath.strip())
                elif x == 'M' or y == 'M':
                    categories['modified'].append(fpath.strip())
                elif xy.strip():
                    categories['modified'].append(fpath.strip())

            # .knowledge_annotations is typically covered by .gitignore (e.g. /.*).
            # Only do a second targeted scan when the directory is actually ignored,
            # to avoid duplicating entries already captured by the first scan.
            ann_dir = local_dir / '.knowledge_annotations'
            if ann_dir.is_dir():
                is_ignored = run(['git', 'check-ignore', '-q', '.knowledge_annotations']).returncode == 0
                if is_ignored:
                    ann_result = run(['git', 'status', '--porcelain', '--ignored', '.knowledge_annotations/'])
                    for line in ann_result.stdout.splitlines():
                        if not line.strip():
                            continue
                        xy = line[:2]
                        fpath = line[3:].strip()
                        if xy == '!!':   # ignored but untracked
                            categories['new'].append(fpath)
                        elif xy == '??':
                            categories['new'].append(fpath)
                        elif xy[0] == 'M' or xy[1] == 'M':
                            categories['modified'].append(fpath)
                        elif xy[0] == 'D' or xy[1] == 'D':
                            categories['deleted'].append(fpath)
            total = sum(len(v) for v in categories.values())
            ahead_r = run(['git', 'rev-list', '--count', 'HEAD...@{u}'])
            ahead = int(ahead_r.stdout.strip()) if ahead_r.returncode == 0 and ahead_r.stdout.strip().isdigit() else 0
            self._json_response({**categories, 'total': total, 'ahead': ahead})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/kb/revert ─────────────────────────────────────────────

    def _handle_kb_revert(self):
        """
        Revert working tree changes in a KB repo.
        Body: { "repo": "lulufoo/xxx", "path": "rel/path", "type": "modified"|"new"|"deleted" }
        Omit "path" to revert everything.
        """
        try:
            data = self._read_json()
            repo = data.get('repo', '').strip()
            path = data.get('path', '').strip()   # optional
            file_type = data.get('type', '').strip()  # 'new' | 'modified' | 'deleted'

            if not repo or '/' not in repo:
                self._json_response({'error': 'invalid repo format'}, 400)
                return
            repo_name = repo.split('/')[-1]
            local_dir = KNOWLEDGE_BASE_DIR / repo_name
            if not local_dir.is_dir():
                self._json_response({'error': f'repo not cloned locally: {repo_name}'}, 404)
                return

            def run(cmd):
                return subprocess.run(
                    cmd, cwd=str(local_dir),
                    capture_output=True, text=True
                )

            if path:
                # Single-file revert
                is_annotation = path.startswith('.knowledge_annotations/')
                if file_type == 'new' or is_annotation:
                    # untracked / ignored — delete the file or directory directly
                    target = local_dir / path
                    if target.is_dir():
                        shutil.rmtree(target)
                    elif target.exists():
                        target.unlink()
                    else:
                        run(['git', 'clean', '-fx', '--', path])  # best-effort
                else:
                    # modified or deleted — restore from HEAD
                    r = run(['git', 'checkout', '--', path])
                    if r.returncode != 0:
                        self._json_response({
                            'error': f'revert failed: {r.stderr.strip() or r.stdout.strip()}'
                        }, 500)
                        return
            else:
                # Revert everything (tracked files)
                r1 = run(['git', 'checkout', '--', '.'])
                # Clean untracked files (including force-ignored like .knowledge_annotations)
                r2 = run(['git', 'clean', '-fdx', '.knowledge_annotations/'])
                r3 = run(['git', 'clean', '-fd'])
                if r1.returncode != 0:
                    self._json_response({
                        'error': f'revert all failed: {(r1.stderr).strip()}'
                    }, 500)
                    return

            self._json_response({'ok': True})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/kb/commit ──────────────────────────────────────────────

    def _handle_kb_commit(self):
        try:
            data = self._read_json()
            repo = data.get('repo', '').strip()
            message = (data.get('message') or '').strip() or 'chore: update via viewer'

            if not repo or '/' not in repo:
                self._json_response({'error': 'invalid repo format'}, 400)
                return
            repo_name = repo.split('/')[-1]
            local_dir = KNOWLEDGE_BASE_DIR / repo_name
            if not local_dir.is_dir():
                self._json_response({'error': f'repo not cloned locally: {repo_name}'}, 404)
                return

            def run(cmd):
                return subprocess.run(
                    cmd, cwd=str(local_dir),
                    capture_output=True, text=True
                )

            # 0. pre-check: abort if repo has unresolved merge conflicts
            unmerged = run(['git', 'ls-files', '--unmerged'])
            if unmerged.stdout.strip():
                self._json_response({
                    'error': '仓库存在未解决的合并冲突，请手动修复后重试',
                    'step': 'pre-check',
                    'stderr': unmerged.stdout.strip()
                }, 500)
                return

            # 1. git stash (exit 0 even when nothing to stash)
            stash = run(['git', 'stash'])
            if stash.returncode != 0:
                self._json_response({
                    'error': 'git stash failed',
                    'step': 'stash',
                    'stderr': stash.stderr,
                    'stdout': stash.stdout
                }, 500)
                return
            stash_was_empty = 'No local changes to save' in stash.stdout

            # 2. git pull
            pull = run(['git', 'pull', '--rebase'])
            if pull.returncode != 0:
                if not stash_was_empty:
                    run(['git', 'stash', 'pop'])
                self._json_response({
                    'error': 'git pull failed',
                    'step': 'pull',
                    'stderr': pull.stderr
                }, 500)
                return

            # 3. git stash pop (only if we stashed something)
            if not stash_was_empty:
                pop = run(['git', 'stash', 'pop'])
                if pop.returncode != 0:
                    self._json_response({
                        'error': 'git stash pop failed',
                        'step': 'stash_pop',
                        'stderr': pop.stderr
                    }, 500)
                    return

            # 4. git add (all tracked + untracked in .knowledge_annotations)
            add = run(['git', 'add', '-A'])
            if add.returncode != 0:
                self._json_response({
                    'error': 'git add failed',
                    'step': 'add',
                    'stderr': add.stderr
                }, 500)
                return
            # Force-add .knowledge_annotations even if covered by .gitignore
            ann_dir = local_dir / '.knowledge_annotations'
            if ann_dir.is_dir():
                run(['git', 'add', '--force', '.knowledge_annotations/'])

            # 5. git commit
            commit = run(['git', 'commit', '-m', message])
            nothing_to_commit = (
                'nothing to commit' in commit.stdout or
                'nothing to commit' in commit.stderr
            )
            if commit.returncode != 0 and not nothing_to_commit:
                self._json_response({
                    'error': 'git commit failed',
                    'step': 'commit',
                    'stderr': commit.stderr,
                    'stdout': commit.stdout
                }, 500)
                return

            # 6. git push
            push = run(['git', 'push'])
            if push.returncode != 0:
                self._json_response({
                    'error': 'git push failed',
                    'step': 'push',
                    'stderr': push.stderr
                }, 500)
                return

            info = commit.stdout.strip() if not nothing_to_commit else 'nothing to commit; pushed'
            print(f'  [kb/commit+push] {repo}: {message}')
            self._json_response({'ok': True, 'info': info})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/open-iterm ─────────────────────────────────────────────

    def _handle_open_iterm(self):
        try:
            data = self._read_json()
            repo = data.get('repo', '').strip()
            if not repo:
                self._json_response({'error': 'repo required'}, 400)
                return
            repo_name = repo.split('/')[-1]
            local_dir = KNOWLEDGE_BASE_DIR / repo_name
            if not local_dir.is_dir():
                self._json_response({'error': f'repo not cloned locally: {repo_name}'}, 404)
                return
            script = (
                'tell application "iTerm"\n'
                '    activate\n'
                '    if (count of windows) = 0 then\n'
                '        create window with default profile\n'
                '    end if\n'
                '    tell current window\n'
                '        create tab with default profile\n'
                '        tell current session of current tab\n'
                f'            write text "cd {str(local_dir)}"\n'
                '        end tell\n'
                '    end tell\n'
                'end tell'
            )
            r = subprocess.run(['osascript', '-e', script],
                               capture_output=True, text=True)
            if r.returncode != 0:
                self._json_response({'error': r.stderr.strip() or 'osascript failed'}, 500)
                return
            self._json_response({'ok': True})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/kb/reindex ─────────────────────────────────────────────

    def _handle_kb_reindex(self):
        try:
            global _reindex_job
            data = self._read_json()
            repo = data.get('repo', '').strip()
            if not repo or '/' not in repo:
                self._json_response({'error': 'invalid repo format'}, 400)
                return
            repo_name = repo.split('/')[-1]
            local_dir = KNOWLEDGE_BASE_DIR / repo_name
            if not local_dir.is_dir():
                self._json_response({'error': f'repo not cloned locally: {repo_name}'}, 404)
                return

            # Verify repo is in topics.json
            try:
                topics_data = load_topics(REPO_ROOT)
                repos_in_topics = [t['repo'] for t in topics_data.get('topics', []) if 'repo' in t]
                if repo not in repos_in_topics:
                    self._json_response({'error': f'repo not in topics.json: {repo}'}, 404)
                    return
            except Exception:
                pass  # proceed even if topics.json can't be read

            if _reindex_job['status'] == 'running':
                self._json_response({'error': 'already running'}, 409)
                return

            now_iso = datetime.now(timezone.utc).isoformat()
            _reindex_job = {
                'status': 'running',
                'started_at': now_iso,
                'finished_at': None,
                'log': f'准备重建 {repo_name}…',
            }

            def _do_kb_reindex():
                global _reindex_job
                try:
                    # Clear commit cache for this repo to force reindex
                    commit_cache_file = CACHE_DIR / 'repo-commits.json'
                    if commit_cache_file.exists():
                        try:
                            cache = json.loads(commit_cache_file.read_text(encoding='utf-8'))
                            if repo in cache:
                                del cache[repo]
                                commit_cache_file.write_text(
                                    json.dumps(cache, ensure_ascii=False, indent=2),
                                    encoding='utf-8'
                                )
                        except Exception:
                            pass

                    _reindex_job['log'] = f'重建索引 {repo_name}…'
                    script = REPO_ROOT / 'scripts' / 'build_knowledge_index.py'
                    r = subprocess.run(
                        ['python3', str(script)],
                        cwd=str(REPO_ROOT), capture_output=True, text=True, timeout=600
                    )
                    if r.returncode != 0:
                        _reindex_job['status'] = 'error'
                        _reindex_job['log'] = r.stderr.strip() or r.stdout.strip() or '未知错误'
                    else:
                        _reindex_job['status'] = 'done'
                        _reindex_job['log'] = r.stdout.strip() or '完成'
                except Exception as e:
                    _reindex_job['status'] = 'error'
                    _reindex_job['log'] = str(e)
                finally:
                    _reindex_job['finished_at'] = datetime.now(timezone.utc).isoformat()

            threading.Thread(target=_do_kb_reindex, daemon=True).start()
            self._json_response({'status': 'running'})
        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/search-workbench ────────────────────────────────────────

    def _handle_search_workbench(self):
        try:
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            q = params.get('q', [''])[0].strip()
            if not q:
                self._json_response({'error': 'q parameter required'}, 400)
                return
            try:
                limit = min(int(params.get('limit', ['10'])[0]), 50)
            except ValueError:
                limit = 10

            result = _meili_request('POST', '/indexes/workbench/search', {
                'q': q,
                'limit': limit,
                'attributesToCrop': ['body'],
                'cropLength': 80,
                'attributesToHighlight': ['body'],
            })
            if result is None:
                self._json_response({'hits': [], 'error': 'unavailable'})
                return
            if result.get('code') == 'index_not_found':
                self._json_response({'hits': [], 'error': 'not_indexed'})
                return

            hits = result.get('hits', [])
            self._json_response({'hits': hits})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: POST /api/reindex-workbench ──────────────────────────────────────

    def _handle_reindex_workbench(self):
        try:
            global _reindex_wb_job
            if _reindex_wb_job['status'] == 'running':
                self._json_response({'error': 'already running'}, 409)
                return

            now_iso = datetime.now(timezone.utc).isoformat()
            _reindex_wb_job = {
                'status': 'running',
                'started_at': now_iso,
                'finished_at': None,
                'log': '启动中…',
            }

            def _do_wb_reindex():
                global _reindex_wb_job
                try:
                    script = REPO_ROOT / 'scripts' / 'build_workbench_index.py'
                    r = subprocess.run(
                        ['python3', str(script), '--wipe'],
                        cwd=str(REPO_ROOT), capture_output=True, text=True, timeout=300
                    )
                    if r.returncode != 0:
                        _reindex_wb_job['status'] = 'error'
                        _reindex_wb_job['log'] = r.stderr.strip() or r.stdout.strip() or '未知错误'
                    else:
                        _reindex_wb_job['status'] = 'done'
                        _reindex_wb_job['log'] = r.stdout.strip() or '完成'
                except Exception as e:
                    _reindex_wb_job['status'] = 'error'
                    _reindex_wb_job['log'] = str(e)
                finally:
                    _reindex_wb_job['finished_at'] = datetime.now(timezone.utc).isoformat()

            threading.Thread(target=_do_wb_reindex, daemon=True).start()
            self._json_response({'status': 'running'})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: GET /api/reindex-workbench-status ────────────────────────────────

    def _handle_reindex_workbench_status(self):
        self._json_response(_reindex_wb_job)

    # ── API: comment draft cache ──────────────────────────────────────────────

    def _handle_get_draft(self):
        qs = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
        common_path = qs.get('path', [''])[0].strip()
        if not common_path or '..' in common_path:
            self._json_response({'error': 'Invalid path'}, 400)
            return
        draft_file = (CACHE_DIR / 'drafts' / common_path).resolve()
        if not str(draft_file).startswith(str((CACHE_DIR / 'drafts').resolve())):
            self._json_response({'error': 'Path traversal not allowed'}, 400)
            return
        content = draft_file.read_text('utf-8') if draft_file.exists() else ''
        self._json_response({'content': content})

    def _handle_save_draft(self):
        try:
            data = self._read_json()
            common_path = data.get('common_path', '').strip()
            content = data.get('content', '')
            if not common_path or '..' in common_path:
                self._json_response({'error': 'Invalid path'}, 400)
                return
            drafts_dir = CACHE_DIR / 'drafts'
            draft_file = (drafts_dir / common_path).resolve()
            if not str(draft_file).startswith(str(drafts_dir.resolve())):
                self._json_response({'error': 'Path traversal not allowed'}, 400)
                return
            if content:
                draft_file.parent.mkdir(parents=True, exist_ok=True)
                draft_file.write_text(content, encoding='utf-8')
            elif draft_file.exists():
                draft_file.unlink()
            self._json_response({'ok': True})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)


print(f'lulu-workbench viewer')
print(f'  Root : {REPO_ROOT}')
print(f'  URL  : http://localhost:{PORT}')
print()

# Auto-start Meilisearch if not already running
if _meili_request('GET', '/health') is None:
    print('  Starting Meilisearch…', end='', flush=True)
    try:
        cmd = ['meilisearch', '--no-analytics',
               '--db-path', str(REPO_ROOT / '.meilisearch'),
               '--master-key', MEILI_KEY]
        subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(10):
            time.sleep(0.5)
            if _meili_request('GET', '/health') is not None:
                print(' ✓')
                break
        else:
            print('\n  ⚠ Meilisearch auto-start failed (not in PATH or timed out)')
    except FileNotFoundError:
        print('\n  ⚠ meilisearch not found in PATH, search disabled')
    except Exception as e:
        print(f'\n  ⚠ Meilisearch auto-start error: {e}')
else:
    print('  Meilisearch running ✓')

try:
    with http.server.HTTPServer(('localhost', PORT), Handler) as server:
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            print('\nStopped.')
except OSError as e:
    print(f'\n❌ 启动失败：{e}')
    print(f'   端口 {PORT} 可能已被占用。')
    print(f'   运行 lsof -ti:{PORT} | xargs kill 后重试。')
    raise SystemExit(1)
