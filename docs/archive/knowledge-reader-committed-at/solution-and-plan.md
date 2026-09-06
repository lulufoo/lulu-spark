# Knowledge 阅读器顶栏显示最后 commit 时间

**状态：** Implemented  
**关联待办：** `kb-reader-committed-at`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/knowledge/ui/viewer/shell.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/ui/viewer/shell.tsx`）：阅读器顶栏左边只有 `.kb-file-size`。  
✅ Verified（`src-tauri/src/services/knowledge/read.rs` `kb_read_json`）：只回 `content`。

目标：打开一篇 Knowledge 文档后，在文件大小旁边显示该文件最后一次 git commit 的本地时间。没有 commit 历史则只显示大小。

## 方案

`kb_read` 增加 `committed_at`（unix 秒，或 `null`）。Host 在知识库 clone 目录对该相对路径跑 `git log -1 --format=%ct`。前端格式化为 `YYYY-MM-DD HH:mm`，画在 `.kb-file-committed`。

## Plan

1. git 取文件最后一次 commit 时间；`kb_read` 带上字段。
2. 阅读器顶栏大小旁绘制。
3. Rust / JS 测试。
