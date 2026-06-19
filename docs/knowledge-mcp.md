# Workbench 知识库 MCP（Cursor 连接指南）

LuLu Workbench 在 App 启动时会在本机暴露 **digest 只读** HTTP API，并由 App **spawn** 薄 MCP sidecar（Streamable HTTP）。Cursor 通过 `mcp.json` 的 `url` 连接 sidecar；sidecar **不**直接读语料盘，仅 proxy 到 Workbench HTTP。

**前提：** 必须先启动 Workbench App，再配置 Cursor。Sidecar 由 App 托管，Cursor **不要**自行 spawn sidecar。

## 架构与端口

| 组件 | 默认地址 | 说明 |
|------|----------|------|
| Workbench read HTTP | `http://127.0.0.1:8765` | `GET /api/corpus-index`、`/api/corpus-file`、`/api/status` |
| MCP sidecar | `http://127.0.0.1:9876/mcp` | tools：`get_corpus_index`、`get_corpus_file`（固定 `layer=digest`） |

App `setup()` 顺序：启动 `local_http`（8765）→ **仅当 HTTP listen 成功** 时 spawn `node packages/knowledge-mcp/index.mjs`；App `Exit` 时 kill sidecar 并停止 HTTP。

Sidecar 环境变量（由 App 注入，一般无需手改）：

| 变量 | 示例 |
|------|------|
| `WORKBENCH_HTTP_URL` | `http://127.0.0.1:8765` |
| `MCP_PORT` | `9876` |

## 配置步骤

### 1. 启动 Workbench

在 workbench 仓库根目录：

```bash
cargo tauri dev
```

确认 HTTP 与 sidecar 就绪（任选其一）：

```bash
curl -s http://127.0.0.1:8765/api/status
curl -s http://127.0.0.1:9876/health
```

stderr 正常时应含 `[local_http]` bind 成功、`[knowledge-mcp] ready on port 9876` 类日志。

### 2. 配置 Cursor `mcp.json`

在 Cursor MCP 配置（项目或用户级）添加：

```json
{
  "mcpServers": {
    "workbench-knowledge": {
      "url": "http://127.0.0.1:9876/mcp"
    }
  }
}
```

保存后打开 Cursor MCP 面板，应能看到 `get_corpus_index`、`get_corpus_file`。

### 3. 调用示例（AC-5）

在 MCP 面板或通过 Agent 调用：

- `get_corpus_index` — 返回与 Tauri `get_corpus_index` 一致的 `index.json` JSON 文本
- `get_corpus_file` — 参数 `{ "path": "topic-id/relative/path.md" }`（digest 相对路径）

## App 未运行 / HTTP 失败时的表现（AC-6）

| 场景 | MCP / HTTP 表现 |
|------|-----------------|
| **Workbench 未启动** | Cursor 连接 `http://127.0.0.1:9876/mcp` 失败（连接 refused / 超时）；无法 list tools 或 call tool |
| **8765 bind 失败（端口占用等）** | App stderr：`[local_http] bind failed on port 8765: …`；**不** spawn sidecar（`HTTP unavailable, MCP sidecar skipped`）；9876 无监听 |
| **8765 正常但 sidecar 未起** | HTTP `curl /api/corpus-index` 可能仍 200；MCP 不可用（9876 无进程） |
| **App 运行中 HTTP 不可达** | sidecar tool 返回 **tool error**，文本形如 `HTTP 404: …` 或 `HTTP 500: …`（透传 HTTP status，无读盘 fallback） |
| **关闭 App 后** | sidecar 随 App exit 被 kill；Cursor 再次 call tool → 连接失败或 tool error（AC-6） |

Sidecar **禁止**读取 `workbench_knowledge_root` 文件系统；HTTP 不可达时不会静默返回空内容。

## 故障排查

### 端口占用

```bash
lsof -i :8765
lsof -i :9876
```

- **8765 被占用：** App 仍可用 UI，但 MCP 链路不可用；释放端口或结束占用进程后重启 App。
- **9876 被占用：** stderr：`port 9876 already in use — spawn failed`；释放 9876 后重启 App。

MVP **不会**自动换端口；默认始终为 8765 / 9876。

### `node` 不在 PATH

spawn 使用系统 `node`（dev MVP，未 bundled）。若未安装或不在 PATH：

- stderr：`[knowledge-mcp] spawn failed (…)`
- App 主窗口仍可用；安装 Node.js 并确保 `node --version` 可执行后重启 App。

Sidecar 脚本路径：`{repo_root}/packages/knowledge-mcp/index.mjs`（相对 App 解析的仓库根）。

### 语料路径

digest 内容来自 `~/.config/lulu-workbench/config.toml` 的 `workbench_knowledge_root`（与 UI 读路径相同）。配置错误时 HTTP 可能 404/500，MCP tool 会透传该错误文本。

## 自动化验证

Sidecar 包内 mock 验证（**不**依赖 App）：

```bash
cd packages/knowledge-mcp
npm install
node scripts/verify.mjs
```

App 联调（需 App 已启、语料已配置）：

```bash
curl -s http://127.0.0.1:8765/api/corpus-index | head
curl -s "http://127.0.0.1:8765/api/corpus-file?layer=digest&path=YOUR_DIGEST_PATH" | head
```

## 手动验收清单（VF）

与 tech-plan Verification 节（VF）对齐：

| AC | 步骤 | 预期 |
|----|------|------|
| AC-1 | `curl -s http://127.0.0.1:8765/api/corpus-index` | 200 JSON |
| AC-2 | `curl` 已知 digest path，对比 Tauri invoke | 内容一致 |
| AC-3 | 启停 App；`ps aux \| grep knowledge-mcp` | 运行时有进程，退出后无残留 |
| AC-4 | `rg 'readFile\|fs\.' packages/knowledge-mcp` | 无 corpus 路径读盘 |
| AC-5 | Cursor MCP 面板 list + call `get_corpus_file` | 成功返回 digest 文本 |
| AC-6 | 关闭 App 后再 call MCP tool | 连接失败或明确 tool error |
| AC-7 | `curl ".../api/corpus-file?layer=raw&path=x"` | 400 |

Tech plan 原文：`.cache/cursor/lulu-dev-workflow/feature-20260619075159-be47f7d7/tech/plan/revision1/tech-doc.md`（Verification / VF 节）。

## 明确排除与 follow-up

- **TPM SKILL**（`lulu-learning-skills` 等）仍通过 `gh api` 读 digest；**本 cycle 不**改为 MCP。后续 follow-up 单独改造 SKILL。
- **Release bundled Node** 不在 MVP；当前 dev 依赖系统 `node`。
- Sidecar 仅 **digest** layer；`layer=raw` / search 等不在 HTTP/MCP 范围。
- Cursor 仅配置 `url` 连接；**不要**在 `mcp.json` 里 spawn `node index.mjs`（避免双实例）。

## 回滚

移除 Cursor `mcp.json` 中 `workbench-knowledge` 条目即可断开 MCP。Tauri UI 内 invoke 读路径不受影响。
