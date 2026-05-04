# Phase 5 — 新增 Feed 模块（1-2h，纯新增）

> **前置确认**：Phase 4 已完成（checklist 1-42 全部通过，console 无报错）。
> **风险**：零。此阶段为纯新增，不修改任何现有代码。
> **回滚代价**：`rm js/feed.js`，移除 index.html 中新增的按钮，移除 main.js 中的 Tab 切换逻辑。

---

## Step 5.1 — index.html header 增加按钮

**执行内容**

在 `index.html` header 区域增加按钮（CSS 样式同现有 header 按钮）：

```html
<button id="btn-feed">📡 Builders</button>
```

> ⏸️ **停止信号**：Step 5.1 完成。请确认按钮出现在 header 区域，样式与其他按钮一致。确认后回复"继续"。

---

## Step 5.2 — 新建 js/feed.js

**执行内容**

新建 `js/feed.js`，实现以下功能：

- 并行 fetch 三个 Raw URL（带 `_=Date.now()` 防缓存）：
  - `https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-x.json`
  - `https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-podcasts.json`
  - `https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-blogs.json`
- 显示 `generatedAt` 更新时间
- **X section**：按人分组，每人展示最新推文（text + 时间 + likes/retweets + 原文链接）
- **Podcasts section**：标题 + 发布时间 + 原文链接；transcript 默认折叠（显示前300字，点击展开）
- **Blogs section**：空状态友好提示（当前 `blogs: []` 为空）

> ⏸️ **停止信号**：Step 5.2 完成。请确认 `js/feed.js` 已创建，包含 fetch 逻辑和渲染函数。确认后回复"继续"。

---

## Step 5.3 — main.js 挂载 Tab 切换逻辑

**执行内容**

在 `js/main.js` 中添加：

- 点击 `#btn-feed` → main 区域切换到 feed 视图，隐藏原有 archive 视图
- 点击 archive 相关区域 → 切回 archive 视图

> ⏸️ **停止信号**：Step 5.3 完成。请验证：checklist 1-42 全部仍然通过（现有功能零影响），checklist 43-49（Feed 功能全部通过）。确认通过后回复"继续"。

---

## 阶段完成 — 收尾

**Commit**

```
feat(phase5): add follow-builders feed tab
```
