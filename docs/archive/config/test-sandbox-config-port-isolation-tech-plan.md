# TestSandbox 配置与端口隔离技术方案

## 1. 目标

1. 废除同目录双文件名（`config.toml` / `dev.config.toml`），统一文件名 `config.toml`。
2. 用进程外开关 `TestSandbox`（及可选 `TestSandboxId`）选择**配置目录**，实现正式与沙箱隔离。
3. HTTP / MCP 端口写入 `config.toml`；Host 启动、Binding、**Tauri 前端**随当前实例配置自适应，避免与正式实例端口冲突。
4. 沙箱可启动完整 Workbench App，与正式实例并存（端口与数据路径不撞）。

决策来源：本会话两轮 `/converge`（缺口清单 + §8 开放问题），exit gate: **locked**。

---

## 2. 现状（须迁移走的行为）

| 项 | 现状 | 标记 |
|----|------|------|
| 正式配置 | `~/.config/lulu-workbench/config.toml` | ✅ Verified（`settings.rs`：`PROD_CONFIG_FILE_NAME`、`settings_config_dir`） |
| 测试/调试配置文件名 | `dev.config.toml`；由 `uses_dev_config()` 选择 | ✅ Verified（`settings.rs`：`uses_dev_config`） |
| `uses_dev_config` 判定 | `LULU_WB_CONFIG_DIR`（仅 `cfg(test)`）→ false；`TEST_MODE=1` → true；`cfg(test)` → true；否则 false | ✅ Verified（`settings.rs`） |
| 配置目录覆盖 | `LULU_WB_CONFIG_DIR` 指向绝对路径（仅 test 编译路径） | ✅ Verified（`settings.rs`：`isolated_config_dir`） |
| App 启动端口 | `setup` 使用 `DEFAULT_HTTP_PORT=8765`、`DEFAULT_MCP_PORT=9876` 常量 bind | ✅ Verified（`lib.rs` setup；`local_http/mod.rs`；`lib.rs`：`DEFAULT_MCP_PORT`） |
| `AppSettings` | 含三根路径、`meili_url`、LLM 等；**无** HTTP/MCP 端口字段 | ✅ Verified（`settings.rs`：`AppSettings`） |
| 前端非 Tauri 基址 | `DEFAULT_DEV_BASE = http://127.0.0.1:8765`（浏览器/`fetch` 模式） | ✅ Verified（`frontend/js/apiClient.js`；`api.js`） |
| debug 密钥文件 | 固定 `~/.config/lulu-workbench/dev-secrets.toml` | ✅ Verified（`secrets.rs`：`dev_secrets_path`） |
| release 密钥 | Keychain `service = "lulu-workbench"` | ✅ Verified（`secrets.rs`：`SERVICE`） |
| close-gate 正式双端口 | 观察 `:9876` + `:8765` 并做 initialize/tools/list | ✅ Verified（`mcp_protocol_adapter.rs`：`close_gate_smoke_initialize_list`） |
| `TEST_MODE` 三态 | prod / automated sandbox / manual cache-first（见 README） | ✅ Verified（`README.md`；`test_mode_kind`） |

---

## 3. 目标模型

### 3.1 配置目录（唯一分流维度）

统一文件名：`config.toml`。不再使用 `dev.config.toml`，删除 `uses_dev_config` 文件名分流。

| 条件 | 配置目录 |
|------|----------|
| `TestSandbox` 未设或为 false | `~/.config/lulu-workbench/` |
| `TestSandbox=true` 且无 `TestSandboxId` | `~/.config/lulu-workbench-sandbox/`（**共享沙箱目录**） |
| `TestSandbox=true` 且 `TestSandboxId=<id>` | `~/.config/lulu-workbench-sandbox-<id>/`（**实例目录**） |

`TestSandboxId`：仅 `[A-Za-z0-9_-]`，长度 1–64；非法 → **拒绝启动**（不净化）。

进程通过**环境变量**获知开关与编号；不能把「选哪个配置根」写进 `config.toml` 本身。

删除 `LULU_WB_CONFIG_DIR`（职责由 `TestSandbox` / `TestSandboxId` 替代）。

### 3.2 共享沙箱目录 vs 实例目录

`~/.config/lulu-workbench-sandbox/` 是**共享目录**（不只一个文件），默认准备好：

| 文件 | 作用 |
|------|------|
| `config.toml` | **共享模板**：全 id 可共用的项预填；实例专属项（端口、三根等）留空或占位 |
| `dev-secrets.toml` | 全沙箱 id **共享**密钥（debug） |

实例目录 `…-sandbox-<id>/`：主要由发起方放入从模板 **fork** 出的 `config.toml`，再填写不可共用字段。

**Fork 责任：** 谁发起测试谁 fork（AI / 脚本 / harness）。Host **不**自动 fork；实例 `config.toml` 缺失 → **启动失败**。

