#!/usr/bin/env python3
"""Semi-automatic WeChat acquire: open system browser, pick saved HTML from Downloads.

Flow:
  1. Open URL in the user's default browser (macOS `open`, Linux `xdg-open`).
  2. Optional: --try-playwright — headed browser, auto-save when #js_content appears.
  3. Otherwise: user Cmd+S / Save Page → script watches --downloads for new .html/.htm.
  4. Copy stable file to --output (for fetch_html.py / plain-html adapter).

Usage:
  python3 acquire_wechat_browser.py "https://mp.weixin.qq.com/s/..." -o /tmp/wechat.html
  python3 acquire_wechat_browser.py --html ~/Downloads/article.html -o /tmp/wechat.html
"""

from __future__ import annotations

import argparse
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

MIN_HTML_BYTES = 8_000
STABLE_POLLS = 2
POLL_INTERVAL = 1.5


def default_downloads() -> Path:
    home = Path.home()
    for candidate in (home / "Downloads", home / "下载"):
        if candidate.is_dir():
            return candidate
    return home / "Downloads"


def open_in_browser(url: str) -> None:
    system = platform.system()
    if system == "Darwin":
        subprocess.run(["open", url], check=True)
    elif system == "Linux":
        subprocess.run(["xdg-open", url], check=True)
    else:
        print(f"请手动在浏览器打开: {url}", file=sys.stderr)


def html_has_wechat_content(path: Path) -> bool:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return False
    if "环境异常" in text and "js_content" not in text:
        return False
    return "js_content" in text or "rich_media_content" in text


def try_playwright_save(url: str, out_path: Path, timeout_sec: int) -> bool:
    try:
        from playwright.sync_api import sync_playwright
    except ImportError:
        print(
            "playwright 未安装，跳过自动保存（pip install playwright && playwright install chromium）",
            file=sys.stderr,
        )
        return False

    print("启动 Playwright 有头浏览器；若出现验证请在窗口内完成…", file=sys.stderr)
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(headless=False)
            page = browser.new_page()
            page.goto(url, wait_until="domcontentloaded", timeout=min(timeout_sec, 120) * 1000)
            page.wait_for_selector("#js_content", timeout=timeout_sec * 1000)
            html = page.content()
            browser.close()
    except Exception as e:
        print(f"Playwright 自动保存失败: {e}", file=sys.stderr)
        return False

    if "js_content" not in html:
        return False
    out_path.write_text(html, encoding="utf-8")
    print(f"Playwright 已保存 → {out_path}", file=sys.stderr)
    return True


def list_recent_html(downloads: Path, since: float) -> list[tuple[float, int, Path]]:
    rows: list[tuple[float, int, Path]] = []
    for pattern in ("*.html", "*.htm", "*.HTML", "*.HTM"):
        for path in downloads.glob(pattern):
            if not path.is_file():
                continue
            try:
                st = path.stat()
            except OSError:
                continue
            if st.st_mtime >= since - 2:
                rows.append((st.st_mtime, st.st_size, path))
    rows.sort(reverse=True)
    return rows


def wait_for_download_html(downloads: Path, since: float, timeout_sec: int) -> Path:
    print(f"监听下载目录: {downloads}", file=sys.stderr)
    print(
        "请在已打开的浏览器中：文章加载完成后 ⌘S / Ctrl+S →「网页，仅 HTML」→ 保存到下载文件夹",
        file=sys.stderr,
    )

    deadline = time.time() + timeout_sec
    last_pick: Path | None = None
    last_size = -1
    stable = 0

    while time.time() < deadline:
        for _mtime, size, path in list_recent_html(downloads, since):
            if size < MIN_HTML_BYTES:
                continue
            if path == last_pick and size == last_size:
                stable += 1
            else:
                last_pick = path
                last_size = size
                stable = 0
            if stable >= STABLE_POLLS and html_has_wechat_content(path):
                return path
        time.sleep(POLL_INTERVAL)

    raise TimeoutError(
        f"{timeout_sec}s 内未在 {downloads} 发现新的微信文章 HTML；"
        "请确认已另存为 HTML，或使用 --html 指定文件路径"
    )


def main() -> int:
    ap = argparse.ArgumentParser(description="Semi-auto WeChat: open browser + pick saved HTML")
    ap.add_argument("url", nargs="?", help="WeChat article URL")
    ap.add_argument("-o", "--output", required=True, help="Copy/save HTML here")
    ap.add_argument("--downloads", type=Path, default=None, help="Downloads folder (default: ~/Downloads)")
    ap.add_argument("--html", type=Path, help="Skip wait; use this saved HTML file")
    ap.add_argument("--timeout", type=int, default=600, help="Wait for manual save (seconds)")
    ap.add_argument("--try-playwright", action="store_true", help="Try headed Playwright auto-save first")
    ap.add_argument("--no-open", action="store_true", help="Do not open browser (only watch downloads)")
    args = ap.parse_args()

    out_path = Path(args.output)
    downloads = args.downloads or default_downloads()

    if args.html:
        src = args.html.expanduser().resolve()
        if not src.is_file():
            print(f"ERROR: file not found: {src}", file=sys.stderr)
            return 1
        if not html_has_wechat_content(src):
            print("WARNING: HTML 中未检测到 js_content，仍继续复制", file=sys.stderr)
        shutil.copy2(src, out_path)
        print(f"OK copied {src} → {out_path}", file=sys.stderr)
        print(out_path)
        return 0

    if not args.url or "mp.weixin.qq.com" not in args.url:
        print("ERROR: 需要微信文章 URL，或改用 --html", file=sys.stderr)
        return 1

    since = time.time()

    if args.try_playwright and try_playwright_save(args.url, out_path, min(args.timeout, 300)):
        print(out_path)
        return 0

    if not args.no_open:
        open_in_browser(args.url)
        print(f"已在默认浏览器打开: {args.url}", file=sys.stderr)

    picked = wait_for_download_html(downloads, since, args.timeout)
    shutil.copy2(picked, out_path)
    print(f"OK picked {picked} → {out_path}", file=sys.stderr)
    print(out_path)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except TimeoutError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        raise SystemExit(1)
    except subprocess.CalledProcessError as e:
        print(f"ERROR: failed to open browser: {e}", file=sys.stderr)
        raise SystemExit(1)
