# 对话蒸馏模型（DDM）— 执行入口

> 参考：[DDM_CONCEPTS](ddm-concepts.md)

---

## 执行模式

| 模式 | 说明 |
|------|------|
| **dtd_normalize** | 仅执行 Phase 0（P0 规范化），将对话写入 `raw/` 并更新 `index.json` |
| **dtd_distill** | 输入 raw 文件路径或 index id，仅执行 P2 生成 distilled |
| **dtd_raw_to_doc** | 输入已有 raw 文件路径（或 index id），执行 P1 → P3 → P4（不含 P2） |

---

## 前置步骤（两种模式共同执行）

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

## dtd_distill 模式

**输入**：用户提供 raw 文件的路径或 index.json 中的 32 位十六进制 id。

**Step 0：解析输入路径**

```
· 若输入为 32 位十六进制 id：
    读取 {archive_root}/index.json，查找对应条目的 common_path
    拼出绝对路径：{archive_root}/raw/<common_path>
· 若输入为路径：直接使用
确认文件存在，且 index.json 中该条目 layers 包含 "raw"

加载 [ddm-p2-generate.md](ddm-p2-generate.md)，执行。先读取 `CACHE_RAW` 全文，再按四条规则生成 distilled 并落盘。

---

**dtd_distill 完成汇总**

```
> ✅ dtd_distill 完成
> 📝 distilled：distilled/<COMMON_PATH>
> 🗂 index.json 已更新（layers 新增 distilled）
```

---

## dtd_raw_to_doc 模式

**输入**：用户提供 raw 文件的路径或 index.json 中的 32 位十六进制 id。

**Step 0：解析输入路径**

```
· 若输入为 32 位十六进制 id：
    读取 {archive_root}/index.json，查找对应条目的 common_path
    拼出绝对路径：{archive_root}/raw/<common_path>
· 若输入为路径：直接使用
确认文件存在，且 index.json 中该条目 layers 包含 "raw"
从路径解析 topic-path / ts / slug（方式同 P0 Step 1）
```

**Step 1 — Phase 1：诊断**

加载 [ddm-p1-diagnose.md](ddm-p1-diagnose.md)，以解析出的路径作为 `CACHE_RAW`，执行。将 DIAGNOSE 写入 `diagnose/<COMMON_PATH>` 后继续。

---

**Step 2 — Phase 3：认知轨迹**

加载 [ddm-p3-trace.md](ddm-p3-trace.md)，执行（若 P3-0 条件不满足则跳过）。

---

**Step 3 — Phase 4：摘要与归档**

加载 [ddm-p4-digest.md](ddm-p4-digest.md)，执行（若 P4-0 条件不满足则跳过）。更新 `index.json` 所有标志。

---

**dtd_raw_to_doc 完成汇总**

```
📦 dtd_raw_to_doc 归档完成

raw       → raw/<COMMON_PATH>（已有）
trace     → trace/<COMMON_PATH>（或"已跳过"）
digest    → digest/<COMMON_PATH>（或"已跳过"）

index.json 条目 <id>：
  layers: ["raw", ...已完成的 layer...]
```
