# WO-C3 · 前端弹窗 + viewer 按钮 + index.html

**依赖**：WO-C2（`moveToProject` / `fetchTopics` 已在 api.js 中导出）  
**新建文件**：`js/components/modals/move-project-dialog.js`  
**修改文件**：`js/components/viewer.js`、`index.html`

---

## 步骤 1 · 新建 move-project-dialog.js

新建文件 `js/components/modals/move-project-dialog.js`，内容：

```js
import { state, getEntryId } from '../../state.js'
import * as api from '../../api.js'

// ── openMoveProjectDialog ──────────────────────────────────────────────────

export async function openMoveProjectDialog() {
  const entry = state.viewer.entry;
  if (!entry) return;

  const currentProject = entry.common_path.split('/')[0];
  const result = document.getElementById('move-project-result');
  result.textContent = '加载项目列表…';
  result.style.color = '#8c959f';
  document.getElementById('move-project-dialog').classList.add('open');

  // Load topics
  let projects = [];
  try {
    const data = await api.fetchTopics();
    projects = data.topics.map(t => t.dir || (t.repo ? t.repo.split('/').pop() : null)).filter(Boolean);
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = '加载失败：' + e.message;
    return;
  }

  // Render project list
  const list = document.getElementById('move-project-list');
  list.innerHTML = '';
  for (const proj of projects) {
    if (proj === currentProject) continue;
    const btn = document.createElement('button');
    btn.className = 'move-project-item';
    btn.textContent = proj;
    btn.addEventListener('click', () => doMoveProject(proj));
    list.appendChild(btn);
  }
  result.textContent = `当前项目：${currentProject}，选择目标项目`;
  result.style.color = '#57606a';
}

export function closeMoveProjectDialog() {
  document.getElementById('move-project-dialog').classList.remove('open');
  document.getElementById('move-project-list').innerHTML = '';
  document.getElementById('move-project-result').textContent = '';
}

async function doMoveProject(newProject) {
  const entry = state.viewer.entry;
  if (!entry) return;

  const result = document.getElementById('move-project-result');
  result.style.color = '#57606a';
  result.textContent = `移动中…`;

  // Disable all project buttons during operation
  document.querySelectorAll('.move-project-item').forEach(b => b.disabled = true);

  try {
    const id = getEntryId(entry);
    if (!id) throw new Error('entry id 不存在');
    const data = await api.moveToProject(id, newProject);
    if (!data.ok) throw new Error(data.error || 'failed');

    result.style.color = '#1a7f37';
    result.textContent = `✓ 已移动到 ${newProject}`;

    // Close viewer and reload
    setTimeout(() => {
      closeMoveProjectDialog();
      document.getElementById('md-modal').style.display = 'none';
      document.body.style.overflow = '';
      document.dispatchEvent(new CustomEvent('cta:reload'));
    }, 800);
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = `✗ ${e.message}`;
    document.querySelectorAll('.move-project-item').forEach(b => b.disabled = false);
  }
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-move-project-close').addEventListener('click', closeMoveProjectDialog);
document.getElementById('move-project-backdrop').addEventListener('click', closeMoveProjectDialog);
```

---

## 步骤 2 · 修改 viewer.js

在 `js/components/viewer.js` 中：

### 2a. 在文件顶部 import 区域追加

```js
import { openMoveProjectDialog } from './modals/move-project-dialog.js'
```

### 2b. 在 `openDoc()` 函数内，`modal.style.display = 'flex'` 这行之后，追加按钮显示控制

在 `openDoc()` 函数内，找到：
```js
  modal.style.display = 'flex';
  document.body.style.overflow = 'hidden';
```

在其后追加：
```js
  document.getElementById('btn-move-project').style.display = '';
```

### 2c. 在 `enterEditMode()` 中隐藏按钮

找到 `enterEditMode()` 中的：
```js
  document.getElementById('btn-edit').style.display = 'none';
```

在紧随其后追加：
```js
  document.getElementById('btn-move-project').style.display = 'none';
```

### 2d. 在 `exitEditMode()` 中恢复按钮

找到 `exitEditMode()` 中的：
```js
  document.getElementById('btn-edit').style.display = '';
```

在紧随其后追加：
```js
  document.getElementById('btn-move-project').style.display = '';
```

### 2e. 在文件末尾（现有事件绑定区域附近）追加

```js
document.getElementById('btn-move-project').addEventListener('click', openMoveProjectDialog);
```

---

## 步骤 3 · 修改 index.html

### 3a. 在 `#md-header` 中新增按钮

找到：
```html
      <button id="md-close" class="md-header-btn">✕ 关闭</button>
```

在其**之前**插入：
```html
      <button class="md-header-btn" id="btn-move-project" style="display:none">↷ 切换项目</button>
```

### 3b. 在 `<!-- Delete confirm dialog -->` 之后、`<script>` 之前插入

```html
<!-- Move project dialog -->
<div id="move-project-dialog">
  <div id="move-project-backdrop"></div>
  <div id="move-project-dialog-box">
    <h3>↷ 切换项目</h3>
    <div id="move-project-result" style="font-size:12px;color:#57606a;margin-bottom:8px;"></div>
    <div id="move-project-list" style="display:flex;flex-direction:column;gap:4px;max-height:300px;overflow-y:auto;"></div>
    <div id="move-project-dialog-actions">
      <button id="btn-move-project-close">取消</button>
    </div>
  </div>
</div>
```

### 3c. 在 main.js 中 import（不新增 `<script>` 标签）

现有 delete-dialog.js 等模块均通过 viewer.js / main.js 的 import 链加载，**不使用独立 `<script>` 标签**，否则会导致模块重复初始化。

由于 move-project-dialog.js 的 Event listeners 在模块加载时立即执行，而 viewer.js 已经 import 了此文件（步骤 2a），不需要在 `index.html` 或 `main.js` 中单独添加任何 import 或 script 标签。

> **结论**：步骤 2a 将 `import { openMoveProjectDialog } from './modals/move-project-dialog.js'` 加入 viewer.js，即可完成全部加载链。**index.html 不需要新增 `<script>` 标签**。

---

## 验收

1. 打开任意文档 → `#md-header` 中出现 "↷ 切换项目" 按钮
2. 点击按钮 → 弹出项目列表，不含当前项目
3. 选择一个项目 → 显示"移动中…" → "✓ 已移动到 xxx"
4. 800ms 后 viewer 关闭，侧边栏重新加载，文档出现在新 project 的日期组中
5. 编辑模式下 "↷ 切换项目" 按钮隐藏，退出编辑模式后恢复
