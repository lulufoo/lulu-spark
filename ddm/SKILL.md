---
name: ddm
description: >-
  Dialogue Distillation Model (DDM) 对话蒸馏工作流。
  dtd_raw_dialogue：P0 对话归一化（ddm-p0-normalize.md）后自动 Phase 4 digest，产出 raw/ + digest/。
  dtd_raw_summary：P0-S 总结归档（ddm-p0-archive-summary.md）后自动 Phase 4 digest，产出 raw/ + digest/。
  dtd_digest：输入 raw 路径或 index id，仅执行 Phase 4 digest（补跑）。
  dtd_distill_dialogue：输入 raw 文件路径或 index id，仅执行 P2 生成 distilled（对话体）。
  dtd_distill_compose：输入 raw 文件路径或 index id；自动检测 diagnose 文件，若不存在则先执行 P1 诊断，再执行 P2-compose 生成完整合成文档。
  dtd_distill_overview：输入 raw 文件路径或 index id，按子话题归组生成 distilled（对话概要，每组 Turn 范围 + 一段话）。
  dtd_trace：输入已有 raw 文件路径，执行 P1→P3（认知轨迹，不含 digest）。
  Use when: 蒸馏 distill ddm normalize archive 归档 对话整理 raw digest distilled compose overview 概要 总结归档
argument-hint: 'dtd_raw_dialogue | dtd_raw_summary | dtd_digest | dtd_distill_dialogue | dtd_distill_compose | dtd_distill_overview | dtd_trace'
---

# DDM Skill — 执行说明

## 触发后的首要动作

1. 读取 skill 目录下的 `config.json`，获取 `archive_root`
2. 根据参数（`acn` 或 `dtd`）加载对应的执行入口文件

## 执行入口

加载 [references/dialogue-to-doc.md](references/dialogue-to-doc.md)，按其规范执行。

## 参数说明

| 参数 | 模式 | 说明 |
|------|------|------|
| `dtd_raw_dialogue` | 对话归一化 + digest | [ddm-p0-normalize.md](references/ddm-p0-normalize.md) → 自动 [ddm-p4-digest.md](references/ddm-p4-digest.md) |
| `dtd_raw_summary` | 总结归档 + digest | [ddm-p0-archive-summary.md](references/ddm-p0-archive-summary.md) → 自动 [ddm-p4-digest.md](references/ddm-p4-digest.md) |
| `dtd_digest` | 仅 digest | 输入 raw 路径或 index id；仅读 raw 生成一段话概述 |
| `dtd_distill_dialogue` | 生成 distilled（对话体） | 输入 raw 文件路径或 index id，仅执行 P2 |
| `dtd_distill_compose` | 生成 distilled（合成文档） | 输入 raw 文件路径或 index id；检测 diagnose 是否存在，不存在则先 P1，再 P2-compose |
| `dtd_distill_overview` | 生成 distilled（对话概要） | 输入 raw 文件路径或 index id，按子话题归组 |
| `dtd_trace` | 诊断 + 认知轨迹 | 输入 raw 路径；P1→P3，不生成 digest |

参数缺省或不明确时，询问用户选择模式。

## 参考文件

- [references/ddm-concepts.md](references/ddm-concepts.md) — 概念定义（保留规则、路径定义）
- [references/ddm-p0-normalize.md](references/ddm-p0-normalize.md) — Phase 0 对话归一化
- [references/ddm-p0-archive-summary.md](references/ddm-p0-archive-summary.md) — Phase 0-S 总结归档
- [references/ddm-p1-diagnose.md](references/ddm-p1-diagnose.md) — Phase 1 诊断
- [references/ddm-p2-generate.md](references/ddm-p2-generate.md) — Phase 2 生成 distilled（对话体）
- [references/ddm-p2-compose.md](references/ddm-p2-compose.md) — Phase 2 生成 distilled（合成文档，基于 P1 诊断输入）
- [references/ddm-p2-overview.md](references/ddm-p2-overview.md) — Phase 2 生成 distilled（对话概要，子话题 Turn 范围 + 一段话）
- [references/ddm-p3-trace.md](references/ddm-p3-trace.md) — Phase 3 认知轨迹
- [references/ddm-p4-digest.md](references/ddm-p4-digest.md) — Phase 4 摘要（仅读 raw，一段话概述）

> 命令速查：`dtd_raw_dialogue` / `dtd_raw_summary` 归档 raw 并自动 digest · `dtd_digest` 补跑 digest · `dtd_distill_dialogue` 对话体 · `dtd_distill_compose` 合成文档 · `dtd_distill_overview` 对话概要 · `dtd_trace` 诊断+轨迹（P1+P3）
