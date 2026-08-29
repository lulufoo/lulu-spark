# Archive conventions (dialogue-archive)

## Sink scope

| `sink` | Corpus / MCP |
|--------|----------------|
| `workbench` | This document applies in full |
| `local-md` | Skip MCP; write workspace `.cache` only — do **not** write notes `raw/` / `digest/` / `index.json` by hand |

## MCP prerequisite (`sink=workbench` only)

Workbench App **must** be running (`workbench-knowledge` at `http://127.0.0.1:9876/mcp`).  
**Do not** write `raw/` / `digest/` / `index.json` directly. Use MCP only.

| Op | MCP tool |
|----|----------|
| Write raw + index | `create_note` |
| Write digest + layers | `create_note_digest` |

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
| normalize / local-md | `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md` |

- `project`: closest topics match; else `inbox`
- `doc-theme` / `slug`: kebab-case
- `ts`: `YYYYMMDDHHMM` (UTC+8 archive time)

## Raw header (verbatim producer — script)

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

## create_note

Hard-cut: body comes from Host reading a file. **Do not** send `document`.

```json
{
  "source_path": "<absolute path to normalized .md>",
  "source_type": "dialogue"
}
```

If the tool/API still rejects `source_path` or requires `document`, stop and report Host/MCP not yet upgraded — do **not** paste the full markdown into the tool call.

## create_note_digest

MCP fields unchanged: `id` + `digest` (+ optional `force`).

Writing contract: non-empty **内容约束** must appear in the digest header:

```markdown
> 内容约束：<content_constraint>
```

Do not re-load full raw solely to draft the digest.
