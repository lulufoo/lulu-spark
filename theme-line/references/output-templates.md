# Output Templates

Use one of these templates depending on the user's ask.

## Archive Header (Save to Archive)

All archived `raw/` files use this header block (navigation paths resolved per [archive-steps.md](archive-steps.md) Step 3):

```md
# {Document Title}

> 创建时间：{YYYY年M月D日 HH:MM}

> 时长：约 {duration_min} 分钟 · 发布：{YYYY-MM-DD}

> 导航：[digest](...)

> 原文：[Video]({url})
```

- **时长** / **发布**: 来自 `bundle.meta.duration_sec` / `bundle.meta.published_at`（非 yt-dlp 直接拉取）
- 非视频源（`duration_sec: null`）省略时长行
- **`-zh.md`**: same metadata lines; Chinese title e.g. `{Speaker}：{Event} | {Outlet}`
- 可选 provenance：`> 采集：{platform} · {strategy} · 嘉宾：{speakers}`

## Default Theme-First Format

Body only (metadata lives in archive header, not `Source:` line):

```md
# {Original Video Title}

## {Theme}
Time: {start} - {end}

Host: {dialogue-style paraphrase}

{Guest Name}: {dialogue-style paraphrase}
```

## Dense Research Format

```md
# {Original Video Title}

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
