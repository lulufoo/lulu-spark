#!/usr/bin/env python3
"""Tests for server corpus path mapping (without starting HTTP server)."""
from __future__ import annotations

import http.server
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

_REPO = Path(__file__).resolve().parent.parent


def _load_server_namespace():
    """Load server.py without executing the HTTPServer block at file end."""
    src = (_REPO / 'server.py').read_text(encoding='utf-8')
    marker = "try:\n    with http.server.HTTPServer"
    idx = src.find(marker)
    if idx > 0:
        src = src[:idx]
    # Skip startup prints / Meilisearch probe
    boot = "print(f'lulu-workbench viewer')"
    boot_idx = src.find(boot)
    if boot_idx > 0:
        src = src[:boot_idx]
    ns: dict = {'__name__': 'server_under_test', '__file__': str(_REPO / 'server.py')}
    exec(compile(src, str(_REPO / 'server.py'), 'exec'), ns)  # noqa: S102
    return ns


class TestTranslatePath(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.srv = _load_server_namespace()

    def _handler(self, repo: Path):
        Handler = self.srv['Handler']
        handler = Handler.__new__(Handler)
        handler.directory = str(repo / 'frontend')
        return handler

    def test_maps_digest_to_corpus_root(self):
        with tempfile.TemporaryDirectory() as repo_tmp, tempfile.TemporaryDirectory() as corpus_tmp:
            repo = Path(repo_tmp)
            corpus = Path(corpus_tmp)
            (repo / 'frontend').mkdir(parents=True)
            (corpus / 'digest').mkdir(parents=True)
            (corpus / 'digest' / 'proj.md').write_text('# x', encoding='utf-8')

            self.srv['REPO_ROOT'] = repo
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            result = self._handler(repo).translate_path('/digest/proj.md')
            self.assertEqual(result, str((corpus / 'digest/proj.md').resolve()))

    def test_non_corpus_path_unchanged(self):
        with tempfile.TemporaryDirectory() as repo_tmp, tempfile.TemporaryDirectory() as corpus_tmp:
            repo = Path(repo_tmp)
            corpus = Path(corpus_tmp)
            (repo / 'frontend' / 'js').mkdir(parents=True)
            (repo / 'frontend' / 'js' / 'app.js').write_text('//', encoding='utf-8')

            self.srv['REPO_ROOT'] = repo
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            handler = self._handler(repo)
            mapped = handler.translate_path('/js/app.js')
            default = http.server.SimpleHTTPRequestHandler.translate_path(handler, '/js/app.js')
            self.assertEqual(mapped, default)

    def test_static_root_under_frontend_still_redirects_corpus(self):
        with tempfile.TemporaryDirectory() as repo_tmp, tempfile.TemporaryDirectory() as corpus_tmp:
            repo = Path(repo_tmp)
            corpus = Path(corpus_tmp)
            (repo / 'frontend' / 'digest').mkdir(parents=True)
            (repo / 'frontend' / 'digest' / 'decoy.md').write_text('# decoy', encoding='utf-8')
            (corpus / 'digest').mkdir(parents=True)
            (corpus / 'digest' / 'proj.md').write_text('# x', encoding='utf-8')

            self.srv['REPO_ROOT'] = repo
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            result = self._handler(repo).translate_path('/digest/proj.md')
            self.assertEqual(result, str((corpus / 'digest/proj.md').resolve()))

    def test_raw_nested_path_maps_to_corpus(self):
        with tempfile.TemporaryDirectory() as repo_tmp, tempfile.TemporaryDirectory() as corpus_tmp:
            repo = Path(repo_tmp)
            corpus = Path(corpus_tmp)
            (corpus / 'raw' / 'nested').mkdir(parents=True)
            (corpus / 'raw' / 'nested' / 'deep.md').write_text('# deep', encoding='utf-8')

            self.srv['REPO_ROOT'] = repo
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            result = self._handler(repo).translate_path('/raw/nested/deep.md')
            self.assertEqual(result, str((corpus / 'raw/nested/deep.md').resolve()))

    def test_handler_directory_is_frontend(self):
        with tempfile.TemporaryDirectory() as repo_tmp:
            repo = Path(repo_tmp)
            (repo / 'frontend').mkdir()
            self.srv['REPO_ROOT'] = repo
            handler = self._handler(repo)
            self.assertEqual(Path(handler.directory).resolve(), (repo / 'frontend').resolve())

    def test_index_json_maps_to_corpus(self):
        with tempfile.TemporaryDirectory() as repo_tmp, tempfile.TemporaryDirectory() as corpus_tmp:
            repo = Path(repo_tmp)
            corpus = Path(corpus_tmp)
            (corpus / 'index.json').write_text('{}', encoding='utf-8')

            self.srv['REPO_ROOT'] = repo
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            result = self._handler(repo).translate_path('/index.json')
            self.assertEqual(result, str((corpus / 'index.json').resolve()))


class TestCorpusGitRoot(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.srv = _load_server_namespace()

    def test_raises_when_not_git_repo(self):
        with tempfile.TemporaryDirectory() as corpus_tmp:
            corpus = Path(corpus_tmp)
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            with self.assertRaises(ValueError) as ctx:
                self.srv['_corpus_git_root']()
            self.assertIn('not a git repository', str(ctx.exception))

    def test_returns_resolved_path_when_git_exists(self):
        with tempfile.TemporaryDirectory() as corpus_tmp:
            corpus = Path(corpus_tmp)
            subprocess.run(['git', 'init'], cwd=corpus, capture_output=True, check=True)
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            self.assertEqual(self.srv['_corpus_git_root'](), corpus.resolve())


class TestCorpusGitHandlers(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.srv = _load_server_namespace()

    def _handler(self):
        Handler = self.srv['Handler']
        handler = Handler.__new__(Handler)
        handler._json_responses = []

        def _json_response(data, status=200):
            handler._json_responses.append((data, status))

        handler._json_response = _json_response
        handler._read_json = lambda: {}
        return handler

    def test_status_uses_corpus_cwd(self):
        with tempfile.TemporaryDirectory() as repo_tmp, tempfile.TemporaryDirectory() as corpus_tmp:
            repo = Path(repo_tmp)
            corpus = Path(corpus_tmp)
            subprocess.run(['git', 'init'], cwd=corpus, capture_output=True, check=True)

            self.srv['REPO_ROOT'] = repo
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            captured: list[str | None] = []

            def fake_run(cmd, **kwargs):
                captured.append(kwargs.get('cwd'))
                return subprocess.CompletedProcess(cmd, 0, stdout='', stderr='')

            handler = self._handler()
            with mock.patch('subprocess.run', side_effect=fake_run):
                handler._handle_status()

            self.assertTrue(handler._json_responses)
            self.assertEqual(captured[0], str(corpus.resolve()))
            self.assertEqual(captured[1], str(corpus.resolve()))

    def test_pull_uses_corpus_cwd(self):
        with tempfile.TemporaryDirectory() as repo_tmp, tempfile.TemporaryDirectory() as corpus_tmp:
            repo = Path(repo_tmp)
            corpus = Path(corpus_tmp)
            subprocess.run(['git', 'init'], cwd=corpus, capture_output=True, check=True)

            self.srv['REPO_ROOT'] = repo
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            captured: list[str | None] = []

            def fake_run(cmd, **kwargs):
                captured.append(kwargs.get('cwd'))
                return subprocess.CompletedProcess(cmd, 0, stdout='Already up to date.', stderr='')

            handler = self._handler()
            with mock.patch('subprocess.run', side_effect=fake_run):
                handler._handle_pull()

            self.assertEqual(len(captured), 1)
            self.assertEqual(captured[0], str(corpus.resolve()))

    def test_status_errors_when_corpus_not_git(self):
        with tempfile.TemporaryDirectory() as corpus_tmp:
            self.srv['KNOWLEDGE_CORPUS_DIR'] = Path(corpus_tmp)
            handler = self._handler()
            handler._handle_status()
            self.assertEqual(handler._json_responses[0][1], 400)
            self.assertIn('not a git repository', handler._json_responses[0][0]['error'])


# ── TestHandleTopics ───────────────────────────────────────────────────────


class TestHandleTopics(unittest.TestCase):
    """验证 _handle_topics 在所有情况下都返回 JSON，不会返回 HTML。"""

    @classmethod
    def setUpClass(cls):
        cls.srv = _load_server_namespace()

    def _handler(self):
        Handler = self.srv['Handler']
        handler = Handler.__new__(Handler)
        handler._json_responses = []

        def _json_response(data, status=200):
            handler._json_responses.append((data, status))

        handler._json_response = _json_response
        return handler

    def test_returns_200_json_when_topics_found(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            cache = repo / '.cache'
            cache.mkdir()
            topics = {'topics': [{'dir': 'ai', 'description': 'AI notes'}]}
            (cache / 'topics.json').write_text(
                json.dumps(topics), encoding='utf-8'
            )
            self.srv['REPO_ROOT'] = repo
            self.srv['TOPICS_CACHE_FILE'] = cache / 'topics.json'

            handler = self._handler()
            handler._handle_topics()

            self.assertEqual(len(handler._json_responses), 1)
            data, status = handler._json_responses[0]
            self.assertEqual(status, 200)
            self.assertEqual(data['topics'], topics['topics'])

    def test_returns_json_404_not_html_when_topics_missing(self):
        """关键：topics.json 缺失时必须返回 JSON error 404，不得触发 HTML 响应。
        这是 BUG 根因之一：若此方法异常泄漏，BaseHTTPServer 会返回 HTML 404。"""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            (repo / '.cache').mkdir()
            self.srv['REPO_ROOT'] = repo
            self.srv['TOPICS_CACHE_FILE'] = repo / '.cache' / 'topics.json'

            handler = self._handler()
            handler._handle_topics()

            self.assertEqual(len(handler._json_responses), 1)
            data, status = handler._json_responses[0]
            self.assertEqual(status, 404)
            # 必须是 JSON dict，不是 HTML 字符串
            self.assertIsInstance(data, dict)
            self.assertIn('error', data)


# ── TestDoGetRouting ───────────────────────────────────────────────────────


class TestDoGetRouting(unittest.TestCase):
    """P1/P3：已迁移只读 API 不再由 Python do_GET 处理。"""

    @classmethod
    def setUpClass(cls):
        cls.srv = _load_server_namespace()

    def test_get_api_topics_delegates_to_super(self):
        """/api/topics 已迁 Rust；Python do_GET 应落到 super().do_GET()。"""
        Handler = self.srv['Handler']
        handler = Handler.__new__(Handler)
        handler.path = '/api/topics?_=1716000000000'

        called = []
        with mock.patch.object(
            http.server.SimpleHTTPRequestHandler,
            'do_GET',
            lambda self: called.append('super'),
        ):
            handler.do_GET()

        self.assertEqual(called, ['super'])

    def test_get_reindex_status_falls_through_to_super(self):
        """P3：/api/reindex-status 已迁 Tauri invoke；Python 不再专用路由。"""
        Handler = self.srv['Handler']
        handler = Handler.__new__(Handler)
        handler.path = '/api/reindex-status'

        called = []
        with mock.patch.object(
            http.server.SimpleHTTPRequestHandler,
            'do_GET',
            lambda self: called.append('super'),
        ):
            handler.do_GET()

        self.assertEqual(called, ['super'])


# ── TestDoPostRouting ──────────────────────────────────────────────────────


class TestDoPostRouting(unittest.TestCase):
    """验证 do_POST 路由：/api/move-project 不触发 send_error(404)。"""

    @classmethod
    def setUpClass(cls):
        cls.srv = _load_server_namespace()

    def test_post_move_project_routes_to_handle_move_project(self):
        Handler = self.srv['Handler']
        handler = Handler.__new__(Handler)
        handler.path = '/api/move-project'

        called = []
        handler._handle_move_project = lambda: called.append('_handle_move_project')

        with mock.patch.object(
            Handler, 'send_error',
            side_effect=AssertionError('send_error() should not be called for /api/move-project'),
        ):
            handler.do_POST()

        self.assertEqual(called, ['_handle_move_project'])


# ── TestHandleMoveProject ──────────────────────────────────────────────────


class TestHandleMoveProject(unittest.TestCase):
    """验证 _handle_move_project 输入校验时返回 JSON error，不返回 HTML。"""

    @classmethod
    def setUpClass(cls):
        cls.srv = _load_server_namespace()

    def _handler(self, request_body):
        Handler = self.srv['Handler']
        handler = Handler.__new__(Handler)
        handler._json_responses = []

        def _json_response(data, status=200):
            handler._json_responses.append((data, status))

        handler._json_response = _json_response
        handler._read_json = lambda: request_body
        return handler

    def test_invalid_id_format_returns_400_json(self):
        """id 格式不合法时返回 400 JSON，不返回 HTML。"""
        handler = self._handler({'id': 'bad-id', 'new_project': 'ai'})
        handler._handle_move_project()
        data, status = handler._json_responses[0]
        self.assertEqual(status, 400)
        self.assertIsInstance(data, dict)
        self.assertIn('error', data)

    def test_empty_new_project_returns_400_json(self):
        handler = self._handler({'id': 'a' * 32, 'new_project': ''})
        handler._handle_move_project()
        data, status = handler._json_responses[0]
        self.assertEqual(status, 400)
        self.assertIn('error', data)

    def test_path_traversal_in_project_returns_400_json(self):
        handler = self._handler({'id': 'a' * 32, 'new_project': '../escape'})
        handler._handle_move_project()
        data, status = handler._json_responses[0]
        self.assertEqual(status, 400)
        self.assertIn('error', data)


if __name__ == '__main__':
    if str(_REPO / 'scripts') not in sys.path:
        sys.path.insert(0, str(_REPO / 'scripts'))
    unittest.main()
