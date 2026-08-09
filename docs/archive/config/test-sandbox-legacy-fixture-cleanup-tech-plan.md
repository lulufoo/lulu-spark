# TestSandbox 历史夹具收敛技术方案

> 源码依据：<https://github.com/lulufoo/lulu-workbench/tree/main/src-tauri/src/test_support.rs>

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**目标：** 删除已由 `TestSandbox` 覆盖的历史测试隔离兼容层，保留唯一的环境生命周期和现有测试语义。

**架构：** ✅ Verified（`src-tauri/src/test_support.rs`）：`TestSandbox::new()` 已创建隔离的 `workbench-knowledge`、`corpus`、`cache` 并写入沙箱 `config.toml`。本方案删除重复创建相同环境的 wrapper，KB / KB Git 夹具改用已配置的 `knowledge_corpus_root()`，其余只删除不再保护资源的测试锁与空 wrapper。

**技术栈：** Rust、`cargo test`、`cargo nextest`。

**范围：** 仅调整 `src-tauri` 测试夹具、测试代码和测试纪律文档；不改生产运行时配置逻辑，不改 `TestConfigEnv`、`with_config_test_serial` 或 `with_sandbox_corpus`。

## 全局约束

- ✅ Verified（`src-tauri/src/test_support.rs`）：所有测试环境变量生命周期仍只由 `TestConfigEnv` / `TestSandbox` 管理。
- ✅ Verified（`src-tauri/src/config/settings.rs`）：保留 `write_test_config_with_cache`、机器正式配置路径保护和原子写入；它们仍是默认沙箱初始化与未来自定义测试配置的安全写入通道。
- ✅ Verified（`src-tauri/src/test_support.rs`）：保留 `with_config_test_serial`，因为机器配置摘要的“前—中—后”断言需要一个比 `TestSandbox` 生命周期更外层的串行区间。
- ✅ Verified（`src-tauri/src/test_support.rs`）：保留 `with_sandbox_corpus`；它为注释、标签和工作台读取测试准备 `annotations` / `annotations/ai` 数据布局。
- ⚠️ Inferred（实施约束）：不创建 commit；用户尚未请求提交。

---

### Task 1: 删除共享夹具兼容 wrapper

**文件：**
- 修改：`src-tauri/src/test_support.rs`
- 修改：`src-tauri/src/unit-tests/test_support.rs`
- 修改：`docs/testing-discipline.md`

**删除对象：**

```rust
pub fn with_sandbox<F: FnOnce(&Path)>(f: F) {
    let sandbox = TestSandbox::new();
    f(sandbox.config_dir());
}

pub fn with_test_config_dir<F: FnOnce(&std::path::Path)>(f: F) {
    let sandbox = TestSandbox::new();
    f(sandbox.config_dir());
}
```

✅ Verified（`src-tauri/src/test_support.rs`）：两个 wrapper 都只委托 `TestSandbox::new()`；共享 `with_sandbox` 无调用点，`with_test_config_dir` 仅由其自身元测试调用。

**步骤：**

- [ ] 删除共享 `with_sandbox` 与 `with_test_config_dir`。
- [ ] 删除 `with_test_config_dir_uses_isolated_config_toml` 元测试及其 import；`test_sandbox_new_creates_isolated_three_roots` 保留为等价默认沙箱覆盖。
- [ ] 删除 `with_corpus`，并删除只验证该 helper 的四个元测试：

```text
with_corpus_restores_environment_when_callback_panics
with_corpus_sets_sandbox_cache_dir_not_prod
with_corpus_false_creates_annotations_only
with_corpus_true_creates_ai_subdir
```

- [ ] 在 `docs/testing-discipline.md` 的夹具建议中移除 `with_test_config_dir` 与 `with_corpus`，保留 `TestSandbox` 与 `with_sandbox_corpus`。
- [ ] 清理删除 helper 后不再使用的 `PathBuf` 和 helper import。
- [ ] 运行：

```bash
cd src-tauri
cargo test --lib test_support -- --test-threads=1
```

预期：通过，且不再有对已删除共享 wrapper 的引用。

---

### Task 2: KB / KB Git 使用默认沙箱知识库根

**文件：**
- 修改：`src-tauri/src/unit-tests/services/kb.rs`
- 修改：`src-tauri/src/unit-tests/services/kb_git.rs`

✅ Verified（`src-tauri/src/config/meili_env.rs`）：KB 读取根目录来自 active settings 的 `knowledge_corpus_root`，不从传入的 `repo_root` 参数推导。

**替换形态：**

```rust
let sandbox = TestSandbox::new();
let cfg_dir = sandbox.config_dir();
let kb = sandbox.knowledge_corpus_root();

setup(cfg_dir, &kb);
f(cfg_dir, &kb);
```

