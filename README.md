# lulu-workbench-skills

个人 Agent Skill 仓库（[lulu-workbench](https://github.com/lulufoo/lulu-workbench) 向：对话归档、DDM 蒸馏、音视频稿主题化整理等）。

本地克隆示例：`/Users/lulu/Code/lulu-workbench-skills`。**唯一配置**：仓库根 [`config.json`](config.json) 的 `archive_root`。

---

## Skills

| Skill | 指令 | 路径 | 说明 |
|-------|------|------|------|
| 安装 / 配置 | `lulu-workbench-skills` | [SKILL.md](SKILL.md) | symlink 与 `archive_root` |
| 对话蒸馏（DDM） | `ddm` | [ddm](ddm/) | `dtd_raw_*` 写 raw 并自动 digest；`dtd_distill_*` / `dtd_trace` |
| 主题时间线稿 | `theme-line` | [theme-line](theme-line/) | 音视频稿 → `raw/`，自动 digest |

共享规范：[shared/](shared/)（[archive-concepts](shared/archive-concepts.md)、[digest](shared/digest/archive-digest.md)）。

---

## 目录规范

- 仓库根 `config.json`：`archive_root` 单例
- 子 skill 各自 `SKILL.md` + `references/`；读配置用 `{skill_dir}/../config.json`
- `shared/`：跨 skill 规范；`shared/digest/` 等为各层子目录，后续可增其它层

---

## 安装

```bash
BASE=/Users/lulu/Code/lulu-workbench-skills
SKILLS=~/.cursor/skills

ln -sf $BASE $SKILLS/lulu-workbench-skills
ln -sf $BASE/ddm $SKILLS/ddm
ln -sf $BASE/theme-line $SKILLS/theme-line
```

编辑 `$BASE/config.json` 中的 `archive_root` 指向你的 lulu-workbench 克隆目录。

详见 [SKILL.md](SKILL.md)。
