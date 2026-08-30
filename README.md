# lulu-workbench-skills

个人 Agent Skill 仓库（面向 [lulu-workbench](https://github.com/lulufoo/lulu-workbench) 的对话归档、过程回顾、字幕对话整理等）。

## 安装

支持 Cursor、Copilot、Claude Code、Codex。安装步骤见 [SKILL.md](SKILL.md)。

| Skill | 指令 | 说明 |
|-------|------|------|
| 安装 / 配置 | `lulu-workbench-skills` | 见 [SKILL.md](SKILL.md) |
| 过程回顾 | `dialogue-summary` | [dialogue-summary/](dialogue-summary/) — 骨架 + 核心加深 + `〔User〕` 定调 → 归档（过程回顾；已定稿文档落盘用 `note-task`） |
| 原文归档 | `dialogue-archive` | [dialogue-archive/](dialogue-archive/) — 节点切片后 **note-task** 录入；`local-md` 仅落 `.cache` |
| 完整对话整理 | `theme-line` | [theme-line/](theme-line/) — 采集字幕/已有稿为完整对话后 **note-task** |
| 网页文章采集 | `theme-fetch` | [theme-fetch/](theme-fetch/) — 组稿后 **note-task** |
| 视频转写流水线 | `theme-transcribe` | [theme-transcribe/](theme-transcribe/) — 下载媒体 + Whisper → 完整逐字稿 → **note-task** |
| 笔记写入 | `note-task` | [note-task/](note-task/) — 笔记 MCP 地图：`create_note` / catalog / files |
| Todo 任务 | `todo-task` | [todo-task/](todo-task/) — Todo 任务树 CRUD（MCP） |
