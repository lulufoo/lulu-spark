# Execution — dialogue-archive

Parent orchestrates; Phase A defaults to a **Grok sub-agent**.  
Shared spine: [`../../shared/dialogue-execution.md`](../../shared/dialogue-execution.md).  
Clean / render: [`../../shared/transcript-clean.md`](../../shared/transcript-clean.md).

---

## Script macros

`$SKILL_DIR` = `lulu-workbench-skills` install root.

| Macro | Command |
|-------|---------|
| `$TRANSCRIPT_CLEAN` | `python3 "$SKILL_DIR/scripts/transcript-clean-control.py"` |

---

## Parent steps

1. Confirm Workbench MCP available ([archive.md](archive.md)).
2. Resolve current session → jsonl (folder basename = session id).
3. `$TRANSCRIPT_CLEAN from-jsonl --session-id … --jsonl … --out {workspace}/.cache/dialogue-archive-<sid>-clean-raw.json`
4. Dispatch Grok worker (prompt below). Prefer await.
5. Phase B: MCP `archive_document` (`source_type: dialogue`) with worker markdown; then digest per [`../../shared/digest-workflow.md`](../../shared/digest-workflow.md).

**Hard:** Parent/worker **MUST NOT** hand-parse jsonl or hand-build `TURN_SEP` bodies. Use `$TRANSCRIPT_CLEAN` only.

Paste path: user supplies finished TURN_SEP markdown → skip clean + worker; Phase B only.

---

## Worker prompt skeleton

```text
You are the dialogue-archive worker for one session.
Load and follow:
  {SKILL_DIR}/dialogue-archive/SKILL.md
  {SKILL_DIR}/shared/transcript-clean.md
  {SKILL_DIR}/shared/dialogue-execution.md
  {SKILL_DIR}/dialogue-archive/references/archive.md

## Input
SKILL_DIR: {abs}
clean_raw_json: {abs}
workspace_cache: {abs .cache}
archive: defer-to-parent

## Feedstock rule
Use **only** clean_raw_json. Do **not** re-read raw jsonl.
Do **not** rewrite turn `u`/`a` text.

## Procedure
1. Infer title, project, doc-theme (kebab-case), slug, ts (YYYYMMDDHHMM UTC+8).
2. Run:
   python3 "{SKILL_DIR}/scripts/transcript-clean-control.py" to-archive-md \
     --clean-raw "{clean_raw_json}" \
     --out "{workspace_cache}/dialogue-archive-<sid>-archive.md" \
     --title "…" --project "…" --doc-theme "…" --slug "…" --ts "…" \
     --omit-empty-ai
3. Verify stdout: match=true, user_headers == user_turns, sep_ok=true.
4. Do **not** call MCP archive (parent Phase B).

## Done (receipt)
Return:
1. title / project / doc-theme / slug / ts / common_path
2. archive md absolute path
3. turn count
4. Gate: no rewrite of u/a; script-only render
```

---

## Done

Observable: clean-raw written; archive md written via script; MCP returns `id` / `common_path` / `raw_path` (and digest when applicable).
