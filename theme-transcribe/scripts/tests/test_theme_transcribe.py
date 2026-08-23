#!/usr/bin/env python3
"""Schema and control checks for theme-transcribe."""
from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
CONTROL = SCRIPTS / "theme-transcribe-control.py"
FIXTURE = Path(__file__).resolve().parent / "fixtures" / "01-transcript.en.txt"


def load_schema():
    path = SCRIPTS / "theme-transcribe-schema.py"
    spec = importlib.util.spec_from_file_location("theme_transcribe_schema", path)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


schema = load_schema()


class SchemaTest(unittest.TestCase):
    def test_parse_and_correct_fixture(self) -> None:
        segments = schema.parse_transcript(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(len(segments), 6)
        self.assertEqual(segments[0]["start"], 0)
        replacements = schema.load_replacements(SCRIPTS / "data" / "proper-nouns.json")
        text, hits = schema.apply_corrections(segments[0]["text"], replacements)
        self.assertIn("core web vitals", text)
        self.assertTrue(hits)

    def test_merge_speaker_turns(self) -> None:
        turns = schema.merge_speaker_turns(
            [
                {"start": 0, "end": 3, "text": "a", "speaker": "Speaker A"},
                {"start": 3, "end": 8, "text": "b", "speaker": "Speaker A"},
                {"start": 8, "end": 12, "text": "c", "speaker": "Speaker B"},
            ]
        )
        self.assertEqual(len(turns), 2)
        self.assertEqual(turns[0]["end"], 8)
        self.assertEqual(turns[0]["texts"], ["a", "b"])
        rendered = schema.render_dialogue(turns)
        self.assertIn("00:00–00:08 · Speaker A", rendered)
        self.assertIn("00:08–00:12 · Speaker B", rendered)

    def test_time_windows_split_on_gap(self) -> None:
        windows = schema.merge_time_windows(
            [
                {"start": 0, "end": 3, "text": "a"},
                {"start": 3.2, "end": 6, "text": "b"},
                {"start": 20, "end": 24, "text": "c"},
            ]
        )
        self.assertEqual(len(windows), 2)
        self.assertEqual(windows[0]["texts"], ["a", "b"])
        rendered = schema.render_time_segmented(windows)
        self.assertIn("**00:00–00:06**", rendered)
        self.assertNotIn("Speaker", rendered)


class ControlTest(unittest.TestCase):
    def _work(self, tmp: str) -> Path:
        work = Path(tmp)
        (work / "01-transcript.en.txt").write_text(
            FIXTURE.read_text(encoding="utf-8"), encoding="utf-8"
        )
        (work / "meta.json").write_text(
            json.dumps({"language": "en", "model": "small", "source_url": "file://fixture"}),
            encoding="utf-8",
        )
        return work

    def _run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(CONTROL), *args],
            check=False,
            capture_output=True,
            text=True,
        )

    def test_verbatim_and_route_without_wav(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = self._work(tmp)
            verbatim = self._run("verbatim", "--work-dir", str(work))
            self.assertEqual(verbatim.returncode, 0, verbatim.stderr + verbatim.stdout)
            v_payload = json.loads(verbatim.stdout)
            self.assertTrue(v_payload["ok"])
            body = Path(v_payload["verbatim"]).read_text(encoding="utf-8")
            self.assertIn("core web vitals", body)
            self.assertIn("Antithesis", body)
            self.assertIn("Claude Code", body)
            self.assertIn("Addy", body)
            self.assertIn("esbuild", body)

            routed = self._run("route", "--work-dir", str(work))
            self.assertEqual(routed.returncode, 0, routed.stderr + routed.stdout)
            r_payload = json.loads(routed.stdout)
            self.assertTrue(r_payload["ok"])
            self.assertEqual(r_payload["mode"], "time-segmented")
            self.assertEqual(r_payload["source_type"], "transcript")
            self.assertTrue(Path(r_payload["primary"]).is_file())
            primary = Path(r_payload["primary"]).read_text(encoding="utf-8")
            self.assertIn("**00:00–", primary)
            self.assertNotIn("Speaker", primary)
            self.assertFalse((work / "03-dialogue-timed.en.md").exists())

    def test_missing_transcript_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            routed = self._run("route", "--work-dir", tmp)
            self.assertNotEqual(routed.returncode, 0)
            payload = json.loads(routed.stdout)
            self.assertFalse(payload["ok"])


if __name__ == "__main__":
    unittest.main()
