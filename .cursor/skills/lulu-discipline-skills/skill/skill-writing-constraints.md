# Skill Writing Constraints

Defines writing constraints for SKILL `.md` units. Compliant prose is scoped,
compact, and actionable.

## SKILL Entry

Applies to the entry `SKILL.md`.

1. **Internal `description`:** name only the component responsibility; omit
   callers, routes, tools, and payloads.
2. **External `description`:** state only the user-facing outcome; omit
   implementation.
3. **H1 purpose:** directly after the H1, state the SKILL's role and completion
   goal in 1–2 sentences. Do not repeat `description` or expand cognitive maps,
   contracts, pipelines, tools, or payload rules.

## Responsibility Boundaries

Each SKILL `.md` unit owns one responsibility.

1. State only its responsibility, inputs, outputs, and boundaries.
2. Reference external capabilities without copying their dispatch or internal
   contracts.

## Writing Standard

Applies to every rule.

1. **Concise:** express one concern using short clauses, lists, or tables.
2. **Distilled:** use exact wording; remove text that adds no instruction.
3. **On-point:** name the constraint that changes the next action.

## Section Shape

Each section covers one scope.

1. State the scope in one line.
2. Follow with numbered, single-concern rules.

## Anti-patterns

These patterns obscure actionable constraints.

1. **Rule wall:** one section stacks unrelated responsibilities, conditions, or
   flows.
2. **Tombstone negation:** a removed step or duty is restated as `Do not` /
   `Must not`. Negate only mistakes still possible on the current path.
