# Execution — dialogue-archive

Parent orchestrates Locate → Normalize → Sink.  
Skill contract: [`../SKILL.md`](../SKILL.md).  
MCP: [`archive.md`](archive.md).

---

## Script macros

`$SKILL_DIR` = `lulu-workbench-skills` install root.  
`$WORKSPACE` = active repo root.

| Macro | Command |
|-------|---------|
| `$NORMALIZE` | `python3 "$WORKSPACE/scripts/dialogue_archive_normalize.py"` |

---

## Parent steps

0. Resolve `sink`: `workbench` (default) or `local-md` when user intent refuses Workbench persist. No phrase enumeration — semantic understanding only.
1. If `sink=workbench`: confirm Workbench MCP available ([archive.md](archive.md)). If `sink=local-md`: skip MCP check.
2. Resolve current session → jsonl (folder basename = session id). Prefer `<uuid>/<uuid>.jsonl` under `agent-transcripts`.
3. Resolve `start_node` / `end_node` (1-based closed interval over non-empty JSONL lines):
   - User gave numbers → use them.
   - User gave text anchors → `rg` / small window reads; prefer **user** nodes on multi-hit; disambiguate.
   - Default → `1` … last non-empty line.
   - **Forbid** loading entire jsonl into model context.
4. Infer `title` / `project` / `doc-theme` / `slug` / `ts`; choose `--out`:
   - `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`
5. Run `$NORMALIZE` with `--transcript` `--start-node` `--end-node` `--out` `--title` (+ optional project/theme/slug). Exit ≠ 0 → stop.
6. Phase B:
   - `workbench`: MCP `archive_document` with **`source_path` only** (`source_type: dialogue`). Then digest under **content_constraint** (header `> 内容约束：…`) per [`../../shared/digest-workflow.md`](../../shared/digest-workflow.md). **Forbid** `"document"`.
   - `local-md`: ensure file at `--out`; stop (no MCP).

**Hard:** Parent **MUST NOT** hand-parse jsonl or hand-build `TURN_SEP` bodies for MCP. Use `$NORMALIZE` only.

Paste path: user supplies finished TURN_SEP markdown on disk → skip steps 3–5; Phase B only (still honor `sink`; workbench still uses `source_path`).

---

## Content constraint (digest)

Required whenever digest is written. Forms: node/Turn range narrative, one-line focus, or both.  
Must appear in digest header. Do not re-fetch full raw solely to write digest.

---

## Done receipt

`workbench`:

```text
> ✅ dialogue-archive complete（sink=workbench）
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或已跳过）
> 🧭 nodes：<start>-<end>；约束：<content_constraint 摘要>
```

`local-md`:

```text
> ✅ dialogue-archive complete（sink=local-md，未上传 workbench）
> 📄 local：.cache/dialogue-archive/<ts>-<slug>.md
```
