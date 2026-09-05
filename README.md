# lulu-workbench

个人 AI 工作台。包含对话知识归档库（DDM 流程）与个人 AI skill 集合。

## 双仓库 Setup

程序与语料分属两个 Git 仓库：

1. **Clone 两个仓库**
   - `git clone https://github.com/lulufoo/lulu-workbench.git`
   - `git clone https://github.com/lulufoo/lulu-workbench-knowledge.git`
2. **配置路径** — 首次启动 App 打开 **设置**，或编辑 `~/.config/lulu-workbench/config.toml`：
   ```toml
   workbench_root = "/你的本地路径/lulu-workbench-knowledge"
   knowledge_root = "/你的本地路径/Code"   # 沉淀知识库各 topic 仓库 clone 根目录
   cache_dir = "/Users/你的用户名/.cache/lulu-workbench"   # 可选；默认即此路径，一般无需改
   github_user_url = ""   # 可选；个人 GitHub 主页，如 https://github.com/lulufoo（结合 workbench_root 目录名生成 blob 链接）
   ```
   GitHub Token 在设置页写入 Keychain。
3. **启动 App** — 在 workbench 根目录执行 `cargo tauri dev`（或安装 release 后从启动台打开）。**无需** Python 或 `server.py`。笔记 / 知识检索走进程内 SQLite FTS5；每次启动自动重建索引（右上角 ↺ Index 转圈即在构建，空闲时可点它手动重建）。
4. **Cursor 读 digest（可选）** — 先启动 App，再在外部 Cursor IDE 的 `mcp.json` 配置 MCP `url`：`http://127.0.0.1:<mcp_port>/mcp/cursor_ide`（默认端口 `9876`）。详见 [docs/knowledge-mcp.md](docs/knowledge-mcp.md)（Host 内嵌 MCP；不经 Binding）。

**⇕ 同步**（↑ 提交变更 / ↓ 更新项目）针对语料仓库；程序仓库变更在 `lulu-workbench` 目录内 `git` 提交。

## 开发

```bash
npm install          # 前端单测（vitest）
cd src-tauri && cargo test --lib -- --test-threads=1
cargo tauri dev      # 官方运行时（同时启 localhost HTTP :8765 + Host MCP :9876）
```

**MCP / digest 读 API：** [docs/knowledge-mcp.md](docs/knowledge-mcp.md) — 外部 IDE `mcp.json`（`cursor_ide`）、App Binding 通道区分、联调说明。

## SSOT 与 TestSandbox

Knowledge 登记、Read Later 与笔记的 SSOT（单一数据源）已迁入语料仓 `workbench_root`：

- `notes/raw/`、`notes/digest/`、`notes/index.json`、`notes/annotations/`、`notes/tags/`
- `knowledge/categories.json`、`knowledge/repos.json`
- `read_later/read_later.json`
- `todo_tasks/todo_tasks.json`

`cache_dir` 仍保留 drafts、keyword-index 等可重建数据。

配置目录由环境变量选择（**不**写入 `config.toml`）：

| 状态 | 触发 | 配置目录 |
|------|------|----------|
| **正式** | 未设 `TestSandbox` | `~/.config/lulu-workbench/config.toml`（默认端口 `8765` / `9876`） |
| **共享沙箱** | `TestSandbox=true` 且无 `TestSandboxId` | `~/.config/lulu-workbench-sandbox/`（模板 + 共享 `dev-secrets.toml`；缺省端口 `18765` / `19876`） |
| **实例沙箱** | `TestSandbox=true` + `TestSandboxId=<id>` | `~/.config/lulu-workbench-sandbox-<id>/config.toml`（由发起方从共享模板 fork） |

技术方案：[`docs/archive/config/test-sandbox-config-port-isolation-tech-plan.md`](docs/archive/config/test-sandbox-config-port-isolation-tech-plan.md)。  
密钥复制到共享沙箱：`./scripts/copy-sandbox-secrets`。

### 部署与迁移顺序

升级含 SSOT 路径变更的版本后，按顺序执行（迁移为**一次性临时脚本**，**非** App 启动挂钩）：

1. **deploy** 新代码（安装 release 或拉取并构建）
2. 运行 `./scripts/migrate-local-state` — 幂等将 `cache_dir` 中旧 SSOT 复制到语料仓
3. 在语料仓目录 **`git commit`** SSOT 变更
