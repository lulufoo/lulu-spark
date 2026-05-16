# WO-D2 · skills/theme-line/SKILL.md — Save to Archive Step 1 改写

**依赖**：WO-A1（topics.json 已改为 v4 结构）  
**修改文件**：`skills/theme-line/SKILL.md`

---

## 变更范围

只改 `Save to Archive` 下的 `### Step 1`、`### Step 3` 中的 prefix 公式、`### Step 4` 中的 zh 路径格式。  
ThemeLine 正文生成逻辑（标题处理、分节规则、扬声器标记等）**完全不变**。

---

## 修改内容

### 1. 替换 Step 1 全文

**旧内容**：

```
### Step 1 · Select topic path and determine file names

Read `{archive_root}/topics.json` and select a topic path:

```
select : one top-level topic [→ optional sub-topic]
output : topic-path = <level-1>[/<level-2>]
         slug       = kebab-case summary of the source title (English, no spaces)
         ts         = YYYYMMDDHHMM (UTC+8)
         source-file = raw/<topic-path>/<ts>-<slug>.md
```

If a slug conflict exists, clarify with the user before proceeding.
```

**新内容**：

```
### Step 1 · Select project and doc-theme, determine file names

Read `{archive_root}/topics.json` and select a project:

- Take each item's `dir` field (if present), otherwise take the last segment of `repo` (after `/`)
- Infer `doc-theme` from the video title or source content (kebab-case, English, no spaces, describes the semantic topic)

```
select : project   = dir field or repo short name (e.g. "learning-ai-agent")
         doc-theme = semantic inference from source title (e.g. "waymo-20m-rides-interview")
output : topic-path  = <project>/<doc-theme>
         slug         = kebab-case summary of the source title (English, no spaces)
         ts            = YYYYMMDDHHMM (UTC+8)
         source-file  = raw/<topic-path>/<ts>-<slug>.md
```

If a slug conflict exists, clarify with the user before proceeding.
```

### 2. 替换 Step 3 中的 prefix 公式

**旧内容**：

```
Where:

```
COMMON_PATH = <topic-path>/<ts>-<slug>.md
N           = number of path segments in topic-path  (e.g. "a/b" → N=2)
prefix      = "../" × (N+1)                          (e.g. N=2 → "../../../")
```
```

**新内容**：

```
Where:

```
COMMON_PATH = <topic-path>/<ts>-<slug>.md
prefix      = "../../../"   (topic-path is always 2 segments: project/doc-theme)
```
```

### 3. 替换 Step 4 中 zh 文件路径格式注释

**旧内容**（Step 4 末尾）：

```
Translation file path: `raw/<topic-path>/<ts>-<slug>-zh.md`
```

保持不变（zh 文件路径格式与新规则一致，`<topic-path>` 已是 `<project>/<doc-theme>`，结果正确）。

---

## 验收

执行 theme-line Save to Archive，确认：
- 生成文件路径为 `raw/<project>/<doc-theme>/<ts>-<slug>.md`
- 文件头 prefix 为 `../../../`
- zh 翻译文件（英文源时）路径为 `raw/<project>/<doc-theme>/<ts>-<slug>-zh.md`
- index.json 新增条目 `common_path` 格式为 `<project>/<doc-theme>/<ts>-<slug>.md`
