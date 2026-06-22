#!/usr/bin/env python3
"""One-shot: migrate KNOWLEDGE_CORPUS repos from repo-list.json → sediment-kb/."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

KNOWLEDGE_CORPUS = "KNOWLEDGE_CORPUS"
UNCATEGORIZED_ID = "uncategorized"
UNCATEGORIZED_NAME = "未分类"

# Keep in sync with frontend/js/main.js `_REPO_TYPE_LABELS`.
TYPE_LABELS: dict[str, str] = {
    "KNOWLEDGE_CORPUS": "沉淀知识库",
    "WORKBENCH_KNOWLEDGE": "工作台知识库",
}


def type_to_category_id(repo_type: str | None) -> str:
    value = (repo_type or "").strip()
    if not value:
        return UNCATEGORIZED_ID
    return value.lower()


def type_to_category_name(repo_type: str | None) -> str:
    value = (repo_type or "").strip()
    if not value:
        return UNCATEGORIZED_NAME
    return TYPE_LABELS.get(value, value)


def load_repo_list_knowledge_corpus(cache_dir: Path) -> list[dict[str, str]]:
    path = cache_dir / "repo-list.json"
    if not path.is_file():
        raise FileNotFoundError(f"repo-list.json not found: {path}")

    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid JSON in repo-list.json: {exc}") from exc

    repos_in = raw.get("repos")
    if not isinstance(repos_in, list):
        raise ValueError("repo-list.json must have 'repos' array")

    out: list[dict[str, str]] = []
    for item in repos_in:
        if not isinstance(item, dict):
            continue
        if item.get("type") != KNOWLEDGE_CORPUS:
            continue
        full_name = str(item.get("full_name") or "").strip()
        if not full_name:
            continue
        description = str(item.get("description") or "").strip()
        repo_type = str(item.get("type") or "").strip()
        out.append(
            {
                "full_name": full_name,
                "description": description,
                "type": repo_type,
            }
        )
    return out


def should_skip(existing_dir: Path, force: bool) -> bool:
    if force:
        return False
    return existing_dir.is_dir()


def write_sediment_kb(categories: dict[str, Any], repos: dict[str, Any], sediment_kb_dir: Path) -> None:
    sediment_kb_dir.mkdir(parents=True, exist_ok=True)
    categories_path = sediment_kb_dir / "categories.json"
    repos_path = sediment_kb_dir / "repos.json"
    categories_path.write_text(
        json.dumps(categories, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    repos_path.write_text(
        json.dumps(repos, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def build_sediment_kb(entries: list[dict[str, str]]) -> tuple[dict[str, Any], dict[str, Any]]:
    type_ids: dict[str, str] = {}
    for entry in entries:
        repo_type = entry.get("type") or ""
        cat_id = type_to_category_id(repo_type or None)
        if cat_id == UNCATEGORIZED_ID:
            continue
        type_ids[cat_id] = type_to_category_name(repo_type or None)

    categories: list[dict[str, str]] = [
        {"id": UNCATEGORIZED_ID, "name": UNCATEGORIZED_NAME},
    ]
    for cat_id in sorted(type_ids):
        categories.append({"id": cat_id, "name": type_ids[cat_id]})

    repos_out: list[dict[str, str]] = []
    for entry in entries:
        repo_type = entry.get("type") or ""
        cat_id = type_to_category_id(repo_type or None)
        repos_out.append(
            {
                "full_name": entry["full_name"],
                "description": entry["description"],
                "category_id": cat_id,
            }
        )

    return (
        {"version": 1, "categories": categories},
        {"version": 1, "repos": repos_out},
    )


def _print_plan(entries: list[dict[str, str]], sediment_kb_dir: Path) -> None:
    categories, repos = build_sediment_kb(entries)
    print(f"Would write sediment-kb under {sediment_kb_dir}")
    for cat in categories["categories"]:
        print(f"category: {cat['id']} ({cat['name']})")
    if not entries:
        print("repos: (none)")
        return
    for entry, repo in zip(entries, repos["repos"]):
        print(f"repo: {entry['full_name']} → category_id={repo['category_id']}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=Path(__file__).resolve().parent.parent / ".cache",
        help="Cache directory containing repo-list.json",
    )
    parser.add_argument("--dry-run", action="store_true", help="Print migration plan without writing")
    parser.add_argument("--force", action="store_true", help="Overwrite existing sediment-kb/")
    args = parser.parse_args(argv)

    cache_dir = args.cache_dir.resolve()
    sediment_kb_dir = cache_dir / "sediment-kb"

    try:
        entries = load_repo_list_knowledge_corpus(cache_dir)
    except FileNotFoundError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    categories, repos = build_sediment_kb(entries)

    if should_skip(sediment_kb_dir, args.force):
        print(f"skip: sediment-kb already exists at {sediment_kb_dir}", file=sys.stderr)
        return 0

    if args.dry_run:
        _print_plan(entries, sediment_kb_dir)
        return 0

    write_sediment_kb(categories, repos, sediment_kb_dir)
    print(f"Wrote {len(entries)} repos → {sediment_kb_dir}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
