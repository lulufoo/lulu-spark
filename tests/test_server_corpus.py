#!/usr/bin/env python3
"""Tests for server corpus path mapping (without starting HTTP server)."""
from __future__ import annotations

import http.server
import sys
import tempfile
import unittest
from pathlib import Path

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


if __name__ == '__main__':
    if str(_REPO / 'scripts') not in sys.path:
        sys.path.insert(0, str(_REPO / 'scripts'))
    unittest.main()
