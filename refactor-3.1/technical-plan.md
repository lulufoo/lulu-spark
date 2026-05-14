# Topic Filter 实现计划

**日期：** 2026-05-14
**版本：** v1.1
**对应规格：** `.cache/topic-filter-product.md` / `.cache/topic-filter-interaction.md`
**状态：** 草稿

---

## 文件一览

| 文件 | 职责 |
|------|------|
| `js/state.js` | 新增 `state.ui.activeTopic` 和 `state.index.filteredGroups` 两个字段 |
| `js/components/sidebar.js` | 新增 `_renderTopicFilter` / `selectTopic`，修改 `renderSidebar` / `selectDate` |
| `js/main.js` | `loadIndex()` 初始化时重置 topic 状态并修正 filteredGroups 引用 |
| `app.css` | 新增 `.topic-filter`、`.topic-select`、`.topic-count` 样式 |

---

## 架构速查（执行任何 Task 前必读）

### 模块边界与职责

```
js/state.js  ← 纯状态容器，无逻辑
  字段（新增）：state.ui.activeTopic: string | null
  字段（新增）：state.index.filteredGroups: GroupItem[]

js/components/sidebar.js  ← topic 筛选逻辑 + sidebar 渲染
  导出（新增）：selectTopic(key: string | null) → void
  导出（已有）：renderSidebar() → void
  导出（已有）：selectDate(date: string) → void
  内部：_renderTopicFilter(aside: HTMLElement) → void

js/main.js  ← 数据加载 + 初始化
  内部（非导出）：loadIndex() — 修改 buildGroups 后的初始化逻辑
```

### 依赖方向（单向，禁止反向）

```
main.js  →  sidebar.js（selectTopic, renderSidebar, selectDate）
sidebar.js  →  state.js（state）
main.js  →  state.js（state）
```

### 数据流

```
页面加载
  → loadIndex(): 初始化 activeTopic=null, filteredGroups=groupedByDate
  → renderSidebar(): 渲染 topic 下拉 + date tabs

用户选择 topic
  → selectTopic(key)
    → state.ui.activeTopic = key
    → state.index.filteredGroups = 按 key 过滤 groupedByDate
    → renderSidebar() + selectDate(filteredGroups[0].date)
```

### 命名合约（跨 Task 必须一致）

| 约束 | 值 |
|------|---|
| 新状态字段路径 | `state.ui.activeTopic` / `state.index.filteredGroups` |
| activeTopic 类型 | `string \| null`（null = 全部，字符串 = topic key） |
| topic key 格式 | `common_path.split('/')[0]`，如 `ai-software-dev` |
| CSS 类名 | `.topic-filter` / `.topic-select` / `.topic-count` |
| 新导出函数 | `selectTopic(key)` from `js/components/sidebar.js` |

---

## Task 列表

| Task | 文件 | 依赖 | 估计步骤数 |
|------|------|------|----------|
| Task 1 | `js/state.js` | 无 | 4 |
| Task 2 | `js/components/sidebar.js` | Task 1 | 5 |
| Task 3 | `js/main.js` | Task 1 | 4 |
| Task 4 | `app.css` | Task 2 | 3 |

---

## Task 1: js/state.js — 新增状态字段

**目标：** `state.ui.activeTopic` 和 `state.index.filteredGroups` 两个字段存在，供 Task 2 / 3 读写。

**为什么这样划分：** 状态结构改动完全独立，无 DOM 依赖，可最先执行并独立测试。

**模块职责：** 只做字段添加，不含任何逻辑；字段赋值由 Task 2 / 3 完成。

**Files:**
- Modify: `js/state.js`
- Test: `tests/state.test.js`（新建）

**验收标准：**
- 终端执行 `npx vitest run tests/state.test.js` → 2 个测试全部通过

**依赖：** 无
**被依赖：** Task 2, Task 3

- [ ] **Step 1: 写失败测试**

