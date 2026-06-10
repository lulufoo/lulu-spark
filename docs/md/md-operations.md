---
rule-guard:
  globs:
    - "**/*.md"
---

# AI Writing — MD Document Discipline

Applies when: creating or editing `.md` document files.

---

## Editing Basics

- When interpreting source code, attach the official link at the top of the document.
- File name must match the document title, using `a-b-c.md` format.
- Temporary files default to the workspace `.cache` directory.
- Before editing an existing document, read through all affected sections first.

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

## Writing Skill — Simplicity First

**Minimum content that fulfills the intent. Nothing speculative.**

Combat the tendency toward over-documentation:
- No sections beyond what the user asked for
- No abstractions for single-use explanations
- No "background" or "context" that wasn't requested
- No caveats for situations that cannot realistically arise
- If 10 bullet points could be 3, rewrite it

**The test:** Would a reader say this is over-explained? If yes, cut it.
