# Workbench 知识库 MCP（Cursor 连接指南）

LuLu Workbench App 启动时由 Host 内嵌 Protocol Adapter 提供 Streamable HTTP MCP（`http://127.0.0.1:9876/mcp/<scene_slot>`）。外部 Cursor IDE 通过 `mcp.json`（或等价配置）手配 HTTP URL，**不经** Host Binding。

## 外部 IDE 通道（`cursor_ide`）

| 项 | 约定 |
|----|------|
| scene_slot | `cursor_ide`（暂名；改名成本限于 MCP 种子与本文/`mcp.json`） |
| HTTP URL | `http://127.0.0.1:<mcp_port>/mcp/cursor_ide` |
| API 面 | corpus/archive + todo |
| 配置面 | Cursor IDE `mcp.json`（或等价），**不经** Binding |
| 端口占位 | `<mcp_port>` = Host MCP 监听端口；正式缺省 `9876`，沙箱缺省 `19876`（`config.toml` 的 `mcp_port`；见 `settings.rs`）。 |

✅ Verified（源码）：`src-tauri/src/services/mcp_protocol_adapter.rs` 登记 `cursor_ide` / `todo_task` 槽；path 形态为 `/mcp/<scene_slot>`。  
✅ Verified（源码）：正式缺省 `DEFAULT_PROD_MCP_PORT = 9876`（`settings.rs` / `lib.rs` 再导出）。  
沙箱 App（`TestSandbox=true`）请把 URL 端口改成实例 `config.toml` 中的 `mcp_port`（或沙箱缺省 `19876`）。  
✅ Verified（源码）：Host 不再 spawn `packages/knowledge-mcp/index.mjs`（该包已归档至 `archive/knowledge-mcp/`）。  
✅ Verified（tech-doc L06-AR / L19-T T10）：IDE 通道仅 `mcp.json` 手配，不经 Binding；Node 包非运行时 SSOT。

### 可按字面执行的 `mcp.json` 示例

先启动 Workbench App（Host 监听 MCP），再在 Cursor MCP 配置中写入：

```json
{
  "mcpServers": {
    "workbench-knowledge-ide": {
      "url": "http://127.0.0.1:9876/mcp/cursor_ide"
    }
  }
}
```

若本机 MCP 端口不是 `9876`，将 URL 中的端口换成实际值，path 仍为 `/mcp/cursor_ide`。

保存后 Reload MCP；`tools/list` 应含 `cursor_ide` slot 的 API 面（corpus/archive + todo）。连接失败或未知 slot 硬拒绝只阻断 **IDE 通道**验收，不阻断 App Binding（`todo_task`）通道。

## 与 App 通道的区别

| 通道 | 配置方式 | URL 形态 | API 面 |
|------|----------|----------|--------|
| App（Binding） | Host registry/readiness 注入 SDK `mcpServers` | `http://127.0.0.1:<mcp_port>/mcp/todo_task` | todo only |
| 外部 IDE | 人工 `mcp.json`（本文） | `http://127.0.0.1:<mcp_port>/mcp/cursor_ide` | corpus/archive + todo |

✅ Verified（源码）：`src-tauri/src/services/mcp_server_registry.rs` 种子 key `todo_task` → `/mcp/todo_task`。
