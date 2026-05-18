# 对话蒸馏模型（DDM）— 执行入口

> 参考：[DDM_CONCEPTS](ddm-concepts.md)

---

## 执行模式

| 模式 | 说明 |
|------|------|
| **dtd_raw_dialogue** | Phase 0 对话归一化（[ddm-p0-normalize.md](ddm-p0-normalize.md)）→ **自动 Phase 4 digest** → `raw/` + `digest/` |
| **dtd_raw_summary** | Phase 0-S 总结归档（[ddm-p0-archive-summary.md](ddm-p0-archive-summary.md)）→ **自动 Phase 4 digest** → `raw/` + `digest/` |
| **dtd_digest** | 输入 raw 路径或 index id，仅执行 Phase 4 digest（补跑） |
| **dtd_distill_dialogue** | 输入 raw 文件路径或 index id，仅执行 P2 生成 distilled（对话体） |
| **dtd_distill_compose** | 输入 raw 文件路径或 index id；自动检测 diagnose 文件，若不存在则先执行 P1 诊断，再执行 P2-compose 生成完整合成文档 |
| **dtd_distill_overview** | 输入 raw 文件路径或 index id，按子话题归组生成 distilled（对话概要，Turn 范围 + 一段话格式） |
| **dtd_trace** | 输入已有 raw 文件路径（或 index id），执行 P1 → P3（认知轨迹，不含 digest） |
| ~~**dtd_trace_digest**~~ | **已废弃** — 等同 `dtd_trace` |
| ~~**dtd_normalize**~~ | **已废弃** — 等同 `dtd_raw_dialogue` |
| ~~**dtd_archive_summary**~~ | **已废弃** — 等同 `dtd_raw_summary` |

---

## 前置步骤（所有模式共同执行）

**Step 0：读取 config.json**

读取 skill 目录下的 `config.json`，从 `archive_root` 字段获取本地 archive 根目录，确认字段存在后输出：`> ✅ config.json 读取完成 · archive_root: <路径>`

---

## P4 digest 链式步骤（`dtd_raw_dialogue` / `dtd_raw_summary` 共用）

以 P0 落盘后的 `{archive_root}/raw/<COMMON_PATH>` 为 **RAW**，加载 [ddm-p4-digest.md](ddm-p4-digest.md) 执行 Phase 4（[P4-0] 不满足则跳过，仍完成 P0 模式汇总）。

---

## dtd_raw_dialogue 模式

**Step 1：执行 P0 对话归一化**

加载 [ddm-p0-normalize.md](ddm-p0-normalize.md)，按其规范执行 Phase 0（Step 1-6）。

完成后输出：

```
> ✅ P0 完成
> 📄 raw：raw/<COMMON_PATH>
> 🗂 index.json 已更新（layers 新增 raw）
```

**Step 2：执行 Phase 4 摘要（自动）**

按上文 **P4 digest 链式步骤** 执行。

完成后输出：

```
> ✅ dtd_raw_dialogue 完成
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或「已跳过」）
> 🗂 index.json 已更新（layers 含 raw；若生成则含 digest）
```

---

## dtd_raw_summary 模式

**输入**：用户提供的总结 Markdown；或 Agent 根据当前会话生成总结并经用户确认后的正文。

**Step 1：执行 P0-S 总结归档**

加载 [ddm-p0-archive-summary.md](ddm-p0-archive-summary.md)，按其规范执行（Step 1–6）。

完成后输出：

```
> ✅ P0-S 完成
> 📄 raw：raw/<COMMON_PATH>
> 🗂 index.json 已更新（layers: raw, entry_kind: summary）
```

**Step 2：执行 Phase 4 摘要（自动）**

按上文 **P4 digest 链式步骤** 执行。

完成后输出：

```
> ✅ dtd_raw_summary 完成
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或「已跳过」）
> 🗂 index.json 已更新（layers 含 raw；若生成则含 digest；entry_kind: summary）
> 💡 如需 distilled，请执行：dtd_distill_overview <raw 路径或 index id>
```

---

## dtd_digest 模式

**输入**：用户提供 raw 文件的路径或 index.json 中的 32 位十六进制 id。

**Step 0：解析输入路径**

```
· 若输入为 32 位十六进制 id：
    读取 {archive_root}/index.json，查找对应条目的 common_path
    拼出绝对路径：{archive_root}/raw/<common_path>
· 若输入为路径：直接使用
确认文件存在，且 index.json 中该条目 layers 包含 "raw"
从路径解析 topic-path / ts / slug（raw/<topic-path>/<ts>-<slug>.md）
```

**Step 1：执行 Phase 4 摘要**

以 `{archive_root}/raw/<COMMON_PATH>` 为 **RAW**，加载 [ddm-p4-digest.md](ddm-p4-digest.md) 执行（[P4-0] 不满足则跳过）。

---

**dtd_digest 完成汇总**

```
> ✅ dtd_digest 完成
> 📋 digest：digest/<COMMON_PATH>（或「已跳过」）
> 🗂 index.json 已更新（若生成则 layers 含 digest）
```

---

## dtd_distill_dialogue 模式

**输入**：用户提供 raw 文件的路径或 index.json 中的 32 位十六进制 id。

**Step 0：解析输入路径**

```
· 若输入为 32 位十六进制 id：
    读取 {archive_root}/index.json，查找对应条目的 common_path
    拼出绝对路径：{archive_root}/raw/<common_path>
· 若输入为路径：直接使用
确认文件存在，且 index.json 中该条目 layers 包含 "raw"
从路径解析 topic-path / ts / slug（raw/<topic-path>/<ts>-<slug>.md）
```

**Step 1：读取 DIAGNOSE（可选）**

