#!/usr/bin/env python3
"""Detect whether archive markdown body (after ---) is full English.

Exit 0 and print `full_english` when the body has Latin letters and no CJK.
Exit 1 and print `not_full_english` otherwise.

Header lines before --- are ignored (they are Chinese archive chrome).
Known body chrome such as `作者 | …` and HTML comments is ignored too.
Any remaining Han / Kana / Hangul means mixed content — do not translate.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

CJK = re.compile(
    r"[\u3400-\u4DBF\u4E00-\u9FFF\uF900-\uFAFF"
    r"\u3040-\u30FF"
    r"\uAC00-\uD7AF]"
)
LATIN = re.compile(r"[A-Za-z]")
CHROME_LINE = re.compile(
    r"^(作者\s*\|.*|<!--.*?-->)\s*$",
    re.UNICODE,
)


def body_after_separator(document: str) -> str:
    for sep in ("\n---\n", "\n---\r\n"):
        idx = document.find(sep)
        if idx != -1:
            return document[idx + len(sep) :]
    return ""


def content_for_detect(document: str) -> str:
    lines = []
    for line in body_after_separator(document).splitlines():
        if CHROME_LINE.match(line.strip()):
            continue
        lines.append(line)
    return "\n".join(lines)


def is_full_english(document: str) -> bool:
    body = content_for_detect(document)
    if not body.strip():
        return False
    if CJK.search(body):
        return False
    return LATIN.search(body) is not None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path, help="Primary archive markdown")
    args = parser.parse_args()
    text = args.path.read_text(encoding="utf-8")
    if is_full_english(text):
        print("full_english")
        return 0
    print("not_full_english")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
