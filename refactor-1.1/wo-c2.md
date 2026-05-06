# WO-C2 · js/api.js — 新增函数

**依赖**：WO-C1（/api/move-project 已实现）  
**修改文件**：`js/api.js`

---

## 步骤

在文件末尾（`deleteEntry` 函数之后）追加以下两个函数：

```js
export async function fetchTopics() {
  const res = await fetch('./topics.json?_=' + Date.now());
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function moveToProject(id, newProject) {
  const res = await fetch('/api/move-project', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id, new_project: newProject })
  });
  return res.json();
}
```

---

## 定位点

在 `js/api.js` 中找到末尾的 `ghMove` 或 `updateHighlight` 函数，在其后追加。  
不修改已有任何函数。

---

## 验收

浏览器 DevTools Console 中：

```js
import { fetchTopics, moveToProject } from './js/api.js';
const t = await fetchTopics();
console.log(t.topics.length);  // 期望 15
```
