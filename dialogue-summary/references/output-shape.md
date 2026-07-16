# Summary body shape

Language of the finished body: match the dialogue.

Global writing constraints for Phase A body (and paste-path Gate). Four sections — keep all four.

Principles: [principles.md](principles.md). Substance checks (Gate-only): [substance-gate.md](substance-gate.md).

---

## Lexicon（用语）

Keep the dialogue’s **domain concepts** as **primary terms** when the body needs them.

- Use the words the dialogue used for concepts the reader must track (`home`, `i_star`, partition, Realign, …).
- **First-use gloss (required for opaque terms):** coined compounds / nicknames / bare abbrevs get **one** apposition on first use; plainly shared words none.
- **§0 mutual exclusion:** glossed in §0 → body first use is the bare term (never gloss twice).
- After first use: **continue with the original term**. Do **not** switch the rest of the doc to a parallel everyday thesaurus.
- **Non-term words follow the doc language.**
- **Implementation identifiers are not domain lexicon by default:** field names, exit codes, runner filenames, status-enum dumps belong in a design SSOT pointer — not as the body’s primary vocabulary. Keeping a concept name ≠ retelling the control plane. Density rules: [substance-gate.md](substance-gate.md).
- **Ground project terms; don't erase them (T2):** keep the domain's real handles (`Realign`, `stale`/待更新, gate names like 风险暴露门 R / 交付确认门 DC) but **introduce each once** with a plain gloss/role, then reuse the short term. Do **not** replace every project term with repeated long paraphrases (bloat + reader builds a private mapping). Do **not** leave a code ungrounded as sole carrier. Pure implementation ids (field/exit/enum/runner filenames) → SSOT pointer, not body vocabulary.
- **Unpack dialogue-emergent shorthand, don't quote it (T2):** option nicknames coined mid-talk (`light 模式`, `同一入口`, `carry`, `断点`) and coined reframes (`更新式回退`, `决策树 / 多节点`) only mean something to a participant. Describe **what the idea actually was** in plain terms; do not lift the nickname as the carrier (“一度倾向「light 模式」” = self-talk). The nickname may trail a full plain description at most once. Apply the **self-talk test**: every phrase must decode from the document alone.

**Anti-pattern:** everyday-term **replacement** of needed domain concepts; OR treating every implementation identifier as mandatory body lexicon.

---

## Craft（工艺）

Write a **self-contained summary**: scannable **and** complete at the unit-substance bar — not a telegram, not a transcript dump, not a second design doc.

- State **argument** fully when the dialogue supplied it (premise / move / conclusion / defeater). Not slogans or telegram compression.
- **Decision-first (required):** each topic leads with 定案 / 否决 / 边界 in plain language; then why; internal pipelines are secondary and minimal.
- **Genre-aware mechanism:** under `G-mech`, operational detail → one-line SSOT pointer. Under `G-arg`, include only the minimal mechanism needed to understand the decision. Full state-machine retell → substance fail.
- **Concrete before abstract (required):** when a topic rests on a mechanism/system the reader has no prior exposure to, state the plain-language scenario **before or in the same clause as** any technical label — see [substance-gate.md](substance-gate.md).
- **Substance ≠ length:** meet [substance-gate.md](substance-gate.md); do **not** pad for word count; do **not** thin because the dialogue was short; do **not** fatten by dumping implementation detail.
- Structure: topic headings along the spine; 【核心】 on cores; lists/tables when they clarify **decisions and causal chains**, not when they dump control-flow.
- Explicit decisions: 定案 / 否决 / defer named as such when the dialogue did.
- **Single statement (去冗余):** state each fact / decision in full **once**, at its owning section; elsewhere reference it by section name / anchor — a one-line reminder is fine, a full restate is not.
- **Granularity:** one decision per sentence or list item; when a sentence carries **≥3** independent 定案, split into a list.
- **Rule + examples:** a rule with **≥2** examples → rule sentence + example list, not a comma/semicolon chain.
- Not literary essay; not watered-down paraphrase that drops substance; not quote anthology.

