# YouTube Adapter

> `platform`: `youtube` · `adapter`: `youtube@v1`

---

## Match

- URL 匹配 `youtube.com/watch`, `youtu.be/*`, `youtube.com/shorts/*`

---

## Acquire

Do **not** download audio/video. Do **not** run Whisper.

1. **Metadata**

```bash
python3 -m yt_dlp --print "%(title)s" --print "%(duration)s" --print "%(upload_date)s" --no-download "{url}"
```

If this fails, keep going for captions. Title may stay as the source page title or URL; `published_at` may be `null`; `duration_sec` may come from the last caption timestamp.

2. **Subtitles (yt-dlp)**

```bash
python3 -m yt_dlp --list-subs "{url}"
python3 -m yt_dlp --write-auto-sub --sub-lang en,zh-Hans,zh --skip-download -o "/tmp/yt-%(id)s" "{url}"
```

Prefer a **manual** track over auto-generated when both exist.

**Caption 回退顺序：** 人工 `en*` → `zh-Hans` → `zh` → 自动 `en` → 下一步

3. **Subtitles (API fallback)** when yt-dlp has no file or errors (`SABR`, reload, geo):

```bash
python3 -c "from youtube_transcript_api import YouTubeTranscriptApi; t=YouTubeTranscriptApi(); print([ (x.language_code, x.is_generated) for x in t.list('{id}') ]); print(t.fetch('{id}', languages=['en','en-IN','en-US','zh-Hans','zh']).to_raw_data()[:2])"
```

Map each caption to `{start_sec, end_sec, text, speaker: null}`. Prefer non-generated tracks.

4. Still no utterances → 降级 `plain`；请用户粘贴 transcript。不要改走 Whisper。

---

## Map

### `meta`

| Field | Source |
|-------|--------|
| `title` | yt-dlp title；失败则页面标题或 URL |
| `speakers` | `["Host", "Guest"]`（推断上限；非 diarization） |
| `duration_sec` | yt-dlp duration；失败则最后一条 caption 的时间 |
| `published_at` | `upload_date` → `YYYY-MM-DD`；未知为 `null` |
| `language` | 字幕语言检测 → `en` / `zh` / `mixed` / `unknown` |

### `segments`

- **恒空** `[]`（YouTube captions 无结构化章节）

### `utterances`

- 解析 VTT/SRT/API captions → `{start_sec, end_sec, text, speaker: null}`
- 说话人在 Compose `complete-dialogue` 保守标注；上限 Host/Guest + `(uncertain)`

### `fidelity`

```yaml
level: verbatim
corrections: []
```

### `source`

```yaml
platform: youtube
url: "{url}"
adapter: youtube@v1
fetched_at: "<ISO8601>"
```

Compose：`utterances>0` → `complete-dialogue`；否则 fail-fast。

---

## Quirks

| 场景 | 处理 |
|------|------|
| yt-dlp 失败或无字幕 | API fallback；仍无则降级 `plain`，请用户粘贴 |
| 仅 auto-generated 字幕 | 正常使用；说话人标 `(uncertain)` |
| 视频不可用 / geo-block | 降级 `plain`；告知用户。**禁止**下载媒体或 Whisper |
