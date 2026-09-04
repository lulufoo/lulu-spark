# X 发帖中译英体验版

Chrome 扩展内容脚本：<https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts>  
Chrome Translator API：<https://developer.chrome.com/docs/ai/translator-api>

**状态：** v1.1.0 覆盖 Home 发帖 + 评论弹层  
**关联：** `feature-20260904084826-d62a1e8e`

## 问题与目标

Home 发帖已通。点击评论出现弹层时，必须也能看见「翻译」，点了就地换成英文。

## 方案

包仍是 `extensions/chrome-x-zh-en/`。✅ Verified（`manifest.json` version 1.1.0）

- 表面：`tweet-compose` 与 `reply-dialog`。✅ Verified（`compose-dom.js` `SURFACES`）
- 弹层优先挂 `dialog` 内 `toolBar`，没有就挂编辑器旁，保证人能看见按钮。✅ Verified（`findMount`）
- 发送键只在弹层里找，避免误认 Home 的「发帖」。✅ Verified（`composeRoot`）
- 写入先对最内层 `contenteditable`，再对 textarea 本身；`insertText` → `keyboard` → `inputEvent` → `paste`，谁点亮发送键用谁。✅ Verified（`compose-write.js` `replaceText`）

不调 Workbench，不代发。

## 验收

扩展页确认 **1.1.0**。点一条推的评论，弹层里打中文，点「翻译」，复制右下角状态。

- 必须先看见蓝色「翻译」
- `表面：reply-dialog`
- `A2 翻译：成功`、`A1 写入：…`、`A3 发帖键：已亮 …`
