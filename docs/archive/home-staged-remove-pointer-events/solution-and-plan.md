# Stage 移除确认框无法点击

**状态：** Implemented  
**关联会话：** `workbench_chat_9856b0ba1248d944beb442a81eba86d3`

## 问题与目标

✅ Verified（`frontend/app.css` `.home-chat-composer`）：composer 为 `pointer-events: none`，只放行 dock / input / send / `.home-chat-staged`。

✅ Verified（`frontend/src/home/page.tsx`、`ui/staged-list.tsx`）：确认框 `.home-chat-delete-confirm` 画在 composer form 内，不在放行名单里。

目标：× 弹出的 Remove / Cancel / 遮罩可点。不改 unstage 命令、不改围栏。

## 方案

把 `.home-chat-delete-confirm` 加入同一组 `pointer-events: auto`。该 class 也用于侧栏删会话，设为 auto 与默认一致。

## Plan

1. ✅ Verified（`app.css`）：`.home-chat-delete-confirm` 与 dock / Stage 同一组 `pointer-events: auto`。
2. ✅ Verified（本会话 vitest `tests/home-entry-shell/hub.test.js`）：28 通过。
