# Phase 3 — 迁移 Archive 核心（2-3h，最高风险阶段）

> **前置确认**：Phase 2 已完成（`js/state.js`、`js/main.js` 存在，DevTools console 无报错，checklist 1-42 通过）。
> **执行原则**：每迁移一个 Step，立即验证，通过后再继续，绝不批量移动。

> **全局变量映射规则（Phase 3-4 所有 Step 必须遵守）**：每迁移一个函数到模块，将函数体内的全局变量引用替换为 `state.*`：
>
> | 全局变量 | 替换为 |
> |---|---|
> | `indexData` | `state.index.data` |
> | `groupedByDate` | `state.index.groupedByDate` |
> | `titleCache` | `state.index.titleCache` |
> | `diffStatus` | `state.index.diffStatus` |
> | `annotationsSummary` | `state.index.annotations` |
> | `activeDate` | `state.ui.activeDate` |
> | `archiveRoot` | `state.ui.archiveRoot` |
> | `currentEntry` | `state.viewer.entry` |
> | `currentLayer` | `state.viewer.layer` |
> | `currentRawText` | `state.viewer.rawText` |
> | `currentAnnotation` | `state.viewer.annotation` |
> | `_commentEditCtx` | `state.viewer.commentEditCtx` |
> | `titleFetchCache` | `state.index.titleFetchCache` |

---

## Step 3.1 — 将 index.html `<script>` 升级为 module（关键切换点）

> ⚠️ **这是整个重构中风险最高的单步操作**。原 `<script>` 不是 module，无法使用 `import`，必须先完成此步骤才能开始后续迁移。

**执行内容**

1. 移除 Phase 2 添加的 `<script type="module" src="js/main.js">` 标签行
2. 将 index.html 原有的 `<script>` 改为 `<script type="module">`
3. 脚本顶部添加：
   ```js
   import { state } from './js/state.js'
   import { escHtml, slugToTitle, filenameFromPath, topicFromPath, formatDate, timeFromTs, nowTs, importanceBadgeHtml } from './js/utils.js'
   import { REPO, LAYERS, IMPORTANCE_CYCLE } from './js/constants.js'
   ```
4. 删除脚本中对应的 8 个 utils 函数定义 + 3 个常量定义（`REPO` / `LAYERS` / `IMPORTANCE_CYCLE`）
5. 将原 main.js 的 DOMContentLoaded 骨架合并至脚本顶部（**main.js 文件保留**，Phase 4 Step 4.10 将完整接管）
6. 修复 `showError` 函数中的 inline handler（L1401）：将 `onclick="loadIndex()"` 改为动态创建按钮并绑定 `addEventListener`，因为 module 脚本函数不再挂载到全局作用域，inline handler 会静默失效
7. 在脚本末尾添加 **window bridge**（`module` 脚本函数不自动挂 `window`，跨模块过渡期需显式赋值）：
   ```js
   // ── Window bridge（过渡期跨模块调用，随函数迁移逐步移除）──
   window.openDoc          = openDoc           // cards.js 需要；Step 4.1 后由 viewer.js 末尾重新赋值
   window.toggleDone       = toggleDone        // cards.js 需要；Step 3.7 后移除
   window.cycleImportance  = cycleImportance   // cards.js 需要；Step 3.7 后移除
   window.renderDocList    = renderDocList     // sidebar.js/selectDate 需要；Step 3.6 后移除
   window.loadTitles       = loadTitles        // sidebar.js/selectDate 需要；Step 3.8 后移除
   window.exitEditMode     = exitEditMode      // viewer.js 需要；Step 4.2 后移除
   window.hideCommitBar    = hideCommitBar     // viewer.js 需要；Step 4.3 后移除
   window.showCommitBar    = showCommitBar     // viewer.js 需要；Step 4.3 后移除
   window.renderLinksBar   = renderLinksBar    // viewer.js 需要；Step 4.5 后移除
   window.renderComments   = renderComments    // viewer.js 需要；Step 4.6 后移除
   window.openDeleteDialog = openDeleteDialog  // viewer.js 需要；Step 4.7 后移除
   ```

> **注意**：`module` 脚本自动延迟执行（等效 `defer`），与原 `<script>` 在 DOM 就绪后执行的行为一致，不影响初始化逻辑。

> ⏸️ **停止信号**：Step 3.1 完成。请验证：DevTools console 无报错，checklist 1-4 全部通过。确认通过后回复"继续"。

---

## Step 3.2 — 启用 api.js（替换 fetch 调用）

**执行内容**

1. 脚本顶部添加：`import * as api from './js/api.js'`
2. 将脚本中所有内联 `fetch('/api/...')` 和 `fetch('./...')` 调用替换为对应的 `api.xxx()` 封装
3. **注意**：`commitFiles` 有两处调用，语义不同——单文件（传 `files` 数组，L2154）vs 全量（不传 `files`，L2300）

> ⏸️ **停止信号**：Step 3.2 完成。请验证：checklist 1-4，以及至少一个 API 操作（如加载索引、刷新状态）正常。确认通过后回复"继续"。

---

## Step 3.3 — 迁移 buildGroups 到 js/components/sidebar.js

> ⚠️ `buildGroups` 当前直接读取全局 `indexData` 并 mutate 全局 `groupedByDate`，**迁移时须同步修改为纯函数形式**：

**执行内容**

