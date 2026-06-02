#!/usr/bin/env python3
"""Fetch WeChat article URL → ArticleBundle JSON."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import datetime, timezone, timedelta
from html import unescape
from pathlib import Path
from typing import Any

try:
    from bs4 import BeautifulSoup, NavigableString, Tag
except ImportError:
    print("Missing dependency: pip3 install -r scripts/requirements.txt", file=sys.stderr)
    sys.exit(2)

TZ = timezone(timedelta(hours=8))
UA = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
)
PROMO_PATTERNS = ("关注", "一手技术干货", "解锁👇")


def now_iso() -> str:
    return datetime.now(TZ).isoformat(timespec="seconds")


def fetch_html(url: str, timeout: int = 45) -> str:
    r = subprocess.run(
        ["curl", "-sL", "-A", UA, "--max-time", str(timeout), url],
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        raise RuntimeError(f"curl failed (exit {r.returncode}): {r.stderr.strip()}")
    html = r.text
    if "环境异常" in html and "js_content" not in html:
        raise RuntimeError("WeChat verification page — save HTML in browser and use plain-html adapter")
    return html


def text_of(el) -> str:
    return re.sub(r"\s+", " ", el.get_text(separator=" ", strip=True))


def table_rows(table: Tag) -> list[list[str]]:
    rows = []
    for tr in table.find_all("tr"):
        cells = tr.find_all(["td", "th"])
        if cells:
            rows.append([text_of(c) for c in cells])
    return rows


def parse_meta(soup: BeautifulSoup, html: str) -> dict[str, Any]:
    title = None
    og = soup.find("meta", property="og:title")
    if og and og.get("content"):
        title = og["content"].strip()
    if not title:
        m = re.search(r'var msg_title\s*=\s*"([^"]*)"', html)
        if m:
            title = unescape(m.group(1))
    author = None
    m = re.search(r'"author"\s*:\s*"([^"]+)"', html)
    if m:
        author = m.group(1).strip()
    publisher = None
    m = re.search(r'var nickname\s*=\s*htmlDecode\("([^"]*)"\)', html)
    if m:
        publisher = unescape(m.group(1)).strip()
    if not publisher:
        name = soup.find(id="js_name")
        if name:
            publisher = text_of(name)
    published_at = None
    m = re.search(r"create_time:\s*'([^']+)'", html)
    if m:
        published_at = m.group(1)
    else:
        pt = soup.find(id="publish_time")
        if pt:
            published_at = text_of(pt)
    return {
        "title": title or "Untitled",
        "author": author,
        "publisher": publisher,
        "published_at": published_at,
        "language": "zh",
    }


def is_promo(text: str) -> bool:
    t = text.strip()
    if len(t) > 80:
        return False
    return any(p in t for p in PROMO_PATTERNS) and "关注" in t


def walk_content(node, blocks: list[dict], tables: dict[str, list[list[str]]]) -> None:
    if isinstance(node, NavigableString):
        t = str(node).strip()
        if t:
            blocks.append({"type": "paragraph", "text": t})
        return
    if not isinstance(node, Tag):
        return
    if node.name == "table":
        rows = table_rows(node)
        if rows:
            blocks.append({"type": "table", "rows": rows})
        return
    if node.name == "img":
        src = node.get("data-src") or node.get("src")
        if src and not src.startswith("data:"):
            blocks.append({"type": "image", "url": src, "text": None})
        return
    if node.name in ("script", "style"):
        return
    if node.name in ("strong", "b"):
        t = text_of(node)
        if re.fullmatch(r"\d{2}", t):
            blocks.append({"type": "heading", "level": 2, "text": t, "_chapter_num": True})
            return
        if re.match(r"\d+\.\d+", t):
            blocks.append({"type": "heading", "level": 3, "text": t})
            return
    if node.name in ("h1", "h2", "h3", "h4"):
        level = int(node.name[1])
        blocks.append({"type": "heading", "level": level, "text": text_of(node)})
        return
    if node.name == "br":
        return
    for child in node.children:
        walk_content(child, blocks, tables)


def merge_blocks(blocks: list[dict]) -> list[dict]:
    out: list[dict] = []
    i = 0
    while i < len(blocks):
        b = blocks[i]
        if b.get("type") == "paragraph" and is_promo(b.get("text", "")):
            i += 1
            continue
        if b.get("_chapter_num") and i + 1 < len(blocks):
            nxt = blocks[i + 1]
            if nxt.get("type") in ("paragraph", "heading") and nxt.get("text"):
                num = b["text"]
                title = nxt["text"]
                if len(title) < 80 and not title.endswith("。"):
                    out.append({"type": "heading", "level": 2, "text": f"{num} {title}"})
                    i += 2
                    continue
        if b.get("type") == "paragraph" and out and out[-1].get("type") == "paragraph":
            prev = out[-1]["text"]
            cur = b["text"]
            if len(prev) < 30 and not re.search(r"[。！？]$", prev):
                out[-1]["text"] = prev + cur
                i += 1
                continue
        clean = {k: v for k, v in b.items() if not k.startswith("_")}
        out.append(clean)
        i += 1
    return out


def parse_article(html: str, url: str) -> dict:
    soup = BeautifulSoup(html, "lxml")
    content = soup.find(id="js_content")
    if not content:
        raise RuntimeError("Cannot find #js_content in HTML")
    meta = parse_meta(soup, html)
    blocks: list[dict] = []
    tables: dict = {}
    walk_content(content, blocks, tables)
    blocks = merge_blocks(blocks)
    blocks = [b for b in blocks if b.get("text") or b.get("rows") or b.get("url")]
    return {
        "schema_version": 1,
        "source": {
            "platform": "wechat",
            "url": url,
            "adapter": "wechat@v1",
            "fetched_at": now_iso(),
        },
        "meta": meta,
        "content": {"blocks": blocks, "markdown_raw": None},
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="Fetch WeChat article → ArticleBundle JSON")
    ap.add_argument("url", help="WeChat article URL")
    ap.add_argument("-o", "--output", help="Write bundle JSON to file")
    ap.add_argument("--save-html", help="Save raw HTML for debugging")
    args = ap.parse_args()
    if "mp.weixin.qq.com" not in args.url:
        print("Warning: URL does not look like WeChat", file=sys.stderr)
    html = fetch_html(args.url)
    if args.save_html:
        Path(args.save_html).write_text(html, encoding="utf-8")
    bundle = parse_article(html, args.url)
    text = json.dumps(bundle, ensure_ascii=False, indent=2)
    if args.output:
        Path(args.output).write_text(text + "\n", encoding="utf-8")
    else:
        print(text)
    print("OK", file=sys.stderr)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        raise SystemExit(1)
