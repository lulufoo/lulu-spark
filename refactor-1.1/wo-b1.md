# WO-B1 · Dry-run 审核（人工门控）

**依赖**：WO-B0 完成  
**执行者**：人工  
**此步骤不修改任何文件**

---

## 步骤

### 1. 执行 dry-run

```bash
cd /Users/lulu/Code/lulu-workbench
python scripts/flatten_to_doc_theme.py --dry-run
```

### 2. 打开审核文件

```bash
cat .cache/migration-preview.json | python -m json.tool | head -100
```

### 3. 检查项

**必须通过（任一不通过则停止）：**

- [ ] `conflicts` 列表为空（`[]`）
  - 若不为空：列出冲突条目，手动决定如何处理（如修改 slug、合并条目）后重新运行
- [ ] `entries` 总数与 `index.json` 中的条目数一致

**随机抽查（至少 5 条 entries）：**

- [ ] `project` = old_cp 第一段（正确）
- [ ] `slug` = 文件名去掉 ts 前缀和 .md（正确）
- [ ] new_cp 格式为 `<project>/<slug>/<ts>-<slug>.md`（正确）

**zh 文件检查：**

- [ ] `zh_mappings` 条目数与 index.json 中有 `translations.zh` 的条目数一致（当前已知至少 2 条：karpathy 视频 + waymo 视频）
- [ ] zh 新路径格式为 `<project>/<slug>/<ts>-<slug>-zh.md`（正确）

**diagnose 检查：**

- [ ] `diagnose_mappings` 条目数与 `diagnose/` 目录下实际文件数一致
  ```bash
  find diagnose -name "*.md" | wc -l
  ```

### 4. 签字

在此文件末尾追加：

```
审核结果: PASS
审核日期: YYYY-MM-DD
审核人: lulu
备注: （如有异常情况记录在此）
```

---

## ⚠️ 未通过则中止

conflicts 不为空时，**不得执行 WO-B2**，需先解决冲突。

---

<!-- 审核结果填写在此 -->
