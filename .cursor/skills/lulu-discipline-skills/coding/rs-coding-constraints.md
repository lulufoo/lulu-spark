---
rule-guard:
  globs:
    - "**/*.rs"
---

# RS Coding Constraints

**Goal:** Guide the AI, at the coding-execution layer, toward good code.

## Length

File at most 300 lines. Function: if 200 lines could be 50, rewrite it.

## Naming

Files and methods use `snake_case`. Types use CapWords. Folders use `snake_case`. No camelCase.

Files and types name a thing (noun). Methods start with an action (`is` / `has` / `can` count as actions).
