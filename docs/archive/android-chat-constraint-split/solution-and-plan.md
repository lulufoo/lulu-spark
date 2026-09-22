# Chat：按 KT 300 行约束拆 ViewModel

**状态：** 已实施（ChatViewModelTest + ChatVoiceTest 绿）  
**源码：** `lulu-workbench-android/lulu-workbench/`  
**决策来源：** 用户要求按最新代码约束重构 Android 现有变更

## 1. 目标

现有 Chat MVI 变更继续有效，同时满足 KT 约束：文件最多 300 行；方法是动作名。

✅ Verified（`ChatViewModel.kt` 333 行；`ChatViewModelTest.kt` 306 行；`kt-coding-constraints.md`：File at most 300 lines）

## 2. 锁定

| # | 题目 | 选择 |
|---|---|---|
| 1 | 范围 | 只动 Chat 这次未提交变更里超标的文件 |
| 2 | 枢纽 | `ChatViewModel` 仍是唯一 `dispatch`；不恢复 `ChatStore` |
| 3 | 抽出 | 无状态查询/恢复放到 `ChatCatalog.kt` |
| 4 | 测试 | `RecordingKeepAlive` 单独文件；不断行为 |

## 3. 步骤

1. `restore` / `listSessionItems` / `listStagedItems` / `loadVisibleTurns` / `voiceErrorHint` 迁到 `ChatCatalog.kt`。  
2. `ChatViewModel` 改调这些函数；删 `listed`、`stagedItems` 这类非动作名。  
3. 测试辅助类拆出。  
4. `:app` 里 Chat 单测绿。

## 4. 明确不做

不改 Settings / Bind / Stage。不把 `dispatch` 再抽成独立 reduce。不改 `ChatCommands` 行为。