新建 `js/components/sidebar.js`，迁移 `buildGroups`，签名改为接收参数、返回结果：

```js
export function buildGroups(indexData) {
  // 原逻辑不变
  return [...map.entries()].sort(...).map(...)
}
```

脚本中的调用方式改为：

```js
state.index.groupedByDate = buildGroups(state.index.data)
```

删除 index.html 脚本中原有的 `buildGroups` 函数定义。

> ⏸️ **停止信号**：Step 3.3 完成。请验证：checklist 2（侧边栏日期列表，count 数字正确）。确认通过后回复"继续"。

---

## Step 3.4 — 迁移 renderSidebar 到 sidebar.js

**执行内容**

将 `renderSidebar` 迁移到 `js/components/sidebar.js`。注意此函数读取 `groupedByDate`，迁移时须切换为 `state.index.groupedByDate`（而非全局变量）。

> ⏸️ **停止信号**：Step 3.4 完成。请验证：checklist 2-3（侧边栏渲染、Tab 切换正常）。确认通过后回复"继续"。

---

## Step 3.5 — 迁移 selectDate 到 sidebar.js

**执行内容**

将 `selectDate` 迁移到 `js/components/sidebar.js`。

> **跨模块调用**：`selectDate` 调用 `renderDocList`（Step 3.6 迁移）和 `loadTitles`（Step 3.8 迁移），当前均通过 Step 3.1 window bridge 过渡（`window.renderDocList()`、`window.loadTitles()`）。

> ⏸️ **停止信号**：Step 3.5 完成。请验证：checklist 3-4（Tab 切换 + Session 恢复）。确认通过后回复"继续"。

---

## Step 3.6 — 新建 js/components/cards.js，迁移 buildCard 及辅助函数

**执行内容**

新建 `js/components/cards.js`，同时迁移以下函数（它们相互依赖，必须一起迁移）：

- `getEntryDiffState`（L1011）— 读取 diffStatus，被 buildCard 调用
- `getLayerBadgeClass`（L1022）— 读取 diffStatus，被 buildCard 调用
- `buildCard`（L1245）— 主卡片构建函数
- `renderDocList`（L1229）— 调用 buildCard，被 selectDate 调用
- `updateDiffInDOM`（L1040）— 查询卡片 DOM，调用 getEntryDiffState / getLayerBadgeClass；**Phase 4 中 viewer.js 需要 import 此函数，须在此处一并迁移**

> **注意**：`sidebar.js` 的 `selectDate` 需要 `import { renderDocList } from './cards.js'`，即 sidebar 依赖 cards，而非反向。迁移完成后 sidebar.js 内将 `window.renderDocList()` 替换为直接调用，并删除 index.html 中的 `window.renderDocList = renderDocList`。

**过渡方案**：`buildCard` 的 click 事件调用 `openDoc`（Step 4.1 才迁入 viewer.js）、`toggleDone`（Step 3.7 迁移）、`cycleImportance`（Step 3.7 迁移），这些函数尚未进入 cards.js，统一使用 `window.openDoc()` / `window.toggleDone()` / `window.cycleImportance()` 调用（Step 3.1 的 window bridge 保证可用）。

> ⏸️ **停止信号**：Step 3.6 完成。请验证：checklist 5-7（卡片徽章、注解、颜色条正常）。确认通过后回复"继续"。

---

## Step 3.7 — 迁移 toggleDone、cycleImportance、attachBadgeListeners 到 cards.js

**执行内容**

将三个函数迁移到 `js/components/cards.js`，移除 `window.xxx` 过渡引用。

**window bridge 清理**：`toggleDone` 和 `cycleImportance` 已进入 cards.js，
1. 删除 index.html 模块脚本中的 `window.toggleDone = toggleDone` 和 `window.cycleImportance = cycleImportance`
2. cards.js 内 `buildCard` 的调用从 `window.toggleDone()` / `window.cycleImportance()` 改为直接调用（同模块）

> ⏸️ **停止信号**：Step 3.7 完成。请验证：checklist 9（Importance 颜色条）、10（Done 灰化）、13（Importance 循环）、14（Done 切换）。确认通过后回复"继续"。

---

## Step 3.8 — 迁移 loadTitles、updateTitlesInDOM 到 cards.js

**执行内容**

将两个函数迁移到 `js/components/cards.js`。

**关键**：`loadTitles` 内部有一处独立的本地文件 fetch（`./raw/{commonPath}`），迁移时**必须**替换为 `api.fetchFileContent('raw', entry.common_path)`，不可遗漏。

**sidebar bridge 清理**：`loadTitles` 已进入 cards.js，sidebar.js 内将 `window.loadTitles()` 改为 `import { loadTitles } from './cards.js'` 并直接调用；同时删除 index.html 中的 `window.loadTitles = loadTitles`。

> ⏸️ **停止信号**：Step 3.8 完成。请验证：checklist 5（shimmer → 标题加载正常）。确认通过后回复"继续"。

---

## Step 3.9 — 清理 index.html

**执行内容**

删除 index.html 脚本中所有已迁移到 Phase 3 的函数定义，保留尚未迁移的部分。

> ⏸️ **停止信号**：Step 3.9 完成。请验证：checklist 1-14 全部通过，console 无报错。确认通过后回复"继续"。

---

## 阶段完成 — 收尾

**Commit**

```
refactor(phase3): migrate sidebar + cards to ES modules
```
