# 对话格式化：AI 执行手册

> **配置参考**：[ddm-concepts.md](ddm-concepts.md)（路径符号 `CACHE_RAW`、`COMMON_PATH`、`DIAGNOSE`、`CTA_BASE` 定义）

> **输入**：当前对话 / 用户提供的对话文档。
>
> **输出**：`CACHE_RAW`（写入本地 archive），并更新 `CACHE_INDEX`。
>
> **路径基准**：执行前先读取 `./config.json`（位于 skill 目录），从 `archive_root` 字段获取本地 archive 根目录，后续所有路径均基于此值。

---

## 执行流程

### Step 1 · 选择目录并确定路径

读取 `{archive_root}/topics.json`，从中选择主题路径：

```
select : repo → 一级主题 [→ 二级主题（可选）]
output : topic-path = <一级>[/<二级>]
         slug       = 与主题语义一致（有冲突先澄清）
         ts         = YYYYMMDDHHMM（东八区，后续 Phase 复用）
         目标路径    = raw/<topic-path>/<ts>-<slug>.md
```

---

### Step 2 · 生成归一化文档

- input  : 当前对话全部轮次
- output : `CACHE_RAW`（`{archive_root}/.cache/<topic-path>/<ts>-<slug>-raw.md`），结构与下方示例同构
- 头部   : # 总标题 / 创建时间 / 导航完整链接（在首个 TURN_SEP 之前）
- rule   : 正文与原始对话逐字一致；仅可加分隔符 / 标题 / 去格式噪音
- 剥离   : AI 推导性独白（折叠思考块 / 无关前缀句）；讲解形式的推理保留
- 禁止   : 压缩 / 改写 / 摘要化 / {{…}} 占位符出现 / 导航行使用占位符

> **导航链接规则**：`topic-path`、`ts`、`slug` 在 Step 1 完成后即完全确定，`COMMON_PATH` 和 `prefix` 可立即计算，**raw / distilled / digest / trace 的完整路径一次性写入，不得使用占位符**。
>
>     N      := |topic-path|      -- topic-path 的路径段数（如 "a/b" → N=2）
>     prefix := "../" × (N+1)     -- 示例：N=2 → "../../../"

#### 完整示例（格式真源）

```markdown
# 总标题

> 创建时间：2026年4月25日 15:32

> 导航：[distilled](`<prefix>`distilled/`<COMMON_PATH>`) · [digest](`<prefix>`digest/`<COMMON_PATH>`) · [trace](`<prefix>`trace/`<COMMON_PATH>`)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

用户第一轮，可多行。

<!-- DDM:TURN_SEP:v1 -->

## AI

AI 第一轮。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

用户第二轮。

<!-- DDM:TURN_SEP:v1 -->

## AI

AI 第二轮。

<!-- DDM:TURN_SEP:v1 -->
```

---

### Step 3 · 确定 `entries` 键

`<id>` 为 32 位小写十六进制，使用 Python `secrets.token_hex(16)`（或等价）生成。

---

### Step 4 · 更新 `CACHE_INDEX`

读取 `{archive_root}/index.json` 到 `CACHE_INDEX`（`{archive_root}/.cache/index.json`）。

在 `CACHE_INDEX` 的 `entries` 末尾追加（键为 Step 3 的 `<id>`）：

```json
"<32位十六进制id>": {
  "common_path": "<topic-path>/<ts>-<slug>.md",
  "created_at": "<ts>",
  "raw": true,
  "distilled": false,
  "diagnose": false,
  "digest": false,
  "trace": false
}
```

---

### Step 5 · 写入本地 archive

先写 `.md`，再写 `index.json`，避免索引短时间指向尚未存在的正文路径。

#### 归一化 `.md`

将 `CACHE_RAW` 内容写入：

```
{archive_root}/raw/<topic-path>/<ts>-<slug>.md
```

（目录不存在则创建）

#### `index.json`

将 `CACHE_INDEX` 内容覆盖写入：

```
{archive_root}/index.json
```

---

### Step 6 · 完成输出

```
> ✅ Step 0 完成
> 📄 raw：raw/<topic-path>/<ts>-<slug>.md
> 🗂 index.json 已更新（新增条目 <id>）
```

---
