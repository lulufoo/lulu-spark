# Knowledge 阅读器顶栏收起目录

**状态：** Implemented  
**关联待办：** `kb-tree-collapse`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/knowledge/ui/viewer/shell.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/ui/viewer/shell.tsx`）：阅读器顶栏左边是 `.kb-reader-header-meta`（最后 commit 时间 + 文件大小），没有收起左侧目录的入口。  
✅ Verified（`frontend/src/knowledge/ui/sidebar.tsx` `KnowledgeDocLayout`）：左侧目录是 `.knowledge-doc-sidebar`，与阅读器并排。  
✅ Verified（`frontend/src/knowledge/ui/knowledge-search.tsx`）：相关知识面板用 `‹` / `›` 收起，不是这篇文档的目录树。

目标：在 `2026-09-06 23:55 3.5 KB` 这类 meta 左侧加展开 / 收起图标，用来把左侧目录收起来，阅读区占满。

## 方案

阅读器顶栏 meta 左侧放按钮 `.kb-btn-tree-toggle`。点击给 `.knowledge-doc-layout` 加 / 去掉 `is-tree-collapsed`。收起后侧栏宽度为 0，拖拽条隐藏。收起状态写入 `localStorage`，与侧栏宽度一样下次打开仍有效。  
✅ Verified（`frontend/src/knowledge/ui/viewer/shell.tsx`）：按钮里是侧栏面板 SVG，不是 `‹` / `›`。收起时按钮加 `is-collapsed`，图标水平翻转。  
⚠️ Inferred：顶栏按钮共用 `.md-header-btn:hover` 的边框，侧栏一收按钮左移，悬停边框会闪一下再消失；这个按钮去掉 hover 边框和阴影。

## Plan

1. state 记住收起；command 切换 class 并画按钮。
2. 顶栏 meta 左侧画按钮；CSS 收起侧栏。
3. 页面铺壳后恢复；阅读器挂载时绑定点击。
4. JS 测试：按钮在 meta 左侧；切换 class 并持久化。
