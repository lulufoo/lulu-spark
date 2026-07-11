# Retrospective body shape

Language of the finished body: match the dialogue.

Global writing constraints for Phase A body (and paste-path Gate). Four sections — keep all four.

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

Write like a **good process note**: scannable **and** complete.

- State the substance **fully** — mechanism: inputs / action / output / failure mode; argument: premise / move / conclusion / defeater. Not slogans or telegram compression.
- Structure: spine headings, 【核心】 on cores, short sections; lists/tables when they clarify step shapes, causal chains, 定案 / 否决 / defer.
- Explicit decisions: 定案 / 否决 / defer named as such when the dialogue did.
- **Single statement (去冗余):** state each fact / decision in full **once**, at its owning section; elsewhere reference it by section name / anchor — a one-line reminder is fine, a full restate is not.
- **Granularity:** one decision per sentence or list item; when a sentence carries **≥3** independent 定案, split into a list.
- **Rule + examples:** a rule with **≥2** examples → rule sentence + example list, not a comma/semicolon chain.
- Not literary essay; not watered-down paraphrase that drops substance.

**Skeleton:**

```text
# <theme>
## 0. 读前说明（范围、骨架、主词表、〔User〕图例）
## 1…N 骨架章（核心标【核心】；机制用表/列表充分写开）
## 剪出骨架的分支 / 仍开放
```

**§0 purity:** §0 carries reader-orientation only (scope / spine / term list / mark legend). Do **not** restate this skill’s own writing laws (Lexicon / Craft / Voice / Fidelity) into the artifact.

**§0 term list = one line per term:** opaque spine/core terms only; one gloss each（`正主 — 命题唯一完整陈述所在的 section`）; no mechanism expansion here.

**Anti-patterns:** slogan / telegram **compression**; literary **essay** dilution; self-redundancy across sections; tool-rule leakage into §0.

---

## Voice（视角）

**Perspective:** one impersonal **process narrative**. User turns and assistant turns are **material for the same story**, not two speakers on stage.

- No 我 / 你 / 我们; no 用户 / 助手旁白; no transcript voice（「追问…」「回答…」作主语）.
- Ordinary process steps: narrate as what the process did / what was decided — **do not** stage “who asked”.
- Shape-making user stances / decisions: keep them inside the narrative, and **attribute with the fixed mark** `〔User〕` (see [by-user-rules.md](by-user-rules.md)). Not a separate quote chapter; not `〔By User〕…因此：` couplets.
- Reader should follow process order from the spine, not only learn the final scheme name.

**Anti-patterns:** person voice; speaker-staged summary; spine-less conclusion abstract.

---

## Fidelity（忠实）

- Fill only from **in-scope** dialogue. Out-of-window content is not primary evidence.
- Do not invent fluent fill; do not smuggle later outcomes into earlier spine nodes (no hindsight as if it were then-known).
- Uncertain / unresolved → say so; prefer an honest gap over a polished guess.
- **No vague hand-wave:** name landed / decided items concretely; if unknown, mark as a gap — never a phrase that is neither a concrete fact nor a declared gap（如「审查补闸与测试」）.
- Paths, commits, file names: only when the dialogue actually used them (or optional appendix); do not fabricate anchors.

**Anti-patterns:** scope bleed; hindsight rewrite; invented completeness; vague hand-wave.
