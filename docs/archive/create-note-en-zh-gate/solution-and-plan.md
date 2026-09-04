# 创建笔记全文英文须附中译 技术方案与实施计划

**状态：** Implemented  
**关联待办：** `task_9ae49100bb0f`  
**收敛：** Host 托底拒漏译；全文英文必须带合格中译；无 skip

源码对照：`src-tauri/src/services/translation_gate.rs`、`src-tauri/src/services/notes/document.rs`、`lulu-workbench-skills/note-task/scripts/detect_full_english.py`

## 问题与目标

✅ Verified（`translation_gate.rs`、`notes/store.rs`）：Host 只在请求已带 `translations.zh` 时做 parity 校验；未带中译时不判断正文是不是全文英文。

✅ Verified（note-task `SKILL.md` Note Norms、`full-english-translate.md`）：翻译检测不在 Create 主路径；规范写「note-task does not translate」。

✅ Verified（theme-fetch / theme-line / theme-transcribe）：采集方各自写「Need `-zh.md`」再交接。`/note-task` 直写不走这步。

目标：所有经 `write_note` 的创建，先由技能检测并在全文英文时交出中译；Host 用同一套检测拒收漏译。Host 不生成中文。

## 方案

### 技能（`lulu-workbench-skills`）

⚠️ Proposed：Create 在调用 `create_note` 前必跑 `detect_full_english.py`。`full_english` → 译 `-zh.md` → `check_zh_parity.py` → `translations`；`not_full_english` → 不带 `translations`。

⚠️ Proposed：把该步骤写入 note-task 规范表。改掉 `full-english-translate.md` 的「does not translate」：这是 Create 前置，不再是「调用方想要才跑」。

⚠️ Proposed：`standalone-resolve.md` 的 Create 段补上同一前置。Done / 写后校验：全文英文时确认 Host 返回了 zh 的 `extra_paths`。

⚠️ Proposed：theme-fetch / theme-line / theme-transcribe 只组主文件并交接 note-task Create，删掉各自的检测/翻译分叉。`bundle.meta.language` 仍不驱动翻译。

### Host（`lulu-workbench`）

✅ Verified（`detect_full_english.py`）：`---` 后正文；忽略 `作者 | …` 与 HTML 注释行；有汉字/假名/韩文 → 非全文英文；其余有拉丁字母 → 全文英文；空正文 → 否。

⚠️ Proposed：把上述规则迁到 `translation_gate.rs`（与 Python 脚本保持同一套）。在 `write_note` 里，`assemble_raw` 且 `parse_translations` 之后：若 `is_full_english(raw_doc)` 且 translations 没有 `lang=zh`，返回 400。建议错误文案：`full English note requires translations.zh`。

⚠️ Proposed：闸门挂在 `write_note`，因此 desktop `source_path`、mobile `content`、jot 共用。中文正文不会被判为全文英文。只有 `fr` 没有 `zh` 仍拒。无 skip 字段。

⚠️ Proposed：`create_note` 的 MCP `translations` 描述补一句：全文英文且缺合格 zh 会被拒。不新增参数。

✅ Verified（`unit-tests/services/notes.rs` `SAMPLE_DOC`）：现有夹具正文是英文、多数测试不带 `translations`。闸门上线后这些测试会 400。把 `SAMPLE_DOC` 正文改成含汉字（测路径/digest 用）；另加英文夹具覆盖拒收与带 zh 放行。新单测放在 `src-tauri/src/unit-tests/`，不写进生产文件。

### 不做什么

⚠️ Proposed：不迁移已存在的英文-only 旧笔记。Host 不自动翻译。不新增翻译 MCP。不增加 skip。不改 digest AD-0。

## 实施计划

1. ⚠️ Proposed：Host 实现 `is_full_english`，接入 `write_note`，更新 MCP 描述。
2. ⚠️ Proposed：修正 `SAMPLE_DOC`；在 `unit-tests` 增加：英文无 zh → 400；英文+合格 zh → 200；中英混排无 zh → 200；jot 中文不受影响。
3. ⚠️ Proposed：改 note-task 规范表、`full-english-translate.md`、`standalone-resolve.md`。
4. ⚠️ Proposed：采集方三处交接改为「交给 note-task Create」，去掉重复检测步骤。

## 验收

1. ⚠️ Proposed：全文英文、不带 zh 的 `create_note` / `create_note_content` 返回 400，且未写入 raw。
2. ⚠️ Proposed：全文英文且 zh 通过现有 parity，写入 raw + `-zh.md`，index 有 `translations.zh`。
3. ⚠️ Proposed：正文含汉字时，不带 `translations` 仍可创建。
4. ⚠️ Proposed：note-task Create 主路径写明检测；采集方不再单独要求 Agent 先译。
5. ⚠️ Proposed：既有 digest / 路径 / 占位中译拒收单测仍绿。
