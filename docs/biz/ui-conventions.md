# UI Conventions

**Status:** Binding  
**Audience:** Product and implementation (including AI changes to frontend copy)

---

## Copy language

Workbench **user-facing UI copy must be English**.

Includes, without limitation: buttons, labels, status names, empty states, placeholders, relative time, navigation, and warning text.

**Out of scope:**

- User-authored content (notes, plan bodies, archived originals, etc.)
- Skill directory display names already excluded elsewhere (see the UI English copy switch verification notes)

Decision dialogue may use Chinese concept names (e.g. 执行中 / 完成 / 废弃); UI must map them to English display (e.g. `In progress` / `Completed` / `Abandoned`). Persistence / API enum values stay English machine values, separate from display copy.

### Evidence

- ✅ Verified: todo-task status labels are English (`frontend/js/todo-task/index.js` → `STATUS_LABELS`: `In progress` / `Completed` / `Abandoned`).
- ✅ Verified: the repo has a one-shot “UI English copy switch” verification asset requiring English status labels, buttons, empty states, etc. (`docs/archive/ui/ui-english-copy-switch/p4-smoke-checklist.md`).
