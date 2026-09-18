---
rule-guard:
  globs:
    - "**/*.md"
---

# AI Writing — MD Document Discipline

Applies when: creating or editing `.md` document files.

---

## Editing Basics

- File name is English `a-b-c.md`.
- Temporary files default to the workspace `.cache` directory.

---

## Editing Language

- SKILL files default to English, other documents default to Chinese.
- Keep the document language consistent; do not mix languages within the same document.

---

## Content Sourcing

**Everything written into a document must carry a confidence label.**

- Factual claims must come from: source code, official documentation, or verified tool output.
- Never write from memory or inference; if unverifiable, say so explicitly.
- Every factual statement must be labeled:
  - ✅ Verified (with source)
  - ⚠️ Inferred (only for logical deductions or tool-unreachable cases; never a substitute for verification)
  - ❌ Unresolved (pending verification)

---

## Editing Boundaries

**User intent is the sole anchor — constrain all written content to its scope.**

- Only write or modify what is strictly necessary to fulfill the user's intent; do not produce sections outside that scope.
- Before adding each new paragraph, ask: can the user still achieve their intent without it? Yes → omit it.
- When output drifts from intent: retract it, do not follow through.
