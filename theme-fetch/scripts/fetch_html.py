#!/usr/bin/env python3
"""Generic HTML file or URL → ArticleBundle JSON."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

try:
    from bs4 import BeautifulSoup, Tag
except ImportError:
    print("Missing dependency: pip3 install -r scripts/requirements.txt", file=sys.stderr)
    sys.exit(2)

from fetch_wechat import merge_blocks, now_iso, parse_meta, text_of, walk_content

UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36"
SELECTORS = ["#js_content", "article", "main", "[role=main]", ".article-content", ".post-content", "body"]


def load_html(path: str | None, url: str | None) -> tuple[str, str | None]:
    if path:
        return Path(path).read_text(encoding="utf-8", errors="replace"), None
    if url:
        r = subprocess.run(
            ["curl", "-sL", "-A", UA, "--max-time", "45", url],
            capture_output=True,
            text=True,
        )
        if r.returncode != 0:
            raise RuntimeError(f"curl failed: {r.stderr}")
        return r.text, url
    raise ValueError("Provide file path or --url")


def find_root(soup: BeautifulSoup, selector: str | None) -> Tag:
    if selector:
        el = soup.select_one(selector)
        if el:
            return el
    for sel in SELECTORS:
        el = soup.select_one(sel)
        if el and text_of(el):
            return el
    body = soup.body
    if not body:
        raise RuntimeError("No body in HTML")
    return body


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("file", nargs="?", help="Local HTML file")
    ap.add_argument("--url", help="Fetch URL")
    ap.add_argument("-o", "--output", required=True)
    ap.add_argument("--selector", help="CSS selector for main content")
    args = ap.parse_args()
    html, url = load_html(args.file, args.url)
    soup = BeautifulSoup(html, "lxml")
    root = find_root(soup, args.selector)
    meta = parse_meta(soup, html)
    if meta["title"] == "Untitled":
        h1 = root.find(["h1", "h2"])
        if h1:
            meta["title"] = text_of(h1)
    blocks: list[dict] = []
    tables: dict = {}
    walk_content(root, blocks, tables)
    blocks = merge_blocks(blocks)
    bundle = {
        "schema_version": 1,
        "source": {
            "platform": "plain-html",
            "url": url,
            "adapter": "plain-html@v1",
            "fetched_at": now_iso(),
        },
        "meta": meta,
        "content": {"blocks": blocks, "markdown_raw": None},
    }
    Path(args.output).write_text(json.dumps(bundle, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("OK", file=sys.stderr)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        raise SystemExit(1)
