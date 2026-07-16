# Substance gate (authoring / Gate only)

**Not** a body outline. Use while composing and at Step 6. Do **not** copy slot names into finished headings.

Principles SSOT: [principles.md](principles.md).

This file is the **single** operational bar for Phase A body quality. It replaces the old split of “richness vs grounding”.

**Top acceptance (from [principles.md](principles.md) §0):** the whole body must pass

- **T1 — process rebuild:** a reader reconstructs problem → forks → tried/rejected+why → convergence (the arc, not just endpoints).
- **T2 — context-free rebuild:** understandable with **no** project context and **no** original dialogue; no internal code as sole carrier; glossary is not a substitute.

Every check below serves T1 + T2. If a check ever conflicts with them, T1 + T2 win.

---

## Per written topic — internal checklist

For each topic on the confirmed spine, land the substance below **when the dialogue supplied it**. Weave into narrative; do not label as slots.

| Slot (internal id) | Substance to land |
|--------------------|-------------------|
| tension | What problem / fork / dissatisfaction was in play |
| argument | How the case moved: premise → move → conclusion / defeater (**not** a control-plane walkthrough) |
| mechanism | How it operates: inputs → action → output / failure mode — **only** when no design SSOT owns it; if SSOT exists, a **one-line pointer** clears this slot |
| cost_or_failure | Why the old path fails, or what the chosen path costs / risks |
| decision | What was decided (定案 / defer named when the dialogue did) |
| rejected_or_aside | Rejected paths or shaped detours (曾议/否决) when discussed |
| boundary | What the choice guarantees / does not guarantee; overclaim limits |
| open | Still open items; if none in-window, say so or omit without inventing |

**Minimum line:** a written topic with dialogue supply that only states a final name / slogan without `argument` (when argument was discussed) or without decision/rejected when those were discussed → **fail Gate**.

**Mechanism credit:**

| Situation | Clears `mechanism` |
|-----------|-------------------|
| No design SSOT in dialogue | Minimal operational sketch in body (enough to understand the decision) |
| Design SSOT produced / cited | One-line pointer to that SSOT — **full credit** |
| Body retells status enums / exit codes / runner pipelines while SSOT exists | **over-dump fail** (does **not** earn extra substance) |

Cores (≤4 weighted topics): prefer fuller weave of the above. Non-core written topics: may be shorter, but **same minimum** when supply exists — not slogan-only bridges.

---

## Self-contained four questions (per topic)

Without opening the dialogue, can the reader answer:

1. What was decided (or left open)?
2. Why?
3. What was rejected or set aside (if discussed)?
4. What are the boundaries (if discussed)?

Missing answer + dialogue had supply → thicken. Missing + no supply → honest gap / omit. **Do not** invent answers to pass.

These questions check **facet presence**. The altitude checks below check **parseability**. Both required.

---

## Altitude checks (whole body)

### 1. Background & positioning (§0)

§0 **must** open by **orienting** the reader (orientation, not a scope/TOC line):

- **What the system / project is** — e.g. what `decision` is (a diagnostic decision workflow), in one plain sentence.
- **The problem being addressed** and **where this change sits** in that project.
- The 2–4 facts that are **enough** to read the body; optionally, what the reader may **ignore**.

A scope line (“问题切分 / 方案收敛 / 触发命名 / 审查补丁”) is a table of contents, **not** background — on its own it reads as “莫名其妙”. A term list alone is not orientation either.

Body chapters **must not** introduce a new layer outside this orientation without expanding §0 (or failing Gate).

### 2. Decision-first (every topic)

Prose order per topic:

1. **定案 / 否决 / 边界** in plain language
2. **argument** (why) in plain language
3. Only then, if still needed and allowed by genre, a **minimal** mechanism sketch — or SSOT pointer under `G-mech`

If a chapter’s spine is a numbered pipeline of internal steps/status transitions, and the decision is buried inside or after that pipeline → **fail**.

### 3. Density (whole body)

Count **ungrounded internal names** in each paragraph: status/field/exit/runner identifiers and process codes a non-participant would not know without this dialogue.

| Fail if |
|---------|
| Any paragraph chains **≥3** such internal names with no intervening plain-language clause |
| Any chapter after §0 relies on the §0 glossary as the *only* explanation for a multi-step internal pipeline |
| `G-mech` body retells the control plane instead of pointing to SSOT |

Cold-read test (whole chapter): assuming zero prior exposure, can the reader state the decision and why **without** reconstructing an internal state machine? If they must reconstruct the machine → **fail**.

### 4. Concrete before abstract

First mention of an unfamiliar concept needs a plain “what happens / what it means” clause before or in the same breath as the label. A Lexicon apposition is a definition, not that scenario.

### 5. Ground terms, don't erase them (T2 — context-free)

Two opposite failures both break T2:

- **Insider:** an ungrounded code (`G9`, `payload`, `reopen_gate`) carries meaning the reader can't rebuild.
- **Over-translation:** every project term is replaced by a long role paraphrase, so the body bloats and the reader must maintain a private mapping.

Correct handling by term kind (**three** kinds):

