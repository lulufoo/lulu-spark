# Home Chat 复制 Session ID

**状态：** Implemented  
**关联待办：** `task_162b0ac4a0c8`

## 问题与目标

✅ Verified（`frontend/src/home/page.tsx`）：`.home-chat-main` 只有消息区和 composer，没有右侧溢出菜单。

✅ Verified（`frontend/src/home/state/store.ts`）：当前会话 id 已在 `currentSessionId`。

目标：Chat 主栏右上角加 Cursor 式 `⋯`，菜单内提供 Copy Session ID，把当前 `currentSessionId` 写入剪贴板，方便排查。

## 方案

- UI：`home/ui/session-menu.tsx` 在 `.home-chat-main` 右上角画 `⋯`。无 `currentSessionId` 时不画。菜单先只放 Copy Session ID。删除仍留在左侧会话行。
- 命令：`home/commands/copy-session-id.ts` 读 `getHomeState().currentSessionId`，调用 `navigator.clipboard.writeText`。
- Store：不改。
- 反馈：成功后菜单项短暂变为 `Copied`，约 1.2 秒后关闭。对齐 FilePopup（✅ Verified：`frontend/src/file-popup/ui/popup.tsx`）。

不改 Android。

## Plan

1. ✅ Verified（本会话）：加 `copyCurrentSessionId` 与主栏 `⋯` 菜单。
2. ✅ Verified（本会话 vitest）：`tests/home/copy-session-id.test.js`、`tests/home-entry-shell/hub.test.js`。
3. ✅ Verified（本会话 vitest）：`tests/home` + `chat-delete` 共 144 通过。
