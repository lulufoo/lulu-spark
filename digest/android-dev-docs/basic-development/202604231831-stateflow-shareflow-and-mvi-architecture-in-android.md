# Android MVI 架构中 StateFlow、SharedFlow 与数据层设计 — 摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/basic-development/202604231831-stateflow-shareflow-and-mvi-architecture-in-android.md)


## 概述

本文从 StateFlow 必须有初始值出发，推导出「StateFlow 持有最新值」这一特性如何制造一次性事件的处理难题——登录成功导航事件若用 StateFlow 存储，重订阅时会重复触发。解法路径分析了 SharedFlow（replay=0）作为一次性事件容器，以及 MVI 架构下单向数据流的完整设计：Intent → ViewModel → State（StateFlow）/ Event（SharedFlow）→ View，副作用必须在 View 层消费，ViewModel 不可持有 NavController。
