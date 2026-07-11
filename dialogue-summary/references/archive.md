# Archive conventions (dialogue-summary)

## MCP prerequisite

Workbench App **must** be running (`workbench-knowledge` at `http://127.0.0.1:9876/mcp`).  
**Do not** write `raw/` / `digest/` / `index.json` directly. Use MCP only.

| Op | MCP tool |
|----|----------|
| Write raw + index | `archive_document` |
| Write digest + layers | `archive_digest` |

First step when archiving: confirm MCP available.

## Paths

```text
COMMON_PATH = <project>/<doc-theme>/<ts>-<slug>.md
topic-path  = <project>/<doc-theme>   # always 2 segments
prefix      = "../../../"
```

| Dir | Pattern |
|-----|---------|
| raw | `raw/<COMMON_PATH>` |
| digest | `digest/<COMMON_PATH>` |

- `project`: closest topics match; else `inbox`
- `doc-theme` / `slug`: kebab-case
- `ts`: `YYYYMMDDHHMM` (UTC+8 archive time)

## Raw header (process / summary producer)

```markdown
# <Title>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：dialogue-summary
> 导航：[digest](../../../digest/<COMMON_PATH>)

---

<body after sanitize; no TURN_SEP; no per-turn User/AI labels>
```

## archive_document

```json
{
  "document": "<full markdown>",
  "source_type": "summary"
}
```
