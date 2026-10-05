#!/usr/bin/env python3
"""Check composed dialogue coverage against a TranscriptBundle.

Every non-empty source utterance word must appear in the composed body after
speaker prefixes and headings are stripped. Default threshold is 98% to allow
legitimate consecutive-duplicate drops.

Usage:
    python3 check_dialogue_coverage.py <composed.md> <bundle.json>
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path

WORD = re.compile(r"[A-Za-z0-9']+")
SPEAKER_PREFIX = re.compile(r"^[^:#]{1,80}[:：]\s*")


def body_after_separator(document: str) -> str:
    for sep in ("\n---\n", "\n---\r\n"):
        idx = document.find(sep)
        if idx != -1:
            return document[idx + len(sep) :]
    return ""


def words(text: str) -> list[str]:
    return [m.group(0).lower() for m in WORD.finditer(text)]


def composed_words(document: str) -> list[str]:
    out: list[str] = []
    for line in body_after_separator(document).splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("## ") or stripped.startswith(">"):
            continue
        out.extend(words(SPEAKER_PREFIX.sub("", stripped)))
    return out


def utterance_words(bundle: dict) -> list[str]:
    out: list[str] = []
    for item in bundle.get("utterances") or []:
        text = (item or {}).get("text") or ""
        out.extend(words(text))
    return out


def coverage(source: list[str], composed: list[str]) -> float:
    if not source:
        return 0.0
    src = Counter(source)
    dst = Counter(composed)
    missing = 0
    for token, n in src.items():
        if dst[token] < n:
            missing += n - dst[token]
    return 1.0 - (missing / sum(src.values()))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("composed", type=Path)
    parser.add_argument("bundle", type=Path)
    parser.add_argument(
        "--min",
        type=float,
        default=0.98,
        help="Minimum coverage ratio (default 0.98)",
    )
    args = parser.parse_args()
    bundle = json.loads(args.bundle.read_text(encoding="utf-8"))
    src = utterance_words(bundle)
    if not src:
        print("bundle has no utterance words")
        return 1
    ratio = coverage(src, composed_words(args.composed.read_text(encoding="utf-8")))
    print(f"coverage {ratio:.4f} (min {args.min:.2f})")
    if ratio + 1e-9 < args.min:
        print("composed dialogue coverage below threshold")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
