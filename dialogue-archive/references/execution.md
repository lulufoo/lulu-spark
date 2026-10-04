# Execution — dialogue-archive

Parent orchestrates Locate → Normalize → Sink.  
Skill contract: [`../SKILL.md`](../SKILL.md).  
MCP: [`archive.md`](archive.md).

---

## Script macros

`$NORMALIZE` is defined in [`../SKILL.md`](../SKILL.md) Script Macros. Do not restate the CLI here.

---

## Parent steps

0. Resolve `sink`: `spark` (default) or `local-md` when user intent refuses Lulu Spark persist. No phrase enumeration — semantic understanding only.
1. If `sink=spark`: confirm Lulu Spark MCP available ([archive.md](archive.md)). If `sink=local-md`: skip MCP check.
2. Resolve current session → jsonl (folder basename = session id). Prefer `<uuid>/<uuid>.jsonl` under `agent-transcripts`.
3. Resolve `start_node` / `end_node` (1-based closed interval over non-empty JSONL lines):
   - User gave numbers → use them.
   - User gave text anchors → `rg` / small window reads; prefer **user** nodes on multi-hit; disambiguate.
   - Default → `1` … last non-empty line.
   - **Forbid** loading entire jsonl into model context.
4. Infer `title` / `project` / `doc-theme` / `slug` / `ts`; choose `--out`:
   - `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`
   - This is a staging path only. Do not treat it as Host `common_path`.
5. Run `$NORMALIZE` with `--transcript` `--start-node` `--end-node` `--out` `--title` (+ optional project/theme/slug). For `local-md`, also pass `--local-md`. Exit ≠ 0 → stop.
6. Phase B:
   - `spark`: load note-task and route Create (`source_type: dialogue`, `content_constraint` when writing digest). **Forbid** `"document"` and Host HTTP.
   - `local-md`: ensure file at `--out`; stop (no MCP).

**Hard:** Parent **MUST NOT** hand-parse jsonl or hand-build `TURN_SEP` bodies for MCP. Use `$NORMALIZE` only.

Paste path: user supplies finished TURN_SEP markdown on disk → skip steps 3–5; Phase B only (still honor `sink`; spark still uses `source_path`).

---

## Content constraint (digest)

Required whenever digest is written. Forms: node/Turn range narrative, one-line focus, or both.  
Must appear in the digest header. Do not re-fetch full raw solely to write digest.

```markdown
> 内容约束：<content_constraint>
```

---

## Done receipt

`spark` — use Host-returned paths, do not invent them:

```text
> ✅ dialogue-archive complete（sink=spark）
> 📄 raw：<raw_path>
> 📋 digest：<digest_path 或已跳过>
> 🧭 nodes：<start>-<end>；约束：<content_constraint 摘要>
```

`local-md`:

```text
> ✅ dialogue-archive complete（sink=local-md，未上传 spark）
> 📄 local：.cache/dialogue-archive/<ts>-<slug>.md
```
