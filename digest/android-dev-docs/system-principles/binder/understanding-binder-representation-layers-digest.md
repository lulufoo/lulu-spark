# 理解 Binder 的多层表示形态 — 要点摘要

> **导航**：[distilled](../../../../distilled/android-dev-docs/system-principles/binder/understanding-binder-representation-layers-distilled.md) | [raw](../../../../raw/android-dev-docs/system-principles/binder/understanding-binder-representation-layers-normalized.md)

## 要点

1. **Binder 的四层形态：应用层 / SMgr 注册层 / 传输层 / 驱动层**
   应用层：Server 侧是 Stub（Java）/BBinder（C++），Client 侧是 Proxy（Java）/BpBinder+BinderProxy（C++ + JNI）；SMgr 层：注册服务名→IBinder 映射；传输层：`flat_binder_object`（跨进程 Binder 传输的载体）；驱动层：`binder_node`（Server 的 Binder 实体）/`binder_ref`（Client 引用，存 handle 编号）。

2. **BpBinder 属于 C++ Native 层（libbinder），不在 JNI 层**
   JNI 层是桥接（Java BinderProxy ↔ C++ BpBinder），BpBinder 本身是 Native 层的客户端代理对象，持有 handle（uint32_t）而非 BBinder 指针。BinderProxy 是 Java 侧的封装，两者不可互换。

3. **驱动拦截改写：Server 发出 BINDER_TYPE_BINDER，Client 收到 BINDER_TYPE_HANDLE**
   Server 发送 `flat_binder_object` 时 type=BINDER_TYPE_BINDER（含 BBinder 指针）；驱动拦截后在对方进程创建 `binder_ref`，改写 type=BINDER_TYPE_HANDLE（含 handle 编号）发给 Client。Client 永远收不到 BBinder 原始指针，只拿到 handle——这是 Binder 安全隔离的核心机制。
