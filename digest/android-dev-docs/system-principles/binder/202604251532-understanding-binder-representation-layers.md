# 理解 Binder 的多层表示形态 — 摘要

> 创建时间：2026年4月25日 15:32


> **导航**：[distilled](../../../../distilled/android-dev-docs/system-principles/binder/202604251532-understanding-binder-representation-layers.md) | [raw](../../../../raw/android-dev-docs/system-principles/binder/202604251532-understanding-binder-representation-layers.md)


## 概述

对话从理解 Binder 在不同层次的表示形态出发，推导出同一个 Binder 实体在传输过程中经历多次形态转换：Java 层的 IBinder/BinderProxy、Native 层的 BBinder（服务端实体）和 BpBinder（客户端代理，持有 handle），以及内核驱动层的 binder_node（服务端）和 binder_ref（客户端引用）。关键转折是厘清了 flat_binder_object 只是传输载体，驱动拦截后将 BINDER_TYPE_BINDER 改写为 BINDER_TYPE_HANDLE 发给 Client——Client 永远收不到 BBinder 原始指针，只拿到 handle，这是 Binder 安全隔离的核心机制。
