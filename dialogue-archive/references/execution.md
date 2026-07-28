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

0. Resolve `sink`: `workbench` (default) or `local-md` when user intent refuses Workbench persist. No phrase enumeration — semantic understanding only.
1. If `sink=workbench`: confirm Workbench MCP available ([archive.md](archive.md)). If `sink=local-md`: skip MCP check.
2. Resolve current session → jsonl (folder basename = session id).
3. `$TRANSCRIPT_CLEAN from-jsonl --session-id … --jsonl … --out {workspace}/.cache/dialogue-archive-<sid>-clean-raw.json`
4. Dispatch Grok worker (prompt below; pass resolved `sink`). Prefer await.
5. Phase B:
   - `workbench`: MCP `archive_document` (`source_type: dialogue`) with worker markdown; then digest per [`../../shared/digest-workflow.md`](../../shared/digest-workflow.md).
   - `local-md`: ensure file at `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`; stop (no MCP).

**Hard:** Parent/worker **MUST NOT** hand-parse jsonl or hand-build `TURN_SEP` bodies. Use `$TRANSCRIPT_CLEAN` only.

Paste path: user supplies finished TURN_SEP markdown → skip clean + worker; Phase B only (still honor `sink`).

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
sink: {workbench|local-md}
archive: defer-to-parent

## Feedstock rule
Use **only** clean_raw_json. Do **not** re-read raw jsonl.
Do **not** rewrite turn `u`/`a` text.

## Procedure
1. Infer title, project, doc-theme (kebab-case), slug, ts (YYYYMMDDHHMM UTC+8).
2. Choose --out:
   - sink=workbench → "{workspace_cache}/dialogue-archive-<sid>-archive.md"
   - sink=local-md → "{workspace_cache}/dialogue-archive/<ts>-<slug>.md"
3. Run:
   python3 "{SKILL_DIR}/scripts/transcript-clean-control.py" to-archive-md \
     --clean-raw "{clean_raw_json}" \
     --out "<chosen out>" \
     --title "…" --project "…" --doc-theme "…" --slug "…" --ts "…" \
     --omit-empty-ai
     # when sink=local-md, also: --omit-digest-nav
4. Verify stdout: match=true, user_headers == user_turns, sep_ok=true.
5. Do **not** call MCP archive (parent Phase B).

## Done (receipt)
Return:
1. title / project / doc-theme / slug / ts / common_path / sink
2. archive md absolute path
3. turn count
4. Gate: no rewrite of u/a; script-only render
```

---

## Done

- `workbench`: clean-raw written; archive md written via script; MCP returns `id` / `common_path` / `raw_path` (and digest when applicable).
- `local-md`: clean-raw written; archive md at `.cache/dialogue-archive/<ts>-<slug>.md`; no MCP.
