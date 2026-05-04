# 重构框架 v1.0

> 定位：引导、约束重构过程的高层抽象框架。
> 在重构过程中遇到任何判断题，优先从本框架中找答案，而不是靠直觉。

---

## 一、目标态定义（Target State）

描述重构完成后代码的终态结构。这是框架的"北极星"。

### 文件结构

```
index.html              ← DOM 骨架（~50行）+ <link> + <script type="module">
app.css                 ← 所有样式（原样迁移）
js/
  state.js              ← 唯一全局状态对象（含 getEntryId、loadDiffStatus 导出函数；依赖 api.js）
  api.js                ← 所有 fetch 调用封装（纯 async，无 DOM 依赖）
  utils.js              ← 纯函数（无副作用，无 DOM/fetch 依赖）
  constants.js          ← 全局常量：REPO、LAYERS、IMPORTANCE_CYCLE（L996/997/2065）
  components/
    sidebar.js          ← 侧边栏：renderSidebar, selectDate, buildGroups
    cards.js            ← 卡片：buildCard, renderDocList, getEntryDiffState, getLayerBadgeClass, attachBadgeListeners, updateDiffInDOM, loadTitles, updateTitlesInDOM, toggleDone, cycleImportance
    viewer.js           ← 文档查看器：openDoc, renderDocBody, editMode, saveDoc
    comments.js         ← 笔记：renderComments, buildCommentItem, openCommentDialog, closeCommentDialog
    links-bar.js        ← 链接栏：renderLinksBar, showAddLinkInput, confirmDeleteLink
    modals/
      delete-dialog.js
      commit-dialog.js
      move-dialog.js
  feed.js               ← follow-builders feed（纯新增）
  main.js               ← 入口：DOMContentLoaded + 所有 addEventListener
tests/
  utils.test.js         ← 纯函数单元测试
  api.test.js           ← mock fetch，验证 API 封装
```

### state.js 状态契约

当前 12 个散落全局变量的归宿：

```js
export const state = {
  index: {
    data: null,                  // 原 indexData
    groupedByDate: [],           // 原 groupedByDate
    titleCache: new Map(),       // 原 titleCache
    diffStatus: new Map(),       // 原 diffStatus
    annotations: {},             // 原 annotationsSummary
    titleFetchCache: new Map()   // 原 titleFetchCache
  },
  ui: {
    activeDate: null,            // 原 activeDate
    archiveRoot: ''              // 原 archiveRoot
  },
  viewer: {
    entry: null,                 // 原 currentEntry
    layer: 'raw',                // 原 currentLayer
    rawText: '',                 // 原 currentRawText
    annotation: {},              // 原 currentAnnotation
    commentEditCtx: null         // 原 _commentEditCtx
  }
}
```

附加四个导出函数：

```js
// 替代 loadAnnotationsSummary 直接 mutate indexData 的行为
export function mergeAnnotations(indexData, summary) { ... }

// 替代 entry._id 反向 mutation
export function buildPathToId(indexData) → Map<common_path, id>

// 被 viewer.js(saveDoc) 调用，通过 common_path 反查条目 id
export function getEntryId(entry) { ... }

// 被 viewer.js(saveDoc/commitCurrentFile) 和 main.js 共同 import；内部调用 api.fetchDiffStatus()
export async function loadDiffStatus() { ... }
```

---

## 二、决策规则（Decision Rules）

**当重构过程中遇到"这个函数该放哪里""现在能动这个吗"等判断题时，用这两条规则回答。**

### 规则 1：模块边界规则

```
稳定层（state.js / api.js / utils.js）
  ↑ 被依赖，不依赖任何业务层
  
业务层（components/*.js / feed.js）
  ↓ 可以 import 稳定层，不能被稳定层 import
  
main.js
  ↓ 唯一的"组装层"，可以 import 所有模块，挂载所有事件
```

判断方式：如果一个函数需要操作 DOM 或读取业务状态，它属于业务层。如果它是无副作用的纯计算，它属于 `utils.js`。如果它封装 `fetch` 调用，它属于 `api.js`。

### 规则 2：迁移顺序规则

> **一个模块的所有依赖迁移完成之前，该模块不能迁移。**
> 迁移方向：从叶子节点（无依赖的纯函数）向根节点（main.js 入口）推进。

依赖层次图（从底到顶，按此顺序迁移）：

```
Layer 0: utils.js（无任何依赖）
Layer 1: api.js（依赖无 / 仅依赖 utils）
Layer 2: state.js（依赖 utils, api）
Layer 3: components/cards.js（依赖 state, api, utils）
Layer 4: components/sidebar.js（依赖 state, api, utils, cards）
Layer 5: components/viewer.js（依赖 state, api, utils）
Layer 5: components/comments.js（依赖 state, api, utils）
Layer 5: components/links-bar.js（依赖 state, api, utils）
Layer 6: components/modals/*.js（依赖 state, api, utils）
Layer 7: feed.js（无内部依赖，仅依赖 state, api）
Layer 8: main.js（依赖所有模块）
```

---

## 三、变更边界（Change Boundary）

**以下内容在整个重构过程中冻结，不允许修改。**

违反变更边界后出现的 bug，定位难度会成倍增加。

| 冻结对象 | 具体内容 | 原因 |
|---|---|---|
| **server.py API 契约** | 14个端点的 URL 路径和请求/响应格式不变 | api.js 封装基于这些契约 |
| **index.json 数据结构** | `entries`, `common_path`, `created_at`, `layers` 字段不变 | state.js 数据模型依赖它 |
| **HTML DOM id / class** | `#md-modal`, `#doc-list`, `#sidebar` 等所有 id 和功能性 class 不变 | JS 通过这些选取元素 |
| **CDN 引用** | `marked.min.js` 的引用方式不变 | viewer.js 依赖 `marked.parse()` |
| **目录结构** | `raw/`, `distilled/`, `digest/`, `trace/` 四层目录不变 | api.js 的路径拼接依赖它 |

---

## 四、验证机制（Verification Gate）

**每个阶段（Phase）结束后必须通过验证门，才能进入下一阶段。**

### 两层验证

```
层 1：自动化（5秒）
  npm test
  → Vitest 运行 utils.test.js + api.test.js
  → 全部通过才能继续

层 2：人工验证（15-20分钟）
  → 按 checklist.md 逐条验证
  → 全部通过才能 git commit 并进入下一 Phase
```

### 验证门与 Git Commit 的绑定关系

```
Phase 完成
  → 验证门通过
  → git commit（每 Phase 一个 commit）
  → 进入下一 Phase

验证门失败
  → git diff 定位本 Phase 内的变更
  → 修复或 git revert 到上一 Phase 的 commit
  → 重新开始当前 Phase
```

验证清单详见 `checklist.md`。

---

## 附：框架四部分关系图

```
目标态定义  ──→  告诉你"要去哪里"
决策规则    ──→  告诉你"每步怎么判断"
变更边界    ──→  告诉你"什么不能动"
验证机制    ──→  告诉你"有没有走偏"
```

三者缺一，重构过程都会在某个点失控：
- 无目标态：重构方向漂移
- 无决策规则：每步都要回来看计划
- 无变更边界：出问题定位不到原因
- 无验证机制：退步时无法察觉
