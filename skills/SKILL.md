---
name: lulu-spark-skills
description: >-
  Lulu Spark 归档技能包安装。克隆到平台 skills 目录（Cursor / Copilot / Claude / Codex）；归档经 Lulu Spark App MCP 落盘。
  Use when: 安装 Lulu Spark skills、安装 spark skills、dialogue-summary dialogue-archive theme-line theme-transcribe note-task
---

# lulu-spark-skills — 安装

## Platform Context

**Detect once; substitute `$SKILL_DIR` throughout:**

| | Cursor | Copilot | Claude | Codex |
|---|---|---|---|---|
| `$SKILL_DIR` | `~/.cursor/skills/lulu-spark-skills` | `~/.copilot/skills/lulu-spark-skills` | `~/.claude/skills/lulu-spark-skills` | `~/.agents/skills/lulu-spark-skills` |
| `$SRC` | `~/.cursor/skills/.lulu-spark-src` | `~/.copilot/skills/.lulu-spark-src` | `~/.claude/skills/.lulu-spark-src` | `~/.agents/skills/.lulu-spark-src` |

检测信号（优先级）：`CURSOR_AGENT` → Cursor · `COPILOT_AGENT` / `VSCODE_TARGET_SESSION_LOG` → Copilot · `CLAUDE_CODE` → Claude · `CODEX_AGENT` → Codex · 均无 → 向用户确认平台后再替换 `$SKILL_DIR`。

## 安装

Skills 在 [lulu-spark](https://github.com/lulufoo/lulu-spark) 的 `skills/`。平台要求 `$SKILL_DIR` 根上有 `SKILL.md`，所以先 sparse checkout 主仓，再把 `skills/` 链到 `$SKILL_DIR`。

若 `$SKILL_DIR` 已是旧的 `lulu-spark-skills` clone，先删掉再装。

```bash
git clone --filter=blob:none --sparse https://github.com/lulufoo/lulu-spark.git "$SRC"
git -C "$SRC" sparse-checkout set skills
ln -sfn "$SRC/skills" "$SKILL_DIR"
```

更新：

```bash
git -C "$SRC" pull --rebase
```

克隆完成后平台自动发现子 skill（`dialogue-summary`、`dialogue-archive`、`theme-line`、`theme-transcribe`、`note-task`），均无需额外操作。digest 写法由 note-task 自己说明，不单独发现。

`dialogue-summary` 清洗脚本见包内 [`scripts/transcript-clean-control.py`](scripts/transcript-clean-control.py)；`dialogue-archive` 切片脚本见包内 [`dialogue-archive/scripts/dialogue_archive_normalize.py`](dialogue-archive/scripts/dialogue_archive_normalize.py)。各 skill 自己编排。

## 前置条件

归档 skill 执行前 **Lulu Spark App 必须运行**（MCP `spark-knowledge` 可用，`http://127.0.0.1:9876/mcp/cursor_ide`）。notes 根目录由 Lulu Spark 管理，**无需**本地配置文件。槽位名是 `spark`。

## 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `dialogue-summary` | [dialogue-summary/](dialogue-summary/) | 自包含总结：覆盖面随对话、单元丰富度固定、忠实整合不灌水 + `〔User〕` → 加载 note-task 归档 |
| `dialogue-archive` | [dialogue-archive/](dialogue-archive/) | 节点切片脚本 + **note-task** 原文归档；`sink=local-md` 仅 `.cache` |
| `theme-line` | [theme-line/](theme-line/) | 直接采集字幕/已有稿 → 完整对话组稿 → **note-task** |
| `theme-transcribe` | [theme-transcribe/](theme-transcribe/) | 下载媒体 + Whisper → 完整逐字稿 → **note-task**；无现成字幕时用 |
| `note-task` | [note-task/](note-task/) | 笔记 MCP 地图：`create_note` / catalog / files |

## 验收

触发任一子 skill 时，首步确认 Lulu Spark MCP 可用；归档成功后 MCP 返回 `id` / `common_path` / `raw_path`（或 digest 路径）。

路径公式与 digest 写法由 note-task 自己的 references 说明（随 `create_note` 的 `digest_body`）。
