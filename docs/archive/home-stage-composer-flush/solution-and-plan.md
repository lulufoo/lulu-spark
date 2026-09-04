# Stage 贴紧对话输入框技术方案与实施计划

**状态：** Implemented  
**关联待办：** `task_61753c1bbc34`

## 问题与目标

✅ Verified（`frontend/app.css` 改前）：`.home-chat-staged` 为 `margin: 0 0 8px`，Stage 与 `.home-chat-composer-dock` 之间空出 8px。两者各自圆角描边，看起来中间一道 gap。

目标：有 Stage 时输入框贴在 Stage 底边；无 Stage 时输入框外观不变。

## 方案

✅ Verified（落地后 `frontend/app.css`）：去掉 Stage 底边距；有 Stage 时上块去底边、下块去顶圆角，中间只留输入框顶边当分割线。无 Stage 时 `+` 选择器不生效，dock 仍是独立 12px 圆角盒。

不改 `staged-list.tsx` 结构、不改 Stage 读写围栏。

## 验收

1. 有 Stage：Stage 与输入框之间无 8px 空隙。
2. 无 Stage：输入框仍是单独圆角盒。
