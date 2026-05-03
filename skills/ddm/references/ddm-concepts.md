# 对话蒸馏模型（DDM）— 介绍与配置

> 版本：v2.9

---

## 这是什么？

**对话蒸馏模型（DDM）** 是一套把对话转成可复用认知文档的框架。

DDM 分为五个 Phase：P0（规范化）→ P1（诊断）→ P2（生成 distilled）→ P3（认知轨迹）→ P4（摘要）。每个 Phase 独立落盘，支持单独触发或全流程执行。

---

## 模型文件注册表

| 标识 | 文件名 | 角色 |
|------|--------|------|
| `DDM_ENTRY` | `dialogue-to-doc.md` | 执行入口 |
| `DDM_CONCEPTS` | `ddm-concepts.md` | 介绍与配置（本文） |
| `DDM_P0` | `ddm-p0-normalize.md` | Phase 0 执行规范 |
| `DDM_P1` | `ddm-p1-diagnose.md` | Phase 1 执行规范（含双轴归属理论） |
| `DDM_P2` | `ddm-p2-generate.md` | Phase 2 执行规范（含 distilled 保留规则） |
| `DDM_P3` | `ddm-p3-trace.md` | Phase 3 执行规范 |
| `DDM_P4` | `ddm-p4-digest.md` | Phase 4 执行规范 |

---

## 归档路径配置

`CTA_BASE` = `archive_root`（从 `config.json` 读取）；`<topic-path>` 在 Phase 0 Step 1 确定后全程不变。

**符号定义**

| 符号 | 类型 | 约束 |
|------|------|------|
| `ts` | string[12] | `YYYYMMDDHHMM`，Phase 0 落盘时东八区本地时间；同一轮归档共享 |
| `slug` | string | 全小写连字符；不含 `ts` |
| `topic-path` | string | Phase 0 Step 1 确定，全程不变 |

**路径定义**

    COMMON_PATH  := <topic-path>/<ts>-<slug>.md
    RAW          := CTA_BASE/raw/<COMMON_PATH>
    DISTILLED    := CTA_BASE/distilled/<COMMON_PATH>
    DIAGNOSE     := CTA_BASE/diagnose/<COMMON_PATH>     -- Phase 1 诊断摘要
    DIGEST       := CTA_BASE/digest/<COMMON_PATH>       -- 可选，无则不建
    TRACE        := CTA_BASE/trace/<COMMON_PATH>        -- 可选，无则不建

**文内创建时间**：一级标题下一行写 `> 创建时间：YYYY年M月D日 HH:MM`（月日不补零），须与 `<ts>` 一致。

**`index.json`（真源在 CTA_BASE/index.json）**

```json
{
  "version": 5,
  "entries": {
    "<id>": {
      "common_path": "<topic-path>/<ts>-<slug>.md",
      "created_at": "<ts>",
      "layers": ["raw", "distilled", "digest", "trace"]
    }
  }
}
```

`layers` 为字符串数组，列出该条目已存在的层（顺序：`raw → distilled → diagnose → digest → trace`）。

**链接维护**：只替换完整相对路径（`../../../<layer>/…/<ts>-<slug>.md`）；禁止对已含 `<ts>-<slug>` 前缀的路径再做 basename 替换，避免双前缀。

---

## 缓存文件

```
- CACHE_INDEX     := CTA_BASE/.cache/index.json
- CACHE_RAW       := CTA_BASE/.cache/<topic-path>/<ts>-<slug>-raw.md
- CACHE_DISTILLED := CTA_BASE/.cache/<topic-path>/<ts>-<slug>-distilled.md
- CACHE_TRACE     := CTA_BASE/.cache/<topic-path>/<ts>-<slug>-trace.md
- CACHE_DIGEST    := CTA_BASE/.cache/<topic-path>/<ts>-<slug>-digest.md
```
