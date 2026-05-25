# Plain-Text Adapter

> `platform`: `plain` · `adapter`: `plain@v1`

---

## Match

- 本地文件路径（`.srt`, `.vtt`, `.md`, `.txt`）
- 用户粘贴的 transcript 文本
- 未知 URL（Phase 0 无法识别平台）
- 其他 adapter 降级路径

---

## Acquire

| Input | Action |
|-------|--------|
| `.srt` / `.vtt` | 读取文件内容 |
| `.md` / `.txt` | 读取全文作为 transcript |
| 用户粘贴 | 直接使用粘贴文本 |
| 无 URL | `source.url: null` |

---

## Map

产出**最小** TranscriptBundle：

```yaml
source:
  platform: plain
  url: null          # 或用户提供的 URL
  adapter: plain@v1
  fetched_at: "<ISO8601>"

meta:
  title: "<用户标题或文件名推断>"
  speakers: []       # 由 Compose 推断
  duration_sec: null
  published_at: null
  language: unknown    # 或由正文检测

fidelity:
  level: mixed
  corrections: []

segments: []           # 通常为空

utterances:
  - start_sec: 0
    text: "<parsed or pasted content>"
    speaker: null

assets:
  subtitle_urls: {}
```

- SRT/VTT → 解析为 utterances 数组
- 纯文本 → 单条 utterance 或按段落拆分

Compose 策略：通常 `flat-caption`（`segments==0, utterances>0`）；若无法解析 → fail-fast

---

## Quirks

| 场景 | 处理 |
|------|------|
| 空输入 | 中止；请用户提供内容 |
| 无法识别格式 | 作为单块 plain text utterance |
| 从 InfoQ/YouTube 降级 | 保留原 URL 于 `source.url`（若已知） |
