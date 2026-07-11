# 对话蒸馏模型（DDM）— 介绍与配置

> 版本：v3.0

---

## 这是什么？

**对话蒸馏模型（DDM）** 把对话与总结转为可复用 archive 文档。

- **P0 raw**：`dtd_raw_dialogue` / `dtd_raw_summary` → 落盘 `raw/` 后 **自动 digest**（[digest-workflow](../../../shared/digest-workflow.md) embedded）
- **P1–P3**：诊断、distilled、认知轨迹，与 digest 无数据依赖
- **配置**：仓库根 [config.json](../../config.json) 单例

---

## 模型文件注册表

| 标识 | 文件名 | 角色 |
|------|--------|------|
| `DDM_ENTRY` | `dialogue-to-doc.md` | 执行入口 |
| `DDM_CONCEPTS` | `ddm-concepts.md` | 本文 |
| `ARCHIVE_CONCEPTS` | `../shared/archive-concepts.md` | archive 路径与 config |
| `THEME_DIGEST` | `../../../shared/digest-workflow.md` | digest 生成与补跑 |
| `DTD_RAW_DIALOGUE` | `dtd-raw-dialogue.md` | `dtd_raw_dialogue` |
| `DTD_RAW_SUMMARY` | `dtd-raw-summary.md` | `dtd_raw_summary` |
| `DDM_DIAGNOSE` | `ddm-diagnose.md` | `dtd_trace` / `dtd_distill_compose`（P1） |
| `DTD_DISTILL_DIALOGUE` | `dtd_distill_dialogue.md` | `dtd_distill_dialogue` |
| `DTD_DISTILL_COMPOSE` | `dtd_distill_compose.md` | `dtd_distill_compose` |
| `DTD_DISTILL_TOPIC` | `dtd_distill_topic.md` | `dtd_distill_topic` |
| `DDM_TRACE` | `ddm-trace.md` | `dtd_trace`（P3） |

---

## 归档路径配置

`CTA_BASE` = `archive_root`（从 `{skill_dir}/../config.json` 读取）。

**路径定义**

```
COMMON_PATH  := <topic-path>/<ts>-<slug>.md
RAW          := CTA_BASE/raw/<COMMON_PATH>
DISTILLED    := CTA_BASE/distilled/<COMMON_PATH>
DIAGNOSE     := CTA_BASE/diagnose/<COMMON_PATH>
DIGEST       := CTA_BASE/digest/<COMMON_PATH>
TRACE        := CTA_BASE/trace/<COMMON_PATH>
```

**index.json** `layers` 顺序：`raw → digest → distilled → diagnose → trace`。

**链接维护**：只替换完整相对路径；禁止双前缀。
