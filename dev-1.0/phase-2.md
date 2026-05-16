# Phase 2 — state.js + main.js 骨架（1-2h）

> **前置确认**：Phase 1 已完成（`js/utils.js`、`js/api.js`、`js/constants.js` 存在，`npm test` 通过）。
> **回滚代价**：`rm js/state.js js/main.js`，移除 index.html 中新增的 `<script type="module">` 标签。

---

## Step 2.1 — 新建 js/state.js

**执行内容**

新建 `js/state.js`：

```js
export const state = {
  index: {
    data: null,
    groupedByDate: [],
    titleCache: new Map(),
    diffStatus: new Map(),
    annotations: {},
    titleFetchCache: new Map()
  },
  ui: {
    activeDate: null,
    archiveRoot: ''
  },
  viewer: {
    entry: null,
    layer: 'raw',
    rawText: '',
    annotation: {},
    commentEditCtx: null
  }
}

export function mergeAnnotations(indexData, summary) {
  for (const entry of Object.values(indexData)) {
    const ann = summary[entry.common_path]
    entry.done = ann?.done || undefined
    entry.importance = ann?.importance || undefined
    entry.links = ann?.links || undefined
    entry._comment_counts = ann?.comment_counts || undefined
  }
}

export function buildPathToId(indexData) {
  const map = new Map()
  for (const [id, entry] of Object.entries(indexData)) map.set(entry.common_path, id)
  return map
}
```

> ⏸️ **停止信号**：Step 2.1 完成。请确认 `js/state.js` 已创建，包含 `state` 导出和两个辅助函数。确认后回复"继续"。

---

## Step 2.2 — 新建 js/main.js（骨架）

**执行内容**

新建 `js/main.js`（骨架，暂时不执行任何业务逻辑）：

```js
import { state } from './state.js'
// 其他 import 在后续 Phase 逐步填入

document.addEventListener('DOMContentLoaded', () => {
  // Phase 3+ 逐步接管 index.html 的初始化逻辑
})
```

> ⏸️ **停止信号**：Step 2.2 完成。请确认 `js/main.js` 已创建。确认后回复"继续"。

---

## Step 2.3 — index.html 新增 module 脚本标签

**执行内容**

在 index.html `<body>` 末尾、现有 `<script>` 标签之前新增一行：

```html
<script type="module" src="js/main.js"></script>
```

**关键**：保留原有 `<script>` 不变，两者此阶段不冲突（module 脚本与普通脚本共存）。

> ⏸️ **停止信号**：Step 2.3 完成。请验证：DevTools console 无报错（特别检查 ES Module 加载正常），checklist 1-42 全部通过。确认通过后回复"继续"。

---

## 阶段完成 — 收尾

**Commit**

```
refactor(phase2): add state.js skeleton + main.js entry
```
