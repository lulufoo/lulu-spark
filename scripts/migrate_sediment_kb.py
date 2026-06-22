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
        out.append({"full_name": full_name, "description": description})
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


def _default_categories() -> dict[str, Any]:
    return {
        "version": 1,
        "categories": [{"id": UNCATEGORIZED_ID, "name": UNCATEGORIZED_NAME}],
    }


def _repos_file(entries: list[dict[str, str]]) -> dict[str, Any]:
    return {
        "version": 1,
        "repos": [
            {
                "full_name": entry["full_name"],
                "description": entry["description"],
                "category_id": UNCATEGORIZED_ID,
            }
            for entry in entries
        ],
    }


def _print_plan(entries: list[dict[str, str]], sediment_kb_dir: Path) -> None:
    print(f"Would write sediment-kb under {sediment_kb_dir}")
    print(f"categories: uncategorized ({UNCATEGORIZED_NAME})")
    if not entries:
        print("repos: (none)")
        return
    for entry in entries:
        print(f"repo: {entry['full_name']} → category_id={UNCATEGORIZED_ID}")


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

    categories = _default_categories()
    repos = _repos_file(entries)

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
