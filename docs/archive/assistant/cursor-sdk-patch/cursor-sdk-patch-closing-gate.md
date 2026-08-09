# Cursor SDK Patch Closing Gate

Official SDK: https://cursor.com/docs/sdk/typescript

## Verdicts

- contract: ✅ Verified（源：`src-tauri/src/unit-tests/services/agent/e2e_gate_tests.rs`、`packages/cursor-agent-runner/tests/`）— 合同通过
- live: ✅ Verified（源：`packages/cursor-agent-runner/tests/live-smoke.test.ts`，本机无 `CURSOR_API_KEY`）— skip，非伪绿
- cursor_ac: ✅ Verified（源：`e2e_gate::evaluate_closing_gate`）— PendingExternalAcceptance / 外部验收待完成
- marks_cursor_ac_from_mock: ✅ Verified（源：`e2e_gate.rs`）— false；mock 不得标记 Cursor AC 已满足

## Infrastructure checks

- runner_installable: ✅ Verified（源：根 `package.json` test 脚本安装 `packages/cursor-agent-runner`）
- runner_startable: ✅ Verified（源：`packages/cursor-agent-runner/tests/runner-jsonl.test.ts`）
- cursor_route_reachable: ✅ Verified（源：`e2e_gate_tests` / `runtime_tests` Cursor 路由）
- endpoint_health_ok: ✅ Verified（源：`mcp_endpoint_readiness` 单测）

## Notes

- 合同通过 ≠ live 通过。
- 默认 CI 无密钥：live-smoke status=`skip`，保留「外部验收待完成」。
- 仅真实 `Agent.create` + 无副作用 `send`/`wait` 且 MCP 可见时，方可 SatisfiedByLive。
