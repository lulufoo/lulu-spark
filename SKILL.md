---
name: lulu-workbench-skills
description: >-
  lulu-workbench 归档技能包安装。克隆到平台 skills 目录（Cursor / Copilot / Claude / Codex）；归档经 Workbench App MCP 落盘。
  Use when: 安装 workbench skills、dialogue-summary dialogue-archive theme-line theme-fetch theme-transcribe theme-archive todo-task
---

# lulu-workbench-skills — 安装

## Platform Context

**Detect once; substitute `$SKILL_DIR` throughout:**

| | Cursor | Copilot | Claude | Codex |
|---|---|---|---|---|
| `$SKILL_DIR` | `~/.cursor/skills/lulu-workbench-skills` | `~/.copilot/skills/lulu-workbench-skills` | `~/.claude/skills/lulu-workbench-skills` | `~/.agents/skills/lulu-workbench-skills` |

检测信号（优先级）：`CURSOR_AGENT` → Cursor · `COPILOT_AGENT` / `VSCODE_TARGET_SESSION_LOG` → Copilot · `CLAUDE_CODE` → Claude · `CODEX_AGENT` → Codex · 均无 → 向用户确认平台后再替换 `$SKILL_DIR`。

## 安装

首次克隆到平台 skills 目录：

```bash
git clone https://github.com/lulufoo/lulu-workbench-skills.git $SKILL_DIR
```

更新：

```bash
git -C $SKILL_DIR pull --rebase
```

克隆完成后平台自动发现子 skill（`dialogue-summary`、`dialogue-archive`、`theme-line`、`theme-fetch`、`theme-transcribe`、`theme-archive`、`todo-task`），均无需额外操作。digest 为 `shared/digest-workflow` shared 契约，不单独发现。

`dialogue-summary` 清洗脚本见包内 [`scripts/transcript-clean-control.py`](scripts/transcript-clean-control.py)；`dialogue-archive` 使用 workbench 仓库 `scripts/dialogue_archive_normalize.py` + MCP `source_path`。编排见 [`shared/dialogue-execution.md`](shared/dialogue-execution.md)。

## 前置条件

归档 skill 执行前 **Workbench App 必须运行**（MCP `workbench-knowledge` 可用，`http://127.0.0.1:9876/mcp`）。corpus 根目录由 Workbench 管理，**无需**本地配置文件。

## 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `dialogue-summary` | [dialogue-summary/](dialogue-summary/) | 自包含总结：覆盖面随对话、单元丰富度固定、忠实整合不灌水 + `〔User〕` → MCP 归档（`dtd_raw_summary`；已定稿落盘用 `theme-archive`） |
| `dialogue-archive` | [dialogue-archive/](dialogue-archive/) | 节点切片脚本 + **theme-archive** 原文归档；`sink=local-md` 仅 `.cache` |
| `theme-line` | [theme-line/](theme-line/) | 视频/访谈稿 → ThemeLine 组稿 → **theme-archive**（全文英文才中译 + digest） |
| `theme-fetch` | [theme-fetch/](theme-fetch/) | 网页文章组稿 → **theme-archive** |
| `theme-transcribe` | [theme-transcribe/](theme-transcribe/) | STT → 子话题 → 流畅性 → **theme-archive**；独立于 `theme-line` |
| `theme-archive` | [theme-archive/](theme-archive/) | 统一录入：raw + 全文英文默认中译 + 自动 digest |
| `todo-task` | [todo-task/](todo-task/) | Todo 任务树 CRUD（MCP `todo_*` tools；经 local_http proxy） |

## 验收

触发任一子 skill 时，首步确认 Workbench MCP 可用；归档成功后 MCP 返回 `id` / `common_path` / `raw_path`（或 digest 路径）。

共享规范：[archive-concepts](shared/archive-concepts.md)。digest 为shared 契约：[shared/digest-workflow.md](shared/digest-workflow.md)（由 theme-archive 自动调用；勿作公开 skill）。
