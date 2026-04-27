
# Android priv-app 权限机制完整链路

> 本文档基于一次 LCCM 引导对话整理，目标是重建理解的过程——跟着推导走一遍，而不是直接读结论。


> **导航**：[digest](../../../digest/android-dev-docs/system-principles/202604231831-android-priv-app-permission-mechanism.md)
---

## 对话目标与边界

**学习目标**：理解 `/system/app` 与 `/system/priv-app` 的权限差异，以及预装 App 开发中权限授予的完整机制。
**主动绕过的内容**：Magisk systemless root 绕过 AVB 的具体实现细节——识别到这是独立话题，未展开。
**下一步方向**：Android 版本演进中权限机制的变化；安全角度的已知绕过方式。

---

## 两个目录到底有什么不同

起点是一个直觉：`/system/app` 和 `/system/priv-app` 都放 App，但后者的权限应该更高。这个方向是对的——但「更高」具体指什么，需要从 PMS 扫描机制往里看。

系统启动时，PackageManagerService 会扫描一组预设目录，对每个扫描到的 APK 记录它来自哪里：

```
系统启动
  └── PackageManagerService 初始化
        └── 扫描目录列表
              · /system/app/          → SCAN_AS_SYSTEM
              · /system/priv-app/     → SCAN_AS_SYSTEM | SCAN_AS_PRIVILEGED
              · /product/app/         → SCAN_AS_PRODUCT
              · /product/priv-app/    → SCAN_AS_PRODUCT | SCAN_AS_PRIVILEGED
              ...
                └── APK 物理路径决定 FLAG_PRIVILEGED 标记
```

**APK 放在哪个目录，决定了它被打上什么标记。** 同一个 APK，放 `/system/app/` 和放 `/system/priv-app/`，PMS 给出的 flag 不同，后续能获得的权限就不同。

这里有一个最初的错误猜测值得记录：`SystemServer`、`Zygote`、`ActivityManagerService` 这些核心组件，直觉上应该在 `priv-app` 里——但它们根本不是 APK。`Zygote` 是 `/system/bin/` 下的可执行文件，`AMS` 跑在 `system_server` 进程里，源码编译后打包进 `/system/framework/services.jar`，由 `system_server` 进程加载。`priv-app` 目录放的是**以 APK 形式打包、但需要特权权限的系统应用**，比如 `Settings.apk`、`PackageInstaller.apk`、`TeleService.apk`。

---

## 放错目录会发生什么

把需要 `privileged` 权限的 APK 放到 `/system/app/` 会发生什么？直觉上应该是运行时报「权限未 grant」的错误——但实际比这更难调试。

错误发生在**安装阶段而非运行时**：

```
安装时 PMS 扫描 APK
  └── 检测到申请 INSTALL_PACKAGES
        └── 检查：是否有 FLAG_PRIVILEGED？ → 否
              └── 权限静默丢弃，不报错
                    └── 运行时调用相关 API
                          └── SecurityException: uid xxx does not have INSTALL_PACKAGES
```

PMS 在安装阶段就静默丢弃了这个权限，不是「申请了但未授权」，而是根本没有进入授权流程。在 `PackageManager` 查询时，该权限的状态是 `PERMISSION_DENIED`。

---

## privileged 权限的第二道门

Android 8.0 之前，APK 在 `priv-app/` 目录就够了——它申请的所有 `privileged` 权限会被自动授予。Android 8.0 之后引入了第二道门：`privapp-permissions-xxx.xml`。

这个文件要求厂商**显式声明**每个 priv-app 被允许使用哪些权限：

```xml
<!-- /system/etc/permissions/privapp-permissions-xxx.xml -->
<permissions>
    <privapp-permissions package="com.example.app">
        <permission name="android.permission.INSTALL_PACKAGES"/>
    </privapp-permissions>
</permissions>
```

两个条件的关系：

```
priv-app/ 目录  →  FLAG_PRIVILEGED（必要条件）
      +
privapp-permissions.xml  →  显式白名单（充分条件）
      ↓
两者同时满足，权限才最终被授予
```

值得注意的是，这个文件只收录 `privileged` 级别的权限——普通权限（`normal` / `dangerous`）不需要在这里声明，走 `AndroidManifest.xml` 的正常流程就够。

Android 9.0 之后，白名单变为**强制执行**：APK 在 `priv-app/` 但 xml 未声明，直接拒绝授予，部分设备会导致启动异常。

---

## Treble 分区与 priv-app 目录的关系

Android 8.0 的 Treble 架构将系统拆分为多个**独立存储分区**。在此之前，`vendor` 代码混在 `/system/` 目录下，物理上同属一个 `system.img`；Treble 之后，各分区拥有独立镜像，可以独立刷写和 OTA：

