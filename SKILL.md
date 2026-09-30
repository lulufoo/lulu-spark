---
name: lulu-workbench-skills
description: >-
  lulu-workbench 归档技能包安装。克隆到平台 skills 目录（Cursor / Copilot / Claude / Codex）；归档经 Workbench App MCP 落盘。
  Use when: 安装 workbench skills、dialogue-summary dialogue-archive theme-line theme-fetch theme-transcribe note-task
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

克隆完成后平台自动发现子 skill（`dialogue-summary`、`dialogue-archive`、`theme-line`、`theme-fetch`、`theme-transcribe`、`note-task`），均无需额外操作。digest 写法由 note-task 自己说明，不单独发现。

`dialogue-summary` 清洗脚本见包内 [`scripts/transcript-clean-control.py`](scripts/transcript-clean-control.py)；`dialogue-archive` 切片脚本见包内 [`dialogue-archive/scripts/dialogue_archive_normalize.py`](dialogue-archive/scripts/dialogue_archive_normalize.py)。各 skill 自己编排。

## 前置条件

归档 skill 执行前 **Workbench App 必须运行**（MCP `workbench-knowledge` 可用，`http://127.0.0.1:9876/mcp/cursor_ide`）。notes 根目录由 Workbench 管理，**无需**本地配置文件。

## 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `dialogue-summary` | [dialogue-summary/](dialogue-summary/) | 自包含总结：覆盖面随对话、单元丰富度固定、忠实整合不灌水 + `〔User〕` → 加载 note-task 归档 |
| `dialogue-archive` | [dialogue-archive/](dialogue-archive/) | 节点切片脚本 + **note-task** 原文归档；`sink=local-md` 仅 `.cache` |
| `theme-line` | [theme-line/](theme-line/) | 直接采集字幕/已有稿 → 完整对话组稿 → **note-task** |
| `theme-fetch` | [theme-fetch/](theme-fetch/) | 网页文章组稿 → **note-task** |
| `theme-transcribe` | [theme-transcribe/](theme-transcribe/) | 下载媒体 + Whisper → 完整逐字稿 → **note-task**；无现成字幕时用 |
| `note-task` | [note-task/](note-task/) | 笔记 MCP 地图：`create_note` / catalog / files |

## 验收

触发任一子 skill 时，首步确认 Workbench MCP 可用；归档成功后 MCP 返回 `id` / `common_path` / `raw_path`（或 digest 路径）。

路径公式与 digest 写法由 note-task 自己的 references 说明（随 `create_note` 的 `digest_body`）。
