# Knowledge 左侧目录视觉分层 技术方案与实施计划

源码：[`frontend/src/knowledge/ui/sidebar.tsx`](../../../frontend/src/knowledge/ui/sidebar.tsx)、[`frontend/app.css`](../../../frontend/app.css)、[`frontend/src/shared/floating-list-select.ts`](../../../frontend/src/shared/floating-list-select.ts)

**状态：** Implemented  
**关联待办：** `task_5f9533dcb5e5`

## 问题与目标

✅ Verified（改前 `sidebar.tsx` 行节点只有 `node.name` 文本按钮）：目录和文件同一套文字行，没有类型列。

✅ Verified（改前 `app.css` `.knowledge-doc-tree-label`）：字号 13px，目录与文件同色同重，长名换行。

目标：目录和文件一眼能分开；文件和文件不再糊成一块。后续约束：目录与文件字体一致并放大；仓库下拉与目录同字号；行距一档，展开子树末行与下一个同级目录不再多留空。

## 方案

类型靠图标，不靠字重。

- ✅ Verified（`sidebar.tsx` `TreeNodeBlock`）：每行 = 箭头位（目录有 ▸，文件留空）+ 文件夹/文档图标 + 名称。点击仍落在 `.knowledge-doc-tree-label`。
- ✅ Verified（`sidebar.tsx` `TreeNodeName`）：文件名按最后一个 `.` 拆后缀；目录不拆。
- ✅ Verified（`app.css` `.knowledge-doc-tree-name` / `.knowledge-doc-tree-ext`）：目录和文件都是 14px、字重 300、颜色 `#24292f`。长名单行省略。
- ✅ Verified（`app.css` `.knowledge-doc-tree-node + …` 与 `.knowledge-doc-tree-children`）：相邻行 `margin-top: 2px`。子树引导线用 `--knowledge-tree-guide`。
- ✅ Verified（`floating-list-select.ts` 菜单 `className`）：挂载 `${pickerClass}-menu`。✅ Verified（`app.css` `.knowledge-repo-picker` / `.knowledge-repo-picker-menu`）：触发条和选项 14px、字重 300。Notes 的 `topic-select` / `tag-select` 仍走全局 13px。

不改展开/打开文件的命令逻辑。不改 Notes 筛选下拉。

## 计划

1. 树行加上类型列，CSS 分层。
2. 目录与文件字体对齐到 14px。
3. Knowledge 仓库下拉跟目录字号；菜单带 `knowledge-repo-picker-menu`。
4. 去掉子树结束后的额外间距，行距统一 2px。
5. 单测：`tests/knowledge/knowledge-doc-list.test.js` 类型列；`tests/shared/floating-list-select.test.js` 菜单 class。

## 验收

- 目录有箭头和蓝色文件夹；文件同一列留空并显示灰色文档图标。
- 目录名、文件名、仓库下拉触发条与选项都是 14px、字重 300。
- 展开目录 A 后，最后一项与同级目录 B 的间距等于其他相邻行。
