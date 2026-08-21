# Output Templates

Default is complete chronological dialogue. Do not use summary or paraphrase templates.

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
- **`-zh.md`**: not built here — theme-archive `[AR-1b]` when body is full English
- 可选 provenance：`> 采集：{platform} · complete-dialogue · 嘉宾：{guest} · 说话人：标题与问答推断`

## Complete Dialogue Format (default)

Body only (metadata lives in archive header). Keep every turn after light clean:

```md
## 00:00–03:12

{Host Name}: {cleaned source wording}

{Guest Name}: {cleaned source wording}

{Host Name}: {cleaned source wording}
```

When the source has chapter titles, use those as headings and keep the same turn order:

```md
## {Source chapter title}
Time: {start} - {end}

{Host Name}: {cleaned source wording}

{Guest Name}: {cleaned source wording}
```

## Naming Conventions

- Prefer source chapter titles or time ranges over invented topic names.
- Keep time on its own line when a source chapter heading is used.
- Use roster names consistently (see [speaker-roster.md](speaker-roster.md)).
- If the host's name is unknown, use `Host`.
- `(uncertain)` only for a turn that cannot be assigned. Do not suffix every inferred name.
