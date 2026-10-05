# Speaker roster

Acquire fills `meta.speakers` with **proper names** when the source gives them. Compose must use that roster on turns. `(uncertain)` is a fallback, not the default.

This is **not** diarization. Do not download audio. Do not run Whisper. Do not hardcode per-video name tables.

---

## Roster (Acquire Map)

Build `meta.speakers` in this order. Deduplicate. Keep source spelling.

1. **In-text labels** — unique values from:
   - `utterance.speaker` if already set
   - line prefix `[Name]` / `【Name】`
   - line prefix `Name:` where `Name` looks like a person (CJK 2–4 chars, or capitalized Latin words)
2. **Title pair** — from `meta.title`. If the title is `Pair: rest` or `Pair | rest`, parse `Pair` only. Then first match:
   - `A x B` / `A × B`
   - `A & B` / `A and B` / `A with B`
   - `A interviews B` / `B interviews A`
   Keep tokens that look like person names. Drop show/outlet tails.
3. **User-supplied names** — if the user named host/guest, those win over title order.
4. **Fallback** — if still empty and the text is a two-person Q&A: `["Host", "Guest"]`. If one speaker only: `["Host"]`.

Set `utterance.speaker` during Map when that line has an in-text label (step 1). Otherwise leave `null`.

---

## Roles (Compose)

When the roster has **two proper names** (not the literals `Host` / `Guest`):

1. Count vocatives in the utterance text: `Hi, {Name}`, `, {Name}`, `{Name}?`, `Thank you, {Name}` (first name or full name).
2. The name addressed more often is **Guest**. The other is **Host**.
3. If tied: title-pair first name = Guest (featured), second = Host.

When the roster is `Host` / `Guest` only, keep those labels.

---

## Turns (Compose C5–C6)

Assign a speaker **per utterance before merge**. Then merge only same-speaker fragments.

Start a new turn when any of these is true:

- `utterance.speaker` or in-text label changes
- a caption line starts with `-` after another speaker (overlap pair)
- previous utterance ends with `?` and the current one is a long declarative (≥ 12 words) and the roster has two names

C6 may merge consecutive fragments only when the assigned speaker is the **same** and (VTT roll-up or gap < 1.5s).

**Forbidden:** merge “both unknown” when the roster has two names. That produces mixed-speaker blobs.

Emit `HostName:` / `GuestName:` using the roster names.

---

## Fallback

Use `(uncertain)` only when:

- the roster has no proper names and is not `Host`/`Guest`, or
- a turn cannot be assigned after the rules above (keep the wording; do not drop it)

Do not suffix every inferred name with `(uncertain)`. Record inference once in provenance:

```md
> 采集：{platform} · complete-dialogue · 嘉宾：{guest} · 说话人：标题与问答推断
```
