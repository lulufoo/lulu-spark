# WO-C1 · server.py — 新增 /api/move-project

**依赖**：WO-B2（迁移完成，所有文件已在新路径格式下）  
**修改文件**：`server.py`

---

## 背景

新增一个本地 project 切换 API，与现有 `/api/gh-move`（GitHub 跨 repo 移动）完全独立。  
操作：在同一 archive_root 内，将文档从一个 project 移动到另一个 project（project = top-level 目录名）。

---

## 步骤

### 1. 在 do_POST() 路由表中新增

在 `elif self.path == '/api/set-importance':` 这行之后追加（当前位于第 52 行）：

```python
elif self.path == '/api/move-project':
    self._handle_move_project()
```

### 2. 新增 `_handle_move_project()` 方法

在 `_handle_delete()` 方法之前插入，完整实现如下：

```python
def _handle_move_project(self):
    try:
        data = self._read_json()
        entry_id = str(data.get('id') or '').strip()
        new_project = str(data.get('new_project') or '').strip()

        # 基本校验
        if not re.fullmatch(r'[0-9a-f]{32}', entry_id):
            self._json_response({'error': 'Invalid id'}, 400)
            return
        if not new_project or '..' in new_project or '/' in new_project:
            self._json_response({'error': 'Invalid new_project'}, 400)
            return

        # 校验 new_project 在 topics.json 中
        topics_path = REPO_ROOT / 'topics.json'
        topics_data = json.loads(topics_path.read_text(encoding='utf-8'))
        valid_projects = set()
        for t in topics_data.get('topics', []):
            if 'dir' in t:
                valid_projects.add(t['dir'])
            elif 'repo' in t:
                valid_projects.add(t['repo'].split('/')[-1])
        if new_project not in valid_projects:
            self._json_response({'error': f'Unknown project: {new_project}'}, 400)
            return

        # 读取 index.json
        index_path = REPO_ROOT / 'index.json'
        index_data = json.loads(index_path.read_text(encoding='utf-8'))
        entries = index_data.get('entries', index_data)

        if entry_id not in entries:
            self._json_response({'error': 'Entry not found'}, 404)
            return

        entry = entries[entry_id]
        old_cp = entry.get('common_path', '')
        if not old_cp or '..' in old_cp:
            self._json_response({'error': 'Invalid common_path'}, 400)
            return

        # 计算新 common_path（仅替换第一个路径段）
        old_parts = old_cp.split('/')
        old_parts[0] = new_project
        new_cp = '/'.join(old_parts)

        if old_cp == new_cp:
            self._json_response({'error': 'Already in this project'}, 400)
            return

        # 收集所有 src → dst 路径对
        moves = []
        for layer in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
            src = (REPO_ROOT / layer / old_cp).resolve()
            dst = (REPO_ROOT / layer / new_cp).resolve()
            if not str(src).startswith(str(REPO_ROOT) + os.sep):
                continue
            if src.exists():
                moves.append((src, dst))

        # annotation 文件
        ann_src = self._annotation_path(old_cp)
        if ann_src and ann_src.exists():
            old_ann_rel = (old_cp[:-3] + '.json') if old_cp.endswith('.md') else (old_cp + '.json')
            new_ann_rel = (new_cp[:-3] + '.json') if new_cp.endswith('.md') else (new_cp + '.json')
            ann_dst = (REPO_ROOT / 'annotations' / new_ann_rel).resolve()
            moves.append((ann_src, ann_dst))

        # zh 翻译文件
        old_zh = entry.get('translations', {}).get('zh')
        new_zh = None
        if old_zh:
            zh_parts = old_zh.split('/')
            zh_parts[0] = new_project
            new_zh = '/'.join(zh_parts)
            zh_src = (REPO_ROOT / 'raw' / old_zh).resolve()
            zh_dst = (REPO_ROOT / 'raw' / new_zh).resolve()
            if zh_src.exists():
                moves.append((zh_src, zh_dst))

        # 原子性 mv：逐一执行，失败则 rollback
        completed = []
        try:
            for src, dst in moves:
                dst.parent.mkdir(parents=True, exist_ok=True)
                src.rename(dst)
                completed.append((src, dst))
                print(f'  [move-project] {src.relative_to(REPO_ROOT)} → {dst.relative_to(REPO_ROOT)}')
        except Exception as mv_err:
            # Rollback
            for src, dst in reversed(completed):
                try:
                    dst.rename(src)
                except Exception:
                    pass
            self._json_response({'error': f'Move failed, rolled back: {mv_err}'}, 500)
            return

        # 更新文件内导航链接（仅针对已成功 mv 的 .md 文件）
        for src, dst in completed:
            if not dst.suffix == '.md':
                continue
            try:
                content = dst.read_text(encoding='utf-8')
                for layer in ('raw', 'distilled', 'digest', 'trace', 'diagnose'):
                    content = content.replace(f'{layer}/{old_cp}', f'{layer}/{new_cp}')
                    content = content.replace(f'`{layer}/{old_cp}`', f'`{layer}/{new_cp}`')
                if old_zh and new_zh:
                    content = content.replace(f'raw/{old_zh}', f'raw/{new_zh}')
                dst.write_text(content, encoding='utf-8')
            except Exception:
                pass  # 链接更新失败不阻断，文件已在新位置

        # 写 index.json（最后执行）
        entry['common_path'] = new_cp
        if new_zh:
            entry.setdefault('translations', {})['zh'] = new_zh
        tmp_path = index_path.with_suffix('.json.tmp')
        tmp_path.write_text(
            json.dumps(index_data, ensure_ascii=False, indent=2),
            encoding='utf-8'
        )
        tmp_path.replace(index_path)

        print(f'  [move-project] entry {entry_id}: {old_cp} → {new_cp}')
        self._json_response({'ok': True, 'new_common_path': new_cp})

    except json.JSONDecodeError:
        self._json_response({'error': 'Invalid JSON body'}, 400)
    except Exception as e:
        self._json_response({'error': str(e)}, 500)
```

---

## 验收

```bash
# 启动 server 后，用现有 entry id 测试（替换为实际 id）
curl -s -X POST http://localhost:8765/api/move-project \
  -H 'Content-Type: application/json' \
  -d '{"id":"<entry_id>","new_project":"ai-software-dev"}' | python -m json.tool
# 期望: {"ok": true, "new_common_path": "ai-software-dev/..."}

# 测试非法 project
curl -s -X POST http://localhost:8765/api/move-project \
  -H 'Content-Type: application/json' \
  -d '{"id":"<entry_id>","new_project":"nonexistent"}' | python -m json.tool
# 期望: {"error": "Unknown project: nonexistent"}
```
