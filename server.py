#!/usr/bin/env python3
"""
Local dev server for cognitive-trace-archive viewer.
Serves static files + provides API for file editing and git commit/push.

Usage:
    python3 server.py
    # open http://localhost:8765
"""

import http.server
import json
import os
import re
import subprocess
import urllib.parse
import base64
from pathlib import Path

REPO_ROOT = Path(__file__).parent.resolve()
EDITABLE_LAYERS = {'raw', 'distilled', 'digest', 'trace'}
PORT = 8765


class Handler(http.server.SimpleHTTPRequestHandler):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(REPO_ROOT), **kwargs)

    # ── Routing ──────────────────────────────────────────────────────────────

    def do_POST(self):
        if self.path == '/api/save':
            self._handle_save()
        elif self.path == '/api/commit':
            self._handle_commit()
        elif self.path == '/api/status':
            self._handle_status()
        elif self.path == '/api/pull':
            self._handle_pull()
        elif self.path == '/api/update-links':
            self._handle_update_links()
        elif self.path == '/api/delete':
            self._handle_delete()
        elif self.path == '/api/set-done':
            self._handle_set_done()
        else:
            self.send_error(404)

    def do_GET(self):
        if self.path.startswith('/api/fetch-title'):
            self._handle_fetch_title()
        elif self.path == '/api/config':
            self._json_response({'archive_root': str(REPO_ROOT)})
        else:
            super().do_GET()

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
            target = (REPO_ROOT / layer / common_path).resolve()
            if not str(target).startswith(str(REPO_ROOT) + os.sep):
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
            data = self._read_json()
            message = data.get('message', '').strip() or 'update: edit via viewer'

            def run(cmd):
                return subprocess.run(
                    cmd, cwd=REPO_ROOT,
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
                self._json_response({'ok': True, 'info': 'nothing to commit'})
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
            result = subprocess.run(
                ['git', 'status', '--porcelain'],
                cwd=REPO_ROOT, capture_output=True, text=True
            )
            modified = []
            conflicted = []
            for line in result.stdout.splitlines():
                if not line.strip():
                    continue
                xy = line[:2]
                path = line[3:].strip()
                if ' -> ' in path:
                    path = path.split(' -> ')[1]
                if xy in ('UU', 'AA', 'DD', 'AU', 'UA', 'DU', 'UD'):
                    conflicted.append(path)
                elif xy.strip():
                    modified.append(path)
            self._json_response({'modified': modified, 'conflicted': conflicted})
        except Exception as e:
            self._json_response({'error': str(e)}, 500)

    # ── API: git pull --rebase ──────────────────────────────────────────────

    def _handle_pull(self):
        try:
            self._read_json()  # consume body
            result = subprocess.run(
                ['git', 'pull', '--rebase'],
                cwd=REPO_ROOT, capture_output=True, text=True
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

    # ── API: update links in index.json ───────────────────────────────────────

    def _handle_update_links(self):
        try:
            data = self._read_json()
            entry_id = str(data.get('id') or '').strip()
            links = data.get('links', [])

            if not re.fullmatch(r'[0-9a-f]{32}', entry_id):
                self._json_response({'error': 'Invalid id'}, 400)
                return

            # Validate each link
            for link in links:
                url = link.get('url', '')
                p = urllib.parse.urlparse(url)
                if p.scheme not in ('http', 'https') or not p.netloc:
                    self._json_response({'error': f'Invalid url: {url}'}, 400)
                    return

            index_path = REPO_ROOT / 'index.json'
            index_data = json.loads(index_path.read_text(encoding='utf-8'))
            entries = index_data.get('entries', index_data)

            if entry_id not in entries:
                self._json_response({'error': 'Entry not found'}, 404)
                return

            entries[entry_id]['links'] = links

            # Atomic write
            tmp_path = index_path.with_suffix('.json.tmp')
            tmp_path.write_text(
                json.dumps(index_data, ensure_ascii=False, indent=2),
                encoding='utf-8'
            )
            tmp_path.replace(index_path)

            print(f'  [update-links] {entry_id} → {len(links)} link(s)')
            self._json_response({'ok': True})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except Exception as e:
            self._json_response({'error': str(e)}, 500)


    # ── API: set done flag on index entry ───────────────────────────────────

    def _handle_set_done(self):
        try:
            data = self._read_json()
            entry_id = str(data.get('id') or '').strip()
            done = bool(data.get('done', False))

            if not re.fullmatch(r'[0-9a-f]{32}', entry_id):
                self._json_response({'error': 'Invalid id'}, 400)
                return

            index_path = REPO_ROOT / 'index.json'
            index_data = json.loads(index_path.read_text(encoding='utf-8'))
            entries = index_data.get('entries', index_data)

            if entry_id not in entries:
                self._json_response({'error': 'Entry not found'}, 404)
                return

            if done:
                entries[entry_id]['done'] = True
            else:
                entries[entry_id].pop('done', None)

            tmp_path = index_path.with_suffix('.json.tmp')
            tmp_path.write_text(
                json.dumps(index_data, ensure_ascii=False, indent=2),
                encoding='utf-8'
            )
            tmp_path.replace(index_path)

            print(f'  [set-done] {entry_id} → done={done}')
            self._json_response({'ok': True})

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

            index_path = REPO_ROOT / 'index.json'
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
                target = (REPO_ROOT / layer / common_path).resolve()
                if not str(target).startswith(str(REPO_ROOT) + os.sep):
                    continue
                if target.exists():
                    target.unlink()
                    deleted.append(f'{layer}/{common_path}')
                    print(f'  [delete] {layer}/{common_path}')

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


if __name__ == '__main__':
    print(f'cognitive-trace-archive viewer')
    print(f'  Root : {REPO_ROOT}')
    print(f'  URL  : http://localhost:{PORT}')
    print()
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