```
Android 8.0 之前：
  所有系统代码（含 vendor）
    └── 混在 /system/ 目录下
          └── 物理上同一个 system.img

Android 8.0 Treble 之后（设备存储分区表简化）：
  ├── boot          → 内核 + ramdisk
  ├── system        → /system 挂载点       system.img      独立分区 ✓
  ├── vendor        → /vendor 挂载点       vendor.img      独立分区 ✓（Android 8.0）
  ├── product       → /product 挂载点      product.img     独立分区 ✓（Android 9.0）
  ├── system_ext    → /system_ext 挂载点   system_ext.img  独立分区 ✓（Android 11）
  ├── userdata      → /data 挂载点
  └── ...
```

每个分区都有自己的 `priv-app/` 子目录，PMS 对这四个目录都打 `FLAG_PRIVILEGED`，**权限等级完全等价**。分区拆分的目的不是权限差异，而是 OTA 隔离：各层可以独立升级，互不污染。

| 分区 | 引入版本 | 维护方 |
|------|----------|--------|
| `/system/priv-app/` | 早期 | Google / AOSP |
| `/vendor/priv-app/` | Android 8.0 | SoC 厂商 |
| `/product/priv-app/` | Android 9.0 | 设备厂商 |
| `/system_ext/priv-app/` | Android 11 | 厂商系统扩展 |

对于设备厂商合作预装的 App，放 `/product/priv-app/` 是最规范的选择——权限效果与 `/system/priv-app/` 完全一致，但不污染 system 分区。

xml 声明需要与 APK 分区配套：

```
/product/priv-app/your.apk
  → /product/etc/permissions/privapp-permissions-xxx.xml
```

---

## signature 权限与 privileged 的组合关系

`INSTALL_PACKAGES` 的 `protectionLevel` 是 `signature|privileged`——这里有一个容易产生的误解：厂商对预装 App「重新签名」，是否就自动满足了 `signature` 条件？

不是。`signature` 校验的是**与权限声明方的签名是否一致**：

```
android.permission.INSTALL_PACKAGES 在哪里声明？
  └── frameworks/base/core/res/AndroidManifest.xml
        └── 由 framework-res.apk 打包
              └── 使用「平台签名」（platform key）签名
```

厂商对预装 App 的「重新签名」，通常是用厂商自己的 release key，而不是 platform key。两者不同，`signature` 条件不满足。

但 `signature|privileged` 是**或**的关系——满足任一即可：

```
权限申请（INSTALL_PACKAGES）
  └── protectionLevel = "signature|privileged"
        ├── signature 路径：App 签名 == platform key？
        │     ├── 是 → 授予 ✓
        │     └── 否 → 继续检查
        └── privileged 路径：FLAG_PRIVILEGED 已标记？
              ├── 是 → 检查 xml 白名单
              │     ├── 已声明 → 授予 ✓
              │     └── 未声明 → 拒绝 ✗
              └── 否 → 拒绝 ✗
```

预装 App 合作场景，走 `privileged` 路径（目录 + xml）就能满足需求，不需要 platform key。需要 platform key 的场景是：App 需要与另一系统 App 共享 UID（`sharedUserId`），或申请 `protectionLevel="signature"`（无 privileged 分支）的权限。

---

## dangerous 权限的默认预授

`POST_NOTIFICATIONS`（Android 13 引入）是 `dangerous` 权限，不是 `privileged`——`priv-app/` 目录对它没有任何作用，走的是另一套机制。

AOSP 标准方式是 `default-permissions-xxx.xml`：

```xml
<!-- /system/etc/default-permissions/default-permissions-partner.xml -->
<exceptions>
    <exception package="com.partner.app">
        <permission name="android.permission.POST_NOTIFICATIONS"
                    fixed="false"/>
    </exception>
</exceptions>
```

- `fixed="false"`：用户仍可手动撤销
- `fixed="true"`：用户无法关闭

PMS 初始化时读取这个文件，对指定包名调用 `grantRuntimePermission()`，用户拿到手机时权限已默认授予。

这个配置必须预置进系统镜像——在预装合作中，这意味着需要向厂商提需求，跟随对方的版本节点发布，不是 APK 层面能自行完成的。

---

## AppOps：叠加在权限之上的运行时开关

`PACKAGE_USAGE_STATS` 的 `protectionLevel` 包含 `appop`——这类权限在满足 `privileged` 条件之外，还受一个独立的 AppOps 开关控制：

```
调用 queryAndAggregateUsageStats
  └── 系统检查 AppOps: OP_GET_USAGE_STATS
        └── 未授权 → 不抛异常，直接返回空数据或阻塞
```

