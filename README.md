# lulu-workbench

个人 AI 工作台。包含对话知识归档库（DDM 流程）与个人 AI skill 集合。

## 双仓库 Setup

程序与语料分属两个 Git 仓库：

1. **Clone 两个仓库**
   - `git clone https://github.com/lulufoo/lulu-workbench.git`
   - `git clone https://github.com/lulufoo/lulu-workbench-knowledge.git`
2. **配置路径** — 首次启动 App 打开 **设置**，或编辑 `~/.config/lulu-workbench/config.toml`：
   ```toml
   workbench_knowledge_root = "/你的本地路径/lulu-workbench-knowledge"
   knowledge_corpus_root = "/你的本地路径/Code"   # 沉淀知识库各 topic 仓库 clone 根目录
   cache_dir = "/Users/你的用户名/.cache/lulu-workbench"   # 可选；默认即此路径，一般无需改
   github_user_url = ""   # 可选；个人 GitHub 主页，如 https://github.com/lulufoo（结合 workbench_knowledge_root 目录名生成 blob 链接）
   meili_url = "http://localhost:7700"
   ```
   GitHub Token、Meili Master Key 在设置页写入 Keychain。
3. **外置 Meilisearch** — 见 [docs/meilisearch-dev.md](docs/meilisearch-dev.md)（App 不启动 Meili）
4. **启动 App** — 在 workbench 根目录执行 `cargo tauri dev`（或安装 release 后从启动台打开）。**无需** Python 或 `server.py`。
5. **Cursor 读 digest（可选）** — 先启动 App，再在 Cursor 配置 MCP `url`：`http://127.0.0.1:9876/mcp`。详见 [docs/knowledge-mcp.md](docs/knowledge-mcp.md)（App spawn sidecar；TPM SKILL 改 MCP 为后续 follow-up）。

**⇕ 同步**（↑ 提交变更 / ↓ 更新项目）针对语料仓库；程序仓库变更在 `lulu-workbench` 目录内 `git` 提交。

## 开发

```bash
npm install          # 前端单测（vitest）
cd src-tauri && cargo test --lib -- --test-threads=1
cargo tauri dev      # 官方运行时（同时启 localhost HTTP :8765 + MCP sidecar :9876）
```

**MCP / digest 读 API：** [docs/knowledge-mcp.md](docs/knowledge-mcp.md) — Cursor 连接、AC-6 降级说明、VF 手动验收清单。

## SSOT 迁移与 TEST_MODE

sediment-kb 与 Read Later 的 SSOT（单一数据源）已迁入语料仓 `workbench_knowledge_root`：

- `sediment-kb/categories.json`、`sediment-kb/repos.json`
- `read_later/read_later.json`
- `plan_tasks/plan_tasks.json`

`cache_dir` 仍保留 drafts、repo-commits、Meili 等可重建数据。**`TEST_MODE` 仅作进程环境变量，不写入 `config.toml`。**

### TEST_MODE=1 三态

| 状态 | 触发条件 | 读路径 | 写路径 |
|------|----------|--------|--------|
| **正式运行（prod）** | 未设 `TEST_MODE` | 语料仓 SSOT | 语料仓 SSOT |
| **自动化测试（automated test sandbox）** | `TEST_MODE=1` + `cargo test` | TestSandbox 临时目录 | TestSandbox 临时目录 |
| **人工调试（manual cache-first）** | `TEST_MODE=1` + 手动启动 App | cache 中旧 SSOT 若存在则读 cache，否则读语料仓 | 始终写语料仓 |

人工 cache-first 模式下，cache 与语料仓内容不一致时，**读侧以 cache 为准（debug overlay），写侧以语料仓为准**（不会因 cache 存在而写回 cache）。

恢复正式运行：在同一 shell 执行 `unset TEST_MODE` 后重启 App，读路径恢复为 **prod 只读语料仓** 语义（不再 overlay cache）。

### 部署与迁移顺序

升级含 SSOT 路径变更的版本后，按顺序执行（迁移为**一次性临时脚本**，**非** App 启动挂钩）：

1. **deploy** 新代码（安装 release 或拉取并构建）
2. 运行 `./scripts/migrate-local-state` — 幂等将 `cache_dir` 中旧 SSOT 复制到语料仓
3. 在语料仓目录 **`git commit`** SSOT 变更

### AC-5：人工 cache-first 验证

用于确认 `TEST_MODE=1` 下读 cache、写语料仓的行为（与 `npm test` / `cargo test` 的 TestSandbox 无关）：

1. 确认 `config.toml` 中 `workbench_knowledge_root` 与 `cache_dir` 已配置。
2. **故意不一致**：在语料仓写入 SSOT 文件 A；在 `cache_dir/sediment-kb/`、`cache_dir/read_later.json` 或 `cache_dir/plan_tasks.json` 写入不同内容的旧路径文件 B。
3. `export TEST_MODE=1`，手动启动 App（`cargo tauri dev` 或 release）。
4. **读 cache**：在 App 中查看 sediment-kb 或 Read Later，应显示 cache 内容 B。
5. **写语料仓**：执行一次写操作（如新增 Read Later 条目或修改分类），确认更新的是语料仓路径下的文件，而非 cache。
6. **`unset TEST_MODE`**，重启 App。
7. **读回语料仓**：读操作应返回语料仓 SSOT（步骤 5 写入后的内容），不再 overlay cache。
