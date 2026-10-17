#!/usr/bin/env python3
"""Fixture run for LW-428: Knowledge inbox, one is_inbox, dirs named by id."""

from __future__ import annotations

import sqlite3
import tempfile
import unittest
from pathlib import Path

from backfill_project_inbox_and_dirs import parse_args, run_backfill

TRIGGER_SQL = (
    "CREATE TRIGGER project_keys_immutable BEFORE UPDATE OF id, is_inbox ON project\n"
    "BEGIN SELECT RAISE(ABORT, 'project id and is_inbox cannot change'); END;"
)


def plant(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def build_copy(root: Path) -> Path:
    db = root / "spark-data.sqlite"
    conn = sqlite3.connect(db)
    conn.executescript(
        """
        CREATE TABLE workspace (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          doc_table TEXT NOT NULL
        );
        CREATE TABLE project (
          id TEXT PRIMARY KEY,
          workspace_id TEXT NOT NULL REFERENCES workspace(id),
          folder TEXT NOT NULL,
          title TEXT NOT NULL,
          description TEXT NOT NULL DEFAULT '',
          icon TEXT,
          sort_index INTEGER NOT NULL DEFAULT 0,
          is_inbox INTEGER NOT NULL DEFAULT 0,
          UNIQUE (workspace_id, folder)
        );
        CREATE UNIQUE INDEX project_one_inbox ON project(workspace_id) WHERE is_inbox = 1;
        INSERT INTO workspace(id, name, doc_table) VALUES
          ('default', 'Default', 'doc_default'),
          ('knowledge', 'Knowledge', 'doc_knowledge'),
          ('made', 'Made', 'doc_made');
        INSERT INTO project(id, workspace_id, folder, title, description, sort_index, is_inbox) VALUES
          ('inbox', 'default', 'inbox', 'Inbox', '', 0, 1),
          ('k1', 'knowledge', 'articles', 'Articles', '', 0, 0),
          ('missed', 'made', 'inbox', 'Inbox', '', 0, 0),
          ('abc', 'made', 'oldname', 'Old', '', 1, 0);
        """
    )
    conn.execute(TRIGGER_SQL)
    conn.commit()
    conn.close()
    plant(root / "workspaces/default/raw/inbox/notes/a.md", "default")
    plant(root / "workspaces/knowledge/raw/articles/x.md", "knowledge")
    plant(root / "workspaces/knowledge/digest/articles/x.md", "digest")
    plant(root / "workspaces/made/raw/inbox/notes/b.md", "made-inbox")
    plant(root / "workspaces/made/raw/oldname/y.md", "old")
    return root


class BackfillInboxAndDirsTest(unittest.TestCase):
    def test_apply_then_second_run_is_noop(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            data = build_copy(Path(tmp))
            self.assertEqual(run_backfill(parse_args(["--data-dir", str(data)])), 0)
            self.assertEqual(run_backfill(parse_args(["--data-dir", str(data), "--apply"])), 0)

            conn = sqlite3.connect(data / "spark-data.sqlite")
            inboxes = conn.execute(
                "SELECT workspace_id, id, folder FROM project WHERE is_inbox = 1 ORDER BY workspace_id"
            ).fetchall()
            self.assertEqual([row[0] for row in inboxes], ["default", "knowledge", "made"])
            self.assertEqual(inboxes[0], ("default", "inbox", "inbox"))
            self.assertEqual(inboxes[1][2], "inbox")
            self.assertEqual(inboxes[2], ("made", "missed", "inbox"))
            self.assertEqual(
                conn.execute("SELECT COUNT(*) FROM project WHERE workspace_id = 'knowledge'").fetchone()[0],
                2,
            )
            self.assertTrue(
                conn.execute(
                    "SELECT 1 FROM sqlite_master WHERE type = 'trigger' AND name = 'project_keys_immutable'"
                ).fetchone()
            )
            conn.close()

            self.assertTrue((data / "workspaces/default/raw/inbox/notes/a.md").is_file())
            self.assertTrue((data / "workspaces/knowledge/raw/k1/x.md").is_file())
            self.assertTrue((data / "workspaces/knowledge/digest/k1/x.md").is_file())
            self.assertFalse((data / "workspaces/knowledge/raw/articles").exists())
            self.assertTrue((data / "workspaces/made/raw/missed/notes/b.md").is_file())
            self.assertFalse((data / "workspaces/made/raw/inbox").exists())
            self.assertTrue((data / "workspaces/made/raw/abc/y.md").is_file())
            self.assertFalse((data / "workspaces/made/raw/oldname").exists())

            self.assertEqual(run_backfill(parse_args(["--data-dir", str(data), "--apply"])), 0)

    def test_dest_exists_stops_without_rename(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            data = build_copy(Path(tmp))
            plant(data / "workspaces/knowledge/raw/k1/x.md", "already")
            self.assertEqual(run_backfill(parse_args(["--data-dir", str(data), "--apply"])), 1)
            self.assertTrue((data / "workspaces/knowledge/raw/articles/x.md").is_file())


if __name__ == "__main__":
    unittest.main()
