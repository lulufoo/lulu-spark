# dialogue-archive 脚本化与 archive_document 路径化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 dialogue 原文归档改为「AI 只定位节点 + 短参数；Python 机械生成 raw；`archive_document` 只传 `source_path`；`archive_digest` 仍由 AI 在内容约束下写摘要」，避免整份对话正文进出 LLM tool 参数。

**Architecture:** Cursor agent transcript 为 JSONL（一行一个 JSON 节点）。脚本按闭区间节点号切片并写成符合 archive 解析器的 Markdown。MCP Adapter 仍只代理 Sidecar HTTP；Host 读取 allow-list 内源文件后走现有 `parse_archive_document` + raw/index 写入。Digest 不经脚本理解，由 AI 基于对话上下文 + 必填 `content_constraint` 生成后调用 `archive_digest`。

**Tech Stack:** Python 3 标准库脚本、Tauri Host（`archive_write` / `local_http`）、`packages/knowledge-mcp`、dialogue-archive SKILL、既有 archive 单测 / verify。

## Global Constraints

- Protocol Adapter 只代理 Sidecar HTTP，不得直读 Corpus FS（`docs/architecture/agent-orientation.md` §5）。
- **`archive_document` 硬切：删除 `document` 正文入参**；只接受绝对路径 `source_path`（及既有可选字段：`source_type` / `translations` / task_ref 等若仍存在则保留，但正文来源改为读文件）。
- 请求体若仍带 `document` → `400`：`document is not supported; use source_path`。
- raw Markdown 形状仍须通过 `parse_archive_document`（header / `---` / 导航等，见 `archive_parse`）。
- 脚本不做语义摘要、不做模糊文本搜索；文本锚点「从 xxx 开始」由 **AI 查清节点号** 后传入。
- Digest：**禁止**为写摘要再 `get` 整份 raw 进模型；摘要遵守必填内容约束。
- 路径白名单与 todo 附件同源思路：corpus / workbench knowledge / cache / repo `.cache` / `temp_dir`（实现时可抽共享 helper，允许先复制一套常量）。

---

## 1. 问题与目标

### 1.1 现状

| 层 | 行为 |
|----|------|
| dialogue-archive SKILL | 要求 Agent **在上下文中拼整份 raw**，再 `archive_document({ document })` |
| MCP `archive_document` | 必填 `document: string` → `POST /api/archive-document`（`packages/knowledge-mcp/index.mjs`） |
| Host | `archive_write::archive_document` 从 payload 取 `document` 字符串后解析写入（`archive_write.rs`） |

后果：长对话归档时，正文两次进入模型轨迹（拼装 + tool 入参），token 成本高。

### 1.2 目标行为

```text
人：从「xxx」开始 …（或等价范围描述）
AI：在 transcript.jsonl 中定位 → start_node / end_node（1-based）
AI：调用 Python 脚本（短参数）→ 写出 raw.md（allow 根内）
AI：archive_document({ source_path, source_type: "dialogue" })  // Host 读盘
AI：在 content_constraint 下写 digest（对话上下文，不读 raw）
AI：archive_digest({ id, digest })
```

### 1.3 非目标

- 脚本内模糊搜索「xxx」文本。
- 脚本生成 digest / 调用 MCP。
- 改 digest MCP 契约（仍收 `digest` 字符串；约束是 SKILL 写作纪律 + digest header 元信息）。
- 把 Cursor transcript 迁入 Corpus 存储布局以外的新权威源。

---

## 2. 已锁定决策（本对话）

1. raw 规范化：**机械脚本**，非模型拼装。  
2. 切片坐标：JSONL **节点号** `--start-node` / `--end-node`（1-based，闭区间）；不是 User Turn 号。  
3. AI 根据用户说的文本锚点查清节点号；多命中时优先 **user** 节点并消歧。  
4. `archive_document`：**只传文件地址**；删除 `document`。  
5. `archive_digest`：仍由 AI 基于**对话上下文**整理；须有 **内容约束**（Turn 范围叙述、一句话焦点、或二者写入同一约束文本）。  
6. 约束原文写入 digest 头（如 `> 内容约束：...`）。  
7. Transcript 格式已用真实文件核实：一行一 JSON；可混有 `turn_ended` 等非 chat 节点。