| Term kind | Handling |
|---|---|
| Stable domain term (`Realign`, `stale`/待更新, gate names like 风险暴露门 R / 交付确认门 DC) | **Keep the real name**; ground it once (real term + one plain gloss or role); then reuse the short handle |
| **Dialogue-emergent shorthand** — option nicknames (`light 模式`, `同一入口`, `carry`, `断点`) or coined reframes (`更新式回退`, `决策树 / 多节点`) | **Unpack into a self-standing plain description of the idea**; do **not** preserve the nickname as carrier; nickname may trail the description at most once |
| Pure implementation id (field / exit / enum / runner filename) | SSOT pointer; not body vocabulary |

- Ground on **first use** (where the term first does real work), then reuse the stable term — do not re-paraphrase it every time.
- **Self-talk test:** every phrase must be decodable from the document alone. “一度倾向「X」” where X is an in-dialogue nickname → rewrite as *what the option proposed* + *why weighed/dropped*.

| Fail if |
|---------|
| A load-bearing term is an ungrounded code the reader can't rebuild (insider) |
| A project term the reader would benefit from is erased into repeated long paraphrases (over-translation) |
| An option nickname / coined reframe is cited as a label without unpacking the idea (self-talk) |
| Any phrase parses only for someone who followed the original dialogue |

### 6. Process reconstruction (T1)

The body must let a context-free reader narrate **how it moved**, not just what was settled:

- Each core topic shows the **fork** (what was in tension), the **paths tried**, **why** the losing ones lost, and how it **converged** — in causal order.
- Endpoints-only prose (“定案：X〔User〕”) with no visible path → **fail T1**, even if the decision is correct.

---

## Skeleton checks (T1 + T2 applied to the spine itself)

The **topic titles** (flat `##` chapters), read alone, are also subject to T1 + T2 — not only the prose under them. This is checked at **Step 3.5** (before compose) and **Step 6** (after).

### The one generative rule

A **topic** earns its **own** place on the spine **iff it carries its own tension → convergence** — the dialogue shows something *specifically about this topic* that was in play / tried / rejected, with its own why. Ask this **one** question per candidate topic; title wording and count follow from the answer.

- **Has its own tension** → give it its own topic. Its title is a **compressed declarative statement of the move** (never a bare question or a handle with nothing to point at). Spell the full from→to in the body opening, not in the heading.
- **Shares its tension with neighbors** (the dialogue decided it **as one bundle**, no independent fork for this piece alone) → it does **not** get its own topic. Keep it inside the owning topic as prose or an inline list — do **not** manufacture a fake sibling topic that restates the *same* motivation with nothing of its own.

### Title length (~20 字)

Target **≤ ~20 Chinese characters** per topic title (exclude `【核心】`). Prefer a short move-statement over packing both poles of a long contrast into the heading.

| Too long | Target |
|---|---|
| 触发从「只有冲突才回退」放宽到「上游一变就对齐」，机制改名 Realign | 触发放宽为上游变更，改名 Realign |
| 下游更新从「中央统一重算 / 销毁重走」改为「每门进门自己更新」 | 下游改为每门进门自更新 |

Shortness must still clear T1/T2 — “触发与命名” / “机制定案” remain fail (bare handle / design-TOC).

### Two required Gate tests (results-based — the check, not a mechanical rule)

- [ ] **Skeleton-alone process-rebuild test:** reading **only** the topic-title list top-to-bottom, a reader states the process arc — problem → forks → why → convergence — without opening the body prose.
- [ ] **Single-topic test:** every title, read alone, names **one** self-decodable tension→convergence — not a bundle wearing a single label, not a bare code, not a question; and fits the ~20-字 budget.

Fail either → **revise the spine** (Step 2/3, re-show 3.5) before composing prose. Common failure the other way: a bundle was split into sibling topics that don't individually pass "own tension" — merge them back into one topic with an inline list.

---

## Sparse vs dense source

| Source shape | Correct behavior |
|--------------|------------------|
| Long, sparse | Few topics; each still meets bar; no padding |
| Short, dense | Few or many topics as coverage requires; each still meets bar; no telegram |
| Facet never discussed | Honest gap or omit — **not** invented fill |
| Design SSOT exists | Mechanism via pointer; do not second-write the design doc |

---

## Anti-patterns

| Failure | Shape |
|---------|--------|
| Telegram | Final names / slogans only; argument missing though discussed |
| Over-dump | Full state-machine / control-plane retell when SSOT exists |
| Glossary-as-grounding | §0 term list + body that is a label pipeline |
| Opening-only grounding | First sentence concrete; rest is status/exit/runner dump |
| Design-TOC spine | Chapter titles mirror design-doc sections instead of reader decision questions |
| Padding for bar | Implementation pipeline invented or restated only to look complete |

---

## Done criterion

Substance gate passes when every written spine topic clears the minimum line + four questions under fidelity, altitude checks hold for the whole body, **skeleton checks hold for the title list itself** (every topic has its own tension→convergence, both Gate tests pass), genre rules for `mechanism` are respected, and the body shows no slot scaffold — **and, above all, the whole body (prose and skeleton alike) passes T1 (process rebuild) and T2 (context-free rebuild).** T1 + T2 are the final acceptance; every other check only serves them.
