# WO-A2 · update_topics_from_github.py 简化

**依赖**：WO-A1（topics.json 已改为 v4 结构）  
**修改文件**：`update_topics_from_github.py`

---

## 背景

当前脚本遍历每个 repo 的一、二级目录，构建 `m` 映射并写入 topics.json。  
v4 结构移除了 `m` 和 `bl`，脚本只需：枚举 KNOWLEDGE_CORPUS repos → 输出 `{repo}` 列表。  
`dir` 字段为手动维护，脚本**不覆盖**已有 `dir` 字段。

---

## 步骤

### 1. 删除 `build_topics_map()` 函数

整个函数（约第 138～170 行）删除：

```python
# 删除这整个函数
def build_topics_map(
    full_name: str, default_branch: str, top_blacklist: tuple[str, ...]
) -> dict[str, list[str]]:
    ...
```

### 2. 删除 `DEFAULT_TOP_LEVEL_BLACKLIST` 常量

```python
# 删除这一行
DEFAULT_TOP_LEVEL_BLACKLIST = ("gradle",)
```

### 3. 改写 `run()` 函数核心逻辑

**删除**：
- `tmap = build_topics_map(full, branch, DEFAULT_TOP_LEVEL_BLACKLIST)` 调用
- `"bl": list(DEFAULT_TOP_LEVEL_BLACKLIST)` 写入

**改为**：
- 遍历 KNOWLEDGE_CORPUS repos，对每个 repo 生成 `{"repo": full_name}` 条目
- 写入前读取已有 topics.json，合并保留现有条目中的 `dir` 字段（避免覆盖手动维护字段）
- 写出 v4 格式（无 `m`、无 `bl`）

### 4. 改写 `out` 字典

```python
# 原来
out = {
    "version": 3,
    "source": ...,
    "defaultBranch": "main",
    "bl": list(DEFAULT_TOP_LEVEL_BLACKLIST),
    "topics": [{"repo": full, "m": tmap} for ...]
}

# 改为
# 读取已有 topics.json，提取手动维护的 dir 字段
existing_dirs = {}
if Path(out_path).exists():
    with open(out_path) as f:
        existing = json.load(f)
    for item in existing.get("topics", []):
        if "repo" in item and "dir" in item:
            existing_dirs[item["repo"]] = item["dir"]

topics_list = []
for full in discovered_repos:
    entry = {"repo": full}
    if full in existing_dirs:
        entry["dir"] = existing_dirs[full]
    topics_list.append(entry)

# 保留无 repo 的虚拟条目（如 common-tech）
for item in existing.get("topics", []):
    if "repo" not in item:
        topics_list.append(item)

out = {
    "version": 4,
    "source": ...,
    "defaultBranch": "main",
    "topics": topics_list
}
```

---

## 验收

```bash
python update_topics_from_github.py --dry-run 2>&1 | head -5
# 或直接运行后：
jq '.version, (has("bl")), ([.topics[] | has("m")] | any)' topics.json
# 期望: 4 / false / false
```
