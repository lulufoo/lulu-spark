# 关键词检索 FTS5 实施计划

**技术方案：** `technical-solution.md`（设计决议与边界，本文不重复）  
**关键技术：** `index-build-key-technologies.md`（rusqlite / FTS5 / sha2，本文不重复）  
**状态：** Implemented + W6 CR 增补（2026-09-05；单测已绿，实机重建时间 / 体积待测）  
**关联待办：** `task_4a8fc6f74e39`（关键词检索 FTS5 替代 Meilisearch）；前序 `task_a2534f91ef79`（语义检索调研，已关闭）  
**对齐来源：** 2026-09-05 `/converge` Q1–Q6

---

## 前置动作

| # | 动作 | 状态 |
|---|---|---|
| P1 | 丢弃语义检索 MVP 未提交改动（`git checkout -- frontend src-tauri tests`，删除 `semantic_index` 与 `.fastembed_cache`），工作区回到 HEAD `587387ff` | ✅ Verified（2026-09-05 用户手动执行；调研文档保留在 `docs/archive/semantic-search-research/`） |
| P2 | Todo 关旧开新：`task_a2534f91ef79` 及 `sub_01` / `sub_02` 标完成并备注"已证伪、代码回滚、文档存 semantic-search-research"；新建 `task_4a8fc6f74e39` | ✅ Verified（MCP `complete_todo` / `create_todo_task` 返回） |

---

## 工作项

按落地顺序。文件路径相对 `src-tauri/src/`，状态以 2026-09-05 工作区为准。

### W1 新模块 `services/keyword_index/`

| 文件 | 职责 | 状态 |
|---|---|---|
| `mod.rs` | 对外 re-export；测试挂 `unit-tests/services/keyword_index.rs` | ✅ Verified |
| `chunk.rs` | ATX 切块；`MAX_CHUNK_CHARS = 1200`；`SHORT_TITLE_MERGE_CHARS = 80`；`first_heading_title` | ✅ Verified |
| `bigram.rs` | CJK 连续段 → 重叠二字切片；非 CJK 段保留为词 | ✅ Verified |
| `collect.rs` | 遍历 notes `raw` / `digest` 与已注册 knowledge 仓；跳过 `.git` / `.cache` 与 `should_skip_md`；notes `doc_id` 取 `index.json`，否则 `workbench_doc_id`；knowledge 用 `knowledge_doc_id` | ✅ Verified |
| `store.rs` | `{cache_dir}/keyword-index.sqlite`，WAL，`SCHEMA_VERSION` 写 `user_version`；`docs_tri` tokenize=trigram + `mtime`，`docs_bi` tokenize=unicode61 + `orig` UNINDEXED；`upsert_chunks` 一次加载 `chunk_id → (hash, mtime)` 再判断 skip（W7）；`replace_path_chunks`；`delete_ghosts`；`distinct_paths` / `indexed_mtimes`；`index_exists` | ✅ Verified |
| `query.rs` | `classify_query` → Empty / Bi / Tri+LIKE；`search` / `fold_by_doc_id`（bm25 越小越好）；`search_in_cache`；桌面适配 `search_desktop_workbench` / `search_desktop_knowledge`（Meili 形状，缺索引返回 `not_indexed`） | ✅ Verified |
| `grep_prefilter.rs` | `extract_literals`（含 `\|` → 全扫；量词后缀丢弃该段；≥3 字才算字面量）；`prefilter_or_none` → `GrepPrefilter { candidates, indexed }`；`should_scan_file`（已索引、未变新且非候选 → 跳过；未索引或文件更新 → 扫） | ✅ Verified |
| `sync.rs` | `sync_note_files` / `sync_note_files_best_effort`：App 内笔记写路径的单文件同步（W6-5） | ✅ Verified |

`Cargo.toml` 新增 `rusqlite = { version = "0.32", features = ["bundled"] }`。✅ Verified

### W2 入口接线

