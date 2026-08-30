#!/usr/bin/env python3
"""Mechanically slice a Cursor agent transcript JSONL into archive raw markdown.

Usage:
  python3 dialogue-archive/scripts/dialogue_archive_normalize.py \\
    --transcript ABS.jsonl --start-node N --end-node M \\
    --out ABS.md --title TITLE [--project P] [--doc-theme D] [--slug S]

Exit codes: 0 ok; 2 arg/range; 3 parse; 4 write.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

UTC8 = timezone(timedelta(hours=8))

RE_TIMESTAMP = re.compile(r"<timestamp>.*?</timestamp>\s*", re.DOTALL | re.IGNORECASE)
RE_USER_QUERY = re.compile(
    r"<user_query>\s*(.*?)\s*</user_query>", re.DOTALL | re.IGNORECASE
)
RE_ATTACHED_SKILLS = re.compile(
    r"<manually_attached_skills>.*?</manually_attached_skills>\s*",
    re.DOTALL | re.IGNORECASE,
)
RE_THINKING = re.compile(
    r"<(?:thinking|think|ai_thinking)>.*?</(?:thinking|think|ai_thinking)>\s*",
    re.DOTALL | re.IGNORECASE,
)


def die(code: int, msg: str) -> None:
    print(msg, file=sys.stderr)
    raise SystemExit(code)


def load_nodes(path: Path) -> list[dict]:
    nodes: list[dict] = []
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as e:
        die(3, f"failed to read transcript: {e}")
    for lineno, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError as e:
            die(3, f"json parse failed at physical line {lineno}: {e}")
        if not isinstance(obj, dict):
            die(3, f"expected object at non-empty line index {len(nodes) + 1}")
        nodes.append(obj)
    if not nodes:
        die(3, "transcript has no non-empty JSON lines")
    return nodes


def extract_text_blocks(node: dict) -> str:
    message = node.get("message") or {}
    content = message.get("content")
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return ""
    parts: list[str] = []
    for block in content:
        if not isinstance(block, dict):
            continue
        if block.get("type") == "text":
            t = block.get("text")
            if isinstance(t, str) and t.strip():
                parts.append(t)
    return "\n\n".join(parts)


def clean_text(text: str, *, role: str) -> str:
    text = RE_TIMESTAMP.sub("", text)
    text = RE_ATTACHED_SKILLS.sub("", text)
    text = RE_THINKING.sub("", text)
    if role == "user":
        m = RE_USER_QUERY.search(text)
        if m:
            text = m.group(1)
    return text.strip()


def pair_turns(slice_nodes: list[dict]) -> list[tuple[str, str]]:
    """Return list of (user_text, ai_text). Drop orphan leading assistants."""
    turns: list[tuple[str, str]] = []
    pending_user: str | None = None
    ai_parts: list[str] = []

    def flush() -> None:
        nonlocal pending_user, ai_parts
        if pending_user is None:
            return
        ai = "\n\n".join(p for p in ai_parts if p).strip() or "（无）"
        turns.append((pending_user, ai))
        pending_user = None
        ai_parts = []

    for node in slice_nodes:
        role = node.get("role")
        if role not in ("user", "assistant"):
            continue
        raw = extract_text_blocks(node)
        cleaned = clean_text(raw, role=role)
        if role == "user":
            if pending_user is not None:
                flush()
            pending_user = cleaned if cleaned else "（空）"
            ai_parts = []
        else:
            if pending_user is None:
                print(
                    "warning: dropping orphan assistant before first user in range",
                    file=sys.stderr,
                )
                continue
            if cleaned:
                ai_parts.append(cleaned)
    flush()
    return turns


def default_created_at() -> str:
    now = datetime.now(UTC8)
    # YYYY年M月D日 HH:MM — no zero-pad month/day per plan samples
    return f"{now.year}年{now.month}月{now.day}日 {now.hour:02d}:{now.minute:02d}"


def default_ts() -> str:
    return datetime.now(UTC8).strftime("%Y%m%d%H%M")


def derive_slug(out: Path, slug: str | None) -> str:
    if slug:
        return slug
    stem = out.stem
    # strip leading YYYYMMDDHHMM- if present
    m = re.match(r"^\d{12}-(.+)$", stem)
    return m.group(1) if m else stem


def render_markdown(
    *,
    title: str,
    created_at: str,
    start: int,
    end: int,
    transcript_name: str,
    turns: list[tuple[str, str]],
    local_md: bool,
) -> str:
    lines = [
        f"# {title}",
        "",
        f"> 创建时间：{created_at}",
        "> 来源：dialogue-archive",
        f"> 源节点：{start}-{end}（{transcript_name}）",
    ]
    if local_md:
        lines.append("> 落点：local-md")
    lines.extend(["", "---", ""])
    for i, (user, ai) in enumerate(turns, 1):
        lines.append("<!-- DDM:TURN_SEP:v1 -->")
        lines.append("")
        lines.append(f"## User（Turn {i}）")
        lines.append("")
        lines.append(user)
        lines.append("")
        lines.append("<!-- DDM:TURN_SEP:v1 -->")
        lines.append("")
        lines.append("## AI")
        lines.append("")
        lines.append(ai)
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--transcript", required=True, type=Path)
    p.add_argument("--start-node", required=True, type=int)
    p.add_argument("--end-node", required=True, type=int)
    p.add_argument("--out", required=True, type=Path)
    p.add_argument("--title", required=True)
    p.add_argument("--project", default="inbox")
    p.add_argument("--doc-theme", default="dialogue")
    p.add_argument("--slug", default=None)
    p.add_argument("--created-at", default=None)
    p.add_argument("--ts", default=None, help="YYYYMMDDHHMM staging filename hint")
    p.add_argument(
        "--local-md",
        action="store_true",
        help="sink=local-md: add 落点：local-md (never writes a digest nav line)",
    )
    p.add_argument("--dry-run", action="store_true")
    return p.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    transcript: Path = args.transcript.expanduser()
    out: Path = args.out.expanduser()
    start: int = args.start_node
    end: int = args.end_node

    if not transcript.is_file():
        die(2, f"transcript not found: {transcript}")
    if start < 1 or end < 1:
        die(2, "start-node and end-node must be >= 1")
    if start > end:
        die(2, f"invalid range: start-node {start} > end-node {end}")

    nodes = load_nodes(transcript)
    n = len(nodes)
    if end > n:
        die(2, f"end-node {end} > transcript length {n}")

    slice_nodes = nodes[start - 1 : end]
    chat_count = sum(1 for x in slice_nodes if x.get("role") in ("user", "assistant"))
    skipped = len(slice_nodes) - chat_count
    turns = pair_turns(slice_nodes)
    if not turns:
        die(2, "range produced zero User turns after filtering")

    created_at = args.created_at or default_created_at()
    ts = args.ts or default_ts()
    slug = derive_slug(out, args.slug)

    if args.dry_run:
        print(
            json.dumps(
                {
                    "transcript": str(transcript.resolve()),
                    "nodes_total": n,
                    "range": [start, end],
                    "chat_nodes": chat_count,
                    "skipped_non_chat": skipped,
                    "turns": len(turns),
                    "out": str(out.resolve()),
                    "staging": f"{args.project}/{args.doc_theme}/{ts}-{slug}.md",
                },
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0

    body = render_markdown(
        title=args.title,
        created_at=created_at,
        start=start,
        end=end,
        transcript_name=transcript.name,
        turns=turns,
        local_md=args.local_md,
    )
    try:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(body, encoding="utf-8")
    except OSError as e:
        die(4, f"failed to write out: {e}")

    print(
        json.dumps(
            {
                "turns": len(turns),
                "chat_nodes": chat_count,
                "skipped_non_chat": skipped,
                "bytes": len(body.encode("utf-8")),
            },
            ensure_ascii=False,
        ),
        file=sys.stderr,
    )
    print(f"OUT={out.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
