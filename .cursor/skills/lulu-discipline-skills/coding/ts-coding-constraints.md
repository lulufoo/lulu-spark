---
rule-guard:
  globs:
    - "**/*.{ts,tsx,js}"
---

# TS Coding Constraints

**Goal:** Guide the AI, at the coding-execution layer, toward good code.

## Length

File at most 300 lines. Function: if 200 lines could be 50, rewrite it.

## Naming

Files use `kebab-case`. Methods use `camelCase`. Types and React components use CapWords. Folders use `x-y-z`.

Files and types name a thing (noun). Methods start with an action (`is` / `has` / `can` count as actions).
