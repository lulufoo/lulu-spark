# Knowledge 菜单与选中 md 记忆

**状态：** Implemented  
**关联待办：** `kb-viewer-state`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/knowledge/page.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/page.tsx`）：`#/knowledge` 无仓库时写死 `repos[0]`。点 md 只改 hash `?path=`，离开后再进会丢。

目标：记住当前仓库和当前选中的 md。再进 Knowledge 时回到该仓库，并展开祖先目录打开那篇 md。不记文件夹展开集合。

## 方案

落点与 Hidden files 同类：Host `cache_dir` 下一份覆盖写的 JSON。

✅ Verified（`src-tauri/src/config/paths.rs` `knowledge_hide_patterns_path`）：现有缓存文件是 `{cache_dir}/knowledge-hide-patterns.json`。

本功能文件：`{cache_dir}/knowledge-viewer-state.json`，字段 `{ repo, path }`。`path` 只接受 `.md`。文件缺失或损坏当空记录，不覆盖、不报错。

URL 已有仓库或 `?path=` 时以 URL 为准。换仓库先写 `{ repo, path: "" }` 再跳转。

## Plan

1. Host：path helper、读写服务、`get_kb_viewer_state` / `set_kb_viewer_state`、ACL 与 invoke map。
2. Knowledge 页：无 URL 时按缓存落地；点 md 写缓存；点目录不写 path。
3. 测试：服务读写、无仓库落地、同仓库恢复 md、换仓库清空 path。
