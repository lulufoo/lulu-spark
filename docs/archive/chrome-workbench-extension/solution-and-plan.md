# Workbench 扩展壳（待读 + X 中译英）

Chrome 扩展清单：<https://developer.chrome.com/docs/extensions/reference/manifest>  
Chrome Translator API：<https://developer.chrome.com/docs/ai/translator-api>

**状态：** Implemented  
**关联待办：** `task_884172b0d212`  
**关联决策：** `feature-20260904084826-d62a1e8e`

## 问题与目标

X Home 发帖和评论弹层没有就地中译英；待读已是独立扩展。要交付一个未打包壳，两个模块按目录隔离。✅ Verified（决策 D / X，本会话已关 DC）

## 方案

就地升级为 `extensions/chrome-workbench-extension/`。✅ Verified（`manifest.json` name=`Workbench` version=`1.2.0`）

- `read-later/`：background + `lib/*`，仍只 POST Gateway `/read-later`。✅ Verified（`read-later/lib/readLaterApi.js`）
- `x-zh-en/`：迁 v1.1.0 的 DOM / 写入 / MAIN Translator；控件为线框图标，`aria-label` 无可见「翻译」字。✅ Verified（`x-zh-en/content.js` `createIcon`；`compose-dom.js` `SURFACES`）
- 清单合并：`activeTab` + `https://localhost:7654/*` + x.com/twitter.com content_scripts。✅ Verified（`manifest.json`）
- 体验包与探测包已删。✅ Verified（`extensions/` 现仅余 `chrome-workbench-extension`）

不做：Mac Accessibility、收费翻译、拦截发送、卡片行内回复、`x-zh-en` 调 Host。

## 验收

- 单测：`tests/extensions/chrome-workbench-extension.test.js` 13 通过；`chrome_and_l0_inbound_still_only_url_and_title` 通过。✅ Verified（本会话 vitest / cargo）
- Chrome 只装一个未打包 `chrome-workbench-extension`，关掉旧待读 / 体验版 / 探测，避免双注入。❌ Unresolved（需本机加载后点 Home 发帖与评论弹层）
