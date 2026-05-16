#!/usr/bin/env python3
"""Load corpus membership from ``.cache/knowledge-index.json``."""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

DEFAULT_OWNER = "lulufoo"
INDEX_REL = Path(".cache") / "knowledge-index.json"


def load_knowledge_index(path: Path) -> list[dict[str, Any]]:
    """Return normalized entries: id, repo, description, indexUrl."""
    if not path.is_file():
        raise FileNotFoundError(f"knowledge-index not found: {path}")

    raw = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(raw, list):
        entries_in = raw
    elif isinstance(raw, dict):
        entries_in = raw.get("entries")
        if entries_in is None:
            raise ValueError("knowledge-index JSON must have 'entries' array")
    else:
        raise ValueError("knowledge-index JSON must be object or array")

    if not isinstance(entries_in, list):
        raise ValueError("'entries' must be an array")

    out: list[dict[str, Any]] = []
    for item in entries_in:
        if not isinstance(item, dict):
            continue
        entry_id = str(item.get("id") or "").strip()
        if not entry_id:
            continue
        desc = str(item.get("description") or "").strip()
        if not desc:
            continue
        repo = str(item.get("repo") or "").strip()
        if not repo:
            repo = f"{DEFAULT_OWNER}/{entry_id}"
        index_url = str(item.get("indexUrl") or item.get("index_url") or "").strip()
        if not index_url:
            index_url = f"https://github.com/{repo}/blob/main/_index.md"
        out.append({
            "id": entry_id,
            "repo": repo,
            "description": desc,
            "indexUrl": index_url,
        })

    if not out:
        raise ValueError("no valid entries in knowledge-index")
    return out


def write_knowledge_index(entries: list[dict[str, Any]], path: Path) -> None:
    """Write authority file (migrate script only; not used by server API)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps({"version": 1, "entries": entries}, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def knowledge_index_path(repo_root: Path | None = None) -> Path:
    """Resolve authority JSON path; raise if missing (server must not synthesize data)."""
    root = (repo_root or Path(__file__).resolve().parent.parent).resolve()
    out = root / INDEX_REL
    if not out.is_file():
        raise FileNotFoundError("knowledge-index.json not found")
    return out
