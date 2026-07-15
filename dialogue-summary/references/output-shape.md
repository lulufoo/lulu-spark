# Summary body shape

Language of the finished body: match the dialogue.

Global writing constraints for Phase A body (and paste-path Gate). Four sections — keep all four.

Principles: [principles.md](principles.md). Richness checks (Gate-only): [richness-gate.md](richness-gate.md).

---

## Lexicon（用语）

Keep the dialogue’s **domain lexicon** as **primary terms**.

- Use the words the dialogue used (`home`, `i_star`, partition, 包含式匹配, … — or philosophy terms in a philosophy dialogue).
- **First-use gloss (required for opaque terms):** coined compounds / nicknames / bare abbrevs（物化分派, 正主, 归 I）get **one** apposition on first use; plainly shared words（partition, home）none.
- **§0 mutual exclusion:** glossed in §0 → body first use is the bare term (never gloss twice).
- After first use: **continue with the original term**. Do **not** switch the rest of the doc to a parallel everyday thesaurus（主章节 / 分家 / 捞料 一类替代词）.
- **Non-term words follow the doc language:** non-term verbs / connectives take the body’s language（写「外挂」不写 bolt）; keep a foreign word only when it **is** the domain term.

**Anti-pattern:** everyday-term **replacement** of domain lexicon; ad-hoc code-switching on non-term words.

---

## Craft（工艺）

Write a **self-contained summary**: scannable **and** complete at the unit-richness bar — not a telegram, not a transcript dump.

- State the substance **fully** when the dialogue supplied it — mechanism: inputs / action / output / failure mode; argument: premise / move / conclusion / defeater. Not slogans or telegram compression.
- **Richness ≠ length:** meet [richness-gate.md](richness-gate.md) substance; do **not** pad for word count; do **not** thin because the dialogue was short.
- Structure: topic headings along the spine; 【核心】 on cores; lists/tables when they clarify mechanisms, causal chains, 定案 / 否决 / defer.
- Explicit decisions: 定案 / 否决 / defer named as such when the dialogue did.
- **Single statement (去冗余):** state each fact / decision in full **once**, at its owning section; elsewhere reference it by section name / anchor — a one-line reminder is fine, a full restate is not.
- **Granularity:** one decision per sentence or list item; when a sentence carries **≥3** independent 定案, split into a list.
- **Rule + examples:** a rule with **≥2** examples → rule sentence + example list, not a comma/semicolon chain.
- Not literary essay; not watered-down paraphrase that drops substance; not quote anthology.

**Skeleton (topic names — never gate-slot names):**

```text
# <theme>
## 0. 读前说明（范围、骨架、主词表、〔User〕图例）
## 1…N 议题章（核心标【核心】；机制用表/列表充分写开）
## 曾议 / 仍开放（as needed）
```

**No slot scaffold in body:** do **not** use headings or section banners named after richness slots (tension / mechanism / cost / decision / rejected / boundary / open, or 中文等价栏目). Weave that substance into topic prose. Slot IDs stay in [richness-gate.md](richness-gate.md) only.

**§0 purity:** §0 carries reader-orientation only (scope / spine / term list / mark legend). Do **not** restate this skill’s own writing laws (Lexicon / Craft / Voice / Fidelity) or principle essays into the artifact.

**§0 term list = one line per term:** opaque spine/core terms only; one gloss each（`正主 — 命题唯一完整陈述所在的 section`）; no mechanism expansion here.

**Anti-patterns:** slogan / telegram **compression**; literary **essay** dilution; padding / synonym inflation; self-redundancy across sections; tool-rule or **slot-scaffold leakage** into §0 or body; speaker-staged quote dump.

---

## Voice（视角）

**Perspective:** one impersonal **summary narrative**. User turns and assistant turns are **material for the same story**, not two speakers on stage.

- No 我 / 你 / 我们; no 用户 / 助手旁白; no transcript voice（「追问…」「回答…」作主语）.
- Ordinary steps: narrate as what was decided / what the process settled — **do not** stage “who asked”.
- Shape-making user stances / decisions: keep them inside the narrative, and **attribute with the fixed mark** `〔User〕` (see [by-user-rules.md](by-user-rules.md)). Not a separate quote chapter; not `〔By User〕…因此：` couplets.
- Reader should follow topic order from the spine and leave with a **self-contained** understanding — not only the final scheme name.

**Anti-patterns:** person voice; speaker-staged summary; spine-less conclusion telegram; quote anthology as body.

---

## Fidelity（忠实）

Aligns with principle **Fidelity integrate** in [principles.md](principles.md).

- Fill only from **in-scope** dialogue. Out-of-window content is not primary evidence.
- **Integrate and refine only** — do not invent fluent fill; do not smuggle later outcomes into earlier spine nodes (no hindsight as if it were then-known).
- Uncertain / unresolved → say so; prefer an honest gap over a polished guess.
- **No padding:** do not add mechanisms, rejects, or boundaries the dialogue did not supply in order to pass richness.
- **No vague hand-wave:** name landed / decided items concretely; if unknown, mark as a gap — never a phrase that is neither a concrete fact nor a declared gap（如「审查补闸与测试」）.
- Paths, commits, file names: only when the dialogue actually used them (or optional appendix); do not fabricate anchors.

**Anti-patterns:** scope bleed; hindsight rewrite; invented completeness; padding; vague hand-wave.
