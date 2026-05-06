# directBootAware 的完整机制链

> 本文档基于一次 LCCM 引导对话整理，目标是重建理解的过程——跟着推导走一遍，而不是直接读结论。


> **导航**：[digest](../../../digest/android-dev-docs/direct-boot-complete-mechanism-chain/202604231831-direct-boot-aware-complete-mechanism-chain.md)
---

## 对话目标与边界

**学习目标**：理解 `directBootAware` 的本质机制、存储边界、组件行为及实际兼容设计。  
**主动绕过的内容**：Activity / ContentProvider / Service 各类型 `directBootAware` 行为差异（后续备用方向）；安全代价权衡。  
**下一步方向**：用户解锁后的状态切换通知机制（已在本对话覆盖）；`directBootAware` 兼容架构的完整设计方案（已在本对话覆盖）。

---

## 一、directBootAware 是什么——从字面到本质

最初的直觉是：`directBootAware` 是一个组件标签，`true` 表示该组件"知道系统启动这件事"，可以在系统重启并处于加密状态时被允许启动。这个理解抓住了使用场景，但还不够精确。

它依附于 Android 7.0 引入的 **File-Based Encryption（FBE）** 机制。FBE 将设备存储拆分为两个区域：

| 存储区域 | 名称 | 解密时机 |
|---|---|---|
| Device-Protected (DE) | 设备保护存储 | 开机即可访问 |
| Credential-Protected (CE) | 凭据保护存储 | 用户输入锁屏密码后才可访问 |

系统开机后、用户首次输入锁屏密码之前的这段时间，就是 **Direct Boot 阶段**。这是一个启动状态（boot state），而不是独立的启动流程——它是 FBE 机制在时间维度上的自然产物：

```
FBE（机制）
 └─ 将存储分为 DE / CE 两层
      └─ 开机到解锁之间产生"中间状态"
           └─ 这个状态 = Direct Boot 阶段
                └─ directBootAware 标记哪些组件可以在此阶段运行
```

---

## 二、directBootAware 只是一种声明吗

一个自然的质疑：这个标签是否只是开发者对系统的"声明"，背后没有实际约束？

答案是否定的，但约束的方向需要细分。

**系统执行的部分**：在 Direct Boot 阶段，`ActivityManagerService` 分发 `ACTION_LOCKED_BOOT_COMPLETED` 广播时，会主动过滤——只有 `directBootAware=true` 的组件才会收到，其余组件即便注册了也不会被唤起。这是系统强制隔离，不依赖开发者自觉。

**开发者自我约束的部分**：即使组件被启动，系统不会验证它是否真的只用了 DE 存储，也不会拦截对 CE 存储的访问。违规的后果是运行时崩溃，而不是系统主动阻止。

因此更准确的定位是：

```
directBootAware=true
 ├─ 对系统：准入凭证（组件调度的过滤条件）
 └─ 对开发者：单向声明 + 运行时自我约束
      └─ 系统信任声明，允许启动；但不验证实现是否合规
```

---

## 三、存储边界与运行时崩溃

CE 存储在 Direct Boot 阶段并非"不存在"，而是已加密、尚未解密。当组件访问默认存储路径（`SharedPreferences`、`getFilesDir()`、数据库等）时，底层指向 CE 存储，系统会抛出异常（通常是 `IllegalStateException` 或底层 `IOException`）。

要访问 DE 存储，需要显式切换上下文：

```java
Context deContext = context.createDeviceProtectedStorageContext();
SharedPreferences prefs = deContext.getSharedPreferences("config", MODE_PRIVATE);
```

声明了 `directBootAware=true` 的组件必须保证每一条数据读写路径都走 DE 上下文，系统不会自动切换。

---

## 四、ContentProvider 的特殊风险

ContentProvider 在进程启动时的初始化顺序早于 `Application.onCreate()`：

```
Application.attachBaseContext()
 → ContentProvider.onCreate()   ← 先于 Application.onCreate()
 → Application.onCreate()
```

当一个 `directBootAware=true` 的组件（如 BroadcastReceiver）收到 `ACTION_LOCKED_BOOT_COMPLETED` 广播后，进程被拉起，进程内所有 ContentProvider 都会按正常顺序初始化——**包括没有声明 `directBootAware=true` 的那些**。系统不会推迟或跳过它们。

这里有一个重要的风险矩阵：

| ContentProvider 声明 | Direct Boot 阶段行为 | 风险 |
|---|---|---|
| 未声明 / `false` | 系统不会主动拉起其所在进程；但进程一旦启动，仍会初始化 | 进程被其他组件拉起时存在风险 |
| `true` + 只用 DE 存储 | 立即初始化，正常运行 | 无 |
| `true` + 访问 CE 存储 | 立即初始化，运行时崩溃 | 💥 高 |

最隐蔽的场景：第三方 SDK 通过 ContentProvider 做自动初始化（如 AndroidX App Startup 等常见模式），SDK 开发者为适配 Direct Boot 加了标记，但没有完整审查所有存储访问路径。

正确的防护责任在 ContentProvider 的实现层：

```java
// ContentProvider.onCreate() 内部
UserManager um = context.getSystemService(UserManager.class);
if (um.isUserUnlocked()) {
    initCredentialStorage();
} else {
    initDeviceStorage();
    // 注册监听，等待解锁后补充初始化
}
```

---

## 五、解锁后的状态切换——广播与主动检查

用户解锁后，系统发出三条容易混淆的广播：

