---
rule-guard:
  globs:
    - "**/*.{html,css}"
---

# HTML / CSS Coding Constraints

**Goal:** Guide the AI, at the coding-execution layer, toward good code.

## Length

File at most 300 lines. If a stylesheet or markup block could be half as long, rewrite it.

## Naming

Files and folders use `kebab-case`. Class names and ids use `kebab-case`. No camelCase in class or id.

Files name a thing (noun). Prefer short, role-based names (`sheet`, `board-dock`) over vague ones (`wrapper`, `container2`).

## Boundaries

HTML is structure only: no large inline `<style>` or `<script>` bodies in source (build/vendor outputs may differ).

Put styles in `css/`. Put behavior in `js/` (or the project’s existing module layout). Do not grow a shell HTML back into a monolith.
