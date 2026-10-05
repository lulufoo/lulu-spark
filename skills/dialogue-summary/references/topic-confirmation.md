# Topic Confirmation

> Produce a user-confirmable summary spine and internal emphasis marks. Do not
> draft document prose.

## Input

Use the supplied feedstock and requested scope as the complete evidence set.

1. Default to the full feedstock and a non-participant reader.
2. Use `argument` mode unless another document owns the implementation; then
   use `design-pointer` and summarize only its reasoning and convergence.
3. Do not reopen raw JSONL or invent missing turns.

## Spine contract

The spine captures the dialogue's reader-facing decision arc.

1. Keep issues, forks, corrections, decisions, meaningful rejects, and
   boundaries that advance or support the main line.
2. Give a topic its own position only when it has its own
   **tension → convergence**; merge candidates with one shared settlement.
3. Order topics by meaning—normally problem, important forks, convergence, and
   resulting boundary—not by turn sequence.
4. Use short, declarative, context-free titles. Avoid questions, bare handles,
   and implementation headings copied from an owning document.
5. Internally mark `core_topics` whose distortion would change the conclusion,
   reasoning foundation, or usage boundary. Marks control writing depth and
   never appear in user-facing titles.

## Output

Return `$CONFIRM` and internal `$CORE_TOPICS`.

Show the user only:

```text
Scope: full | Turn X～Y | <topic filter>
Reader: non-participant
Mode: design-pointer       # omit for default argument mode
Spine:
  1. …
  2. …
  …
```

Stop for explicit confirmation. A user edit rebuilds both outputs.

## Acceptance

The phase passes only when:

1. scope is valid and has not silently expanded;
2. every topic owns one tension→convergence and the spine alone reveals the
   decision arc; and
3. titles are context-free while noise, duplicates, and internal core marks
   remain outside the user-facing block.
