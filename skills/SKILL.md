---
name: lulu-spark-skills
description: >-
  Lulu Spark 技能包安装。克隆到平台 skills 目录（Cursor / Copilot / Claude / Codex）。
  Use when: 安装 Lulu Spark skills、安装 spark skills、spark-task
---

# lulu-spark-skills — 安装

Spark 的 workspace / project / 文档 / 搜索 / Read Later（五组 MCP）相关 skill。包根可继续加子 skill。

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

更新后子 skill 的名字是 `spark-task`（旧版叫 `note-task`，已不存在）。`$SKILL_DIR` 链到的是整个 `skills/` 目录，所以更新不需要重装。

克隆完成后平台自动发现子 skill（`spark-task`）。digest 写法由 spark-task 自己说明。

## 前置条件

`spark-task` 执行前 **Lulu Spark App 必须运行**（MCP `spark-knowledge` 可用，`http://127.0.0.1:9876/mcp/cursor_ide`）。文档数据目录由 Lulu Spark 管理，**无需**本地配置文件。槽位名是 `spark`。

## 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `spark-task` | [spark-task/](spark-task/) | 五组 MCP 地图：Global `search` · Workspace · Project · Document（`create_doc` 等）· Read Later |

## 验收

触发 `spark-task` 时，首步确认 Lulu Spark MCP 可用；写入成功后 MCP 返回 `ok` / `id`，以及按需出现的 `digest_written` / `zh_written` / `assets_copied`。

digest 写法由 spark-task 自己的 references 说明（随 `create_doc` 的 `digest_body`）。
