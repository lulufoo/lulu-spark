# 从对话到 DDM 归一化：AI 执行手册

**目标**：将当前对话/用户提供的轮次产出符合 DDM 的归一化 `.md`，本地可执行时完成 git add → commit → push。

---

## 1. 格式真源

读 `templates/ddm-normalized-output-template.md` **第一节 + few-shot**，成稿与其同构：

- 首行 `# 总标题`，随后 `<!-- DDM:TURN_SEP:v1 -->`
- 交替 `## User` / `## AI（Turn n）`，`n` 从 1 起仅 AI 轮次递增
- 正文中**不出现** `{{…}}` 占位符

---

## 2. 文件路径

- **归档根**：`$COGNITIVE_TRACE_ARCHIVE_ROOT`（未设置时：`~/Code/cognitive-trace-archive`）
- **落盘位置**：`{归档根}/raw/`
- **文件名**：`主题-slug-normalized.md`（小写 a-b-c 风格）

---

## 3. 执行路径

**优先 A（有脚本环境）**

1. 整理原始稿 `raw/slug.md`（行首用 `**User：**` / `**AI：**` 标记）
2. 执行 `python3 normalize.py` → 自动生成 `slug-normalized.md`、更新 `index.json`

**降级 B（无脚本）**

1. 按第 1 节直接写出 `raw/slug-normalized.md`
2. 按第 4 节手动更新 `index.json`

---

## 4. 判定「本地可执行」

满足以下**全部**条件时视为本地可执行：

- `~/Code/cognitive-trace-archive` 存在且有 `.git`
- 具备写文件、执行终端的能力（push 还需网络）
- 用户/任务已明确允许提交与推送

**否则**：只输出文件内容与可供用户自行执行的命令块，说明缺失条件。

---

## 5. 手动更新 index.json（仅路径 B）

```bash
cd "${COGNITIVE_TRACE_ARCHIVE_ROOT:-$HOME/Code/cognitive-trace-archive}"
python3 -c "
from pathlib import Path
import normalize
p = Path('raw/xxx-normalized.md')
assert p.is_file(), p
idx = normalize.load_index()
cid = normalize.find_chat_id_by_raw_filename(idx, p.name) or normalize.generate_chat_id()
normalize.index_set_raw(cid, p.name)
print('index_set_raw', cid, '->', p.name)
"
```

---

## 6. git add → commit → push（本地且已授权）

```bash
cd "${COGNITIVE_TRACE_ARCHIVE_ROOT:-$HOME/Code/cognitive-trace-archive}"
git add raw/<文件名>.md index.json
git commit -m "chore(archive): add DDM normalized <slug>"
git push
```

commit 失败时停止并说明原因，不得强推。

---

## 7. 检查清单

- [ ] 格式与模板第一节 few-shot 同构，无 `{{…}}` 占位符
- [ ] 文件在 `raw/`，命名为 `a-b-c-normalized.md`
- [ ] `index.json` 已更新（经脚本或手动）
- [ ] 本地且已授权：已 add/commit/push 或说明原因
