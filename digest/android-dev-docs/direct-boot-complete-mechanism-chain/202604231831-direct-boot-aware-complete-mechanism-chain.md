# directBootAware 的完整机制链 — 摘要

> **导航**：[raw](../../../raw/android-dev-docs/direct-boot-complete-mechanism-chain/202604231831-direct-boot-aware-complete-mechanism-chain.md)


## 概述

本文从 directBootAware 的字面含义出发，推导出它依附于 FBE 机制：存储被拆分为 Device-Protected（开机即可访问）和 Credential-Protected（解锁后才可访问）两个区域，开机到首次解锁之间就是 Direct Boot 阶段，只有标记了 directBootAware=true 的组件可以在此阶段运行。这一标记是系统准入凭证，但系统不验证组件是否真的只用 DE 存储——违规访问 CE 存储会在运行时直接崩溃，ContentProvider 因最先执行而成为最容易崩溃的位置。
