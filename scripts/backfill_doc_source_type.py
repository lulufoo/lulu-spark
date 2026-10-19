#!/usr/bin/env python3
"""One-off data correction (LW-418): map leftover `source_type` values onto the locked list.

Knowledge documents imported without a category still have NULL. Notes videos still use the
retired id `theme-line`. This script writes those two mappings and nothing else:

  * Knowledge workspace rows with `source_type IS NULL` → `article`
  * every workspace document table, `source_type = 'theme-line'` → `video`

It is not an app migration: run it by hand, once. `transcript` and other old ids stay as they
are.

    python3 scripts/backfill_doc_source_type.py            # dry run: report only
    python3 scripts/backfill_doc_source_type.py --apply    # back up, then write

Rules:
  * Table names come from the `workspace` registry (`doc_` + workspace id), never from argv.
  * Only the `source_type` column is written. Files, digest, comments, and other columns stay.
  * --apply first writes a consistent copy with VACUUM INTO into <data dir>/_backup/.
  * A second --apply changes 0 rows.
"""

from __future__ import annotations

import argparse
import re
import sqlite3
import sys
from datetime import datetime
from pathlib import Path

DEFAULT_DATA_DIR = Path.home() / ".cache" / "lulu-spark" / "data"
DOC_TABLE_RE = re.compile(r"^doc_[a-z0-9_]{1,48}$")


def validate_doc_table(table: str) -> str:
    if not DOC_TABLE_RE.fullmatch(table):
        raise SystemExit(f"invalid document table name: {table!r}")
    return table


def workspaces(conn: sqlite3.Connection) -> list[tuple[str, str, str | None]]:
    cols = {row[1] for row in conn.execute("PRAGMA table_info(workspace)")}
    if "name" in cols:
        rows = conn.execute("SELECT id, doc_table, name FROM workspace ORDER BY id").fetchall()
        return [(workspace_id, validate_doc_table(table), name) for workspace_id, table, name in rows]
    rows = conn.execute("SELECT id, doc_table FROM workspace ORDER BY id").fetchall()
    return [(workspace_id, validate_doc_table(table), None) for workspace_id, table in rows]


def is_knowledge_workspace(workspace_id: str, name: str | None) -> bool:
    return workspace_id == "knowledge" or name == "Knowledge"


def count_rows(conn: sqlite3.Connection, table: str, where: str) -> int:
    return int(conn.execute(f"SELECT COUNT(*) FROM {table} WHERE {where}").fetchone()[0])


def plan_changes(conn: sqlite3.Connection) -> tuple[list[tuple[str, str, int]], list[tuple[str, str, int]]]:
    """Per-workspace NULL→article (Knowledge only) and theme-line→video (every table)."""
    nulls: list[tuple[str, str, int]] = []
    themes: list[tuple[str, str, int]] = []
    for workspace_id, table, name in workspaces(conn):
        themes.append((workspace_id, table, count_rows(conn, table, "source_type = 'theme-line'")))
        if is_knowledge_workspace(workspace_id, name):
            nulls.append((workspace_id, table, count_rows(conn, table, "source_type IS NULL")))
    return nulls, themes


def print_report(nulls: list[tuple[str, str, int]], themes: list[tuple[str, str, int]], apply: bool) -> int:
    mode = "APPLY" if apply else "DRY RUN"
    null_total = sum(n for _, _, n in nulls)
    theme_total = sum(n for _, _, n in themes)
    print(f"[{mode}] Knowledge NULL → article: {null_total}")
    for workspace_id, table, n in nulls:
        print(f"  {workspace_id} ({table}): {n}")
    print(f"[{mode}] theme-line → video: {theme_total}")
    for workspace_id, table, n in themes:
        print(f"  {workspace_id} ({table}): {n}")
    print(f"total: {null_total + theme_total} row(s)")
    return null_total + theme_total


def write_backup(conn: sqlite3.Connection, data_dir: Path) -> Path:
    backup_dir = data_dir / "_backup"
    backup_dir.mkdir(parents=True, exist_ok=True)
    target = backup_dir / f"spark-data-before-source-type-backfill-{datetime.now():%Y%m%d%H%M%S}.sqlite"
    conn.execute("VACUUM INTO ?", (str(target),))
    return target


def apply_changes(
    conn: sqlite3.Connection,
    nulls: list[tuple[str, str, int]],
    themes: list[tuple[str, str, int]],
) -> int:
    changed = 0
    with conn:
        for _, table, n in nulls:
            if not n:
                continue
            changed += conn.execute(
                f"UPDATE {table} SET source_type = 'article' WHERE source_type IS NULL"
            ).rowcount
        for _, table, n in themes:
            if not n:
                continue
            changed += conn.execute(
                f"UPDATE {table} SET source_type = 'video' WHERE source_type = 'theme-line'"
            ).rowcount
    return changed


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--data-dir", type=Path, default=DEFAULT_DATA_DIR, help="runtime data dir")
    parser.add_argument("--apply", action="store_true", help="write the changes (default: report only)")
    return parser.parse_args(argv)


def run_backfill(args: argparse.Namespace) -> int:
    db_path = args.data_dir / "spark-data.sqlite"
    if not db_path.is_file():
        print(f"database not found: {db_path}", file=sys.stderr)
        return 1
    conn = sqlite3.connect(db_path, timeout=30)
    try:
        nulls, themes = plan_changes(conn)
        total = print_report(nulls, themes, args.apply)
        if not args.apply:
            print("dry run only; run again with --apply to write")
            return 0
        if total == 0:
            print("nothing to change")
            return 0
        print(f"backup written: {write_backup(conn, args.data_dir)}")
        print(f"updated {apply_changes(conn, nulls, themes)} row(s)")
        return 0
    finally:
        conn.close()


if __name__ == "__main__":
    sys.exit(run_backfill(parse_args(sys.argv[1:])))
