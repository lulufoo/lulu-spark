# Phase 0 — 提取 app.css（20min）

> **前置确认**：无前置要求，可直接开始。
> **回滚代价**：`rm app.css`，还原 index.html 中的 `<link>` 为 `<style>...</style>`。

---

## Step 0.1 — 提取 CSS 到 app.css

**执行内容**

1. 新建 `app.css`，将 `index.html` 中 `<style>` 标签内的全部内容整体移入（一字不改）
2. `index.html` 中将 `<style>...</style>` 替换为 `<link rel="stylesheet" href="app.css">`

**不做什么**

- 不修改任何 CSS 内容
- 不修改任何 JS 逻辑

> ⏸️ **停止信号**：Step 0.1 完成。请验证：启动 `server.py`，对比修改前后界面视觉完全一致，checklist 1-4 通过。确认通过后回复"继续"。

---

## 阶段完成 — 收尾

**Commit**

```
refactor(phase0): extract app.css
```
