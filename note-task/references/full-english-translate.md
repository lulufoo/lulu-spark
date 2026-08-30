# Full-English translation (caller utility)

Callers who need a Chinese companion file run this **before** `create_note`. `note-task` does not translate.

## Script Macros

| Macro | Command |
|-------|---------|
| `$DETECT_EN` | `python3 "$SKILL_DIR/note-task/scripts/detect_full_english.py"` |
| `$CHECK_ZH` | `python3 "$SKILL_DIR/note-task/scripts/check_zh_parity.py"` |

`$SKILL_DIR` = `lulu-workbench-skills` install root. Exit codes and stdout: `$DETECT_EN --help`, `$CHECK_ZH --help`.

## Detect

On the primary markdown:

`$DETECT_EN "<primary.md>"`

| Result | Action |
|--------|--------|
| exit 0 / `full_english` | Translate the body after `---` into Chinese → `-zh.md` |
| exit 1 / `not_full_english` | Omit `translations` |

Inspect **body after `---` only**. Header chrome does not count. Ignore `作者 | …` lines and HTML comments. Remaining body has Latin letters and no Han / Kana / Hangul → full English. Any remaining CJK → do not translate isolated English spans.

## Translate

- Same header metadata as primary if present; Chinese `#` title. No digest nav line.
- Translate the full body. Write the zh file next to the primary. No `SEE_FILE` / `PLACEHOLDER` / `FULL_ZH` stubs.
- `$CHECK_ZH "<primary.md>" "<zh.md>"`. Exit 1 → stop; do not call `create_note`.
- Do not summarize or add commentary.
- theme-line zh title: `{Speaker}：{Event} | {Outlet}` when known; else a concise Chinese title.

Pass the zh file through `create_note` `translations` (live schema).
