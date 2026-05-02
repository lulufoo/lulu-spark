# 对话蒸馏模型（DDM）— 执行入口

> 参考：[DDM_CONCEPTS](ddm-concepts.md)

---

## 执行模式

| 模式 | 说明 |
|------|------|
| **ACN** | 仅执行 Phase 0（P0 规范化），将对话写入 `raw/` 并更新 `index.json`，后续蒸馏从 web 页面触发 |
| **DTD** | 完整归档当前对话，依次执行 P0 → P1 → P2 → P3 → P4 |

---

## 前置步骤（两种模式共同执行）

**Step 0：读取 config.json**

读取 skill 目录下的 `config.json`，从 `archive_root` 字段获取本地 archive 根目录。

```json
{
  "archive_root": "/path/to/cognitive-trace-archive"
}
```

确认字段存在后输出：`> ✅ config.json 读取完成 · archive_root: <路径>`

---

## ACN 模式

加载 [ddm-p0-normalize.md](ddm-p0-normalize.md)，按其规范执行 Phase 0（Step 1-6）。

完成后输出：

```
> ✅ ACN 完成
> 📄 raw：raw/<COMMON_PATH>
> 🗂 index.json 已更新（raw=true）
```

---

## DTD 模式

依次执行四个 Phase，每个 Phase 完成后再进入下一个。

**Step 1 — Phase 0：规范化**

加载 [ddm-p0-normalize.md](ddm-p0-normalize.md)，执行。输出 Step 6 汇总。

---

**Step 2 — Phase 1：诊断**

加载 [ddm-p1-diagnose.md](ddm-p1-diagnose.md)，执行。将 DIAGNOSE 写入 `diagnose/<COMMON_PATH>` 后继续。

---

**Step 3 — Phase 2：生成**

加载 [ddm-p2-generate.md](ddm-p2-generate.md)，执行。先读取 `CACHE_RAW` 全文，再按四条规则生成 distilled 并落盘。

---

**Step 4 — Phase 3：认知轨迹**

加载 [ddm-p3-trace.md](ddm-p3-trace.md)，执行（若 P3-0 条件不满足则跳过）。

---

**Step 5 — Phase 4：摘要与归档**

加载 [ddm-p4-digest-archive.md](ddm-p4-digest-archive.md)，执行（若 P4-0 条件不满足则跳过）。更新 `index.json` 所有标志。

---

**DTD 完成汇总**

```
📦 DTD 归档完成

raw       → raw/<COMMON_PATH>
distilled → distilled/<COMMON_PATH>
trace     → trace/<COMMON_PATH>（或"已跳过"）
digest    → digest/<COMMON_PATH>（或"已跳过"）

index.json 条目 <id>：
  raw: true · distilled: true · trace: <bool> · digest: <bool>
```
