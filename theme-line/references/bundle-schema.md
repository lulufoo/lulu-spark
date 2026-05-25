# TranscriptBundle Schema

> **Version:** `schema_version: 1`
>
> Phase 1 (Acquire) 输出、Phase 2 (Compose) 输入的唯一中间契约。内存 ephemeral；**禁止**写入 `trace/`。可选调试落盘：`{archive_root}/.cache/{topic-path}/{ts}-{slug}-bundle.json`。

---

## Top-level shape

```yaml
schema_version: 1          # required

source:                    # required — 采集来源
meta:                      # required — 展示与归档元数据
fidelity:                  # required — 文本保真度与 ASR 校正
segments: []               # optional — 结构化章节（InfoQ manuscripts/outlines）
utterances: []             # optional — 逐句字幕/转写
assets:                    # optional — 原始字幕 URL
```

---

## `source`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `platform` | `"youtube"` \| `"infoq"` \| `"plain"` | yes | Phase 0 Resolve 结果 |
| `url` | `string` \| `null` | yes | 原始 URL；plain 源可为 `null` |
| `adapter` | `string` | yes | 版本字符串，格式 `{platform}@v1`（如 `infoq@v1`） |
| `fetched_at` | ISO8601 string | yes | 采集时间 |

---

## `meta`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | `string` | yes | 文档标题（优先源标题） |
| `speakers` | `string[]` | yes | 已知嘉宾/讲者 canonical 名列表 |
| `duration_sec` | `int` \| `null` | no | 视频时长（秒）；未知为 `null` |
| `published_at` | `string` \| `null` | no | 发布日期 `YYYY-MM-DD` |
| `language` | `"zh"` \| `"en"` \| `"mixed"` \| `"unknown"` | yes | 主语言；驱动 Phase 3 翻译分支 |

### `meta.language` → Phase 3 翻译规则

| `language` | Archive 行为 |
|------------|--------------|
| `en` | 英文源文件 + 生成 `-zh.md` 翻译 |
| `zh` | 仅中文源文件，无 `-zh.md` |
| `mixed` | 仅源文件，无 `-zh.md` |
| `unknown` | 仅源文件，无 `-zh.md` |

---

## `fidelity`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `level` | `"verbatim"` \| `"segment_summary"` \| `"mixed"` | yes | 整体保真级别 |
| `corrections` | `{wrong, correct}[]` | yes | ASR 谐音校正表；可为 `[]` |

### `fidelity.corrections`

```yaml
corrections:
  - wrong: "蒋林全"      # ASR 误识别 token
    correct: "蒋林泉"     # canonical speaker 名
```

**规则：**

- 结构固定为 `{wrong: string, correct: string}`
- **禁止** adapter 硬编码 per-video 人名表
- 由 runtime 从 `meta.speakers` 在 utterances 文本中推导（见 adapter Map / compose C1）

---

## `segments[]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `start_sec` | `number` | yes | 章节起始秒 |
| `end_sec` | `number` | no | 章节结束秒 |
| `title` | `string` | no | 章节标题 |
| `summary` | `string` | no | 章节摘要（summary-only 策略锚点） |
| `speaker` | `string` | no | 关联讲者 |

---

## `utterances[]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `start_sec` | `number` | yes | 起始秒 |
| `end_sec` | `number` | no | 结束秒 |
| `text` | `string` | yes | 转写文本 |
| `speaker` | `string` \| `null` | no | 说话人；未知为 `null` |

---

## `assets`

```yaml
assets:
  subtitle_urls:
    srt: "https://..."    # optional
    vtt: "https://..."    # optional
```

---

## Minimal examples

### InfoQ（segment-seeded：`segments>0` AND `utterances>0`）

```json
{
  "schema_version": 1,
  "source": {
    "platform": "infoq",
    "url": "https://www.infoq.cn/video/iCkjD5xFrVSN3eijYq0e",
    "adapter": "infoq@v1",
    "fetched_at": "2026-05-25T10:00:00+08:00"
  },
  "meta": {
    "title": "InfoQ 访谈示例",
    "speakers": ["蒋林泉", "龚银"],
    "duration_sec": 2700,
    "published_at": "2026-01-15",
    "language": "zh"
  },
  "fidelity": {
    "level": "mixed",
    "corrections": []
  },
  "segments": [
    {
      "start_sec": 0,
      "title": "开场",
      "summary": "嘉宾介绍与主题概述"
    }
  ],
  "utterances": [
    {
      "start_sec": 12,
      "end_sec": 18,
      "text": "今天我们聊聊…",
      "speaker": null
    }
  ],
  "assets": {
    "subtitle_urls": {
      "srt": "https://example.com/subtitle.srt"
    }
  }
}
```

### YouTube flat（flat-caption：`segments==0` AND `utterances>0`）

```json
{
  "schema_version": 1,
  "source": {
    "platform": "youtube",
    "url": "https://www.youtube.com/watch?v=cdiD-9MMpb0",
    "adapter": "youtube@v1",
    "fetched_at": "2026-05-25T10:00:00+08:00"
  },
  "meta": {
    "title": "YouTube Talk Example",
    "speakers": ["Host", "Guest"],
    "duration_sec": 2160,
    "published_at": "2025-01-21",
    "language": "en"
  },
  "fidelity": {
    "level": "verbatim",
    "corrections": []
  },
  "segments": [],
  "utterances": [
    {
      "start_sec": 0,
      "end_sec": 5,
      "text": "Welcome to the show.",
      "speaker": null
    }
  ],
  "assets": {
    "subtitle_urls": {}
  }
}
```

---

## Conformance

- 所有 adapter Map 输出 MUST conform 本 schema
- Compose MUST 读 bundle 形状选策略；Archive 只读 `bundle.meta`（不读 adapter 字段）
- 本文件不含平台 fetch 命令（见 `adapters/{platform}.md`）
