# Refactor 3.0 — 架构设计

---

## 一、整体架构

```
层3知识仓库（15 个 GitHub repos）
    已克隆至 /Users/lulu/Code/{repo-name}/
            ↓
    scripts/build_knowledge_index.py（首次全量 / 手动增量）
            ↓
    Meilisearch（本地 :7700 / 服务端 Docker）
            ↓
    server.py  /api/search-knowledge
               /api/reindex-knowledge（异步）
               /api/reindex-status
            ↓
    前端 js/components/knowledge-search.js
    viewer 右侧「相关知识」面板
```

settle 路径额外追加单条实时同步：

```
_handle_settle() → gh api 写入成功
    → _meili_upsert_doc()（内存中直接推送，不读磁盘）
```

---

## 二、搜索引擎选型

**Meilisearch**

| 维度 | 说明 |
|------|------|
| 中文分词 | 内置 charabia tokenizer，开箱支持中文，无需 jieba 预处理 |
| 容错搜索 | 原生拼写容错 |
| 部署 | 本地：`brew install meilisearch`；服务端：Docker 单命令 |
| Python 接入 | 使用 stdlib `urllib.request`，无需安装 `meilisearch` Python 包 |
| 本地/服务端一致 | 同一套 HTTP API，切换仅改 URL 配置 |

---

## 三、配置文件

**`.cache/meili.env`**（不纳入 git）

```
MEILI_MASTER_KEY=your-key-here
MEILI_URL=http://localhost:7700
KNOWLEDGE_BASE_DIR=/Users/lulu/Code
MEILI_STALE_HOURS=24
```

服务端部署时同路径写入，`KNOWLEDGE_BASE_DIR` 改为服务端克隆路径。

---

## 四、Meilisearch 文档 Schema

索引名：`knowledge`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string（主键） | `{repo-name}__{urlsafe_path}`，唯一 |
| `title` | string | 文件第一个 H1，没有则用文件名（去 `.md`） |
| `body` | string | Markdown 原文 |
| `repo` | string | `lulufoo/ai-thinking-framework` |
| `path` | string | `topic/filename.md` |
| `url` | string | GitHub blob 完整链接 |
| `topic_desc` | string | `topics.json` 的 `description` 字段 |

可搜索字段：`title`、`body`、`topic_desc`（按权重降序）  
可展示字段：全部  
snippet 字段：`body`（裁取匹配上下文 80 字）

---

## 五、新增文件 & 改动清单

### 5.1 新建文件

| 文件 | 说明 |
|------|------|
| `scripts/build_knowledge_index.py` | 索引构建脚本，支持 `--wipe` 全量 / 无参增量 |
| `js/components/knowledge-search.js` | 前端搜索面板组件 |

### 5.2 修改文件

| 文件 | 改动 |
|------|------|
| `server.py` | 加载 `meili.env`；`_meili_request()` 工具函数；3 个新路由；settle upsert |
| `js/api.js` | 加 `searchKnowledge(q, limit=10)`、`reindexKnowledge()`、`getReindexStatus()` 三个函数 |
| `app.css` | `#md-panel` 改为横向 flex；新增 `#knowledge-panel` 样式 |
| `index.html` | `#md-panel` 内加 `<div id="knowledge-panel">` 挂载点 |
| `js/components/viewer.js` | entry 打开时触发初始搜索 |
| `.gitignore` | 追加 `.cache/meili.env` |

---

## 六、server.py 扩展详设

### 6.1 启动时加载配置

```python
# 启动时读取 .cache/meili.env
MEILI_URL = "http://localhost:7700"
MEILI_KEY = ""
KNOWLEDGE_BASE_DIR = Path("/Users/lulu/Code")
MEILI_STALE_HOURS = 24
```

### 6.2 `_meili_request(method, path, body=None)`

使用 `urllib.request.Request`，携带 `Authorization: Bearer {MEILI_KEY}`，JSON 序列化 body，返回解析后的 dict。Meilisearch 不可达时返回 `None`（不抛异常，不影响主流程）。

### 6.3 新路由

```
GET  /api/search-knowledge?q=...&limit=10
POST /api/reindex-knowledge
GET  /api/reindex-status
```

### 6.4 `GET /api/search-knowledge`

```
1. 读 .cache/meili-meta.json，计算过期
2. 调 Meilisearch POST /indexes/knowledge/search
3. 返回：
   {
     "hits": [{ "title", "repo", "url", "_snippetResult" }],
     "stale": bool,
     "last_indexed_at": "ISO8601"
   }
```

