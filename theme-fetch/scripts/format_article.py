#!/usr/bin/env python3
"""ArticleBundle JSON → Markdown body (no archive header)."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


def table_md(rows: list[list[str]]) -> str:
    if not rows:
        return ""
    cols = max(len(r) for r in rows)
    rows = [r + [""] * (cols - len(r)) for r in rows]
    lines = [
        "| " + " | ".join(rows[0]) + " |",
        "| " + " | ".join(["---"] * cols) + " |",
    ]
    for row in rows[1:]:
        lines.append("| " + " | ".join(row) + " |")
    return "\n".join(lines)


def normalize_headings(blocks: list[dict]) -> list[dict]:
    out: list[dict] = []
    i = 0
    while i < len(blocks):
        b = blocks[i]
        t = (b.get("text") or "").strip()
        if b.get("type") == "heading" and re.fullmatch(r"\d{2}", t) and i + 1 < len(blocks):
            nxt = blocks[i + 1]
            nt = (nxt.get("text") or "").strip()
            if nxt.get("type") == "paragraph" and len(nt) < 60:
                out.append({"type": "heading", "level": 2, "text": f"{t} {nt}"})
                i += 2
                continue
        if b.get("type") == "paragraph":
            m = re.match(r"^(\d+\.\d+)\s*(.+)$", t)
            if m and len(m.group(2)) < 80:
                out.append({"type": "heading", "level": 3, "text": f"{m.group(1)} {m.group(2)}"})
                i += 1
                continue
            if t.endswith("节点") and len(t) < 35:
                out.append({"type": "heading", "level": 4, "text": t})
                i += 1
                continue
        out.append(b)
        i += 1
    return out


def flow_to_codeblock(text: str) -> str | None:
    if "↓" not in text and text.count("→") < 2:
        return None
    if len(text) < 40:
        return None
    parts = re.split(r"\s*(↓|→)\s*", text)
    if len(parts) < 3:
        return None
    lines = ["```text"]
    if parts[0].strip():
        lines.append(parts[0].strip())
    for k in range(1, len(parts), 2):
        if parts[k] == "↓":
            lines.append("  ↓")
        if k + 1 < len(parts) and parts[k + 1].strip():
            lines.append(parts[k + 1].strip())
    lines.append("```")
    return "\n".join(lines)


def render_block(b: dict) -> str:
    typ = b.get("type")
    if typ == "heading":
        level = min(max(b.get("level") or 2, 1), 4)
        return f"{'#' * level} {b.get('text', '')}"
    if typ == "paragraph":
        text = b.get("text") or ""
        flow = flow_to_codeblock(text)
        if flow:
            return flow
        if "经典软件工程" in text and "AI 软件工程" in text:
            return "- **经典软件工程**：人在写代码\n- **AI 软件工程**：人在设计\"AI 写代码的系统\""
        return text
    if typ == "table":
        return table_md(b.get("rows") or [])
    if typ == "list":
        items = b.get("items") or []
        if b.get("ordered"):
            return "\n".join(f"{i+1}. {x}" for i, x in enumerate(items))
        return "\n".join(f"- {x}" for x in items)
    if typ == "code":
        lang = b.get("lang") or "text"
        return f"```{lang}\n{b.get('text','')}\n```"
    if typ == "image":
        return f"![image]({b.get('url')})"
    if typ == "quote":
        return f"> {b.get('text','')}"
    return b.get("text") or ""


def bundle_to_markdown(bundle: dict, normalize: bool) -> str:
    content = bundle.get("content") or {}
    raw = content.get("markdown_raw")
    if raw:
        return re.sub(r"\n{3,}", "\n\n", raw.strip())
    blocks = content.get("blocks") or []
    if normalize:
        blocks = normalize_headings(blocks)
    parts = []
    for b in blocks:
        piece = render_block(b).strip()
        if piece:
            parts.append(piece)
    md = "\n\n".join(parts)
    return re.sub(r"\n{3,}", "\n\n", md)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("bundle", help="ArticleBundle JSON file or '-' for stdin")
    ap.add_argument("--normalize-headings", action="store_true", default=True)
    ap.add_argument("--no-normalize-headings", action="store_false", dest="normalize_headings")
    args = ap.parse_args()
    if args.bundle == "-":
        data = json.load(sys.stdin)
    else:
        data = json.loads(Path(args.bundle).read_text(encoding="utf-8"))
    print(bundle_to_markdown(data, args.normalize_headings))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
