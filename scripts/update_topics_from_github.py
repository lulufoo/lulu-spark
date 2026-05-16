#!/usr/bin/env python3
"""Regenerate ``topics.json`` from ``.cache/knowledge-index.json`` + GitHub meta.

Run from a clone of ``lulufoo/lulu-workbench`` with ``gh`` logged in.
Reads corpus membership only from the knowledge-index JSON (never writes it back).
"""
from __future__ import annotations

import argparse
import base64
import binascii
import json
import os
import subprocess
import sys
import urllib.parse
from pathlib import Path
from typing import Any

_SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = _SCRIPT_DIR.parent
DEFAULT_TOPICS = REPO_ROOT / "topics.json"
DEFAULT_INDEX = REPO_ROOT / ".cache" / "knowledge-index.json"

if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

from knowledge_index_loader import knowledge_index_path, load_knowledge_index  # noqa: E402


def _read_repo_meta(full_name: str, default_branch: str) -> dict[str, Any]:
    """Read keywords from .repository-type.json (best-effort). Description comes from index."""
    path = f"repos/{full_name}/contents/{urllib.parse.quote('.repository-type.json', safe='')}"
    r = subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github+json",
         f"{path}?ref={urllib.parse.quote(default_branch)}"],
        capture_output=True, text=True, check=False,
    )
    if r.returncode != 0:
        return {}
    try:
        meta = json.loads(r.stdout)
        rawb = base64.b64decode((meta.get("content") or "").replace("\n", ""))
        doc = json.loads(rawb.decode("utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError, ValueError, binascii.Error):
        return {}
    result: dict[str, Any] = {}
    if doc.get("keywords"):
        result["keywords"] = doc["keywords"]
    return result


def run(out_path: str, index_path: str) -> int:
    index_file = Path(index_path)
    if not index_file.is_file():
        index_file = knowledge_index_path(REPO_ROOT)
    index_entries = load_knowledge_index(index_file)

    existing_dirs: dict[str, str] = {}
    virtual_entries: list[dict[str, Any]] = []
    default_branch_hint = "main"
    out = Path(out_path)
    if out.is_file():
        with open(out, encoding="utf-8") as f:
            existing = json.load(f)
        default_branch_hint = existing.get("defaultBranch", "main")
        for item in existing.get("topics", []):
            if "repo" in item:
                if "dir" in item:
                    existing_dirs[item["repo"]] = item["dir"]
            else:
                virtual_entries.append(item)

    topics_list: list[dict[str, Any]] = []
    for ent in index_entries:
        full = ent["repo"]
        entry: dict[str, Any] = {"repo": full, "description": ent["description"]}
        if full in existing_dirs:
            entry["dir"] = existing_dirs[full]
        meta = _read_repo_meta(full, default_branch_hint)
        if meta.get("keywords"):
            entry["keywords"] = meta["keywords"]
        topics_list.append(entry)

    topics_list.extend(virtual_entries)

    doc: dict[str, Any] = {
        "version": 4,
        "source": "knowledge-index.json + gh meta",
        "defaultBranch": default_branch_hint,
        "topics": topics_list,
    }
    with open(out, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
        f.write("\n")
    repo_count = sum(1 for t in topics_list if "repo" in t)
    print(
        f"Wrote {out_path} with {len(topics_list)} topic(s) "
        f"({repo_count} repos from index, {os.path.getsize(out_path)} bytes).",
        file=sys.stderr,
    )
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "-o",
        "--output",
        default=str(DEFAULT_TOPICS),
        help=f"Path to topics.json (default: {DEFAULT_TOPICS})",
    )
    p.add_argument(
        "--index-path",
        default=str(DEFAULT_INDEX),
        help=f"Path to knowledge-index.json (default: {DEFAULT_INDEX})",
    )
    args = p.parse_args()
    try:
        return run(args.output, args.index_path)
    except FileNotFoundError as e:
        print(e, file=sys.stderr)
        return 1
    except ValueError as e:
        print(e, file=sys.stderr)
        return 1
    except RuntimeError as e:
        print(e, file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
