# Dialogue execution (shared)

Parent / worker orchestration for `dialogue-summary` and `dialogue-archive`.

Archive MCP paths: [`archive-concepts.md`](archive-concepts.md).  
Digest writing: [`digest-workflow.md`](digest-workflow.md).  
Legacy clean feedstock (summary): [`transcript-clean.md`](transcript-clean.md).

---

## Roles

| Actor | May do | Must not |
|-------|--------|----------|
| **Parent** | Resolve session → jsonl; locate range; run normalize/clean scripts; Phase B MCP archive | Hand-parse jsonl into TURN_SEP body; pass full `document` to MCP |
| **Worker (sub-agent)** | Skill-specific Phase A; return receipt | Modify skill files; invent turns; MCP archive unless parent handed Phase B |

---

## `archive_document` contract (both skills)

Hard-cut path-only:

```json
{
  "source_path": "<absolute path to .md under allow-list>",
  "source_type": "summary|dialogue|…"
}
```

**Forbid:** `"document": "…"`. Write markdown under `{workspace}/.cache/…` first, then pass `source_path`.

---

## Skill branches

### `dialogue-archive` (verbatim raw)

1. Resolve transcript `.jsonl` and **1-based node range** (`start_node` / `end_node`; AI locates text anchors — script does not fuzzy-search).
2. Run `$NORMALIZE` from `dialogue-archive/SKILL.md` (not hand-assemble body):

```bash
$NORMALIZE \
  --transcript "<abs.jsonl>" \
  --start-node <N> --end-node <M> \
  --out "<abs.md>" --title "<title>" \
  [--project inbox] [--doc-theme dialogue] [--slug <slug>]
```

3. Resolve `sink`: `workbench` (default) → theme-archive Embedded (`source_path` + digest under **内容约束**); `local-md` → keep `.cache` only, no MCP.
4. Legacy `$TRANSCRIPT_CLEAN` / `to-archive-md` is **not** the path for Workbench `archive_document` after the path-only contract.

### `dialogue-summary` (process summary)

1. Session / jsonl path may still use mechanical clean feedstock:

```bash
$TRANSCRIPT_CLEAN from-jsonl \
  --session-id "$SESSION_ID" \
  --jsonl "$ABS_JSONL" \
  --out "$ABS_CLEAN_RAW" \
  --title "$TITLE"
```

Refuse worker dispatch if clean fails (`chrome_tags_remaining` or `user_turns==0`).

2. Worker writes summary markdown to `.cache/…`, then Parent Phase B: theme-archive Embedded (`source_type: summary`, `content_constraint`).

Paste path (either skill): no jsonl → skip normalize/clean; feedstock = pasted Markdown on disk → still path-based sink.

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

## Parent happy path (summary)

1. Resolve jsonl / session.
2. Resolve `sink` when applicable (`workbench` default).
3. Run skill-specific normalize/clean → absolute `.md` path.
4. Phase B: theme-archive Embedded for `workbench`; local `.cache` only for `local-md`.
5. Digest when `[AD-0]` applies — with **内容约束** for dialogue producers (theme-archive `[AR-3]`).

**Done when:** body written to path and sunk per `sink`; parent chat shows receipt / paths only — **no** full-body reprint.
