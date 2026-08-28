---
rule-guard:
  globs:
    - "**/*.{js,ts,jsx,tsx,py,go,rs,rb,swift,html,css,json,java,kt}"
---

# Coding Execution Constraints

**Goal:** Guide the AI, at the coding-execution layer, toward good code.

## File

A file is at most 300 lines.

## Function

If 200 lines could be 50, rewrite it
