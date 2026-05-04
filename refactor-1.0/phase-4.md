# Phase 4 — 迁移 Viewer + Dialogs（2-3h）

> **前置确认**：Phase 3 已完成（checklist 1-14 通过，console 无报错）。
> **执行原则**：每迁移一个 Step，立即验证，通过后再继续。

---

## Step 4.0 — 补充迁移：state.js 新增 getEntryId + loadDiffStatus

**执行内容**

将以下两个函数迁移到 `js/state.js`（`saveDoc` 和 `commitCurrentFile` 在后续步骤迁入 viewer.js 时需要 import 它们，必须先完成此步）：

1. 将 `getEntryId`（L2115）作为 export function 加入 `state.js`，使用 `state.index.data` 替代原全局变量 `indexData`：

   ```js
   export function getEntryId(entry) {
     if (entry._id) return entry._id;
     for (const [id, e] of Object.entries(state.index.data || {})) {
       if (e.common_path === entry.common_path) return id;
     }
     return null;
   }
   ```

2. 将 `loadDiffStatus`（L1029）作为 export function 加入 `state.js`，调用已有的 `api.fetchDiffStatus()` 并更新 `state.index.diffStatus`（state.js 顶部加 `import * as api from './api.js'`）：

   ```js
   export async function loadDiffStatus() {
     try {
       const data = await api.fetchDiffStatus();
       state.index.diffStatus.clear();
       for (const p of (data.modified || [])) state.index.diffStatus.set(p, 'modified');
       for (const p of (data.conflicted || [])) state.index.diffStatus.set(p, 'conflict');
     } catch { /* non-critical */ }
   }
   ```

> ⏸️ **停止信号**：Step 4.0 完成。请确认 `js/state.js` 已新增 `getEntryId` 和 `loadDiffStatus` 两个 export function，`npm test` 仍通过。确认后回复"继续"。

---

## Step 4.1 — 迁移 openDoc、renderDocBody、postProcessLinks、resolveRelativeLink 到 viewer.js

**执行内容**

新建 `js/components/viewer.js`，迁移以上四个函数。`resolveRelativeLink`（L1428）是 `postProcessLinks` 的内部依赖，必须一起迁移。

**跨模块调用**（以下函数尚未迁移至 viewer.js，通过 Step 3.1 window bridge 过渡）：
- `openDoc` 内：`exitEditMode` → `window.exitEditMode()`，`hideCommitBar` → `window.hideCommitBar()`
- `renderDocBody` 内：`renderLinksBar` → `window.renderLinksBar()`，`renderComments` → `window.renderComments()`，`openDeleteDialog` → `window.openDeleteDialog()`

**window bridge 清理**：
- `openDoc` 已从 index.html 迁出 → 删除 index.html 中的 `window.openDoc = openDoc`
- 在 viewer.js 末尾添加 `window.openDoc = openDoc`（cards.js 仍通过 `window.openDoc` 调用它，不形成 circular import）

> ⏸️ **停止信号**：Step 4.1 完成。请验证：checklist 15, 16（Viewer 打开两种方式）、17（Markdown 渲染）、18（相对链接解析）。确认通过后回复"继续"。

---

## Step 4.2 — 迁移 enterEditMode、exitEditMode、saveDoc 到 viewer.js

**执行内容**

将三个函数迁移到 `js/components/viewer.js`。

**跨模块调用**：
- `saveDoc` 内的 `showCommitBar()` 尚未迁入 viewer.js（Step 4.3），改用 `window.showCommitBar()`
- `saveDoc` 内的 `updateDiffInDOM()` 已在 cards.js（Step 3.6）→ `import { updateDiffInDOM } from './cards.js'`
- `saveDoc` 内的 `updateTitlesInDOM()` 已在 cards.js（Step 3.8）→ `import { updateTitlesInDOM } from './cards.js'`

**window bridge 清理**：删除 index.html 中的 `window.exitEditMode = exitEditMode`（已迁出）。

> ⏸️ **停止信号**：Step 4.2 完成。请验证：checklist 22（进入编辑）、23（取消编辑）、24（保存文档）。确认通过后回复"继续"。

---

## Step 4.3 — 迁移 commitCurrentFile、showCommitBar、hideCommitBar 到 viewer.js

**执行内容**

将三个函数迁移到 `js/components/viewer.js`。

> **关键**：`commitCurrentFile` 内的 `await loadIndex()` 调用无法跨模块引用，改为触发自定义事件：
> ```js
> document.dispatchEvent(new CustomEvent('cta:reload'))
> ```
> main.js 在 Phase 4.10 中监听该事件并调用 `loadIndex()`，实现解耦，避免 viewer.js → main.js 循环依赖。

> **window bridge 清理**：删除 index.html 中的 `window.showCommitBar = showCommitBar`、`window.hideCommitBar = hideCommitBar`（已迁出）。viewer.js 内将 `window.showCommitBar()` 替换为直接调用（同模块）。

> ⏸️ **停止信号**：Step 4.3 完成。请验证：checklist 25（单文件提交流程正常）。确认通过后回复"继续"。

---

## Step 4.4 — 迁移 closeModal 和复制按钮逻辑到 viewer.js

**执行内容**

将 `closeModal` 和复制按钮相关逻辑迁移到 `js/components/viewer.js`。

> ⏸️ **停止信号**：Step 4.4 完成。请验证：checklist 19-21（Viewer 关闭三种方式）、26（复制 GitHub URL）、27（复制本地路径）。确认通过后回复"继续"。

---

## Step 4.5 — 新建 links-bar.js，迁移 renderLinksBar、showAddLinkInput、confirmDeleteLink

