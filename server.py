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
import subprocess
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
        else:
            self.send_error(404)

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


if __name__ == '__main__':
    print(f'cognitive-trace-archive viewer')
    print(f'  Root : {REPO_ROOT}')
    print(f'  URL  : http://localhost:{PORT}')
    print()
    with http.server.HTTPServer(('localhost', PORT), Handler) as server:
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            print('\nStopped.')
