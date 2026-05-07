# Refactor 3.0 — 实施计划

> 参考文档：`requirements.md` · `architecture.md` · `interaction-design.md`

---

## TL;DR

四个模块。前置环境（P0）→ 索引构建脚本（P1）→ server.py 扩展（P2）→ 前端面板（P3）。

- **P0**：Meilisearch 安装 + 配置文件 + server.py 工具函数
- **P1**：`scripts/build_knowledge_index.py`，读 topics.json + 本地克隆，全量写入 Meilisearch
- **P2**：server.py 新增 3 路由（search / reindex / status）+ settle upsert
- **P3**：前端 knowledge-search 组件 + layout 改动 + viewer 接入

---

## Work Orders

| WO | 标题 | 依赖 |
|----|------|------|
| WO-P0 | 环境配置：Meilisearch + meili.env + _meili_request | — |
| WO-P1 | 索引构建脚本 build_knowledge_index.py | WO-P0 |
| WO-P2A | server.py：GET /api/search-knowledge | WO-P0 |
| WO-P2B | server.py：POST /api/reindex-knowledge（异步） + GET /api/reindex-status | WO-P0 |
| WO-P2C | server.py：_handle_settle 追加单条 upsert | WO-P0 |
| WO-P3A | index.html + app.css：#md-content-row wrapper + #knowledge-panel 布局 | — |
| WO-P3B | js/api.js：新增 3 个函数 | — |
| WO-P3C | js/components/knowledge-search.js：面板组件 | WO-P3A + WO-P3B |
| WO-P3D | js/components/viewer.js：entry 打开时触发搜索 | WO-P3C |

---

## 并行执行分组

```
批次1（无依赖，可并行）：WO-P0 · WO-P3A · WO-P3B
批次2（WO-P0 完成后，可并行）：WO-P1 · WO-P2A · WO-P2B · WO-P2C
批次3（WO-P3A + WO-P3B 完成后）：WO-P3C
批次4（所有前置完成后）：WO-P3D（端到端验证）
```

---

## 各 WO 详细说明

---

### WO-P0 · 环境配置

**手动操作（不写代码）**

1. 安装 Meilisearch
   - 本地：`brew install meilisearch`
   - 服务端：`docker run -d -p 7700:7700 -e MEILI_MASTER_KEY=<key> meilisearch/meilisearch:latest`

2. 创建 `.cache/meili.env`（目录已存在，加入 `.gitignore`）：
   ```
   MEILI_MASTER_KEY=<生成一个随机字符串>
   MEILI_URL=http://localhost:7700
   KNOWLEDGE_BASE_DIR=/Users/lulu/Code
   MEILI_STALE_HOURS=24
   ```

**代码改动：`server.py`**

- 启动时读 `.cache/meili.env`，写入模块级常量：
  `MEILI_URL` / `MEILI_KEY` / `KNOWLEDGE_BASE_DIR` / `MEILI_STALE_HOURS`
- 新增工具函数 `_meili_request(method, path, body=None)`：
  - 使用 `urllib.request`（stdlib），携带 `Authorization: Bearer {MEILI_KEY}`
  - Meilisearch 不可达时静默返回 `None`，不抛异常，不影响主流程
- 新增模块级变量 `_reindex_job = {"status": "idle", "started_at": None, "finished_at": None, "log": ""}`

**改动文件**：`server.py`、`.gitignore`

---

### WO-P1 · 索引构建脚本

**新建文件**：`scripts/build_knowledge_index.py`

逻辑流程：
```
读 topics.json → 筛选有 repo 字段的条目
for each repo:
    local_dir = KNOWLEDGE_BASE_DIR / repo.split("/")[-1]
    for each .md file in local_dir.rglob("*.md"):
        跳过黑名单（_index.md、README.md）
        title = 第一个 "# " 行内容，没有则取文件名去 .md
        id = f"{repo_name}__{path.replace('/', '__')}"
        构建文档 dict（id/title/body/repo/path/url/topic_desc）
每 100 条批量 PUT /indexes/knowledge/documents
结束：写 .cache/meili-meta.json {last_indexed_at, doc_count}
```

命令行参数：
- `--wipe`：先 `DELETE /indexes/knowledge`，再重建
- 无参数：直接 upsert（按 id 覆盖）

运行方式（首次建库）：
```bash
# 确认 Meilisearch 已启动
python3 scripts/build_knowledge_index.py --wipe
```

**新建文件**：`scripts/build_knowledge_index.py`

---

### WO-P2A · GET /api/search-knowledge

**改动文件**：`server.py`

- `do_GET` 新增分支：`/api/search-knowledge` → `_handle_search_knowledge()`
- 逻辑：
  1. 取 `q` 参数（必填）、`limit` 参数（默认 10）
  2. 读 `.cache/meili-meta.json`，计算 `stale = (now - last_indexed_at) > MEILI_STALE_HOURS`
  3. 调 Meilisearch `POST /indexes/knowledge/search`，`attributesToCrop: ["body"]`，`cropLength: 80`
  4. 返回：`{hits, stale, last_indexed_at}`
  5. Meilisearch 不可用（`_meili_request` 返回 `None`）时返回 `{hits:[], stale:null, error:"unavailable"}`

---

### WO-P2B · 异步 reindex

**改动文件**：`server.py`

