# Core principles (dialogue-summary)

Constitution for Phase A. Operational checks live in [substance-gate.md](substance-gate.md). Finished-body craft lives in [output-shape.md](output-shape.md).

SKILL **controls substance and fidelity**, **not** overall length or “% of dialogue size”.

---

## 0. Acceptance — reconstructable without context (the one test)

The summary passes **iff** a reader can, from the document alone:

- **T1 · Rebuild the key process** — reconstruct how the dialogue moved: the starting problem, the major forks, what was tried / rejected and **why**, and how it converged. The **arc**, not only the endpoint decisions.
- **T2 · Rebuild it context-free** — do so **without the project and without the original dialogue**. This needs two things: (a) the doc **orients** the reader up front — what the system / project is, the problem it addresses, where this change sits; (b) every load-bearing term is **grounded**, so nothing is an unexplained code.

Everything below (coverage, substance, fidelity, altitude, genre, surface form) is a **tactic in service of T1 + T2**. If any rule conflicts with T1 + T2, **T1 + T2 win**.

**Ground the project's own names — do not erase them.** Context-free does **not** mean replacing every project term with a long paraphrase; that bloats the text and forces the reader to build a private mapping. Instead: keep the domain's real handles (`Realign`, `stale`/待更新, the gate names), **introduce each once** with a plain gloss or role, then reuse the short real term. A grounded proper noun is clearer and shorter than a paraphrase repeated everywhere.

| Failure mode | Shape |
|---|---|
| Insider | Bare code as sole carrier, never grounded |
| Over-translation | Every project term erased into long role phrases; reader rebuilds the mapping; text bloats |
| Right | Real term, grounded on first use, then reused as the stable handle |

Pure **implementation identifiers** (field / exit / runner-filename / enum names) still go to the SSOT pointer, not the body.

**Beware dialogue-emergent shorthand (a third kind, distinct from a stable domain term).** Nicknames coined mid-conversation for the options being weighed (`light 模式`, `同一入口`, `carry`, `断点`) and reframes coined in the dialogue (`更新式回退`, `决策树 / 多节点`) are **not** vocabulary to preserve — they only mean something to someone who was in the room. **Unpack each into a self-standing plain description of the idea**; do not cite the nickname as the carrier. Quoting an in-dialogue phrase (“一度倾向「light 模式」”) is **self-talk**, not summary — describe what the option actually proposed and why it was weighed / dropped.

**Self-talk test (T2, apply to every phrase):** take any noun phrase, option label, or coined reframe in the body — could a stranger state what it **concretely means** using only this document? If it parses only for someone who followed the dialogue, **unpack it or cut it**.

**T1 + T2 apply to the spine itself, not only the prose.** A reader who reads **only the chapter titles, top to bottom**, must already be able to state the process arc (T1) and must not need the original dialogue to parse any title (T2).

**The one generative rule for skeleton shape:** a **topic** (flat `##` chapter) earns its **own** place on the spine **iff it carries its own tension → convergence** — i.e. the dialogue shows something *specifically about this topic* that was in play, tried, or rejected, with its own why. Title wording and topic count are **consequences** of this one test, not separate rules:

- Satisfying it **produces** a declarative title that states movement (never a question or bare handle). Full from→to detail belongs in the body opening; the title is a **compressed** statement of that move.
- Satisfying it **produces** correct granularity (independent tension = its own topic; pieces decided **together in one bundle**, with no tension of their own, stay inside that topic as prose or an inline list — do not invent sibling topics that each fake their own "decision + why").

**Title length budget (~20 字):** under scanning pressure, each topic title targets **≤ ~20 Chinese characters** (exclude the `【核心】` mark). Compress the move; do not pack both full poles of a long from→to into the heading. Still must pass T1/T2 on the title list — shortness is not a license for bare handles (“触发与命名”).

Operational form (result tests, not a mechanical checklist): [substance-gate.md](substance-gate.md) §Skeleton, checked at Step 3.5 and Step 6.

---

## 1. Coverage — follows the dialogue

**Coverage** = which topics are written into the document.

- Include topics that advance the mainline, hold shape-making stances, or later steps depend on.
- Drop noise: pure execute-confirm, abandoned probes that do not shape understanding, one-off chores.
- **Do not** invent topics to look complete. **Do not** pad coverage because the transcript was long.

Coverage **may** change with the dialogue. Depth per written topic **must not**.

**Granularity follows from the one generative rule in §0** (own tension → own topic). Topic *count* is a **result**, never a target to minimize or inflate. Do **not** treat "few chapters" as a default virtue, and do **not** split a bundle that was decided together just to hit a smaller unit — see [substance-gate.md](substance-gate.md) §Skeleton for the operational form.

