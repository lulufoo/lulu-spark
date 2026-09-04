# FilePopup 复制绝对路径 技术方案与实施计划

**状态：** Implemented  
**关联待办：** `task_c2734ec35293`

## 问题与目标

✅ Verified（`frontend/src/file-popup/ui/popup.tsx`）：顶栏只有 Edit / Save / Cancel / Close，标题是 `view.title`，用户看不到也复制不了绝对路径。

✅ Verified（`frontend/src/file-popup/commands/popup.ts`）：`openFilePopup` 已把调用方传入的 `path` trim 后写入 `view.path`。Stage（`frontend/src/home/commands/staged.ts`）和 Todo 附件（`frontend/src/todo-task/commands/attachments.ts`）都走这条口。

目标：在 FilePopup 顶栏加 Copy，把当前文档绝对路径写入剪贴板。

## 方案

- UI：`file-popup/ui/popup.tsx` 的 `.file-popup-actions` 增加 Copy，放在 Edit/Cancel/Save 旁、Close 前。`view.path` 为空时禁用。
- 命令：`file-popup/commands/popup.ts` 新增 `copyFilePopupPath()`，读 `view.path`，调用 `navigator.clipboard.writeText`。
- Store：不改。路径已在 `FilePopupView.path`。
- 反馈：成功后按钮短暂变为 `✓`，约 1.2 秒后回到 `Copy`。对齐 Notes / Knowledge 现成模式（✅ Verified：`frontend/src/notes/page.tsx`、`frontend/src/knowledge/commands/viewer/mount.ts`）。
- 加载失败与编辑态仍可复制：路径在打开时已写入。

不改 Stage 列表，不把路径展示进标题。

## 计划

1. 加 `copyFilePopupPath` 与顶栏 Copy。
2. 补 `tests/file-popup/popup.test.js`（按钮存在）与 `tests/file-popup/open.test.js`（写入 `view.path`）。
3. 跑 file-popup 单测。

## 验收

- 打开 FilePopup 时顶栏有 Copy。
- 点击后剪贴板内容等于 `view.path`。
- 无路径时 Copy 禁用。
