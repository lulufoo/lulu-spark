# Skill Design Philosophy

SKILL should be a lever for model capability, not its ceiling. It should provide
the minimum structure needed for purposeful action while preserving room for
better models to produce better outcomes. A strong SKILL amplifies model
progress instead of fixing performance at the level of its original authoring
model.

## Constraint Boundary

Constraints protect system correctness; they should not prescribe intelligence.

- Constrain goals, invariants, ownership, interfaces, permissions, and
  completion conditions.
- Leave semantic judgment, reasoning depth, exploration, synthesis, and
  expression to the model.
- Add procedural constraints only when required for safety, deterministic state
  transitions, state integrity, or reliable external effects.
- State what must hold instead of simulating how the model should think.

## Capability Space

Preserve the model's ability to adapt its execution to the actual context.

- Prefer outcome criteria over fixed reasoning chains.
- Avoid exhaustive branches when the model can judge the case from explicit
  boundaries.
- Put mechanical state transitions in tools; keep cognition in the model.
- Use the least orchestration needed to make responsibilities and handoffs
  unambiguous.

## Evolution Criterion

A SKILL remains open to model progress when:

- a stronger model can improve results without requiring more rules;
- its goals and invariants remain useful across model generations;
- model-specific workarounds can be removed without weakening correctness; and
- each new constraint answers a demonstrated system need rather than assumed
  model weakness.

## Failure Modes

- **Model ceiling** — fixed procedures prevent a stronger model from producing
  a better result.
- **Reasoning substitution** — instructions imitate or replace model judgment.
- **Procedural overfitting** — rules accumulate around one model, example, or
  failure case.
- **Capability suppression** — uniformity is pursued by removing useful
  exploration or adaptation.

## Relationship to Constraints

This philosophy governs the architecture and writing constraints. Those
documents operationalize it without redefining it. When they are refined,
preserve hard correctness boundaries while removing constraints that merely
limit model capability.
