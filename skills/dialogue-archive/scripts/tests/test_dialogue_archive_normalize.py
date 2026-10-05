#!/usr/bin/env python3
"""Fixture checks for dialogue-archive/scripts/dialogue_archive_normalize.py."""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
SCRIPT = SCRIPTS / "dialogue_archive_normalize.py"
FIXTURE = Path(__file__).resolve().parent / "fixtures" / "sample.jsonl"


class DialogueArchiveNormalizeTest(unittest.TestCase):
    def test_fixture_slice_merges_assistants_and_skips_turn_ended(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out.md"
            proc = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--transcript",
                    str(FIXTURE),
                    "--start-node",
                    "1",
                    "--end-node",
                    "6",
                    "--out",
                    str(out),
                    "--title",
                    "fixture",
                    "--project",
                    "inbox",
                    "--doc-theme",
                    "dialogue",
                    "--slug",
                    "fixture-test",
                    "--created-at",
                    "2026年1月1日 12:00",
                    "--ts",
                    "202601011200",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(proc.returncode, 0, proc.stderr)
            self.assertIn(f"OUT={out.resolve()}", proc.stdout)
            body = out.read_text(encoding="utf-8")
            self.assertIn("<!-- DDM:TURN_SEP:v1 -->", body)
            self.assertIn("## User（Turn 1）", body)
            self.assertIn("hello start", body)
            self.assertIn("first reply", body)
            self.assertIn("second assistant chunk", body)
            self.assertIn("visible answer", body)
            self.assertNotIn("<thinking>", body)
            self.assertNotIn("<user_query>", body)
            self.assertNotIn("导航：[digest]", body)
            self.assertEqual(body.count("## User（Turn"), 2)

    def test_invalid_range_exits_2(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out.md"
            proc = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--transcript",
                    str(FIXTURE),
                    "--start-node",
                    "9",
                    "--end-node",
                    "10",
                    "--out",
                    str(out),
                    "--title",
                    "t",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(proc.returncode, 2)

    def test_dry_run_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out.md"
            proc = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--transcript",
                    str(FIXTURE),
                    "--start-node",
                    "1",
                    "--end-node",
                    "6",
                    "--out",
                    str(out),
                    "--title",
                    "t",
                    "--dry-run",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(proc.returncode, 0, proc.stderr)
            data = json.loads(proc.stdout)
            self.assertEqual(data["turns"], 2)
            self.assertEqual(data["skipped_non_chat"], 1)
            self.assertFalse(out.exists())


if __name__ == "__main__":
    unittest.main()
