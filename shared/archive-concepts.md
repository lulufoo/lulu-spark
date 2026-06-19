# Archive Concepts (shared)

> 供 `theme-archive`、`theme-fetch`、`theme-line`、`theme-digest`、`dialogue-summary`、`theme-summary` 共用的路径约定。

---

## MCP Prerequisite

<HARD-GATE mcp="archive">
Workbench App **必须运行**（MCP `workbench-knowledge` 可用，`http://127.0.0.1:9876/mcp`）。不可用 → **明确报错并停止**。

**禁止** Agent 直写 corpus 文件系统（raw / digest / index.json）。落盘 MUST 经 MCP：

| 操作 | MCP tool |
|------|----------|
| 写 raw + index | `archive_document` |
| 写 digest + layers | `archive_digest` |
| 读 digest 目录 | `get_corpus_catalog` / `get_corpus_files` |
</HARD-GATE>

各 skill 首步确认：`> ✅ Workbench MCP 可用`

---

## Path symbols

```
COMMON_PATH = <topic-path>/<ts>-<slug>.md
topic-path  = <project>/<doc-theme>    # 始终 2 段
prefix      = "../../../"              # raw/ 文件内导航相对前缀
```

| 目录 | 路径模式 |
|------|---------|
| raw | `raw/<topic-path>/<ts>-<slug>.md` |
| raw (zh) | `raw/<topic-path>/<ts>-<slug>-zh.md` |
| digest | `digest/<COMMON_PATH>` |
| debug bundle | `.cache/<topic-path>/<ts>-<slug>-bundle.json`（可选调试；禁止 Agent 直写） |

**禁止** 将 TranscriptBundle 写入 `trace/`（trace 仅 DDM 认知 trace）。

---

## Project selection

推断 `project`（语义最接近的 topics 项；不清楚 → `inbox`）。**不**读取本地 `topics.json` 文件。

---

## `layers` 顺序

```json
"layers": ["raw"]
```

Archive digest 完成后追加 `"digest"`：

```json
"layers": ["raw", "digest"]
```

**不追加** `"trace"`（theme-line 产出范围）。

---

## Navigation line

**MCP 产出者**（`theme-summary`、`theme-line`、`dialogue-summary`、`theme-fetch`）raw header **仅含 digest 链接**（Workbench `archive_document` 解析要求）：

```markdown
> 导航：[digest]({prefix}digest/{COMMON_PATH})
```

digest 文件导航行：若已知 index 条目 `layers` 含 `distilled` / `trace` 时追加链接。

---

## Slug & timestamp

```
ts   = YYYYMMDDHHMM (UTC+8)
slug = kebab-case summary of source title (English, no spaces)
```

slug 冲突 → 与用户确认后再继续。

---

## Entry ID

32 字符小写 hex：由 `archive_document` 返回；Agent **不**自行生成。

---

## References

- theme-archive（raw + index，不触发 digest）：[../theme-archive/SKILL.md](../theme-archive/SKILL.md)
- theme-line archive 步骤：[../theme-line/references/archive-steps.md](../theme-line/references/archive-steps.md)
- theme-digest workflow：[../theme-digest/SKILL.md](../theme-digest/SKILL.md)
