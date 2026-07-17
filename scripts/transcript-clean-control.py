#!/usr/bin/env python3
"""CLI: clean Cursor transcripts into clean-raw.json; render archive markdown.

SSOT for lulu-workbench-skills (shared by dialogue-summary / dialogue-archive).

Usage:
  transcript-clean-control.py from-jsonl --session-id ID --jsonl PATH --out PATH [--title T]
  transcript-clean-control.py from-raw-md --session-id ID --raw PATH --out PATH [--title T]
  transcript-clean-control.py to-archive-md --clean-raw PATH --out PATH \\
      --title T --project P --doc-theme D --slug S --ts YYYYMMDDHHMM \\
      [--source-label dialogue-archive] [--ts-display '...']
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path


def _load_schema():
    path = Path(__file__).resolve().parent / "transcript-clean-schema.py"
    spec = importlib.util.spec_from_file_location("transcript_clean_schema", path)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


schema = _load_schema()


def _slice_turns(
    paired: list[dict], turn_from: int | None, turn_to: int | None
) -> list[dict]:
    """Filter compact turns by original n (inclusive), then renumber 1..N."""
    if turn_from is None and turn_to is None:
        return paired
    lo = turn_from if turn_from is not None else 1
    hi = turn_to if turn_to is not None else 10**9
    sliced = [t for t in paired if lo <= int(t.get("n") or 0) <= hi]
    return schema.renumber_compact_turns(sliced)


def _write_clean(
    session_id: str,
    flat: list[dict],
    *,
    source: str,
    out: Path,
    title: str | None,
    turn_from: int | None = None,
    turn_to: int | None = None,
) -> int:
    paired = schema.pair_flat_turns(flat)
    paired = _slice_turns(paired, turn_from, turn_to)
    doc = schema.build_clean_raw_doc(
        session_id, paired, source=source, title=title
    )
    schema.write_clean_raw_json(out, doc)
    bad = schema.assert_no_chrome_doc(doc)
    print(
        json.dumps(
            {
                "written": str(out.resolve()),
                "user_turns": len(doc.get("turns") or []),
                "turn_from": turn_from,
                "turn_to": turn_to,
                "chrome_tags_remaining": bad,
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    if bad:
        return 1
    if not (doc.get("turns") or []):
        return 1
    return 0


def cmd_from_jsonl(
    session_id: str,
    jsonl: Path,
    out: Path,
    title: str | None,
    turn_from: int | None,
    turn_to: int | None,
) -> int:
    flat = schema.turns_from_jsonl(jsonl)
    return _write_clean(
        session_id,
        flat,
        source=str(jsonl.resolve()),
        out=out,
        title=title,
        turn_from=turn_from,
        turn_to=turn_to,
    )


def _parse_messy_raw_md(path: Path) -> list[dict]:
    """Parse prior messy ## User/## AI or **User** raw into turns, then clean user blobs."""
    text = path.read_text(encoding="utf-8")
    blocks: list[dict] = []
    cur = None
    buf: list[str] = []
    header_re = re.compile(r"^(?:## |\*\*)(User（Turn (\d+)）|AI)(?:\*\*)?\s*$")
    for line in text.splitlines():
        m = header_re.match(line.strip())
        if m:
            if cur is not None:
                blocks.append({**cur, "raw": "\n".join(buf).strip()})
            kind = m.group(1)
            if kind.startswith("User"):
                cur = {"role": "user", "turn": int(m.group(2))}
            else:
                cur = {"role": "assistant", "turn": None}
            buf = []
        else:
            if cur is not None:
                buf.append(line)
    if cur is not None:
        blocks.append({**cur, "raw": "\n".join(buf).strip()})

    turns: list[dict] = []
    for b in blocks:
        if b["role"] == "user":
            cleaned = schema.clean_user_text(b["raw"])
            if not cleaned["text"]:
                continue
            turns.append(
                {
                    "role": "user",
                    "turn": b["turn"],
                    "text": cleaned["text"],
                    "timestamp": cleaned["timestamp"],
                }
            )
        else:
            t = schema.clean_assistant_text(b["raw"])
            if t:
                turns.append({"role": "assistant", "turn": None, "text": t})
    return turns


def cmd_from_raw_md(
    session_id: str,
    raw: Path,
    out: Path,
    title: str | None,
    turn_from: int | None,
    turn_to: int | None,
) -> int:
    flat = _parse_messy_raw_md(raw)
    return _write_clean(
        session_id,
        flat,
        source=str(raw.resolve()),
        out=out,
        title=title,
        turn_from=turn_from,
        turn_to=turn_to,
    )


_TS_RE = re.compile(r"^\d{12}$")


def _ts_display(ts: str, override: str | None) -> str:
    if override:
        return override
    if not _TS_RE.match(ts):
        raise SystemExit(f"invalid --ts {ts!r}; expected YYYYMMDDHHMM")
    y, mo, d = int(ts[0:4]), int(ts[4:6]), int(ts[6:8])
    hh, mm = ts[8:10], ts[10:12]
    return f"{y}年{mo}月{d}日 {hh}:{mm}"