| 入口 | 改动 | 状态 |
|---|---|---|
| `services/workbench_read/notes_catalog.rs::search_notes` | `search_in_cache` + `category=notes` / `layer=raw` / catalog 前缀；仍经 `project_raw_search_hits` 与 `index.json` 投影 | ✅ Verified |
| `services/knowledge/mcp.rs::search_knowledge_mcp` | FTS5 + `doc_map::remember_path` | ✅ Verified |
| `commands/read.rs::search_knowledge / search_workbench` | 走 `search_desktop_*`，经 `cache_dir_or_err()` | ✅ Verified |
| `agent/tools/fs.rs::grep` | 可选预筛；`walk_grep` / `grep_file` 接 `should_scan_file`；输出格式与上限不变 | ✅ Verified |
| `services/index_build/workbench.rs::full_rebuild` | `collect_notes` → `keyword_index::rebuild(category=notes)` | ✅ Verified |
| `services/index_build/knowledge.rs::rebuild` | `collect_knowledge` → `keyword_index::rebuild(category=knowledge, repo)`；`repo-commits.json` 已在 W6-8 移除 | ✅ Verified |
| `services/settle.rs` | `keyword_upsert_best_effort` 替代 `meili_upsert_best_effort`，用 `collect_knowledge_text` + `upsert_document`（W6 起为按路径替换、缺索引 no-op） | ✅ Verified |

### W3 Meili 移除

| 项 | 改动 | 状态 |
|---|---|---|
| 删除文件 | `config/meili_env.rs`、`integrations/meilisearch.rs`、`integrations/search/{mod,meili_admin,meili_backend}.rs`、`unit-tests/integrations/search/meili_backend.rs` | ✅ Verified（`git status` 显示 `D`） |
| 路径 helper 迁移 | `meili_env` 的 `workbench_root_path` / `notes_root_path` / `knowledge_root_string` / `github_user_url_string` 移至 `config/roots.rs`；全部 import 改写 | ✅ Verified |
| settings / secrets | 移除 `AppSettings.meili_url`、`KEY_MEILI_MASTER`、`has_meili_key`、`meili_master_key` payload、sandbox `meili_url` 撞车检查；`to_config_json` 变为 3 参数 | ✅ Verified |
| host | 移除 `MeiliProcess` / `try_autostart_meilisearch` 及 `lib.rs` 的 manage / kill | ✅ Verified |
| 测试夹具 | `write_test_config_with_cache` 去掉 `meili_url` 参数；相关调用点同步 | ✅ Verified |

### W4 前端与文档

| 项 | 改动 | 状态 |
|---|---|---|
| `frontend/src/notes/ui/search.tsx`、`knowledge/ui/search.tsx`、`knowledge/ui/knowledge-search.tsx` | 去掉 "Meilisearch is not running" 文案；`not_indexed` → "Index not built yet. Use ↺ Index in the header."；其他错误 → "Search error" | ✅ Verified |
| `frontend/src/boot.ts` | 清理 Meili 注释 | ✅ Verified |
| `tests/notes/search.test.js`、`tests/knowledge/knowledge-search.test.js` | 断言随文案更新 | ✅ Verified（vitest 27/27） |
| `docs/architecture/arch-layer-constraints.md` | L5 只剩 GitHub；删 `MEILI` 节点与 `IDX -.-> Meili HTTP` 边 | ✅ Verified |
| `README.md` | 启动步骤改为 FTS5 + ↺ 建索引；步骤编号修正 | ✅ Verified |
| `docs/archive/meilisearch-dev.md` | 保留为历史，不改 | — |

### W5 单元测试

`unit-tests/services/keyword_index.rs`，✅ Verified（2026-09-05 `cargo test --lib -- --test-threads=1 keyword_index …` 13/13）：

| 用例 | 覆盖 |
|---|---|
| `classify_routes_by_term_length` | 1 字空 / 纯 2 字 Bi / ≥3 Tri / 混合 Tri+LIKE |
| `short_title_merges_into_next_chunk` | Q5 短标题合并 |
| `no_heading_is_one_chunk_until_cap` | 无标题整篇一块 |
| `content_hash_skips_unchanged_and_ghost_delete_removes_stale` | 增量与幽灵删除 |
| `two_char_and_mixed_queries_return_snippets` | 「角色」纯二字与 `search 角色` 混合均有片段 |
| `grep_prefilter_matches_full_scan_on_random_patterns` | 100 组正则，预筛 ≡ 全扫（3 个已索引 md + 1 个未索引 txt） |
| `desktop_search_reports_not_indexed_when_sqlite_missing` | 缺索引文件返回 `not_indexed` |
| `open_drops_tables_from_older_schema_version` | W6-7 `user_version` 不符 → 删表重建，`mtime` 列可查 |
| `replace_path_chunks_drops_stale_tail_chunks` | W6-5 整篇替换不留尾块；空块 = 删除 |
| `upsert_document_is_noop_without_index_file` | 写钩子不制造半成品索引 |
| `desktop_workbench_hits_always_expose_raw_layer` | W6-4 digest 命中输出 `layer=raw` |
| `grep_prefilter_forces_scan_when_file_newer_than_index` | W6-7 文件比索引新 → 必扫；重建后可跳 |
| `prefilter_does_not_create_missing_index_file` | 预筛不产生副作用文件 |

