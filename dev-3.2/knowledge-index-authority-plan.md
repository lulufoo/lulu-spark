---
name: knowledge-index cache fix
overview: .cache/knowledge-index.json 为 corpus 权威（JSON）；topics 由全量同步派生；☰ 仓库列表仍用 /api/repo-list。
todos:
  - id: task-0
    content: "Task 0: .gitignore + 恢复 refactor-2.0/index-generation-plan.md"
    status: pending
  - id: task-1
    content: "Task 1: 迁移脚本 + loader + .cache/knowledge-index.json"
    status: pending
  - id: task-2
    content: "Task 2: update_topics 读 JSON 写 topics"
    status: pending
  - id: task-3
    content: "Task 3: server API + api.js"
    status: pending
  - id: task-4
    content: "Task 4: 前端菜单与列表"
    status: pending
  - id: task-5
    content: "Task 5: workflow 与总验收"
    status: pending
isProject: false
---

# knowledge-index 权威与 topics 一致性 实现计划

**日期：** 2026-05-16  
**版本：** v1.4（TPQA 三轮迭代闭合）  
**文档目录：** `dev-3.2/`  
**TPQA：** [knowledge-index-authority-tpqa-evaluation.md](./knowledge-index-authority-tpqa-evaluation.md)（v1.4 终评：10/10 通过）  
**状态：** 已审阅，可执行

---

## 需求清单（本次聊天）

| # | 需求 | 落点 |
|---|------|------|
| 1 | 生成物勿放 `refactor-2.0/` | 删除 `refactor-2.0/knowledge-index.md` |
| 2 | 输出 `.cache` | `.cache/knowledge-index.json` |
| 3 | JSON 格式 | Schema + loader |
| 4 | index 权威，仅人工改 | 脚本/API 只读 |
| 5 | topics 来源于 index | 仅全量同步 |
| 6 | ☰ 仓库列表仍为全量 repo（按 type） | `GET /api/repo-list`（不变） |
| 7 | 改 index 后手动全量 | 不自动写 topics |
| 8 | 去本地刷新、项目添加 | 菜单 2 项 |
| 9 | index 与 topics 一致 | 总体验收 |
| 10 | 文档在 `dev-3.2/` | 本目录 |
| 11 | index-generation-plan 留 refactor-2.0 | Task 0 |

---

## 文件一览

| 文件 | Task |
|------|------|
| `.gitignore` | 0 |
| `refactor-2.0/index-generation-plan.md` | 0 Create |
| `dev-3.2/index-generation-plan-superseded.md` | 0 Delete |
| `scripts/migrate_knowledge_index_md_to_json.py` | 1 Create |
| `scripts/knowledge_index_loader.py` | 1 Create |
| `.cache/knowledge-index.json` | 1 Create |
| `refactor-2.0/knowledge-index.md` | 1 Delete |
| `scripts/update_topics_from_github.py` | 2 Modify only |
| `server.py` | 3 Modify |
| `js/api.js` | 3 Modify |
| `index.html` / `js/main.js` / `app.css` | 4 Modify |
| `lulu-workbench-workflow.md` | 5 Modify |

---

## 权威文件：`.cache/knowledge-index.json`

`.gitignore`：

```gitignore
.cache/
!.cache/knowledge-index.json
```

`load_knowledge_index(path) -> list[dict]` 每项：`id`, `repo`, `description`, `indexUrl`。

规范化：缺 `repo` 用 `defaultOwner/id`；缺 `indexUrl` 用 `main` branch URL；缺 `description` 跳过；0 条有效 → 失败。

单条示例：

```json
{
  "id": "ai-software-dev",
  "repo": "lulufoo/ai-software-dev",
  "description": "AI 软件开发工作流与框架",
  "indexUrl": "https://github.com/lulufoo/ai-software-dev/blob/main/refactor-2.0/_index.md"
}
```

文件顶层：`{ "version": 1, "entries": [ ... ] }`（`version` 可选，loader 忽略未知顶层键）。

---

## 架构速查

```
migrate_knowledge_index_md_to_json.py   # 不依赖 loader
knowledge_index_loader.load_knowledge_index(path)
update_topics_from_github.run(out_path, index_path)
server: sys.path += REPO_ROOT/scripts → import loader
GET /api/knowledge-index[?force=1]
POST /api/update-topics
```

| 约束 | 值 |
|------|---|
| 权威路径 | `.cache/knowledge-index.json` |
| 加载函数 | `load_knowledge_index` |
| PORT | `8765` |
| 全量按钮 | `#btn-repo-full-sync` |
| localStorage | `lulu_wb_knowledge_index_cache` |

---

## API 合约

### GET `/api/knowledge-index`

404 `{"error":"knowledge-index.json not found"}` | 500 非法或 0 条 | 200 `{entries, cached_at}`

### POST `/api/update-topics`

Body `{}`；subprocess `update_topics_from_github.py -o topics.json --index-path .cache/knowledge-index.json`，`cwd=REPO_ROOT`

### `js/api.js`

