# Drawer 刷新先进入模式再渲染

**状态：** Implementing  
**关联待办：** `drawer-mode-boot`

✅ Verified（`drawer.html` `applyDiagramTheme` 在 L3614 + `renderBoard` 空源码分支）：模块一加载就 `applyDiagramTheme()`，此时 `data-drawer-mode` 已是 `board` 但 `boardSource` 仍空，空渲染会 `clearDrawerBoot()`。真正 `BoardRender.render` 把标题先插进画布时遮罩已没了，于是闪「AI Agent + drawio-skill」。

✅ Verified（同文件结构）：`#btnShowSheet` 在 `.top-float` 里，是 `#preview-stage` 的兄弟。把整条顶栏跟画布一起藏，揭开时整栏一起闪；顶栏应一直在，只藏画布。

源码：`/Users/lulu/Code/lulu-dev-skills/lulu-draw-skills/drawer/assets/drawer.html`

## 问题与目标

✅ Verified（`drawer.html` 启动段）：`restoreCachedSvg()` → `await bootstrap()`（内含 `renderDiagram()`）→ `await bootstrapBoard()` → 最后才 `restoreDrawerMode()` 读 `?mode=`。

✅ Verified（同文件 `<html>`）：首屏没有 `data-drawer-mode`；Mermaid 页签默认选中。

目标：渲染之前先进入模式。`?mode=board` 刷新不再先闪 Mermaid 图。

## 方案

1. `<head>` 同步脚本：URL `mode` > `drawer.ui.mode` > `mermaid`，立刻写 `data-drawer-mode`。
2. body 里页签一出来就按该属性同步页签/面板，避免壳层先露出 Mermaid。
3. Board 模式下：`restoreCachedSvg` / `renderDiagram` / `showEmpty` 不画 Mermaid；启动时先 `bootstrapBoard` + `setMode`，Mermaid 源后台再拉。

## Plan

1. 写入 head peek 与页签同步脚本。
2. 给 `renderDiagram` / `restoreCachedSvg` / `showEmpty` 加 Board 早退。
3. 重排启动：Board 不阻塞在 `diagram.mmd` 上。
4. 刷新 `?mode=board` 看画布是否还先出现 Mermaid SVG。
