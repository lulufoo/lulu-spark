---
name: dialogue-summary
description: >-
  Self-contained summary of a dialogue: select topic coverage, meet a fixed
  substance bar at non-participant altitude, integrate without inventing, mark
  shape-making stances 〔User〕, then archive via Workbench MCP.
  Use when: dialogue-summary、过程回顾、决策回顾、方案怎么定的、dtd_raw_summary、
  总结归档、自包含总结.
  Not for verbatim turn archive — use dialogue-archive.
  If an older workbench “dialogue-summary” (verbatim) is also installed, prefer
  this skill for 总结 / 过程回顾 and dialogue-archive for 原文.
---

# dialogue-summary

> **Read this file in full before executing.** Two phases:
> 1. **Phase A — Self-contained summary** (Steps 0–6 Gate)
> 2. **Phase B — Save to Archive** (MCP only after Gate; skip if user opts out)
>
> Do **not** start Phase B until Step 3.5 has passed and Step 6 Gate has passed.

**Not** verbatim dialogue (`dialogue-archive`). **Not** a conclusion-only telegram. **Not** a quote anthology. **Not** a second design / control-plane document.

**Formula:** clean feedstock → reader+genre → topic coverage (decision arc) + unit substance + fidelity integrate (`〔User〕` where shape-making) → **self-contained summary** → (optional) archive.

**Chat surface:** 3.5 + Done receipt / file path only. **Forbid** pasting the finished summary body into chat after Gate (duplicate of the archive / cache file).

This `SKILL.md` + `references/` are the runtime contract. Do **not** require any `.cache` scheme file to run.

---

## Execution (default)

SSOT: [references/execution.md](references/execution.md).

| Rule | Default |
|------|---------|
| **Who composes** | **Sub-agent worker** — parent orchestrates only (resolve session, clean, dispatch, archive). Do **not** inline Phase A craft unless Paste path or user explicitly asks parent to write. |
| **Worker model** | **Grok** (`cursor-grok-4.5-high-fast` or current platform Grok slug). Do **not** re-ask every run. |
| **Feedstock** | Mechanical **clean-raw** from session jsonl **before** Steps 1–6 via `$SKILL_DIR/scripts/transcript-clean-control.py` ([`../shared/transcript-clean.md`](../shared/transcript-clean.md); [execution.md](references/execution.md)). Compose from clean-raw, **not** raw jsonl. |

Parent happy path: resolve jsonl → clean → dispatch Grok worker → deliver receipt / Phase B.

---

## Core principles

SSOT expand: [references/principles.md](references/principles.md). Operational checks: [references/substance-gate.md](references/substance-gate.md) (**Gate-only — never as body headings**).

1. **Coverage** — which topics to write follows the dialogue (drop noise; do not invent topics; length of transcript does not force more chapters); topic shape (title + count) is a **consequence** of one test — own tension→convergence, or not — so topic count is a byproduct, never minimized or inflated on purpose.
2. **Substance** — each **written** topic meets a **fixed** bar at the correct reading altitude; argument is always body-worthy; mechanism is pointer-or-minimal by genre; the skill does **not** optimize for short word count.
3. **Fidelity integrate (no padding)** — respect the original dialogue; **do not add what was not there**; AI only integrates and refines. Unsupplied facets → honest gap, never fabricated fill.
4. **Self-contained** — without opening the dialogue, a **non-participant** can see what was decided, why, what was rejected (if discussed), and boundaries — present **and** parseable (one bar; see substance-gate.md).
5. **Reader & genre** — default reader = non-participant; genre `G-arg` or `G-mech` (SSOT owns ops) fixed at Step 0.
6. **Surface form** — substance slots are authoring/Gate only; finished body must **not** expose slot labels or one-slot-one-heading scaffolding.

---

## Writing constraints

Applies to **all** Phase A finished body (and paste-path Gate). SSOT: [references/output-shape.md](references/output-shape.md) — four sections:

| Section | Meaning |
|---------|---------|
| **Lexicon（用语）** | Domain **concepts** as primary when needed; first-use gloss; implementation IDs not default body lexicon |
| **Craft（工艺）** | Self-contained; decision-first; 背景与定位; genre-aware mechanism; **no** slot scaffold |
| **Voice（视角）** | One summary narrative; shape-making stances marked `〔User〕` |
| **Fidelity（忠实）** | In-scope only; no invent / no padding; honest gaps |

