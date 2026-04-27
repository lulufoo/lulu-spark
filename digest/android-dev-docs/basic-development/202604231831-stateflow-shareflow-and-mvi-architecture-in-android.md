# Android MVI 架构中 StateFlow、SharedFlow 与数据层设计 — 要点摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/basic-development/202604231831-stateflow-shareflow-and-mvi-architecture-in-android.md)


## 要点

1. **StateFlow vs SharedFlow 的语义边界：持续状态 vs 一次性事件**
   StateFlow 永远持有最新值，新订阅者立即收到当前值（适合 UI 状态）；SharedFlow replay=0 不补发历史（适合导航、Toast、错误弹窗等一次性事件）。将导航事件放入 StateFlow，用户按返回键重新订阅后会自动再次触发，这是边界失效的典型表现。

2. **ViewModel 不能持有 NavController：导航执行权必须在 View 层**
   Activity 销毁后 ViewModel 仍存活，持有 NavController 导致内存泄漏。即使导航看起来是业务动作，ViewModel 只能发出信号，由 View 层响应后调用 `navController.navigate()`。官方推荐方案是 `LaunchedEffect + SharedFlow`，这是约束下的妥协，不是最优解。

3. **MVI 单向数据流：Intent→ViewModel→State/Event→View**
   Intent（用户操作）由 View 发出，ViewModel 处理后发出 State（用 StateFlow）和 Event（用 SharedFlow）；View 消费 State 渲染界面，通过 `LaunchedEffect` 消费一次性 Event。副作用处理必须在 View 层，不能让 View 层引用流入 ViewModel。
