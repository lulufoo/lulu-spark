#!/usr/bin/env python3
"""Tests for scripts/workbench_config.py"""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

_SCRIPTS = Path(__file__).resolve().parent.parent / 'scripts'
if str(_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS))

from workbench_config import (  # noqa: E402
    get_corpus_github,
    get_corpus_root,
    load_meili_env,
)

_DEFAULT_GITHUB = 'https://github.com/lulufoo/lulu-workbench-knowledge/blob/main'


class TestLoadMeiliEnv(unittest.TestCase):
    def test_missing_file_returns_empty_dict(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(load_meili_env(Path(tmp)), {})

    def test_parses_keys_and_skips_comments(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'meili.env').write_text(
                '# comment\n'
                '\n'
                'KNOWLEDGE_CORPUS_DIR=/path/to/knowledge\n'
                'MEILI_URL=http://localhost:7700\n',
                encoding='utf-8',
            )
            env = load_meili_env(root)
            self.assertEqual(env['KNOWLEDGE_CORPUS_DIR'], '/path/to/knowledge')
            self.assertEqual(env['MEILI_URL'], 'http://localhost:7700')
            self.assertNotIn('# comment', env)


class TestGetCorpusRoot(unittest.TestCase):
    def test_returns_configured_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'meili.env').write_text(
                'KNOWLEDGE_CORPUS_DIR=/path/to/knowledge\n',
                encoding='utf-8',
            )
            self.assertEqual(get_corpus_root(root), Path('/path/to/knowledge'))

    def test_falls_back_to_prog_root_when_unset(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.assertEqual(get_corpus_root(root), root)

    def test_falls_back_when_empty_value(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'meili.env').write_text('KNOWLEDGE_CORPUS_DIR=\n', encoding='utf-8')
            self.assertEqual(get_corpus_root(root), root)


class TestGetCorpusGithub(unittest.TestCase):
    def test_returns_configured_url_without_trailing_slash(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'meili.env').write_text(
                'KNOWLEDGE_CORPUS_GITHUB=https://example.com/repo/blob/main/\n',
                encoding='utf-8',
            )
            self.assertEqual(get_corpus_github(root), 'https://example.com/repo/blob/main')

    def test_default_when_unset(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(get_corpus_github(Path(tmp)), _DEFAULT_GITHUB)


class TestImportContract(unittest.TestCase):
    def test_public_symbols_importable(self):
        from workbench_config import (  # noqa: F401
            get_corpus_github as g,
            get_corpus_root as r,
            load_meili_env as l,
        )

        self.assertTrue(callable(l))
        self.assertTrue(callable(r))
        self.assertTrue(callable(g))


if __name__ == '__main__':
    unittest.main()
