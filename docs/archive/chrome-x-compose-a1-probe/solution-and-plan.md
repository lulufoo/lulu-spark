# X 输入框 A1 探测扩展

Chrome 扩展内容脚本：<https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts>  
Chrome Translator API：<https://developer.chrome.com/docs/ai/translator-api>

**状态：** v0.3.0 一次出 A1/A2/A4 全量；假设仍未释放  
**关联：** `feature-20260904084826-d62a1e8e`

## 问题与目标

一次加载、一次点击，覆盖到能关假设的证据，不再拆成「先问关不关」。

- A1：对准最内层 `contenteditable`，六种写入，真「发帖」键被点亮
- A2：`availability` + `create`（含下载）+ 翻「这是一句探测」
- A4：挂到 compose 工具条旁；停掉回注后看 idle / 写入后按钮还在；必要时点一条回复把回复框打开

## 方案

独立包 `extensions/chrome-x-compose-probe/`。✅ Verified（`manifest.json` version 0.3.0）

- 右下角「完整探测」一条路径跑完。✅ Verified（`content.js` `runMatrix`）
- 写入目标：`tweetTextarea_*` 内最深可见 `contenteditable`。✅ Verified（`probe-dom.js` `findWriteTarget`）
- 策略：`paste` / `insertText` / `inputEvent` / `insertFromPaste` / `clipboardPaste` / `keyboard`。✅ Verified（`probe-write.js` `STRATEGIES`）
- 真发送键：`tweetButton*` 或「发帖」，排除「可以回复」。✅ Verified（`probe-dom.js` `isPostControl`）
- Translator 在 MAIN world：`create` + `translate`，进度写 `data-a1-translator`，最多等 90 秒。✅ Verified（`probe-translator.js`，`content.js` `waitTranslator`）
- A4：注入后暂停 Observer 1 秒记 `persistIdle`，写完再记 `persistWrite`。✅ Verified（`content.js` `probeOne`）
- 无回复框时点第一条可见 `[data-testid=reply]`。✅ Verified（`probe-run.js` `openReplyIfNeeded`）

不发推，不调 Workbench。

## 验收

扩展页加载本目录，确认版本 **0.3.0**。打开已登录的 x.com Home。只点「完整探测」一次；语言包下载时按钮会停住，等面板出现 `A2: ... status=done` 再复制。

关假设只看这一块板：

- A2：`status=done` 且 `create=true` 且 `out=` 是英文
- A4：发帖和回复都有 `persistIdle=Y`（`persistWrite` 一并看）
- A1：该表面至少一行 `read=Y` 且 `en=Y` 且 `post=tweetButton*` 或「发帖」
