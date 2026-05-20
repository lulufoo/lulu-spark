# 外置 Meilisearch 开发指南

LuLu Workbench P3 使用**外置** Meilisearch HTTP 服务。App（Tauri）与 `server.py` **不会**启动 Meili 进程；搜索与 reindex 由 Rust Tauri command 调用外置实例。

## 版本 pin

| 组件 | 版本（验证记录） |
|------|------------------|
| Meilisearch | `1.43.0`（`meilisearch --version`） |

在 `meili.env` 顶部注释中记录当前验证版本，升级后重跑 V1 脚本。

## 配置（`meili.env`）

| 变量 | 说明 | 默认 |
|------|------|------|
| `MEILI_URL` | HTTP 基址 | `http://localhost:7700` |
| `MEILI_MASTER_KEY` | Bearer 密钥 | 空（本地可设 `lulu-workbench-local`） |

Rust 单源：`src-tauri/src/config/meili_env.rs`（`meili_url` / `meili_master_key`）。Python settle 仍读同一文件。

## 数据目录 `--db-path`

与仓库根目录下的 **`.meilisearch`** 对齐（与历史 Python autostart 一致）：

```bash
export REPO_ROOT="$(git rev-parse --show-toplevel)"
meilisearch --no-analytics \
  --db-path "$REPO_ROOT/.meilisearch" \
  --master-key "$(grep '^MEILI_MASTER_KEY=' meili.env | cut -d= -f2-)"
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

## V1 验证（decision spike）

在**外置 Meili 已运行**且 `meili.env` 配置正确时：

```bash
cd "$REPO_ROOT"
meilisearch --version
python3 scripts/build_workbench_index.py
python3 scripts/build_knowledge_index.py   # 可能较久
curl -s -H "Authorization: Bearer $(grep '^MEILI_MASTER_KEY=' meili.env | cut -d= -f2-)" \
  -H 'Content-Type: application/json' \
  -d '{"q":"test","limit":3}' \
  "$MEILI_URL/indexes/knowledge/search" | head
curl -s -H "Authorization: Bearer $(grep '^MEILI_MASTER_KEY=' meili.env | cut -d= -f2-)" \
  -H 'Content-Type: application/json' \
  -d '{"q":"test","limit":3}' \
  "$MEILI_URL/indexes/workbench/search" | head
```

记录：版本号、两 index 均有 hits 或 `index_not_found`（需先 build）。

## V2 验证（Tauri spawn）

`cargo tauri dev` + 外置 Meili → UI 或 devtools 调用 `reindex_workbench` → 应在约 500ms 内返回 `{ status: "running" }`；轮询 `get_reindex_workbench_status` 至 `done`。依赖 `capabilities/default.json` 中的 `shell:allow-spawn`（Rust 内 `python3 scripts/build_workbench_index.py`）。

## 索引 UID

- `knowledge` — `scripts/build_knowledge_index.py`
- `workbench` — `scripts/build_workbench_index.py`

## 明确排除

- **不**由 `server.py` 或 Tauri `setup` autostart Meilisearch
- settle 路径仍用 Python `_meili_upsert_doc`（非 HTTP）；勿删 `server.py` 内 `_meili_request` 辅助函数