```javascript
// tests/state.test.js
import { state } from '../js/state.js'
import { test, expect } from 'vitest'

test('state.ui.activeTopic 初始值为 null', () => {
  expect(state.ui.activeTopic).toBe(null)
})

test('state.index.filteredGroups 初始值为空数组', () => {
  expect(Array.isArray(state.index.filteredGroups)).toBe(true)
  expect(state.index.filteredGroups).toHaveLength(0)
})
```

- [ ] **Step 2: 确认测试失败**

预期：`TypeError: Cannot read properties of undefined (reading 'activeTopic')`

- [ ] **Step 3: 修改 js/state.js**

`state.ui` 加入 `activeTopic: null`：

```javascript
  ui: {
    activeDate: null,
    archiveRoot: '',
    kbRoot: '',
    activeTopic: null,      // ← 新增
  },
```

`state.index` 加入 `filteredGroups: []`：

```javascript
  index: {
    data: null,
    groupedByDate: [],
    filteredGroups: [],     // ← 新增
    titleCache: new Map(),
    diffStatus: new Map(),
    annotations: {},
    titleFetchCache: new Map(),
    topicDescriptions: {},
    topicRepos: {}
  },
```

- [ ] **Step 4: 确认测试通过**

```bash
npx vitest run tests/state.test.js
```
预期：`共 2 个测试，通过 2，失败 0`

- [ ] **Step 5: 提交**

```bash
git add js/state.js tests/state.test.js
git commit -m "feat(state): add activeTopic and filteredGroups fields"
```

---

## Task 2: js/components/sidebar.js — topic 筛选逻辑

**目标：** 侧边栏顶部出现 topic 下拉选择器；选择 topic 后 date tabs 只显示该 topic 有文档的日期；`selectDate` 改用 `filteredGroups` 取数据。

**模块职责：**
- 新增内部函数 `_renderTopicFilter(aside)` — 渲染 `.topic-filter` 区域
- 修改 `renderSidebar()` — 注入 topic filter，date tabs 改用 `state.index.filteredGroups`
- 修改 `selectDate(date)` — `groupedByDate.find` → `filteredGroups.find`
- 新增导出函数 `selectTopic(key: string | null)`

**Files:**
- Modify: `js/components/sidebar.js`
- Test: `tests/sidebar.test.js`（新建，测试 selectTopic 的纯状态变更）

**验收标准：**
- 终端执行 `npx vitest run tests/sidebar.test.js` → 4 个测试全部通过
- 浏览器刷新：侧边栏顶部出现下拉，默认「全部 (N)」
- 选择某 topic → date tabs 只显示有该 topic 文档的日期，count 变化
- 选「全部」→ 恢复全量
- 选中 topic 后点击任意文档 → 文档正常打开；标记 done → 视觉变化，文档仍保留在列表中（GAP-1）

**依赖：** Task 1（`state.ui.activeTopic` / `state.index.filteredGroups` 必须存在）
**被依赖：** Task 4（CSS class）

- [ ] **Step 1: 写失败测试**

