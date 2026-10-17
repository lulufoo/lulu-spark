#!/usr/bin/env python3
"""One-off data correction (LW-428): inbox rows and disk dirs named by project id.

Run by hand on a copy of the runtime data dir. Not a Host migration.

    python3 scripts/backfill_project_inbox_and_dirs.py --data-dir COPY
    python3 scripts/backfill_project_inbox_and_dirs.py --data-dir COPY --apply

1. A workspace with no `is_inbox=1` row gets an inbox (`folder=inbox`, minted id).
2. A single `folder=inbox` row with `is_inbox=0` is marked. Two such rows stop the run.
3. `{raw|digest}/{folder}/` is renamed to `{raw|digest}/{project.id}/` when they differ.
   A directory already named as the id is skipped.

The `folder` column is left in place. A second --apply is a no-op.
"""

from __future__ import annotations

import argparse
import secrets
import sqlite3
import string
import sys
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path

DEFAULT_DATA_DIR = Path.home() / ".cache" / "lulu-spark" / "data"
LAYERS = ("raw", "digest")
ALPHABET = string.ascii_lowercase + string.digits
INBOX_FOLDER = "inbox"
TRIGGER_SQL = (
    "CREATE TRIGGER project_keys_immutable BEFORE UPDATE OF id, is_inbox ON project\n"
    "BEGIN SELECT RAISE(ABORT, 'project id and is_inbox cannot change'); END;"
)


@dataclass
class Project:
    id: str
    folder: str
    is_inbox: bool


@dataclass
class Rename:
    layer: str
    src: Path
    dst: Path
    action: str


@dataclass
class Row:
    workspace_id: str
    old_folder: str
    project_id: str
    inbox: str
    rename: str
    error: str = ""


@dataclass
class Plan:
    rows: list[Row] = field(default_factory=list)
    marks: list[str] = field(default_factory=list)
    inserts: list[tuple[str, str]] = field(default_factory=list)
    renames: list[Rename] = field(default_factory=list)
    errors: list[str] = field(default_factory=list)


def require_schema(conn: sqlite3.Connection) -> None:
    cols = {row[1] for row in conn.execute("PRAGMA table_info(project)")}
    if "is_inbox" not in cols:
        raise SystemExit("project.is_inbox missing; apply Host schema step 12 first")


def mint_project_id(conn: sqlite3.Connection) -> str:
    tz = timezone(timedelta(hours=8))
    now = datetime.now(tz)
    stamp = f"{now:%Y%m%d%H%M%S}{now.microsecond // 1000:03d}"
    for _ in range(8):
        candidate = stamp + "".join(secrets.choice(ALPHABET) for _ in range(6))
        taken = conn.execute("SELECT 1 FROM project WHERE id = ?", (candidate,)).fetchone()
        if taken is None:
            return candidate
    raise SystemExit("could not allocate project id")


def list_workspaces(conn: sqlite3.Connection) -> list[str]:
    return [row[0] for row in conn.execute("SELECT id FROM workspace ORDER BY id")]


def list_projects(conn: sqlite3.Connection, workspace_id: str) -> list[Project]:
    rows = conn.execute(
        "SELECT id, folder, is_inbox FROM project WHERE workspace_id = ? ORDER BY id",
        (workspace_id,),
    )
    return [Project(id=r[0], folder=r[1], is_inbox=bool(r[2])) for r in rows]


def plan_renames(data_dir: Path, workspace_id: str, project: Project) -> list[Rename]:
    out: list[Rename] = []
    if project.folder == project.id:
        return out
    root = data_dir / "workspaces" / workspace_id
    for layer in LAYERS:
        src = root / layer / project.folder
        dst = root / layer / project.id
        if not src.exists():
            out.append(Rename(layer, src, dst, "skip-missing"))
        elif dst.exists():
            out.append(Rename(layer, src, dst, "dest-exists"))
        else:
            out.append(Rename(layer, src, dst, "rename"))
    return out


def describe_renames(items: list[Rename]) -> str:
    if not items:
        return "skip (already id)"
    if any(item.action == "dest-exists" for item in items):
        return "conflict dest exists"
    doing = [item.layer for item in items if item.action == "rename"]
    if doing:
        return ",".join(doing)
    return "skip-missing"


