# Archive Concepts (shared)

> 供 `theme-archive`、`theme-fetch`、`theme-line`、`dialogue-summary`、`dialogue-archive` 共用的路径约定（digest 规则见 `digest-workflow.md`；对话 jsonl 清洗见 `transcript-clean.md`）。

---

## MCP Prerequisite

<HARD-GATE mcp="archive">
Workbench App **必须运行**（MCP `workbench-knowledge` 可用，`http://127.0.0.1:9876/mcp`）。不可用 → **明确报错并停止**。

**禁止** Agent 直写 notes 文件系统（raw / digest / index.json）。落盘 MUST 经 MCP：

| 操作 | MCP tool |
|------|----------|
| 写 raw + index | `create_note`（**仅** `source_path`；禁止 `document` 正文） |
| 写 digest + layers | `create_note_digest`（`id` + `digest`；写作须含内容约束，见 digest-workflow） |
| 读 digest 目录 | `get_notes_catalog` / `get_notes_files` |
</HARD-GATE>

`create_note` 合同：先将 Markdown 落到 allow-list 绝对路径（建议 `{workspace}/.cache/…`），再传：

```json
{ "source_path": "<abs.md>", "source_type": "<summary|dialogue|article|…>" }
```

各 skill 首步确认：`> ✅ Workbench MCP 可用`

---

## Path symbols

```
COMMON_PATH = <topic-path>/<ts>-<slug>.md
topic-path  = <project>/<doc-theme>    # 始终 2 段
prefix      = "../../../"              # raw/ 文件内导航相对前缀
```

| 目录 | 路径模式（均在 workbench store 的 `notes/` 下） |
|------|---------|
| raw | `notes/raw/<topic-path>/<ts>-<slug>.md` |
| raw (zh) | `notes/raw/<topic-path>/<ts>-<slug>-zh.md` |
| digest | `notes/digest/<COMMON_PATH>` |
| debug bundle | `.cache/<topic-path>/<ts>-<slug>-bundle.json`（可选调试；禁止 Agent 直写） |

**禁止** 将 TranscriptBundle 或其它中间产物写入 notes。

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

---

## Navigation line

**MCP 产出者**（`theme-archive`、`theme-line`、`dialogue-summary`、`dialogue-archive`、`theme-fetch`）raw header **仅含 digest 链接**（Workbench `create_note` 解析要求）：

```markdown
> 导航：[digest]({prefix}digest/{COMMON_PATH})
```

digest 文件导航行仅链回 raw。`prefix` 仍是 `../../../`（`notes/raw` 与 `notes/digest` 为兄弟目录）。

---

## Slug & timestamp

```
ts   = YYYYMMDDHHMM (UTC+8)
slug = kebab-case summary of source title (English, no spaces)
```

slug 冲突 → 与用户确认后再继续。

---

## Entry ID

32 字符小写 hex：由 `create_note` 返回；Agent **不**自行生成。

---

## References

- theme-archive（raw + 全文英文默认中译 + 自动 digest）：[../theme-archive/SKILL.md](../theme-archive/SKILL.md)
- theme-line archive 步骤：[../theme-line/references/archive-steps.md](../theme-line/references/archive-steps.md)
- digest workflow（shared）：[digest-workflow.md](digest-workflow.md)
