# Skill Arch Constraints

Applies when: creating or modifying SKILL, `scripts/`, and state data files.

**How to read this doc:** Principles = what SKILL content centers on and how prose is written; Architecture = layer duties; Binding Rules = MUST / MUST NOT; Domain = `lulu-dev-workflow/` only.

---

## Principles

1. **Goal-first: SKILL = tool catalog + cognitive map + thin orchestration**

SKILL is not a step-by-step how-to. Goal is the unit's target state and completion condition — not its Why. The three components are justified by it:

- **Goal precedes components** — each SKILL unit states its goal before any component (the H1 purpose in Skill Writing Constraints).
- **Goal traceability** — every `$MACRO` in the catalog, ruler in the map, and branch in the orchestration contributes to reaching or deciding the goal; otherwise cut it.
- **Decidable completion** — DONE is decided from `$MACRO` stdout or the Return shape, not from model judgment alone.

Placement:

| Concern | Lives in |
|---------|----------|
| **How** | Tools (`scripts/*-control.py`, schema, `--help`) |
| **What** | Tool entry points + session cognition (variables, branches, completion conditions) |
| **Why** | Design docs or script comments — not the install-facing SKILL |

**Thin orchestration keeps:** scenario order and branches; what to ask and when DONE; how session variables bind from tool output.

**Thin orchestration omits:** algorithms, sort/index rules, hand-computing from data files; long subcommand / stdout contracts (point to `--help`); design rationale.

2. **Precise, lean prose** — Prefer the most exact wording at high information density; cut length that adds no new instruction (redundant restatement, hedging filler, same rule in different words). This governs SKILL wording — not script help or schemas.

3. **Unit decomposition** — Split SKILL content into self-contained units, each
   with one responsibility.

4. **Progressive disclosure** — Load only units required by the current state;
   do not expose all SKILL content at entry.

---

## Architecture — Three-Layer Separation

```text
SKILL.md  →  scripts/*.py  →  data files
(orchestration)  (tool layer)    (data layer)
```

| Layer | Responsibility |
|-------|----------------|
| **SKILL** | When to do what; what to do on failure |
| **Script** | State transitions, validation, persistence I/O |
| **Data** | Persistent SSOT — **Script only**; SKILL read-only |

**Invariant:** SKILL orchestrates; Script executes side effects; Data is modified by Script only.

---

## Binding Rules — Script Layer

| Layer | Script | Role | Invoked by |
|-------|--------|------|------------|
| Data | `<prefix>-schema.py` | Schema, validation, and I/O for one data file | `*-control.py` only |
| State machine | `<prefix>-control.py` | Transitions and mechanical side effects | SKILL (CLI) |

1. One data file → one `<prefix>-schema.py`.
2. One state machine → one `<prefix>-control.py`.
3. SKILL **MUST** call `*-control.py`; SKILL **MUST NOT** call `*-schema.py`.
4. Control **MUST** read/write data only via the corresponding `*-schema.py`.

---

## Binding Rules — SKILL Layer

1. **Script Macros**
   - Every script invocation **MUST** be registered in a `## Script Macros` table
     as `$MACRO` → one-line CLI.
   - Register macros in the unit that uses them:
     - Orchestrator SKILL: commands used by that SKILL's own steps.
     - On-demand loaded `references/*.md`: commands used only after that
       reference is loaded.
   - Do not copy a CLI definition between the orchestrator SKILL and a deferred
     reference, or the reverse.
   - Do not invoke a `$MACRO` until its defining table is in context.
   - Nested SKILLs loaded from a unit may register the subset they invoke.

2. **Workflow steps**
   - **MUST** reference `$MACRO` only.
   - **MUST NOT** scatter bare `python3 ...` calls (including session bootstrap —
     register start scripts in the Script Macros table of the unit that runs them).

3. **Contract SSOT**
   - Subcommands and stdout semantics: script module docstring or `--help`.
   - **MUST NOT** duplicate long subcommand lists or stdout rules in the SKILL.

4. **Data access**
   - **MUST NOT** name session data file paths as agent workflow steps or branch conditions.
   - Session state for routing: `$MACRO` stdout and `resolve-context` JSON (`$CTX`) only — not direct Read/Write of data files.

---

## Install Boundary — Self-Containment

1. **Runtime references resolve in-tree** — every runtime reference **MUST** resolve to a shipped artifact: this SKILL's prose, its `references/`, its `scripts/` (`--help` / schema), or an allowed module dependency (see Domain Scope). Long contracts stay out of SKILL prose per Contract SSOT.
2. **Rationale and docs pointers only in scripts** — design-rationale pointers to repo `docs/**` **MUST** reside solely in `scripts/*.py` (docstrings). SKILL `.md` and `references/*.md` **MUST NOT** cite `docs/**` at all (including as runtime Design SSOT or "read during execution") — the `.md` layer ships to installs where `docs/**` is absent.

---

## Domain Scope — lulu-dev-workflow Module Dependencies

Applies when: creating or modifying modules under `lulu-dev-workflow/`.

Only allowlisted edges; everything else forbidden. Cross-stage data: delivered cache only.

`shared` is a references-only leaf library: no `SKILL.md`, scripts, runtime
state, or outgoing module dependencies.

**Allow:**

```text
lulu-bet | lulu-approach                                         →  decision
lulu-arch | lulu-blueprint | lulu-design | lulu-plan | lulu-spec →  compose | eval
compose                                                          →  agenda | eval
decision                                                         →  eval
compose | decision                                               →  shared
```

**Forbid:**

```text
kernel | agenda  ↛  stage-shell
kernel           ↛  kernel   except compose → eval
stage-shell      ↛  stage-shell.scripts
shared           ↛  *
```
