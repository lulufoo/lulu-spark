# Execution — clean input + default sub-agent

Runtime orchestration for `dialogue-summary`. Parent agent **orchestrates**; compose work **defaults to a sub-agent**.

Shared spine: [`../../shared/dialogue-execution.md`](../../shared/dialogue-execution.md).  
Clean contract: [`../../shared/transcript-clean.md`](../../shared/transcript-clean.md).

---

## Roles

| Actor | May do | Must not |
|-------|--------|----------|
| **Parent** | resolve session → jsonl; run mechanical clean; fix scope/genre if ambiguous; dispatch worker; show 3.5 if user must confirm in-parent; Phase B archive after Gate | inline Steps 1–6 body craft (except Paste path / user explicitly asks parent to write) |
| **Worker (sub-agent)** | load this skill; read **clean-raw** (not raw jsonl); Steps 1–6; return spine + body (+ Gate) | modify skill files; archive MCP unless parent handed Phase B; invent turns not in clean-raw |

---

## Script macros

`$SKILL_DIR` = `lulu-workbench-skills` install root (Cursor: `~/.cursor/skills/lulu-workbench-skills`).

| Macro | Command |
|-------|---------|
| `$TRANSCRIPT_CLEAN` | `python3 "$SKILL_DIR/scripts/transcript-clean-control.py"` |

---

## Mechanical clean (required before compose)

**Why:** Cursor jsonl carries tool_use, chrome wrappers, process lines. Clean feedstock improves skeleton extraction and saves tokens.

**Default tool** (package SSOT — do not require a separate `lulu-dialogue-distill` checkout):

```bash
$TRANSCRIPT_CLEAN from-jsonl \
  --session-id <SESSION_ID> \
  --jsonl <ABS_JSONL> \
  --out <ABS_OUT_CLEAN_RAW.json>
```

| Variable | Meaning |
|----------|---------|
| `<SESSION_ID>` | Cursor session uuid (folder / jsonl basename), **not** a request-id shortcut alone |
| `<ABS_OUT_CLEAN_RAW.json>` | Eval/default: workspace `.cache/dialogue-summary-<id>-clean-raw.json` |

Optional turn window: `--turn-from N --turn-to N` (inclusive).

**Done when:** clean-raw json exists; parent passes its path to the worker. Refuse compose if clean failed (`chrome_tags_remaining` non-empty or `user_turns == 0`).

Paste path: no jsonl → skip clean; feedstock = pasted Markdown.

---

## Default worker dispatch

| Setting | Default |
|---------|---------|
| Tool | `Task` (platform sub-agent) |
| `subagent_type` | `generalPurpose` |
| **Model** | **Grok** (`cursor-grok-4.5-high-fast` or current platform Grok slug) — **do not ask every time** |
| Sync | Prefer await until worker returns (unless user asked background) |

User need **not** say “用 sub-agent / 用 Grok” each run. Override only when user names another model or asks parent to write inline.

### Worker prompt skeleton

```text
You are the dialogue-summary worker for one session.
Load and follow in full:
  {SKILL_DIR}/dialogue-summary/SKILL.md
  and its references/ (especially principles, substance-gate, output-shape, by-user-rules).

## Input
SKILL_DIR: {abs package root}
clean_raw_json: {abs path from mechanical clean}
scope: full | Turn X～Y | topic filter…
genre_hint: G-arg | G-mech | infer
SSOT_hint: {path if known} | none
write_body_to: {abs .cache or omit → return in final message only}
archive: no | defer-to-parent

## Feedstock rule
Use **only** clean_raw_json (+ user scope notes). Do **not** re-read the raw jsonl for compose.
Do not invent turns.

## Procedure
Phase A Steps 1–6. For eval / “直接看效果”: auto-confirm Step 3.5 after showing it in the receipt.
Chinese body unless user asked otherwise.

## Done (receipt)
Return:
1. Step 3.5 block (verbatim)
2. Full summary body (verbatim) — and write to write_body_to if set
3. Gate checklist (pass/fail one-liners)
4. clean_raw path used + approx turn count
```

---

## Parent scenario (happy path)

1. Resolve jsonl from session id / current chat / user path.
2. Run mechanical clean → clean-raw path.
3. Dispatch Grok worker with prompt above.
4. On receipt: if interactive 3.5 required and not auto-eval → show spine to user; else deliver body.
5. Phase B only if user wants archive and Gate passed.

**Done when:** body delivered (and archived if requested).
