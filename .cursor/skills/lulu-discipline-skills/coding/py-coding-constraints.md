---
rule-guard:
  globs:
    - "**/*.py"
---

# Coding Python Constraints

**Goal:** Guide the AI, at the coding-execution layer, toward good code.

## Length

File at most 300 lines. Function: if 200 lines could be 50, rewrite it.

## Naming

Files and methods use `snake_case`. Classes use CapWords. Folders use `x-y-z`. No camelCase.

Files and classes name a thing (noun). Methods start with an action (`is_` / `has_` / `can_` count as actions).
