# DDM Distill：子话题模式

> **配置参考**：[ddm-concepts.md](ddm-concepts.md)（路径符号定义）
>
> **模式**：`dtd_distill_topic`
>
> **输入**：`RAW`（`{archive_root}/raw/<COMMON_PATH>`）；路径分量从 raw 文件解析。
>
> **输出**：`DISTILLED`，写入 `distilled/<COMMON_PATH>`。

> ⚠️ 生成时直接读取 `RAW` 全文，不依赖记忆。

---

## PART 1 — 子话题块定义

**子话题 distilled = 以话题为单位的索引**

将整个对话按内容分组，每组：

- 一行标题（抽象话题名）
- Turn 范围标识
- 每个 Turn 一条 bullet

**输出块格式**：

```markdown
**[话题标题]**｜Turn X–Y

- Turn X：[简短描述]
```

---

## PART 2 — 执行规范

### [P2-0] 强制加载 raw

第一个动作：读取 `RAW` 全文。输出 `> ✅ 已读取 raw：<路径>` 后进入 [P2-1]。

### [P2-1] 文档结构

```markdown
# [主题标题]

> 创建时间：[与 raw ts 一致]
> 导航：[digest](`<prefix>`digest/`<COMMON_PATH>`) · [trace](`<prefix>`trace/`<COMMON_PATH>`) · [raw](`<prefix>`raw/`<COMMON_PATH>`)

---

**[话题一]**｜Turn 1–3
- Turn 1：…
```

### [P2-2] 生成步骤

扫描 User 轮次 → 归组 → 生成标题与 Turn 列表。

### [P2-3] 纯执行指令

并入相邻话题组，不单独建组。

### [P2-4] 输出模式

默认 `只写文件`。

### [P2-5] 落盘

写入 `distilled/<topic-path>/<ts>-<slug>.md`，`layers` 追加 `distilled`。

```
> ✅ dtd_distill_topic 完成 · distilled：distilled/<COMMON_PATH>
```

---