| 广播 | 触发时机 | 本质 |
|---|---|---|
| `ACTION_LOCKED_BOOT_COMPLETED` | 进入 Direct Boot 阶段 | 存储尚未解锁 |
| `ACTION_USER_UNLOCKED` | 用户输入密码，CE 解密完成 | **存储解锁事件** |
| `ACTION_USER_PRESENT` | 用户解除屏幕锁定（滑动也触发） | **屏幕可见事件** |
| `ACTION_BOOT_COMPLETED` | 系统完全启动 | 解锁后才发出 |

判断"CE 是否可访问"应监听 `ACTION_USER_UNLOCKED`，而非 `ACTION_USER_PRESENT`。两者在有密码场景下时序接近，但语义不同——`USER_PRESENT` 是屏幕解锁，不等于存储解锁，用错了是偶然工作而非必然正确。

接收 `ACTION_USER_UNLOCKED` 的 BroadcastReceiver 本身也必须声明 `directBootAware=true`，否则 Direct Boot 阶段进程内无法激活它。

**只依赖广播不够可靠**。广播在进程未启动时动态注册收不到，且时序不确定——广播回调执行时，其他组件可能已经开始使用未完成初始化的对象。更健壮的做法是广播与主动检查双保险：

```java
// 每次 onResume / onStartCommand 等生命周期入口
if (UserManager.isUserUnlocked() && state != FULLY_READY) {
    initCredentialStorage(); // 补充初始化
}
```

---

## 六、半初始化问题与架构层面的解法

一个真实遇到的问题：Direct Boot 阶段 App 被唤起，`Application.onCreate()` 只能完成 DE 部分的初始化。用户解锁后，`Application` 不会重新启动，CE 相关初始化永远不会被触发。此时对外的 Service 被第三方 App 唤起，在初始化一半的状态下进入了业务流程。

在每个业务入口单独做判断是一种打补丁的方式，容易遗漏。根本原因在于：

```
初始化入口：一个（Application onCreate）
初始化完成时机：不确定
业务入口：多个
防护逻辑：分散在各处 → 遗漏风险高
```

收拢的方向是把"初始化状态"变成一个可查询的对象，防护逻辑上移到基类：

```java
// 初始化状态管理器
public class AppInitManager {
    public enum State { DEVICE_READY, FULLY_READY }
    private static volatile State currentState = null;

    public static void init(Context context) {
        initDeviceStorage(context);
        currentState = State.DEVICE_READY;

        UserManager um = context.getSystemService(UserManager.class);
        if (um.isUserUnlocked()) {
            initCredentialStorage(context);
            currentState = State.FULLY_READY;
        } else {
            listenForUnlock(context);
        }
    }

    public static synchronized void onUserUnlocked(Context context) {
        if (currentState == State.FULLY_READY) return; // 防重入
        initCredentialStorage(context);
        currentState = State.FULLY_READY;
    }

    public static boolean isFullyReady() {
        return currentState == State.FULLY_READY;
    }
}
```

所有 Service 继承统一基类，防护写一次，新增入口自动受保护：

```java
public abstract class BaseService extends Service {
    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (!AppInitManager.isFullyReady()) {
            handleNotReady(intent);
            return START_NOT_STICKY;
        }
        return onStartCommandReady(intent, flags, startId);
    }
    protected abstract int onStartCommandReady(Intent intent, int flags, int startId);
    protected void handleNotReady(Intent intent) { /* 降级或排队 */ }
}
```

整体结构：

```
Application onCreate
 └─ AppInitManager.init()
      ├─ DE 初始化（同步完成）
      └─ CE 初始化（条件完成 / 等待解锁）

解锁事件（广播 + 生命周期主动检查）
 └─ AppInitManager.onUserUnlocked()
      └─ CE 初始化完成 → 执行待处理队列

所有业务入口（Service / Activity / BroadcastReceiver）
 └─ 继承基类 / 走拦截层
      └─ 查询 AppInitManager 状态
           ├─ FULLY_READY → 正常执行
           └─ 未就绪 → 降级 / 排队
```

---

## 对话中出现的误解（回答者视角）

> 以下是本次对话推导过程中出现的明确偏差，记录在此供后续参考。

| 误解 | 准确表述 |
|------|---------|
| 未声明 `directBootAware` 的 ContentProvider 在 Direct Boot 阶段会被系统"延迟初始化" | 系统不会主动拉起未声明 `directBootAware=true` 的组件所在进程；但一旦进程因其他原因被拉起，该进程内所有 ContentProvider 都会立即初始化，无法跳过 |
| `directBootAware=true` 是开发者与系统之间的"双向契约" | 更准确的表述是单向声明：系统信任声明并允许启动，但不验证实现是否合规；违规后果是运行时崩溃，不是系统主动拦截 |

---

## 遗留问题

1. Activity 和 Service 声明 `directBootAware=true` 后，行为与 BroadcastReceiver 的差异是什么？（备用切入方向，优先级：低）
2. 允许组件在加密前启动带来了哪些安全代价？系统如何设计边界来控制攻击面？（备用切入方向，优先级：低）
3. `runWhenReady` 队列在极端情况下（如队列堆积、解锁事件丢失）的健壮性处理方案。（优先级：中）

---

## 对话质量诊断

### 对话质量

| 维度 | 评级 | 说明 |
|------|------|------|
| 推导过程完整度 | 高 | 对话中有多处主动质疑：对"只是声明"的质疑、对"延迟初始化"说法的质疑、对广播可靠性的质疑 |
| 结论直给比例 | 低 | 绝大多数结论通过推导路径得出，直接给结论的片段较少 |
| 关键转折覆盖度 | 高 | 所有关键转折（声明 vs 约束、延迟初始化的修正、广播不可靠性）均有完整对话记录 |

**综合评级**：高质量

### 模型适用性

**适用性**：完全适用

对话具有明确的疑问→推导→落点主线，多处由用户主动质疑触发推导转折，认知推进路径清晰完整。