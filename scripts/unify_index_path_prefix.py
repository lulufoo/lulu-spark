#!/usr/bin/env python3
"""
将 index.json 中每条能合并为同一路径前缀的条目，统一为：
  "path_prefix": "<topic path under each bucket>"
  以及 raw / distilled / digest / trace 上「仅」文件名

若同一条中 raw 与 distilled（及 digest/trace）所在目录不一致（异树），
则不设 path_prefix，各字段仍写桶内**完整**相对路径（与 norm 一致）。

不修改 version；原地写回时请先备份。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path, PurePosixPath

ARCHIVE = Path(__file__).resolve().parents[1]
INDEX_PATH = ARCHIVE / "index.json"
FIELDS = ("raw", "distilled", "digest", "trace")


def _full_field(entry: dict, field: str) -> str | None:
    v = entry.get(field)
    if v is None:
        return None
    if not isinstance(v, str):
        return None
    pfx = entry.get("path_prefix")
    if pfx and field in FIELDS and "/" not in v:
        return f"{pfx}/{v}"
    return v


def _parent_dir(relp: str) -> str:
    p = PurePosixPath(relp)
    s = p.parent.as_posix()
    return "" if s == "." else s


def normalize_entry(entry: dict) -> dict:
    meta = {k: v for k, v in entry.items() if k not in FIELDS and k != "path_prefix"}
    full: dict[str, str | None] = {f: _full_field(entry, f) for f in FIELDS}
    active = [full[f] for f in FIELDS if full[f] is not None]
    if not active:
        out = {**meta}
        for f in FIELDS:
            if f in entry:
                out[f] = entry.get(f)
        return out
    parents = {_parent_dir(s) for s in active if isinstance(s, str)}
    if len(parents) == 1:
        pfx = parents.pop()
        out = {**meta}
        if pfx:
            out["path_prefix"] = pfx
        for f in FIELDS:
            r = full[f]
            if r is None:
                out[f] = None
            else:
                out[f] = PurePosixPath(r).name
        return out
    out = {**meta}
    for f in FIELDS:
        out[f] = full[f]
    return out


def main() -> int:
    data = json.loads(INDEX_PATH.read_text(encoding="utf-8"))
    entries = {k: normalize_entry(v) for k, v in data["entries"].items()}
    data["entries"] = entries
    INDEX_PATH.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"OK: {INDEX_PATH} ({len(entries)} entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
