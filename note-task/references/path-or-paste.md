# Path or paste

Use when the input is a path or pasted Markdown and no producer supplied the note fields.

Stage the Markdown on an allow-listed path. Load [chinese-companion.md](chinese-companion.md). Call `create_note` with the live schema parameters.

## Fields

| Field | Rule |
|-------|------|
| `project` | Closest topic; else `inbox` |
| `theme` | kebab-case English from title or body (3–5 words) |
| `title` | `#` heading or first line |
| `created_at` | Default is the current time (`YYYYMMDDHHMM`, UTC+8). Omit unless the user gives a time. |
| `source_type` | `--source-type article` → `article`; `--source-type theme-line` → `theme-line`; else `summary` |

Do not invent `{ts}-{slug}` as the write identity.

## Body

Separate the title from the body with `---`. Do not add a digest nav line.

For a summary, delete external images before staging. Keep prose, headings, relative images, and non-image links.

## Images

When the live schema requires companion-image paths, pass each relative image beside the staged Markdown. Field name, limits, and allowed extensions come from that schema.

Reject a path outside that directory. A missing file, or a file the schema rejects, stops the create. Do not drop the image and create anyway. Do not embed the image in the Markdown.
