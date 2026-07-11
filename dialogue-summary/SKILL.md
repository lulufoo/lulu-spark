---
name: dialogue-summary
description: >-
  Process retrospective of a dialogue: prune a narrative spine, deepen ≤4 core
  topics, embed shape-making user stances, then archive via Workbench MCP.
  Use when: dialogue-summary、过程回顾、决策回顾、方案怎么定的、dtd_raw_summary、
  总结归档（process form）.
  Not for verbatim turn archive — use dialogue-archive.
  If an older workbench “dialogue-summary” (verbatim) is also installed, prefer
  this skill for 过程回顾 and dialogue-archive for 原文.
---

# dialogue-summary

> **Read this file in full before executing.** Two phases:
> 1. **Phase A — Process retrospective** (Steps 0–6 Gate)
> 2. **Phase B — Save to Archive** (MCP only after Gate; skip if user opts out)
>
> Do **not** start Phase B until Step 3.5 has passed and Step 6 Gate has passed.

**Not** verbatim dialogue (`dialogue-archive`). **Not** a conclusion-only abstract.

**Formula:** pruned spine + core deepening (3 faces) + shape-making stances marked `〔User〕` + dialogue fill → process doc → (optional) archive.

This `SKILL.md` + `references/` are the runtime contract. Do **not** require any `.cache` scheme file to run.

## Writing constraints

Applies to **all** Phase A finished body (and paste-path Gate). SSOT: [references/output-shape.md](references/output-shape.md) — four sections:

| Section | Meaning |
|---------|---------|
| **Lexicon（用语）** | Domain terms as primary; required first-use gloss for opaque terms (§0-exclusive) |
| **Craft（工艺）** | Full substance + scannable process-note structure |
| **Voice（视角）** | One process narrative; shape-making stances marked `〔User〕` |
| **Fidelity（忠实）** | In-scope only; no hindsight invent; honest gaps |

Do not restate the full contract in steps; compose and Gate against that file.

---

## Phase A — Process retrospective

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

### Step 1 — Candidate subtopics (internal)

List issue raw material from in-scope dialogue (short titles; branches allowed).

**Done:** candidate list ≥ 1 covering the main timeline.

### Step 2 — Prune to spine (internal)

Candidates **discover** the narrative; the **spine** is the **pruned** chain — not the full candidate list.

| Keep | Cut | Merge |
|------|-----|-------|
| Advances mainline; later steps depend; holds stances | One-off branch; abandoned **noise**; pure execute-confirm; parallel probe | Tiny consecutive steps → one node |

Detours that **shaped** the mainline: keep as a **short** spine node or one line in 曾议 — do not erase the bend entirely.

**Spine acceptance:** From node titles + one-line “what this segment did” only, a reader can **coherently reconstruct what the dialogue did in order**. If they only learn the final scheme name → fail (became a summary).

**Done:** ordered spine passes acceptance.

### Step 3 — Mark core topics (internal)

Mark **at most 4** cores on the spine (typical 2–4; short sessions may have 1; extremely short may have 0 with all-spine short write).

| Face (EN) | 中文稿用名 | Meaning |
|-----------|------------|---------|
| Make-or-break | **成败关键** | Factual focus; wrong/missing → whole effort skews or fails |
| Load-bearing | **基础承重** | Key decisions / theory / abstract stance later steps rest on |
| Contract & boundary | **契约与边界** | Hard constraints after fork-converge, or needed boundary patches |

Enter core if ≥2 faces, or one face is very strong. Do **not** core: pure execute chores; abandoned probes that do not affect mainline understanding.

**Self-check:** Cores alone answer “why this could succeed / what it rests on”; non-core spine alone still answers “what happened in order.”

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

Only stances that **materially shaped process shape**. Rules: [references/by-user-rules.md](references/by-user-rules.md). Finished body must also satisfy **Writing constraints** (Lexicon / Craft / Voice / Fidelity).

### Step 5 — Compose along spine

Write body per [references/output-shape.md](references/output-shape.md) (Lexicon / Craft / Voice / Fidelity).

- Order = confirmed spine; cut branches → optional “曾议/否决” table, not chapters
- Core nodes: deepen (tension → directions → stance `〔User〕` → process consequence → evidence)
- Non-core nodes: short bridge
- Self-contained prose; paths/commits only in optional appendix (default: omit / independent on)
- Fill from in-scope dialogue only

### Step 6 — Gate → deliver body

Gate:

- [ ] Body follows confirmed spine; cores match 3.5
- [ ] Writing constraints: Lexicon / Craft / Voice / Fidelity ([output-shape.md](references/output-shape.md))
- [ ] Shape-making stances attributed with `〔User〕` (or rare `— User` blockquote); no speaker-staged「追问/回答」; no `〔By User〕`/`因此：` couplets
- [ ] **`〔User〕` coverage:** every shape-making item on the Step 4 internal list carries `〔User〕` in the body **including tables**; no user 定案 silently demoted to subject-less「定案」in condensed/table form
- [ ] Scope declared; reads as process replay, not conclusion abstract

**On pass:**

1. Post the **full retrospective body** in chat (so the user can see it; this is **not** a second confirm gate).
2. If user said 不归档 / no archive → **stop** (Phase A complete).
3. Else → **Phase B** immediately (no waiting for another “ok” unless the user interrupts).

If spine/cores must change after compose → back to Step 2/3 → **3.5 again** → recompose. Do not silently change spine in Phase B.

### Paste path

User-supplied Markdown:

1. Check **spine feel** (all three): ordered process sections; not conclusion-only; scope declared (or addable). Fail → reject or offer regenerate via Steps 0–5.
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
| [references/by-user-rules.md](references/by-user-rules.md) | Shape-making stances; `〔User〕` mark |
| [references/output-shape.md](references/output-shape.md) | Writing constraints: Lexicon / Craft / Voice / Fidelity |
| [references/digest-shape.md](references/digest-shape.md) | Digest short form |
| [references/body-sanitize.md](references/body-sanitize.md) | Strip external images |
| [references/archive.md](references/archive.md) | MCP paths / header |
