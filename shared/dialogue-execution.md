# Dialogue execution (shared)

Parent / worker orchestration for `dialogue-summary` and `dialogue-archive`.

Feedstock clean: [`transcript-clean.md`](transcript-clean.md).  
Archive MCP paths: [`archive-concepts.md`](archive-concepts.md).

---

## Roles

| Actor | May do | Must not |
|-------|--------|----------|
| **Parent** | Resolve session → jsonl; run `$TRANSCRIPT_CLEAN`; dispatch worker; Phase B MCP archive | Hand-parse jsonl; inline worker craft (except Paste path / user asks parent to write) |
| **Worker (sub-agent)** | Load skill; read **clean-raw only**; skill-specific Phase A; return receipt | Modify skill files; re-read raw jsonl; invent turns; MCP archive unless parent handed Phase B |

---

## Mechanical clean (required)

Session / jsonl path **must** run clean before worker:

```bash
$TRANSCRIPT_CLEAN from-jsonl \
  --session-id "$SESSION_ID" \
  --jsonl "$ABS_JSONL" \
  --out "$ABS_CLEAN_RAW" \
  --title "$TITLE"
```

Refuse worker dispatch if clean fails (`chrome_tags_remaining` or `user_turns==0`).

Paste path: no jsonl → skip clean; feedstock = pasted Markdown.

---

## Default worker dispatch

| Setting | Default |
|---------|---------|
| Tool | `Task` (platform sub-agent) |
| `subagent_type` | `generalPurpose` |
| **Model** | **Grok** (`cursor-grok-4.5-high-fast` or current platform Grok slug) — do **not** ask every time |
| Sync | Prefer await until worker returns |

User need **not** say “用 sub-agent / 用 Grok” each run.

---

## Skill branches

| Skill | Worker Phase A | Parent Phase B |
|-------|----------------|----------------|
| `dialogue-summary` | Steps 1–6 (spine / Gate / summary body) per that SKILL | `archive_document` `source_type: summary` + digest |
| `dialogue-archive` | Infer title / project / doc-theme / slug / ts; run `$TRANSCRIPT_CLEAN to-archive-md`; **do not rewrite** `u`/`a` | `archive_document` `source_type: dialogue` + digest when ≥2 Turn blocks |

---

## Parent happy path

1. Resolve jsonl from session id / current chat / user path.
2. Confirm Workbench MCP available (archive Phase B).
3. `$TRANSCRIPT_CLEAN from-jsonl` → clean-raw path.
4. Dispatch Grok worker with skill-specific prompt (see each skill `references/execution.md`).
5. On receipt: deliver body / archive markdown path; Phase B MCP if archiving.

**Done when:** body delivered (and archived if requested).