`unit-tests/services/entry_write.rs::save_entry_syncs_keyword_index_for_touched_file`：W6-5 保存后单文件可检索；删除后 `sync_note_files` 清空该路径。✅ Verified

`unit-tests/mcp_host.rs::proxy_dispatches_search_document_on_blocking_worker`：沙箱无索引文件时接受映射后的 `not_indexed` 工具错误（非 worker panic）。✅ Verified

前端 `tests/app-shell/index-rebuild.test.js`（4 例）：单一控件 / 启动接管转圈 / 点击一次触发并轮询到完成、运行中不重复触发 / 后端错误回显。✅ Verified

### W6 CR 增补与清理

**对齐来源：** 2026-09-05 CR 三条 + `/converge` 五项定夺（UI 单入口 / 不拉仓 / 露出笔记本身 / App 内写路径同步 / MCP 503）。

| # | 项 | 改动 | 状态 |
|---|---|---|---|
| W6-1 | 重建入口收敛 | 删除 `notes/ui/search.tsx`、`knowledge/ui/search.tsx` 两个搜索框旁 ↺ 及其轮询代码；删除 `knowledge-search.tsx` 面板标题 ↺（同为全量拉仓+建索引）与 `.ks-refresh-btn` / `.ks-sync-bar` / `.ks-unavailable` 样式；新增 header 右上角单一「↺ Index」控件（`app-shell/{ui,commands,state}/index-rebuild`，`#btn-index-rebuild`）：运行中转圈不可点，空闲可点，一次触发 notes + knowledge 两段重建 | ✅ Verified（tsc 通过；vitest 相关 11 文件 93/93） |
| W6-2 | 启动自动建索引 | `lib.rs` setup 后台线程调用 `reindex::run_all_reindex_blocking`（notes → knowledge 顺序执行，写两个 `JobState` 槽位）；`IndexRebuildButton` 挂载即 `watchIndexRebuildOnBoot` 轮询 `get_reindex_all_status` | ✅ Verified（代码 + 单测）；实机转圈 ❌ Unresolved |
| W6-3 | 重建不拉仓 | 删除 `reindex_knowledge` / `reindex_workbench` / `get_reindex_workbench_status` 命令与 `run_knowledge_reindex_blocking` / `run_workbench_reindex_blocking`；新增 `reindex_all` / `get_reindex_all_status`（`permissions/search-api.toml` 同步）；`sync_knowledge`（Sync 菜单）与 `reindex_kb_repo`（单仓 Sync）保留原语义 | ✅ Verified |
| W6-4 | 桌面笔记命中露出笔记本身 | `search_desktop_workbench` 命中 `layer` 固定为 `raw`（digest 仅参与召回，`fold_by_doc_id` 已按 `doc_id` 折叠） | ✅ Verified |
| W6-5 | App 内写路径同步索引 | 新增 `keyword_index/sync.rs::sync_note_files(repo_root, [(layer, common_path)])`：文件存在 → `replace_path_chunks` 整篇替换；不存在 → 删除该路径块；索引文件不存在时 no-op。挂到 `notes/document.rs::write_note`、`entry_write.rs::save_entry`、`entry_admin.rs::delete_entry / move_entry_project`（best-effort，不影响主响应）。外部改 digest 仍靠重建 | ✅ Verified |
| W6-6 | MCP 缺索引 503 | `notes_catalog::search_notes`、`knowledge/mcp.rs::search_knowledge_mcp` 在 `!index_exists` 时返回 `{ error: "not_indexed", _status: 503 }`；`search_document` 两侧皆错时透传 | ✅ Verified |
| W6-7 | grep 预筛防陈旧 | `docs_tri` 增 `mtime UNINDEXED`（纳秒），`SCHEMA_VERSION = 2` 写入 `PRAGMA user_version`，不符则删表重建；`prefilter_or_none` 返回 `GrepPrefilter { candidates, indexed: path → mtime }` 且缺索引时不创建文件；`should_scan_file`：文件 mtime > 索引 mtime → 强制扫描；hash 未变但 mtime 变时只更新 `mtime` 列 | ✅ Verified |
| W6-8 | 清理 `repo-commits.json` | 删除 `index_build/knowledge.rs` 提交缓存读写、`repo_head`、`reindex.rs::clear_repo_commit_cache / run_kb_reindex_blocking`；`rebuild_knowledge_index(repo_root, force_repo)` 去掉 `wipe`；README 去掉 `repo-commits` | ✅ Verified |
| W6-9 | 清理 `index_build` 死代码 | 删除 `common.rs::{BATCH_SIZE, extract_date_from_filename, workbench_topic_from_path, build_workbench_document, upsert_batches}`、`workbench.rs::collect_documents`、`knowledge.rs::collect_repo_documents` 及 `unit-tests/services/index_build/{workbench,knowledge}.rs` | ✅ Verified |

