# TestSandbox 环境竞态修复 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> ✅ Verified（项目源码）：[`settings.rs`](https://github.com/lulufoo/lulu-workbench/blob/main/src-tauri/src/config/settings.rs) 与 [`test_support.rs`](https://github.com/lulufoo/lulu-workbench/blob/main/src-tauri/src/test_support.rs)。
> ✅ Verified（Rust Reference）：[`Drop` 先执行，随后 struct 字段按声明顺序析构](https://doc.rust-lang.org/reference/destructors.html)。

**Goal:** 建立唯一的测试环境夹具，杜绝测试将临时配置写入用户正式 `config.toml`。

**Architecture:** ⚠️ Inferred（设计）：同一 Rust 测试进程内，所有 `HOME` / `TestSandbox` / `TestSandboxId` 切换由一个 RAII scope 串行管理；scope 必须在恢复环境后才释放锁。`nextest` 保留多进程并行；普通 `cargo test` 的环境型用例不承诺线程并行。

**Tech Stack:** ✅ Verified（项目源码）：Rust `std::env`、`Mutex`、`tempfile`、Tauri Host unit tests、`cargo nextest`。

## Global Constraints

- ⚠️ Inferred（设计）：正式 App 的配置选择契约不变；本方案只修改 `#[cfg(test)]` 路径、测试夹具与测试纪律文档。
- ⚠️ Inferred（设计）：不恢复 `LULU_WB_CONFIG_DIR`、`dev.config.toml` 或 `TEST_MODE`。
- ⚠️ Inferred（设计）：测试进程不能写入启动测试前的正式 `~/.config/lulu-workbench/config.toml`；命中时必须失败，而不是静默改写。
- ✅ Verified（`docs/testing-discipline.md`）：全量 Rust 测试的标准命令是 `cargo nextest run --lib`。

---

## 1. 问题与根因

### 1.1 已验证行为

- ✅ Verified（`src-tauri/src/config/settings.rs`）：`config_file_path()` 在 `TestSandbox` 未设或非 `true`/`1` 时返回正式配置路径。
- ✅ Verified（`src-tauri/src/config/settings.rs`）：`write_test_config_with_cache()` 同时依赖环境解析写入路径与端口；未处于沙箱时会写正式默认端口 `8765` / `9876`。
- ✅ Verified（`src-tauri/src/test_support.rs`）：`TestSandbox::Drop` 先取走并释放 `_config_lock`；`EnvRestore` 是独立字段，随后才析构并恢复环境变量。
- ✅ Verified（`src-tauri/src/unit-tests/config/settings.rs`、`src-tauri/src/unit-tests/config/paths.rs`）：还有多个测试守卫直接调用 `std::env::set_var` / `remove_var`，不统一使用 `TestSandbox` 的生命周期锁。

### 1.2 竞态路径

⚠️ Inferred（由上述源码与本次受损配置形态推导）：

```text
测试 A：持锁 → 设置沙箱环境 → 用例结束 → 提前释放锁
测试 B：取得锁 → 设置自己的沙箱环境 → 尚在执行
测试 A：字段析构 → 恢复真实 HOME、清除 TestSandbox
测试 B：写测试 config → 环境被判为正式面 → 写入正式 config.toml
```

### 1.3 范围与副作用

- ✅ Verified（`src-tauri/src/config/settings.rs`）：风险包括 `settings::save()` 与测试配置写入器；两者都通过环境解析目标路径。
- ⚠️ Inferred（设计）：实施不迁移用户数据、不修改 Corpus、不改正式 App 的端口或配置字段。
- ⚠️ Inferred（设计）：实施前后都不得把用户正式配置作为自动化测试的断言夹具或临时目标。

---

## 2. 锁定决策

1. ⚠️ Inferred（设计）：支持**多进程并行**，不支持同一测试进程内的环境型测试并行。
2. ⚠️ Inferred（设计）：`TestConfigEnv` 是唯一允许设置或恢复 `HOME`、`TestSandbox`、`TestSandboxId` 的测试 API。
3. ⚠️ Inferred（设计）：所有测试配置写入接受显式目标路径；测试写入器不得再从当前环境推导目标。
4. ⚠️ Inferred（设计）：`settings::save()` 在 test build 中增加正式路径拒绝层，作为夹具失效时的最终保险。
5. ⚠️ Inferred（设计）：除 `src-tauri/src/test_support.rs` 外，测试源代码不得保留对上述三个环境变量的直接写操作。
6. ⚠️ Inferred（设计）：任何测试写目标都在创建目录前解析为规范路径；`..` 与符号链接指向机器正式配置时必须拒绝。
7. ⚠️ Inferred（设计）：测试 build 的配置写入使用“验证后的同目录临时文件 + rename”替代直接截断写入，避免 hard link 别名修改机器正式配置 inode。

---

## 3. 文件结构

⚠️ Inferred（设计）：

| 文件 | 改动责任 |
|---|---|
| `src-tauri/src/test_support.rs` | 新建统一 `TestConfigEnv` RAII scope；`TestSandbox` / `with_corpus` 复用它。 |
| `src-tauri/src/config/settings.rs` | 测试配置写入改为显式路径；test build 拒绝用户正式配置写入。 |
| `src-tauri/src/unit-tests/test_support.rs` | 覆盖环境恢复顺序、显式配置目标与正式配置保护。 |
| `src-tauri/src/unit-tests/config/settings.rs` | 删除 `IsolatedConfigGuard` / `TestSandboxEnvGuard`，改用统一 scope。 |
| `src-tauri/src/unit-tests/config/paths.rs` | 删除手写环境保存/恢复，改用统一 scope。 |
| `src-tauri/src/unit-tests/services/kb.rs` | 为测试配置写入传入 `TestSandbox` 的显式配置路径。 |
| `src-tauri/src/unit-tests/services/kb_git.rs` | 为测试配置写入传入 `TestSandbox` 的显式配置路径。 |
| `docs/testing-discipline.md` | 明确禁止直改环境变量；规定 `TestConfigEnv` 与 nextest 的并行边界。 |

---

## 4. 实施任务

### Task 1: 建立唯一的 `TestConfigEnv` 生命周期

**Files:**
- Modify: `src-tauri/src/test_support.rs`
- Test: `src-tauri/src/unit-tests/test_support.rs`

**Interfaces:**

```rust
pub enum TestConfigPlane {
    Prod,
    Sandbox { id: String },
}

pub struct TestConfigEnv {
    home: PathBuf,
    plane: TestConfigPlane,
    env: Option<EnvRestore>,
    lock: Option<MutexGuard<'static, ()>>,
}

impl TestConfigEnv {
    pub fn prod(home: &Path) -> Self;
    pub fn sandbox(home: &Path, id: &str) -> Self;
    pub fn config_file_path(&self) -> PathBuf;
    pub fn ports(&self) -> (u16, u16);
}

impl TestSandbox {
    pub fn config_file_path(&self) -> PathBuf;
    pub fn ports(&self) -> (u16, u16);
}
```

- [ ] **Step 1: 写失败测试，锁定 scope 销毁后环境恢复。**

```rust
#[test]
fn test_config_env_drop_restores_original_environment() {
    let original_home = std::env::var("HOME").ok();
    let original_sandbox = std::env::var("TestSandbox").ok();
    let original_sandbox_id = std::env::var("TestSandboxId").ok();
    let dir = tempfile::tempdir().expect("tmp");
    {
        let _env = TestConfigEnv::sandbox(dir.path(), "restore_order");
        assert_eq!(std::env::var("HOME").ok().as_deref(), dir.path().to_str());
        assert_eq!(std::env::var("TestSandbox").as_deref(), Ok("true"));
    }
    assert_eq!(std::env::var("HOME").ok(), original_home);
    assert_eq!(std::env::var("TestSandbox").ok(), original_sandbox);
    assert_eq!(std::env::var("TestSandboxId").ok(), original_sandbox_id);
}
```

- [ ] **Step 2: 实现 `EnvRestore::capture` 与 plane 应用。**

```rust
fn capture() -> EnvRestore;
fn apply_prod(home: &Path);
fn apply_sandbox(home: &Path, id: &str);
```

`capture()` 只保存旧值；`apply_*` 才设置新值。这样 `TestConfigEnv` 可以持有 `Option<EnvRestore>`，并在 `Drop` 中显式调用 `take()`。

- [ ] **Step 3: 实现锁覆盖完整环境生命周期。**

```rust
impl Drop for TestConfigEnv {
    fn drop(&mut self) {
        drop(self.env.take()); // 必须先恢复原环境
        CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        drop(self.lock.take()); // 必须最后释放
    }
}
```

- [ ] **Step 4: 加入可重复的析构顺序回归测试。**

在 `EnvRestore::drop()` 增加仅 test build 生效的 probe；probe 执行 `CONFIG_TEST_SERIAL.try_lock()`。测试销毁一个外层 `TestConfigEnv` 时，probe 必须得到 `TryLockError::WouldBlock`，证明环境恢复发生时锁仍被该 scope 持有：

```rust
#[test]
fn restore_runs_while_config_lock_is_still_held() {
    let _probe = install_env_restore_probe(|| {
        assert!(matches!(
            CONFIG_TEST_SERIAL.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        ));
    });
    let dir = tempfile::tempdir().expect("tmp");
    drop(TestConfigEnv::sandbox(dir.path(), "drop_order"));
}
```

再加入两线程 + `Barrier` 测试：线程 B 在 A 的环境恢复 probe 触发后尝试创建 `TestConfigEnv::sandbox`；B 必须等 A 恢复环境并释放锁后才返回，并且 B 捕获到的前置 `HOME` 必须等于机器 `HOME`，而不是 A 的临时目录。

- [ ] **Step 5: 将 `TestSandbox::new` 与 `with_corpus` 改为组合 `TestConfigEnv`。**

`TestSandbox` 保留既有 `config_dir()`、三根路径 getter 与 `assert_not_prod_path()` API；`with_corpus` 保留现有回调签名。删除手写 `drop(env)` / `drop(_lock)`，让 scope 在 unwind 时自动恢复环境。

- [ ] **Step 6: 运行单元测试。**

Run: `cd src-tauri && cargo test --lib test_support -- --test-threads=1`  
Expected: PASS，且所有现有 `TestSandbox` / `with_corpus` 夹具测试仍通过。

### Task 2: 让测试配置写入显式定址，并加正式路径保险

**Files:**
- Modify: `src-tauri/src/config/settings.rs`
- Modify: `src-tauri/src/test_support.rs`
- Modify: `src-tauri/src/unit-tests/test_support.rs`
- Modify: `src-tauri/src/unit-tests/services/kb.rs`
- Modify: `src-tauri/src/unit-tests/services/kb_git.rs`

**Interfaces:**

```rust
#[cfg(test)]
pub fn write_test_config_with_cache(
    config_path: &Path,
    workbench_knowledge_root: &Path,
    knowledge_corpus_root: Option<&Path>,
    cache_dir: Option<&Path>,
    meili_url: &str,
    http_port: u16,
    mcp_port: u16,
) -> Result<(), SettingsError>;

#[cfg(test)]
pub(crate) fn register_test_machine_config_path(path: PathBuf);

#[cfg(test)]
pub(crate) fn reject_test_write_to_machine_config(path: &Path) -> Result<(), SettingsError>;

#[cfg(test)]
pub(crate) fn reject_write_to_protected_config(
    path: &Path,
    protected_config_path: &Path,
) -> Result<(), SettingsError>;

#[cfg(test)]
pub(crate) fn atomic_write_test_config(
    verified_target_path: &Path,
    content: &str,
) -> Result<(), SettingsError>;
```

- [ ] **Step 1: 写失败测试，测试写入器不能依赖环境解析目标。**

```rust
#[test]
fn test_config_writer_uses_explicit_target_path() {
    let sandbox = TestSandbox::new();
    let expected = sandbox.config_file_path();
    settings::write_test_config_with_cache(
        &expected,
        sandbox.config_dir(),
        Some(sandbox.config_dir()),
        Some(&sandbox.config_dir().join("cache")),
        "http://127.0.0.1:17700",
        18_765,
        19_876,
    )
    .expect("write explicit sandbox config");
    assert!(expected.is_file());
}
```

- [ ] **Step 2: 将 `write_test_config_with_cache` 改为显式 `config_path`、`meili_url` 与端口参数。**

该函数不得调用 `config_file_path()`、`settings_config_dir()` 或 `is_test_sandbox()` 来决定写入目标或端口；调用者必须传入 `meili_url`、`http_port` / `mcp_port`，并由 `TestConfigEnv` 的 plane 选择正式或沙箱缺省值。

- [ ] **Step 3: 在 `TestConfigEnv` 建立前登记机器正式配置路径。**

外层 scope 取得 `CONFIG_TEST_SERIAL` 后、改写 `HOME` 前调用：

```rust
settings::register_test_machine_config_path(settings::prod_config_file_path());
```

使用 `OnceLock<PathBuf>` 保存每个测试进程启动时的正式配置文件路径；重复登记必须要求路径相同。写保护尚未登记时必须 fail-closed，禁止任何 `save()` 或测试配置写入。

- [ ] **Step 4: 在测试 build 的写路径上拒绝机器正式配置。**

```rust
#[cfg(test)]
reject_test_write_to_machine_config(&path)?;
```

在 `save()` 写入前和 `write_test_config_with_cache()` 写入前调用。解析规则为：先规范化已存在目标；不存在目标则规范化已存在父目录后再拼接文件名；任何解析错误、`..` 别名或符号链接最终指向机器正式配置时都返回 `SettingsError::ConfigGuard`，并且发生在 `create_dir_all` 前。

通过路径验证后，`save()` 与 `write_test_config_with_cache()` 都调用 `atomic_write_test_config()`：在已验证的目标父目录中创建唯一临时文件，写入完整 TOML，然后 `rename` 覆盖目标。不得再用 `fs::write(target, ...)` 直接截断目标；这样现有 hard link 只会被替换为新目录项，不会改写机器正式配置所指 inode。

- [ ] **Step 5: 迁移四个显式写入调用点。**

`TestSandbox::new`、`kb.rs` 与 `kb_git.rs` 改传 `sandbox.config_file_path()` 或当前 scope 的显式路径。`write_fake_prod_config` 也必须改用该写入器，并显式传入临时正式面路径、`http://127.0.0.1:7700`、`8765`、`9876`；它不得保留直接 `fs::write(config.toml)`。

- [ ] **Step 6: 写入 `..` 与符号链接逃逸测试。**

```rust
let protected = temp_machine_home
    .path()
    .join(".config/lulu-workbench/config.toml");
std::fs::create_dir_all(protected.parent().unwrap()).expect("create machine config parent");
std::fs::write(&protected, "cache_dir = \"safe\"").expect("create machine config");
let alias_parent = protected.parent().unwrap().join("alias");
std::fs::create_dir_all(&alias_parent).expect("create alias parent");
let dot_dot_alias = alias_parent
    .join("..")
    .join(protected.file_name().unwrap());
assert!(reject_write_to_protected_config(&dot_dot_alias, &protected).is_err());
std::os::unix::fs::symlink(&protected, temp_dir.join("config.toml"))
    .expect("create symlink");
assert!(reject_write_to_protected_config(
    &temp_dir.join("config.toml"),
    &protected
).is_err());
```

在 macOS/Linux 执行该用例；Windows 编译目标使用条件编译跳过符号链接分支。

- [ ] **Step 7: 写入 hard link 回归测试。**

```rust
let protected_before = std::fs::read_to_string(&protected).expect("read protected");
let hard_link_alias = protected
    .parent()
    .unwrap()
    .join("hard-link-config.toml");
std::fs::hard_link(&protected, &hard_link_alias).expect("create hard link");
atomic_write_test_config(&hard_link_alias, "cache_dir = \"replacement\"")
    .expect("atomic replacement");
assert_eq!(
    std::fs::read_to_string(&protected).expect("read protected"),
    protected_before
);
assert_ne!(
    std::fs::read_to_string(&hard_link_alias).expect("read replacement"),
    protected_before
);
```

- [ ] **Step 8: 运行定向测试。**

Run: `cd src-tauri && cargo test --lib "test_config_writer\|kb_" -- --test-threads=1`  
Expected: PASS；针对机器正式配置路径的保护测试返回拒绝，且不改变该文件内容。

### Task 3: 删除旁路环境修改并更新测试纪律

**Files:**
- Modify: `src-tauri/src/unit-tests/config/settings.rs`
- Modify: `src-tauri/src/unit-tests/config/paths.rs`
- Modify: `docs/testing-discipline.md`

- [ ] **Step 1: 用 `TestConfigEnv::prod` 替换 `IsolatedConfigGuard`。**

```rust
let dir = tempfile::tempdir().expect("tmp");
let _env = TestConfigEnv::prod(dir.path());
```

删除 `IsolatedConfigGuard` 的定义与实现。涉及 secret 测试的清理维持为独立的 `test_secrets_clear()` 调用，不再承担环境恢复职责。

- [ ] **Step 2: 用 `TestConfigEnv::sandbox` 替换 `TestSandboxEnvGuard`。**

```rust
let dir = tempfile::tempdir().expect("tmp");
let _env = TestConfigEnv::sandbox(dir.path(), "id1");
assert!(is_test_sandbox());
```

对“未设沙箱”断言使用 `TestConfigEnv::prod(dir.path())`，不直接 `remove_var`。

- [ ] **Step 3: 用统一 scope 替换 `paths.rs` 的手写保存/恢复。**

```rust
let dir = tempfile::tempdir().expect("tmp");
let _env = TestConfigEnv::sandbox(dir.path(), "badcfg");
let cfg_dir = _env.config_file_path().parent().unwrap().to_path_buf();
```

保留损坏 TOML 的路径 helper 失败断言；删除 `prev_home`、`prev_sb`、`prev_id` 与末尾手写恢复块。

- [ ] **Step 4: 更新 `docs/testing-discipline.md`。**

将“不得添加 ENV_LOCK / test mutexes”与“`settings` tests may use `EnvGuard`”改为：“不得创建旁路锁、`EnvGuard` 或直接改环境变量；环境型测试必须使用 `TestConfigEnv`。nextest 提供多进程并行，单一进程内由 `TestConfigEnv` 串行化环境生命周期。”

- [ ] **Step 5: 运行静态约束与 Rust 定向测试。**

Run: `rg -n 'std::env::(set_var|remove_var)' src-tauri/src --glob '*.rs'`  
Expected: 仅 `src-tauri/src/test_support.rs` 的 `TestConfigEnv` / `EnvRestore` 实现命中。

Run: `cd src-tauri && cargo test --lib "config::settings\|config::paths" -- --test-threads=1`  
Expected: PASS。

### Task 4: 回归验证与文档状态

**Files:**
- Modify: `src-tauri/src/unit-tests/test_support.rs`
- Modify: `docs/archive/config/test-sandbox-config-port-isolation-tech-plan.md`

- [ ] **Step 1: 加入正式配置不变回归测试。**

测试在 scope 建立前记录已登记机器正式配置文件的内容摘要；构造并销毁 sandbox、调用测试配置写入器、再比较摘要。该测试只读取机器正式配置，不写入它。

- [ ] **Step 2: 加入嵌套 scope 回归测试。**

```rust
let outer = TestConfigEnv::sandbox(outer_dir.path(), "outer");
{
    let inner = TestConfigEnv::sandbox(inner_dir.path(), "inner");
    assert_eq!(
        settings::config_file_path().expect("inner config"),
        inner.config_file_path()
    );
}
assert_eq!(
    settings::config_file_path().expect("outer config"),
    outer.config_file_path()
);
drop(outer);
```

断言内层析构恢复外层环境，外层析构恢复机器环境。

- [ ] **Step 3: 运行标准验证。**

Run: `cd src-tauri && cargo test --lib -- --test-threads=1`  
Expected: PASS。

Run: `cd src-tauri && cargo nextest run --lib`  
Expected: PASS；测试以多进程方式并行，不触碰机器正式配置。固定正式端口 `8765` / `9876` 的 MCP close-gate 用例使用 `mcp-formal-ports` test group（`max-threads = 1`）顺序执行，避免彼此抢占端口；执行前这两个端口仍必须空闲。

- [ ] **Step 4: 更新原端口隔离方案状态。**

在 `docs/archive/config/test-sandbox-config-port-isolation-tech-plan.md` 的实施状态加入本修复计划链接，并删除“`cargo test --lib` 全绿”这类未限定并发模型的表述。

---

## 5. 完成条件

- ✅ Verified（命令输出，2026-08-09）：`cargo test --lib -- --test-threads=1` 与 `cargo nextest run --lib` 均为 `1113 passed; 0 failed`；`mcp-formal-ports` 将固定正式端口的 close-gate 用例限制为单并发。
- ⚠️ Inferred（验收）：普通 `cargo test --lib` 的环境型测试不会让任何写入目标落到机器正式配置路径。
- ⚠️ Inferred（验收）：`cargo test --lib -- --test-threads=1` 与 `cargo nextest run --lib` 均通过。
- ⚠️ Inferred（验收）：所有三项环境变量的写入、恢复和锁释放只在 `TestConfigEnv` 生命周期中发生。
- ⚠️ Inferred（验收）：正式 App 未设 `TestSandbox` 时，仍只读取用户正式 `config.toml`；测试夹具不会改写该文件。
