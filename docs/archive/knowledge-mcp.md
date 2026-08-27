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

✅ Verified（源码）：`src-tauri/src/services/mcp_protocol_adapter.rs` 登记 `cursor_ide` / `workbench` 槽；path 形态为 `/mcp/<scene_slot>`。  
✅ Verified（源码）：正式缺省 `DEFAULT_PROD_MCP_PORT = 9876`（`settings.rs` / `lib.rs` 再导出）。  
沙箱 App（`TestSandbox=true`）请把 URL 端口改成实例 `config.toml` 中的 `mcp_port`（或沙箱缺省 `19876`）。  
✅ Verified（源码）：Host 不再 spawn `packages/knowledge-mcp/index.mjs`；`packages/knowledge-mcp/` 与 `archive/knowledge-mcp/` 均已从仓库移除。  
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

保存后 Reload MCP；`tools/list` 应含 `cursor_ide` slot 的 API 面（corpus/archive + todo）。连接失败或未知 slot 硬拒绝只阻断 **IDE 通道**验收，不阻断 App Binding（`workbench`）通道。

## 与 App 通道的区别

| 通道 | 配置方式 | URL 形态 | API 面 |
|------|----------|----------|--------|
| App（Binding） | key-only Binding Set；Host 按业务 key 直接查 registry | `http://127.0.0.1:<mcp_port>/mcp/workbench` | corpus/archive + todo |
| 外部 IDE | 人工 `mcp.json`（本文） | `http://127.0.0.1:<mcp_port>/mcp/cursor_ide` | corpus/archive + todo |

✅ Verified（源码）：`src-tauri/src/services/mcp_server_registry.rs` 种子 key `workbench` → `/mcp/workbench`。
✅ Verified（源码）：`mcp_protocol_adapter.rs` 的 `workbench` 槽 `include_corpus` + `include_todo`；不再挂 notes-selection 工具。
✅ Verified（源码）：`src-tauri/src/services/agent/loop.rs::try_set_binding_json` 直接按业务 key 调用 `mcp_server_registry::lookup`，并把结果加载到 Host session capability；当前路径不经过 `mcp_endpoint_readiness`，也不注入 SDK `mcpServers`。
