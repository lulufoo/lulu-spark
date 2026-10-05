# InfoQ Adapter

> `platform`: `infoq` · `adapter`: `infoq@v1`

---

## Match

- URL 匹配 `https://www.infoq.cn/video/{uuid}` 或 `https://www.infoq.cn/article/{uuid}`
- 提取 `uuid` 作为 API 参数

样例：`iCkjD5xFrVSN3eijYq0e`

---

## Acquire

Do **not** download the video file. Do **not** run Whisper.

1. **Article detail API**

```bash
curl -s -X POST "https://www.infoq.cn/public/v1/article/getDetail" \
  -H "Content-Type: application/json" \
  -H "User-Agent: Mozilla/5.0" \
  -H "Referer: https://www.infoq.cn/video/{uuid}" \
  -d '{"uuid":"{uuid}"}'
```

预期：`code=0`，`data.article_title` 非空

2. **SRT subtitle**

- 从响应 `data.video.ai_subtitle`（或等效字段）获取 SRT URL
- GET 下载 SRT 文本（字幕文件，不是媒体）

---

## Map

### `meta`

| Field | Source |
|-------|--------|
| `title` | `data.article_title` |
| `speakers` | `data.no_author` 或 manuscripts；再按 [speaker-roster.md](../speaker-roster.md) 补全 |
| `duration_sec` | `data.video.duration`（秒） |
| `published_at` | `data.publish_time` → `YYYY-MM-DD` |
| `language` | 正文检测 → `zh` / `en` / `mixed` / `unknown` |

### `segments`（manuscripts + outlines）

导航标记 only：

1. 合并 `data.video.manuscripts` 与 `data.video.outlines`
2. 按 `start_sec` 去重
3. **同 `start_sec` 冲突时 manuscripts 优先**；outlines 仅补充未覆盖的 `start_sec`
4. Compose 只用 `title` / `start_sec` 做 heading，**不用** `summary` 当正文

### `utterances`（SRT）

- 解析 SRT → `{start_sec, end_sec, text, speaker: null}`
- SRT 不可用 → `utterances: []` → Compose fail-fast

### `fidelity.corrections`（runtime-derived）

**禁止** hardcode per-video 人名表。算法：

1. 对每个 `speaker` ∈ `meta.speakers`，在 utterances 合并文本中搜索 2–3 字中文片段，匹配 speaker 名子串但不在 speakers 列表的 token → 候选 `wrong`
2. 候选与某 speaker 共享首字且编辑距离 ≤2（或 Levenshtein 归一化 ≤0.5）→ 关联 `correct`
3. 同一 `wrong` 出现 ≥3 次 → 写入 `{wrong, correct}`；不确定的不写入

### `assets`

```yaml
subtitle_urls:
  srt: "<SRT URL from API>"
```

### `source`

```yaml
platform: infoq
url: "https://www.infoq.cn/video/{uuid}"
adapter: infoq@v1
fetched_at: "<ISO8601>"
```

---

## Quirks

| 场景 | 处理 |
|------|------|
| API `code != 0` 或空 `data` | 降级 `plain`；请用户粘贴 transcript |
| SRT 403 / 网络失败 | `utterances: []`；Compose **fail-fast**。可请用户粘贴 SRT，或改走 `theme-transcribe` |
| manuscripts 与 outlines 均为空 | `segments: []`；有 utterances 仍走 `complete-dialogue` |

Compose：`utterances>0` → `complete-dialogue`。
