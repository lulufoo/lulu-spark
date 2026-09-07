# Knowledge 阅读器删除 Pending commit

**状态：** Implemented  
**关联待办：** `kb-reader-remove-pending`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/knowledge/ui/viewer/shell.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/ui/viewer/shell.tsx`）：阅读器顶栏右上角有 `● Pending commit`（`#kb-btn-pending`）。  
✅ Verified（`frontend/src/app-shell/commands/header-sync.ts`）：Knowledge 页顶栏 Sync → `↑ Commit changes` 打开 `KbDiffDialog`，提交同一仓库。

目标：去掉阅读器右上角这颗按钮及其专用接线。提交改走顶栏 Sync。

## 方案

删除按钮、badge 显示/隐藏、以及只为这颗按钮服务的 `kb:dirty` 监听。不删顶栏 Commit，不删 `KbCommitDialog`。

## Plan

1. 从 `ReaderShell` / fallback HTML 去掉按钮。
2. 去掉 mount / modal / chrome 里的 badge 接线。
3. 测试断言阅读器顶栏不再出现 Pending commit。  
   ✅ Verified（本会话 vitest）：`tests/knowledge` 138、`tests/shared` 58、`header-sync` + `knowledge-scope` 9，均通过。
