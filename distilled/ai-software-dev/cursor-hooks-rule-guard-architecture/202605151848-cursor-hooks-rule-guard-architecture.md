# Cursor Hooks 调试与 Rule Guard 体系设计对话 · 概要

> 创建时间：2026年5月15日 18:48

> 导航：[digest](../../../digest/ai-software-dev/cursor-hooks-rule-guard-architecture/202605151848-cursor-hooks-rule-guard-architecture.md) · [trace](../../../trace/ai-software-dev/cursor-hooks-rule-guard-architecture/202605151848-cursor-hooks-rule-guard-architecture.md) · [raw](../../../raw/ai-software-dev/cursor-hooks-rule-guard-architecture/202605151848-cursor-hooks-rule-guard-architecture.md)

---

**Cursor Hooks 初次搭建与观测**｜Turn 1–6

- Turn 1–2：用 bash + `postToolUse(Read)` 打印路径；询问 Hooks 输出位置。
- Turn 3：改用日志文件（`.cursor/hooks/read.log`）；重启提示。
- Turn 4–5：`@js/main.js` 不触发 hook；改用 `beforeReadFile`/`postToolUse` 无 matcher 试探；JSON 路径错误（`.input.path` vs `.tool_input.path`/`.file_path`）。
- Turn 6：日志迁到 `.cache/agent-audit.log`，脚本重命名为 `audit-read.sh`。

---

**Hook 不触发与根因定位**｜Turn 7–15

- Turn 7–8：Hooks 无红字但事件未命中；查 Cursor 3.3.30 内 `Read` 与 matcher；发现需 Workspace Trust 才加载 project hooks。
- Turn 9：`workbench.trust.manage` 查证；子任务确认 `postToolUse` 的 payload 应为 `tool_input.path`。
- Turn 10：`/tmp` 测试仍无触发。
- Turn 11–12：对齐可工作的 `beforeReadFile` + Python + `{"permission":"allow"}`；解释 `postToolUse`、`{}`、路径三问题。
- Turn 13：`beforeReadFile` 在 newsfeed 可记日志；`@file` 绕过 Read。
- Turn 14–15：分析任务只记到 `SKILL.md`；会话内已读文件缓存导致不重复触发；新 Chat 不继承附件式缓存。

---

**@file 监测、需求浮现与 A/B 取舍**｜Turn 16–25

- Turn 16：`@file` 用 `beforeSubmitPrompt` 仅能抓文本，否则接受只审计 agent 主动 Read。
- Turn 17：明确需求——改代码前须读 `coding-global.mdc`，否则不合法。
- Turn 18：`B` 不需改提示词，`A` 为自我反馈；`.mdc` 走规则缓存 hook 难追踪。
- Turn 19–22：讨论 A 的软约束风险、B 机械可靠性与「读了不看」漏洞；同意加载层与合规层分离。
- Turn 23–25：`read-rules.json` 映射、`docs` vs `.cursor/docs`、机器级脚本 + `cursor-rule-guard` skill。

---

**话题归档与设计方案草稿**｜Turn 26–31

- Turn 26：按 Turn 聚合话题列表。
- Turn 27–28：补写设计方案入 `.cache`；`conversation.md`/规则同步边界。
- Turn 29–31：`read-events` 日志目录、多会话；审查 `log-task-id.py`；调研 chat_id/tab_id/session_id。

---

**会话标识完备性与架构收口**｜Turn 32–45

- Turn 32–35：`conversation_id` 混淆与子 agent 隔离；Shell 写入纳入 guard；补充风险备忘。
- Turn 36–38：提炼主题→完整架构；写入校验为独立特性；全文读取 vs offset/limit、`audit-read` 语义。
- Turn 39–43：cmd-k 内联场景、无会话 id、上下文是否共享、`beforeSubmitPrompt` 不可及。
- Turn 44–45：厘清 feature 范围与「写入须加载映射规则」的施工单诉求。

---

**实施落地与 skill 分发**｜Turn 46–58

- Turn 46–48：撤销/覆盖策略确认；执行重构；质疑 `hooks-lib` 放置。
- Turn 49–55：skill 本地化方案；提交 `lulu-dev-skills`；重装与 GitHub 安装链接；删除/恢复 `hooks.json` 排障。
- Turn 56–58：`hooks/rules-state` 与 `logs` 职责说明。

---

**BUG、初始化与仓库同步**｜Turn 59–70

- Turn 59–60：`@js/main` 无日志与 state 的复现与修复确认。
- Turn 61–64：init 拉取 `docs`/`read-rules.json`；`coding-global` 上游；`docs` 与 `rules-state` 同笼 `cursor-rule-guard`。
- Turn 65–70：`.gitignore`、更新 `.cache/cursor-rule-guard-design.md`、章节框架与成文、提交 `ai-software-dev` 的 `sys-prompt`。

---

**DDM 规范化**｜Turn 71

- Turn 71：执行 `dtd_normalize`，将本会话归档为 raw 并生成本 distilled 概要。

---
