# Workbench commit 命名收口

**状态：** Implemented  
**关联待办：** `wb-commit-rename`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/app-shell/ui/workbench-commit-dialog.tsx>

## 问题与目标

✅ Verified（`frontend/src/app-shell/ui/workbench-commit-dialog.tsx`）：整仓提交对话框现为 `WorkbenchCommitDialog`，标题是 `↑ Workbench commit`。  
✅ Verified（`frontend/src/app-shell/commands/header-sync.ts`）：顶栏同一颗 `↑ Commit changes` 在 Knowledge 页打开知识库 diff，不在 Knowledge 页才打开这套对话框。

目标：这套整仓提交的代码名、文件名、对话框标题收到 `Workbench commit`。顶栏按钮文案不动，避免 Knowledge 页说错。

## 方案

`CommitChanges*` → `WorkbenchCommit*`。文件改为 `workbench-commit-dialog` / `workbench-commit`。对话框标题改为 `↑ Workbench commit`。顶栏仍写 `↑ Commit changes`。

## Plan

1. 改 state / commands / ui 符号和文件。
2. 改 DOM id、CSS、测试、gate 路径。