```javascript
export async function fetchKnowledgeIndex(force = false) {
  if (!force) {
    try {
      const raw = localStorage.getItem('lulu_wb_knowledge_index_cache');
      if (raw) {
        const data = JSON.parse(raw);
        if (data.entries?.length) return data;
      }
    } catch (_) { /* fall through */ }
  }
  const url = force ? '/api/knowledge-index?force=1' : '/api/knowledge-index';
  const res = await fetch(url);
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  try {
    localStorage.setItem('lulu_wb_knowledge_index_cache', JSON.stringify(data));
  } catch (_) {}
  return data;
}

export async function updateTopics() {
  const res = await fetch('/api/update-topics', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: '{}',
  });
  return res.json();
}
```

---

## Task 0: gitignore 与 refactor-2.0 文档归位

**目标：** `.cache/knowledge-index.json` 可提交；历史 plan 回 `refactor-2.0/`。

**Files:** `.gitignore`；`refactor-2.0/index-generation-plan.md`；删 `dev-3.2/index-generation-plan-superseded.md`；`README.md`

**依赖：** 无 | **被依赖：** 1

**验收：** `git add -n .cache/knowledge-index.json`；`git check-ignore -v` 含 negation；`test -f refactor-2.0/index-generation-plan.md`；`test ! -f dev-3.2/index-generation-plan-superseded.md`

- [ ] Step 1–4：gitignore → 复制 plan → 删 superseded → 验收

---

## Task 1: 迁移脚本 + loader + JSON

**目标：** `.cache/knowledge-index.json` 17 条；`load_knowledge_index` 可用。

**Files:** `migrate_*.py`、`knowledge_index_loader.py`、`.cache/knowledge-index.json`；删 `refactor-2.0/knowledge-index.md`

**依赖：** 0 | **被依赖：** 2, 3

**验收：**

```bash
python3 scripts/migrate_knowledge_index_md_to_json.py \
  --in refactor-2.0/knowledge-index.md --out .cache/knowledge-index.json
PYTHONPATH=scripts python3 -c "
from pathlib import Path
from knowledge_index_loader import load_knowledge_index
e = load_knowledge_index(Path('.cache/knowledge-index.json'))
assert len(e) == 17
print('ok', len(e))
"
```

- [ ] Step 1: 实现 migrate（fenced 行 → JSON）  
- [ ] Step 2: 实现 loader  
- [ ] Step 3: 运行 migrate  
- [ ] Step 4: 验收 `ok 17`  
- [ ] Step 5: `git rm refactor-2.0/knowledge-index.md`  

---

## Task 2: update_topics

**目标：** 只读 index 写 topics；删 fast/check-repo/rediscover/`_write_knowledge_index`。

**Files:** 仅 `update_topics_from_github.py`

**验收：**

```bash
python3 scripts/update_topics_from_github.py -o topics.json \
  --index-path .cache/knowledge-index.json
# Expected: exit 0

jq '[.topics[] | select(.type=="repo")] | length' topics.json
# Expected: 17

jq '[.topics[] | select(.type=="dir")] | length' topics.json
# Expected: 2（与改前 topics 中 dir 条数一致；若不同则对照 index 中 repo 的二级目录）

IDX_DESC=$(jq -r '.topics[] | select(.id=="ai-software-dev") | .description' topics.json)
JSON_DESC=$(jq -r '.entries[] | select(.id=="ai-software-dev") | .description' .cache/knowledge-index.json)
test "$IDX_DESC" = "$JSON_DESC" && echo "description ok"

shasum -a 256 .cache/knowledge-index.json | tee /tmp/ki-before.txt
python3 scripts/update_topics_from_github.py -o topics.json --index-path .cache/knowledge-index.json
shasum -a 256 .cache/knowledge-index.json | diff /tmp/ki-before.txt -
# Expected: 无 diff（脚本不得写 index）

python3 scripts/update_topics_from_github.py --help | grep -E 'fast|rediscover|check-repo'
# Expected: 无匹配（旧 flag 已删除）
```

- [ ] Step 1: `load_knowledge_index` 读 entries  
- [ ] Step 2: 合并既有 topics 中虚拟条目与 dir 手工字段  
- [ ] Step 3: 按 index 成员写 repo 条；GH meta 补 keywords（失败则仅无 keywords）  
- [ ] Step 4: 写 `topics.json`  
- [ ] Step 5: 删除 `_write_knowledge_index`、`--fast`/`--rediscover`/`--check-repo` 及 rediscover 分支  

---

## Task 3: server API

**目标：** HTTP 读 index；POST 跑脚本。

**Files:** `server.py`、`js/api.js`

**server 导入：**

```python
_SCRIPTS = REPO_ROOT / "scripts"
if str(_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS))
from knowledge_index_loader import load_knowledge_index
```

**GET handler（`server.py`，示意与 v1.4 一致）：**

