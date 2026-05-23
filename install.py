#!/usr/bin/env python3
"""install.py — lulu-workbench-skills multi-platform installer

Usage:
    python3 install.py [--platform cursor|copilot]

If --platform is omitted, auto-detects via COPILOT_AGENT=1 env var.
"""

import argparse
import os
import pathlib
import subprocess
import sys

REPO_URL  = "https://github.com/lulufoo/lulu-workbench-skills.git"
REPO_NAME = "lulu-workbench-skills"

# ── Platform registry ─────────────────────────────────────────────────────────
# To add a new platform: append one entry here. No other changes needed.
PLATFORM_ROOTS: dict[str, pathlib.Path] = {
    "cursor":  pathlib.Path.home() / ".cursor"  / "skills",
    "copilot": pathlib.Path.home() / ".copilot" / "skills",
}


def detect_platform() -> str:
    """Auto-detect platform from env; default to cursor."""
    if os.environ.get("COPILOT_AGENT") == "1":
        return "copilot"
    return "cursor"


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Install lulu-workbench-skills for Cursor or Copilot"
    )
    parser.add_argument(
        "--platform",
        choices=list(PLATFORM_ROOTS),
        help="Target platform (default: auto-detect via COPILOT_AGENT env var)",
    )
    args = parser.parse_args()

    platform = args.platform or detect_platform()
    root: pathlib.Path = PLATFORM_ROOTS[platform]
    root.mkdir(parents=True, exist_ok=True)

    # ── Clone or update ───────────────────────────────────────────────────────
    dest = root / REPO_NAME
    if dest.exists():
        print(f"Updating {dest} ...")
        subprocess.run(
            ["git", "-C", str(dest), "pull", "--rebase"],
            check=True,
        )
    else:
        print(f"Cloning into {dest} ...")
        subprocess.run(["git", "clone", REPO_URL, str(dest)], check=True)

    # ── Summary ───────────────────────────────────────────────────────────────
    config = dest / "config.json"
    print(f"\n✅ lulu-workbench-skills installed ({platform})")
    print(f"   Root  : {root}")
    print(f"   Config: {config}")
    print(f"   Skills: dialogue-summary, theme-summary, theme-line, theme-digest")
    if config.exists():
        print(f"\n   ⚠️  Remember to set archive_root in {config} if not already done.")


if __name__ == "__main__":
    main()
