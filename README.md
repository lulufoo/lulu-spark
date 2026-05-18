# lulu-workbench-skills

个人 Agent Skill 仓库（[lulu-workbench](https://github.com/lulufoo/lulu-workbench) 向：对话归档、DDM 蒸馏、音视频稿主题化整理等）。**规则同步、缺陷分析、研发工作流等开发与工具类技能**见姊妹仓库 [lulu-dev-skills](https://github.com/lulufoo/lulu-dev-skills)；**学习 / 领域认知类技能**见 [lulu-skills](https://github.com/lulufoo/lulu-skills)。

本地克隆路径示例：`/Users/lulu/Code/lulu-workbench-skills`（请按你的本机路径调整下文 `BASE`）。各 skill 的 `config.json` 中 `archive_root` 需指向你的 [lulu-workbench](https://github.com/lulufoo/lulu-workbench) 克隆目录。

---

## Skills

### 归档与蒸馏

| Skill | 指令 | 路径 | 触发场景 |
|-------|------|------|---------|
| 对话蒸馏（DDM） | `ddm` | [ddm/ddm](./ddm/ddm/) | 「蒸馏」「归档」「对话整理」；`dtd_normalize` 写入 `raw/`；`dtd_distill_*` / `dtd_trace_digest` 生成 distilled / 轨迹摘要；`dtd_archive_summary` 总结归档 |

### 内容整理

| Skill | 指令 | 路径 | 触发场景 |
|-------|------|------|---------|
| 主题时间线稿 | `theme-line` | [theme-line/theme-line](./theme-line/theme-line/) | 访谈 / 播客 / 演讲稿按主题归组、弱化时间戳；「按主题整理 transcript」「Host / guest 对话展开」；整理后写入 workbench 归档 |

---

## 目录规范

- 每个 skill 一个目录，目录名用 `kebab-case`（可按领域再包一层，如 `ddm/ddm/`）
- `SKILL.md`：执行入口，`name` 字段为实际触发指令
- `references/`：分阶段或模板参考，按需读取
- `config.json`：`archive_root` 指向 lulu-workbench 根目录（必填）

---

## 安装方式

> 前提：已克隆本仓库与 [lulu-workbench](https://github.com/lulufoo/lulu-workbench)，并已按需修改各 skill 内 `config.json` 的 `archive_root`。

```bash
BASE=/Users/lulu/Code/lulu-workbench-skills

# 对话蒸馏（DDM）
ln -s $BASE/ddm/ddm ~/.cursor/skills/ddm

# 主题时间线稿
ln -s $BASE/theme-line/theme-line ~/.cursor/skills/theme-line
```

各 skill 内文档会说明 Claude Code、VS Code 等平台的安装目录；多平台并存时可按需重复 symlink。
