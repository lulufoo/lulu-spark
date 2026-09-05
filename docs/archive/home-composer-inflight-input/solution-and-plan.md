# Home Composer 请求执行中可输入不可发送

**状态：** Implemented  
**关联待办：** `task_305b75cedaa3`

## 问题与目标

✅ Verified（`frontend/src/home/state/store.ts` `composerLocked`）：未 Binding，或当前会话在 `inFlightIds` 中，整段 composer 视为锁定。

✅ Verified（`frontend/src/home/page.tsx`）：textarea 与 Send 都 `disabled={locked}`。Enter 发送也走同一把锁。

目标：当前会话请求执行中，可以继续在输入框打下一句；不能发送（按钮、Enter、submit 均不发出第二轮）。未 Binding 时输入和发送仍都禁用。

## 方案

在 state 拆两把锁：

- ✅ Verified（现有 `composerLocked`）：发送锁 = 未 Binding **或** 当前会话 in-flight。Send、Enter、submit、Stage 移除继续用这把锁。
- 输入锁 = 仅未 Binding。请求执行中不再 `disabled` textarea。

✅ Verified（`page.tsx` `onSubmit`）：submit 已有 `inFlightIds` 早退；输入解锁后仍靠这道闸挡住发送。

不改 `sendMessage`、不改 Android。

## Plan

1. ✅ Verified（`frontend/src/home/state/store.ts`）：增加 `composerInputLocked`（仅 `!hostBound`）；textarea 改用它。
2. ✅ Verified（本会话 vitest）：`tests/home/composer-lock.test.js`、`tests/home-entry-shell/hub.test.js`、`tests/gates/ai-assistant-composer.test.js`、`tests/home/composer-ime.test.js` 共 34 通过。执行中 input 可写、Send 禁用、二次 submit 不发。
