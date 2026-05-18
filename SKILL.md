---
name: lulu-workbench-skills
description: >-
  lulu-workbench 归档技能包安装与配置。编辑仓库根 config.json 的 archive_root，并 symlink 子 skill（ddm、theme-line）。
  Use when: 安装 workbench skills、配置 archive_root、symlink ddm theme-line
---

# lulu-workbench-skills — 安装与配置

## 1. 配置 archive_root

编辑本仓库根目录 [`config.json`](config.json)：

```json
{
  "archive_root": "/path/to/your/lulu-workbench"
}
```

所有子 skill 通过 `{skill_dir}/../config.json` 读取同一配置。

## 2. 安装 symlink（Cursor 示例）

```bash
BASE=/Users/lulu/Code/lulu-workbench-skills
SKILLS=~/.cursor/skills

ln -sf $BASE $SKILLS/lulu-workbench-skills
ln -sf $BASE/ddm $SKILLS/ddm
ln -sf $BASE/theme-line $SKILLS/theme-line
```

## 3. 子 skill

| 指令 | 目录 | 说明 |
|------|------|------|
| `ddm` | [ddm/](ddm/) | 对话蒸馏：`dtd_raw_dialogue`、`dtd_raw_summary`、`dtd_distill_*`、`dtd_trace` |
| `theme-line` | [theme-line/](theme-line/) | 音视频稿主题化 → `raw/`，自动 digest |

## 4. 验收

任一子 skill 执行时输出 `archive_root` 与根 `config.json` 一致即可。

共享规范：[shared/archive-concepts.md](shared/archive-concepts.md)、[shared/archive-digest.md](shared/archive-digest.md)。
