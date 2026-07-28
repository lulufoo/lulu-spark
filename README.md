# lulu-workbench-skills

个人 Agent Skill 仓库（面向 [lulu-workbench](https://github.com/lulufoo/lulu-workbench) 的对话归档、过程回顾、音视频稿主题化整理等）。

## 安装

支持 Cursor、Copilot、Claude Code、Codex。安装步骤见 [SKILL.md](SKILL.md)。

| Skill | 指令 | 说明 |
|-------|------|------|
| 安装 / 配置 | `lulu-workbench-skills` | 见 [SKILL.md](SKILL.md) |
| 过程回顾 | `dialogue-summary` | [dialogue-summary/](dialogue-summary/) — 骨架 + 核心加深 + `〔User〕` 定调 → 归档（过程回顾；已定稿文档落盘用 `theme-archive`） |
| 原文归档 | `dialogue-archive` | [dialogue-archive/](dialogue-archive/) — clean-raw + `to-archive-md`；默认 MCP，`local-md` 仅落 `.cache`（与 summary 共用 `scripts/transcript-clean-*`） |
| 主题时间线稿 | `theme-line` | [theme-line/](theme-line/) |
| 网页文章采集 | `theme-fetch` | [theme-fetch/](theme-fetch/) |
| 文档归档 | `theme-archive` | [theme-archive/](theme-archive/) — raw 落盘；适用时**自动** digest（shared 契约，非公开 skill） |
