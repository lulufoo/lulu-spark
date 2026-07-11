---
name: lulu-workbench-skills
description: >-
  lulu-workbench 归档技能包安装。克隆到平台 skills 目录（Cursor / Copilot / Claude / Codex）；归档经 Workbench App MCP 落盘。
  Use when: 安装 workbench skills、dialogue-summary dialogue-archive theme-summary theme-line theme-fetch theme-archive theme-digest plan-task
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

克隆完成后平台自动发现子 skill（`dialogue-summary`、`dialogue-archive`、`theme-summary`、`theme-line`、`theme-fetch`、`theme-archive`、`theme-digest`、`plan-task`），均无需额外操作。

## 前置条件

归档 skill 执行前 **Workbench App 必须运行**（MCP `workbench-knowledge` 可用，`http://127.0.0.1:9876/mcp`）。corpus 根目录由 Workbench 管理，**无需**本地配置文件。

## 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `dialogue-summary` | [dialogue-summary/](dialogue-summary/) | 过程回顾：骨架脉络 + 核心加深 + `〔User〕` → MCP 归档（`dtd_raw_summary` / 过程回顾；替代旧 theme-summary 生成定位） |
| `dialogue-archive` | [dialogue-archive/](dialogue-archive/) | 对话逐轮原文归一化归档（原 `dialogue-summary` 逐字；`dtd_raw_dialogue`） |
| `theme-summary` | [theme-summary/](theme-summary/) | **Legacy：** 已写好的总结原文落盘 + digest；过程性整理请用 `dialogue-summary` |
| `theme-line` | [theme-line/](theme-line/) | 多平台视频/访谈稿（YouTube、InfoQ、plain）→ TranscriptBundle → 主题优先时间线大纲，保存至 `raw/` 并自动 digest |
| `theme-fetch` | [theme-fetch/](theme-fetch/) | 多平台网页文章（WeChat、plain HTML…）→ ArticleBundle → 格式化 Markdown；Phase 3 经 MCP 落盘 + digest |
| `theme-archive` | [theme-archive/](theme-archive/) | 文档落盘 `raw/` + 更新 `index.json`（MCP；不触发 digest） |
| `theme-digest` | [theme-digest/](theme-digest/) | 从 raw 生成或补跑 `digest/`（MCP）；亦可独立调用 |
| `plan-task` | [plan-task/](plan-task/) | Plan 任务树 CRUD（8 个 MCP plan tools；经 local_http proxy） |

## 验收

触发任一子 skill 时，首步确认 Workbench MCP 可用；归档成功后 MCP 返回 `id` / `common_path` / `raw_path`（或 digest 路径）。

共享规范：[archive-concepts](shared/archive-concepts.md)。digest 执行见 [theme-digest/SKILL.md](theme-digest/SKILL.md)。
