# Compose Strategies

> 输入：[TranscriptBundle](bundle-schema.md)（内存）→ 输出：ThemeLine markdown body（无 archive header）

---

## Strategy selection（互斥，无 hybrid）

| Strategy | Condition |
|----------|-----------|
| `complete-dialogue` | `utterances.length > 0` |
| **fail-fast** | `utterances.length == 0` |

**fail-fast：** 立即中止 Compose，报告无字幕/转写可整理，**不进入 Archive**。`segments` 有值也不能改走摘要。

---

## Steps C0–C6

### C0 · Fail-fast check

若 `utterances` 为空 → 输出失败原因，停止。不要用 `segments[].summary` 组稿。

### C1 · Apply `fidelity.corrections`

对每条 utterance 的 `text` 应用 `{wrong → correct}` 替换。

**算法（可操作）：**

1. 对每个 `meta.speakers` 中的 canonical 名，在 utterances 合并文本中搜索：长度 ≥2 的中文 token，不在 speakers 列表，且与 canonical 名共享首字或 Levenshtein 距离 ≤2
2. 候选 `wrong` 出现 ≥3 次 → 写入 `{wrong, correct: canonical}`
3. 无候选则 `corrections: []`；**禁止** adapter 硬编码 per-video 表

> adapter Map 阶段可预填 corrections；Compose C1 须再应用/合并。

### C2 · Select strategy

`utterances.length > 0` → `complete-dialogue`。否则 fail-fast。

### C3 · Build navigation skeleton

Keep utterance order. Navigation only:

- **Source chapters present:** insert each `segments[].title` at its `start_sec` as a heading. Do not reorder utterances to fit a theme.
- **No chapters:** insert time-range headings (about 3–8 minutes) as markers. Do not invent topic titles.

### C4 · Assign utterances

Walk utterances by `start_sec`. Each utterance appears **exactly once** after C6 merge/dedupe. Coverage < 100% of non-empty source text → fix before Archive. Before handoff, run `theme-line/scripts/check_dialogue_coverage.py` (see [archive-steps.md](archive-steps.md)).

### C5 · Speaker turns

Follow [speaker-roster.md](speaker-roster.md). Caption-only sources are **not** diarization.

1. Confirm `meta.speakers` has proper names when title or in-text labels give them. If Acquire left `["Host", "Guest"]` but the title has a name pair, rebuild the roster here.
2. Assign a speaker on **each utterance** before C6: `utterance.speaker` → in-text label → Host/Guest roles from the roster.
3. Split overlap / Q→A boundaries as in speaker-roster. Do not leave two people inside one turn.
4. Emit roster names (`{Host Name}:`, `{Guest Name}:`). Use `(uncertain)` only for turns that still cannot be assigned.

### C6 · Light clean & emit

Allowed:

- Merge consecutive caption fragments **only when the assigned speaker is the same** (VTT roll-up / gap < 1.5s)
- Drop consecutive exact duplicates
- Apply C1 corrections
- Merge same-speaker turns that were split only by caption chunking

Forbidden:

- Merge “both unknown” when the roster has two names
- Paraphrase or summarize
- Drop a turn or clause to “tighten”
- Regroup by invented themes
- Replace dialogue with manuscript summaries

---

## Fidelity

- Body is lightly cleaned **source wording**
- Invariant: every non-empty input utterance is retained once after merge/dedupe
- Short quotes are unnecessary; the body *is* the dialogue

---

## Strategy details

### complete-dialogue

- Typical sources: YouTube captions, InfoQ SRT, pasted/local transcript
- Heading = source chapter title or `MM:SS–MM:SS`
- Body = speaker turns in time order

---

## Output

ThemeLine body only（title / metadata / navigation 由 Phase 3 Archive 处理，见 [output-templates.md](output-templates.md)）。

可选 provenance（Archive header）：

```markdown
> 采集：{platform} · complete-dialogue · 嘉宾：{guest} · 说话人：标题与问答推断
```