**步骤：**

- [ ] 在 `with_kb_repo`、`with_kb_and_sediment_cache` 和 `with_kb_git` 中，将：

```rust
let kb = cfg_dir.join("kb");
settings::write_test_config_with_cache(/* ... */);
```

替换为上述默认根形态。
- [ ] 删除两个文件中只为重写配置保留的 `use crate::config::settings;` import。
- [ ] 保留 fixture callback、Git 初始化和 `set_test_repo_validator` 的显式 reset；这些是业务测试数据，不是历史隔离层。
- [ ] 运行：

```bash
cd src-tauri
cargo test --lib services::kb::tests -- --test-threads=1
cargo test --lib services::kb_git::tests -- --test-threads=1
cargo test --lib kb_doc_count_json_category_aggregates -- --test-threads=1
```

预期：通过；KB 文件和 Git 仓库均位于 `sandbox.knowledge_corpus_root()`。

---

### Task 3: 删除重复配置回写、测试 mutex 与空 wrapper

**文件：**
- 修改：`src-tauri/src/unit-tests/services/settle.rs`
- 修改：`src-tauri/src/unit-tests/services/read_later.rs`
- 修改：`src-tauri/src/unit-tests/commands/read_later.rs`
- 修改：`src-tauri/src/unit-tests/services/entry_admin.rs`
- 修改：`src-tauri/src/unit-tests/services/local_http.rs`
- 修改：`src-tauri/src/unit-tests/services/archive_write.rs`

**步骤：**

- [ ] 在 `settle.rs::with_repo_list` 删除 identity re-save：

```rust
let mut cfg = settings::load().expect("load");
cfg.cache_dir = cache;
settings::save(&cfg).expect("save");
```

`cache` 已由 `sandbox.cache_dir()` 取得，默认沙箱配置已经指向同一路径。

- [ ] 删除 `READ_LATER_TEST_LOCK`、`READ_LATER_CMD_TEST_LOCK`、`ENTRY_ADMIN_TEST_LOCK` 及其 guard。保留每个测试已有的 `TestSandbox::new()`；它持有统一 `CONFIG_TEST_SERIAL`。
- [ ] 删除仅调用 `f()` 的 `with_todo_task_http_test`、`with_archive_plan_task_test` 与 `setup_corpus_with_plan_tasks`，将其调用点直接改为原 closure body 或 `setup_corpus()`。
- [ ] 清理删除 mutex、配置回写和空 wrapper 后不再使用的 `Mutex`、`OnceLock`、`settings` 及 helper import。
- [ ] 运行：

```bash
cd src-tauri
cargo test --lib services::read_later -- --test-threads=1
cargo test --lib commands::read_later -- --test-threads=1
cargo test --lib delete_entry_removes_file_and_index -- --test-threads=1
cargo test --lib move_entry_project_moves_zh_translation -- --test-threads=1
cargo test --lib services::local_http -- --test-threads=1
cargo test --lib services::archive_write -- --test-threads=1
```

预期：通过；测试不再声明旁路环境 mutex 或空 wrapper。

---

### Task 4: 静态约束与全量验收

**文件：**
- 验证：`src-tauri/src/test_support.rs`
- 验证：`src-tauri/.config/nextest.toml`

**步骤：**

- [ ] 验证删除的 shared helper、旧环境开关和旁路 mutex 不再存在：

```bash
rg -n 'pub\s+fn\s+with_sandbox\b|pub\s+fn\s+with_test_config_dir\b|pub\s+fn\s+with_corpus\b|READ_LATER_(CMD_)?TEST_LOCK|ENTRY_ADMIN_TEST_LOCK|TEST_MODE|dev\.config\.toml|LULU_WB_CONFIG_DIR' src-tauri/src --glob '*.rs'
```

预期：无匹配。

- [ ] 运行单进程串行全量测试：

```bash
cd src-tauri
cargo test --lib -- --test-threads=1
```

预期：通过。

- [ ] 运行多进程全量测试：

```bash
cd src-tauri
cargo nextest run --lib
```

预期：通过；`mcp-formal-ports` 继续单并发运行固定正式端口 close-gate。

- [ ] 运行：

```bash
git diff --check
```

预期：无输出。

## 完成条件

- ⚠️ Inferred（验收）：默认 `TestSandbox` 足以为不需自定义根目录的测试提供隔离配置。
- ⚠️ Inferred（验收）：KB / KB Git fixture 不再重写沙箱 `config.toml`，但其业务测试结果保持不变。
- ⚠️ Inferred（验收）：不存在通过旁路测试 mutex 或已删除 wrapper 实现的环境隔离。
- ⚠️ Inferred（验收）：单进程和 nextest 多进程全量测试均通过。
