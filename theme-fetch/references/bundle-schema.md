# ArticleBundle Schema

> **Version:** `schema_version: 1`
>
> Phase 1 (Acquire) 输出、Phase 2 (Format) 输入的唯一中间契约。内存 ephemeral。可选调试落盘：`{workspace}/.cache/theme-fetch/{ts}-{slug}-bundle.json`。

---

## Top-level shape

```yaml
schema_version: 1          # required

source:                    # required — 采集来源
meta:                      # required — 展示与归档元数据
content:                   # required — 正文载体
```

---

## `source`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `platform` | `string` | yes | Phase 0 Resolve 结果（如 `wechat`） |
| `url` | `string` \| `null` | yes | 原始 URL；本地 HTML 可为 `null` |
| `adapter` | `string` | yes | `{platform}@v1` |
| `fetched_at` | ISO8601 string | yes | 采集时间 |

---

## `meta`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | `string` | yes | 文章标题 |
| `author` | `string` \| `null` | no | 作者 |
| `publisher` | `string` \| `null` | no | 刊载方（公众号名、站点名） |
| `published_at` | `string` \| `null` | no | `YYYY-MM-DD` 或 `YYYY-MM-DD HH:MM` |
| `language` | `"zh"` \| `"en"` \| `"mixed"` \| `"unknown"` | yes | 采集元数据。**不**驱动翻译。 |

### `meta.language` → Phase 3 翻译规则

| `language` | Archive 行为 |
|------------|--------------|
| `en` | 英文源 + 可选 `-zh.md` |
| `zh` | 仅中文源 |
| `mixed` / `unknown` | 仅源文件 |

---

## `content`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `blocks` | `Block[]` | yes | 结构化块；可为 `[]` 若使用 `markdown_raw` |
| `markdown_raw` | `string` \| `null` | no | 跳过多步 Format 时的预格式化正文 |

### `Block`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | `"paragraph"` \| `"heading"` \| `"table"` \| `"list"` \| `"code"` \| `"image"` \| `"quote"` | yes | 块类型 |
| `level` | `number` \| `null` | no | heading 层级 1–4 |
| `text` | `string` \| `null` | no | 段落/标题/quote 文本 |
| `items` | `string[]` | no | list 条目 |
| `ordered` | `boolean` | no | list 是否有序 |
| `rows` | `string[][]` | no | table 行（首行可为表头） |
| `lang` | `string` | no | code 块语言 |
| `url` | `string` \| `null` | no | image 地址 |

---

## Minimal example (WeChat)

```json
{
  "schema_version": 1,
  "source": {
    "platform": "wechat",
    "url": "https://mp.weixin.qq.com/s/example",
    "adapter": "wechat@v1",
    "fetched_at": "2026-06-02T12:00:00+08:00"
  },
  "meta": {
    "title": "示例标题",
    "author": "王鹏程",
    "publisher": "腾讯云开发者",
    "published_at": "2026-06-02 08:45",
    "language": "zh"
  },
  "content": {
    "blocks": [
      { "type": "paragraph", "text": "开篇段落…" },
      { "type": "heading", "level": 2, "text": "01 章节标题" },
      { "type": "table", "rows": [["列A", "列B"], ["a1", "b1"]] }
    ],
    "markdown_raw": null
  }
}
```

---

## Conformance

- 所有 adapter Map 输出 MUST conform 本 schema
- Format MUST 读 bundle 选策略；Archive 读 `bundle.meta` + Format 输出的 body
- 平台 fetch 命令见 `adapters/{platform}.md`
