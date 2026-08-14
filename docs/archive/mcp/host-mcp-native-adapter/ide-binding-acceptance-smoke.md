# Host MCP IDE/Binding 验收冒烟记录（T11）

来源：tech-doc L19-T T11 / L02-CTX / F-28（cycle `feature-20260804150233-cfadba96`）。

**总状态**

| 项 | 值 |
|---|---|
| IDE confirmation: BLOCKED | 本机未完成 Cursor IDE `mcp.json` → Host URL 的人工 list/call 确认 |
| Binding confirmation: BLOCKED | 本机未跑 Workbench App Binding 路径做消费侧确认 |
| Live Host `:9876` list/call | 未跑通（`ECONNREFUSED`） |
| AC: NOT_PASSED | 按 F-28 / T11：无 IDE 确认前不得宣称 AC 通过 |

## 目标合同（调用方无感）

- Host URL 形态：`http://127.0.0.1:9876/mcp/<slot>`
- 槽位：`cursor_ide`（外部 IDE）与 `todo_task`（Binding）
- Host Agent Loop 通过 Host MCP registry 使用 `todo_task` capability；不依赖 Cursor Agent runner sidecar

## 清单

- [ ] 在 `http://127.0.0.1:9876/mcp/<slot>` 上对 `cursor_ide` + `todo_task` 完成 list/call 验收冒烟（IDE + Binding 消费侧）
- [x] Host Agent Loop 不依赖 Cursor Agent runner sidecar（源码）
- [x] 无 IDE 确认时保持验收阻断（F-28 / L19-T T11）— **IDE confirmation: BLOCKED**；**AC: NOT_PASSED**

## 自动化 / 探针结果（本任务执行时）

### 1) 存活 Host（`http://127.0.0.1:9876`）

✅ Verified（工具输出，`2026-08-05T14:00:11.771Z`）：`GET /health` → `ECONNREFUSED`；`host_reachable: false`。  
因此 **未**对存活 Host 执行 `tools/list` / `tools/call` 冒烟。

### 2) Host 进程内双槽合同冒烟（cargo，非 IDE）

✅ Verified（命令输出）：

```text
cargo test --lib p3_t10_host_dual_slot_list_call_and_unknown_hard_fail_smoke -- --test-threads=1
→ ok（1 passed；夹具经 `TestSandbox` 注入，无需 `TEST_MODE`）
```

说明：该测在 Host MCP 监听上覆盖 `todo_task` / `cursor_ide` 的 list/call 与未知槽硬拒绝；**不替代** Cursor IDE 或 App Binding 消费侧确认。

### 3) Host-only 运行时边界

✅ Verified（源码）：

- `packages/cursor-agent-runner/` 已从仓库移除。
- `src-tauri/src/lib.rs` 不再暴露 runner 启动路径。
- `src-tauri/src/services/mcp_protocol_adapter.rs` **不含** `cursor-agent-runner` 引用（未内嵌进 Host MCP Adapter）。
- App Binding 的 key-only Set 由 `src-tauri/src/services/agent/loop.rs::try_set_binding_json` 直接按业务 key 查 Host registry，并加载到 session capability。

## IDE / Binding 手工确认步骤（解除阻断时执行）

1. 启动 Workbench App，确认 `GET http://127.0.0.1:9876/health` 返回 JSON（`ok` + `mcp`）。
2. **IDE（`cursor_ide`）**：按 `docs/knowledge-mcp.md` 配置 `mcp.json` → `http://127.0.0.1:9876/mcp/cursor_ide`；Reload MCP；完成 `tools/list` + 至少 1 次代表性 `tools/call`。
3. **Binding（`todo_task`）**：在 App Agent 路径确认 key-only Set 查到 Host registry 中的 `todo_task` 配置，并完成 list + 至少 1 次代表性 call。
4. 两项均确认后，方可把本记录中的 `IDE confirmation: BLOCKED` / `AC: NOT_PASSED` 改为已确认/通过（本任务不宣称通过）。
