# Chinese companion

Load before every `create_note`. A full-English primary with no Chinese companion is rejected.

## Detect

Run `scripts/detect_full_english.py` on the staged primary.

| Result | Do |
|--------|----|
| `full_english` | Write `-zh.md` beside the primary. Run `scripts/check_zh_parity.py`. A failed check stops `create_note`. |
| `not_full_english` | Omit `translations`. |

## Companion

Chinese `#` title. Translate the full body after `---`. No digest nav line. No stub markers (`SEE_FILE` / `PLACEHOLDER` / `FULL_ZH`).

Keep the same header metadata as the primary when it is present.

theme-line title: `{Speaker}：{Event} | {Outlet}` when known; otherwise a concise Chinese title.

Pass the zh file through `create_note` `translations` using the live schema.
