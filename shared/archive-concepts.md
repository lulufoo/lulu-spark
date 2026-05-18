# Archive 层 — 路径与配置

> 供 `shared/archive-digest.md` 及各 raw 生产者（`dtd_raw_*`、`theme-line` 等）引用。

---

## 读取 config.json

| 执行位置 | 路径 |
|----------|------|
| 仓库根 install skill（`lulu-workbench-skills/SKILL.md`） | `./config.json` |
| 子 skill（`ddm/`、`theme-line/` 等） | `{skill_dir}/../config.json` |

`archive_root` 字段必填。缺失时中止，提示先编辑仓库根 `config.json` 并完成安装（见根 `SKILL.md`）。

**禁止**读取各子 skill 目录内已废弃的 `config.json`。

`CTA_BASE` = `archive_root`。

---

## 符号定义

| 符号 | 类型 | 约束 |
|------|------|------|
| `ts` | string[12] | `YYYYMMDDHHMM`，落盘时东八区本地时间 |
| `slug` | string | 全小写连字符；不含 `ts` |
| `topic-path` | string | 归档时确定，全程不变 |

**路径定义**

```
COMMON_PATH  := <topic-path>/<ts>-<slug>.md
RAW          := CTA_BASE/raw/<COMMON_PATH>
DISTILLED    := CTA_BASE/distilled/<COMMON_PATH>
DIAGNOSE     := CTA_BASE/diagnose/<COMMON_PATH>
DIGEST       := CTA_BASE/digest/<COMMON_PATH>
TRACE        := CTA_BASE/trace/<COMMON_PATH>
```

**文内创建时间**：一级标题下一行写 `> 创建时间：YYYY年M月D日 HH:MM`（月日不补零），须与 `<ts>` 一致。

---

## index.json

真源：`{archive_root}/index.json`。

`layers` 顺序（去重）：`raw → digest → distilled → diagnose → trace`。各层可独立存在。

**导航前缀**：`N := |topic-path 段数|`（按 `/` 分割），`prefix := "../" × (N+1)`。

---

## digest 规范

见 [archive-digest.md](archive-digest.md)。由 `dtd_raw_dialogue`、`dtd_raw_summary`、`theme-line` 等在落 `raw/` 后自动链式执行。