**执行内容**

新建 `js/components/links-bar.js`，迁移以上**五个**函数（注意实际函数名：`showAddLinkInput`（L1903）、`confirmDeleteLink`（L1984），而非 addLink/deleteLink）：

- `fetchTitle`（L1841）— `renderLinksBar` 内部依赖，调用 `/api/fetch-title`
- `fallbackTitle`（L1851）— `fetchTitle` 内部依赖，纯函数
- `renderLinksBar`（L1859）
- `showAddLinkInput`（L1903）
- `confirmDeleteLink`（L1984）

> **注意**：`fetchTitle` 内部使用了 `titleFetchCache`（全局变量），迁移时替换为 `state.index.titleFetchCache`（参照 preamble 映射表）。

**迁移后 viewer.js 更新**：将 `renderDocBody` 内的 `window.renderLinksBar()` 替换为：
```js
import { renderLinksBar } from './links-bar.js'
// 直接调用 renderLinksBar(...)
```

**window bridge 清理**：删除 index.html 中的 `window.renderLinksBar = renderLinksBar`（已迁出）。

> ⏸️ **停止信号**：Step 4.5 完成。请验证：checklist 28（显示已有链接）、29（添加链接）、30（删除链接）。确认通过后回复"继续"。

---

## Step 4.6 — 新建 comments.js，迁移 renderComments、buildCommentItem、openCommentDialog、closeCommentDialog

**执行内容**

新建 `js/components/comments.js`，迁移以上**五个**函数（注意实际函数名：`openCommentDialog`（L1742）、`closeCommentDialog`（L1767），而非 commentDialog）：

- `renderComments`
- `buildCommentItem`
- `openCommentDialog`（L1742）
- `closeCommentDialog`（L1767）
- `saveComment`（L1795）— 笔记保存核心逻辑，与上述四个函数共用 `_commentEditCtx`，**必须一起迁移**

**迁移后 viewer.js 更新**：将 `renderDocBody` 内的 `window.renderComments()` 替换为：
```js
import { renderComments } from './comments.js'
// 直接调用 renderComments(...)
```

**window bridge 清理**：删除 index.html 中的 `window.renderComments = renderComments`（已迁出）。

> ⏸️ **停止信号**：Step 4.6 完成。请验证：checklist 31（添加笔记）、32（Ctrl+Enter 保存）、33（编辑笔记）、34（删除笔记）。确认通过后回复"继续"。

---

## Step 4.7 — 新建 modals/delete-dialog.js

**执行内容**

新建 `js/components/modals/delete-dialog.js`，迁移删除对话框逻辑。

**关键 import**：
- `import { closeModal } from '../viewer.js'`（closeModal 在 Step 4.4 迁入 viewer.js）
- `import { getEntryId } from '../../state.js'`（getEntryId 在 Step 4.0 迁入 state.js）
- 成功删除后的 `await loadIndex()` → 改为 `document.dispatchEvent(new CustomEvent('cta:reload'))`

**迁移后 viewer.js 更新**：将 `renderDocBody` 内的 `window.openDeleteDialog()` 替换为：
```js
import { openDeleteDialog } from './modals/delete-dialog.js'
// 直接调用 openDeleteDialog()
```

**window bridge 清理**：删除 index.html 中的 `window.openDeleteDialog = openDeleteDialog`（已迁出）。

> ⏸️ **停止信号**：Step 4.7 完成。请验证：checklist 35（删除对话框弹出）、36（输入 CONFIRM 激活按钮）、37（执行删除）。确认通过后回复"继续"。

---

## Step 4.8 — 新建 modals/commit-dialog.js

**执行内容**

新建 `js/components/modals/commit-dialog.js`，迁移全量提交对话框逻辑。

**关键**：`doCommitChanges` 成功后的 `loadIndex()` 调用 → 改为 `document.dispatchEvent(new CustomEvent('cta:reload'))`。

> ⏸️ **停止信号**：Step 4.8 完成。请验证：checklist 40（提交变更对话框正常）。确认通过后回复"继续"。

---

## Step 4.9 — 新建 modals/move-dialog.js

**执行内容**

新建 `js/components/modals/move-dialog.js`，迁移移动对话框逻辑（实际函数名：`openMoveDocDialog`（L2383）、`closeMoveDocDialog`、`doMoveDoc`）。

**关键**：`doMoveDoc` 成功后如有 `loadIndex()` 调用 → 改为 `document.dispatchEvent(new CustomEvent('cta:reload'))`。

> ⏸️ **停止信号**：Step 4.9 完成。请验证：checklist 41（移动文档对话框正常）。确认通过后回复"继续"。

---

## Step 4.10 — main.js 接管所有 addEventListener，index.html 彻底清空

**执行内容**

将 index.html `<script type="module">` 中剩余的所有内容移入 `js/main.js`，包括：

- 所有 `addEventListener` 绑定
- `loadAnnotationsSummary`、`loadIndex`、`showError` 等初始化函数
- `fetchConfig` 初始化调用（当前在 L2457）
- `document.addEventListener('cta:reload', () => loadIndex())`（对应 Phase 4.3 中 `commitCurrentFile` 触发的事件）

index.html 的 script 标签只保留一行：

```html
<script type="module" src="js/main.js"></script>
```

> ⏸️ **停止信号**：Step 4.10 完成。请验证：`npm test` 通过，checklist 1-42 全部通过，console 无报错。确认通过后回复"继续"。

---

## 阶段完成 — 收尾

**Commit**

```
refactor(phase4): migrate viewer + all dialogs to ES modules
```
