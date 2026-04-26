# directBootAware 的完整机制链 — 要点摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/system-principles/direct-boot-aware-complete-mechanism-chain-distilled.md)

## 要点

1. **Direct Boot 阶段是 FBE 机制在时间维度上的自然产物**
   FBE 将存储分为 DE（Device-Protected，开机即可访问）和 CE（Credential-Protected，用户解锁后才可访问）。"开机到用户首次输入锁屏密码"这段时间就是 Direct Boot 阶段，不是独立的启动流程，是 FBE 的必然中间状态。

2. **directBootAware=true 的双重性质：系统准入凭证 + 开发者单向声明**
   AMS 分发 `ACTION_LOCKED_BOOT_COMPLETED` 广播时主动过滤，只有标记为 true 的组件才收到（系统强制）。但系统不验证组件是否真的只用了 DE 存储，违规的后果是运行时崩溃，不是系统阻止（开发者自我约束）。

3. **ContentProvider 是 Direct Boot 下最容易崩溃的位置**
   ContentProvider 的 `onCreate()` 比 `Application.onCreate()` 先执行。若 ContentProvider 没有声明 `directBootAware=true` 但所在进程在 Direct Boot 阶段被拉起，访问默认存储路径（CE 存储）会立即崩溃。访问 DE 存储必须显式调用 `createDeviceProtectedStorageContext()` 切换上下文。
