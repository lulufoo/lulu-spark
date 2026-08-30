# Archive Concepts (shared)

> 供 `theme-archive`、`theme-fetch`、`theme-line`、`dialogue-summary`、`dialogue-archive` 共用的路径约定（digest 规则见 `digest-workflow.md`；对话 jsonl 清洗见 `transcript-clean.md`）。

---

## MCP Prerequisite

<HARD-GATE mcp="archive">
Workbench App **必须运行**（MCP `workbench-knowledge` 可用，`http://127.0.0.1:9876/mcp`）。不可用 → **明确报错并停止**。

**禁止** Agent 直写 notes 文件系统（raw / digest / index.json）。落盘 MUST 经 MCP：

| 操作 | MCP tool |
|------|----------|
| 写 raw + index | `create_note`（`source_path` + `title`；禁止 `document`） |
| 写 digest + layers | `create_note_digest`（`id` + `digest`；写作须含内容约束，见 digest-workflow） |
| 读 digest 目录 | `get_notes_catalog` / `get_notes_files` |
</HARD-GATE>

`create_note` 合同：先将 Markdown 落到 allow-list 绝对路径（建议 `{workspace}/.cache/…`），再传：

```json
{
  "source_path": "<abs.md>",
  "title": "<display title>",
  "project": "<optional; default inbox>",
  "theme": "<optional; default notes>",
  "created_at": "<optional YYYYMMDDHHMM UTC+8>",
  "source_type": "<summary|article|theme-line|dialogue|transcript|jot>"
}
```

Host 分配 `common_path`：`{project}/{theme}/{created_at}-{6 alnum}-{source basename}`。不要再拼 `{ts}-{slug}`，不要发送 `document`。

各 skill 首步确认：`> ✅ Workbench MCP 可用`

---

## Path symbols

```
COMMON_PATH = <project>/<theme>/<created_at>-<6 alnum>[-<source-basename>].md
```

| 目录 | 路径模式（均在 workbench store 的 `notes/` 下） |
|------|---------|
| raw | `notes/raw/<COMMON_PATH>` |
| raw (zh) | `notes/raw/<stem>-zh.md` |
| digest | `notes/digest/<COMMON_PATH>` |
| debug bundle | `.cache/…`（可选调试；禁止 Agent 直写 notes） |

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

## Raw shell

Host 写入 raw 时只保留：

```markdown
# {title}

> 创建时间：{YYYY年M月D日 HH:MM}

---

{body}
```

不要要求、也不要依赖 digest 导航行。Digest 与 raw 通过同一 `common_path` 和 `layers` 关联。

---

## Timestamp

```
created_at = YYYYMMDDHHMM (UTC+8)
```

文件名中的 6 位随机段由 Host 生成，用于避免同秒冲突。

---

## Entry ID

32 字符小写 hex：由 `create_note` 返回；Agent **不**自行生成。

---

## References

- theme-archive（raw + 全文英文默认中译 + 自动 digest）：[../theme-archive/SKILL.md](../theme-archive/SKILL.md)
- theme-line archive 步骤：[../theme-line/references/archive-steps.md](../theme-line/references/archive-steps.md)
- digest workflow（shared）：[digest-workflow.md](digest-workflow.md)
