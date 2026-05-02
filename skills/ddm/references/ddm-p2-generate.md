# DDM Phase 2：生成

> **前置依赖**：[DDM_CONCEPTS](ddm-concepts.md)（distilled 结构定义、四条保留规则）
>
> **输入**：`CACHE_RAW`（Phase 0 产出）；`DIAGNOSE`（Phase 1 产出，`diagnose/<COMMON_PATH>`）；`<topic-path>`、`<ts>`、`<slug>` 从 `CACHE_RAW` 文件路径解析。
>
> **输出**：`CACHE_DISTILLED`，写入本地 archive。

> ⚠️ 生成时直接读取 `CACHE_RAW` 文件，按 [P2-2] 四条规则从后往前扫，不依赖记忆。

---

## [P2-0] 强制加载 raw（第一步，不可跳过）

执行 P2 的**第一个动作**必须是读取 `CACHE_RAW` 全文（`{archive_root}/.cache/<topic-path>/<ts>-<slug>-raw.md`）。

加载完成后输出：`> ✅ 已读取 raw：<路径>`，然后才可进入 [P2-1]。

- 禁止以 `DIAGNOSE` 摘要替代 raw 全文
- 禁止凭记忆生成内容，所有 User Turn 必须从 raw 文件逐行读取

---

## [P2-1] 文档结构

**文档格式**：User 原文为主干，AI 内容为注解，按对话顺序交织。正文不使用章节标题。

```markdown
# [主题标题]

> 创建时间：[参考：DDM_CONCEPTS 的文内创建时间]

> 导航：[digest](`<prefix>`digest/`<COMMON_PATH>`) · [trace](`<prefix>`trace/`<COMMON_PATH>`) · [raw](`<prefix>`raw/`<COMMON_PATH>`)

---

**User（Turn 1）**

[User 原文]

---

**AI**

[AI 最小注解]

---

**User（Turn 2）**

[User 原文]

---

**AI**

[AI 最小注解]

---

**遗留**：[仅来自 User 明示的未解决问题；User 未提则省略此部分，含 --- 分隔线和 **遗留** 行]
```

**格式说明**：
- 导航行为普通 bold 行（非 blockquote），使用 `·` 分隔，紧跟创建时间之后
- 每个 block（User / AI）之间均插入 `---` 水平分隔线
- Turn 标签使用 `**User（Turn x）**` / `**AI（Turn x）**`（bold 行），与 P0 的 `## User` / `## AI（Turn x）` 对应，Turn 编号与 raw 一致
- **禁止跳过任何 User Turn**：distilled 的 User 块数量必须与 raw 一致（排除纯执行指令如"执行：ACN"）

---

## [P2-2] 正文写法

**User 块规则**：

```text
· 标记：blockquote（> 前缀）
· 内容：对话中的 User 原文，100% 保留，一字不改
· 包括：错别字、口语、不完整句、标点不规范
· 禁止：改写、摘要、合并多条 User 消息
```

**AI 块生成规则**：

```text
Step 1：依赖扫描（从后往前）
  · 所有 User 节点标记为"必须理解"
  · 从最后一段 AI 开始往前扫：
    若此 AI 段被任何"必须理解"节点引用 → 标记为保留
    否则 → 删除
  · 被标记保留的 AI 段，它引用的更早 AI 段递归加入保留集合
  · 依据 DDM_CONCEPTS 四条规则判断引用关系

Step 2：内部一致性检查（对每个保留节点）
  · 若保留了结论 C，且 C 依赖同一 AI 段内的推导步骤 S
  · 则 S 必须同时保留（不允许结论凭空出现）
  · 上界：使结论成立的最少前提，不是完整 AI 输出

Step 3：格式输出
  · 纯散文，无标题，无 bullet list
  · 代码块仅在空间结构/数据结构用语言无法清晰表达时使用
  · 纠偏直接体现在对应 AI 块正文中，不另立偏差表
  · 禁止输出 DDM 内部术语
```

**特殊情况**：

```text
· User 消息是纯确认（"对""是的""明白了"）：
  保留 User 原文；前面 AI 块只保留被确认的结论句

· User 消息引用了前一 AI 块的具体词语（"你说的 X 是什么意思"）：
  前 AI 块必须保留该词语及最简定义，即使它不在其他依赖路径上

· AI 块内有推导链，结论需要但推导步骤可删：
  按 Step 2 内部一致性规则，保留最少前提，不是全部推导过程
```

---

## [P2-3] 遗留问题

**遗留**（条件输出）：仅来自 User 在对话中明示的未解决问题或下一步方向，保留原文或近似原文。AI 不生成遗留问题总结。User 未提则省略 Footer（`---` 分隔线和 `**遗留**` 行均不输出）。

---

## [P2-4] 格式规则

```text
· 输出必须是完整 Markdown 文档
· 落盘路径：distilled/<topic-path>/<ts>-<slug>.md
· 代码块：仅在空间结构/数据结构用语言无法清晰表达时使用，不用于知识解释
· 正文不使用二级及以下标题，文档只有一级标题（主题标题）
· 不输出 DDM 内部术语
```

---

## [P2-5] 落盘

**导航前缀**（`COMMON_PATH` 由 Phase 0 确定）：

    N      := |topic-path|     -- topic-path 的路径段数（如 "a/b" → N=2）
    prefix := "../" × (N+1)    -- 示例：N=2 → "../../../"

将生成内容写入 `CACHE_DISTILLED`（`{archive_root}/.cache/<topic-path>/<ts>-<slug>-distilled.md`），然后写入本地 archive：

```
{archive_root}/distilled/<topic-path>/<ts>-<slug>.md
```

同时更新 `{archive_root}/index.json`，将对应条目的 `"distilled"` 设为 `true`。

完成后输出：`> ✅ Step 2 完成 · distilled：distilled/<topic-path>/<ts>-<slug>.md（输入来源：raw）`
