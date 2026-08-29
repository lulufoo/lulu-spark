# Full-English default translation

> Owned by `theme-archive` `[AR-1b]`. Producers must **not** translate.

## When to translate

Run after `[AR-1]`, on the **primary** markdown that will be archived:

```bash
python3 "$SKILL_DIR/theme-archive/scripts/detect_full_english.py" "<primary.md>"
```

| Result | Action |
|--------|--------|
| exit 0 / `full_english` | Agent translates the body (`---` after) into Chinese → `-zh.md` |
| exit 1 / `not_full_english` | Do **not** translate; omit `translations` |

`$SKILL_DIR` = `lulu-workbench-skills` install root.

## What “full English” means

- Inspect **body after `---` only**. Header chrome (`创建时间` / `来源` / `导航`) is Chinese by contract and does not count.
- Ignore body chrome: `作者 | …` lines, HTML comments.
- Remaining body has Latin letters **and** no Han / Kana / Hangul → full English.
- Any remaining CJK → mixed or Chinese. The English spans are content; **do not** translate those spans alone.

## How to translate

- Same header metadata and digest nav as primary; **Chinese `#` title**.
- Translate the full body, not selected paragraphs.
- Write the full zh file next to the primary (`<ts>-<slug>-zh.md`). Do **not** put `SEE_FILE`, `PLACEHOLDER`, `FULL_ZH`, or “see file” stubs in the body.
- Run the parity script **before** `[AR-2]`:

```bash
python3 "$SKILL_DIR/theme-archive/scripts/check_zh_parity.py" "<primary.md>" "<zh.md>"
```

- Exit 1 → stop. Do not call `create_note`. Host repeats the same checks.
- Do not summarize or add commentary.
- theme-line zh title: `{Speaker}：{Event} | {Outlet}` when those are known; otherwise a concise Chinese title.

## Skip

| Case | Action |
|------|--------|
| Embedded `skip_translate: true` | Do not detect; omit `translations` |
| Digest-only Repair | Do not translate |

Producers **must not** pass `translations`. This step is the only source of `-zh.md`.
