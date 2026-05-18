"""Shared configuration loader for lulu-workbench.

Single source of truth for reading meili.env.
All consumers (server.py, scripts) must use this module.
"""
from __future__ import annotations

from pathlib import Path

_DEFAULT_CORPUS_GITHUB = 'https://github.com/lulufoo/lulu-workbench-knowledge/blob/main'


def load_meili_env(prog_root: Path) -> dict[str, str]:
    """Parse meili.env from prog_root and return a key→value dict."""
    env_file = prog_root / 'meili.env'
    result: dict[str, str] = {}
    if not env_file.exists():
        return result
    for line in env_file.read_text(encoding='utf-8').splitlines():
        line = line.strip()
        if not line or line.startswith('#') or '=' not in line:
            continue
        key, _, val = line.partition('=')
        result[key.strip()] = val.strip()
    return result


def get_corpus_root(prog_root: Path) -> Path:
    """Return KNOWLEDGE_CORPUS_DIR from meili.env, or prog_root as fallback."""
    env = load_meili_env(prog_root)
    val = env.get('KNOWLEDGE_CORPUS_DIR', '').strip()
    return Path(val) if val else prog_root


def get_corpus_github(prog_root: Path) -> str:
    """Return GitHub blob URL base for corpus files (no trailing slash)."""
    env = load_meili_env(prog_root)
    val = env.get('KNOWLEDGE_CORPUS_GITHUB', '').strip().rstrip('/')
    return val or _DEFAULT_CORPUS_GITHUB
