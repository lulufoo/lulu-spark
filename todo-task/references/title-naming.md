# Title Naming

SSOT for `create_todo_task` / proposed `title` values under Todo Norms.

## Template

```text
{Domain} {Topic}：{Focus}{WorkType}
```

All four segments are required unless the user explicitly supplies the title.

## Segments

| Segment | Rule |
|---------|------|
| **Domain** | Scannable system or product surface (e.g. `Compose`, `lulu-tasks`, `decision`). A bare gate code (`G3`, `RS`) **MUST NOT** be the only subject. |
| **Topic** | Stable problem area; noun phrase. No version tags, no dates. |
| **Separator** | Full-width `：` for new titles. |
| **Focus** | This todo's concrete cut (object, scene, or defect). Necessary jargon is OK if it reads after Topic. |
| **WorkType** | Closed set (pick one): `效果分析` \| `问题描述与分析` \| `方案` \| `缺陷修复` \| `流程优化` \| `执行待办` \| `待优化计划` |

## Good example

```text
Compose 写入节奏控制：Init逐章写入效果分析
```

- Domain: `Compose`
- Topic: `写入节奏控制`
- Focus: `Init逐章写入`
- WorkType: `效果分析`

## Bad → rewrite

| Bad | Why | Rewrite |
|-----|-----|---------|
| `extra wt 误绑仓` | Internal jargon; no Domain; no WorkType | `lulu-code 跨仓 worktree：extra误绑主仓缺陷修复` |

## Self-check

Without chat context, can a reader answer:

1. Which system?
2. What problem area?
3. Is this analysis, a fix, a plan, or execution?

Any "no" → rewrite before create.

## Length budget

After composing, obey the live MCP/Host `title` constraints (Parameter SSOT — do not hardcode limits here).

If over budget, compress in order: **Focus → Topic → Domain abbreviation**. Do **not** drop WorkType.

## User override

If the user explicitly gives a title, use it as given and briefly note that the naming template was skipped.
