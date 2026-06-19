# Archive Concepts (shared)

> 供 `theme-archive`、`theme-fetch`、`theme-line`、`theme-digest`、`dialogue-summary`、`theme-summary` 共用的路径与配置约定。

---

## Configuration

读取 `{skill_dir}/../config.json`（repository 根）获取 `archive_root`：

```json
{
  "archive_root": "/path/to/lulu-workbench-knowledge"
}
```

- `archive_root` 未配置或读 `config.json` 失败 → **立即中止**（不进入后续 Step）
- 各 skill 首步确认：`> ✅ config.json 读取完成 · archive_root: <路径>`

---

## Path symbols

```
COMMON_PATH = <topic-path>/<ts>-<slug>.md
topic-path  = <project>/<doc-theme>    # 始终 2 段
prefix      = "../../../"              # raw/ 文件内导航相对前缀
```

| 目录 | 路径模式 |
|------|---------|
| raw | `{archive_root}/raw/<topic-path>/<ts>-<slug>.md` |
| raw (zh) | `{archive_root}/raw/<topic-path>/<ts>-<slug>-zh.md` |
| digest | `{archive_root}/digest/<COMMON_PATH>` |
| debug bundle | `{archive_root}/.cache/<topic-path>/<ts>-<slug>-bundle.json` |

**禁止** 将 TranscriptBundle 写入 `trace/`（trace 仅 DDM 认知 trace）。

---

## `layers` 顺序

```json
"layers": ["raw"]
```

Archive digest 完成后追加 `"digest"`：

```json
"layers": ["raw", "digest"]
```

**不追加** `"trace"`（theme-line 产出范围）。

---

## Navigation line

**MCP 产出者**（`theme-summary`、`theme-line`、`dialogue-summary`）raw header **仅含 digest 链接**（Workbench `archive_document` 解析要求）：

```markdown
> 导航：[digest]({prefix}digest/{COMMON_PATH})
```

**Legacy / 全链格式**（`references/legacy/` 等旧手册）：可含 distilled · digest · trace 三链。

digest 文件导航行：读取 `index.json` 对应条目 `layers`；含 `distilled` / `trace` 时追加链接。

---

## Slug & timestamp

```
ts   = YYYYMMDDHHMM (UTC+8)
slug = kebab-case summary of source title (English, no spaces)
```

slug 冲突 → 与用户确认后再继续。

---

## Entry ID

32 字符小写 hex：`secrets.token_hex(16)`（Python）或等效。

---

## References

- theme-archive（raw + index，不触发 digest）：[../theme-archive/SKILL.md](../theme-archive/SKILL.md)
- theme-line archive 步骤：[../theme-line/references/archive-steps.md](../theme-line/references/archive-steps.md)
- theme-digest workflow：[../theme-digest/SKILL.md](../theme-digest/SKILL.md)
