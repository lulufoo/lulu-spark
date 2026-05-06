# Android priv-app 特权权限机制 — 摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/android-priv-app-permission-mechanism/202604231831-android-priv-app-permission-mechanism.md)


## 概述

本文从「/system/app 与 /system/priv-app 有什么不同」出发，推导出区别来源于 PackageManagerService 扫描时对目录的标记——priv-app 路径赋予 FLAG_PRIVILEGED，使 APK 可以申请特权权限组。Android 8.0 后还需在 /etc/permissions/ 下放置 privapp-permissions XML 白名单文件明确声明，两者缺一不可；放错目录或缺少 XML 均静默失败，运行时才暴露缺权限。Zygote/AMS 是框架进程，不走 APK 打包流程，不适用这套机制。
