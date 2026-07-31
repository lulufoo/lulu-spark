#!/usr/bin/env bash
# ThemeTranscribe Phase 1: URL/file → timestamped Whisper transcript.
# Usage: transcribe.sh <url-or-file> <work_dir> [whisper_model]
# Stdout: result paths only. Tool noise → <work_dir>/logs/

set -euo pipefail

SRC="${1:-}"
WORK="${2:-}"
MODEL="${3:-small}"

if [[ -z "$SRC" || -z "$WORK" ]]; then
  echo "usage: transcribe.sh <url-or-file> <work_dir> [whisper_model]" >&2
  exit 2
fi

mkdir -p "$WORK"/{media,logs}
LOG="$WORK/logs"
MEDIA="$WORK/media"

resolve_ytdlp() {
  if command -v yt-dlp >/dev/null 2>&1; then
    command -v yt-dlp
    return
  fi
  local cand="$HOME/Library/Python/3.9/bin/yt-dlp"
  if [[ -x "$cand" ]]; then
    echo "$cand"
    return
  fi
  echo "ERROR: yt-dlp not found (PATH or ~/Library/Python/3.9/bin/yt-dlp)" >&2
  exit 1
}

resolve_ffmpeg() {
  if command -v ffmpeg >/dev/null 2>&1; then
    command -v ffmpeg
    return
  fi
  echo "ERROR: ffmpeg not found" >&2
  exit 1
}

VENV="${THEME_TRANSCRIBE_VENV:-$HOME/.local/share/theme-transcribe/venv}"
WHISPER_PY="$VENV/bin/python"
WHISPER_BIN="$VENV/bin/whisper"

if [[ ! -x "$WHISPER_BIN" && ! -x "$WHISPER_PY" ]]; then
  echo "ERROR: Whisper venv missing at $VENV" >&2
  echo "Bootstrap:" >&2
  echo "  uv venv $VENV --python 3.12" >&2
  echo "  uv pip install --python $VENV/bin/python openai-whisper" >&2
  exit 1
fi

YTDLP="$(resolve_ytdlp)"
FFMPEG="$(resolve_ffmpeg)"

AUDIO_IN=""
if [[ -f "$SRC" ]]; then
  AUDIO_IN="$SRC"
else
  echo "Downloading audio…" >&2
  "$YTDLP" --no-playlist -f "bestaudio/best" \
    -o "$MEDIA/source.%(ext)s" \
    "$SRC" >"$LOG/yt-dlp.log" 2>&1
  AUDIO_IN="$(ls -1t "$MEDIA"/source.* 2>/dev/null | head -1 || true)"
  if [[ -z "$AUDIO_IN" || ! -f "$AUDIO_IN" ]]; then
    echo "ERROR: yt-dlp produced no media; see $LOG/yt-dlp.log" >&2
    exit 1
  fi
fi

WAV="$MEDIA/audio.wav"
echo "Converting to 16k mono WAV…" >&2
"$FFMPEG" -y -i "$AUDIO_IN" -ac 1 -ar 16000 "$WAV" >"$LOG/ffmpeg.log" 2>&1

echo "Whisper model=$MODEL …" >&2
# Prefer whisper CLI; fall back to python -m whisper
if [[ -x "$WHISPER_BIN" ]]; then
  WHISPER_CMD=("$WHISPER_BIN")
else
  WHISPER_CMD=("$WHISPER_PY" -m whisper)
fi

"${WHISPER_CMD[@]}" "$WAV" \
  --model "$MODEL" \
  --output_dir "$WORK" \
  --output_format all \
  --fp16 False \
  >"$LOG/whisper.log" 2>&1

# Whisper names outputs after input stem: audio.txt / audio.srt
RAW_TXT="$WORK/audio.txt"
RAW_SRT="$WORK/audio.srt"
if [[ ! -f "$RAW_TXT" ]]; then
  echo "ERROR: Whisper did not write audio.txt; see $LOG/whisper.log" >&2
  exit 1
fi

# Detect language from whisper log if present
LANG_CODE="und"
if rg -q "Detected language:" "$LOG/whisper.log" 2>/dev/null; then
  LANG_CODE="$(rg -o "Detected language:[[:space:]]*[A-Za-z]+" "$LOG/whisper.log" | head -1 | awk '{print tolower($NF)}' | cut -c1-2)"
fi
# If user forced via WHISPER_LANGUAGE
if [[ -n "${WHISPER_LANGUAGE:-}" ]]; then
  LANG_CODE="$WHISPER_LANGUAGE"
fi
# Default und → en when SRT exists and no detection (caller may override)
if [[ "$LANG_CODE" == "und" ]]; then
  LANG_CODE="en"
fi

OUT_TXT="$WORK/01-transcript.${LANG_CODE}.txt"
OUT_SRT="$WORK/01-transcript.${LANG_CODE}.srt"

# Prefer SRT → bracket timestamp text; else keep Whisper txt and require SRT
if [[ -f "$RAW_SRT" ]]; then
  cp "$RAW_SRT" "$OUT_SRT"
  python3 - "$RAW_SRT" "$OUT_TXT" <<'PY'
import re, sys
srt_path, out_path = sys.argv[1], sys.argv[2]
text = open(srt_path, encoding="utf-8").read()
blocks = re.split(r"\n\s*\n", text.strip())
lines = []
for b in blocks:
    parts = b.strip().splitlines()
    if len(parts) < 2:
        continue
    # find time line
    tline = next((p for p in parts if "-->" in p), None)
    if not tline:
        continue
    m = re.match(
        r"(\d{2}:\d{2}:\d{2},\d{3})\s*-->\s*(\d{2}:\d{2}:\d{2},\d{3})",
        tline.strip(),
    )
    if not m:
        continue
    start = m.group(1).replace(",", ".")[:12]
    end = m.group(2).replace(",", ".")[:12]
    # trim to HH:MM:SS.mmm-ish → HH:MM:SS
    def short(t: str) -> str:
        return t.split(".")[0] if "." in t else t
    body = " ".join(parts[parts.index(tline) + 1 :]).strip()
    if body:
        lines.append(f"[{short(start)} --> {short(end)}] {body}")
open(out_path, "w", encoding="utf-8").write("\n".join(lines) + ("\n" if lines else ""))
PY
else
  cp "$RAW_TXT" "$OUT_TXT"
  echo "WARN: no SRT; copied prose txt — Phase 1 may fail timestamp check" >&2
fi

META="$WORK/meta.json"
python3 - "$META" "$LANG_CODE" "$MODEL" "$SRC" <<'PY'
import json, sys
path, lang, model, src = sys.argv[1:5]
json.dump(
    {"language": lang, "model": model, "source_url": src},
    open(path, "w", encoding="utf-8"),
    ensure_ascii=False,
    indent=2,
)
print()
PY

echo "OK transcript=$OUT_TXT"
[[ -f "$OUT_SRT" ]] && echo "OK srt=$OUT_SRT"
echo "OK meta=$META"
echo "OK logs=$LOG"
