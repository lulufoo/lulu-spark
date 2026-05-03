# 对话蒸馏模型（DDM）— 执行入口

> 参考：[DDM_CONCEPTS](ddm-concepts.md)

---

## 执行模式

| 模式 | 说明 |
|------|------|
| **dtd_normalize** | 仅执行 Phase 0（P0 规范化），将对话写入 `raw/` 并更新 `index.json` |
| **dtd_distill_dialogue** | 输入 raw 文件路径或 index id，仅执行 P2 生成 distilled（对话体） |
| **dtd_distill_compose** | 输入 raw 文件路径或 index id；自动检测 diagnose 文件，若不存在则先执行 P1 诊断，再执行 P2-compose 生成完整合成文档 |
| **dtd_trace_digest** | 输入已有 raw 文件路径（或 index id），执行 P1 → P3 → P4（不含 P2） |

---

## 前置步骤（所有模式共同执行）

**Step 0：读取 config.json**

读取 skill 目录下的 `config.json`，从 `archive_root` 字段获取本地 archive 根目录，确认字段存在后输出：`> ✅ config.json 读取完成 · archive_root: <路径>`

---

## dtd_normalize 模式

加载 [ddm-p0-normalize.md](ddm-p0-normalize.md)，按其规范执行 Phase 0（Step 1-6）。

完成后输出：

```
> ✅ dtd_normalize 完成
> 📄 raw：raw/<COMMON_PATH>
> 🗂 index.json 已更新（layers 新增 raw）
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
> � diagnose：diagnose/<COMMON_PATH>（已有或本次生成）
> 🗂 index.json 已更新（layers 含 distilled；若本次执行 P1 则同时含 diagnose）
```

---

## dtd_trace_digest 模式

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

---

**Step 2 — Phase 3：认知轨迹**

加载 [ddm-p3-trace.md](ddm-p3-trace.md)，传入 `DISTILLED` 导航路径（`{archive_root}/distilled/<COMMON_PATH>`），执行（若 P3-0 条件不满足则跳过）。

---

**Step 3 — Phase 4：摘要与归档**

检查 `{archive_root}/distilled/<COMMON_PATH>` 是否存在：
- 不存在 → 跳过 Phase 4，输出：`> ⏭ Phase 4 跳过（distilled 文件不存在，请先执行 dtd_distill_dialogue 或 dtd_distill_compose）`
- 存在 → 加载 [ddm-p4-digest.md](ddm-p4-digest.md)，传入 `DISTILLED`（`{archive_root}/distilled/<COMMON_PATH>`），执行（若 P4-0 条件不满足则跳过）。更新 `index.json` 所有标志。

---

**dtd_trace_digest 完成汇总**

```
📦 dtd_trace_digest 归档完成

distilled → distilled/<COMMON_PATH>（若存在）
diagnose  → diagnose/<COMMON_PATH>（已有或本次生成）
trace     → trace/<COMMON_PATH>（或"已跳过"）
digest    → digest/<COMMON_PATH>（或"已跳过"）

index.json 条目 <id>：
  layers: ["raw", ...已完成的 layer...]
```
