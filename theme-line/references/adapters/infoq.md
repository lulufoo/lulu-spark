# InfoQ Adapter

> `platform`: `infoq` · `adapter`: `infoq@v1`

---

## Match

- URL 匹配 `https://www.infoq.cn/video/{uuid}` 或 `https://www.infoq.cn/article/{uuid}`
- 提取 `uuid` 作为 API 参数

样例：`iCkjD5xFrVSN3eijYq0e`

---

## Acquire

1. **Article detail API**

```bash
curl -s -X POST "https://www.infoq.cn/public/v1/article/getDetail" \
  -H "Content-Type: application/json" \
  -H "User-Agent: Mozilla/5.0" \
  -H "Referer: https://www.infoq.cn/video/{uuid}" \
  -d '{"uuid":"{uuid}"}'
```

预期：`code=0`，`data.article_title` 非空，`data.video.manuscripts` ≥1（正常视频）

2. **SRT subtitle**

- 从响应 `data.video.ai_subtitle`（或等效字段）获取 SRT URL
- GET 下载 SRT 文本

---

## Map

### `meta`

| Field | Source |
|-------|--------|
| `title` | `data.article_title` |
| `speakers` | `data.no_author` 或从 manuscripts 推断 |
| `duration_sec` | `data.video.duration`（秒） |
| `published_at` | `data.publish_time` → `YYYY-MM-DD` |
| `language` | 正文检测 → `zh` / `en` / `mixed` / `unknown` |

### `segments`（manuscripts + outlines）

1. 合并 `data.video.manuscripts` 与 `data.video.outlines`
2. 按 `start_sec` 去重
3. **同 `start_sec` 冲突时 manuscripts 优先**（有 content/summary）；outlines 仅补充 manuscripts 未覆盖的 `start_sec`

### `utterances`（SRT）

- 解析 SRT → `{start_sec, end_sec, text, speaker: null}`
- SRT 不可用 → `utterances: []`（见 Quirks）

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
| SRT 403 / 网络失败 | `utterances: []`；compose 降级 `summary-only` |
| manuscripts 与 outlines 均为空 | `segments: []`；若 utterances 也为空 → compose fail-fast |

Compose 策略：`segments>0 AND utterances>0` → `segment-seeded`；`segments>0 AND utterances==0` → `summary-only`。
