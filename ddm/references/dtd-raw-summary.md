# dtd_raw_summary — 总结归档执行手册

> **配置参考**：[ddm-concepts.md](ddm-concepts.md)（路径符号 `COMMON_PATH`、`RAW`、`prefix` 定义）
>
> **模式**：`dtd_raw_summary`
>
> **输入**：用户提供的总结 Markdown，或 Agent 根据当前会话生成、经用户确认后的总结正文。
>
> **输出**：`RAW`（写入本地 archive），并更新 `index.json`。
>
> **路径基准**：读取 `{skill_dir}/../config.json`（仓库根），从 `archive_root` 获取 archive 根目录。
>
> **与 dtd_raw_dialogue 的区别**：本模式归档**已是总结形态**的正文，**不使用** `<!-- DDM:TURN_SEP:v1 -->`，**不**逐轮还原 User/AI，**不**对正文做摘要化压缩。

---

## 执行流程

### Step 1 · 选择目录并确定路径

与 [dtd-raw-dialogue.md](dtd-raw-dialogue.md) Step 1 相同。

（topics.json、project、doc-theme、topic-path、ts、slug、COMMON_PATH、prefix 规则同对话归一化。）

---

### Step 2 · 组装总结归档文档

**输入来源（二选一，执行前须明确）：**

| 来源 | 说明 |
|------|------|
| A. 用户粘贴 | 用户消息内包含完整总结 Markdown |
| B. 当场生成 | Agent 根据当前会话先写出总结，**须用户确认后再归档** |

**文档结构（格式真源）：**

```markdown
# <总标题>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：summary · 归档模式 dtd_raw_summary
> 导航：[distilled](<prefix>distilled/<COMMON_PATH>) · [digest](<prefix>digest/<COMMON_PATH>) · [trace](<prefix>trace/<COMMON_PATH>)

---

<总结正文>
```

**规则**：禁止 TURN_SEP；禁止二次摘要；导航须完整路径。

---

### Step 3 · 确定 `entries` 键

32 位小写十六进制，`secrets.token_hex(16)` 或等价。

---

### Step 4 · 更新 `CACHE_INDEX`

```json
"<id>": {
  "common_path": "<topic-path>/<ts>-<slug>.md",
  "created_at": "<ts>",
  "layers": ["raw"],
  "entry_kind": "summary"
}
```

---

### Step 5 · 写入本地 archive

写入 `raw/<topic-path>/<ts>-<slug>.md`，再覆盖 `index.json`。

---

### Step 6 · 完成输出

```
> ✅ P0-S 完成
> 📄 raw：raw/<topic-path>/<ts>-<slug>.md
> 🗂 index.json 已更新（layers: raw, entry_kind: summary）
> 💡 如需 distilled，请执行：dtd_distill_topic <raw 路径或 index id>
```

---
