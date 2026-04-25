# 从对话到 DDM 归一化：AI 执行手册

> 输入：当前对话/用户提供的轮次。
> 输出：DDM 归一化 `.md` + 更新后的 `index.json`。

---

## 执行流程

### Step 1 · 生成归一化文档

按格式真源整理当前对话，得到 `<主题-slug>-normalized.md` 内容。

### Step 2 · 确定 chat_id

优先从对话上下文中提取（AI Turn 1 首行格式为 `CHAT_ID-<32位随机数>`）：

- **能提取到** → 使用该值作为 `chat_id`
- **提取不到** → 自动生成一个 UUID 作为 `chat_id`

### Step 3 · 更新 index.json

从以下路径读取当前 `index.json`（本地环境自动映射为本地文件，非本地环境直接 fetch）：

```
https://github.com/lulufoo/cognitive-trace-archive/blob/main/index.json
```

在 `entries` 中追加：

```json
"<chat_id>": {
  "raw": "<slug>-normalized.md",
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

两份文档（`<slug>-normalized.md` 和更新后的 `index.json`）生成完毕后，按运行环境选择提交方式：

### 本地环境

**判定**：`/Users/lulu/Code/cognitive-trace-archive/.git` 存在且具备写文件与终端执行能力。

1. 写入 `/Users/lulu/Code/cognitive-trace-archive/raw/<slug>-normalized.md`
2. 写入 `/Users/lulu/Code/cognitive-trace-archive/index.json`
3. 提交并推送：
   ```bash
   cd /Users/lulu/Code/cognitive-trace-archive
   git add raw/<slug>-normalized.md index.json
   git commit -m "chore(archive): add DDM normalized <slug>"
   git push
   ```
   commit 失败则停止并说明原因，不得 `--force`。

---

### 非本地环境（如 GitHub Copilot）

**判定**：无法访问本地文件系统。

1. 在对话中输出完整的归一化 markdown 文件内容
2. 在对话中输出更新后的 `index.json` 完整内容
3. 通过 GitHub API 发起 PR，变更两个文件：
   - **新增**：`raw/<slug>-normalized.md`
   - **更新**：`index.json`

   PR 目标：`lulufoo/cognitive-trace-archive` → `main`
   PR 标题：`chore(archive): add DDM normalized <slug>`

---

## 检查清单

- [ ] 格式与 few-shot 同构，无 `{{…}}` 占位符
- [ ] 文件名为 `a-b-c-normalized.md`
- [ ] `index.json` 已追加新 `chat_id` 条目（含 `raw` 字段）
- [ ] **本地环境**：已落盘 `raw/` 和 `index.json`，已 add/commit/push
- [ ] **非本地环境**：已在对话中输出两份文件内容，已发起 PR
