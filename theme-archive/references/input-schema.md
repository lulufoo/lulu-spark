# Embedded Input Schema

> Producer 组稿完成后链式调用 theme-archive（从 `[AR-1]`）。翻译由 theme-archive `[AR-1b]` 决定，不在本载荷里传入。

## Required fields

```yaml
title: "<display title>"
source_type: "summary | article | theme-line | dialogue | transcript | jot"
primary_path: "<absolute path to header+body .md under .cache>"
project: "<optional; default inbox>"
theme: "<optional; default notes>"
created_at: "<optional YYYYMMDDHHMM UTC+8>"
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
| theme-transcribe | `dialogue` or `transcript` (from route stdout) |
| dialogue-summary | `summary` |
| dialogue-archive | `dialogue` |

## Call pattern

```text
Phase N 组稿完成（仅主文件）→
加载并完整执行 ../theme-archive/SKILL.md（Embedded，从 [AR-1] 起）
```

Producer 负责：选 project/theme、确定 title、Compose 主文件。  
theme-archive 负责：全文英文中译、`create_note`（含 `title`）、`[AR-3]` digest。
