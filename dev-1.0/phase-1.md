# Phase 1 — 纯函数层 + Vitest（2-3h）

> **前置确认**：Phase 0 已完成（`app.css` 存在，界面正常）。
> **回滚代价**：`rm -rf js/utils.js js/api.js js/constants.js tests/ package.json package-lock.json node_modules/`

---

## Step 1.1 — 初始化测试环境

**执行内容**

```bash
npm init -y
npm i -D vitest
```

在 `package.json` 中加入：

```json
"scripts": { "test": "vitest run" }
```

> ⏸️ **停止信号**：Step 1.1 完成。请验证：`npm test` 可以运行（无测试文件时输出 "No test files found" 而非报错）。确认通过后回复"继续"。

---

## Step 1.2 — 新建 js/utils.js

**执行内容**

新建 `js/utils.js`，从 `index.html` 迁移以下 8 个纯函数（函数体一字不改，只加 `export`）：

| 函数 | index.html 行号 | 特征 |
|---|---|---|
| `escHtml` | L1304 | 无副作用 |
| `slugToTitle` | L1073 | 无副作用 |
| `filenameFromPath` | L1081 | 无副作用 |
| `topicFromPath` | L1085 | 无副作用 |
| `formatDate` | L1091 | 无副作用 |
| `timeFromTs` | L1109 | 无副作用 |
| `importanceBadgeHtml` | L2054 | 无副作用 |
| `nowTs` | L1643 | 无副作用 |

**关键**：index.html 内同名函数**暂时保留**，utils.js 此阶段不被任何文件引用。

> ⏸️ **停止信号**：Step 1.2 完成。请确认 `js/utils.js` 已创建，包含 8 个 `export function`。确认后回复"继续"。

---

## Step 1.3 — 新建 tests/utils.test.js

**执行内容**

新建 `tests/utils.test.js`，覆盖每个函数的边界 case：

```js
import { escHtml, formatDate, timeFromTs, slugToTitle, nowTs, importanceBadgeHtml, filenameFromPath, topicFromPath } from '../js/utils.js'

test('escHtml 转义 HTML 特殊字符', () => {
  expect(escHtml('<script>alert(1)</script>')).toBe('&lt;script&gt;alert(1)&lt;/script&gt;')
})

test('timeFromTs 从12位时间戳提取 HH:MM', () => {
  expect(timeFromTs('202605041430')).toBe('14:30')
})

// ... 其余函数的边界 case
```

> ⏸️ **停止信号**：Step 1.3 完成。请运行 `npm test`，确认所有测试通过。确认通过后回复"继续"。

---

## Step 1.4 — 新建 js/api.js

**执行内容**

新建 `js/api.js`，将以下 fetch 调用封装为命名函数（此阶段不被任何文件引用）：

| 函数名 | 对应 endpoint | 说明 |
|---|---|---|
| `fetchIndex()` | `./index.json` | GET |
| `fetchDiffStatus()` | `/api/status` | GET |
| `fetchAnnotationsSummary()` | `/api/annotations` | GET |
| `fetchAnnotation(path)` | `/api/annotation?path=...` | GET |
| `fetchConfig()` | `/api/config` | GET |
| `fetchFileContent(layer, commonPath)` | `./{layer}/{commonPath}` | GET，返回文本 |
| `fetchLinkTitle(url)` | `/api/fetch-title?url=...` | GET |
| `saveFile(layer, commonPath, content)` | `/api/save` | POST |
| `commitFiles(message, files?)` | `/api/commit` | POST；`files` 省略时提交全部变更，为数组时仅提交指定文件（两处调用：单文件 L2154、全量 L2300，语义不同） |
| `pullProject()` | `/api/pull` | POST |
| `updateComments(commonPath, layer, comment, ts)` | `/api/update-comments` | POST |
| `updateLinks(commonPath, links)` | `/api/update-links` | POST |
| `setImportance(commonPath, importance)` | `/api/set-importance` | POST |
| `setDone(commonPath, done)` | `/api/set-done` | POST |
| `deleteEntry(id)` | `/api/delete` | POST |
| `ghMove(srcUrl, dstDirUrl)` | `/api/gh-move` | POST |

> ⏸️ **停止信号**：Step 1.4 完成。请确认 `js/api.js` 已创建，包含以上所有导出函数。确认后回复"继续"。

---

## Step 1.5 — 新建 tests/api.test.js

**执行内容**

新建 `tests/api.test.js`，mock `globalThis.fetch`，验证每个函数的入参格式和返回值处理。

> ⏸️ **停止信号**：Step 1.5 完成。请运行 `npm test`，确认所有测试（含 api.test.js）通过。确认通过后回复"继续"。

---

## Step 1.6 — 新建 js/constants.js

**执行内容**

新建 `js/constants.js`：

```js
export const REPO = 'https://github.com/lulufoo/lulu-workbench/blob/main'
export const LAYERS = ['raw', 'distilled', 'digest', 'trace']
export const IMPORTANCE_CYCLE = [undefined, 'high', 'medium', 'low']
```

**关键**：index.html 中对应的 `const REPO`（L996）、`const LAYERS`（L997）、`const IMPORTANCE_CYCLE`（L2065）**暂时保留**，与 utils.js / api.js 的处理方式一致。

> ⏸️ **停止信号**：Step 1.6 完成。请验证：`npm test` 仍通过，checklist 1-42 全部通过（index.html 功能无任何变化）。确认通过后回复"继续"。

---

## 阶段完成 — 收尾

**Commit**

```
refactor(phase1): extract utils.js + api.js + constants.js, add vitest
```
