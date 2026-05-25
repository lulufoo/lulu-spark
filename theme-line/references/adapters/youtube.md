# YouTube Adapter

> `platform`: `youtube` · `adapter`: `youtube@v1`

---

## Match

- URL 匹配 `youtube.com/watch`, `youtu.be/*`, `youtube.com/shorts/*`

---

## Acquire

1. **Metadata**

```bash
python3 -m yt_dlp --print "%(title)s" --print "%(duration)s" --print "%(upload_date)s" --no-download "{url}"
```

2. **Subtitles**

```bash
python3 -m yt_dlp --write-auto-sub --sub-lang en,zh-Hans,zh --skip-download -o "/tmp/yt-%(id)s" "{url}"
```

**Caption 回退顺序：** `en` → `zh-Hans` → `zh` → 手动粘贴（降级 plain）

列出可用字幕：

```bash
python3 -m yt_dlp --list-subs "{url}"
```

---

## Map

### `meta`

| Field | Source |
|-------|--------|
| `title` | yt-dlp title |
| `speakers` | `["Host", "Guest"]`（推断上限；非 diarization） |
| `duration_sec` | yt-dlp duration |
| `published_at` | `upload_date` → `YYYY-MM-DD` |
| `language` | 字幕语言检测 → `en` / `zh` / `mixed` / `unknown` |

### `segments`

- **恒空** `[]`（YouTube auto-sub 无结构化章节）

### `utterances`

- 解析 VTT/SRT → `{start_sec, end_sec, text, speaker: null}`
- 说话人推断在 Compose `flat-caption` 阶段完成；上限 Host/Guest + `(uncertain)`

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

Compose 策略：`segments==0 AND utterances>0` → `flat-caption`

---

## Quirks

| 场景 | 处理 |
|------|------|
| yt-dlp 无字幕 | 降级 `plain`；请用户粘贴 transcript |
| 仅 auto-generated 字幕 | 正常使用；标注 `(uncertain)` 说话人 |
| 视频不可用 / geo-block | 降级 `plain`；告知用户 |
