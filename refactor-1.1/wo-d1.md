# WO-D1 · skills/ddm/references/ddm-p0-normalize.md — Step 1 改写

**依赖**：WO-A1（topics.json 已改为 v4 结构）  
**修改文件**：`skills/ddm/references/ddm-p0-normalize.md`

---

## 变更范围

只改 `### Step 1 · 选择目录并确定路径` 一节，以及 Step 2 中的 prefix 公式注释。  
其余内容（Step 2 正文、Step 3、Step 4、Step 5）保持不变。

---

## 修改内容

### 替换 Step 1 全文

**旧内容**（完整替换以下段落）：

```
### Step 1 · 选择目录并确定路径

读取 `{archive_root}/topics.json`，从中选择主题路径：

```
select : repo → 一级主题 [→ 二级主题（可选）]
output : topic-path = <一级>[/<二级>]
         slug       = 与主题语义一致（有冲突先澄清）
         ts         = YYYYMMDDHHMM（东八区，后续 Phase 复用）
         目标路径    = raw/<topic-path>/<ts>-<slug>.md
```
```

**新内容**：

```
### Step 1 · 选择目录并确定路径

读取 `{archive_root}/topics.json`，从 `topics` 数组中选择 project：

- 取每项的 `dir` 字段（若存在），否则取 `repo` 的最后一段（`/` 之后）
- 根据对话内容和标题，**语义推断 `doc-theme`**（kebab-case，描述本文档的核心主题，英文，无空格）
- 若 slug 已确定且与 doc-theme 语义一致，可直接复用

```
select : project   = dir 字段 或 repo 短名（如 "ai-software-dev"）
         doc-theme = 语义推断（如 "agentic-coding-discipline"）
output : topic-path = <project>/<doc-theme>
         slug       = 与主题语义一致的 kebab-case（有冲突先澄清）
         ts         = YYYYMMDDHHMM（东八区，后续 Phase 复用）
         目标路径    = raw/<topic-path>/<ts>-<slug>.md
```
```

### 替换 Step 2 中的 prefix 公式注释

**旧内容**：

```
>     N      := |topic-path|      -- topic-path 的路径段数（如 "a/b" → N=2）
>     prefix := "../" × (N+1)     -- 示例：N=2 → "../../../"
```

**新内容**：

```
>     prefix := "../../../"       -- topic-path 固定为 2 段（project/doc-theme），N 恒=2
```

---

## 验收

执行 DDM P0（`dtd_normalize`），确认：
- Step 1 输出的 `topic-path` 格式为 `<project>/<doc-theme>`（2段，无第3段）
- 生成文件路径为 `raw/<project>/<doc-theme>/<ts>-<slug>.md`
- 文件内导航链接 prefix 为 `../../../`
- 无 `N = ...` 或 `"../" × (N+1)` 字样出现在推理过程中
