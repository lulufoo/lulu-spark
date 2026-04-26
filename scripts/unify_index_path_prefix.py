#!/usr/bin/env python3
"""
将 index.json 每条目规范为 path_prefix* + 文件名：

- 同桶同目录：单一 path_prefix，各字段为文件名。
- 异桶/异目录：path_prefix_raw / path_prefix_distilled / path_prefix_digest / path_prefix_trace
  与对应短文件名；仅位于桶根的文件仍写整段相对路径，不写前缀键。

不修改 version；原地写回时请先备份。
"""
from __future__ import annotations

import json
from pathlib import Path, PurePosixPath

ARCHIVE = Path(__file__).resolve().parents[1]
INDEX_PATH = ARCHIVE / "index.json"
FIELDS = ("raw", "distilled", "digest", "trace")
PER_FIELD_PREFIX = tuple(f"path_prefix_{f}" for f in FIELDS)


def _is_prefix_key(k: str) -> bool:
    if k == "path_prefix":
        return True
    return k in PER_FIELD_PREFIX


def _meta(entry: dict) -> dict:
    return {k: v for k, v in entry.items() if k not in FIELDS and not _is_prefix_key(k)}


def _full_field(entry: dict, field: str) -> str | None:
    v = entry.get(field)
    if v is None or not isinstance(v, str):
        return v if v is None else None
    p1 = entry.get(f"path_prefix_{field}")
    if p1 and "/" not in v:
        return f"{p1}/{v}"
    p0 = entry.get("path_prefix")
    if p0 and field in FIELDS and "/" not in v:
        return f"{p0}/{v}"
    return v


def _parent_dir(relp: str) -> str:
    p = PurePosixPath(relp)
    s = p.parent.as_posix()
    return "" if s == "." else s


def normalize_entry(entry: dict) -> dict:
    full: dict[str, str | None] = {f: _full_field(entry, f) for f in FIELDS}
    active = [full[f] for f in FIELDS if full[f] is not None]
    if not active:
        out = _meta(entry)
        for f in FIELDS:
            if f in entry:
                out[f] = entry.get(f)
        return out
    parents = {_parent_dir(s) for s in active if isinstance(s, str)}

    if len(parents) == 1:
        pfx = parents.pop()
        out = _meta(entry)
        if pfx:
            out["path_prefix"] = pfx
        for f in FIELDS:
            r = full[f]
            if r is None:
                out[f] = None
            else:
                out[f] = PurePosixPath(r).name
        if not pfx:
            for f in FIELDS:
                if full[f] is not None:
                    out[f] = full[f]
        return out

    out = _meta(entry)
    for f in FIELDS:
        r = full[f]
        if r is None:
            out[f] = None
            continue
        d = _parent_dir(r)
        if d:
            out[f"path_prefix_{f}"] = d
            out[f] = PurePosixPath(r).name
        else:
            out[f] = r
    return out


def main() -> int:
    data = json.loads(INDEX_PATH.read_text(encoding="utf-8"))
    data["entries"] = {k: normalize_entry(v) for k, v in data["entries"].items()}
    INDEX_PATH.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"OK: {INDEX_PATH} ({len(data['entries'])} entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