```javascript
// tests/sidebar.test.js
import { test, expect, vi, beforeEach } from 'vitest'
import { state } from '../js/state.js'

// 隔离 DOM 依赖：mock renderSidebar 和 selectDate
vi.mock('../js/components/sidebar.js', async (importOriginal) => {
  const original = await importOriginal()
  return { ...original, renderSidebar: vi.fn(), selectDate: vi.fn() }
})

import { selectTopic } from '../js/components/sidebar.js'

beforeEach(() => {
  state.index.groupedByDate = [
    { date: '20260514', entries: [
      { id: '1', entry: { common_path: 'ai-software-dev/sub/f1.md', created_at: '202605141000' } },
      { id: '2', entry: { common_path: 'learning-with-ai/sub/f2.md', created_at: '202605140900' } },
    ]},
    { date: '20260513', entries: [
      { id: '3', entry: { common_path: 'ai-software-dev/sub/f3.md', created_at: '202605131000' } },
    ]},
  ]
  state.index.filteredGroups = [...state.index.groupedByDate]
  state.ui.activeTopic = null
})

test('selectTopic 设置 activeTopic', () => {
  selectTopic('ai-software-dev')
  expect(state.ui.activeTopic).toBe('ai-software-dev')
})

test('selectTopic filteredGroups 只含目标 topic 的 entries', () => {
  selectTopic('ai-software-dev')
  const allEntries = state.index.filteredGroups.flatMap(g => g.entries)
  expect(allEntries.every(({ entry }) =>
    entry.common_path.split('/')[0] === 'ai-software-dev'
  )).toBe(true)
})

test('selectTopic 移除无目标 topic 文档的 date', () => {
  // '20260513' 只有 ai-software-dev；'20260514' 两个 topic 各一条
  selectTopic('learning-with-ai')
  // 只有 20260514 有 learning-with-ai 文档
  expect(state.index.filteredGroups).toHaveLength(1)
  expect(state.index.filteredGroups[0].date).toBe('20260514')
})

test('selectTopic(null) 恢复 filteredGroups 为全量', () => {
  selectTopic('ai-software-dev')
  selectTopic(null)
  expect(state.ui.activeTopic).toBe(null)
  expect(state.index.filteredGroups).toHaveLength(state.index.groupedByDate.length)
})
```

- [ ] **Step 2: 确认测试失败**

预期：`SyntaxError: The requested module '../js/components/sidebar.js' does not provide an export named 'selectTopic'`

- [ ] **Step 3: 实现代码**

**3a. 在文件末尾新增 `selectTopic`：**

```javascript
// ── selectTopic ─────────────────────────────────────────────────────────────

export function selectTopic(key) {
  state.ui.activeTopic = key;
  if (!key) {
    state.index.filteredGroups = state.index.groupedByDate;
  } else {
    state.index.filteredGroups = state.index.groupedByDate
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) =>
          entry.common_path?.split('/')[0] === key
        ),
      }))
      .filter(({ entries }) => entries.length > 0);
  }
  renderSidebar();
  if (state.index.filteredGroups.length > 0) {
    selectDate(state.index.filteredGroups[0].date);
  }
}
```

**3b. 在 `renderSidebar` 前新增 `_renderTopicFilter`：**

```javascript
// ── _renderTopicFilter ──────────────────────────────────────────────────────

function _renderTopicFilter(aside) {
  const allEntries = Object.values(state.index.data || {});
  if (allEntries.length === 0) return;
  const topicCounts = {};
  for (const entry of allEntries) {
    const topic = entry.common_path?.split('/')[0] || 'unknown'; // GAP-2: 异常条目计入 unknown
    topicCounts[topic] = (topicCounts[topic] || 0) + 1;
  }
  const total = allEntries.length;
  const topics = Object.keys(topicCounts).filter(t => t !== 'unknown').sort();

  const wrap = document.createElement('div');
  wrap.className = 'topic-filter';

  const sel = document.createElement('select');
  sel.className = 'topic-select';
  const optAll = document.createElement('option');
  optAll.value = '';
  optAll.textContent = `全部 (${total})`;
  sel.appendChild(optAll);
  const sep = document.createElement('option');
  sep.disabled = true;
  sep.textContent = '─────────';
  sel.appendChild(sep);
  for (const t of topics) {
    const opt = document.createElement('option');
    opt.value = t;
    opt.textContent = `${t} (${topicCounts[t]})`;
    sel.appendChild(opt);
  }
  if (topicCounts['unknown']) {  // GAP-2: 仅在有异常条目时显示 unknown 选项
    const optUnknown = document.createElement('option');
    optUnknown.value = 'unknown';
    optUnknown.textContent = `unknown (${topicCounts['unknown']})`;
    optUnknown.title = 'common_path 格式异常的条目';
    sel.appendChild(optUnknown);
  }
  sel.value = state.ui.activeTopic || '';
  sel.addEventListener('change', (e) => selectTopic(e.target.value || null));
  wrap.appendChild(sel);

  const countEl = document.createElement('div');
  countEl.className = 'topic-count';
  if (state.ui.activeTopic) {
    countEl.textContent = `${topicCounts[state.ui.activeTopic] || 0} / ${total} 篇`;
    countEl.style.display = '';
  } else {
    countEl.style.display = 'none';
  }
  wrap.appendChild(countEl);
  aside.appendChild(wrap);
}
```

