# Knowledge 树右键添加 / 删除

**状态：** Implemented  
**关联待办：** `kb-tree-entry-host`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/knowledge/ui/sidebar.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/ui/sidebar.tsx`）：左侧树只有点开和行内重命名，没有右键增删。  
✅ Verified（`src-tauri/src/services/knowledge/write.rs`）：`kb_save` 只写已有文件，不能新建。  
✅ Verified（`src-tauri/src/services/knowledge/rename.rs`）：已有同目录改名，并会带走注释 sidecar。

目标：左侧目录右键可以添加、删除。添加只有文件 / 目录；文件只能是 `.md`。

## 方案

- 右键目录：Add file / Add folder 落到该目录的子项。  
- 右键文件：Add 落到该文件的同级。  
- 右键空白树：Add 落到仓库根。  
- 名字用现有行内输入。文件名没有 `.md` 就补上，其它后缀拒绝。  
- 删除要输入 `CONFIRM` 才能确认，同时删注释 sidecar。  
- 只改本地磁盘，不自动 commit。

## Plan

1. Host 增加 `kb_create` / `kb_delete`，接 invoke map 和 write-api ACL。  
2. 树菜单 + 行内新建 + 删除确认，接到现有树会话。  
3. Rust / JS 单测锁父路径规则、`.md` 限制、遍历拒绝。
