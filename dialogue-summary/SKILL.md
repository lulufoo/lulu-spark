---
name: dialogue-summary
description: >-
  Self-contained summary of a dialogue: select topic coverage, meet a fixed
  unit-richness bar, integrate without inventing, mark shape-making stances
  〔User〕, then archive via Workbench MCP.
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

**Not** verbatim dialogue (`dialogue-archive`). **Not** a conclusion-only telegram. **Not** a quote anthology.

**Formula:** topic coverage + unit richness + fidelity integrate (`〔User〕` where shape-making) → **self-contained summary** → (optional) archive.

This `SKILL.md` + `references/` are the runtime contract. Do **not** require any `.cache` scheme file to run.

---

## Core principles

SSOT expand: [references/principles.md](references/principles.md). Operational richness checks: [references/richness-gate.md](references/richness-gate.md) (**Gate-only — never as body headings**).

1. **Coverage** — which topics to write follows the dialogue (drop noise; do not invent topics; length of transcript does not force more chapters).
2. **Richness** — each **written** topic meets a **fixed** substance bar; the bar does **not** scale with dialogue length or density; the skill does **not** optimize for short word count.
3. **Fidelity integrate (no padding)** — respect the original dialogue; **do not add what was not there**; AI only integrates and refines. Unsupplied facets → honest gap, never fabricated fill.
4. **Self-contained** — without opening the dialogue, the reader can see what was decided, why, what was rejected (if discussed), and boundaries (if discussed).
5. **Surface form** — richness slots are authoring/Gate only; finished body must **not** expose slot labels or one-slot-one-heading scaffolding.

---

## Writing constraints

Applies to **all** Phase A finished body (and paste-path Gate). SSOT: [references/output-shape.md](references/output-shape.md) — four sections:

| Section | Meaning |
|---------|---------|
| **Lexicon（用语）** | Domain terms as primary; required first-use gloss for opaque terms (§0-exclusive) |
| **Craft（工艺）** | Self-contained summary prose at unit-richness bar; scannable; **no** slot scaffold in body |
| **Voice（视角）** | One summary narrative; shape-making stances marked `〔User〕` |
| **Fidelity（忠实）** | In-scope only; no invent / no padding; honest gaps |

Do not restate the full contract in steps; compose and Gate against that file + principles + richness-gate.

---

## Phase A — Self-contained summary

**Visibility:** Steps 1–3 are **internal** by default (do not flood the user with candidate lists). User-facing stops: Step 0 only if scope is ambiguous; **Step 3.5** (required); Step 6 delivers the body in chat then Phase B (unless opt-out).

### Step 0 — Scope

| Mode | Meaning |
|------|---------|
| Default | Full current session |
| **Turn window** | User `Turn X`–`Turn Y` (inclusive). Parse `Turn12-40` / `turn 12 到 40` / `T12-T40`. Require `X ≤ Y`. Illegal/OOB → ask once; never silently expand. Prefer in-session `TurnN` labels; if ambiguous, ask. If the window lacks enough stance signal → say so and **suggest** widening; do not change scope unilaterally. |
| Topic filter | Optional; default **window first**, then organize inside window |
| Paste | See **Paste path** below |

Declare scope in the final doc header. Out-of-window content is not primary evidence.

**Done:** scope mode (+ interval) fixed.

### Step 1 — Candidate topics (internal)

List issue raw material from in-scope dialogue (short titles; branches allowed).

**Done:** candidate list ≥ 1 covering the main timeline.

### Step 2 — Select coverage / order topics (internal)

Candidates **discover** material; the **spine** is the ordered list of topics **worth writing** — coverage selection, **not** “cut for shortness”.

| Keep | Drop | Merge |
|------|------|-------|
| Advances mainline; later steps depend; holds stances; carries reusable substance | Pure execute-confirm; abandoned **noise** that does not shape understanding; one-off chores | Tiny consecutive steps → one topic |

Detours that **shaped** the mainline: keep as a spine topic or a short 曾议 weave inside a neighbor — do **not** erase the bend, and do **not** strip supplied substance to save length.

**Coverage acceptance:** From topic titles + one-line “what this segment settled”, a reader can see the arc of **what mattered**. If they only learn a final scheme name with no path → fail (telegram). If substance that the dialogue supplied was dropped only to shorten → fail (over-prune).

**Done:** ordered spine passes acceptance.

### Step 3 — Mark core topics (internal)

Mark **at most 4** cores on the spine (typical 2–4; short sessions may have 1; extremely short may have 0).

