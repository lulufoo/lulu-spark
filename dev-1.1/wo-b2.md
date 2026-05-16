# WO-B2 · 执行全量迁移

**依赖**：WO-B1（审核签字 PASS）  
**修改对象**：全量 layer 文件 + annotations + index.json（高风险，不可逆）

---

## ⚠️ 执行前确认

1. WO-B1 中 `审核结果: PASS` 已填写
2. 当前工作区无未提交的文件变更（git status 干净）：
   ```bash
   git status
   ```
3. 可选：创建迁移前的 git commit 作为回滚点：
   ```bash
   git add -A && git commit -m "chore: pre-migration snapshot for refactor-1.1"
   ```

---

## 步骤

### 1. 执行迁移

```bash
cd /Users/lulu/Code/lulu-workbench
python scripts/flatten_to_doc_theme.py
```

观察控制台输出，正常结束应无 ERROR 行。

### 2. 验证文件存在

```bash
python - <<'EOF'
import json
from pathlib import Path

root = Path('.')
idx = json.load(open('index.json'))
missing = []
for entry in idx['entries'].values():
    cp = entry['common_path']
    for layer in entry.get('layers', []):
        p = root / layer / cp
        if not p.exists():
            missing.append(str(p))
    if 'translations' in entry:
        zh = entry['translations'].get('zh')
        if zh:
            p = root / 'raw' / zh
            if not p.exists():
                missing.append(f"raw/{zh}")
if missing:
    print("MISSING FILES:")
    for m in missing:
        print(" ", m)
else:
    print("OK: 所有 index.json 路径对应文件均存在")
EOF
```

### 3. 验证旧路径已清理

抽查 3 个旧路径，确认不再存在：

```bash
# 旧路径示例（从 migration-preview.json 取）
ls raw/ai-assisted-domain-learning/dialogue-distillation-model/ 2>/dev/null && echo "旧目录仍存在！" || echo "OK"
```

### 4. 启动 server + 打开 Web UI

```bash
python3 server.py &
# 打开 http://localhost:8765 随机点开 3 篇文档
# 确认 nav 链接（distilled/digest/trace）点击后无 404
```

### 5. 验证导航链接格式

抽查 1 个迁移后的 raw 文件，确认导航链接使用新路径格式：

```bash
# 示例（替换为实际文件路径）
head -10 raw/ai-assisted-domain-learning/dialogue-distillation-model/202604251532-dialogue-to-doc-opt.md
# 应看到：导航链接中路径已变为 ai-assisted-domain-learning/dialogue-to-doc-opt/202604251532-...
```

---

## 验收标准

- [ ] 控制台输出无 ERROR
- [ ] 验证脚本输出 "OK: 所有 index.json 路径对应文件均存在"
- [ ] 旧路径已不存在
- [ ] Web UI 随机文档可正常打开，nav 无 404
