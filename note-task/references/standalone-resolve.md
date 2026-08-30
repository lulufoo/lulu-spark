# Standalone resolve

When the user gives a path or pasted Markdown (no producer), infer fields, then load [`../SKILL.md`](../SKILL.md) and route **Create**. Host assigns `common_path`.

## Infer

| Field | Rule |
|-------|------|
| `project` | Closest topic; else `inbox` |
| `theme` | kebab-case English from title or body (3–5 words) |
| `title` | `#` heading or first line |
| `created_at` | `YYYYMMDDHHMM` (UTC+8); omit to use Host now |
| `source_type` | `--source-type article` → `article`; `--source-type theme-line` → `theme-line`; else `summary` |

Do not invent `{ts}-{slug}` as the write identity.

## GitHub URL timestamp

If the input is `github.com/{owner}/{repo}/blob/{ref}/{path}` or `raw.githubusercontent.com/{owner}/{repo}/{ref}/{path}`:

```bash
gh api "repos/{owner}/{repo}/commits?path={path}&sha={ref}&per_page=100" \
  --jq '[.[] | select(.commit.message | test("Delete"; "i") | not)] | last.commit.committer.date'
```

Convert that `commit.committer.date` to UTC+8 `YYYYMMDDHHMM`. Query fail or empty → omit `created_at`. Other URLs: skip this block.

## Header

If title or `---` is missing, wrap:

```markdown
# {Title}

> 创建时间：{YYYY年M月D日 HH:MM}

---

{body}
```

Do not add a digest nav line. Host rewrites the raw shell.

## Sanitize (summary body)

Before staging, remove **external images** only. Keep prose, headings, and non-image links.

| Pattern | Action |
|---------|--------|
| `![alt](http…)` / `![alt](https…)` | Delete the token; drop the line if empty |
| `<img … src="http…">` | Delete the tag; drop the line if empty |
| `![alt][ref]` when `[ref]` is an `https?://` image | Delete image line and the definition |
| A line that is only an `https?://` URL ending in `.png` / `.jpg` / `.jpeg` / `.gif` / `.webp` / `.svg` | Delete the line |

Keep relative images and non-image links. Collapse 3+ blank lines to 2. Confirm no `![…](http` / `<img` with `http` src remains.

## Create

Stage on an allow-listed path (desktop) or send `content` (mobile). Call `create_note` per the live schema.
