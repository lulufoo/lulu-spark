# Chat：Store 并入 ViewModel + StateFlow

**状态：** 已实施（`:app` compileDebugKotlin + testDebugUnitTest 绿）  
**源码：** `lulu-workbench-android/lulu-workbench/`  
**决策来源：** 2026-09-21 会话目标架构；Chat 先做，其余页不动

## 1. 目标

Chat 页不再有独立 `ChatStore`。`ChatViewModel` 是 MVI 状态机：持一条 `StateFlow<ChatState>`，唯一写入口 `dispatch(ChatIntent)`，副作用走 `ChatCommands`。UI 用 `collectAsStateWithLifecycle` 采集。

✅ Verified（现 `ChatViewModel` 只组 `ChatStore`；`ChatStore` 用 `var state by mutableStateOf`；`ChatScreen` 直接读 `store.state`）

## 2. 锁定

| # | 题目 | 选择 |
|---|---|---|
| 1 | 范围 | 只改 Chat。Settings / Bind / Stage 仍是 Store + `mutableStateOf` |
| 2 | 枢纽 | `ChatViewModel` 吸收 `ChatStore` 行为；删除 `ChatStore` |
| 3 | 持有 | `MutableStateFlow` + `update` / 赋值；不 import Compose runtime |
| 4 | 入口 | `dispatch(intent)`；不抽独立 `reduce` 类型；不上 `UiEffect` |
| 5 | Commands | 仍单独注入；不并进 ViewModel 方法表 |
| 6 | 测试 | 直接 `ChatViewModel(ChatCommands(...))`，默认同步调度；不上 `HiltTestApplication` |
| 7 | 调度 | 生产仍 `Thread` + `Handler`；可注入 `runOffMain` / `runOnMain` |
| 8 | keepAlive | 生产 `start()`；`onCleared` 是否停仍不改 |

未锁：`ImmutableList`、精准传参、`viewModelScope`。本期不做。

## 3. 形状

```
Compose → dispatch(ChatIntent) → ChatViewModel
ChatViewModel → ChatCommands → :agent / :asr / :wmcp
ChatViewModel ─ StateFlow<ChatState> → collectAsStateWithLifecycle → Compose
```

Hilt 构造：`(ChatCommands, WorkbenchRuntime)`，内部组 keepAlive 与线程调度。  
测试构造：`(commands, keepAlive, runOffMain, runOnMain)`，默认 `IdleKeepAlive`、同步执行。

✅ Verified（现 `ChatStore` 已是这组参数；`ChatViewModel` 已 `@HiltViewModel` + `@Inject`）

## 4. 步骤

1. 把 `ChatStore` 体迁入 `ChatViewModel`；`mutableStateOf` 换成 `MutableStateFlow`。  
2. `ChatScreen` 收 `ChatViewModel`，采集 `state`；点击里 `dispatch`。  
3. `MainActivity` 打开 Stage 时读 `viewModel.state.value.sessionId`。  
4. 单测改类型与 `state.value`；类名跟 `ChatViewModel`。  
5. 删除 `ChatStore.kt`。  
6. `:app:testDebugUnitTest` 绿。

## 5. 明确不做

不改 Settings / Bind / Stage。不拆 UseCase / Repository。不上 `UiEffect`。不把 `Thread` 换成协程。不改 `ChatCommands` 行为。不改 `:agent` 以下。
