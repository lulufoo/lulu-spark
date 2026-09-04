# Full-English translation

Load before every `create_note`. Host rejects a full-English primary that has no `translations.zh`.

## Script Macros

| Macro | Command |
|-------|---------|
| `$DETECT_EN` | `python3 "$SKILL_DIR/note-task/scripts/detect_full_english.py"` |
| `$CHECK_ZH` | `python3 "$SKILL_DIR/note-task/scripts/check_zh_parity.py"` |

`$SKILL_DIR` = `lulu-workbench-skills` install root. Exit codes and stdout: `$DETECT_EN --help`, `$CHECK_ZH --help`.

## Detect

Run `$DETECT_EN "<primary.md>"` on the staged primary.

| Result | Action |
|--------|--------|
| exit 0 / `full_english` | Write `-zh.md` next to the primary. Run `$CHECK_ZH "<primary.md>" "<zh.md>"`. Exit 1 → stop; do not call `create_note`. |
| exit 1 / `not_full_english` | Omit `translations`. |

## Companion invariants

- Same header metadata as the primary if present; Chinese `#` title. No digest nav line.
- Full body after `---`. No stub markers (`SEE_FILE` / `PLACEHOLDER` / `FULL_ZH`).
- theme-line zh title: `{Speaker}：{Event} | {Outlet}` when known; else a concise Chinese title.

Pass the zh file through `create_note` `translations` (live schema).
