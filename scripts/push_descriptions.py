#!/usr/bin/env python3
"""Phase B: Push description+keywords to each repo's .repository-type.json.

Reads .cache/repo-descriptions.json, for each repo:
  1. Fetches current .repository-type.json + SHA via gh api
  2. Merges description + keywords fields
  3. PUTs the updated file back

Run from the lulu-workbench root:
    python3 scripts/push_descriptions.py [--dry-run]
"""
from __future__ import annotations

import argparse
import base64
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DESCRIPTIONS_PATH = REPO_ROOT / ".cache" / "repo-descriptions.json"


def _gh_get(endpoint: str) -> dict:
    r = subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github+json", endpoint],
        capture_output=True, text=True, check=False
    )
    if r.returncode != 0:
        raise RuntimeError(f"gh api GET {endpoint} failed: {r.stderr.strip()}")
    return json.loads(r.stdout)


def _gh_put(endpoint: str, payload: dict) -> dict:
    r = subprocess.run(
        ["gh", "api", "-X", "PUT", endpoint,
         "--input", "-",
         "-H", "Accept: application/vnd.github+json"],
        input=json.dumps(payload),
        capture_output=True, text=True, check=False
    )
    if r.returncode != 0:
        raise RuntimeError(f"gh api PUT {endpoint} failed: {r.stderr.strip()}")
    return json.loads(r.stdout)


def push_repo(repo: str, description: str, keywords: list[str], dry_run: bool) -> None:
    owner, name = repo.split("/", 1)
    endpoint = f"repos/{owner}/{name}/contents/.repository-type.json"

    # Read current file
    meta = _gh_get(endpoint)
    sha = meta["sha"]
    current_content = base64.b64decode(meta["content"].replace("\n", "")).decode("utf-8")
    doc = json.loads(current_content)

    # Merge fields
    doc["description"] = description
    doc["keywords"] = keywords

    new_content = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    encoded = base64.b64encode(new_content.encode("utf-8")).decode("ascii")

    if dry_run:
        print(f"[dry-run] {repo}: would write → {json.dumps(doc, ensure_ascii=False)}")
        return

    payload = {
        "message": "chore: add description+keywords to .repository-type.json",
        "content": encoded,
        "sha": sha,
    }
    _gh_put(endpoint, payload)
    print(f"[ok] {repo}: pushed description+keywords")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true", help="Print what would be done, do not push")
    args = parser.parse_args()

    if not DESCRIPTIONS_PATH.exists():
        print(f"Error: {DESCRIPTIONS_PATH} not found. Run Phase A first.", file=sys.stderr)
        return 1

    data = json.loads(DESCRIPTIONS_PATH.read_text(encoding="utf-8"))
    repos = data.get("repos", [])

    if not repos:
        print("No repos in descriptions file.", file=sys.stderr)
        return 1

    errors = []
    for item in repos:
        repo = item["repo"]
        description = item.get("description", "")
        keywords = item.get("keywords", [])
        try:
            push_repo(repo, description, keywords, dry_run=args.dry_run)
        except Exception as e:
            print(f"[error] {repo}: {e}", file=sys.stderr)
            errors.append(repo)

    if errors:
        print(f"\n{len(errors)} repo(s) failed: {', '.join(errors)}", file=sys.stderr)
        return 2

    if args.dry_run:
        print(f"\nDry run complete. {len(repos)} repo(s) would be updated.")
    else:
        print(f"\nDone. {len(repos)} repo(s) updated.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
