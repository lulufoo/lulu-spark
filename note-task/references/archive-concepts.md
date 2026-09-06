# Note path symbols

Owned by `note-task`. Host assigns `common_path`. Do not invent `{ts}-{slug}` as the write identity. Do not write `notes/` on disk.

## Path

```
COMMON_PATH = <project>/<theme>/<created_at>-<6 alnum>[-<source-basename>].md
```

| Layer | Path under the Workbench notes store |
|-------|--------------------------------------|
| raw | `notes/raw/<COMMON_PATH>` |
| raw (zh) | `notes/raw/<stem>-zh.md` |
| companion image | Sibling of the raw Markdown (`notes/raw/<project>/<theme>/<basename>`). Copied by Host from `asset_paths`. Not in `common_path`. |
| digest | `notes/digest/<COMMON_PATH>` |
| staging / debug | workspace `.cache/…` only |

Do not write TranscriptBundle or other intermediates into notes.

## Host raw shell

```markdown
# {title}

> 创建时间：{YYYY年M月D日 HH:MM}

---

{body}
```

No digest nav line. Digest and raw share `common_path` and `layers`.

## Layers

```json
"layers": ["raw"]
```

After a digest is written:

```json
"layers": ["raw", "digest"]
```

## Other Host fields

| Field | Rule |
|-------|------|
| `project` | Closest topic; else `inbox`. Do not read local `topics.json`. |
| `created_at` | `YYYYMMDDHHMM` (UTC+8). The 6-char segment is Host-generated. |
| `id` | 32-char lowercase hex from `create_note`. Do not invent it. |
