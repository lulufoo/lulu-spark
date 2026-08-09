# UI English Copy Switch — P4 Smoke Checklist

Source: tech-doc §P4 / T8 / AC-2 (cycle `feature-20260718130642-22a23648`).

Manual spot-check for the key user path after the one-shot English copy switch. Automated gate: `npm test` (includes `tests/copy-switch-p4-post-verify.test.js`).

**Exclusions (not failures):** Skills directory display names (`skills-workbench-content.js`, `skills-software-dev-content.js`) and user-created content may remain non-English.

## Home hub

- [ ] Open app → home hub shows four shortcuts: **Notes**, **Read Later**, **Knowledge**, **Todos** (table A brands).
- [ ] No Chinese or mixed labels on shortcut tiles.
- [ ] Click **Notes** → navigates to `#/workbench`.

## Workbench

- [ ] Header shows **LuLu Workbench**; **Knowledge** repo menu; **← Home** nav.
- [ ] Sidebar date tabs use English month/day labels (e.g. `May 4`, `2026 Wed`).
- [ ] Card importance badges show **↑ High** / **→ Med** / **↓ Low** (not Chinese).
- [ ] Search placeholder: `Search notes…`; no Chinese in empty states or dialogs.

## Plan tasks

- [ ] From home hub, click **Todos** → `#/plan-tasks` loads.
- [ ] Status labels, buttons, empty states, and relative time strings are English.
- [ ] AI assistant entry opens assistant window; no Chinese in plan-task chrome.

## Corpus (Knowledge)

- [ ] From home hub, click **Knowledge** → `#/corpus` or knowledge viewer loads.
- [ ] Knowledge search placeholder and viewer chrome are English.
- [ ] No Chinese in corpus list/viewer UI (user document body excluded).

## Skills (excluded — informational only)

- [ ] Skills menu may show Chinese directory names (e.g. 对话回顾) — **not a failure** per tech-doc invariant #3.

## User content (excluded)

- [ ] Note titles, tags, and document bodies may be any language — **not a failure**.

## Regression

- [ ] `npm test` exits 0 (full suite green).