def plan_workspace(conn: sqlite3.Connection, data_dir: Path, workspace_id: str) -> Plan:
    plan = Plan()
    projects = list_projects(conn, workspace_id)
    inboxes = [p for p in projects if p.is_inbox]
    folder_inboxes = [p for p in projects if p.folder == INBOX_FOLDER]
    inbox_action = "keep"
    inbox_id = inboxes[0].id if len(inboxes) == 1 else ""

    if len(inboxes) > 1:
        plan.errors.append(f"{workspace_id}: more than one is_inbox")
    elif len(folder_inboxes) > 1:
        plan.errors.append(f"{workspace_id}: more than one folder=inbox")
    elif inboxes and folder_inboxes and folder_inboxes[0].id != inboxes[0].id:
        plan.errors.append(f"{workspace_id}: is_inbox and folder=inbox disagree")
    elif not inboxes and len(folder_inboxes) == 1:
        inbox_action = "marked"
        inbox_id = folder_inboxes[0].id
        plan.marks.append(inbox_id)
    elif not inboxes:
        inbox_action = "created"
        inbox_id = mint_project_id(conn)
        plan.inserts.append((workspace_id, inbox_id))
        projects.append(Project(id=inbox_id, folder=INBOX_FOLDER, is_inbox=True))

    if plan.errors:
        plan.rows.append(Row(workspace_id, "", "", "", "", plan.errors[0]))
        return plan

    for project in projects:
        items = plan_renames(data_dir, workspace_id, project)
        conflict = [item for item in items if item.action == "dest-exists"]
        if conflict:
            plan.errors.append(f"{workspace_id}/{project.folder}: dest {project.id} exists")
        plan.renames.extend(item for item in items if item.action == "rename")
        inbox = inbox_action if project.id == inbox_id else "—"
        plan.rows.append(
            Row(workspace_id, project.folder, project.id, inbox, describe_renames(items),
                plan.errors[-1] if conflict else "")
        )
    return plan


def merge_plans(parts: list[Plan]) -> Plan:
    merged = Plan()
    for part in parts:
        merged.rows.extend(part.rows)
        merged.marks.extend(part.marks)
        merged.inserts.extend(part.inserts)
        merged.renames.extend(part.renames)
        merged.errors.extend(part.errors)
    return merged


def print_report(plan: Plan, apply: bool) -> None:
    mode = "APPLY" if apply else "DRY RUN"
    print(f"[{mode}] workspace  folder  id  inbox  rename")
    for row in plan.rows:
        extra = f"  ERROR {row.error}" if row.error else ""
        print(f"  {row.workspace_id}  {row.old_folder or '—'}  {row.project_id or '—'}  "
              f"{row.inbox}  {row.rename}{extra}")
    print(f"mark={len(plan.marks)} insert={len(plan.inserts)} rename={len(plan.renames)} "
          f"errors={len(plan.errors)}")


def write_backup(conn: sqlite3.Connection, data_dir: Path) -> Path:
    backup_dir = data_dir / "_backup"
    backup_dir.mkdir(parents=True, exist_ok=True)
    target = backup_dir / f"spark-data-before-inbox-dir-backfill-{datetime.now():%Y%m%d%H%M%S}.sqlite"
    conn.execute("VACUUM INTO ?", (str(target),))
    return target


def drop_inbox_trigger(conn: sqlite3.Connection) -> bool:
    row = conn.execute(
        "SELECT 1 FROM sqlite_master WHERE type = 'trigger' AND name = 'project_keys_immutable'"
    ).fetchone()
    if row is None:
        return False
    conn.execute("DROP TRIGGER project_keys_immutable")
    return True


def apply_plan(conn: sqlite3.Connection, plan: Plan) -> None:
    restored = False
    try:
        if plan.marks:
            restored = drop_inbox_trigger(conn)
            for project_id in plan.marks:
                conn.execute("UPDATE project SET is_inbox = 1 WHERE id = ?", (project_id,))
        for workspace_id, project_id in plan.inserts:
            conn.execute(
                "INSERT INTO project(id, workspace_id, folder, title, description, icon, sort_index, is_inbox) "
                "VALUES (?1, ?2, ?3, 'Inbox', '', NULL, "
                "(SELECT coalesce(max(sort_index) + 1, 0) FROM project WHERE workspace_id = ?2), 1)",
                (project_id, workspace_id, INBOX_FOLDER),
            )
        conn.commit()
    finally:
        if restored:
            conn.execute(TRIGGER_SQL)
            conn.commit()
    for item in plan.renames:
        item.src.rename(item.dst)


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
        require_schema(conn)
        plan = merge_plans(
            plan_workspace(conn, args.data_dir, workspace_id)
            for workspace_id in list_workspaces(conn)
        )
        print_report(plan, args.apply)
        if plan.errors:
            print("stopped: fix the conflicts above", file=sys.stderr)
            return 1
        if not args.apply:
            print("dry run only; run again with --apply to write")
            return 0
        if not (plan.marks or plan.inserts or plan.renames):
            print("nothing to change")
            return 0
        print(f"backup written: {write_backup(conn, args.data_dir)}")
        apply_plan(conn, plan)
        print("applied")
        return 0
    finally:
        conn.close()


if __name__ == "__main__":
    sys.exit(run_backfill(parse_args(sys.argv[1:])))
