# 对话格式化：AI 执行手册

> **输入**：当前对话 / 用户提供的轮次。  
> **输出**：DDM 归一化 `.md` + 更新后的 `index.json`。

---

## 执行流程

### Step 1 · 选择目录（独立步骤）

在生成文件前，按以下顺序从 `topics.json` 确定知识落点，生成 `<topic-path>`：

1. **选知识库**：在 `topics` 列表中找到对应 `repo`（如 `lulufoo/ai-thinking-framework`）
2. **选一级主题**：在该 repo 的 `m` 中选取一级 key（如 `thinking-pattern`）
3. **选二级主题（可选）**：若一级 key 下有子目录数组，选取其中一项（如 `framework-driven-decomposition-model`）

`<topic-path>` = `<repo-name>/<一级>[/<二级>]`，如 `ai-thinking-framework/thinking-pattern`。  
`<slug>` 需与所选主题保持语义一致，有冲突先澄清。

无合适主题时，本次**落 `raw/.cache/`**，并说明原因。

**本步产出**：完整目标路径 `raw/<topic-path>/<slug>-normalized.md`（或 `raw/.cache/<slug>-normalized.md`），供后续步骤直接使用。

**通过条件**：能给出完整目标路径（或「`raw/.cache/`」及原因）后进入 Step 2。

---

### Step 2 · 生成归一化文档

以 Step 1 确定的目标路径 `raw/<topic-path>/<slug>-normalized.md` 为准，按[格式真源](#格式真源few-shot-内联)整理当前对话，得到文件内容。

> ⚠️ **内容原则：只做格式，不改内容。**  
> User 轮次和 AI 轮次的正文必须与原始对话**逐字一致**，禁止压缩、改写、摘要化。  
> 唯一允许的变更是：加分隔符 `<!-- DDM:TURN_SEP:v1 -->`、加标题 `## User` / `## AI（Turn n）`、去除与内容无关的格式噪音（如多余空行）。

---

### Step 3 · 确定 chat_id

优先从对话上下文中提取（AI Turn 1 首行格式为 `CHAT_ID-<32位随机数>`）：

- **能提取到** → 使用该值作为 `chat_id`
- **提取不到** → 自动生成一个 UUID 作为 `chat_id`

---

### Step 4 · 更新 index.json

读取 **Step 1 选定仓库** 的 `index.json`：

- 本地环境：直接读 `/Users/lulu/Code/<repo-name>/index.json`
- 非本地环境：fetch `https://github.com/lulufoo/<repo-name>/blob/main/index.json`

在 `entries` 中追加：

```json
"<chat_id>": {
  "raw": "raw/<topic-path>/<slug>-normalized.md",
  "distilled": null
}
```

---

## 格式真源（few-shot 内联）

规则：

- 首行 `# 总标题`，随后 `<!-- DDM:TURN_SEP:v1 -->`
- 交替 `## User` / `## AI（Turn n）`，`n` 从 1 起仅 AI 轮次递增
- 正文中**不出现** `{{…}}` 占位符

```markdown
# 总标题

<!-- DDM:TURN_SEP:v1 -->

## User

用户第一轮，可多行。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

AI 第一轮。

<!-- DDM:TURN_SEP:v1 -->

## User

用户第二轮。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 2）

AI 第二轮。

<!-- DDM:TURN_SEP:v1 -->
```

---

## 提交方式

归一化稿与 `index.json` 就绪后，按运行环境选择提交方式。

### 本地环境

**判定**：Step 1 选定仓库的 `.git` 存在且具备写文件与终端执行能力。

1. 写入 `raw/<topic-path>/<slug>-normalized.md`
2. 写入/更新 `index.json`
3. 提交并推送：

   ```bash
   cd /Users/lulu/Code/<repo-name>
   git add raw/<topic-path>/<slug>-normalized.md index.json
   git commit -m "chore(archive): add DDM normalized <slug>"
   git push
   ```

   commit 失败则停止并说明原因，不得 `--force`。

### 非本地环境（如 GitHub Copilot）

**判定**：无法访问本地文件系统。

1. 在对话中输出完整的归一化 markdown 文件内容
2. 在对话中输出更新后的 `index.json` 完整内容
3. 通过 GitHub API 发起 PR，变更（至少）两个文件：

   - **新增**：`raw/<topic-path>/<slug>-normalized.md`
   - **更新**：`index.json`

   PR 目标：`lulufoo/<repo-name>` → `main`  
   PR 标题：`chore(archive): add DDM normalized <slug>`

---

## 检查清单

- [ ] **Step 1**：已确定 `<topic-path>`（或「`raw/.cache/`」及原因），与 slug 不矛盾
- [ ] **Step 2**：格式与 few-shot 同构，无 `{{…}}` 占位符；文件落在 `raw/<topic-path>/<slug>-normalized.md`
- [ ] **Step 3**：`chat_id` 已确定（从上下文提取或自动生成 UUID）
- [ ] **Step 4**：`index.json` 已追加新 `chat_id` 条目（含 `raw` 字段）
- [ ] **提交**：本地已 add/commit/push；非本地已输出两份文件内容并发起 PR
