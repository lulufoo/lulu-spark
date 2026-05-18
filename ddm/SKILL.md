---
name: ddm
description: >-
  Dialogue Distillation Model (DDM)：对话归档与蒸馏，产出 raw、digest、distilled、trace。
  Use when: 蒸馏 distill ddm 归档 对话整理 raw digest distilled compose topic 总结归档
argument-hint: 'dtd_raw_dialogue | dtd_raw_summary | dtd_distill_dialogue | dtd_distill_compose | dtd_distill_topic | dtd_trace'
---

# DDM Skill — 执行说明

> 参考：[ddm-concepts.md](references/ddm-concepts.md)

## 触发后的首要动作

1. 读取 `{skill_dir}/../config.json`（仓库根），获取 `archive_root`，确认存在：

   `> ✅ config.json 读取完成 · archive_root: <路径>`

2. 根据下方「参数说明」选择模式，执行对应章节。

## 参数说明

| 参数 | 模式 | 说明 |
|------|------|------|
| `dtd_raw_dialogue` | 对话归一化 + digest | [dtd-raw-dialogue.md](references/dtd-raw-dialogue.md) → [archive-digest.md](../shared/digest/archive-digest.md) |
| `dtd_raw_summary` | 总结归档 + digest | [dtd-raw-summary.md](references/dtd-raw-summary.md) → [archive-digest.md](../shared/digest/archive-digest.md) |
| `dtd_distill_dialogue` | 对话体 distilled | [dtd_distill_dialogue.md](references/dtd_distill_dialogue.md) |
| `dtd_distill_compose` | 合成文档 | [ddm-diagnose](./ddm-diagnose.md) → [dtd_distill_compose.md](references/dtd_distill_compose.md) |
| `dtd_distill_topic` | 子话题 distilled | [dtd_distill_topic.md](references/dtd_distill_topic.md) |
| `dtd_trace` | 诊断 + 轨迹 | [ddm-diagnose](./ddm-diagnose.md) → [ddm-p3-trace.md](references/ddm-p3-trace.md) |

## dtd_raw_dialogue 模式

**Step 1**：加载 [dtd-raw-dialogue.md](references/dtd-raw-dialogue.md)，执行 Step 1–6。

**Step 2**：加载 [archive-digest.md](../shared/digest/archive-digest.md)，按 **digest 链式步骤** 执行。

完成汇总：

```
> ✅ dtd_raw_dialogue 完成
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或「已跳过」）
> 🗂 index.json 已更新
```

## dtd_raw_summary 模式

**输入**：总结 Markdown（或经用户确认后生成）。

**Step 1**：加载 [dtd-raw-summary.md](references/dtd-raw-summary.md)，执行 Step 1–6。

**Step 2**：加载 [archive-digest.md](../shared/digest/archive-digest.md)，按 **digest 链式步骤** 执行。

完成汇总：

```
> ✅ dtd_raw_summary 完成
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或「已跳过」）
> 🗂 index.json 已更新（entry_kind: summary）
> 💡 如需 distilled：dtd_distill_topic <raw 路径或 index id>
```

## dtd_distill_dialogue 模式

**Step 1**: 解析 raw 路径（或 index id → common_path）；

**Step 2**: 加载 [dtd_distill_dialogue.md](references/dtd_distill_dialogue.md) 执行；

## dtd_distill_compose 模式

**Step 1**: 解析 raw；

**Step 2**: 检测 diagnose，无diagnose文件，则先加载 [ddm-diagnose](./ddm-diagnose.md) 执行；

**Step 3**: 加载 [dtd_distill_compose.md](references/dtd_distill_compose.md) 执行；

## dtd_distill_topic 模式

**Step 1**: 解析 raw；

**Step 2**: 加载 [dtd_distill_topic.md](references/dtd_distill_topic.md)，按规则落盘 distilled；

完成汇总：

```
> ✅ dtd_distill_topic 完成
> 📝 distilled：distilled/<COMMON_PATH>
```

## dtd_trace 模式

**Step 1**: 解析 raw；

**Step 2**: 检测 diagnose，无diagnose文件，则先加载 [ddm-diagnose](./ddm-diagnose.md) 执行；

**Step 3**: 加载 [ddm-p3-trace.md](references/ddm-p3-trace.md) 执行；
