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
import uuid
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
        elif self.path == '/api/pull':
            self._handle_pull()
        elif self.path == '/api/update-links':
            self._handle_update_links()
        elif self.path == '/api/update-comments':
            self._handle_update_comments()
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
        else:
            self.send_error(404)

    def do_GET(self):
        parsed_path = urllib.parse.urlparse(self.path).path
        if parsed_path.startswith('/api/fetch-title'):
            self._handle_fetch_title()
        elif parsed_path == '/api/config':
            self._json_response({'archive_root': str(REPO_ROOT)})
        elif parsed_path == '/api/annotations':
            self._handle_get_annotations()
        elif parsed_path == '/api/annotation':
            self._handle_get_annotation()
        elif parsed_path == '/api/status':
            self._handle_status()
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
            result = subprocess.run(
                ['git', 'status', '--porcelain'],
                cwd=REPO_ROOT, capture_output=True, text=True
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
                cwd=REPO_ROOT, capture_output=True, text=True
            )
            ahead = int(ahead_r.stdout.strip()) if ahead_r.returncode == 0 and ahead_r.stdout.strip().isdigit() else 0
            self._json_response({**categories, 'total': total, 'ahead': ahead})
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

    # ── Annotation helpers ─────────────────────────────────────────────────────

    def _annotation_path(self, common_path):
        """Return resolved Path for annotation file, or None if invalid."""
        if not common_path or '..' in common_path:
            return None
        rel = (common_path[:-3] + '.json') if common_path.endswith('.md') else (common_path + '.json')
        target = (REPO_ROOT / 'annotations' / rel).resolve()
        if not str(target).startswith(str(REPO_ROOT) + os.sep):
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
            index_path = REPO_ROOT / 'index.json'
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

            hid  = str(highlight.get('id') or '').strip()
            text = str(highlight.get('text') or '').strip()
            ts   = str(data.get('ts') or '').strip()

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
                highlights.append({'id': hid, 'text': text, 'ts': ts})

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


    # ── API: gh move (copy file to new repo/dir, delete source) ─────────────

    def _handle_gh_move(self):
        try:
            data = self._read_json()
            src_url = (data.get('src_url') or '').strip()
            dst_dir_url = (data.get('dst_dir_url') or '').strip()

            # Parse source: must be a GitHub blob URL
            src_m = re.match(
                r'https://github\.com/([^/]+)/([^/]+)/blob/([^/]+)/(.+)',
                src_url
            )
            if not src_m:
                self._json_response({'error': '源文件 URL 格式无效，需为 GitHub blob URL'}, 400)
                return
            src_owner, src_repo, src_ref, src_path = (
                src_m.group(1), src_m.group(2), src_m.group(3), src_m.group(4)
            )
            filename = src_path.split('/')[-1]

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

            # Build destination path
            if dst_dir:
                dst_path = f'{dst_dir}/{filename}'
            else:
                dst_path = filename

            # Step 1: Fetch source file content + SHA
            get_r = subprocess.run(
                ['gh', 'api', f'repos/{src_owner}/{src_repo}/contents/{src_path}?ref={src_ref}'],
                capture_output=True, text=True, timeout=20
            )
            if get_r.returncode != 0:
                self._json_response({
                    'error': f'获取源文件失败：{get_r.stderr.strip() or get_r.stdout.strip()}'
                }, 500)
                return
            src_info = json.loads(get_r.stdout)
            file_content_b64 = src_info.get('content', '').replace('\n', '')
            src_sha = src_info.get('sha', '')
            if not file_content_b64:
                self._json_response({'error': '源文件内容为空或无法读取'}, 500)
                return

            # Step 2: Create file at destination
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
                self._json_response({'error': f'创建目标文件失败：{err_msg}'}, 500)
                return

            # Step 3: Delete source file (only after create succeeded)
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
                # File was already copied; warn but don't fail completely
                self._json_response({
                    'ok': True,
                    'warn': f'目标文件已创建，但删除源文件失败：{err_msg}',
                    'dst_path': dst_path
                })
                return

            print(f'  [gh-move] {src_owner}/{src_repo}/{src_path} → {dst_owner}/{dst_repo}/{dst_path}')
            self._json_response({'ok': True, 'dst_path': dst_path})

        except json.JSONDecodeError:
            self._json_response({'error': 'Invalid JSON body'}, 400)
        except subprocess.TimeoutExpired:
            self._json_response({'error': 'gh 命令超时'}, 500)
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
