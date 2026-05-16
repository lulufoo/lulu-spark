# Global Search — Workbench 模式升级

## TL;DR

在现有全局搜索（知识库模式）基础上新增 Workbench 模式。输入 `#` 前缀自动切换；独立 Meilisearch index `workbench` 存全文；点击结果自动导航到对应日期并打开 Item；手动 ↺ 重建索引。

---

## Phases

### Phase 1 — 索引构建

**1-A. 新建 `scripts/build_workbench_index.py`**

扫描本地目录：`raw/`、`distilled/`、`digest/`、`diagnose/`（`annotations/` 仅含 `.json` 文件，排除）

每个 `.md` 文件提取字段：

| 字段 | 来源 |
|------|------|
| `id` | 基于 `layer + common_path` 生成（与 knowledge 索引的 id 命名规则一致） |
| `layer` | 路径第一段（raw / distilled / digest / annotations / diagnose） |
| `common_path` | 去掉 layer 前缀的相对路径（与 index.json 中的 `common_path` 保持一致） |
| `title` | 文件内容首个 `# ` 标题；无则取文件名去掉时间戳前缀 |
| `date` | 文件名前 8 位数字，用 `re.match(r'(\d{8})', filename)` 提取（YYYYMMDD） |
| `topic` | common_path 第一段路径（如 `ai-software-dev`） |
| `body` | 文件全文 |

写入 Meilisearch index `workbench`，filterable attributes: `layer`、`common_path`。支持 `--wipe` 标志（清空 index）。参考 `build_knowledge_index.py` 的 `_req`、`_wait_for_task`、`_ensure_index` 结构。每次 rebuild 全量，无增量。

**1-B. `server.py` 新增**

- `_reindex_wb_job` 全局变量（与 `_reindex_job` 结构相同）
- 三个新接口（注意挂载位置）：
  - `GET /api/search-workbench` → 挂 `do_GET`，用 `parsed_path` 匹配；Meilisearch `workbench` index 搜索，返回 `{hits}`
  - `POST /api/reindex-workbench` → 挂 `do_POST`，用 `self.path` 匹配；后台线程运行 `build_workbench_index.py --wipe`
  - `GET /api/reindex-workbench-status` → 挂 `do_GET`，用 `parsed_path` 匹配；返回 `_reindex_wb_job`

---

### Phase 2 — 前端接口层

**`js/api.js` 新增：**

```js
searchWorkbench(q, limit = 10)
reindexWorkbench()
getReindexWorkbenchStatus()
```

---

### Phase 3 — 模式切换 UI

**`js/components/global-search.js` 改动：**

- `input.value.startsWith('#')` → workbench 模式，去掉 `#` 后为实际查询 `q`；否则知识库模式
- `q.length === 0`（即仅输入 `#`）时不发请求，显示「输入关键词开始搜索」
- workbench 模式时：
  - 显示 `.gs-mode-pill`（`Workbench` 标识）
  - 显示 `#gs-rebuild-btn`（↺）
  - 调 `searchWorkbench`
- 知识库模式时：调 `searchKnowledge`，隐藏 pill 和 ↺
- Workbench hit 渲染：`<div class="gs-hit gs-hit-wb" data-common-path="...">` + 点击 dispatch `cta:open-entry`
- ↺ 按钮：触发重建 + 轮询 status，期间旋转动画

**`index.html` 改动：**

在 `#gs-wrap` 内加：

```html
<button id="gs-rebuild-btn" style="display:none" title="重建 Workbench 索引">↺</button>
```

---

### Phase 4 — 导航

**`js/main.js` 新增监听：**

```js
document.addEventListener('cta:open-entry', ({ detail }) => {
  const entry = Object.values(state.index.data)
    .find(e => e.common_path === detail.common_path)
  if (!entry) return
  const date = entry.created_at?.slice(0, 8)
  if (date) selectDate(date)
  window.openDoc(entry, entry.layers?.[0] || 'raw')  // selectDate 是同步的，openDoc 不依赖 doc list DOM，无需 setTimeout
})
```

annotations / diagnose 文件若无对应 index.json 条目，点击仅展示结果，不导航。

---

### Phase 5 — 样式

**`app.css` 新增：**

- `#gs-rebuild-btn`：与 header 深色按钮风格一致
- `.gs-mode-pill`：小标签，显示当前模式（Workbench）
- `.gs-hit-wb`：layer badge 颜色区分（`raw`=绿、`distilled`=蓝、`digest`=橙、`annotations`=紫、`diagnose`=灰）
- `#gs-rebuild-btn.syncing`：旋转动画

---

## Relevant Files

| 文件 | 操作 |
|------|------|
| `scripts/build_workbench_index.py` | 新建，参考 `build_knowledge_index.py` |
| `server.py` | 参考 `_handle_search_knowledge`、`_handle_reindex_knowledge` |
| `js/api.js` | 新增 3 个函数 |
| `js/components/global-search.js` | 主改动文件 |
| `js/main.js` | 新增 `cta:open-entry` 监听 |
| `index.html` | `#gs-wrap` 内加 ↺ 按钮 |
| `app.css` | 新增样式块 |

---

## Verification Checklist

- [ ] `python3 scripts/build_workbench_index.py --wipe` 成功，输出各目录索引文件数
- [ ] `curl "http://localhost:8000/api/search-workbench?q=meilisearch&limit=3"` 返回 hits
- [ ] 浏览器：输入 `meilisearch` → 知识库结果；输入 `#meilisearch` → Workbench 结果 + ↺ 出现
- [ ] 点击 workbench 结果 → 左侧日期切换，文档自动打开
- [ ] 点 ↺ → 重建动画出现，完成后消失

---

## Design Decisions

- workbench 每次 rebuild 全量（`--wipe`），无增量（本地文件小、本地操作快）
- annotations / diagnose 无对应 index.json 条目时，点击不导航，仅展示
- 重建入口只在 Workbench 模式下的 ↺ 按钮，不放进 ⚙ 菜单
- 模式切换通过 `#` 前缀，无额外 UI 组件
