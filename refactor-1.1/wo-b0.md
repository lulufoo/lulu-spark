# WO-B0 · 创建迁移脚本 flatten_to_doc_theme.py

**依赖**：无（可与 WO-A1 并行）  
**新建文件**：`scripts/flatten_to_doc_theme.py`

---

## 背景

全量迁移脚本。将所有文档从旧路径结构（最多4级）迁移到固定3级结构：  
`<project>/<slug>/<ts>-<slug>.md`

两种运行模式：
- `--dry-run`：只计算映射，输出 `.cache/migration-preview.json`，**不修改任何文件**
- 正式模式：执行实际迁移

---

## 路径计算规则

```python
import re as _re

def sanitize_slug(raw_slug: str) -> str:
    """将 slug 规范化为合法的目录名（kebab-case）。
    - 下划线 '_' 替换为连字符 '-'
    - 以 "数字." 开头（如 "3.aidl-..."）时，将首个 '.' 替换为 '-'
    """
    slug = raw_slug.replace('_', '-')
    slug = _re.sub(r'^(\d+)\.', r'\1-', slug)
    return slug

def compute_new_cp(old_cp: str) -> str:
    parts = old_cp.split('/')
    project = parts[0]
    filename = parts[-1]                     # e.g. "202605010056-prompt-format-table-vs-text.md"
    ts = filename[:12]                       # "202605010056"
    raw_slug = filename[13:].removesuffix('.md')
    slug = sanitize_slug(raw_slug)           # 规范化：去下划线、去首位点
    return f"{project}/{slug}/{ts}-{slug}.md"
```

**说明**：
- 不论原路径有多少层（2、3、4级），统一取 `parts[0]` 为 project，`parts[-1]` 为文件名
- `filename[13:]`：跳过 12位ts + 1位连字符
- zh 文件（以 `-zh.md` 结尾）：`raw_slug = filename[13:].removesuffix('-zh.md')`，slug 同样经 `sanitize_slug()`，新路径 `{project}/{slug}/{ts}-{slug}-zh.md`
- `sanitize_slug()` 同时处理：
  - **下划线**（如 `cursor_objectivity_of_subjective_statem` → `cursor-objectivity-of-subjective-statem`）
  - **数字+点开头**（如 `3.aidl-proxy-stub-...` → `3-aidl-proxy-stub-...`）

---

## 脚本实现要点

### dry-run 模式输出结构

```json
{
  "entries": {
    "<id>": {
      "old_cp": "...",
      "new_cp": "...",
      "old_zh": "...",    // 仅 translations.zh 存在时
      "new_zh": "..."
    }
  },
  "conflicts": [
    { "project": "...", "slug": "...", "ids": ["id1", "id2"] }
  ],
  "diagnose_mappings": [
    { "old": "diagnose/...", "new": "diagnose/..." }
  ],
  "zh_mappings": [
    { "old": "raw/...", "new": "raw/..." }
  ]
}
```

### 正式模式执行顺序

1. **前置检查**：有冲突则打印冲突列表并 `sys.exit(1)`
2. **层文件迁移**（顺序：`raw` → `distilled` → `digest` → `trace`）：
   - 对每个 entry，对每个存在于 `layers[]` 的 layer，**跳过 `diagnose`**（diagnose 在 Step 3 单独处理）：
     - `if layer == 'diagnose': continue`
     - `src = ARCHIVE_ROOT / layer / old_cp`
     - `dst = ARCHIVE_ROOT / layer / new_cp`
     - `dst.parent.mkdir(parents=True, exist_ok=True)`
     - `src.rename(dst)`
3. **diagnose 迁移**：扫描 `diagnose/` 目录，找到 `diagnose/<old_cp>` 对应文件，mv 到 `diagnose/<new_cp>`
4. **zh 文件迁移**：按 `zh_mappings` mv（在 `raw/` 下）
5. **annotations 重命名**：`annotations/<old_cp_no_ext>.json` → `annotations/<new_cp_no_ext>.json`
6. **更新文件内导航链接**（每个已迁移的 .md 文件）：
   - 对 4 个 layer 的 `old_cp` → `new_cp` 路径做正则替换（含 zh 路径）
7. **写 index.json**（全部 mv 成功后才执行）：
   - 更新 `common_path` 和 `translations.zh`
8. **清理旧空目录**：对每个旧 `dst.parent` 的旧父目录，若为空则删除

### 导航链接替换规则

```python
for layer in ['raw', 'distilled', 'digest', 'trace', 'diagnose']:
    old_pattern = f"{layer}/{old_cp}"
    new_pattern = f"{layer}/{new_cp}"
    content = content.replace(old_pattern, new_pattern)
```

也要处理反引号包裹的变体（部分文档中存在）：
```python
old_pattern_bt = f"`{layer}/{old_cp}`"
new_pattern_bt = f"`{layer}/{new_cp}`"
```

---

## 验收

```bash
# dry-run 正常退出
python scripts/flatten_to_doc_theme.py --dry-run
echo "Exit: $?"   # 期望 0

# 无冲突
python -c "
import json
data = json.load(open('.cache/migration-preview.json'))
print('conflicts:', data['conflicts'])
print('entries:', len(data['entries']))
print('diagnose:', len(data['diagnose_mappings']))
"
```
