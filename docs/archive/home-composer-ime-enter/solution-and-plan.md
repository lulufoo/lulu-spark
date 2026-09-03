# Home Composer IME Enter — Solution and Plan

## Solution

- ✅ Verified（`frontend/src/home/page.tsx`）：回车发送只拦 `nativeEvent.isComposing`，随后 `requestSubmit()`。
- ✅ Verified（W3C UI Events）：组合期间的 `keydown` 的 `isComposing` 应为 `true`；IME 处理中的 `keydown` 返回 `keyCode` 229。
- ✅ Verified（WebKit / Safari 事件顺序）：确认候选时先 `compositionend` 再 `keydown Enter`，此时 `isComposing` 已是 `false`。
- ⚠️ Inferred：自管组合态，并在 `compositionend` 后下一拍再清旗，可挡住 WKWebView 上那次确认回车。

## Plan

1. ✅ Verified：抽出 `createImeEnterGuard`（`isComposing` / 229 / 下一拍清旗）。
2. ✅ Verified：首页 textarea 接上 `compositionstart` / `compositionend`，回车发送前走 `isBlocked`。
3. ✅ Verified：补 helper 单测，以及「先 compositionend 再 Enter → 不发送；下一拍再 Enter → 发送」。
