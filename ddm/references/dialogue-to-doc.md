# 对话蒸馏模型（DDM）— 执行入口

> 参考：[ddm-concepts.md](ddm-concepts.md)

---

## 执行模式

| 模式 | 说明 |
|------|------|
| **dtd_raw_dialogue** | [dtd-raw-dialogue.md](dtd-raw-dialogue.md) → 自动 [archive-digest](../../shared/archive-digest.md) → `raw/` + `digest/` |
| **dtd_raw_summary** | [dtd-raw-summary.md](dtd-raw-summary.md) → 自动 digest → `raw/` + `digest/` |
| **dtd_distill_dialogue** | 输入 raw 路径或 index id，仅 P2 对话体 distilled |
| **dtd_distill_compose** | 输入 raw；无 diagnose 则先 P1，再 P2-compose |
| **dtd_distill_topic** | 输入 raw；按子话题生成 distilled（[ddm-p2-topic.md](ddm-p2-topic.md)） |
| **dtd_trace** | 输入 raw；P1 → P3（不含 digest） |
| ~~**dtd_trace_digest**~~ | 已废弃 → `dtd_trace` |
| ~~**dtd_normalize**~~ | 已废弃 → `dtd_raw_dialogue` |
| ~~**dtd_archive_summary**~~ | 已废弃 → `dtd_raw_summary` |
| ~~**dtd_distill_overview**~~ | 已废弃 → `dtd_distill_topic` |

---

## 前置步骤（所有模式）

**Step 0：读取 config.json**

读取 `{skill_dir}/../config.json`（仓库根），确认 `archive_root` 存在：

`> ✅ config.json 读取完成 · archive_root: <路径>`

---

## digest 链式步骤（`dtd_raw_dialogue` / `dtd_raw_summary` 共用）

以 P0 落盘后的 `{archive_root}/raw/<COMMON_PATH>` 为 **RAW**，加载 [../../shared/archive-digest.md](../../shared/archive-digest.md)（[AD-0] 不满足则跳过）。

---

## dtd_raw_dialogue 模式

**Step 1**：加载 [dtd-raw-dialogue.md](dtd-raw-dialogue.md)，执行 Step 1–6。

**Step 2**：按 **digest 链式步骤** 执行。

完成汇总：

```
> ✅ dtd_raw_dialogue 完成
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或「已跳过」）
> 🗂 index.json 已更新
```

---

## dtd_raw_summary 模式

**输入**：总结 Markdown（或经用户确认后生成）。

**Step 1**：加载 [dtd-raw-summary.md](dtd-raw-summary.md)，执行 Step 1–6。

**Step 2**：按 **digest 链式步骤** 执行。

完成汇总：

```
> ✅ dtd_raw_summary 完成
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或「已跳过」）
> 🗂 index.json 已更新（entry_kind: summary）
> 💡 如需 distilled：dtd_distill_topic <raw 路径或 index id>
```

---

## dtd_distill_dialogue 模式

解析 raw 路径（或 index id → common_path）。可选读 diagnose。加载 [ddm-p2-generate.md](ddm-p2-generate.md) 执行。

---

## dtd_distill_compose 模式

解析 raw；检测 diagnose，无则 P1；加载 [ddm-p2-compose.md](ddm-p2-compose.md) 执行。

---

## dtd_distill_topic 模式

解析 raw；加载 [ddm-p2-topic.md](ddm-p2-topic.md)，按子话题规则落盘 distilled。

完成汇总：

```
> ✅ dtd_distill_topic 完成
> 📝 distilled：distilled/<COMMON_PATH>
```

---

## dtd_trace 模式

解析 raw；P1（[ddm-p1-diagnose.md](ddm-p1-diagnose.md)）→ P3（[ddm-p3-trace.md](ddm-p3-trace.md)）。不执行 digest。

完成汇总不含 digest 补跑提示；digest 由 `dtd_raw_*` / theme-line 自动生成，或见 [shared/archive-digest.md](../../shared/archive-digest.md) 补跑节。

---

## 已废弃模式

- `dtd_trace_digest` → `dtd_trace`
- `dtd_normalize` → `dtd_raw_dialogue`
- `dtd_archive_summary` → `dtd_raw_summary`
- `dtd_distill_overview` → `dtd_distill_topic`

---