Do not restate the full contract in steps; compose and Gate against that file + principles + substance-gate.

---

## Phase A — Self-contained summary

**Visibility:** Steps 1–3 are **internal** by default (do not flood the user with candidate lists). User-facing stops: Step 0 only if scope/genre is ambiguous; **Step 3.5** (required); Step 6 is Gate + receipt only — **never** re-print the finished summary in chat.

**Who runs Phase A:** by default the **Grok worker** after parent clean ([execution.md](references/execution.md)). Steps below are the worker contract (also used if parent was told to write inline).

### Step 0 — Scope, reader, genre

| Mode | Meaning |
|------|---------|
| Default | Full current session |
| **Turn window** | User `Turn X`–`Turn Y` (inclusive). Parse `Turn12-40` / `turn 12 到 40` / `T12-T40`. Require `X ≤ Y`. Illegal/OOB → ask once; never silently expand. Prefer in-session `TurnN` labels; if ambiguous, ask. If the window lacks enough stance signal → say so and **suggest** widening; do not change scope unilaterally. |
| Topic filter | Optional; default **window first**, then organize inside window |
| Paste | See **Paste path** below |

**Reader (default):** a non-participant with **zero project context and no access to the original dialogue** (archive-grade). The doc must let them **rebuild the key process (T1)** and understand it **standalone (T2)** — see [principles.md](references/principles.md) §0.

**Genre (required, internal then shown at 3.5):**

| Genre | When |
|-------|------|
| `G-arg` | Default — argument/decision is the body payload; minimal mechanism only as needed |
| `G-mech` | Dialogue produced or updated a **design SSOT** that owns operational detail |

Under `G-mech`, summary must not second-write that SSOT. Declare scope (+ interval) and genre in the final doc header / §0. Out-of-window content is not primary evidence.

**Done:** scope mode (+ interval), reader, and genre fixed.

### Step 0.5 — Feedstock (clean-raw)

**Parent** runs mechanical clean (execution.md) and passes `clean_raw_json` to the worker. **Worker** loads that file as the sole dialogue feedstock for Steps 1–6.

| Mode | Feedstock |
|------|-----------|
| Session / jsonl | `clean-raw.json` from `$TRANSCRIPT_CLEAN from-jsonl` |
| Paste | User Markdown (no clean step) |

**Done when:** feedstock path/content available; worker does **not** open raw jsonl for compose.

### Step 1 — Candidate topics (internal)

List issue raw material from in-scope **clean** dialogue (short titles; branches allowed).

**Done:** candidate list ≥ 1 covering the main timeline.

### Step 2 — Select coverage / order topics (internal)

Candidates **discover** material; the **spine** is the ordered list of topics **worth writing** — coverage selection, **not** “cut for shortness”.

Order the spine as a **reader decision arc** when the material allows:

1. What problem was in play
2. What was chosen and why it won
3. What was rejected / deferred and why
4. Boundaries and still-open items

| Keep | Drop | Merge |
|------|------|-------|
| Advances mainline; later steps depend; holds stances; carries reusable substance | Pure execute-confirm; abandoned **noise** that does not shape understanding; one-off chores | Tiny consecutive steps → one topic |

Detours that **shaped** the mainline: keep as a spine topic or a short 曾议 weave inside a neighbor — do **not** erase the bend, and do **not** strip supplied substance to save length.

**Anti-mirror gate (hard):** spine titles **must not** mirror a design-doc table of contents (e.g. “机制定案 / 触发与命名 / 落地与审查” as a pipeline of design sections). If candidates look like design TOC, **re-chunk** into reader decision questions. Design SSOT detail stays behind a pointer under `G-mech`.

