# Notes 阅读器删除 Pending commit

**状态：** Implemented  
**关联待办：** `notes-remove-pending-btn`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/notes/page.tsx>

## 问题与目标

✅ Verified（`frontend/src/notes/page.tsx`）：阅读器有 `● Pending commit`（`#btn-panel-commit`），打开 `MdCommitDialog`。  
✅ Verified（`frontend/src/notes/commands/viewer/commit.ts`）：该对话框打的是 Workbench `/api/status` + `/api/commit`，与顶栏同一仓库。

目标：去掉阅读器这颗按钮及专用对话框。提交只走顶栏 Sync → `↑ Commit changes`。卡片上的脏点保留。

## 方案

删除按钮、`MdCommitDialog`、badge 接线。不改顶栏 `CommitChangesDialog`。

## Plan

1. 阅读器去掉按钮和空 commit bar。
2. 撤掉 dialog / badge / `pendingCommit` 接线。
3. 测试不再断言这颗按钮。  
   ✅ Verified（本会话 vitest）：`tests/notes` 119、`header-sync` + `commit-dialog` 12，均通过。
