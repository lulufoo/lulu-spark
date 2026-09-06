# Knowledge 树选中回车改名

**状态：** Implemented  
**关联待办：** `kb-tree-rename`

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/knowledge/ui/sidebar.tsx>

## 问题与目标

✅ Verified（`frontend/src/knowledge/page.tsx` `onSidebarClick`）：点目录只展开，点文件只打开。树节点没有改名入口。

目标：已选中的目录或文件，回车进入行内改名（Esc 取消，再回车确认）。点击仍展开/打开。只改当前目录下的名字，不移动到别的目录。

## 方案

Host 新增写接口：`from` 用现有 `kb_safe_path`，新名只允许单个路径分量。目标已存在则拒绝。文件改名时，若有 `.knowledge_annotations` 旁路文件一并改；目录改名时移动对应注释子目录。

✅ Verified（`src-tauri/src/repositories/knowledge.rs` `kb_annotation_path`）：`docs/a.md` 的注释在 `.knowledge_annotations/docs/a.json`。

前端：树 store 增加 `renamingPath`；回车出输入框，确认后改树路径、打开中的文档 hash、viewer-state。不自动 commit。

## Plan

1. `kb_rename` + ACL + invoke map。
2. 树行内输入；page 拦截 Enter / Escape。
3. 服务层与 Knowledge 挂载测试。
