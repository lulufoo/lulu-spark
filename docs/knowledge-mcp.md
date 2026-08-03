# Workbench 知识库 MCP（Cursor 连接指南）

LuLu Workbench App 启动时 spawn knowledge-mcp sidecar（Streamable HTTP）。外部 Cursor IDE 通过 `mcp.json`（或等价配置）手配 HTTP URL，**不经** Host Binding。

## 外部 IDE 通道（`cursor_ide`）

| 项 | 约定 |
|----|------|
| scene_slot | `cursor_ide`（暂名；改名成本限于 MCP 种子与本文/`mcp.json`） |
| HTTP URL | `http://127.0.0.1:<mcp_port>/mcp/cursor_ide` |
| 配置面 | Cursor IDE `mcp.json`（或等价），**不经** Binding |
| 端口占位 | `<mcp_port>` = sidecar 监听端口；默认与 `MCP_PORT` / `DEFAULT_MCP_PORT` 一致为 `9876`（见 `packages/knowledge-mcp/index.mjs`、`src-tauri/src/lib.rs`）。若环境改写了 `MCP_PORT`，把占位换成实际端口。 |

✅ Verified（源码）：`packages/knowledge-mcp/index.mjs` 登记 `cursor_ide` slot，path 形态为 `/mcp/<scene_slot>`；默认 `MCP_PORT=9876`。  
✅ Verified（源码）：`src-tauri/src/lib.rs` 中 `DEFAULT_MCP_PORT = 9876`。  
✅ Verified（tech-doc L06-AR / L11-T T7）：IDE 通道仅 `mcp.json` 手配，不经 Binding。

### 可按字面执行的 `mcp.json` 示例

先启动 Workbench App（sidecar 由 App 托管），再在 Cursor MCP 配置中写入：

```json
{
  "mcpServers": {
    "workbench-knowledge-ide": {
      "url": "http://127.0.0.1:9876/mcp/cursor_ide"
    }
  }
}
```

若本机 `MCP_PORT` 不是 `9876`，将 URL 中的端口换成实际值，path 仍为 `/mcp/cursor_ide`。

保存后 Reload MCP；`tools/list` 应仅含 `cursor_ide` slot 的 API 面。连接失败或未知 slot 硬拒绝只阻断 **IDE 通道**验收，不阻断 App Binding（`todo_task`）通道。

## 与 App 通道的区别

| 通道 | 配置方式 | URL 形态 |
|------|----------|----------|
| App（Binding） | Host registry/readiness 注入 SDK `mcpServers` | `http://127.0.0.1:<mcp_port>/mcp/todo_task` |
| 外部 IDE | 人工 `mcp.json`（本文） | `http://127.0.0.1:<mcp_port>/mcp/cursor_ide` |

✅ Verified（源码）：`src-tauri/src/services/mcp_server_registry.rs` 种子 key `todo_task` → `/mcp/todo_task`。