---

## 3. Transcript 格式合同（脚本输入）

**权威样本（已核实）：**  
`~/.cursor/projects/.../agent-transcripts/<uuid>/<uuid>.jsonl`

| 事实 | 结论 |
|------|------|
| 每行一个 `json.loads` 对象 | JSONL |
| 样本空行 = 0；节点序 = 物理行序（1-based） | `--start-node N` 即第 N 个非空行 JSON |
| 主体 | `{ "role": "user"|"assistant", "message": { "content": [ ... ] } }` |
| 杂质 | `{ "type": "turn_ended", "status": "..." }` 等；**切片后写入 raw 时跳过**（不进入 User/AI 块） |
| content | 数组；取 `type=="text"` 的 `text` 拼接；忽略 `tool_use` 等（或仅丢弃 tool 块，保留 text——与现 dialogue-archive「剥 tool 噪音」一致） |

**空行策略：** 计数前跳过空行再编号；若实现简化且实测无空行，可等价于物理行号，但合同写「非空 JSON 行序号」。

---

## 4. Python 脚本合同

### 4.1 路径与入口

- Create: `scripts/dialogue_archive_normalize.py`（仓库根下；可执行 `python3 scripts/dialogue_archive_normalize.py ...`）  
- 仅标准库：`argparse` / `json` / `pathlib` / `re` / `sys` / `datetime`

### 4.2 CLI 参数（敲定）

| 参数 | 必填 | 含义 |
|------|------|------|
| `--transcript` | 是 | transcript `.jsonl` 绝对路径 |
| `--start-node` | 是 | 起始节点，1-based，含 |
| `--end-node` | 是 | 结束节点，1-based，含 |
| `--out` | 是 | 输出 raw `.md` 绝对路径 |
| `--title` | 是 | `#` 标题 |
| `--project` | 否 | 默认 `inbox`；写入导航 COMMON_PATH |
| `--doc-theme` | 否 | 默认 `dialogue` |
| `--slug` | 否 | 默认从 `--out` basename 去掉 `.md` 与可选时间前缀推导；用于导航 path |
| `--created-at` | 否 | 默认脚本运行时 UTC+8，`YYYY年M月D日 HH:MM` |
| `--dry-run` | 否 | 打印节点统计与将写路径，不写文件 |

**退出码：** 0 成功；2 参数/范围错误；3 transcript 解析失败；4 写出失败。

### 4.3 机械处理步骤

1. 读取 transcript：跳过空行，解析 JSON 列表 `nodes[1..=N]`。  
2. 校验 `1 ≤ start ≤ end ≤ N`。  
3. 取闭区间 `nodes[start..=end]`。  
4. 过滤：仅保留 `role in {user, assistant}`。  
5. 组装对话轮次：  
   - 每个 `user` 开启一轮；随后连续 `assistant` 合并为该轮 AI（text 块拼接，块间 `\n\n`）。  
   - 区间以 `assistant` 开头：丢弃前缀 orphan assistant，或挂到 Turn 0——**选定：丢弃 orphan assistant，并在 stderr 打 warning**。  
   - 区间以 `user` 结尾且无 assistant：仍输出 User 块，AI 为 `（无）`。  
6. 文本清洗（确定性）：  
   - 去掉 `<timestamp>...</timestamp>`  
   - 提取 `<user_query>...</user_query>` 内文（若存在）  
   - 去掉 `<manually_attached_skills>...</manually_attached_skills>`（若 user 内嵌 query 则已由 query 提取覆盖）  
   - 去掉 `<thinking>...</thinking>` / 常见折叠思考块  
   - **不**做摘要、不改写表述  
