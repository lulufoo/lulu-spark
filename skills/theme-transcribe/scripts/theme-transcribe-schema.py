#!/usr/bin/env python3
"""I/O and validation for one theme-transcribe work directory.

Data files owned here: meta.json, 01-transcript, 02-verbatim, corrections.json,
diarization.json, 03-dialogue-timed / 03-time-segmented.
Invoked by theme-transcribe-control.py only.
"""
from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

LINE_RE = re.compile(
    r"^\[(\d{2}:\d{2}:\d{2}(?:\.\d+)?)\s*-->\s*(\d{2}:\d{2}:\d{2}(?:\.\d+)?)\]\s*(.*)$"
)

DEFAULT_GAP_S = 2.0
DEFAULT_MAX_S = 45.0
MIN_EMBED_S = 0.35
SILHOUETTE_MIN = 0.15


def parse_clock(stamp: str) -> float:
    parts = stamp.split(":")
    if len(parts) != 3:
        raise ValueError(f"bad clock: {stamp}")
    hours, minutes, sec = parts
    return int(hours) * 3600 + int(minutes) * 60 + float(sec)


def format_clock(seconds: float) -> str:
    total = max(0, int(round(seconds)))
    hours, rem = divmod(total, 3600)
    minutes, secs = divmod(rem, 60)
    if hours:
        return f"{hours}:{minutes:02d}:{secs:02d}"
    return f"{minutes:02d}:{secs:02d}"


def parse_transcript(text: str) -> list[dict[str, Any]]:
    segments: list[dict[str, Any]] = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        match = LINE_RE.match(line)
        if not match:
            continue
        start = parse_clock(match.group(1))
        end = parse_clock(match.group(2))
        body = re.sub(r"\s+", " ", match.group(3)).strip()
        if not body:
            continue
        segments.append({"start": start, "end": max(end, start), "text": body})
    return segments


def read_transcript(path: Path) -> list[dict[str, Any]]:
    return parse_transcript(path.read_text(encoding="utf-8"))


def find_transcript(work_dir: Path) -> Path:
    matches = sorted(work_dir.glob("01-transcript.*.txt"))
    if not matches:
        raise FileNotFoundError("01-transcript.<lang>.txt missing")
    return matches[0]


def language_from_transcript(path: Path) -> str:
    name = path.name
    prefix, suffix = "01-transcript.", ".txt"
    if name.startswith(prefix) and name.endswith(suffix):
        return name[len(prefix) : -len(suffix)]
    return "und"


def load_meta(work_dir: Path) -> dict[str, Any]:
    path = work_dir / "meta.json"
    if not path.is_file():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def write_meta(work_dir: Path, payload: dict[str, Any]) -> None:
    path = work_dir / "meta.json"
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def load_replacements(path: Path) -> list[tuple[str, str]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    rows = data.get("replacements") or []
    out: list[tuple[str, str]] = []
    for row in rows:
        if isinstance(row, (list, tuple)) and len(row) == 2:
            old, new = str(row[0]), str(row[1])
            if old and old != new:
                out.append((old, new))
    return out


def apply_corrections(
    text: str, replacements: list[tuple[str, str]]
) -> tuple[str, list[dict[str, str]]]:
    hits: list[dict[str, str]] = []
    updated = text
    for old, new in replacements:
        if old in updated:
            updated = updated.replace(old, new)
            hits.append({"from": old, "to": new})
    return updated, hits


def write_verbatim(
    path: Path,
    segments: list[dict[str, Any]],
    hits: list[dict[str, str]],
) -> None:
    lines = []
    for item in segments:
        lines.append(
            f"[{format_clock(item['start'])} --> {format_clock(item['end'])}] {item['text']}"
        )
    path.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")
    corrections_path = path.with_name("corrections.json")
    corrections_path.write_text(
        json.dumps({"hits": hits, "count": len(hits)}, ensure_ascii=False, indent=2)
        + "\n",
        encoding="utf-8",
    )


def merge_time_windows(
    segments: list[dict[str, Any]],
    *,
    gap_s: float = DEFAULT_GAP_S,
    max_s: float = DEFAULT_MAX_S,
) -> list[dict[str, Any]]:
    if not segments:
        return []
    windows: list[dict[str, Any]] = []
    current = {
        "start": segments[0]["start"],
        "end": segments[0]["end"],
        "texts": [segments[0]["text"]],
    }
    for item in segments[1:]:
        gap = item["start"] - current["end"]
        span = item["end"] - current["start"]
        if gap > gap_s or span > max_s:
            windows.append(current)
            current = {
                "start": item["start"],
                "end": item["end"],
                "texts": [item["text"]],
            }
        else:
            current["end"] = item["end"]
            current["texts"].append(item["text"])
    windows.append(current)
    return windows


def merge_speaker_turns(segments: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not segments:
        return []
    turns: list[dict[str, Any]] = []
    current = {
        "start": segments[0]["start"],
        "end": segments[0]["end"],
        "speaker": segments[0]["speaker"],
        "texts": [segments[0]["text"]],
    }
    for item in segments[1:]:
        if item.get("speaker") == current["speaker"]:
            current["end"] = item["end"]
            current["texts"].append(item["text"])
        else:
            turns.append(current)
            current = {
                "start": item["start"],
                "end": item["end"],
                "speaker": item["speaker"],
                "texts": [item["text"]],
            }
    turns.append(current)
    return turns


def render_time_segmented(windows: list[dict[str, Any]]) -> str:
    blocks = []
    for window in windows:
        text = " ".join(window["texts"]).strip()
        blocks.append(
            f"**{format_clock(window['start'])}–{format_clock(window['end'])}**\n\n{text}"
        )
    return "\n\n".join(blocks) + ("\n" if blocks else "")


def render_dialogue(turns: list[dict[str, Any]]) -> str:
    blocks = []
    for turn in turns:
        text = " ".join(turn["texts"]).strip()
        speaker = turn["speaker"]
        blocks.append(
            f"**{format_clock(turn['start'])}–{format_clock(turn['end'])} · {speaker}**\n\n{text}"
        )
    return "\n\n".join(blocks) + ("\n" if blocks else "")


def write_text(path: Path, body: str) -> None:
    path.write_text(body, encoding="utf-8")


def write_diarization(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2, default=_json_default) + "\n",
        encoding="utf-8",
    )


def _json_default(value: Any) -> Any:
    if hasattr(value, "item"):
        return value.item()
    raise TypeError(f"Unsupported JSON value: {type(value).__name__}")


def require_timestamped(segments: list[dict[str, Any]]) -> None:
    if not segments:
        raise ValueError("no timestamped segments")
