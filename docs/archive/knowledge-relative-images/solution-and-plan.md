# Knowledge 相对图片展示

**状态：** Implemented  
**关联待办：** `kb-relative-images`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/notes/ui/viewer/body.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/ui/viewer/highlight.ts` `renderKbMdBody`）：知识库只把 md 转成 HTML，不改写相对 `img src`。

✅ Verified（`src-tauri/src/services/knowledge/read.rs` `kb_read_json`）：`kb_read` 只回正文。

✅ Verified（`frontend/src/notes/ui/viewer/body.tsx`）：笔记用 `get_notes_asset` 把相对图换成 blob URL。

目标：知识库预览对相对图走同一条读取。`![](foo.png)` / `![](./foo.png)` 能显示。外链、`data:`、`blob:` 不走这条。

## 方案

Host 新增只读接口 `get_kb_asset(repo, base, href)`：

- `base` 是当前 md 路径，`href` 是相对图路径。
- 拼到 md 所在目录后，走现有 `kb_safe_path`。
- MIME 白名单与笔记相同：png / jpg / jpeg / gif / webp。✅ Verified（`src-tauri/src/services/workbench_read/notes.rs` `mime_from_extension`）
- `..`、绝对路径、http(s) 拒绝。

前端：渲染后扫 `<img src>`，相对路径换成 blob URL。放在 commands，不放进 ui paint。

不改笔记，不改 MCP 读文档。

## Plan

1. `kb_asset_json` + `get_kb_asset` + read-api ACL + `/api/kb/asset` invoke map。
2. `fetchKbAssetAsBlobUrl`；预览 hydrate；卸载时 revoke。
3. Rust 服务测试 + invoke 契约测试 + 前端 hydrate 测试。
