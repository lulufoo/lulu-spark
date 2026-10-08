# Execution — dialogue-archive

Parent orchestrates Locate → Normalize → Write.  
Skill contract: [`../SKILL.md`](../SKILL.md).  
Path and header: [`archive.md`](archive.md).

---

## Script macros

`$NORMALIZE` is defined in [`../SKILL.md`](../SKILL.md) Script Macros. Do not restate the CLI here.

---

## Parent steps

1. Resolve current session → jsonl (folder basename = session id). Prefer `<uuid>/<uuid>.jsonl` under `agent-transcripts`.
2. Resolve `start_node` / `end_node` (1-based closed interval over non-empty JSONL lines):
   - User gave numbers → use them.
   - User gave text anchors → `rg` / small window reads; prefer **user** nodes on multi-hit; disambiguate.
   - Default → `1` … last non-empty line.
   - **Forbid** loading entire jsonl into model context.
3. Infer `title` / `slug` / `ts`; choose `--out`:
   - `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`
4. Run `$NORMALIZE` with `--transcript` `--start-node` `--end-node` `--out` `--title` `--local-md` (+ optional project/theme/slug). Exit ≠ 0 → stop.
5. Confirm the file exists at `--out`.

**Hard:** Parent **MUST NOT** hand-parse jsonl or hand-build `TURN_SEP` bodies. Use `$NORMALIZE` only.

Paste path: user supplies finished TURN_SEP markdown on disk → skip steps 2–4; confirm that path.

---

## Done receipt

```text
> ✅ dialogue-archive complete
> 📄 local：<absolute .cache path>
> 🧭 nodes：<start>-<end>
```
