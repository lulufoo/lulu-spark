# Embedded Input Schema

> Producer 组稿完成后链式调用 theme-archive（从 `[AR-1]`）。翻译由 theme-archive `[AR-1b]` 决定，不在本载荷里传入。

## Required fields

```yaml
COMMON_PATH: "<topic-path>/<ts>-<slug>.md"
source_type: "summary | article | theme-line | dialogue | …"
primary_path: "<absolute path to header+body .md under .cache>"
```

或等价地传入 `documents[0].content`（组稿正文）；落盘仍由 theme-archive 写成 `source_path` 再调 MCP。

## Optional fields

```yaml
skip_digest: false          # true → 跳过 [AR-3]
skip_translate: false       # true → 跳过 [AR-1b]
content_constraint: "…"     # dialogue-* 写 digest 时必填
```

**Forbid:** producer 传入 `translations` / `-zh.md` 正文。

## Producer mapping

| Producer | source_type |
|----------|-------------|
| theme-fetch | `article` |
| theme-line | `theme-line` |
| theme-transcribe | `summary` |
| dialogue-summary | `summary` |
| dialogue-archive | `dialogue` |

## Call pattern

```text
Phase N 组稿完成（仅主文件）→
加载并完整执行 ../theme-archive/SKILL.md（Embedded，从 [AR-1] 起）
```

Producer 负责：选 project/doc-theme、组 header、Compose 主文件。  
theme-archive 负责：全文英文中译、`archive_document`、`[AR-3]` digest。
