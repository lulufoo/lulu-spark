# Body sanitize — external images

Apply to the summary body **after** input is confirmed and **before** composing the archive document (theme-archive [AR-1] 之前).

## Goal

Remove **external-linked images** only. Do not rewrite prose, headings, or non-image links.

## Remove

| Pattern | Action |
|---------|--------|
| Markdown image `![alt](http…)` or `![alt](https…)` | Delete the image token; if the line becomes empty, delete the whole line |
| HTML `<img … src="http…">` / `src='https…'` | Delete the tag; delete the line if empty afterward |
| Reference-style image `![alt][ref]` when `[ref]: https?://…` points to an image URL | Delete image line and the reference definition line |
| Line that is **only** a bare `https?://…` URL whose path ends in `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.svg` (case-insensitive) | Delete the line |

## Keep

- Non-image hyperlinks in prose: `[text](https://example.com/page)`
- Relative images: `![](assets/foo.png)` — not external
- Plain-text placeholder lines such as `图片` (no URL) — unless the user asks to strip them

## After removal

- Collapse 3+ consecutive blank lines to at most 2
- Do not change wording outside removed image tokens

## Verification

Before archive, confirm the body contains no `![…](http` / `![…](https` / `<img` with `http` src.