**One generative rule for topic shape (hard):** a topic earns its **own** place on the spine **iff it carries its own tension → convergence** — something specifically about it was in play / tried / rejected, with its own why. Title wording and topic count are **consequences** of this one test — see [substance-gate.md](references/substance-gate.md) §Skeleton (including the failure mode of splitting a bundle into sibling topics that don't each have their own tension).

**Title length (~20 字):** each topic title targets **≤ ~20 Chinese characters** (exclude `【核心】`) — compressed declarative move, not a full from→to essay in the heading. Full contrast belongs in the body opening. Shortness must still clear T1/T2 (bare handles remain fail).

**Coverage acceptance:** From topic titles + one-line “what this segment settled”, a reader can see the arc of **what mattered** and rebuild **how it moved** (forks + pivots), not only the endpoints (**T1**). If they only learn a final scheme name with no path → fail (telegram / T1). If substance that the dialogue supplied was dropped only to shorten → fail (over-prune). If the spine is a second design outline → fail (anti-mirror). If a topic lacks its own tension (bundle, question, or bare handle) → fail (skeleton checks). If a title is a long from→to essay (>~20 字) → fail (length budget).

**Done:** ordered spine passes acceptance + anti-mirror + the generative topic-shape test.

### Step 3 — Mark core topics (internal)

Mark **at most 4** cores on the spine (typical 2–4; short sessions may have 1; extremely short may have 0).

Cores = **weighted emphasis** (thicker weave), **not** “only these may be rich” and **not** a topic-count cap — never use the `≤4` limit to justify cramming multiple decisions into one core (that breaks the topic-shape rule, Step 2). **Every written topic** (core or not) must clear the [substance-gate.md](references/substance-gate.md) minimum when the dialogue supplied substance.

| Face (EN) | 中文稿用名 | Meaning |
|-----------|------------|---------|
| Make-or-break | **成败关键** | Factual focus; wrong/missing → whole effort skews or fails |
| Load-bearing | **基础承重** | Key decisions / theory / abstract stance later steps rest on |
| Contract & boundary | **契约与边界** | Hard constraints after fork-converge, or needed boundary patches |

Enter core if ≥2 faces, or one face is very strong. Do **not** core: pure execute chores; abandoned probes that do not affect mainline understanding.

**Self-check:** Cores alone answer “why this could succeed / what it rests on”; full spine still answers “what mattered in order.”

**Done:** cores marked with faces. Do **not** draft the full body yet.

### Step 3.5 — Spine + core overview (HARD STOP · only user confirm)

**After Step 3 only.** Show:

```text
Scope: full | Turn X～Y
Reader: non-participant
Genre: G-arg | G-mech
  (if G-mech: SSOT = <path or title from dialogue>)
Spine (each topic's own tension → convergence; titles ≤~20 字):
  1. <≤~20字 · compressed declarative move>
  2. <≤~20字 · compressed declarative move>  [core]
  …
Cores (≤4): title + faces (one line each; use 中文面名 when doc will be Chinese)
Altitude: decision-first; mechanism = minimal | SSOT pointer
Skeleton self-check: titles alone rebuild the process arc (T1); each topic has its own tension→convergence (not fake siblings); each title ≤~20 字 (T2 scan pressure)
```

User confirms / edits scope / genre / spine / cores → if edited, revise Step 0/2/3 and **re-show 3.5**.  
**No** Step 4 / compose / Phase B without 3.5 pass — including the skeleton self-check line above.

### Step 4 — Embed shape-making user stances

Only stances that **materially shaped** the summary’s shape. Rules: [references/by-user-rules.md](references/by-user-rules.md). Finished body must also satisfy **Writing constraints** and **Core principles**.

### Step 5 — Compose summary along spine

Write body per [references/output-shape.md](references/output-shape.md). While writing, satisfy [references/substance-gate.md](references/substance-gate.md) **internally** (do not print slot headings).

- Order = confirmed spine; cut noise only — optional “曾议/否决” weave or short table, not slot chapters
- Core topics: fuller weave of tension → paths → stance `〔User〕` → consequence → evidence
- Non-core topics: may be shorter; **must not** be slogan-only if dialogue supplied argument/decision/reject/boundary
- **Substance (full file):** §0 背景与定位（项目是什么/解决什么/处于何位）+ reader/genre; decision-first per topic; altitude on whole body; under `G-mech` mechanism = SSOT pointer (do not retell control plane)
- Self-contained prose; paths/commits only if needed (default: omit / appendix)
- Fill from in-scope dialogue only — integrate and refine; **never invent** to look complete; **never fatten** with implementation pipeline

### Step 6 — Gate → receipt (no body reprint)

Gate (T1 + T2 are the top acceptance; the rest only serve them):

- [ ] **T1 · process rebuild:** from the doc alone a reader reconstructs the key process — problem → forks → tried/rejected + why → convergence — not just endpoint decisions
- [ ] **T2 · context-free rebuild:** §0 orients (what the project is / the problem / where this sits); project terms are **kept but grounded once** then reused (not erased into long paraphrases, not left as ungrounded codes); dialogue-emergent shorthand / option-nicknames / coined reframes are **unpacked** into plain ideas, not quoted (self-talk test: every phrase decodes from the doc alone); pure implementation ids → SSOT pointer
- [ ] **Skeleton (T1+T2 on titles):** every topic has **its own** tension→convergence — not a shared bundle split into fake sibling topics, not a question, not a bare handle; each title ≤~20 字 (exclude `【核心】`); skeleton-alone process-rebuild test and single-topic test both pass ([substance-gate.md](references/substance-gate.md) §Skeleton)
- [ ] Body follows confirmed spine; cores match 3.5; genre matches 3.5
- [ ] [principles.md](references/principles.md): coverage / substance / fidelity-no-padding / self-contained / reader-genre / no slot scaffold in body
- [ ] [substance-gate.md](references/substance-gate.md): every written topic clears minimum + four questions; altitude checks; no telegram; no over-dump
- [ ] Writing constraints: Lexicon / Craft / Voice / Fidelity ([output-shape.md](references/output-shape.md))
- [ ] Shape-making stances attributed with `〔User〕` (or rare `— User` blockquote); no speaker-staged「追问/回答」; no `〔By User〕`/`因此：` couplets
- [ ] **`〔User〕` coverage:** every shape-making item on the Step 4 internal list carries `〔User〕` in the body **including tables**; no user 定案 silently demoted to subject-less「定案」in condensed/table form
- [ ] Scope declared; reads as **self-contained summary**, not conclusion telegram, not transcript replay, not second design doc

**On pass:**

1. **Do not** paste the finished summary body into chat (3.5 already showed the spine; reprint is duplicate).
2. Write the body to `.cache/dialogue-summary/<ts>-<slug>.md` (or `write_body_to` if set).
3. If user said 不归档 / no archive → chat shows **only** the cache path; **stop** (Phase A complete).
4. Else → **Phase B** immediately (no waiting for another “ok” unless the user interrupts). Chat shows **only** the Done receipt (paths). The reader opens `raw/` / the cache file.

If spine/cores/genre must change after compose → back to Step 0/2/3 → **3.5 again** → recompose. Do not silently change spine in Phase B.

### Paste path

User-supplied Markdown:

1. Check **summary feel** (all three): ordered topic sections; self-contained substance (not conclusion-only telegram; not second design doc); scope declared (or addable). Fail → reject or offer regenerate via Steps 0–5.
2. Show title + spine outline + genre once for ack (substitutes 3.5 when paste skipped generation).
3. Step 6 Gate → Phase B (unless opt-out). Chat: receipt / path only — no body reprint.

---

## Phase B — Save to Archive

**Only** entry for MCP archive. Conventions: [references/archive.md](references/archive.md). Sanitize: [references/body-sanitize.md](references/body-sanitize.md).

<HARD-GATE mcp="archive">
Workbench App must be running (`workbench-knowledge` MCP). **Do not** write corpus files directly. Unavailable → error and stop (body already on the cache path; still do not dump it in chat).
</HARD-GATE>

1. Infer `project` / `doc-theme` / `slug` / `ts` / `COMMON_PATH` (defaults: `inbox`, archive time UTC+8).
2. Sanitize body; wrap per archive output shape (`来源：dialogue-summary`).
3. Write summary markdown under `.cache/`, then load [`theme-archive`](../theme-archive/SKILL.md) Embedded (`source_type: "summary"`, `content_constraint`). Do **not** call MCP here.
4. theme-archive writes digest when the threshold is met.

Done output:

```text
> ✅ dialogue-summary complete
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH> (or skipped)
```

Post-archive fixes: new run or `archive_digest` with `force` after user confirm — do not silently rewrite spine.

---

## References

| Doc | Purpose |
|-----|---------|
| [references/execution.md](references/execution.md) | Clean feedstock + default Grok sub-agent orchestration |
| [references/principles.md](references/principles.md) | Coverage / substance / fidelity / self-contained / reader-genre / surface form |
| [references/substance-gate.md](references/substance-gate.md) | Authoring/Gate substance + altitude — not body outline |
| [references/output-shape.md](references/output-shape.md) | Writing constraints: Lexicon / Craft / Voice / Fidelity |
| [references/by-user-rules.md](references/by-user-rules.md) | Shape-making stances; `〔User〕` mark |
| [references/digest-shape.md](references/digest-shape.md) | Digest short form |
| [references/body-sanitize.md](references/body-sanitize.md) | Strip external images |
| [references/archive.md](references/archive.md) | MCP paths / header |
