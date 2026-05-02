---
name: ddm
description: >-
  Dialogue Distillation Model (DDM) 对话蒸馏工作流。
  DTD 模式：完整归档当前对话，依次执行 P0→P1→P2→P3→P4，每步写入本地 archive。
  ACN 模式：仅执行 P0 规范化，将当前对话写入 raw/ 并更新 index.json，后续蒸馏从 web 页面触发。
  Use when: 蒸馏 distill ddm normalize archive 归档 对话整理 raw distilled
argument-hint: 'acn | dtd'
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
| `acn` | ACN 模式 | 仅执行 Phase 0，归档到 `raw/` |
| `dtd` | DTD 模式 | 执行完整 P0→P1→P2→P3→P4 流程 |

参数缺省或不明确时，询问用户选择模式。

## 参考文件

- [references/ddm-concepts.md](references/ddm-concepts.md) — 概念定义（保留规则、双轴归属模型、路径定义）
- [references/ddm-p0-normalize.md](references/ddm-p0-normalize.md) — Phase 0 执行规范
- [references/ddm-p1-diagnose.md](references/ddm-p1-diagnose.md) — Phase 1 诊断
- [references/ddm-p2-generate.md](references/ddm-p2-generate.md) — Phase 2 生成 distilled
- [references/ddm-p3-trace.md](references/ddm-p3-trace.md) — Phase 3 认知轨迹
- [references/ddm-p4-digest-archive.md](references/ddm-p4-digest-archive.md) — Phase 4 摘要与归档
