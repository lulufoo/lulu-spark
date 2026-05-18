# 总结归档：AI 执行手册（Phase 0-S）

> **配置参考**：[ddm-concepts.md](ddm-concepts.md)（路径符号 `COMMON_PATH`、`RAW`、`prefix` 定义）
>
> **模式**：`dtd_archive_summary`
>
> **输入**：用户提供的总结 Markdown，或 Agent 根据当前会话生成、经用户确认后的总结正文。
>
> **输出**：`RAW`（写入本地 archive），并更新 `index.json`。
>
> **路径基准**：执行前先读取 skill 目录下的 `config.json`，从 `archive_root` 字段获取本地 archive 根目录，后续所有路径均基于此值。
>
> **与 P0 对话归一化的区别**：本模式归档**已是总结形态**的正文，**不使用** `<!-- DDM:TURN_SEP:v1 -->`，**不**逐轮还原 User/AI，**不**对正文做摘要化压缩；**不**自动执行 P2 overview。

---

## 执行流程

### Step 1 · 选择目录并确定路径

与 [ddm-p0-normalize.md](ddm-p0-normalize.md) Step 1 相同。

读取 `{archive_root}/topics.json`，从 `topics` 数组中选择 project：

- 取每项的 `dir` 字段（若存在），否则取 `repo` 的最后一段（`/` 之后）
- 根据**总结标题与正文主题**，**语义推断 `doc-theme`**（kebab-case，英文，无空格，**3–5 个单词**）
- 若 slug 已确定且与 doc-theme 语义一致，可直接复用
- **若无合适项目**，`project` 填 `inbox`，不得默认选一个相近项目

```
select : project   = dir 字段 或 repo 短名；无合适项目时填 "inbox"
         doc-theme = 语义推断（如 "lulu-workbench-cs-refactor"）
output : topic-path = <project>/<doc-theme>
         slug       = 与主题语义一致的 kebab-case（有冲突先澄清）
         ts         = YYYYMMDDHHMM（东八区）
         目标路径    = raw/<topic-path>/<ts>-<slug>.md
         COMMON_PATH = <topic-path>/<ts>-<slug>.md
         prefix     = "../../../"
```

**总标题 `<h1>` 来源（按优先级）：**

1. 总结正文首行 `# 标题`
2. 用户消息中显式给出的 `title`
3. 由 `doc-theme` 人工可读化（最后手段，执行前向用户确认）

---

### Step 2 · 组装总结归档文档

**输入来源（二选一，执行前须明确）：**

| 来源 | 说明 |
|------|------|
| A. 用户粘贴 | 用户消息内包含完整总结 Markdown |
| B. 当场生成 | Agent 根据当前会话先写出总结，**须用户确认后再归档**；不得未经确认擅自改写用户已提供的总结 |

**输出路径：**

```
{archive_root}/raw/<topic-path>/<ts>-<slug>.md
```

**文档结构（格式真源）：**

```markdown
# <总标题>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：summary · 归档模式 dtd_archive_summary
> 导航：[distilled](<prefix>distilled/<COMMON_PATH>) · [digest](<prefix>digest/<COMMON_PATH>) · [trace](<prefix>trace/<COMMON_PATH>)

---

<总结正文>
```

**规则：**

| 类型 | 规则 |
|------|------|
| **禁止** | 插入 `<!-- DDM:TURN_SEP:v1 -->`（除非总结正文内作为示例引用） |
| **禁止** | 将总结正文再压缩、改写、二次摘要 |
| **禁止** | 导航行使用占位符；`topic-path`、`ts`、`slug` 确定后必须写**完整相对路径** |
| **允许** | 修正明显错别字；统一标题层级与列表格式；补全一级 `# 总标题` |
| **允许** | 在元信息与正文之间使用 `---` 分隔 |
| **允许** | 可选元信息行（有则写，无则省略） |

**可选元信息行（写在「来源」行之后、`---` 之前）：**

```markdown
> 对话范围：Turn 1～N
> 主题标签：关键词1 · 关键词2
```

**创建时间行**：须与 Step 1 的 `ts`（东八区）一致；月日不补零，格式见 [ddm-concepts.md](ddm-concepts.md)。

**正文区 `<总结正文>`：**

- 保留用户或已确认总结的 Markdown 主体（可含 `##` 小节、列表、表格、代码块）
- 若正文已含与头部重复的 `# 总标题`，保留一处即可，避免连续两个一级标题；优先保留头部 `# <总标题>`

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
  "layers": ["raw"],
  "entry_kind": "summary"
}
```

> `entry_kind` 为可选扩展字段；旧版 viewer 可忽略。若需最大兼容，可省略 `entry_kind`，但推荐保留以便筛选。

---

### Step 5 · 写入本地 archive

先写 `.md`，再写 `index.json`，避免索引短时间指向尚未存在的正文路径。

#### 总结归档 `.md`

将 Step 2 生成内容写入：

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
> ✅ dtd_archive_summary 完成
> 📄 raw：raw/<topic-path>/<ts>-<slug>.md
> 🗂 index.json 已更新（layers: raw, entry_kind: summary）
> 💡 如需 distilled，请执行：dtd_distill_overview <raw 路径或 index id>
```

---

## 完整示例（参考）

**输入（用户粘贴节选）：**

```markdown
# LuLu Workbench C/S 模式重构方案讨论 — 总结

## 一、讨论背景
…
```

**输出文件 `raw/ai-software-dev/lulu-workbench-cs-refactor/202605181430-lulu-workbench-cs-refactor-discussion.md`（节选）：**

```markdown
# LuLu Workbench C/S 模式重构方案讨论 — 总结

> 创建时间：2026年5月18日 14:30
> 来源：summary · 归档模式 dtd_archive_summary
> 导航：[distilled](../../../distilled/ai-software-dev/lulu-workbench-cs-refactor/202605181430-lulu-workbench-cs-refactor-discussion.md) · [digest](../../../digest/ai-software-dev/lulu-workbench-cs-refactor/202605181430-lulu-workbench-cs-refactor-discussion.md) · [trace](../../../trace/ai-software-dev/lulu-workbench-cs-refactor/202605181430-lulu-workbench-cs-refactor-discussion.md)
> 对话范围：Turn 1～14

---

## 一、讨论背景
…
```

---

## 常见错误

| 错误 | 处理 |
|------|------|
| 无总结正文、仅一句「归档」 | 向用户索要 Markdown，或先生成总结并确认 |
| 误用 `dtd_normalize` 归档长总结 | 改用本模式；`dtd_normalize` 要求逐轮对话与 TURN_SEP |
| 导航链接含 `{{…}}` 或省略 `ts-slug` | 按 Step 1 已确定的 `COMMON_PATH` 写全路径 |
| 自动写入 `distilled/` | 本模式禁止；需用户另行执行 `dtd_distill_overview` 等 |
