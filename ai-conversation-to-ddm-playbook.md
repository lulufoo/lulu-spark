# 从对话到 DDM 归一化：AI 执行手册

> 输入：当前对话/用户提供的轮次。
> 输出：DDM 归一化 `.md`。

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

## 执行模式

### 模式 A · 本地环境

**判定条件**：`/Users/lulu/Code/cognitive-trace-archive/.git` 存在，且具备写文件与终端执行能力。

**执行**：

1. 按格式真源将对话写入：
   ```
   /Users/lulu/Code/cognitive-trace-archive/raw/<主题-slug>-normalized.md
   ```
2. 提交并推送：
   ```bash
   cd /Users/lulu/Code/cognitive-trace-archive
   git add raw/<slug>-normalized.md
   git commit -m "chore(archive): add DDM normalized <slug>"
   git push
   ```
   commit 失败则停止并说明原因，不得 `--force`。

---

### 模式 B · 非本地环境（如 GitHub Copilot）

**判定条件**：无法访问本地文件系统。

**执行**：直接在对话中输出完整 markdown 文件内容，用户自行保存。

---

## 检查清单

- [ ] 格式与 few-shot 同构，无 `{{…}}` 占位符
- [ ] 文件名为 `a-b-c-normalized.md`
- [ ] **模式 A**：已落盘 `raw/` 并 add/commit/push
- [ ] **模式 B**：已在对话中完整输出 markdown
