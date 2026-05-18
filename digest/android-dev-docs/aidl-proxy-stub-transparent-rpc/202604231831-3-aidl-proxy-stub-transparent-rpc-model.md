# AIDL Proxy-Stub 透明 RPC 模型 — 要点摘要

> **导航**：[raw](../../../raw/android-dev-docs/aidl-proxy-stub-transparent-rpc/202604231831-3-aidl-proxy-stub-transparent-rpc-model.md)


## 要点

1. **透明性的实现全在 asInterface() 的分流逻辑**
   `queryLocalInterface()` 返回非空→同进程→直接返回 Stub 真身，不走 Binder 驱动；返回 null→跨进程→创建 Proxy 对象。调用方始终持有 `IXxx` 接口引用，背后是 Stub 还是 Proxy 由 `asInterface()` 在绑定时静默决定，这是"透明"的本质。

2. **透明性的三个层面与失效边界**
   位置透明（不知道服务在哪个进程）、实现透明（不关心服务端实现细节）、协议透明（不关心传输协议）。失效边界：版本不对齐时 method code 映射错乱（Server 插入新方法 C 后，Client 调 B 的 code=2 实际触发 C）；`FLAG_ONEWAY` 在同进程/跨进程行为差异也会使透明性穿帮。

3. **Proxy 持有链：Proxy → BinderProxy → BpBinder（JNI）→ binder 驱动句柄**
   这是跨进程路径。同进程路径则是直接的 Stub 对象。两条路径的切换点是 `asInterface()` 的判断，对调用方代码完全不可见。
