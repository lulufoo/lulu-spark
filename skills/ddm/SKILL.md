---
name: ddm
description: >-
  Dialogue Distillation Model (DDM) 对话蒸馏工作流。
  dtd_normalize：仅执行 P0 规范化，将当前对话写入 raw/ 并更新 index.json。
  dtd_distill_dialogue：输入 raw 文件路径或 index id，仅执行 P2 生成 distilled（对话体）。
  dtd_distill_compose：输入 raw 文件路径或 index id；自动检测 diagnose 文件，若不存在则先执行 P1 诊断，再执行 P2-compose 生成完整合成文档。
  dtd_distill_overview：输入 raw 文件路径或 index id，按子话题归组生成 distilled（对话概要，每组 Turn 范围 + 一段话）。
  dtd_trace_digest：输入已有 raw 文件路径，执行 P1→P3→P4（不含 P2）。
  dtd_archive_summary：输入总结 Markdown（或先生成并确认），归档为 raw/ 一条 Entry（entry_kind: summary），不自动 distilled。
  Use when: 蒸馏 distill ddm normalize archive 归档 对话整理 raw distilled compose overview 概要 总结归档
argument-hint: 'dtd_normalize | dtd_archive_summary | dtd_distill_dialogue | dtd_distill_compose | dtd_distill_overview | dtd_trace_digest'
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
| `dtd_normalize` | 规范化 | 仅执行 Phase 0，归档到 `raw/` |
| `dtd_archive_summary` | 总结归档 | 总结 Markdown → `raw/`（无 TURN_SEP，不自动 P2） |
| `dtd_distill_dialogue` | 生成 distilled（对话体） | 输入 raw 文件路径或 index id，仅执行 P2 |
| `dtd_distill_compose` | 生成 distilled（合成文档） | 输入 raw 文件路径或 index id；检测 diagnose 文件是否存在，不存在则先执行 P1，再执行 P2-compose |
| `dtd_distill_overview` | 生成 distilled（对话概要） | 输入 raw 文件路径或 index id，按子话题归组，每组输出 Turn 范围 + 一段话 |
| `dtd_trace_digest` | raw→轨迹+摘要 | 输入 raw 文件路径，执行 P1→P3→P4（不含 P2） |

参数缺省或不明确时，询问用户选择模式。

## 参考文件

- [references/ddm-concepts.md](references/ddm-concepts.md) — 概念定义（保留规则、路径定义）
- [references/ddm-p0-normalize.md](references/ddm-p0-normalize.md) — Phase 0 执行规范（对话归一化）
- [references/ddm-p0-archive-summary.md](references/ddm-p0-archive-summary.md) — Phase 0-S 总结归档
- [references/ddm-p1-diagnose.md](references/ddm-p1-diagnose.md) — Phase 1 诊断
- [references/ddm-p2-generate.md](references/ddm-p2-generate.md) — Phase 2 生成 distilled（对话体）
- [references/ddm-p2-compose.md](references/ddm-p2-compose.md) — Phase 2 生成 distilled（合成文档，基于 P1 诊断输入）
- [references/ddm-p2-overview.md](references/ddm-p2-overview.md) — Phase 2 生成 distilled（对话概要，子话题 Turn 范围 + 一段话）
- [references/ddm-p3-trace.md](references/ddm-p3-trace.md) — Phase 3 认知轨迹
- [references/ddm-p4-digest.md](references/ddm-p4-digest.md) — Phase 4 摘要

> 命令速查：`dtd_normalize` 对话归档 raw · `dtd_archive_summary` 总结归档 raw · `dtd_distill_dialogue` 对话体 distilled · `dtd_distill_compose` 合成文档 distilled · `dtd_distill_overview` 对话概要 distilled · `dtd_trace_digest` 全流程（P1+P3+P4）
