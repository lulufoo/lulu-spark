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


def _read_repo_meta(full_name: str, default_branch: str) -> dict[str, Any]:
    """Read description + keywords from .repository-type.json (best-effort)."""
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
    except Exception:
        return {}
    result: dict[str, Any] = {}
    if doc.get("description"):
        result["description"] = doc["description"]
    if doc.get("keywords"):
        result["keywords"] = doc["keywords"]
    return result


def _check_and_get_meta(full_name: str, default_branch: str) -> tuple[bool, dict[str, Any]]:
    """Check KNOWLEDGE_CORPUS type AND read meta in a single API call.

    Returns (is_corpus, meta_dict). Avoids the double-fetch that
    _is_knowledge_corpus + _read_repo_meta would require.
    """
    path = f"repos/{full_name}/contents/{urllib.parse.quote('.repository-type.json', safe='')}"
    r = subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github+json",
         f"{path}?ref={urllib.parse.quote(default_branch)}"],
        capture_output=True, text=True, check=False,
    )
    if r.returncode != 0:
        return False, {}
    try:
        meta = json.loads(r.stdout)
        rawb = base64.b64decode((meta.get("content") or "").replace("\n", ""))
        doc = json.loads(rawb.decode("utf-8"))
    except Exception:
        return False, {}
    t = doc.get("type")
    if not isinstance(t, str) or t.strip().upper() != KNOWLEDGE_TYPE:
        return False, {}
    result: dict[str, Any] = {}
    if doc.get("description"):
        result["description"] = doc["description"]
    if doc.get("keywords"):
        result["keywords"] = doc["keywords"]
    return True, result


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


def run(out_path: str, rediscover: bool = False, check_repo: str = "") -> int:
    # 读取已有 topics.json，保留手动维护的 dir 字段和虚拟条目
    existing_dirs: dict[str, str] = {}
    existing_repos: list[str] = []  # ordered list of repo full_names
    virtual_entries: list[dict[str, Any]] = []
    default_branch_hint = "main"
    if Path(out_path).exists():
        with open(out_path, encoding="utf-8") as f:
            existing = json.load(f)
        default_branch_hint = existing.get("defaultBranch", "main")
        for item in existing.get("topics", []):
            if "repo" in item:
                existing_repos.append(item["repo"])
                if "dir" in item:
                    existing_dirs[item["repo"]] = item["dir"]
            else:
                virtual_entries.append(item)

    # discovered: (full_name, branch, pre_fetched_meta_or_None)
    # pre_fetched_meta is set for newly discovered repos (check+meta in one call)
    # None means meta will be fetched in the unified loop below
    discovered: list[tuple[str, str, dict[str, Any] | None]] = []

    if check_repo:
        # Single-repo check: verify corpus type, append if new, then fast-refresh all
        check_repo = check_repo.strip()
        existing_set = set(existing_repos)
        if check_repo in existing_set:
            print(f"  [update-topics] check-repo: {check_repo} already exists, refreshing meta only", file=sys.stderr)
        else:
            branch = default_branch_hint
            is_corpus, meta = _check_and_get_meta(check_repo, branch)
            if not is_corpus:
                print(f"  [update-topics] check-repo: {check_repo} is NOT a KNOWLEDGE_CORPUS repo", file=sys.stderr)
                return 2
            existing_repos.append(check_repo)
            print(f"  [update-topics] check-repo: added {check_repo}", file=sys.stderr)
        # Fall through to fast refresh of all repos (including newly added)
        discovered = [(repo, default_branch_hint, None) for repo in existing_repos]
        dbranch = default_branch_hint
        print(f"  [update-topics] fast-refreshing {len(existing_repos)} repos after check-repo…", file=sys.stderr)

    elif rediscover:
        # Smart rediscover: skip type-check for already-known repos (saves N API calls)
        all_repos = list_owner_repos()
        existing_set = set(existing_repos)
        repo_branch_map: dict[str, str] = {
            r["full_name"]: (r.get("default_branch") or "main")
            for r in all_repos if r.get("full_name")
        }
        # Known repos: keep existing order, skip corpus check
        for full in existing_repos:
            branch = repo_branch_map.get(full, default_branch_hint)
            discovered.append((full, branch, None))
        # New repos: check corpus type + read meta in a single API call each
        new_found = 0
        for full in sorted(repo_branch_map):
            if full in existing_set:
                continue
            branch = repo_branch_map[full]
            is_corpus, meta = _check_and_get_meta(full, branch)
            if is_corpus:
                discovered.append((full, branch, meta))
                new_found += 1
                print(f"  [update-topics] new corpus repo: {full}", file=sys.stderr)
        print(
            f"  [update-topics] rediscover: known={len(existing_repos)}, new={new_found}",
            file=sys.stderr,
        )
        dbranch = _pick_default_branch([b for _, b, _ in discovered])
    else:
        # Fast mode: only refresh meta for known repos, no new repo discovery
        print(f"  [update-topics] fast mode: refreshing {len(existing_repos)} repos…", file=sys.stderr)
        discovered = [(repo, default_branch_hint, None) for repo in existing_repos]
        dbranch = default_branch_hint

    topics_list: list[dict[str, Any]] = []
    for full, branch, pre_meta in discovered:
        entry: dict[str, Any] = {"repo": full}
        if full in existing_dirs:
            entry["dir"] = existing_dirs[full]
        meta = pre_meta if pre_meta is not None else _read_repo_meta(full, branch)
        if meta.get("description"):
            entry["description"] = meta["description"]
        if meta.get("keywords"):
            entry["keywords"] = meta["keywords"]
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

    # Generate knowledge-index.md next to this script's refactor-2.0 dir
    index_path = Path(out_path).parent / "refactor-2.0" / "knowledge-index.md"
    _write_knowledge_index(topics_list, index_path)
    print(f"Wrote {index_path}", file=sys.stderr)
    return 0


def _write_knowledge_index(topics_list: list[dict[str, Any]], out_path: Path) -> None:
    lines = [
        "# LuLu 知识库一级索引",
        "",
        "> 本文件由 `update_topics_from_github.py` 自动生成，勿手动编辑。",
        "> 将下方文本块整体复制，粘贴到 system prompt 或 instruction。",
        "",
        "---",
        "",
        "```",
        "## LuLu 知识库",
        "",
    ]
    for t in topics_list:
        repo = t.get("repo")
        desc = t.get("description")
        if not repo or not desc:
            continue
        repo_name = repo.split("/")[1]
        url = f"https://github.com/{repo}/blob/main/_index.md"
        lines.append(f"{repo_name}: {desc}; {url}")
    lines += ["", "```", ""]
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "-o",
        "--output",
        default=os.environ.get("CTA_TOPICS_PATH", DEFAULT_CTA_TOPICS),
        help=f"Path to topics.json (default: {DEFAULT_CTA_TOPICS})",
    )
    p.add_argument(
        "--fast",
        action="store_true",
        help="Fast mode (default): only refresh meta for known repos, no new repo discovery",
    )
    p.add_argument(
        "--rediscover",
        action="store_true",
        help="Smart rediscover: scan all owner repos for new KNOWLEDGE_CORPUS repos (~37 API calls)",
    )
    p.add_argument(
        "--check-repo",
        metavar="OWNER/REPO",
        default="",
        help="Check a single repo, append to topics.json if it is KNOWLEDGE_CORPUS, then fast-refresh",
    )
    args = p.parse_args()
    try:
        return run(args.output, rediscover=args.rediscover, check_repo=args.check_repo)
    except FileNotFoundError as e:
        print(e, file=sys.stderr)
        return 1
    except RuntimeError as e:
        print(e, file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
