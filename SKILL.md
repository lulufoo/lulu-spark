---
name: lulu-workbench-skills
description: >-
  lulu-workbench 归档技能包安装。克隆到平台 skills 目录；归档经 Workbench App MCP 落盘。
  Use when: 安装 workbench skills、dialogue-summary theme-summary theme-line theme-fetch theme-archive theme-digest
---

# lulu-workbench-skills — 安装

## Platform Context

**Detect once; substitute `$SKILL_DIR` throughout:**

| | Cursor | Copilot |
|---|---|---|
| `$SKILL_DIR` | `~/.cursor/skills/lulu-workbench-skills` | `~/.copilot/skills/lulu-workbench-skills` |

## 安装

首次克隆到平台 skills 目录：

```bash
git clone https://github.com/lulufoo/lulu-workbench-skills.git $SKILL_DIR
```

更新：

```bash
git -C $SKILL_DIR pull --rebase
```

克隆完成后平台自动发现子 skill（`dialogue-summary`、`theme-summary`、`theme-line`、`theme-fetch`、`theme-archive`、`theme-digest`），均无需额外操作。

## 前置条件

归档 skill 执行前 **Workbench App 必须运行**（MCP `workbench-knowledge` 可用，`http://127.0.0.1:9876/mcp`）。corpus 根目录由 Workbench 管理，**无需**本地配置文件。

## 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `dialogue-summary` | [dialogue-summary/](dialogue-summary/) | 对话归一化归档至 `raw/` 并自动 digest（MCP；`dtd_raw_dialogue` 触发词仍可用） |
| `theme-summary` | [theme-summary/](theme-summary/) | 总结归档至 `raw/` 并自动 digest（`dtd_raw_summary`） |
| `theme-line` | [theme-line/](theme-line/) | 多平台视频/访谈稿（YouTube、InfoQ、plain）→ TranscriptBundle → 主题优先时间线大纲，保存至 `raw/` 并自动 digest |
| `theme-fetch` | [theme-fetch/](theme-fetch/) | 多平台网页文章（WeChat、plain HTML…）→ ArticleBundle → 格式化 Markdown；Phase 3 经 MCP 落盘 + digest |
| `theme-archive` | [theme-archive/](theme-archive/) | 文档落盘 `raw/` + 更新 `index.json`（MCP；不触发 digest） |
| `theme-digest` | [theme-digest/](theme-digest/) | 从 raw 生成或补跑 `digest/`（MCP）；亦可独立调用 |

## 验收

触发任一子 skill 时，首步确认 Workbench MCP 可用；归档成功后 MCP 返回 `id` / `common_path` / `raw_path`（或 digest 路径）。

共享规范：[archive-concepts](shared/archive-concepts.md)。digest 执行见 [theme-digest/SKILL.md](theme-digest/SKILL.md)。
