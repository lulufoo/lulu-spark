# lulu-spark-skills

Lulu Spark 的 Agent Skills。本目录是 [lulu-spark](https://github.com/lulufoo/lulu-spark) 里的 `skills/`。

## 安装

支持 Cursor、Copilot、Claude Code、Codex。安装步骤见 [SKILL.md](SKILL.md)。

| Skill | 指令 | 说明 |
|-------|------|------|
| 安装 / 配置 | `lulu-spark-skills` | 见 [SKILL.md](SKILL.md) |
| 过程回顾 | `dialogue-summary` | [dialogue-summary/](dialogue-summary/) — 骨架 + 核心加深 + `〔User〕` 定调 → `.cache` |
| 原文归档 | `dialogue-archive` | [dialogue-archive/](dialogue-archive/) — 节点切片后写入 `.cache` |
| 完整对话整理 | `theme-line` | [theme-line/](theme-line/) — 采集字幕/已有稿为完整对话后写入 `.cache` |
| 视频转写流水线 | `theme-transcribe` | [theme-transcribe/](theme-transcribe/) — 下载媒体 + Whisper → 完整逐字稿 → `.cache` |
| 笔记写入 | `note-task` | [note-task/](note-task/) — 笔记 MCP 地图：`create_note` / catalog / files |
