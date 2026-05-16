#!/usr/bin/env python3
"""One-shot: parse legacy ``knowledge-index.md`` fenced block → JSON."""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

DEFAULT_OWNER = "lulufoo"
_LINE_RE = re.compile(r"^([^:]+):\s*(.+?);\s*(https://\S+)\s*$")


def _parse_md(text: str) -> list[dict]:
    in_fence = False
    entries: list[dict] = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped == "```":
            in_fence = not in_fence
            continue
        if not in_fence or not stripped or stripped.startswith("##"):
            continue
        m = _LINE_RE.match(stripped)
        if not m:
            continue
        entry_id, desc, url = m.group(1).strip(), m.group(2).strip(), m.group(3).strip()
        entries.append({
            "id": entry_id,
            "repo": f"{DEFAULT_OWNER}/{entry_id}",
            "description": desc,
            "indexUrl": url,
        })
    return entries


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--in", dest="in_path", required=True, help="Source knowledge-index.md")
    p.add_argument("--out", dest="out_path", required=True, help="Output knowledge-index.json")
    args = p.parse_args()
    in_path = Path(args.in_path)
    out_path = Path(args.out_path)
    if not in_path.is_file():
        print(f"input not found: {in_path}", file=sys.stderr)
        return 1
    entries = _parse_md(in_path.read_text(encoding="utf-8"))
    if not entries:
        print("no entries parsed", file=sys.stderr)
        return 1
    from knowledge_index_loader import write_knowledge_index

    write_knowledge_index(entries, out_path)
    print(f"Wrote {len(entries)} entries → {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