```
检查是否存在：{archive_root}/diagnose/<COMMON_PATH>
· 存在 → 读取全文，供 P2 辅助参考
· 不存在 → 跳过
```

加载 [ddm-p2-generate.md](ddm-p2-generate.md)，以解析出的 raw 文件路径执行，按四条规则生成 distilled 并落盘。

---

**dtd_distill_dialogue 完成汇总**

```
> ✅ dtd_distill_dialogue 完成
> 📝 distilled：distilled/<COMMON_PATH>
> 🗂 index.json 已更新（layers 新增 distilled）
```

---

## dtd_distill_compose 模式

**输入**：用户提供 raw 文件的路径或 index.json 中的 32 位十六进制 id。

**Step 0：解析输入路径**

```
· 若输入为 32 位十六进制 id：
    读取 {archive_root}/index.json，查找对应条目的 common_path
    拼出绝对路径：{archive_root}/raw/<common_path>
· 若输入为路径：直接使用
确认文件存在，且 index.json 中该条目 layers 包含 "raw"
从路径解析 topic-path / ts / slug（raw/<topic-path>/<ts>-<slug>.md）
```

**Step 1：检测 P1 诊断**

```
检查是否存在：{archive_root}/diagnose/<COMMON_PATH>
· 存在 → 输出：> ✅ diagnose 已存在，跳过 P1 执行，直接进入 Step 2
· 不存在 → 加载 [ddm-p1-diagnose.md](ddm-p1-diagnose.md)，以 `{archive_root}/raw/<COMMON_PATH>` 作为 RAW 执行。
               将 DIAGNOSE 写入 diagnose/<COMMON_PATH>，更新 index.json（layers 追加 "diagnose"）后继续。
```

**Step 2：生成 distilled（合成文档）**

加载 [ddm-p2-compose.md](ddm-p2-compose.md)，执行。先读取 `{archive_root}/diagnose/<COMMON_PATH>` 全文，再按规范生成 distilled 并落盘。

---

**dtd_distill_compose 完成汇总**

```
> ✅ dtd_distill_compose 完成
> 📝 distilled：distilled/<COMMON_PATH>
> 📋 diagnose：diagnose/<COMMON_PATH>（已有或本次生成）
> 🗂 index.json 已更新（layers 含 distilled；若本次执行 P1 则同时含 diagnose）
```

---

## dtd_distill_overview 模式

**输入**：用户提供 raw 文件的路径或 index.json 中的 32 位十六进制 id。

**Step 0：解析输入路径**

```
· 若输入为 32 位十六进制 id：
    读取 {archive_root}/index.json，查找对应条目的 common_path
    拼出绝对路径：{archive_root}/raw/<common_path>
· 若输入为路径：直接使用
确认文件存在，且 index.json 中该条目 layers 包含 "raw"
从路径解析 topic-path / ts / slug（raw/<topic-path>/<ts>-<slug>.md）
```

加载 [ddm-p2-overview.md](ddm-p2-overview.md)，以解析出的 raw 文件路径执行，按话题归组规则生成对话概要并落盘。

---

**dtd_distill_overview 完成汇总**

```
> ✅ dtd_distill_overview 完成
> 📝 distilled：distilled/<COMMON_PATH>
> 🗂 index.json 已更新（layers 新增 distilled）
```

---

## dtd_trace 模式

**输入**：用户提供 raw 文件的路径或 index.json 中的 32 位十六进制 id。

**Step 0：解析输入路径**

```
· 若输入为 32 位十六进制 id：
    读取 {archive_root}/index.json，查找对应条目的 common_path
    拼出绝对路径：{archive_root}/raw/<common_path>
· 若输入为路径：直接使用
确认文件存在，且 index.json 中该条目 layers 包含 "raw"
从路径解析 topic-path / ts / slug（raw/<topic-path>/<ts>-<slug>.md）
```

**Step 1 — Phase 1：诊断**

```
检查是否存在：{archive_root}/diagnose/<COMMON_PATH>
· 存在 → 输出：> ✅ diagnose 已存在，跳过 P1 执行，直接进入 Step 2
· 不存在 → 加载 [ddm-p1-diagnose.md](ddm-p1-diagnose.md)，以 `{archive_root}/raw/<COMMON_PATH>` 作为 RAW 执行。
               将 DIAGNOSE 写入 diagnose/<COMMON_PATH>，更新 index.json（layers 追加 "diagnose"）后继续。
```

**Step 2 — Phase 3：认知轨迹**

加载 [ddm-p3-trace.md](ddm-p3-trace.md)，传入 `DISTILLED` 导航路径（`{archive_root}/distilled/<COMMON_PATH>`，文件可不存在，仅作导航占位），执行（若 P3-0 条件不满足则跳过）。

---

**dtd_trace 完成汇总**

```
📦 dtd_trace 归档完成

diagnose  → diagnose/<COMMON_PATH>（已有或本次生成）
trace     → trace/<COMMON_PATH>（或「已跳过」）

index.json 条目 <id>：
  layers: ["raw", ...已完成的 layer...]

💡 digest 已由 dtd_raw_dialogue / dtd_raw_summary 生成，或请执行：dtd_digest <raw 路径或 index id>
```

---

## dtd_trace_digest 模式（已废弃）

调用本参数时，**按 `dtd_trace` 执行**，不执行 Phase 4 digest。

---

## dtd_normalize 模式（已废弃）

调用本参数时，**按 `dtd_raw_dialogue` 执行**（P0 对话归一化 → 自动 Phase 4 digest）。

---

## dtd_archive_summary 模式（已废弃）

调用本参数时，**按 `dtd_raw_summary` 执行**（P0-S 总结归档 → 自动 Phase 4 digest）。