### W7 `existing_hash` 一次加载

**对齐来源：** 2026-09-05 实机 `sample`：启动重建卡在 `upsert_chunks` → `existing_hash`（`UNINDEXED` 列逐条 `WHERE` = 全表扫，约 N²）。Todo：`task_4a8fc6f74e39_sub_01`。

| # | 项 | 改动 | 状态 |
|---|---|---|---|
| W7-1 | 重建查 hash 改一次扫表 | `upsert_chunks` 开头 `load_existing_hashes`（`SELECT chunk_id, content_hash, mtime FROM docs_tri` → HashMap）；循环内 `map.get`；`existing_hash` 单行查询保留给测试 / 调试。语义不变：hash 同则 skip（mtime 变只 UPDATE），hash 变则删+插 | ✅ Verified（2026-09-05 `cargo test --lib keyword_index` 15/15，含 `upsert_chunks_preloads_hashes_and_updates_mtime_only`） |

---

## 验收

| # | 项 | 状态 |
|---|---|---|
| 1 | `cargo test --lib keyword_index` 全绿：拆词路由、短块合并、`content_hash` 增量、幽灵删除、grep 预筛 ≡ 全扫（≥100 组）、缺索引 `not_indexed`、W6 六例 | ✅ Verified（2026-09-05，13/13） |
| 2 | 受影响模块回归：keyword_index / entry_write / entry_admin / index_build / reindex / search_document / knowledge::mcp / workbench_read / mcp_host proxy | ✅ Verified（2026-09-05，61/61） |
| 3 | 前端：`tests/notes/search`、`tests/knowledge/knowledge-search{,-host}`、`tests/app-shell/index-rebuild`、`tests/host/searchApiInvokeMap`、gate `copy-switch-t1-shell` / `dual-search-markup`、路由与 fixture 清理 | ✅ Verified（2026-09-05，93/93；全量 vitest 1368/1369，唯一失败 `todo-task/ac-gate` 依赖不存在的兄弟 worktree 路径，与本次无关） |
| 4 | 实机：Meili 未安装、未运行时，`search_document` / `search_notes` / `search_knowledge` 正常返回；`grep` 结果与替代前一致 | ❌ Unresolved（需重启 App 后观察右上角 ↺ Index 转圈结束再手测） |
| 5 | 中文二字查询「角色」纯二字与混合形态均有结果且带片段 | ✅ Verified（单测）；实机 ❌ Unresolved |
| 6 | 全库重建 ≤ 10 s；记录 `keyword-index.sqlite` 体积 | ⚠️ Inferred（预估 50–100 MB）；待实测后回填 |
| 7 | `rg 'meili\|Meili' src-tauri frontend docs/architecture` 无残留 | ✅ Verified（2026-09-05） |
| 8 | `rg 'reindex_knowledge\|reindex_workbench\|get_reindex_workbench_status\|repo-commits\|reindexKnowledge\|reindexWorkbench'` 于 `src-tauri/src`、`permissions`、`frontend`、`tests`、README 无残留（`searchApiInvokeMap.test.js` 中断言"已移除"的三行除外） | ✅ Verified（2026-09-05） |
| 9 | `npx tsc --noEmit -p tsconfig.json` 通过 | ✅ Verified（2026-09-05） |

全量 `cargo test --lib -- --test-threads=1`：1282 通过、7 失败（`chat_stage_*` 3 条、`close_gate_*` / `health_*` / `p3_t10_*` 4 条），均为 `:8765` / `:9876` 被运行中的 App 进程占用所致，与本次改动无关。✅ Verified（2026-09-05 `lsof` 显示 pid 63864 `lulu-workbench` 监听两端口）

---

## 剩余事项

1. 重启 App（旧进程仍是 Meili 时代二进制）；启动后右上角「↺ Index」应自动转圈，结束后即可搜索。
2. 手测「角色」、「search 角色」与一条既有 `grep`，回填验收 4 / 5。
3. 记录重建耗时与 `keyword-index.sqlite` 体积，回填验收 6。
4. 以上完成后提交；提交前按 `docs/git/git-workflow-standard.md` 执行。
