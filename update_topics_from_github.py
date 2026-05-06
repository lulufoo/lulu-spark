#!/usr/bin/env python3
"""Regenerate this repo's ``topics.json`` from GitHub: KNOWLEDGE_CORPUS repos, 1st/2nd level dirs.

Run from a clone of ``lulufoo/lulu-workbench`` (or anywhere), with ``gh`` logged in.
Uses ``gh api`` only (no ``echo`` of large JSON). **Schema v3 (compact):**
  - Root: ``version``, short ``source``, ``defaultBranch``, ``bl`` (top-level name blacklist),
    ``topics`` = array of projects.
  - Each project: ``repo`` (``owner/name``), ``m`` = 1st→2nd dir map, optional ``b`` if
    default branch differs from root ``defaultBranch``.
  - No ``html_url``; compact JSON (no extra whitespace).
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
from collections import Counter
from pathlib import Path
from typing import Any

# Default: ``topics.json`` next to this script (repo root)
_REPO_ROOT = Path(__file__).resolve().parent
DEFAULT_CTA_TOPICS = str(_REPO_ROOT / "topics.json")
KNOWLEDGE_TYPE = "KNOWLEDGE_CORPUS"


def _parse_gh_paginate_array(stdout: str) -> list[dict[str, Any]]:
    if not stdout.strip():
        return []
    if stdout.strip().startswith("["):
        return json.loads(stdout)
    out: list[dict[str, Any]] = []
    i, n = 0, len(stdout)
    while i < n:
        while i < n and stdout[i] in " \t\n\r":
            i += 1
        if i >= n:
            break
        if stdout[i] != "[":
            raise ValueError("Expected JSON array from gh --paginate")
        depth = 0
        j = i
        while j < n:
            c = stdout[j]
            if c == "[":
                depth += 1
            elif c == "]":
                depth -= 1
                if depth == 0:
                    j += 1
                    break
            j += 1
        out.extend(json.loads(stdout[i:j]))
        i = j
    return out


def list_owner_repos() -> list[dict[str, Any]]:
    r = subprocess.run(
        [
            "gh",
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            "user/repos?per_page=100&affiliation=owner",
            "--paginate",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if r.returncode != 0:
        raise RuntimeError("gh user/repos: " + (r.stderr or r.stdout))
    return _parse_gh_paginate_array(r.stdout)


def _is_knowledge_corpus(full_name: str, default_branch: str) -> bool:
    path = f"repos/{full_name}/contents/{urllib.parse.quote('.repository-type.json', safe='')}"
    r = subprocess.run(
        [
            "gh",
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            f"{path}?ref={urllib.parse.quote(default_branch)}",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if r.returncode != 0:
        return False
    try:
        meta = json.loads(r.stdout)
    except json.JSONDecodeError:
        return False
    if meta.get("type") != "file" or "content" not in meta:
        return False
    rawb = base64.b64decode((meta.get("content") or "").replace("\n", ""))
    try:
        doc = json.loads(rawb.decode("utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError, ValueError, binascii.Error):
        return False
    t = doc.get("type")
    if not isinstance(t, str) or t.strip().upper() != KNOWLEDGE_TYPE:
        return False
    return True


def get_directory_children(full_name: str, rel_path: str, default_branch: str) -> list[dict[str, Any]]:
    if rel_path in ("", "."):
        q = f"repos/{full_name}/contents?ref={urllib.parse.quote(default_branch)}"
    else:
        p = rel_path.split("/")
        enc = "/".join(urllib.parse.quote(seg, safe="") for seg in p)
        q = f"repos/{full_name}/contents/{enc}?ref={urllib.parse.quote(default_branch)}"
    r = subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github+json", q],
        capture_output=True,
        text=True,
        check=False,
    )
    if r.returncode != 0:
        return []
    j = json.loads(r.stdout)
    if isinstance(j, list):
        return j
    return [j]


def _pick_default_branch(branches: list[str]) -> str:
    if not branches:
        return "main"
    c = Counter(branches)
    mx = max(c.values())
    top = [b for b, n in c.items() if n == mx]
    if "main" in top:
        return "main"
    return min(top)


def run(out_path: str) -> int:
    # 读取已有 topics.json，保留手动维护的 dir 字段和虚拟条目
    existing_dirs: dict[str, str] = {}
    virtual_entries: list[dict[str, Any]] = []
    if Path(out_path).exists():
        with open(out_path, encoding="utf-8") as f:
            existing = json.load(f)
        for item in existing.get("topics", []):
            if "repo" in item and "dir" in item:
                existing_dirs[item["repo"]] = item["dir"]
            if "repo" not in item:
                virtual_entries.append(item)

    repos = list_owner_repos()
    discovered: list[tuple[str, str]] = []
    for r in sorted(repos, key=lambda x: (x.get("full_name") or "")):
        full = r.get("full_name")
        if not full:
            continue
        branch = r.get("default_branch") or "main"
        if not _is_knowledge_corpus(full, branch):
            continue
        discovered.append((full, branch))

    dbranch = _pick_default_branch([b for _, b in discovered])
    topics_list: list[dict[str, Any]] = []
    for full, branch in discovered:
        entry: dict[str, Any] = {"repo": full}
        if full in existing_dirs:
            entry["dir"] = existing_dirs[full]
        topics_list.append(entry)

    # 追加虚拟条目（无 repo 字段，如 common-tech）
    topics_list.extend(virtual_entries)

    doc: dict[str, Any] = {
        "version": 4,
        "source": "CTA KNOWLEDGE_CORPUS (gh)",
        "defaultBranch": dbranch,
        "topics": topics_list,
    }
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(
        f"Wrote {out_path} v4 with {len(topics_list)} project(s) ({os.path.getsize(out_path)} bytes).",
        file=sys.stderr,
    )
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "-o",
        "--output",
        default=os.environ.get("CTA_TOPICS_PATH", DEFAULT_CTA_TOPICS),
        help=f"Path to topics.json (default: {DEFAULT_CTA_TOPICS})",
    )
    args = p.parse_args()
    try:
        return run(args.output)
    except FileNotFoundError as e:
        print(e, file=sys.stderr)
        return 1
    except RuntimeError as e:
        print(e, file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