密钥始终读共享目录（与 id 无关）：`…/lulu-workbench-sandbox/dev-secrets.toml`；Keychain service：`lulu-workbench-sandbox`。

### 3.3 人启 App

人启动 App：**一律非 `TestSandbox`**（读正式配置根）。

**废除**原 `TEST_MODE=1` 人工 cache-first（读 overlay cache / 写语料仓）；人启只走正式 `config.toml` 路径语义。

备忘（本次不细化）：人侧后续再区分 `DEBUG` / `Release`（如日志），与 `TestSandbox` 正交。

### 3.4 `config.toml` 字段与端口缺省

正式与沙箱文件形态相同，至少包括：

- 既有：`workbench_knowledge_root`、`knowledge_corpus_root`、`cache_dir`、`meili_url`、LLM 等  
  ✅ Verified（现 `AppSettings`）
- 新增：`http_port`、`mcp_port`（`u16`）

缺省：

| 平面 | `http_port` | `mcp_port` |
|------|-------------|------------|
| 正式 | `8765` | `9876` |
| 沙箱（字段未写时） | `18765` | `19876` |

正式缺省数值沿用今日常量。✅ Verified（`DEFAULT_HTTP_PORT` / `DEFAULT_MCP_PORT`）。沙箱专用缺省为 converge 锁定，避免未改端口即撞正式。

### 3.5 密钥

| 平面 | debug 密钥文件 | Keychain service |
|------|----------------|------------------|
| 正式 | `~/.config/lulu-workbench/dev-secrets.toml` | `lulu-workbench` |
| 沙箱（所有 id 共用） | `~/.config/lulu-workbench-sandbox/dev-secrets.toml` | `lulu-workbench-sandbox` |

规则：

- `TestSandbox=true` 时**只**读写沙箱密钥面，禁止回读正式密钥文件 / 正式 Keychain service。
- 正式 → 沙箱共享密钥：**显式复制**，不自动；载体为仓库脚本 `scripts/copy-sandbox-secrets` + 文档（无 App UI）。
- `TestSandboxId` **不**拆分密钥。

### 3.6 运行时读端口（硬前提）

以下一律以**当前配置根**加载的 settings 为准；常量仅作字段缺省：

1. Sidecar HTTP bind（`http_port`）  
2. Embedded MCP bind（`mcp_port`）  
3. Binding 注入的 MCP URL  
4. Tauri 前端：由 Rust 将 settings 中的端口传给前端  

**推迟：** 非 Tauri / 浏览器 `fetch` 模式（`DEFAULT_DEV_BASE`）自适应；本方案主路径保证完整 Tauri App。  
✅ Verified（浏览器/`fetch` 路径存在于 `apiClient.js` / `api.js`；范围决策为 converge 推迟）。

### 3.7 Host 启动校验（硬守卫）

在 Workbench（Tauri Host）启动路径中，加载 settings 之后、bind / 写业务数据之前：

1. 读 `TestSandbox` / `TestSandboxId` → 解析配置目录。  
2. 加载该目录 `config.toml`（实例由发起方预先 fork 并填写）。  
3. 若 `TestSandbox=true`，与正式 `~/.config/lulu-workbench/config.toml` 对比，**任一命中则 fail-closed（拒绝启动）**：  
   - 三根任一路径等于或落在正式三根之下；  
   - `meili_url` 与正式相同；  
   - `http_port` / `mcp_port` 与正式**当前**端口相同；  
   - 实例必填字段仍为空/占位未填。  
4. 通过后：按 settings bind、读沙箱密钥面、向 Tauri 前端下发端口。

### 3.8 外部 Cursor IDE

- 日常：IDE 连接**正式** MCP 端口。  
- 若自动化要测 IDE 连沙箱：修改 MCP 配置指向沙箱端口即可。  
- Host **不**自动改写用户全局 `mcp.json`。

### 3.9 测试与 close-gate

- `cargo test` 与完整沙箱 App：**同一套**目录规则——设 `TestSandbox=true`（可加 `TestSandboxId`）；夹具写入对应沙箱 `config.toml`。  
- 绑定真实端口的测试：读 settings / 注入端口 / ephemeral；删除对正式默认端口的隐式硬编码依赖。  
- 仍验证「正式双端口 `:8765` + `:9876`」的 close-gate：**标明仅正式场景**。

---

## 4. 环境变量契约

| 变量 | 取值 | 作用 |
|------|------|------|
| `TestSandbox` | `true` / `false`（未设 ≡ false） | 是否使用沙箱配置体系 |
| `TestSandboxId` | 可选；`[A-Za-z0-9_-]{1,64}` | 实例配置目录后缀；**不影响**共享密钥目录 |

进程级环境：并行用例须**每进程**注入不同 `TestSandboxId`（及各自实例 `config.toml`）。

废弃：

- `LULU_WB_CONFIG_DIR`  
- 以 `TEST_MODE` 选择 `dev.config.toml` / 驱动 cache-first  
- `AI_TEST_MODE` 命名（不采用）

