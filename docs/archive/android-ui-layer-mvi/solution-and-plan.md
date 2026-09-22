# Settings / Bind / Stage：按 UI 层约束对齐

**状态：** 已实施（BindViewModelTest、SettingsViewModelTest、StageViewModelTest 绿）  
**源码：** `lulu-workbench-android/lulu-workbench/`  
**约束：** `docs/architecture/ui-layer-constraints.md`

## 1. 目标

Settings、Bind、Stage 各有自己的 `ViewModel`。页面状态用 `StateFlow`，唯一写入口是 `dispatch`。Activity 用 `viewModels()` 取。Store 删掉。

## 2. 锁定

| # | 题目 | 选择 |
|---|---|---|
| 1 | 一块一个状态机 | Bind、Settings、Stage 各一个 ViewModel，不合并 |
| 2 | 持有 | `MutableStateFlow`，不 import Compose runtime |
| 3 | 副作用 | Commands 仍单独注入 |
| 4 | Bind 监听 | `onCleared` 里拆 keepAlive |
| 5 | Stage 入参 | 生产从 `SavedStateHandle` 读 Intent extras；测试仍直接传入 id |
| 6 | 控件状态 | 输入草稿、确认框留在 Composable |

## 3. 步骤

1. 三块 Store 迁入对应 ViewModel，Activity 改为采集 `StateFlow`。  
2. Commands 增加 Hilt 构造。  
3. 单测改类型与 `state.value`。  
4. 删除三个 Store。  
5. 三块单测绿。

## 4. 明确不做

不改 Chat。不改 `:agent` 的 `StageStore` / `HistoryStore`。不把三块收成一个 ViewModel。
