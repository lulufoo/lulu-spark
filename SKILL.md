---
name: lulu-workbench-skills
description: >-
  lulu-workbench 归档技能包安装与配置。支持 Cursor 和 Copilot，通过 install.py 安装；编辑 config.json 的 archive_root。
  Use when: 安装 workbench skills、配置 archive_root、dialogue-summary theme-summary theme-line theme-fetch theme-archive theme-digest
---

# lulu-workbench-skills — 安装与配置

## Platform Context

**Detect once; substitute `$SKILL_DIR` throughout:**

| | Cursor | Copilot |
|---|---|---|
| `$SKILL_DIR` | `~/.cursor/skills/lulu-workbench-skills` | `~/.copilot/skills/lulu-workbench-skills` |
| `$PLATFORM_FLAG` | cursor | copilot |
		
## 安裃

```bash
python3 install.py --platform $PLATFORM_FLAG
```

克隆完成后平台自动发现子 skill（`dialogue-summary`、`theme-summary`、`theme-line`、`theme-fetch`、`theme-archive`、`theme-digest`），均无需额外操作。

## 配置 archive_root

编辑 `$SKILL_DIR/config.json`：

```json
{
  "archive_root": "/path/to/your/lulu-workbench-knowledge"
}
```

子 skill 执行时读取此文件获取 `archive_root`。

## 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `dialogue-summary` | [dialogue-summary/](dialogue-summary/) | 对话蒸馏：`dtd_raw_dialogue`、`dtd_distill_*`、`dtd_trace` |
| `theme-summary` | [theme-summary/](theme-summary/) | 总结归档至 `raw/` 并自动 digest（`dtd_raw_summary`） |
| `theme-line` | [theme-line/](theme-line/) | 多平台视频/访谈稿（YouTube、InfoQ、plain）→ TranscriptBundle → 主题优先时间线大纲，保存至 `raw/` 并自动 digest |
| `theme-fetch` | [theme-fetch/](theme-fetch/) | 多平台网页文章（WeChat、plain HTML…）→ ArticleBundle → 格式化 Markdown；Phase 3 编排 theme-archive + theme-digest |
| `theme-archive` | [theme-archive/](theme-archive/) | 文档落盘 `raw/` + 更新 `index.json`（不触发 digest） |
| `theme-digest` | [theme-digest/](theme-digest/) | 从 `raw/` 生成或补跑 `digest/`；由 producer 链式 Embedded，亦可独立调用 |

## 验收

触发任一子 skill 时，AI 首步读取 `archive_root`；确认输出路径与 `$SKILL_DIR/config.json` 中的值一致即可。

共享规范：[archive-concepts](shared/archive-concepts.md)。digest 执行见 [theme-digest/SKILL.md](theme-digest/SKILL.md)。