### 6.5 `POST /api/reindex-knowledge`（异步）

```
1. 检查 _reindex_job["status"] == "running" → 返回 409
2. 起 threading.Thread：
   a. git pull 所有层3 repo（subprocess，逐个执行）
   b. subprocess 运行 build_knowledge_index.py
3. 立即返回 { "status": "running" }
```

### 6.6 `GET /api/reindex-status`

返回模块级 `_reindex_job` 字典：

```json
{
  "status": "running | done | error | idle",
  "started_at": "ISO8601",
  "finished_at": "ISO8601 | null",
  "log": "最后一条日志"
}
```

### 6.7 `_handle_settle()` 追加 upsert

Step1（`_gh_put_file`）成功后：

```python
self._meili_upsert_doc(
    repo=target_repo,          # "lulufoo/xxx"
    path=dst_path,             # "theme/ts-slug.md"
    content=full_content,      # 内存中已有
    topic_desc=...             # 从 topics.json 查
)
# 注意：不更新 meili-meta.json 的 last_indexed_at
```

---

## 七、索引构建脚本设计（`build_knowledge_index.py`）

```
读 topics.json → 筛选有 repo 字段的条目
    for repo in repos:
        local_dir = KNOWLEDGE_BASE_DIR / repo.split("/")[-1]
        for md_file in local_dir.rglob("*.md"):
            跳过黑名单（_index.md, README.md）
            解析 title（第一个 H1 or 文件名）
            构建文档 dict
    每 100 条批量 PUT /indexes/knowledge/documents
写 .cache/meili-meta.json（last_indexed_at, doc_count）
```

参数：
- `--wipe`：执行前先 `DELETE /indexes/knowledge`
- 无参数：直接 upsert（Meilisearch 按 id 自动去重覆盖）

---

## 八、前端设计（`knowledge-search.js`）

### 8.1 面板 UI 结构

```
#knowledge-panel
├── .ks-search-bar
│   └── <input> 搜索框（300ms 防抖）
├── .ks-stale-banner（stale=true 时显示）
│   ├── "⚠ 索引 26h 未更新"
│   └── <button> 立即同步
├── .ks-sync-progress（同步中显示）
│   └── "⟳ 同步中..."
└── .ks-results
    └── .ks-hit × N（整卡可点击，新 Tab 打开 GitHub）
        ├── .ks-hit-title
        ├── .ks-hit-repo（repo 标签）
        └── .ks-hit-snippet
```

### 8.2 同步流程

```
点击「立即同步」
    → POST /api/reindex-knowledge
    → 按钮变 disabled，显示进度条
    → setInterval 每 2s GET /api/reindex-status
    → status=done → clearInterval → 重新触发搜索 → 隐藏进度条
    → status=error → 显示错误信息
```

### 8.3 layout 调整（`index.html` + `app.css`）

**DOM 结构原则**：`#md-panel` 保持 `flex-direction: column` 不变，`#md-header` / `#md-commit-bar` / `#md-links-bar` 继续作为其直接子元素（全宽）。仅在 body 区域外包一层横向容器。

**`index.html` 改动**（`#md-panel` 内部）：

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

**`app.css` 新增**：

```css
/* 新增：body 区域横向分栏包裹层 */
#md-content-row {
  display: flex;
  flex-direction: row;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

/* #md-body 在新容器内保持 flex:1 + overflow-y:auto（原有样式不变） */
#md-body {
  flex: 1;
  overflow-y: auto;
}

/* 新增：右侧知识面板 */
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

---

## 九、Verification

| 步骤 | 验证目标 |
|------|---------|
| `meilisearch --master-key=xxx` 启动 | `localhost:7700` 返回 200 |
| `python3 scripts/build_knowledge_index.py --wipe` | 输出入库文档数，无报错 |
| `curl "localhost:8765/api/search-knowledge?q=TPM"` | 返回包含 `hits` 的 JSON |
| 改 `meili-meta.json` 时间为旧值 | 面板显示过期 banner |
| 点「立即同步」 | 按钮 loading → 完成 → 结果刷新 |
| Settle 一条测试文档 | 立即搜索，新文档出现在结果中 |
| 打开任意 Entry | 面板自动以 title+slug 触发搜索并展示结果 |