---

## 5. 启动与消费路径（逻辑）

```text
[发起方] fork 共享模板 → …-sandbox-<id>/config.toml 并填写实例字段
[发起方] 可选：scripts/copy-sandbox-secrets
[发起方] env: TestSandbox=true (+ TestSandboxId)
        │
        ▼
 Host: settings_config_dir()
        │
        ▼
 load config.toml → AppSettings（http_port / mcp_port / 三根 / meili_url…）
        │
        ├─ 硬守卫（撞正式三根 / meili / 端口，或必填空）→ 失败退出
        ├─ secrets → 共享沙箱密钥面
        ├─ bind HTTP / MCP ← settings
        ├─ Binding MCP URL ← mcp_port
        └─ Tauri 前端 ← Rust 下发端口
```

---

## 6. 迁移要点

| 步骤 | 内容 |
|------|------|
| M1 | `AppSettings` 增加 `http_port` / `mcp_port`；正式缺省 `8765`/`9876`，沙箱缺省 `18765`/`19876` |
| M2 | `settings_config_dir` 按 `TestSandbox`/`TestSandboxId` 解析（含 id 校验）；删除 `uses_dev_config` / `dev.config.toml` |
| M3 | 删除 `LULU_WB_CONFIG_DIR`；单测/夹具改为 `TestSandbox`(+Id) + 写沙箱 `config.toml` |
| M4 | `lib.rs` setup、MCP registry/Binding、Tauri 前端端口传递改为读 settings |
| M5 | `secrets` 沙箱路径与 Keychain service；新增 `scripts/copy-sandbox-secrets` + 文档 |
| M6 | 启动硬守卫（三根 / meili_url / 端口 / 必填）+ 单测 |
| M7 | close-gate / 硬编码端口测试分类改造 |
| M8 | README / `docs/knowledge-mcp.md` / `TEST_MODE` 相关契约测试改写 |
| M9 | 废除 `TEST_MODE` 三态与 cache-first；删除或改写 `test_mode_kind` 等 |
| M10 | 提供/文档化共享沙箱目录初始模板（`config.toml` 占位 + 可选空 `dev-secrets.toml`） |

---

## 7. 非目标（本方案不包含）

- 人侧 `DEBUG` / `Release` 日志等行为细化（仅备忘）。  
- Host 自动修改 Cursor 全局 `mcp.json`。  
- Host 自动 fork 实例 `config.toml`。  
- 按 `TestSandboxId` 拆分密钥。  
- 在 `config.toml` 内配置「配置目录自身路径」。  
- 非 Tauri / 浏览器 `fetch` 基址自适应（**推迟**）。

---

## 8. §8 开放问题决议摘要

| 原 # | 决议 |
|------|------|
| O1 | 字段名 `http_port` + `mcp_port` |
| O2 | id：`[A-Za-z0-9_-]` 1–64；非法拒绝 |
| O3 | 沙箱缺省 `18765` / `19876` |
| O4 | 对比正式 config：三根 / `meili_url` / 端口撞则拒；必填空则拒；发起方 fork |
| O5 | 废除人工 cache-first |
| O6 | `scripts/copy-sandbox-secrets` + 文档 |
| O7 | 浏览器 fetch **推迟** |
| O8 | `cargo test` 与 App 同一套 `TestSandbox`(+Id) |
| O9 | 密钥固定共享目录 `…/lulu-workbench-sandbox/`，与 id 无关 |

开放问题表已清空；无未决项挡实施设计。

---

## 9. 成功标准

1. 正式未设 `TestSandbox`：行为与今日正式一致（默认端口与正式配置根），且仅使用 `config.toml`。  
2. `TestSandbox=true` + 实例配置使用非冲突端口（含沙箱缺省）：可与正式 App 同时运行。  
3. 沙箱不读正式密钥面；共享密钥可经脚本显式从正式复制。  
4. 沙箱配置撞正式三根 / Meili / 端口，或实例必填未填：启动失败。  
5. Tauri 前端与 Binding 使用的端口与当前实例 settings 一致。  
6. 发起方负责 fork；Host 不自动创建实例配置。

---

## 10. 文档状态

- 路径：`docs/archive/config/test-sandbox-config-port-isolation-tech-plan.md`  
- 本文件：技术方案（设计决策已锁 + 迁移要点）；§8 已决议。  
- **实施状态（2026-08-09）：** Host `settings` / `TestSandbox` 配置根、端口字段、启动 bind、沙箱守卫、密钥面、`TestSandbox` 夹具、`scripts/copy-sandbox-secrets`、README 已落地；`cargo test --lib` 全绿。浏览器 `fetch` 基址自适应仍按 §7 推迟。  
- 说明：`scripts/migrate-local-state` 等 Python 一次性脚本仍可用自有的 `LULU_WB_CONFIG_DIR` 指向配置目录（与 Host Rust 无关）。
