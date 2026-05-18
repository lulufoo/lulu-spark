---
name: lulu-workbench-skills
description: >-
  lulu-workbench 归档技能包安装与配置。克隆仓库至 ~/.cursor/skills/，编辑 config.json 的 archive_root。
  Use when: 安装 workbench skills、配置 archive_root、dialogue-summary theme-summary theme-line theme-digest
---

# lulu-workbench-skills — 安装与配置

## 1. 安装（Cursor）

```bash
cd ~/.cursor/skills
git clone https://github.com/lulufoo/lulu-workbench-skills.git lulu-workbench-skills
```

克隆完成后 Cursor 自动发现子 skill（`dialogue-summary`、`theme-summary`、`theme-line`、`theme-digest`），`shared/` 通过相对路径访问，均无需额外操作。

## 2. 配置 archive_root

编辑 `~/.cursor/skills/lulu-workbench-skills/config.json`：

```json
{
  "archive_root": "/path/to/your/lulu-workbench"
}
```

子 skill 执行时读取此文件获取 `archive_root`。

## 3. 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `dialogue-summary` | [dialogue-summary/](dialogue-summary/) | 对话蒸馏：`dtd_raw_dialogue`、`dtd_distill_*`、`dtd_trace` |
| `theme-summary` | [theme-summary/](theme-summary/) | 总结归档至 `raw/` 并自动 digest（`dtd_raw_summary`） |
| `theme-line` | [theme-line/](theme-line/) | 视频/访谈稿重构为主题优先时间线大纲，保存至 `raw/` 并自动 digest |
| `theme-digest` | [theme-digest/](theme-digest/) | 从 `raw/` 生成或补跑 `digest/`；producer 链式 Embedded，亦可独立调用 |

## 4. 验收

触发任一子 skill 时，AI 首步读取 `archive_root`；确认输出路径与 `config.json` 中的值一致即可。

共享规范：[archive-concepts](shared/archive-concepts.md)。digest 执行见 [theme-digest](theme-digest/SKILL.md)（[archive-digest](shared/archive-digest.md) 为兼容跳转）。
