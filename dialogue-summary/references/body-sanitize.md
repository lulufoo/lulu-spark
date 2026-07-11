# Body sanitize — external images

Apply to the retrospective body **before** composing the archive document.

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

- Non-image hyperlinks in prose
- Relative images: `![](assets/foo.png)`
- Plain-text placeholders without URL

## After removal

- Collapse 3+ consecutive blank lines to at most 2
- Do not change wording outside removed image tokens

## Verification

Body must contain no `![…](http` / `![…](https` / `<img` with `http` src.
