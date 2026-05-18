---
name: ddm
description: >-
  Dialogue Distillation Model (DDM)：对话归档与蒸馏，产出 raw、digest、distilled、trace。
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
| `dtd_raw_summary` | 总结归档 + digest | [dtd-raw-summary.md](references/dtd-raw-summary.md) → [shared/digest/archive-digest.md](../shared/digest/archive-digest.md) |
| `dtd_distill_dialogue` | 对话体 distilled | [dtd_distill_dialogue.md](references/dtd_distill_dialogue.md) |
| `dtd_distill_compose` | 合成文档 | [ddm-diagnose.md](references/ddm-diagnose.md) + [dtd_distill_compose.md](references/dtd_distill_compose.md) |
| `dtd_distill_topic` | 子话题 distilled | [dtd_distill_topic.md](references/dtd_distill_topic.md) |
| `dtd_trace` | 诊断 + 轨迹 | [ddm-diagnose.md](references/ddm-diagnose.md) + [ddm-trace.md](references/ddm-trace.md) |
| ~~`dtd_trace_digest`~~ | 已废弃 | 等同 `dtd_trace` |
| ~~`dtd_normalize`~~ | 已废弃 | 等同 `dtd_raw_dialogue` |
| ~~`dtd_archive_summary`~~ | 已废弃 | 等同 `dtd_raw_summary` |
| ~~`dtd_distill_overview`~~ | 已废弃 | 等同 `dtd_distill_topic` |

参数缺省或不明确时，询问用户选择模式。

## 参考文件

- [references/dialogue-to-doc.md](references/dialogue-to-doc.md) — 执行入口
- [references/ddm-concepts.md](references/ddm-concepts.md)
- [references/dtd-raw-dialogue.md](references/dtd-raw-dialogue.md)
- [references/dtd-raw-summary.md](references/dtd-raw-summary.md)
- [references/ddm-diagnose.md](references/ddm-diagnose.md)
- [references/dtd_distill_dialogue.md](references/dtd_distill_dialogue.md)
- [references/dtd_distill_compose.md](references/dtd_distill_compose.md)
- [references/dtd_distill_topic.md](references/dtd_distill_topic.md)
- [references/ddm-trace.md](references/ddm-trace.md)
- [../shared/archive-concepts.md](../shared/archive-concepts.md)
- [../shared/digest/archive-digest.md](../shared/digest/archive-digest.md)

> 命令速查：`dtd_raw_dialogue` / `dtd_raw_summary` 归档并自动 digest · `dtd_distill_dialogue` · `dtd_distill_compose` · `dtd_distill_topic` · `dtd_trace`