**3c. 修改 `renderSidebar()` — 注入 filter，date tabs 用 `filteredGroups`：**

```javascript
export function renderSidebar() {
  const aside = document.getElementById('sidebar');
  aside.innerHTML = '';
  _renderTopicFilter(aside);                                    // ← 新增
  for (const { date, entries } of state.index.filteredGroups) { // ← 改这行
    // ... 其余不变
  }
}
```

**3d. 修改 `selectDate()` 中的 `groupedByDate.find` → `filteredGroups.find`：**

```javascript
  const group = state.index.filteredGroups.find(g => g.date === date); // ← 修改
```

- [ ] **Step 4: 确认测试通过**

```bash
npx vitest run tests/sidebar.test.js
```
预期：`共 4 个测试，通过 4，失败 0`

- [ ] **Step 5: 提交**

```bash
git add js/components/sidebar.js tests/sidebar.test.js
git commit -m "feat(sidebar): add topic filter selector and selectTopic"
```

---

## Task 3: js/main.js — loadIndex 初始化

**目标：** 页面加载（含 ⟳ / ⊙ 刷新）时 `activeTopic` 重置为 `null`，`filteredGroups` 与 `groupedByDate` 同步；`savedDate` fallback 改为从 `filteredGroups` 查找。

**模块职责：** 仅修改 `loadIndex()` 内 `buildGroups` 之后的 3 处语句，不改其他逻辑。

**Files:**
- Modify: `js/main.js`

**验收标准：**
1. 浏览器：选中某 topic（如 `ai-software-dev`）→ date tabs 已过滤
2. 点击 ⟳ 更新项目按钮 → 预期：topic 下拉恢复「全部」，date tabs 显示全量
3. 页面 F5 刷新 → topic 下拉默认「全部」

**依赖：** Task 1（`state.ui.activeTopic` / `state.index.filteredGroups` 字段）
**被依赖：** 无

- [ ] **Step 1: 在 `buildGroups` 调用后插入 2 行重置（js/main.js 第 27-28 行附近）**

```javascript
    state.index.groupedByDate = buildGroups(state.index.data);
    // ── 新增：重置 topic 状态 ──────────────────────────────
    state.ui.activeTopic = null;
    state.index.filteredGroups = state.index.groupedByDate;
    // ──────────────────────────────────────────────────────
    renderSidebar();
```

- [ ] **Step 2: 将 `savedDate` fallback 的 `groupedByDate` 改为 `filteredGroups`**

原：
```javascript
    const targetDate = (savedDate && state.index.groupedByDate.find(g => g.date === savedDate))
      ? savedDate
      : (state.index.groupedByDate.length > 0 ? state.index.groupedByDate[0].date : null);
```

改为：
```javascript
    const targetDate = (savedDate && state.index.filteredGroups.find(g => g.date === savedDate))
      ? savedDate
      : (state.index.filteredGroups.length > 0 ? state.index.filteredGroups[0].date : null);
```

- [ ] **Step 3: 浏览器验证**

操作序列：
1. 下拉选 `ai-software-dev` → date tabs 过滤
2. 点 ⟳ → topic 下拉恢复「全部」，date tabs 全量显示

- [ ] **Step 4: 提交**

```bash
git add js/main.js
git commit -m "feat(main): reset topic filter state on loadIndex"
```

---

## Task 4: app.css — topic filter 样式

**目标：** `.topic-filter`、`.topic-select`、`.topic-count` 渲染效果符合交互文档视觉规格。

