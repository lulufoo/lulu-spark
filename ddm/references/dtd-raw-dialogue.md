# dtd_raw_dialogue — 对话归一化执行手册

> **配置参考**：[ddm-concepts.md](ddm-concepts.md)（路径符号 `COMMON_PATH`、`RAW`、`CTA_BASE` 定义）
>
> **模式**：`dtd_raw_dialogue`
>
> **输入**：当前对话 / 用户提供的对话文档。
>
> **输出**：`RAW`（写入本地 archive），并更新 `index.json`。
>
> **路径基准**：读取 `{skill_dir}/../config.json`（仓库根），从 `archive_root` 获取 archive 根目录。

---

## 执行流程

### Step 1 · 选择目录并确定路径

读取 `{archive_root}/topics.json`，从 `topics` 数组中选择 project：

- 取每项的 `dir` 字段（若存在），否则取 `repo` 的最后一段（`/` 之后）
- 根据对话内容和标题，**语义推断 `doc-theme`**（kebab-case，描述本文档的核心主题，英文，无空格，**3-5 个单词**）
- 若 slug 已确定且与 doc-theme 语义一致，可直接复用
- **若无合适项目**，`project` 填 `inbox`，不得默认选一个相近项目。inbox 为暂缓目录，卡片会标红提示用户后续通过「↳ 移项」迁移到正式项目。

```
select : project   = dir 字段 或 repo 短名（如 "ai-software-dev"）；无合适项目时填 "inbox"
         doc-theme = 语义推断（如 "agentic-coding-discipline"，3-5 个单词）
output : topic-path = <project>/<doc-theme>
         slug       = 与主题语义一致的 kebab-case（有冲突先澄清）
         ts         = YYYYMMDDHHMM（东八区，后续 Phase 复用）
         目标路径    = raw/<topic-path>/<ts>-<slug>.md
```

---

### Step 2 · 生成归一化文档

- input  : 当前对话全部轮次
- output : `RAW`（`{archive_root}/raw/<topic-path>/<ts>-<slug>.md`），结构与下方示例同构
- 头部   : # 总标题 / 创建时间 / 导航完整链接（在首个 TURN_SEP 之前）
- rule   : 正文与原始对话逐字一致；仅可加分隔符 / 标题 / 去格式噪音
- 剥离   : AI 推导性独白（折叠思考块 / 无关前缀句）；讲解形式的推理保留
- 禁止   : 压缩 / 改写 / 摘要化 / {{…}} 占位符出现 / 导航行使用占位符

> **导航链接规则**：`topic-path`、`ts`、`slug` 在 Step 1 完成后即完全确定，`COMMON_PATH` 和 `prefix` 可立即计算，**raw / distilled / digest / trace 的完整路径一次性写入，不得使用占位符**。
>
>     prefix := "../../../"       -- topic-path 固定为 2 段（project/doc-theme），N 恒=2

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
  "layers": ["raw"]
}
```

---

### Step 5 · 写入本地 archive

先写 `.md`，再写 `index.json`，避免索引短时间指向尚未存在的正文路径。

#### 归一化 `.md`

将生成内容写入：

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
> ✅ P0 完成
> 📄 raw：raw/<topic-path>/<ts>-<slug>.md
> 🗂 index.json 已更新（新增条目 <id>）
```

---