def render_archive_markdown(
    doc: dict,
    *,
    title: str,
    project: str,
    doc_theme: str,
    slug: str,
    ts: str,
    source_label: str,
    ts_display: str | None,
) -> str:
    """Render clean-raw turns into dialogue-archive Core Output Shape (verbatim u/a)."""
    if not _TS_RE.match(ts):
        raise SystemExit(f"invalid --ts {ts!r}; expected YYYYMMDDHHMM")
    common = f"{project}/{doc_theme}/{ts}-{slug}.md"
    disp = _ts_display(ts, ts_display)
    turns = doc.get("turns") or []
    parts: list[str] = [
        f"# {title}",
        "",
        f"> 创建时间：{disp}",
        f"> 来源：{source_label}",
        f"> 导航：[digest](../../../digest/{common})",
        "",
        "---",
        "",
    ]
    for t in turns:
        n = int(t.get("n") or 0)
        u = (t.get("u") or "").rstrip()
        a = (t.get("a") or "").rstrip()
        parts += [
            "<!-- DDM:TURN_SEP:v1 -->",
            "",
            f"## User（Turn {n}）",
            "",
            u,
            "",
            "<!-- DDM:TURN_SEP:v1 -->",
            "",
            "## AI",
            "",
            a,
            "",
        ]
    return "\n".join(parts).rstrip() + "\n"


def cmd_to_archive_md(
    clean_raw: Path,
    out: Path,
    *,
    title: str,
    project: str,
    doc_theme: str,
    slug: str,
    ts: str,
    source_label: str,
    ts_display: str | None,
    omit_empty_ai: bool,
) -> int:
    doc = json.loads(clean_raw.read_text(encoding="utf-8"))
    turns = list(doc.get("turns") or [])
    if omit_empty_ai:
        turns = [t for t in turns if str(t.get("a") or "").strip()]
        doc = {**doc, "turns": turns}
    if not turns:
        print(
            json.dumps(
                {"error": "clean-raw has zero turns", "clean_raw": str(clean_raw)},
                ensure_ascii=False,
            ),
            file=sys.stderr,
        )
        return 1
    bad = schema.assert_no_chrome_doc(doc)
    if bad:
        print(
            json.dumps(
                {"error": "chrome_tags_remaining", "detail": bad},
                ensure_ascii=False,
            ),
            file=sys.stderr,
        )
        return 1
    md = render_archive_markdown(
        doc,
        title=title,
        project=project,
        doc_theme=doc_theme,
        slug=slug,
        ts=ts,
        source_label=source_label,
        ts_display=ts_display,
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(md, encoding="utf-8")
    user_headers = md.count("## User（Turn")
    common = f"{project}/{doc_theme}/{ts}-{slug}.md"
    print(
        json.dumps(
            {
                "written": str(out.resolve()),
                "common_path": common,
                "user_turns": len(turns),
                "user_headers": user_headers,
                "sep_ok": "<!-- DDM:TURN_SEP:v1 -->" in md,
                "match": user_headers == len(turns),
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0 if user_headers == len(turns) else 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p1 = sub.add_parser("from-jsonl")
    p1.add_argument("--session-id", required=True)
    p1.add_argument("--jsonl", required=True)
    p1.add_argument("--out", required=True)
    p1.add_argument("--title")
    p1.add_argument("--turn-from", type=int, help="Inclusive start turn n")
    p1.add_argument("--turn-to", type=int, help="Inclusive end turn n")

    p2 = sub.add_parser("from-raw-md")
    p2.add_argument("--session-id", required=True)
    p2.add_argument("--raw", required=True)
    p2.add_argument("--out", required=True)
    p2.add_argument("--title")
    p2.add_argument("--turn-from", type=int, help="Inclusive start turn n")
    p2.add_argument("--turn-to", type=int, help="Inclusive end turn n")

    p3 = sub.add_parser("to-archive-md")
    p3.add_argument("--clean-raw", required=True)
    p3.add_argument("--out", required=True)
    p3.add_argument("--title", required=True)
    p3.add_argument("--project", required=True)
    p3.add_argument("--doc-theme", required=True)
    p3.add_argument("--slug", required=True)
    p3.add_argument("--ts", required=True, help="YYYYMMDDHHMM UTC+8")
    p3.add_argument("--source-label", default="dialogue-archive")
    p3.add_argument("--ts-display", help="Override 创建时间 display string")
    p3.add_argument(
        "--omit-empty-ai",
        action="store_true",
        help="Skip turns whose assistant body is empty (e.g. in-progress last turn)",
    )

    args = ap.parse_args()
    if args.cmd == "from-jsonl":
        return cmd_from_jsonl(
            args.session_id,
            Path(args.jsonl),
            Path(args.out),
            args.title,
            args.turn_from,
            args.turn_to,
        )
    if args.cmd == "from-raw-md":
        return cmd_from_raw_md(
            args.session_id,
            Path(args.raw),
            Path(args.out),
            args.title,
            args.turn_from,
            args.turn_to,
        )
    if args.cmd == "to-archive-md":
        return cmd_to_archive_md(
            Path(args.clean_raw),
            Path(args.out),
            title=args.title,
            project=args.project,
            doc_theme=args.doc_theme,
            slug=args.slug,
            ts=args.ts,
            source_label=args.source_label,
            ts_display=args.ts_display,
            omit_empty_ai=bool(args.omit_empty_ai),
        )
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