7. 写出 Core Output Shape（与现 SKILL 一致）：

```markdown
# <title>

> 创建时间：...
> 来源：dialogue-archive
> 导航：[digest](../../../digest/<project>/<doc-theme>/<ts>-<slug>.md)

---

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

...

<!-- DDM:TURN_SEP:v1 -->

## AI

...
```

8. **输出内 Turn 编号：** 对切片结果从 **1** 重排（仅表示本文件内顺序）。在 header 增加一行机器可读元数据（可选但推荐）：

```markdown
> 源节点：<start>-<end>（transcript basename）
```

9. stdout 最后一行打印：`OUT=<absolute out path>` 便于 Agent 抓取。

### 4.4 脚本不做的事

- 不调用 MCP / HTTP  
- 不写 digest  
- 不搜索用户自然语言锚点  

---

## 5. `archive_document` 路径化（Host + MCP）

### 5.1 HTTP / MCP 合同

`POST /api/archive-document`

| 字段 | 规则 |
|------|------|
| `source_path` | **必填**；绝对路径；指向已存在 `.md` |
| `source_type` | 可选；dialogue-archive 传 `"dialogue"` |
| `document` | **禁止**；出现 → 400 |
| `translations` / 其它既有可选字段 | 行为保持；translation content 是否同期改 path **本方案不做**（仍字符串，或标为后续） |

成功响应形状保持现网（`id` / `common_path` / `raw_path` 等）。

### 5.2 Host 行为

1. 校验无 `document`、有 `source_path`。  
2. `resolve_allowed_source_file`（可与 todo attachment 共享或复制）：canonicalize、白名单、常规文件、大小上限（建议 ≥ 附件 2MiB，对话 raw 建议 **8 MiB**）。  
3. `fs::read_to_string` → 既有 `parse_archive_document` → 既有写 raw + index 逻辑。  
4. **不**把整份正文经 Adapter；Adapter 只转发短 JSON。

### 5.3 MCP schema

```text
archive_document:
  source_path: string (required)
  source_type: string (optional)
  # document: removed
```

### 5.4 兼容

- **硬切**，无 `document` 过渡期。  
- 同步改：verify.mjs、archive 相关 Rust/HTTP 单测、其它调用方 grep。  
- `archive_note_document` / 内部 `synthesize` 若走同一 HTTP：内部可继续组 string 后写盘，或改为写 temp 再 path——**内部 Rust 直调可保留 string 私有 API**；仅 **HTTP/MCP 公面** 删 `document`。

---

## 6. `archive_digest` 与内容约束

### 6.1 MCP

不改字段：`id` + `digest` + optional `force`。

### 6.2 SKILL 纪律（写作合同）

调用前必须具备非空 **内容约束** `content_constraint`（文本），允许：

- Turn/节点范围叙述：`源节点 71-200` / `从「进入技术决策」到风险释放`  
- 一句话焦点：`只写 A2 库选型门禁`  
- 组合：`节点 71-200；只覆盖决策与风险释放`

Digest 头必须含：

```markdown
> 内容约束：<content_constraint 原文>
```

禁止：为写 digest 再次拉取整份 raw；约束外扩写。

### 6.3 与脚本范围关系

脚本节点范围决定 **raw 覆盖**；digest 约束可更窄。两者都应在 Agent 当轮说明里写清。

---

## 7. SKILL 改造（dialogue-archive）

**Files:** `~/.cursor/skills/dialogue-archive/SKILL.md` + `references/archive.md`（及 workbench 副本若存在）

新工作流：

1. MCP 可用。  
2. 解析用户范围意图 → **定位** transcript 路径与 `start_node`/`end_node`（可对 jsonl 用 rg/小段读，禁止把全文读进上下文）。  
3. 选定 `title` / `out`（建议 `.cache/dialogue-archive/<ts>-<slug>.md`）。  
4. **Shell：** 跑 `dialogue_archive_normalize.py`（禁止手写 raw 正文）。  
5. `archive_document({ source_path: out, source_type: "dialogue" })`。  
6. 在 `content_constraint` 下写 digest → `archive_digest`。  
7. 输出完成摘要（raw/digest 路径）。

