#!/usr/bin/env python3
"""Check a zh translation against the English primary.

Mirrors Host `translation_gate.rs`. Exit 0 when the pair may be archived.
Exit 1 and print the reason otherwise.

Usage:
    python3 check_zh_parity.py <english.md> <zh.md>
"""

from __future__ import annotations

import argparse
from pathlib import Path

STUB_MARKERS = (
    "SEE_FILE",
    "PLACEHOLDER",
    "FULL_ZH",
    "ZH_BODY",
    "SEE_FULL",
    "见文件",
)


def body_after_separator(document: str) -> str:
    for sep in ("\n---\n", "\n---\r\n"):
        idx = document.find(sep)
        if idx != -1:
            return document[idx + len(sep) :]
    return ""


def has_stub_marker(text: str) -> bool:
    upper = text.upper()
    for marker in STUB_MARKERS:
        if marker.isascii():
            if marker.upper() in upper:
                return True
        elif marker in text:
            return True
    return False


def count_headings(body: str) -> int:
    return sum(1 for line in body.splitlines() if line.lstrip().startswith("## "))


def is_turn_line(line: str) -> bool:
    trimmed = line.strip()
    if not trimmed or trimmed.startswith("#") or trimmed.startswith(">"):
        return False
    head = trimmed[:80]
    lower = head.lower()
    if lower.startswith("http://") or lower.startswith("https://"):
        return False
    return ":" in head or "：" in head


def count_turns(body: str) -> int:
    return sum(1 for line in body.splitlines() if is_turn_line(line))


def check_zh_parity(en_doc: str, zh_doc: str) -> str | None:
    if has_stub_marker(zh_doc):
        return "zh translation looks like a stub"
    en_body = body_after_separator(en_doc)
    zh_body = body_after_separator(zh_doc)
    if not zh_body.strip():
        return "zh translation body is empty"
    en_len = len(en_body.strip())
    zh_len = len(zh_body.strip())
    if zh_len == 0 or zh_len * 4 < en_len:
        return "zh translation is too short relative to the English body"
    en_headings = count_headings(en_body)
    zh_headings = count_headings(zh_body)
    if en_headings > 0 and zh_headings != en_headings:
        return f"zh heading count {zh_headings} != English {en_headings}"
    en_turns = count_turns(en_body)
    zh_turns = count_turns(zh_body)
    if en_turns > 0 and zh_turns * 5 < en_turns * 4:
        return f"zh turn count {zh_turns} below 80% of English {en_turns}"
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("english", type=Path)
    parser.add_argument("zh", type=Path)
    args = parser.parse_args()
    reason = check_zh_parity(
        args.english.read_text(encoding="utf-8"),
        args.zh.read_text(encoding="utf-8"),
    )
    if reason:
        print(reason)
        return 1
    print("zh_parity_ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
