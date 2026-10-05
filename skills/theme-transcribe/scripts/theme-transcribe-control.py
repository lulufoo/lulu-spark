#!/usr/bin/env python3
"""Theme-transcribe state machine: acquire, verbatim, route.

Usage:
  theme-transcribe-control.py acquire --src URL_OR_FILE --work-dir DIR [--model small]
  theme-transcribe-control.py verbatim --work-dir DIR
  theme-transcribe-control.py route --work-dir DIR

Stdout: one JSON object. Tool noise → <work_dir>/logs/.
Do not re-run Whisper in route; reuse 01-transcript time windows.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

SCRIPTS = Path(__file__).resolve().parent
TRANSCRIBE_SH = SCRIPTS / "transcribe.sh"
REPLACEMENTS = SCRIPTS / "data" / "proper-nouns.json"
DIARIZE_VENV = Path(
    os.environ.get(
        "THEME_TRANSCRIBE_DIARIZE_VENV",
        str(Path.home() / ".local/share/theme-transcribe/diarization-venv"),
    )
)


def _load_schema():
    path = SCRIPTS / "theme-transcribe-schema.py"
    spec = importlib.util.spec_from_file_location("theme_transcribe_schema", path)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


schema = _load_schema()


def _emit(payload: dict[str, Any], code: int = 0) -> int:
    print(json.dumps(payload, ensure_ascii=False, default=schema._json_default))
    return code


def _load_segments(work_dir: Path) -> tuple[Path, str, list[dict[str, Any]]]:
    transcript = schema.find_transcript(work_dir)
    language = schema.language_from_transcript(transcript)
    segments = schema.read_transcript(transcript)
    schema.require_timestamped(segments)
    return transcript, language, segments


def cmd_acquire(args: argparse.Namespace) -> int:
    work_dir = Path(args.work_dir).expanduser().resolve()
    work_dir.mkdir(parents=True, exist_ok=True)
    if not TRANSCRIBE_SH.is_file():
        return _emit({"ok": False, "phase": "acquire", "error": "transcribe.sh missing"}, 1)
    proc = subprocess.run(
        ["bash", str(TRANSCRIBE_SH), args.src, str(work_dir), args.model],
        check=False,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        return _emit(
            {
                "ok": False,
                "phase": "acquire",
                "error": "transcribe.sh failed",
                "stderr": (proc.stderr or "")[-400:],
            },
            proc.returncode,
        )
    transcript, language, segments = _load_segments(work_dir)
    duration_s = segments[-1]["end"] if segments else 0
    meta = schema.load_meta(work_dir)
    meta.update(
        {
            "language": language,
            "model": args.model,
            "source_url": args.src,
            "segment_count": len(segments),
            "duration_s": duration_s,
        }
    )
    schema.write_meta(work_dir, meta)
    return _emit(
        {
            "ok": True,
            "phase": "acquire",
            "work_dir": str(work_dir),
            "language": language,
            "segment_count": len(segments),
            "duration_s": duration_s,
            "transcript": str(transcript),
        }
    )


def cmd_verbatim(args: argparse.Namespace) -> int:
    work_dir = Path(args.work_dir).expanduser().resolve()
    _, language, segments = _load_segments(work_dir)
    replacements = schema.load_replacements(REPLACEMENTS) if REPLACEMENTS.is_file() else []
    hits: list[dict[str, str]] = []
    polished: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()
    for item in segments:
        text, item_hits = schema.apply_corrections(item["text"], replacements)
        polished.append({**item, "text": text})
        for hit in item_hits:
            key = (hit["from"], hit["to"])
            if key not in seen:
                seen.add(key)
                hits.append(hit)
    out = work_dir / f"02-verbatim.{language}.md"
    schema.write_verbatim(out, polished, hits)
    meta = schema.load_meta(work_dir)
    meta.update(
        {
            "language": language,
            "verbatim": str(out),
            "correction_count": len(hits),
        }
    )
    schema.write_meta(work_dir, meta)
    return _emit(
        {
            "ok": True,
            "phase": "verbatim",
            "language": language,
            "segment_count": len(polished),
            "correction_count": len(hits),
            "verbatim": str(out),
        }
    )


def _wav_path(work_dir: Path) -> Path | None:
    wav = work_dir / "media" / "audio.wav"
    return wav if wav.is_file() else None


def _extend_sys_path() -> None:
    site = DIARIZE_VENV / "lib"
    if not site.is_dir():
        return
    for candidate in site.glob("python*/site-packages"):
        path = str(candidate)
        if path not in sys.path:
            sys.path.insert(0, path)


def _cluster_speakers(
    wav: Path, segments: list[dict[str, Any]], log_path: Path
) -> tuple[list[dict[str, Any]] | None, str]:
    """Return labeled segments or (None, reason). Never runs Whisper."""
    _extend_sys_path()
    try:
        import numpy as np
        from sklearn.cluster import KMeans
        from sklearn.metrics import silhouette_score
        from resemblyzer import VoiceEncoder, preprocess_wav
    except ImportError as exc:
        return None, f"embeddings_unavailable:{exc.name}"

    usable = [item for item in segments if (item["end"] - item["start"]) >= schema.MIN_EMBED_S]
    if len(usable) < 8:
        return None, "too_few_embeddable_segments"

    try:
        wav_arr = preprocess_wav(str(wav))
        encoder = VoiceEncoder()
        embeddings = []
        keep = []
        sr = 16000
        for item in usable:
            start_i = int(item["start"] * sr)
            end_i = int(item["end"] * sr)
            slice_ = wav_arr[start_i:end_i]
            if len(slice_) < int(schema.MIN_EMBED_S * sr):
                continue
            embeddings.append(encoder.embed_utterance(slice_))
            keep.append(item)
        if len(embeddings) < 8:
            return None, "too_few_embeddings"
        matrix = np.vstack(embeddings)
        best_k = None
        best_score = -1.0
        best_labels = None
        max_k = 2 if len(keep) < 24 else 3
        for k in range(2, max_k + 1):
            model = KMeans(n_clusters=k, n_init=10, random_state=0)
            labels = model.fit_predict(matrix)
            if len(set(labels)) < 2:
                continue
            score = float(silhouette_score(matrix, labels))
            if score > best_score:
                best_k, best_score, best_labels = k, score, labels
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_path.write_text(
            f"k={best_k} silhouette={best_score:.4f} embeddings={len(keep)}\n",
            encoding="utf-8",
        )
        if best_labels is None or best_score < schema.SILHOUETTE_MIN:
            return None, f"unstable_clusters:{best_score:.4f}"
        first_seen: dict[int, int] = {}
        for idx, label in enumerate(best_labels):
            label_i = int(label)
            if label_i not in first_seen:
                first_seen[label_i] = idx
        order = sorted(first_seen, key=lambda lab: first_seen[lab])
        names = {lab: f"Speaker {chr(ord('A') + i)}" for i, lab in enumerate(order)}
        by_id = {id(item): item for item in keep}
        labeled = []
        keep_i = 0
        for item in segments:
            if id(item) in by_id:
                lab = int(best_labels[keep_i])
                labeled.append({**item, "speaker": names[lab], "cluster": lab})
                keep_i += 1
            else:
                neighbor = labeled[-1]["speaker"] if labeled else names[order[0]]
                labeled.append({**item, "speaker": neighbor, "cluster": -1})
        return labeled, f"ok:k={best_k}:score={best_score:.4f}"
    except Exception as exc:  # noqa: BLE001 — degrade, do not fail the run
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_path.write_text(f"diarize failed: {exc}\n", encoding="utf-8")
        return None, f"diarize_error:{type(exc).__name__}"


def cmd_route(args: argparse.Namespace) -> int:
    work_dir = Path(args.work_dir).expanduser().resolve()
    _, language, segments = _load_segments(work_dir)
    wav = _wav_path(work_dir)
    log_path = work_dir / "logs" / "diarize.log"
    labeled = None
    reason = "no_audio"
    if wav is not None:
        labeled, reason = _cluster_speakers(wav, segments, log_path)
    speakers = sorted({item["speaker"] for item in labeled}) if labeled else []
    if labeled is None or len(speakers) < 2:
        windows = schema.merge_time_windows(segments)
        body = schema.render_time_segmented(windows)
        primary = work_dir / f"03-time-segmented.{language}.md"
        schema.write_text(primary, body)
        mode = "time-segmented"
        source_type = "transcript"
        turn_count = len(windows)
        speaker_count = 1
        schema.write_diarization(
            work_dir / "diarization.json",
            {"mode": mode, "reason": reason, "segments": segments},
        )
    else:
        turns = schema.merge_speaker_turns(labeled)
        body = schema.render_dialogue(turns)
        primary = work_dir / f"03-dialogue-timed.{language}.md"
        schema.write_text(primary, body)
        mode = "dialogue-timed"
        source_type = "dialogue"
        turn_count = len(turns)
        speaker_count = len(speakers)
        schema.write_diarization(
            work_dir / "diarization.json",
            {
                "mode": mode,
                "reason": reason,
                "speaker_count": speaker_count,
                "dialogue_turns": turn_count,
                "segments": labeled,
            },
        )
    meta = schema.load_meta(work_dir)
    meta.update(
        {
            "mode": mode,
            "source_type": source_type,
            "primary": str(primary),
            "speaker_count": speaker_count,
            "turn_count": turn_count,
            "route_reason": reason,
        }
    )
    schema.write_meta(work_dir, meta)
    return _emit(
        {
            "ok": True,
            "phase": "route",
            "mode": mode,
            "source_type": source_type,
            "language": language,
            "segment_count": len(segments),
            "speaker_count": speaker_count,
            "turn_count": turn_count,
            "primary": str(primary),
            "reason": reason,
        }
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Theme-transcribe control",
        epilog=(
            "Acquire needs yt-dlp, ffmpeg, and ~/.local/share/theme-transcribe/venv "
            "(openai-whisper). Route embeddings are optional: "
            "uv venv ~/.local/share/theme-transcribe/diarization-venv --python 3.12 && "
            "uv pip install --python ~/.local/share/theme-transcribe/diarization-venv/bin/python "
            "-r theme-transcribe/scripts/requirements-diarize.txt"
        ),
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    acquire = sub.add_parser("acquire", help="Download + Whisper once")
    acquire.add_argument("--src", required=True)
    acquire.add_argument("--work-dir", required=True)
    acquire.add_argument("--model", default="small")
    acquire.set_defaults(func=cmd_acquire)

    verbatim = sub.add_parser("verbatim", help="Light-correct full transcript")
    verbatim.add_argument("--work-dir", required=True)
    verbatim.set_defaults(func=cmd_verbatim)

    route = sub.add_parser("route", help="Dialogue turns or time segments; no second STT")
    route.add_argument("--work-dir", required=True)
    route.set_defaults(func=cmd_route)

    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except (FileNotFoundError, ValueError) as exc:
        return _emit({"ok": False, "error": str(exc)}, 1)


if __name__ == "__main__":
    sys.exit(main())
