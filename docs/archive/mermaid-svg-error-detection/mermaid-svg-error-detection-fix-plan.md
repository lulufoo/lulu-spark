# Mermaid SVG 错误检测修复方案

源码：<https://github.com/lulufoo/lulu-workbench/blob/main/frontend/src/shared/mermaid-render.ts>

## 方案

✅ Verified（`frontend/src/shared/mermaid-render.ts`）：当前通过全文搜索 SVG 字符串中的 `error-icon` 判断渲染失败；正常 Mermaid SVG 的样式文本也会包含该选择器。

⚠️ Inferred（基于当前渲染器与 `tests/shared/mermaid-render.test.js` 的错误 SVG 形状）：解析 SVG 后只检测实际 `.error-icon` 元素，可保留失败 SVG 的源码回退，同时不误匹配样式规则。

## 验收

✅ Verified（`task_427b72deef82`）：解析失败的 Mermaid 仍须显示源码回退，不得把 Mermaid 的错误图插入文档。

⚠️ Inferred：新增正常 SVG 含 `.error-icon` 样式规则的测试；保留错误 SVG 与 `render()` 抛错的回退测试。