`appop` 类权限未满足时不一定抛 `SecurityException`，可能**静默返回空或挂起**——比报错更难调试。

ROOT 设备上可通过 `adb shell appops set` 或 `pm grant` 直接绕过这个开关，这是「放进去就可以了」背后的原因之一。普通设备需要用户在「设置 → 有权查看使用情况的应用」里手动开启，或通过 adb 授权。

---

## 通过 adb root 修改权限的实际边界

在工程机或解锁设备上，`adb root + adb remount` 成功后，理论上可以修改任何 App 的权限配置——但实际有三道独立障碍：

**障碍 1：Verified Boot（AVB）**

量产锁定设备上，`adb root` 直接失败。解锁 Bootloader 后，修改 `/system` 分区内容重启时 AVB 会校验分区 hash，不匹配则启动失败或进入警告状态。工程机（userdebug/eng build）通常关闭 AVB 校验。

**障碍 2：SELinux**

即使 remount 成功，SELinux 独立控制文件操作权限。量产设备默认 `enforcing` 模式，`shell` 进程写 `/system/priv-app/` 会被拒绝。工程机通常是 `permissive` 模式。

```bash
adb shell getenforce   # 查看当前模式
```

**障碍 3：PMS 缓存**

文件写入成功后，PMS 不会实时感知，必须重启设备触发重新扫描，`FLAG_PRIVILEGED` 才生效。

完整操作链（工程机）：

```bash
adb root && adb remount
adb push your.apk /system/priv-app/
adb push privapp-permissions-xxx.xml /system/etc/permissions/
adb reboot
# 重启后 PMS 重新扫描，权限生效
```

`dangerous` 权限（如 `POST_NOTIFICATIONS`）有更快的方式，不需要修改分区：

```bash
adb shell pm grant com.your.app android.permission.POST_NOTIFICATIONS
```

ROOT 设备上直接生效，无需重启。

| 设备类型 | adb root | remount | 修改生效 |
|----------|----------|---------|----------|
| 量产锁定设备 | ✗ | ✗ | ✗ |
| 解锁 Bootloader + AVB 开启 | ✓ | ✓ | 重启后可能 AVB 失败 |
| 工程机 userdebug | ✓ | ✓ | 重启后 ✓ |
| Magisk ROOT 量产机 | ✓ | 部分 ✓ | 需额外处理 |

---

## 对话中出现的误解（回答者视角）

> 以下是本次对话推导过程中出现的明确偏差，记录在此供后续参考。

| 误解 | 准确表述 |
|------|---------|
| `SystemServer`、`Zygote` 在 `priv-app/` 目录下 | 它们是进程/可执行文件，位于 `/system/bin/` 或 `/system/framework/`，不是 APK，不经过 `app/` 或 `priv-app/` |
| `privapp-permissions.xml` 可以声明所有类型的权限 | 该文件只收录 `privileged` 级别权限；`normal` / `dangerous` 权限走 `AndroidManifest.xml` 正常流程 |
| 厂商「重新签名」= 满足 `signature` 权限条件 | `signature` 校验的是与权限声明方（platform key）是否同签名；厂商 release key 与 platform key 通常不同，不满足 `signature` 条件，走 `privileged` 路径 |

---

## 遗留问题

1. **（高）Android 版本演进关键节点**：8.0 前后权限机制的分水岭，及 13/14 的最新变化。
2. **（高）安全角度的已知绕过方式**：这套设计在供应链层面、已安装恶意 App 层面的实际防护边界。
3. **（中）Magisk systemless root 方案**：如何通过 overlay 挂载绕过 AVB 校验，不修改 `/system` 分区实现类似效果。
4. **（低）`sharedUserId` 与签名的关系**：需要 platform key 的具体场景及风险。

---

## 对话质量诊断

### 对话质量

| 维度 | 评级 | 说明 |
|------|------|------|
| 推导过程完整度 | 高 | 对话中有多处主动猜测（SystemServer 在 priv-app、重新签名即满足 signature 条件）、主动质疑（privapp-permissions.xml 只写高级权限）、边界确认（放错目录会发生什么） |
| 结论直给比例 | 低 | 大部分内容有推导过程，仅 Treble 分区结构等背景知识为直给 |
| 关键转折覆盖度 | 高 | 主要认知转折点均有对话记录：目录→flag 机制、静默丢弃、xml 第二道门、signature 误解校准 |

**综合评级**：高质量

### 模型适用性

**适用性**：完全适用

对话有清晰的疑问→推导→落点主线，用户有预装开发实际经验作为认知锚点，推导过程中多次出现有价值的错误猜测和主动质疑，蒸馏结果能有效重建理解路径。