```python
def _handle_knowledge_index(self, force: bool = False):
    index_path = REPO_ROOT / ".cache" / "knowledge-index.json"
    if not index_path.is_file():
        self._json_response({"error": "knowledge-index.json not found"}, 404)
        return
    try:
        entries = load_knowledge_index(index_path)
    except ValueError as e:
        self._json_response({"error": str(e)}, 500)
        return
    except Exception as e:
        self._json_response({"error": f"invalid knowledge-index: {e}"}, 500)
        return
    if not entries:
        self._json_response({"error": "no valid entries in knowledge-index"}, 500)
        return
    self._json_response({
        "entries": entries,
        "cached_at": datetime.now(timezone.utc).isoformat(),
    })
```

路由：`GET /api/knowledge-index`；`?force=1` 时跳过 server 侧缓存（若有）并仍返回 200 体。

**POST `/api/update-topics`：** 删除 body 分支 `fast`/`rediscover`/`check_repo`；固定：

```python
cmd = [
    sys.executable,
    str(REPO_ROOT / "scripts" / "update_topics_from_github.py"),
    "-o", str(REPO_ROOT / "topics.json"),
    "--index-path", str(REPO_ROOT / ".cache" / "knowledge-index.json"),
]
subprocess.run(cmd, cwd=REPO_ROOT, capture_output=True, text=True, timeout=300)
```

**验收：**

```bash
curl -s http://127.0.0.1:8765/api/knowledge-index | jq '.entries | length'
# Expected: 17

curl -s -X POST http://127.0.0.1:8765/api/update-topics \
  -H 'Content-Type: application/json' -d '{}' | jq '.ok'
# Expected: true
```

- [ ] Step 1: `sys.path` + `load_knowledge_index` 导入  
- [ ] Step 2: `_handle_knowledge_index` + 路由  
- [ ] Step 3: 简化 `POST /api/update-topics`  
- [ ] Step 4: `js/api.js` 两函数（见 API 合约）  
- [ ] Step 5: curl 验收  

---

## Task 4: 前端

**目标：** 菜单 2 项；☰ 对话框仍走 `fetchRepoList`（type 分组+筛选）；全量同步读 index 写 topics。

**Files:** `index.html`、`main.js`、`app.css`

**验收：** 下拉 2 项；打开列表有 `/api/repo-list`；全量同步用 index；无本地刷新/项目添加

- [ ] Step 1: html 删 refresh/add/dialog；rediscover→full-sync；保留 `repo-list-filter`  
- [ ] Step 2: 删旧监听器（fast/add/check-repo）  
- [ ] Step 3: **勿改** repo-list 渲染逻辑（`fetchRepoList`）  
- [ ] Step 4:

```javascript
document.getElementById('btn-repo-full-sync').addEventListener('click', async () => {
  _repoMenuDropdown.classList.remove('open');
  const btn = document.getElementById('btn-repo-menu');
  btn.disabled = true;
  btn.textContent = '⚙ 同步中…';
  try {
    const data = await api.updateTopics();
    if (data.error) {
      alert(`全量同步失败：${data.error}`);
      return;
    }
    await _reloadTopicsIntoState();
    titleCache.clear();
    await loadIndex({ managedBtn: true });
  } catch (e) {
    alert(`全量同步失败：${e.message}`);
  } finally {
    btn.disabled = false;
    btn.textContent = '⚙ 知识库';
  }
});
```

- [ ] Step 5: 列表 catch alert  
- [ ] Step 6: css 删 add-repo  

---

## Task 5: workflow + 总验收

**目标：** 文档与端到端行为与「index 权威 → 手动全量 → topics」一致。

**Files:** `lulu-workbench-workflow.md`（Modify only）

**依赖：** 0–4 | **被依赖：** 无

**workflow 必写两句：**

1. 改 `.cache/knowledge-index.json`（人工，git 提交）。  
2. 浏览器 **⊙ 全量同步** → 更新 `topics.json`（不自动触发）。

**总验收（端到端）：**

```text
1. ☰ 知识库列表 → 查看列表：条数 = jq '.entries|length' .cache/knowledge-index.json（17）
2. 编辑 index 某条 description → 保存 → 列表 force 刷新可见新文案；topics 仍为旧值
3. ⊙ 全量同步 → topics 中同 id 的 description 与 index 一致
4. DevTools Network：无 /api/repo-list；有 /api/knowledge-index
```

- [ ] Step 1: 更新 workflow  
- [ ] Step 2: 跑总验收四则  
- [ ] Step 3: TPQA 评估文档已归档  

---

## 错误处理汇总

| 场景 | HTTP/exit | Task |
|------|-----------|------|
| JSON 不存在 | 404/1 | 1,3 |
| JSON 非法 | 500/1 | 1,3 |
| 单条无效 | 跳过 | 1 |
| 0 条 | 500/1 | 1,2 |
| GH meta 失败 | 无 keywords | 2 |
| 全量/列表失败 | alert | 4 |

---

## 变更记录

| 版本 | 变更 |
|------|------|
| v1.3 | TPQA v1.2 闭合 |
| v1.4 | 二轮 TPQA：migrate 步骤、server import、handler 内联；三轮终评通过 |