**Skeleton (topic names — never gate-slot names):** every topic exists **iff it carries its own tension→convergence** ([substance-gate.md](substance-gate.md) §Skeleton — the one generative rule). Titles are **compressed declarative move-statements** (target **≤ ~20 字**, exclude `【核心】`); full from→to lives in the body opening.

```text
# <theme>
## 0. 读前说明（范围、读者/体裁、背景与定位、骨架、主词表、〔User〕图例）
## 1. <≤~20字 · 压缩陈述式动向>【核心】
## 2. <≤~20字 · 压缩陈述式动向>【核心】
   (pieces decided together with no tension of their own stay as prose / inline list
    inside this topic — they do NOT become sibling ## chapters)
## 曾议 / 仍开放（as needed）
```

**No slot scaffold in body:** do **not** use headings or section banners named after substance slots (tension / argument / mechanism / cost / decision / rejected / boundary / open, or 中文等价栏目). Weave that substance into topic prose. Slot IDs stay in [substance-gate.md](substance-gate.md) only.

**§0 purity:** §0 carries reader-orientation only (scope / reader+genre / spine / **背景与定位** / term list / mark legend). Do **not** restate this skill’s own writing laws or principle essays into the artifact.

**§0 背景与定位 (required):** first orient the reader — what the system/project is (e.g. what `decision` is), the problem, where this change sits — then the 2–4 facts enough to read on (optionally what to ignore). A scope/TOC line or a term list does **not** replace this. See [substance-gate.md](substance-gate.md).

**§0 term list = one line per term:** opaque spine/core **concepts** only; one gloss each; no mechanism expansion here.

**Anti-patterns:** slogan / telegram **compression**; literary **essay** dilution; padding / synonym inflation; self-redundancy; slot-scaffold leakage; speaker-staged quote dump; **implementation-pipeline body** / second design doc under `G-mech`.

---

## Voice（视角）

**Perspective:** one impersonal **summary narrative**. User turns and assistant turns are **material for the same story**, not two speakers on stage.

- No 我 / 你 / 我们; no 用户 / 助手旁白; no transcript voice（「追问…」「回答…」作主语）.
- Ordinary steps: narrate as what was decided / what the process settled — **do not** stage “who asked”.
- Shape-making user stances / decisions: keep them inside the narrative, and **attribute with the fixed mark** `〔User〕` (see [by-user-rules.md](by-user-rules.md)). Not a separate quote chapter; not `〔By User〕…因此：` couplets.
- Reader should follow topic order from the spine and leave with a **self-contained** understanding — not only the final scheme name.

**Anti-patterns:** person voice; speaker-staged summary; spine-less conclusion telegram; quote anthology as body; **self-talk** — lifting the dialogue's internal shorthand / option-nicknames / coined reframes as if the reader shares them, instead of describing the idea plainly.

---

## Fidelity（忠实）

Aligns with principle **Fidelity integrate** in [principles.md](principles.md).

- Fill only from **in-scope** dialogue. Out-of-window content is not primary evidence.
- **Integrate and refine only** — do not invent fluent fill; do not smuggle later outcomes into earlier spine nodes (no hindsight as if it were then-known).
- Uncertain / unresolved → say so; prefer an honest gap over a polished guess.
- **No padding:** do not add mechanisms, rejects, or boundaries the dialogue did not supply in order to pass substance.
- **No vague hand-wave:** name landed / decided items concretely; if unknown, mark as a gap — never a phrase that is neither a concrete fact nor a declared gap（如「审查补闸与测试」）.
- Paths, commits, file names: only when the dialogue actually used them (or optional appendix); do not fabricate anchors. Prefer SSOT pointer over path dumps.

**Anti-patterns:** scope bleed; hindsight rewrite; invented completeness; padding; vague hand-wave.
