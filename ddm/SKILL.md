---
name: ddm
description: >-
  Dialogue Distillation Model (DDM) 对话蒸馏工作流。
  dtd_raw_dialogue：对话归一化后自动 digest，产出 raw/ + digest/。
  dtd_raw_summary：总结归档后自动 digest，产出 raw/ + digest/。
  dtd_distill_dialogue：输入 raw，生成对话体 distilled。
  dtd_distill_compose：输入 raw，P1 诊断 + 合成文档 distilled。
  dtd_distill_topic：输入 raw，按子话题生成 distilled。
  dtd_trace：输入 raw，P1→P3 认知轨迹。
  Use when: 蒸馏 distill ddm 归档 对话整理 raw digest distilled compose topic 总结归档
argument-hint: 'dtd_raw_dialogue | dtd_raw_summary | dtd_distill_dialogue | dtd_distill_compose | dtd_distill_topic | dtd_trace'
---

# DDM Skill — 执行说明

## 触发后的首要动作

1. 读取 `{skill_dir}/../config.json`，获取 `archive_root`
2. 根据参数加载 [references/dialogue-to-doc.md](references/dialogue-to-doc.md)

## 参数说明

| 参数 | 模式 | 说明 |
|------|------|------|
<<<<<<< HEAD
| `dtd_raw_dialogue` | 对话归一化 + digest | [ddm-p0-normalize.md](references/ddm-p0-normalize.md) → 自动 [ddm-p4-digest.md](references/ddm-p4-digest.md) |
| `dtd_raw_summary` | 总结归档 + digest | [ddm-p0-archive-summary.md](references/ddm-p0-archive-summary.md) → 自动 [ddm-p4-digest.md](references/ddm-p4-digest.md) |
| `dtd_digest` | 仅 digest | 输入 raw 路径或 index id；仅读 raw 生成一段话概述 |
| `dtd_distill_dialogue` | 生成 distilled（对话体） | 输入 raw 文件路径或 index id，仅执行 P2 |
| `dtd_distill_compose` | 生成 distilled（合成文档） | 输入 raw 文件路径或 index id；检测 diagnose 是否存在，不存在则先 P1，再 P2-compose |
| `dtd_distill_overview` | 生成 distilled（对话概要） | 输入 raw 文件路径或 index id，按子话题归组 |
| `dtd_trace` | 诊断 + 认知轨迹 | 输入 raw 路径；P1→P3，不生成 digest |

参数缺省或不明确时，询问用户选择模式。
=======
| `dtd_raw_dialogue` | 对话归一化 + digest | [dtd-raw-dialogue.md](references/dtd-raw-dialogue.md) → [shared/archive-digest.md](../shared/archive-digest.md) |
| `dtd_raw_summary` | 总结归档 + digest | [dtd-raw-summary.md](references/dtd-raw-summary.md) → shared digest |
| `dtd_distill_dialogue` | 对话体 distilled | [ddm-p2-generate.md](references/ddm-p2-generate.md) |
| `dtd_distill_compose` | 合成文档 | P1 + [ddm-p2-compose.md](references/ddm-p2-compose.md) |
| `dtd_distill_topic` | 子话题 distilled | [ddm-p2-topic.md](references/ddm-p2-topic.md) |
| `dtd_trace` | 诊断 + 轨迹 | P1 + [ddm-p3-trace.md](references/ddm-p3-trace.md) |
| ~~`dtd_trace_digest`~~ | 已废弃 | 等同 `dtd_trace` |
| ~~`dtd_normalize`~~ | 已废弃 | 等同 `dtd_raw_dialogue` |
| ~~`dtd_archive_summary`~~ | 已废弃 | 等同 `dtd_raw_summary` |
| ~~`dtd_distill_overview`~~ | 已废弃 | 等同 `dtd_distill_topic` |
>>>>>>> 0a28a4a (refactor(archive): shared digest, root config, and command/file renames)

## 参考文件

- [references/ddm-concepts.md](references/ddm-concepts.md)
- [references/dtd-raw-dialogue.md](references/dtd-raw-dialogue.md)
- [references/dtd-raw-summary.md](references/dtd-raw-summary.md)
- [references/ddm-p1-diagnose.md](references/ddm-p1-diagnose.md)
- [references/ddm-p2-generate.md](references/ddm-p2-generate.md)
- [references/ddm-p2-compose.md](references/ddm-p2-compose.md)
- [references/ddm-p2-topic.md](references/ddm-p2-topic.md)
- [references/ddm-p3-trace.md](references/ddm-p3-trace.md)
- [../shared/archive-concepts.md](../shared/archive-concepts.md)
- [../shared/archive-digest.md](../shared/archive-digest.md)

> 命令速查：`dtd_raw_dialogue` / `dtd_raw_summary` 归档并自动 digest · `dtd_distill_dialogue` · `dtd_distill_compose` · `dtd_distill_topic` · `dtd_trace`
