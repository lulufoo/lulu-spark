# Knowledge Diff 弹框点击即关

**状态：** Implemented  
**关联待办：** `knowledge-diff-close-toast`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/knowledge/ui/diff-dialog.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/ui/diff-dialog.tsx` `handleKbDiffCommit`）：点 Commit 后弹框留着，等接口结束再关，约 1500ms。  
✅ Verified（`frontend/src/app-shell/commands/commit-dialog.ts` `doCommitChanges`）：Notes/顶栏是先关弹框，后台提交，结果走 toast。

目标：Knowledge Diff 点 Commit / Discard 后立刻关弹框，不挡住后面操作。组件名从 `KbDiffDialog` 改为 `KnowledgeDiffDialog`。

## 方案

Commit / Discard：先关弹框，再调接口，成功或失败用 `showToast`。树刷新仍发 `kb-diff-updated`。

公开符号：`KnowledgeDiffDialog` / `openKnowledgeDiffDialog` / `closeKnowledgeDiffDialog`。文件改为 `knowledge-diff-dialog.tsx`。

## Plan

1. 点击即关 + toast。
2. 重命名并改调用点、测试。  
   ✅ Verified（本会话 vitest）：`tests/knowledge` 139、`header-sync` + `copy-switch-t6` + `settings-knowledge-ui` 通过。点 Commit / Discard 后弹框立刻关掉，结果走 toast。
