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
| `dtd_raw_dialogue` | 对话归一化 + digest | [dtd-raw-dialogue.md](references/dtd-raw-dialogue.md) → [shared/digest/archive-digest.md](../shared/digest/archive-digest.md) |
| `dtd_raw_summary` | 总结归档 + digest | [dtd-raw-summary.md](references/dtd-raw-summary.md) → [shared/digest/archive-digest.md] |
| `dtd_distill_dialogue` | 对话体 distilled | [dtd_distill_dialogue.md](references/dtd_distill_dialogue.md) |
| `dtd_distill_compose` | 合成文档 | P1 + [dtd_distill_compose.md](references/dtd_distill_compose.md) |
| `dtd_distill_topic` | 子话题 distilled | [dtd_distill_topic.md](references/dtd_distill_topic.md) |
| `dtd_trace` | 诊断 + 轨迹 | P1 + [ddm-p3-trace.md](references/ddm-p3-trace.md) |

参数缺省或不明确时，询问用户选择模式。

> 命令速查：`dtd_raw_dialogue` / `dtd_raw_summary` 归档并自动 digest · `dtd_distill_dialogue` · `dtd_distill_compose` · `dtd_distill_topic` · `dtd_trace`