`do_POST` 新增：`/api/reindex-knowledge` → `_handle_reindex_knowledge()`  
`do_GET` 新增：`/api/reindex-status` → `_handle_reindex_status()`

`_handle_reindex_knowledge()` 逻辑：
```
1. 检查 _reindex_job["status"] == "running" → 返回 409 {"error":"already running"}
2. 更新 _reindex_job = {status:"running", started_at:now, ...}
3. 启动 threading.Thread(_do_reindex_background)
4. 立即返回 {"status":"running"}
```

`_do_reindex_background()` 后台线程逻辑：
```
for each repo in topics.json（有 repo 字段）:
    subprocess.run(["git", "-C", local_dir, "pull", "--rebase"])
    更新 _reindex_job["log"]
subprocess.run(["python3", "scripts/build_knowledge_index.py"])
_reindex_job = {status:"done", finished_at:now, ...}
```

`_handle_reindex_status()` 逻辑：直接返回 `_reindex_job` 字典

---

### WO-P2C · settle 追加 upsert

**改动文件**：`server.py`，`_handle_settle()` 方法

位置：Step1（`_gh_put_file`）成功后、Step2（更新 annotation）之前

追加调用 `_meili_upsert_doc(repo, dst_path, full_content, topic_desc)`：
- 用内存中已有的 `target_repo` / `dst_path` / `full_content`，不读磁盘
- 调 `_meili_request("PUT", "/indexes/knowledge/documents", [doc])`
- 失败时写入 `warns`（与现有 Step2/Step3 失败处理一致），不中断主流程
- **不更新** `.cache/meili-meta.json` 的 `last_indexed_at`

---

### WO-P3A · layout 改动

**改动文件**：`index.html`、`app.css`

`index.html` 改动（`#md-panel` 内部）：
```html
<!-- 改动前 -->
<div id="md-body"></div>
<textarea id="md-edit-area" ...></textarea>

<!-- 改动后 -->
<div id="md-content-row">
  <div id="md-body"></div>
  <textarea id="md-edit-area" ...></textarea>
  <div id="knowledge-panel"></div>
</div>
```

`app.css` 新增：
```css
#md-content-row {
  display: flex;
  flex-direction: row;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}
#knowledge-panel {
  width: 280px;
  flex-shrink: 0;
  border-left: 1px solid #d0d7de;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  background: #f6f8fa;
}
#knowledge-panel.ks-unavailable { display: none; }
```

`app.css` 修改（`#md-body` 和 `#md-edit-area`）：确认 `flex:1` 在新容器内生效，`overflow-y:auto` 保持。

---

### WO-P3B · api.js 新增函数

**改动文件**：`js/api.js`

末尾追加 3 个函数：`searchKnowledge(q, limit=10)` / `reindexKnowledge()` / `getReindexStatus()`

详见 `interaction-design.md` §五·js/api.js 新增函数。

---

### WO-P3C · knowledge-search.js 组件

**新建文件**：`js/components/knowledge-search.js`

导出两个函数：
- `mountKnowledgeSearch(container)` — 注入 HTML 骨架，绑定事件（搜索框防抖、sync 按钮）
- `triggerKnowledgeSearch(entry)` — 以 entry 构建初始 query，触发搜索

内部实现：
- 状态机（idle / loading / showing / empty / syncing），参见 `interaction-design.md` §五
- 结果卡片渲染：snippet 用 `innerHTML`（Meilisearch `<em>` 高亮标签，安全）
- 同步轮询：`setInterval 2s`，`status=done` 时 `clearInterval` + 重新搜索
- Meilisearch 不可用（`error:unavailable`）时：`container.classList.add('ks-unavailable')`

样式均通过 class（`ks-*`），样式定义写在 `app.css`（`WO-P3A` 一并加入）。

---

### WO-P3D · viewer.js 接入

**改动文件**：`js/components/viewer.js`

1. 顶部引入：`import { mountKnowledgeSearch, triggerKnowledgeSearch } from './knowledge-search.js'`
2. entry 打开时（现有 `openViewer` / `showDoc` 函数，文档渲染完成后）：
   ```js
   mountKnowledgeSearch(document.getElementById('knowledge-panel'))
   triggerKnowledgeSearch(entry)
   ```
3. 关闭 viewer 时无需特殊清理（下次打开重新 mount）

---

## Verification 检查单

| 步骤 | 验证方式 |
|------|---------|
| V1 | `meilisearch --master-key=xxx` 启动，`curl localhost:7700/health` 返回 `{"status":"available"}` |
| V2 | `python3 scripts/build_knowledge_index.py --wipe` 输出 doc_count，无报错 |
| V3 | `curl "localhost:8765/api/search-knowledge?q=TPM"` 返回含 hits 的 JSON |
| V4 | 修改 `.cache/meili-meta.json` last_indexed_at 为 48h 前，打开任意 Entry，面板显示 ⚠ 过期 banner |
| V5 | 点「立即同步」，按钮 disabled，进度条出现，约 30s 后完成，结果刷新 |
| V6 | Settle 一条测试文档，立即搜索该文档标题，新文档出现在结果中 |
| V7 | 停止 Meilisearch 进程，打开 Entry，`#knowledge-panel` 隐藏，viewer 正常使用不受影响 |
| V8 | 打开任意 Entry，右侧面板自动搜索 + 展示结果，点击卡片在新 Tab 打开 GitHub 链接 |
