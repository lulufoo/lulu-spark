# lulu-workbench-skills

个人 Agent Skill 仓库（面向 [lulu-workbench](https://github.com/lulufoo/lulu-workbench) 的对话归档、过程回顾、音视频稿主题化整理等）。

## 安装

支持 Cursor、Copilot、Claude Code、Codex。安装步骤见 [SKILL.md](SKILL.md)。

| Skill | 指令 | 说明 |
|-------|------|------|
| 安装 / 配置 | `lulu-workbench-skills` | 见 [SKILL.md](SKILL.md) |
| 过程回顾 | `dialogue-summary` | [dialogue-summary/](dialogue-summary/) — 骨架 + 核心加深 + `〔User〕` 定调 → 归档（过程回顾；已定稿文档落盘用 `theme-archive`） |
| 原文归档 | `dialogue-archive` | [dialogue-archive/](dialogue-archive/) — 节点切片后 **theme-archive** 录入；`local-md` 仅落 `.cache` |
| 主题时间线稿 | `theme-line` | [theme-line/](theme-line/) — 组稿后 **theme-archive** |
| 网页文章采集 | `theme-fetch` | [theme-fetch/](theme-fetch/) — 组稿后 **theme-archive** |
| 视频转写流水线 | `theme-transcribe` | [theme-transcribe/](theme-transcribe/) — STT → 流畅性 → **theme-archive** |
| 文档归档 | `theme-archive` | [theme-archive/](theme-archive/) — 统一录入：raw + 全文英文默认中译 + digest |
