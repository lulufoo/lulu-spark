# Android Parcel 与 Binder IPC 内部机制 — 摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/system-principles/202604231831-android-parcel-and-binder-ipc-internals.md)


## 概述

本文从「Java 层 Parcel.writeInt() 数据存在哪里」出发，推导出 Java 层的 android.os.Parcel 只是持有 nativePtr 的壳，真正的数据存储和操作在 Native 层 android::Parcel 的 mData 连续内存 buffer 中。进而分析 mData 与 mObjects 的双缓冲结构：mData 存普通数据，mObjects 专门记录 Binder 对象的偏移位置；Parcel 写入无类型标记，读写顺序必须严格对称。最终穿透到 writeStrongBinder() 如何触发驱动层的 flat_binder_object 传输与引用转换。