Cores = **weighted emphasis** (thicker weave), **not** “only these may be rich”. **Every written topic** (core or not) must clear the [richness-gate.md](references/richness-gate.md) minimum when the dialogue supplied substance.

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
Spine:
  1. …
  2. …  [core]
  …
Cores (≤4): title + faces (one line each; use 中文面名 when doc will be Chinese)
```

User confirms / edits spine / edits cores → if edited, revise Step 2/3 and **re-show 3.5**.  
**No** Step 4 / compose / Phase B without 3.5 pass.

### Step 4 — Embed shape-making user stances

Only stances that **materially shaped** the summary’s shape. Rules: [references/by-user-rules.md](references/by-user-rules.md). Finished body must also satisfy **Writing constraints** and **Core principles**.

### Step 5 — Compose summary along spine

Write body per [references/output-shape.md](references/output-shape.md). While writing, satisfy [references/richness-gate.md](references/richness-gate.md) **internally** (do not print slot headings).

- Order = confirmed spine; cut noise only — optional “曾议/否决” weave or short table, not slot chapters
- Core topics: fuller weave of tension → paths → stance `〔User〕` → consequence → evidence
- Non-core topics: may be shorter; **must not** be slogan-only if dialogue supplied mechanism/decision/reject/boundary
- Self-contained prose; paths/commits only in optional appendix (default: omit / independent on)
- Fill from in-scope dialogue only — integrate and refine; **never invent** to look rich

### Step 6 — Gate → deliver body

Gate:

- [ ] Body follows confirmed spine; cores match 3.5
- [ ] [principles.md](references/principles.md): coverage / richness / fidelity-no-padding / self-contained / no slot scaffold in body
- [ ] [richness-gate.md](references/richness-gate.md): every written topic clears minimum + four questions under fidelity
- [ ] Writing constraints: Lexicon / Craft / Voice / Fidelity ([output-shape.md](references/output-shape.md))
- [ ] Shape-making stances attributed with `〔User〕` (or rare `— User` blockquote); no speaker-staged「追问/回答」; no `〔By User〕`/`因此：` couplets
- [ ] **`〔User〕` coverage:** every shape-making item on the Step 4 internal list carries `〔User〕` in the body **including tables**; no user 定案 silently demoted to subject-less「定案」in condensed/table form
- [ ] Scope declared; reads as **self-contained summary**, not conclusion telegram and not transcript replay

**On pass:**

1. Post the **full summary body** in chat (so the user can see it; this is **not** a second confirm gate).
2. If user said 不归档 / no archive → **stop** (Phase A complete).
3. Else → **Phase B** immediately (no waiting for another “ok” unless the user interrupts).

If spine/cores must change after compose → back to Step 2/3 → **3.5 again** → recompose. Do not silently change spine in Phase B.

### Paste path

User-supplied Markdown:

1. Check **summary feel** (all three): ordered topic sections; self-contained substance (not conclusion-only telegram); scope declared (or addable). Fail → reject or offer regenerate via Steps 0–5.
2. Show title + spine outline once for ack (substitutes 3.5 when paste skipped generation).
3. Step 6 Gate → deliver body → Phase B (unless opt-out).

---

## Phase B — Save to Archive

**Only** entry for MCP archive. Conventions: [references/archive.md](references/archive.md). Sanitize: [references/body-sanitize.md](references/body-sanitize.md).

<HARD-GATE mcp="archive">
Workbench App must be running (`workbench-knowledge` MCP). **Do not** write corpus files directly. Unavailable → error and stop (body already delivered in chat).
</HARD-GATE>

1. Infer `project` / `doc-theme` / `slug` / `ts` / `COMMON_PATH` (defaults: `inbox`, archive time UTC+8).
2. Sanitize body; wrap per archive output shape (`来源：dialogue-summary`).
3. MCP `archive_document` with `source_type: "summary"`.
4. If digest threshold met (raw body ≳ 200 chars): write digest per [references/digest-shape.md](references/digest-shape.md); MCP `archive_digest`.

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
| [references/principles.md](references/principles.md) | Coverage / richness / fidelity / self-contained / surface form |
| [references/richness-gate.md](references/richness-gate.md) | Authoring/Gate slot checks — not body outline |
| [references/output-shape.md](references/output-shape.md) | Writing constraints: Lexicon / Craft / Voice / Fidelity |
| [references/by-user-rules.md](references/by-user-rules.md) | Shape-making stances; `〔User〕` mark |
| [references/digest-shape.md](references/digest-shape.md) | Digest short form |
| [references/body-sanitize.md](references/body-sanitize.md) | Strip external images |
| [references/archive.md](references/archive.md) | MCP paths / header |
