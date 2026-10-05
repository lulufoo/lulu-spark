#!/usr/bin/env python3
"""CLI: clean Cursor transcripts into clean-raw.json for dialogue-summary.

Usage:
  transcript-clean-control.py from-jsonl --session-id ID --jsonl PATH --out PATH [--title T]
  transcript-clean-control.py from-raw-md --session-id ID --raw PATH --out PATH [--title T]
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import re
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
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
