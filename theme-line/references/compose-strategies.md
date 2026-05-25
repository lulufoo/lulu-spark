# Compose Strategies

> 输入：[TranscriptBundle](bundle-schema.md)（内存）→ 输出：ThemeLine markdown body（无 archive header）

---

## Strategy selection（互斥，无 hybrid）

| Strategy | Condition |
|----------|-----------|
| `segment-seeded` | `segments.length > 0` AND `utterances.length > 0` |
| `flat-caption` | `segments.length == 0` AND `utterances.length > 0` |
| `summary-only` | `segments.length > 0` AND `utterances.length == 0` |
| **fail-fast** | `segments.length == 0` AND `utterances.length == 0` |

**fail-fast：** 立即中止 Compose，报告 Acquire 失败，**不进入 Archive**。

---

## Steps C0–C6

### C0 · Fail-fast check

若 `segments` 与 `utterances` 均为空 → 输出失败原因，停止。

### C1 · Apply `fidelity.corrections`

对每条 utterance 的 `text` 应用 `{wrong → correct}` 替换。

**算法（可操作）：**

1. 对每个 `meta.speakers` 中的 canonical 名，在 utterances 合并文本中搜索：长度 ≥2 的中文 token，不在 speakers 列表，且与 canonical 名共享首字或 Levenshtein 距离 ≤2
2. 候选 `wrong` 出现 ≥3 次 → 写入 `{wrong, correct: canonical}`
3. 无候选则 `corrections: []`；**禁止** adapter 硬编码 per-video 表

> adapter Map 阶段（t2）可预填 corrections；Compose C1 须再应用/合并。

### C2 · Select strategy

按上表精确匹配（非 fail-fast 时）。

### C3 · Build skeleton

- **segment-seeded：** section 边界 = segments 按 `start_sec` 排序；标题取自 segment `title` 或 `summary` 首句
- **flat-caption：** 语义 + 时间窗口切 section（目标 3–8 分钟/段）
- **summary-only：** 每 segment 一节；body 来自 `summary`

### C4 · Assign utterances to sections

- **segment-seeded：** utterances 分配区间 `[start_sec, next_start_sec)`；coverage < 30% 时以 segment `summary` 为锚，utterances 补充
- **flat-caption：** 按时间窗口分组
- **summary-only：** 无 utterances；仅用 segment summary

### C5 · Speaker turns

- 推断说话人；无法确定标 `(uncertain)`
- **flat-caption 非 diarization**；上限 Host/Guest + `(uncertain)`
- segment-seeded 可用 segment.speaker 或 meta.speakers

### C6 · Quality pass & emit

- 去除 ASR 重复
- Fidelity 版权检查（见下）
- 输出 ThemeLine body

---

## Fidelity（copyright）

- Compose 输出为 **dialogue-style paraphrase**
- **禁止** near-complete verbatim SRT 复制
- 短引语 ≤2 句/section 可接受

---

## Strategy details

### segment-seeded

- 典型来源：InfoQ（manuscripts + SRT）
- Section heading = manuscript 标题
- Time line = segment `start_sec` 范围

### flat-caption

- 典型来源：YouTube auto-sub
- 语义切分 + 时间窗口（3–8 min/section）
- 说话人推断保守；`(uncertain)` 仅必要时

### summary-only

- 典型来源：InfoQ SRT 403（utterances 空）
- 每 segment 一节；content 来自 `summary` 字段

---

## Output

ThemeLine body only（title / metadata / navigation 由 Phase 3 Archive 处理，见 [output-templates.md](output-templates.md)）。

可选 provenance（Archive header）：

```markdown
> 采集：{platform} · {strategy} · 嘉宾：{speakers}
```