**模块职责：** 纯样式，无逻辑。

**Files:**
- Modify: `app.css`

**验收标准：**
- 浏览器：topic filter 区域与 date tabs 之间有 `#d0d7de` 分隔线
- 下拉选择器宽度 100%，字号 12px
- 计数行字号 11px，颜色 `#57606a`

**依赖：** Task 2（DOM 输出 `.topic-filter` / `.topic-select` / `.topic-count`）
**被依赖：** 无

- [ ] **Step 1: 在 app.css Sidebar 区块末尾追加**

```css
/* ── Topic Filter ────────────────────────────────────────────────────────── */
.topic-filter {
  padding: 8px 10px 6px;
  border-bottom: 1px solid #d0d7de;
}

.topic-select {
  width: 100%;
  font-size: 12px;
  box-sizing: border-box;
  border: 1px solid #d0d7de;
  border-radius: 4px;
  padding: 3px 4px;
  background: #fff;
}

.topic-count {
  font-size: 11px;
  color: #57606a;
  margin-top: 4px;
  padding-left: 2px;
}
```

- [ ] **Step 2: 浏览器验证**

刷新页面，确认下拉选择器宽度填满侧边栏，与 date tabs 之间有灰色分隔线。

- [ ] **Step 3: 提交**

```bash
git add app.css
git commit -m "feat(css): add topic filter styles"
```

---

## 错误处理汇总

| 场景 | 处理方式 | 实现 Task |
|------|---------|---------|
| index.json 加载中或失败 | `_renderTopicFilter` 检查 `state.index.data` 为空时不渲染 | Task 2 |
| common_path 缺失/格式异常 | `entry.common_path?.split('/')[0]` 可选链，undefined 跳过计数 | Task 2 |
| 选中 topic 后 filteredGroups 为空 | `selectTopic` 内 `filteredGroups.length > 0` 判断，不调 `selectDate` | Task 2 |
| 切换 topic 后 savedDate 不在 filteredGroups | `loadIndex` 的 targetDate fallback 使用 `filteredGroups[0].date` | Task 3 |

---

## 审查记录

### TPQA（实现计划质量审查）

| 时间 | 发现 | 状态 |
|------|------|------|
| 2026-05-14 | TPQA-1：文件路径与仓库实际结构不符（state.js → js/state.js 等） | ✅ 已修复 v1.1 |
| 2026-05-14 | TPQA-2：接口签名与现有代码不一致（renderSidebar 参数错误、loadIndex 未导出） | ✅ 已修复 v1.1 |
| 2026-05-14 | TPQA-3：命名混用 state.groups / state.activeTopic，与实际结构不符 | ✅ 已修复 v1.1 |
| 2026-05-14 | TPQA-4：验收标准不可观测（「正确更新」「正确渲染」无具体命令或操作序列） | ✅ 已修复 v1.1 |
| 2026-05-14 | TPQA-5：测试导入路径、测试框架（vitest）与仓库实际不匹配 | ✅ 已修复 v1.1 |

### SPCA（Spec-Plan 交叉检测）

| 时间 | 发现 | 状态 |
|------|------|------|
| 2026-05-14 | GAP-1：验收标准 #8「不影响现有文档操作」无显式验证步骤 | ✅ 已修复 v1.2 |
| 2026-05-14 | GAP-2：common_path 异常条目未实现 unknown 分类和 tooltip | ✅ 已修复 v1.2 |

---

## 变更记录

| 版本 | 日期 | 变更内容 |
|------|------|---------|
| v1.0 | 2026-05-14 | 初稿（存在 TPQA 阻塞项） |
| v1.1 | 2026-05-14 | TPQA 修复：文件路径、接口合约、命名一致性、验收可观测性、测试完整性 |
| v1.2 | 2026-05-14 | SPCA 修复：GAP-1 补全验收步骤，GAP-2 实现 unknown 分类 + tooltip |