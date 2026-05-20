# 外置 Meilisearch 开发指南

LuLu Workbench P4 使用**外置** Meilisearch HTTP 服务。Tauri App **不会**启动 Meili 进程；搜索、reindex、settle upsert 均由 Rust 经 HTTP 调用外置实例。

> 遗留 `server.py` 仅作可选浏览器调试，**不是** P4 官方运行时。

## 版本 pin

| 组件 | 版本（验证记录） |
|------|------------------|
| Meilisearch | `1.43.0`（`meilisearch --version`） |

在 `~/.config/lulu-workbench/config.toml` 或 App 设置中记录当前验证版本；升级 Meili 后重跑下方 V1/V2。

## 配置（Tauri 单源）

| 项 | 位置 | 说明 |
|------|------|------|
| `meili_url` | `~/.config/lulu-workbench/config.toml` | HTTP 基址，默认 `http://localhost:7700` |
| `meili_master_key` | Keychain（设置页写入） | Bearer 密钥 |
| `workbench_knowledge_root` / `knowledge_corpus_root` / `cache_dir` | 同上 `config.toml` | 工作台归档仓、沉淀知识库克隆根、`.cache` |

Rust：`src-tauri/src/config/settings.rs`、`secrets.rs`、`meili_env.rs`（**不**读取仓库内 `meili.env`）。

### 从 `meili.env` 迁移（一次性）

若仓库根目录仍有 `meili.env`，将下列键写入 `config.toml`（路径按本机调整）：

```toml
# ~/.config/lulu-workbench/config.toml
meili_url = "http://localhost:7700"
workbench_knowledge_root = "/path/to/lulu-workbench-knowledge"
knowledge_corpus_root = "/path/to/Code"
cache_dir = "/path/to/lulu-workbench/.cache"
github_user_url = "https://github.com/lulufoo"
```

`MEILI_MASTER_KEY` → 在 App **设置** 中保存（写入 Keychain）。完成后可保留 `meili.env` 作备忘，App 不再读取。

### 数据目录 `--db-path`

Tauri App 启动时自动以下列路径启动 Meilisearch（若系统中已安装）：

```
~/.cache/lulu-workbench/.meilisearch
```

即 `config.toml` 中 `cache_dir` 设置项下的 `.meilisearch` 子目录（默认 `~/.cache/lulu-workbench`）。

若需手动启动（与 App 外置共用），可用同一路径保持数据一致：

```bash
export MEILI_KEY="lulu-workbench-local"
meilisearch --no-analytics \
  --db-path ~/.cache/lulu-workbench/.meilisearch \
  --master-key "$MEILI_KEY"
```

## 启动方式

### Homebrew（macOS）

```bash
brew install meilisearch
# 开发时建议前台或 launchd，见上 --db-path
```

### Docker

```bash
docker run -p 7700:7700 \
  -v "$(pwd)/.meilisearch:/meili_data" \
  getmeili/meilisearch:v1.43 \
  meilisearch --db-path /meili_data --master-key "${MEILI_MASTER_KEY:-dev}"
```

## V1 验证（索引可搜索）

在**外置 Meili 已运行**且 `config.toml` + Keychain 配置正确时：

```bash
cd "$REPO_ROOT"
meilisearch --version
# 在 App 内：知识库 reindex / 工作台 reindex，或 devtools：
#   invoke('reindex_knowledge') / invoke('reindex_workbench')
# 轮询 get_reindex_status / get_reindex_workbench_status 至 status=done
curl -s -H "Authorization: Bearer $MEILI_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"q":"test","limit":3}' \
  "http://localhost:7700/indexes/knowledge/search" | head
curl -s -H "Authorization: Bearer $MEILI_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"q":"test","limit":3}' \
  "http://localhost:7700/indexes/workbench/search" | head
```

记录：版本号、两 index 有 hits 或需先 reindex。

## V2 验证（Tauri reindex）

`cargo tauri dev` + 外置 Meili → UI **重建索引** 或 devtools `reindex_workbench` / `reindex_knowledge` → 约 500ms 内 `{ status: "running" }`；轮询至 `done`。**不**再 spawn `python3`。

实现：`src-tauri/src/services/index_build/`（`workbench.rs`、`knowledge.rs`）+ `integrations/search/meili_admin.rs`。

## 索引 UID

| UID | Rust 构建 |
|-----|-----------|
| `knowledge` | `index_build::knowledge::rebuild` |
| `workbench` | `index_build::workbench::full_rebuild` |

## 明确排除

- **不**由 Tauri `setup` autostart Meilisearch
- settle Meili upsert：`integrations/search/meili_backend.rs`（`upsert_knowledge_documents`），非 Python
