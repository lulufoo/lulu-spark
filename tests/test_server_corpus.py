#!/usr/bin/env python3
"""Tests for server corpus path mapping (without starting HTTP server)."""
from __future__ import annotations

import http.server
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
        handler.directory = str(repo)
        return handler

    def test_maps_digest_to_corpus_root(self):
        with tempfile.TemporaryDirectory() as repo_tmp, tempfile.TemporaryDirectory() as corpus_tmp:
            repo = Path(repo_tmp)
            corpus = Path(corpus_tmp)
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
            (repo / 'js').mkdir()
            (repo / 'js' / 'app.js').write_text('//', encoding='utf-8')

            self.srv['REPO_ROOT'] = repo
            self.srv['KNOWLEDGE_CORPUS_DIR'] = corpus
            handler = self._handler(repo)
            mapped = handler.translate_path('/js/app.js')
            default = http.server.SimpleHTTPRequestHandler.translate_path(handler, '/js/app.js')
            self.assertEqual(mapped, default)

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


if __name__ == '__main__':
    if str(_REPO / 'scripts') not in sys.path:
        sys.path.insert(0, str(_REPO / 'scripts'))
    unittest.main()
