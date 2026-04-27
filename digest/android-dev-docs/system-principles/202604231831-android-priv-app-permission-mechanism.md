# Android priv-app 特权权限机制 — 要点摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/system-principles/202604231831-android-priv-app-permission-mechanism.md)


## 要点

1. **APK 放置目录决定 FLAG_PRIVILEGED 标记，放错目录静默丢弃权限**
   PackageManagerService 扫描 APK 时检查路径，位于 `/system/priv-app` 等特权目录才打上 `FLAG_PRIVILEGED`。APK 若放在 `/system/app`，声明再多 `signatureOrSystem` 级权限也会在安装时静默丢弃，不报错，运行时才暴露。

2. **Android 8.0 后 privapp-permissions.xml 是必要且充分条件**
   白名单文件必须同时满足：APK 在特权目录（必要）、权限在 XML 中显式声明（充分）。两者缺任何一个权限都不会被授予。漏掉 XML 声明是常见部署错误，现象和放错目录完全一样：安装无报错，运行时缺权限。

3. **Zygote/AMS 是框架进程，不在 priv-app 目录**
   `priv-app` 存放的是以 APK 形式打包的特权系统应用（如系统拨号器、设置、短信等）。Zygote 和 AMS 是 Android 框架的一部分，以 DEX/native 形式存在于 `/system/framework`，不走 APK 打包和 PMS 扫描流程，也不存在 priv-app 授权问题。