Explicit forbid：模型拼装完整 `document` 字符串传 MCP。

---

## 8. 影响面

| 区域 | 文件 |
|------|------|
| 脚本 | `scripts/dialogue_archive_normalize.py`（新建）+ 可选 `scripts/tests/` 或 `tests/dialogue_archive_normalize.test.py` |
| Host | `archive_write.rs`、`local_http` archive handler、路径校验 helper |
| MCP | `packages/knowledge-mcp/index.mjs`、`scripts/verify.mjs` |
| 单测 | `unit-tests/services/archive_write.rs`、`local_http` archive 用例、commands archive |
| SKILL | dialogue-archive |
| 文档 | 本计划；`references/archive.md` |

---

## 9. 验收标准

1. 对真实 jsonl：`--start-node/--end-node` 生成的 md 含 `DDM:TURN_SEP`、`---`，且 `parse_archive_document`（或 Host 归档）成功。  
2. 区间内 `turn_ended` 不出现在 User/AI 正文。  
3. MCP `archive_document` 仅 `source_path` 成功；传 `document` → 400。  
4. Agent 按新 SKILL 归档时，tool 参数无整份 raw。  
5. Digest 含 `> 内容约束：`；verify + 相关 Rust 单测绿。  
6. 脚本单元测试：固定夹具 jsonl → 稳定 md（黄金文件或关键断言）。

---

## 10. 实现任务

### Task 1：脚本 + 夹具测试

**Files:** `scripts/dialogue_archive_normalize.py`；`tests/fixtures/dialogue-archive/sample.jsonl`；测试 runner

- [x] 夹具含 user/assistant/`turn_ended`/多 assistant  
- [x] 实现 CLI 与清洗/轮次合并  
- [x] 黄金断言或 snapshot  
- [x] `--dry-run` / 非法范围 exit code  

### Task 2：Host `archive_document` 改 `source_path`

**Files:** `archive_write.rs`、路径校验、`local_http`、Rust 单测

- [x] 公面删 `document`；读 `source_path`  
- [x] 白名单 + 大小限制  
- [x] 内部 string 写入可保留私有函数供 note synthesize  
- [x] 单测：path 成功、document 拒绝、白名单外 403  

### Task 3：MCP + verify

**Files:** `knowledge-mcp/index.mjs`、`verify.mjs`

- [x] schema 硬切  
- [x] mock Host 收 `source_path`；verify 全绿  

### Task 4：SKILL

**Files:** dialogue-archive `SKILL.md`、`references/archive.md`

- [x] 新工作流；内容约束；禁止正文 document  
- [x] 示例命令行  

### Task 5：联调烟测

- [x] 对本会话或样本 jsonl：脚本 → `archive_document(source_path)` → 带约束的 `archive_digest`  
- [x] 确认 MCP 入参无巨文  

---

## 11. 风险

| 风险 | 处理 |
|------|------|
| 文本锚点多命中 | SKILL：优先 user；列出候选节点号 |
| 单行节点极大 | 脚本流式按行读；归档大小上限 8MiB |
| 内部仍有 document 调用方 | grep 清 HTTP/MCP；Rust 内部私有 API 例外写清 |
| digest 偷读 raw | SKILL 禁止；评审靠约束头 |

---

## 12. 参考锚点

- 现网写入：`src-tauri/src/services/archive_write.rs` → `archive_document`  
- 解析：`src-tauri/src/services/archive_parse.rs`  
- MCP：`packages/knowledge-mcp/index.mjs` → `archive_document` / `archive_digest`  
- SKILL：`dialogue-archive/SKILL.md`  
- 路径化先例：`docs/superpowers/plans/2026-08-05-todo-attachment-source-path.md`  
- Transcript 核实：agent-transcripts JSONL（一行一节点；含 `turn_ended`）
