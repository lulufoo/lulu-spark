# Output Templates

Use one of these templates depending on the user's ask.

## Default Theme-First Format

```md
# {Original Video Title}

Source: [Video]({url})

## {Theme}
Time: {start} - {end}

Host: {dialogue-style paraphrase}

{Guest Name}: {dialogue-style paraphrase}
```

## Dense Research Format

```md
# {Original Video Title}

Source: [Video]({url})

## {Theme}
Time: {start} - {end}
Focus: {one-line explanation}

Host: {turn 1}
{Guest Name}: {turn 2}
Host: {turn 3}
{Guest Name}: {turn 4}
```

Use this format when the user wants something closer to interview flow.

## Summary-Lighter Format

```md
# {Original Video Title}

## {Theme}
Time: {start} - {end}

Host: {question or framing}

{Guest Name}: {main answer}
```

Use this format when the user wants fewer turns and higher compression.

## Naming Conventions

- Prefer topic names over generic labels.
- Keep section titles short and concrete.
- Keep time on its own line.
- Use speaker names consistently across sections.
- If the host's name is unknown, use `Host`.
- If a speaker label is inferred but not certain, use `{Speaker} (uncertain)`.
