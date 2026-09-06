# Knowledge Edit 模式左侧空白

**状态：** Implemented  
**关联待办：** `kb-edit-layout`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/knowledge/ui/viewer/shell.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/ui/viewer/shell.tsx` `ReaderShell`）：阅读列是 `.viewer-body`（内含评论条 + `.kb-reader-body`），编辑框 `.viewer-edit-area` 是同一 `.viewer-content-row` 的兄弟节点。

✅ Verified（`frontend/src/knowledge/commands/viewer/mount.ts`）：Edit 只把 `.kb-reader-body` 交给 `setDocEditMode` 隐藏。

✅ Verified（`frontend/app.css`）：`.viewer-body` 与 `.viewer-edit-area` 都是 `flex: 1; width: 100%`。外层 `.viewer-body` 仍占左半列，编辑文本被挤到右侧。

✅ Verified（`frontend/src/notes/page.tsx` `NotesReaderBody`）：Notes 在 Edit 时把整个 `.viewer-body` 设为 `display: none`。

目标：Knowledge Edit 与 Notes 一致，阅读列整列让出，编辑框铺满内容行。

## 方案

✅ Verified（`frontend/src/doc-editor/view.tsx` `setDocEditMode`）：当前只切换传入的 `bodyEl`。Knowledge 传入的是内层正文，不是 flex 阅读列。

在 `setDocEditMode` 中隐藏/显示 `bodyEl.closest('.viewer-body') || bodyEl`。Notes 的 `#md-body`、file-popup 的 `#file-popup-body.viewer-body` 都会落到同一阅读列，不改 mount 调用点。

## Plan

1. ✅ Verified（`frontend/src/doc-editor/view.tsx`）：Edit 切换最近的 `.viewer-body`。
2. ✅ Verified（本会话 vitest：`tests/doc-editor/view.test.js`、`tests/knowledge/knowledge-viewer-mount.test.js` 及相关 41 项通过）：进入 Edit 隐藏 `.viewer-body`，退出恢复；无 `closest` 的测试桩仍走 `bodyEl` 自身。
