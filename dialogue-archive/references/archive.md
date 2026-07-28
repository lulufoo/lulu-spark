# Archive conventions (dialogue-archive)

## Sink scope

| `sink` | Corpus / MCP |
|--------|----------------|
| `workbench` | This document applies in full |
| `local-md` | Skip MCP; write workspace `.cache` only — do **not** write corpus `raw/` / `digest/` / `index.json` by hand |

## MCP prerequisite (`sink=workbench` only)

Workbench App **must** be running (`workbench-knowledge` at `http://127.0.0.1:9876/mcp`).  
**Do not** write `raw/` / `digest/` / `index.json` directly. Use MCP only.

| Op | MCP tool |
|----|----------|
| Write raw + index | `archive_document` |
| Write digest + layers | `archive_digest` |

First step when archiving to Workbench: confirm MCP available.

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
| local-md | `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md` |

- `project`: closest topics match; else `inbox`
- `doc-theme` / `slug`: kebab-case
- `ts`: `YYYYMMDDHHMM` (UTC+8 archive time)

## Raw header (verbatim producer)

`sink=workbench`:

```markdown
# <Title>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：dialogue-archive
> 导航：[digest](../../../digest/<COMMON_PATH>)

---

<!-- DDM:TURN_SEP:v1 -->
…turn body…
```

`sink=local-md`: omit the digest navigation line (`to-archive-md --omit-digest-nav`).

## archive_document

```json
{
  "document": "<full markdown>",
  "source_type": "dialogue"
}
```
