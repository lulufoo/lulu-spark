# 从对话到 DDM 归一化：AI 执行手册

> 输入：当前对话/用户提供的轮次。
> 输出：DDM 归一化 `.md`；本地可执行时完成 git add → commit → push。

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

## 文件路径

- **归档根**：`$COGNITIVE_TRACE_ARCHIVE_ROOT`（未设置时：`~/Code/cognitive-trace-archive`）
- **落盘位置**：`{归档根}/raw/`
- **文件名**：`主题-slug-normalized.md`（小写 a-b-c 风格）

---

## 执行步骤

按序执行以下三步，不得跳跃：

### Step 1 · 生成归一化文档

按「格式真源」将当前对话上下文整理为 DDM 归一化 `.md`，写入：

```
{归档根}/raw/<主题-slug>-normalized.md
```

### Step 2 · 检测本地环境

文档落盘后，检测以下全部条件：

| 条件 | 检测方式 |
|------|----------|
| 归档根存在且有 `.git` | `test -d ~/Code/cognitive-trace-archive/.git` |
| 具备写文件与终端执行能力 | 上一步已成功落盘 |
| 用户/任务已授权提交推送 | 当前对话中有明确指示 |

- **全部满足** → 执行 Step 3
- **任一不满足** → 输出文件内容与命令块，说明缺失条件，停止

### Step 3 · git add → commit → push

```bash
cd "${COGNITIVE_TRACE_ARCHIVE_ROOT:-$HOME/Code/cognitive-trace-archive}"
git add raw/<slug>-normalized.md
git commit -m "chore(archive): add DDM normalized <slug>"
git push
```

commit 失败则停止并说明原因，不得 `--force`。

---

## 检查清单

- [ ] 格式与 few-shot 同构，无 `{{…}}` 占位符
- [ ] 文件在 `raw/`，命名为 `a-b-c-normalized.md`
- [ ] Step 2 检测通过：已 add/commit/push；或说明缺失条件
