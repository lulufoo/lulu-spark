# Android Parcel 与 Binder IPC 内部机制 — 要点摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/system-principles/android-parcel-and-binder-ipc-internals-distilled.md)

## 要点

1. **Java 层 Parcel 只是壳：所有读写经 JNI 穿透到 C++ android::Parcel**
   Java 的 `android.os.Parcel` 对象只持有一个 `long nativePtr`，数据存在 C++ 层的 `uint8_t* mData` 连续内存 buffer。Java 层的操作是入口，实际存储和运算全在 Native 层。

2. **Parcel 写入无类型标记、无长度字段：读写顺序必须严格对称**
   `writeInt32` 直接把 4 字节原始值写进 buffer，游标后移，没有任何结构信息。Parcel 本身无法告诉你"第 N 个字节是什么类型"，顺序约定完全由 Client 和 Server 双方自己维护。顺序错位会读出错误数据而不会报错。

3. **writeStrongBinder 写入两个位置：mData（数据流）+ mObjects（对象偏移索引）**
   写入普通数据只写 mData，写入 Binder 对象时同时写入 mObjects 记录该 Binder 在 buffer 中的偏移。驱动通过 mObjects 定位 `flat_binder_object`，在传输时拦截并改写（BINDER_TYPE_BINDER→BINDER_TYPE_HANDLE），实现跨进程 Binder 引用转换。