**Cores are weight, not a topic-count cap.** `≤4` bounds how many topics get the *thicker weave*; it does **not** cap how many topics the spine may have, and it must never be used to justify cramming multiple decisions into one of the 4 slots.

---

## 2. Substance — fixed bar, altitude-aware

**Unit substance** = completeness of each **written** topic at the correct **reading altitude**.

- Every included topic must meet the **same** substance bar (see [substance-gate.md](substance-gate.md)), whether the source turn was long or short.
- The skill does **not** target word count, compression ratio, or “shorter is better”.
- Telegram / slogan compression of a topic that **had** supply in dialogue = fail.
- **Over-dump** of internal machinery when a design SSOT already owns it = fail (symmetric to telegram).
- Sparse dialogue that **did not** supply a facet → honest gap (declared), not invented fill — and **not** a substance fail for that facet.

**What counts as substance**

| Facet | Role |
|-------|------|
| **argument** | Why a path won / lost (premise → move → conclusion / defeater). Always body-worthy when discussed. |
| **mechanism** | How a system operates (inputs → action → output). Body-worthy **only** when no design SSOT owns it; otherwise a one-line pointer is full credit. |

Do **not** treat control-plane retell as a substitute for argument.

---

## 3. Fidelity integrate — no padding (constitution)

**Respect the original dialogue. Do not add what was not there. The AI integrates and refines only.**

Allowed: reorder, compress noise, paraphrase, name decisions/rejects/boundaries that the dialogue actually made, mark honest gaps.

Forbidden:

- Invent mechanisms, rejects, boundaries, or outcomes to “look rich”
- Hindsight as if it were known earlier
- Synonym stuffing or multi-pass restatement to inflate length
- Filling empty slots with plausible guesswork
- Fattening with implementation pipeline to clear the substance bar

Unsupplied facet → state that the dialogue did not cover it (or omit quietly if the facet is optional and absence is obvious). **Never** fabricate to clear the substance gate.

---

## 4. Self-contained — one bar, not two axes

A **non-participant** reader who never opens the dialogue should still understand, for each written topic:

- what was decided (or left open),
- why,
- what was rejected or set aside (when discussed),
- what the boundaries are (when discussed).

Those answers must be **present** and **parseable at non-participant altitude**. Facet presence without reachable prose is not self-contained. Operational checks are unified in [substance-gate.md](substance-gate.md) — there is **no** second competing axis.

**Reading altitude:** summary defaults to a non-participant. Internal machinery (fields, exit codes, runner paths, status-enum pipelines) is not default body density. When the dialogue produced a design SSOT, point to it; do not restates the whole machine so facets look “complete”.

---

## 5. Reader & genre

Before composing, fix:

| Input | Default |
|-------|---------|
| **Reader** | Non-participant (zero prior exposure to this dialogue) |
| **Genre** | `G-arg` unless the dialogue produced / updated a design SSOT that owns operational detail → then `G-mech` |

- **`G-arg`:** body carries argument + minimal mechanism needed to understand the decision.
- **`G-mech`:** body carries decisions / why / rejects / boundaries; operational mechanism → SSOT pointer. Spine answers **reader decision questions**, not the design doc’s table of contents.

Genre is declared at Step 0 and shown at Step 3.5. Wrong genre (e.g. `G-mech` body that retells the control plane) → substance fail.

---

## 6. Surface form — no scaffold in the body

Substance checks (slots) are **authoring / Gate only**.

Finished body **must not**:

- expose slot labels as section headings (e.g. “问题/张力”, “失效/代价”, “边界”),
- use one-heading-per-slot fill-in scaffolding,
- read like a checklist dumped into prose.

Body = natural, topic-ordered **summary narrative** (see [output-shape.md](output-shape.md)). Headings name **topics**, not gate slots.

---

## Anti-patterns (principle level)

| Failure | Violates |
|---------|----------|
| Thin telegram of conclusions only | Substance, self-contained |
| Quote anthology / speaker stage | Summary nature, Voice |
| Long dialogue → padded restatement | Coverage, fidelity (padding) |
| Short dialogue → emptied substance | Substance |
| Slot labels visible in body | Surface form |
| Invented “completeness” | Fidelity integrate |
| Facets present but only as unexplained labels | Self-contained |
| Body retells full internal state machine; decision secondary | Self-contained, genre |
| Spine mirrors design-doc TOC instead of decision arc | Coverage, genre |
| §0 lacks orientation (no 背景与定位; only scope/TOC or a glossary) | Self-contained |
| Project terms erased into long paraphrases; reader must build a mapping | Self-contained (over-translation) |
