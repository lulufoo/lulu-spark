#!/usr/bin/env python3
"""One-off data correction (LW-377): fill `created_at` of Knowledge documents from file mtime.

The Knowledge documents were imported without a `created_at`, so the timeline view puts all
of them in one `Undated` group. This script gives every such row the modification date of
its raw file, in the same 12-digit local-time format Default uses (`YYYYMMDDHHmm`).

It is not an app migration: run it by hand, once.

    python3 scripts/backfill_knowledge_created_at.py            # dry run: report only
    python3 scripts/backfill_knowledge_created_at.py --apply    # back up, then write

Rules:
  * Only rows of the workspace's doc table whose `created_at` is NULL or empty are touched,
    so a second run changes 0 rows and existing values are never overwritten.
  * File path: <data dir>/workspaces/<workspace id>/raw/<project folder>/<raw_path>.
    A row whose file is missing keeps NULL and is listed in the report.
  * --apply first writes a consistent copy with VACUUM INTO into <data dir>/_backup/ under a
    name the app's own snapshot pruning does not match.
"""

from __future__ import annotations

import argparse
import sqlite3
import sys
from collections import Counter
from datetime import datetime
from pathlib import Path

DEFAULT_DATA_DIR = Path.home() / ".cache" / "lulu-spark" / "data"


def format_local_stamp(timestamp: float) -> str:
    """12-digit local time, the format of `doc_default.created_at`."""
    return datetime.fromtimestamp(timestamp).strftime("%Y%m%d%H%M")


def plan_fills(conn: sqlite3.Connection, data_dir: Path, workspace_id: str):
    """Rows to fill as (doc_id, stamp) and rows without a file as (doc_id, path)."""
    row = conn.execute("SELECT doc_table FROM workspace WHERE id = ?", (workspace_id,)).fetchone()
    if row is None:
        raise SystemExit(f"workspace `{workspace_id}` not found")
    table = row[0]  # the table name comes from the registry, never from user input
    rows = conn.execute(
        f"SELECT d.doc_id, p.folder, d.raw_path FROM {table} d "
        "JOIN project p ON p.id = d.project_id "
        "WHERE d.created_at IS NULL OR d.created_at = '' ORDER BY d.doc_id"
    ).fetchall()
    fills, missing = [], []
    raw_root = data_dir / "workspaces" / workspace_id / "raw"
    for doc_id, folder, raw_path in rows:
        path = raw_root / folder / raw_path
        try:
            fills.append((doc_id, format_local_stamp(path.stat().st_mtime)))
        except OSError:
            missing.append((doc_id, str(path)))
    return table, fills, missing


def print_report(table: str, fills, missing, apply: bool) -> None:
    mode = "APPLY" if apply else "DRY RUN"
    print(f"[{mode}] table {table}: {len(fills)} row(s) to fill, {len(missing)} without a file")
    days = Counter(stamp[:8] for _, stamp in fills)
    if days:
        print("date distribution (day: rows):")
        for day, count in sorted(days.items()):
            print(f"  {day}: {count}")
    if missing:
        print("rows whose file is missing (stay NULL):")
        for doc_id, path in missing:
            print(f"  {doc_id}  {path}")


def write_backup(conn: sqlite3.Connection, data_dir: Path) -> Path:
    backup_dir = data_dir / "_backup"
    backup_dir.mkdir(parents=True, exist_ok=True)
    target = backup_dir / f"spark-data-before-created-at-backfill-{datetime.now():%Y%m%d%H%M%S}.sqlite"
    conn.execute("VACUUM INTO ?", (str(target),))
    return target


def apply_fills(conn: sqlite3.Connection, table: str, fills) -> int:
    """Write each stamp once; a row that gained a value in the meantime is left alone."""
    changed = 0
    with conn:
        for doc_id, stamp in fills:
            changed += conn.execute(
                f"UPDATE {table} SET created_at = ? WHERE doc_id = ? AND (created_at IS NULL OR created_at = '')",
                (stamp, doc_id),
            ).rowcount
    return changed


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--data-dir", type=Path, default=DEFAULT_DATA_DIR, help="runtime data dir")
    parser.add_argument("--workspace", default="knowledge", help="workspace id to correct")
    parser.add_argument("--apply", action="store_true", help="write the changes (default: report only)")
    return parser.parse_args(argv)


def run_backfill(args: argparse.Namespace) -> int:
    db_path = args.data_dir / "spark-data.sqlite"
    if not db_path.is_file():
        print(f"database not found: {db_path}", file=sys.stderr)
        return 1
    conn = sqlite3.connect(db_path, timeout=30)
    try:
        table, fills, missing = plan_fills(conn, args.data_dir, args.workspace)
        print_report(table, fills, missing, args.apply)
        if not args.apply:
            print("dry run only; run again with --apply to write")
            return 0
        if not fills:
            print("nothing to change")
            return 0
        print(f"backup written: {write_backup(conn, args.data_dir)}")
        print(f"updated {apply_fills(conn, table, fills)} row(s)")
        return 0
    finally:
        conn.close()


if __name__ == "__main__":
    sys.exit(run_backfill(parse_args(sys.argv[1:])))
