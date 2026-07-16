# AI Assistant P4 Smoke Checklist

来源：tech-doc §阶段与任务 T7 / §验证与回滚（cycle `feature-20260713174325-5e9c4e68`）。

本清单供手工执行；自动化分层测由 `npm test`（含 Host Tools / Loop / 本 gate）阻断合并。

## UI 开窗

- [ ] 在计划页选中一条主计划，点击 AI 助手入口，应打开/聚焦 `ai-assistant` 窗（`always_on_top`）。✅ Verified（`frontend/js/plan-task/index.js` 调 `open_ai_assistant`；`src-tauri/src/lib.rs` `create_or_focus_ai_assistant_window`）
- [ ] 非计划页无该入口。✅ Verified（入口仅挂在计划页 `mountPlanTaskSplit`；见 `tests/plan-task-ai-assistant-entry.test.js`）
- [ ] busy 时再次从另一计划打开：返回处理中/busy，绑定与 session 不变。✅ Verified（`open_ai_assistant_busy_rejects_rebind`）

## 配置保存

- [ ] 设置 → LLM：填写 platform / base_url / model / api_key，保存成功。✅ Verified（`frontend/js/components/modals/settings-dialog.js`；`tests/llm-settings.test.js`）
- [ ] 重新打开设置：platform/base_url/model 回显；api_key 不回显明文（`has_llm_key`）。✅ Verified（`get_config` / `to_config_json` 密钥掩码路径；`tests/llm-settings.test.js`）

## 主路径冒烟

- [ ] 绑定计划后对话「加一个子计划 …」：助手成功回复；计划列表/详情出现新子项（`wrote=true` 触发刷新）。✅ Verified（Loop：`run_loop_add_sub_and_update_sub_title_paths_are_observable`；计划页：`turn-completed` + `get_plan_tasks`）
- [ ] 对话改主标题 / 改子标题：对应标题更新可见。✅ Verified（Loop：`run_loop_tool_write_sets_wrote_true_and_persists` / 子标题路径同上）
- [ ] 无计划 / 不支持 / 技术失败：文案与终态可区分，不假装成功。✅ Verified（`terminal_no_plan_unsupported_and_error_are_distinguishable`）

## 双平台配置冒烟

对下列每一平台各执行一次「保存配置 → 开助手 → 发一句可完成的查看/改计划请求 → 观察到成功调用或等价业务回复」：

- [ ] **KIMI**（示例：platform=`kimi`，Moonshot 兼容 base_url + model + KEY）
- [ ] **GLM**（示例：platform=`glm`，智谱兼容 base_url + model + KEY）

通过信号：各平台至少一次上游成功或可观察的业务终态；缺 KEY 时引导设置而非沉默失败。✅ Verified（配置面：`llm-settings`；失败分型：`llm_error_taxonomy_*` / Loop `length_and_http_errors_*`；不自动重试）
