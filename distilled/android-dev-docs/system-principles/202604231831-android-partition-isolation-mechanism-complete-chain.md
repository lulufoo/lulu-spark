
# Android 分区结构与隔离机制的完整链条

> 本文档基于一次 LCCM 引导对话整理。
> 目标是重建理解的过程——跟着推导走一遍，而不是直接读结论。


> **导航**：[digest](../../../digest/android-dev-docs/system-principles/202604231831-android-partition-isolation-mechanism-complete-chain.md)
---

## 对话目标与边界

**学习目标**：理解 Android 分区结构，重点是隔离机制（权限隔离）如何在分区结构上实现。
**主动绕过的内容**：Dynamic Partitions（super 分区机制）、system_ext 分区定位、Recovery 分区、AVB 签名细节——均识别到但主动推迟，当前对话未深入。
**下一步方向**：Dynamic Partitions、system_ext 分区定位、Magisk overlay 机制原理、AVB 深入。

---

## 分区是物理概念还是逻辑概念？

最初的直觉是：分区像是"给存储划了区域，每个区域有个入口"——这个方向对，但需要在物理和逻辑两个层次上分别定位。

物理层面，Android 设备的 UFS/eMMC 芯片在出厂或刷机时，由分区表（GPT）切割成若干连续的物理块，每个块承载特定职责：

```
UFS/eMMC 芯片（物理存储）
├── block/by-name/bootloader
├── block/by-name/boot_a / boot_b
├── block/by-name/init_boot_a / init_boot_b
├── block/by-name/system_a / system_b
├── block/by-name/vendor_a / vendor_b
├── block/by-name/product_a / product_b
└── block/by-name/userdata
```

"boot 分区"这个说法是人理解时用的统称，物理上不存在一个叫 boot 的块里面装着 boot_a 和 boot_b——**物理上就是两个独立的块 boot_a 和 boot_b**。

---

## A/B Slot：为什么 bootloader 不进 slot？

A/B slot 的设计是：系统正在 slot_a 运行时，OTA 静默写入 slot_b，重启后切换，失败自动回退。所有系统分区（boot、init_boot、system、vendor、product）都有 _a / _b 两份，**启动哪个 slot，就全套用那个 slot 的版本**。

但 bootloader 本身不参与 A/B——因为它是"决定切换哪个 slot 的决策者"本身。如果 bootloader 也在 slot 里，出问题时没有任何东西能救它。userdata 同样只有一份，不进 slot，因为它存的是用户数据，不能因为系统切换 slot 就回滚或丢失。

---

## 挂载：分区如何变成可访问的路径

分区在物理上存在，但没有挂载之前，没有任何路径可以访问它的内容。挂载的本质是把一个物理块**映射到目录树的某个路径**：

```
/dev/block/sda3（物理块）
      ↓  mount
/system（目录树路径）
```

挂载之后，所有访问 `/system/` 的操作，内核都会转发到那个物理块上。**Mount Flags 是挂载时附加的属性**，直接约束这个访问入口的行为：

| Flag | 含义 |
|------|------|
| `ro` | 只读，写操作内核直接拒绝 |
| `noexec` | 禁止执行该分区上的二进制文件 |
| `nosuid` | 禁止该分区上的 suid 提权 |
| `nodev` | 禁止该分区上的设备文件生效 |

`system` / `vendor` / `product` 挂载时同时带 `ro, nodev, nosuid`——不只是只读，连提权路径也一并堵死。`ro` 的限制发生在内核 VFS 层，**早于 DAC 和 SELinux 检查**，是最先触发的一道墙。

---

## DAC：权限记录在文件上，不在进程上

理解 DAC（自主访问控制）时，有一个直觉上容易混淆的地方：进程对不同文件的权限不一样，那"进程有没有权限"这件事，是进程自己记录的吗？

不是。**权限记录在文件上**，每个文件自身存储三组 rwx：

```
文件自身存储：
  owner UID: u0_a65
  rwx------（owner可读写执行，其他人无权限）
```

访问时内核当场比对：

```
进程 UID == 文件 owner UID？
    ↓ 是 → 用 owner 的 rwx 判断
    ↓ 否 → 进程 GID == 文件 group？
              ↓ 是 → 用 group 的 rwx 判断
              ↓ 否 → 用 other 的 rwx 判断
```

类比：文件是锁（锁上写着"只有钥匙编号65能开"），进程是人（手里拿着编号65的钥匙），内核是保安（每次开门前核对）。**权限记录在门锁上，不是记录在人身上。**

### Android 对 UID 语义的重新定义

在桌面 Linux 上，UID 代表"登录系统的人"。Android 沿用了 UID 机制，但**重新映射了语义**：手机只有一个使用者，所以 UID 被用来区分每个 App——

```
安装 com.example.app  → 分配 UID: u0_a65
安装 com.another.app  → 分配 UID: u0_a66
system_server         → UID: 1000（system）
```

同一个人操作手机，每个 App 跑在自己的 UID 下。**人没有 UID，App 有 UID。**

UID=0 是 root，DAC 体系里的特殊身份——内核对 UID=0 完全放行，无视 rwx。这意味着 root 能绕过 DAC，但绕不过 SELinux（下一层独立验证）。

### 共享 UID 的含义与边界

某些系统服务共享 UID=1000（system）。在 DAC 层面，共享 UID 的进程确实互信——访问 owner=1000 的文件时全部放行。但共享 UID **不等于完全互信**，因为 SELinux 会进一步区分它们：即使两个进程都是 UID=1000，它们的 SELinux 标签可以不同，策略可以规定它们不能互相访问。

---

## SELinux：标签粒度的独立验证

DAC 是"分区粗粒度 + 文件细粒度"的身份比对，SELinux 是在此之上的第三层，**粒度最细、也最灵活**。

每个进程、每个文件、每个分区都有 SELinux 标签，访问规则由策略文件定义：

```
DAC 检查通过
    ↓
SELinux 检查：进程标签 是否允许访问 文件标签？
    ↓ 通过
访问成功
```

两层都要通过，缺一不可。**UID=0（root）无法绕过 SELinux**——这是为什么 Magisk 在获取 root 之后还要同时修改 SELinux 策略，否则即使有 UID=0，SELinux 仍会拦截特权操作。

### 策略文件规模问题

SELinux 策略的粒度是**标签类型（type）**，不是具体文件路径。所有打了同一个标签的文件，共享同一条规则——策略规模是标签类型数量，不是文件数量。路径到标签的映射由 `file_contexts` 统一管理，用正则批量覆盖：

```
/system(/.*)?           u:object_r:system_file:s0
/data/data/([^/]+)(/.*)? u:object_r:app_data_file:s0
```

Android 10+ 之后，SELinux 策略也跟随 Treble 分层：Google 维护 platform 策略（`/system`），芯片厂商维护 vendor 策略（`/vendor`），编译时合并，运行时加载。

---

## dm-verity：启动前的分区完整性校验

前三层（Mount Flags、DAC、SELinux）都是运行时的访问控制，dm-verity 解决的是另一个问题：**分区内容在启动之前有没有被篡改？**

dm-verity 不是在 bootloader 阶段运行，而是在 **kernel 启动后、分区挂载前**，由内核的 device-mapper 子系统执行。boot 分区里的 kernel 提供 dm-verity 能力，init_boot 分区里的 init 进程读取 vbmeta 后驱动校验执行——两者协作完成。

校验用的不是整体 MD5，而是 **SHA-256 Merkle Tree（哈希树）**：

```
分区内容按 4KB 分块
    ↓
每块计算 SHA-256 哈希值
    ↓
哈希值逐层计算，形成树状结构
    ↓
最顶层：Root Hash（根哈希）

        [ Root Hash ]
        /           \
  [ Hash A ]     [ Hash B ]
   /      \       /      \
[块1]   [块2] [块3]   [块4]
```

用树而不是整体哈希，是因为 system 分区有几个 GB，整体校验需要全量读取才能验证。Merkle Tree 支持**按需校验**：读哪个块，只校验那条路径上的哈希，不需要全量读取，启动速度可接受。

Root Hash 存储在 vbmeta 分区，vbmeta 由 OEM 私钥签名，bootloader 内部烧录了对应的 OEM 公钥（部分设备写入硬件 fuse，不可修改）——这是整个信任链的硬件锚点。

---

## 完整信任链

```
硬件 fuse（OEM公钥，出厂烧录，不可修改）
    ↓ 验证
bootloader 验证 vbmeta 签名
    ↓ 信任
vbmeta 内容（包含各分区 Root Hash）
    ↓ 交给
dm-verity（kernel）校验各分区内容
    ↓
init 挂载经过验证的分区
    ↓
系统启动
```

unlocked bootloader 本质是**主动放弃了信任链的起点**——bootloader 跳过或放宽签名验证，允许自定义 vbmeta（替换 Root Hash），代价是启动时显示橙色警告屏，系统无法再保证分区内容的可信性。

---

## 反向验证：预装 App 为什么无法被用户卸载？

预装 App 存放在 `/system/app/` 或 `/product/app/` 下。用户点击"卸载"，本质是删除文件，但四层隔离机制逐层拦截：

| 层次 | 障碍 | 绕过方式 |
|------|------|---------|
| Mount Flags | 分区 `ro`，内核 VFS 层直接拒绝写操作 | `adb remount`（需 unlocked bootloader + UID=0） |
| DAC | 文件 owner=root，需要 UID=0 | `adb root` |
| SELinux | 进程标签需有 `system_file:unlink` 权限 | Magisk 修改 SELinux 策略 |
| dm-verity | 分区内容变化，下次启动校验失败 | 关闭 dm-verity，或 Magisk overlay 绕过 |

"禁用"之所以可以——它只在 `userdata` 里写一个标记，物理文件完全没动，四层都不触发。

**Magisk 的核心思路**是不修改任何分区的物理内容，在挂载层用 overlay 文件系统覆盖——系统看到的路径里没有该 App，但四层隔离机制没有一层被破坏，dm-verity 自然不触发。这是一个在隔离层之上再加一层覆盖的设计，而不是逐层击破。

`adb root` 和 `adb remount` 是解决**两个不同问题**的独立操作：`adb root` 改变 adbd 的 UID（身份问题），`adb remount` 修改分区挂载属性（挂载属性问题）。两者没有因果关系，是并列的前提条件。

---

## 对话中出现的误解（回答者视角）

> 以下是本次对话推导过程中出现的明确偏差，记录在此供后续参考。

| 误解 | 准确表述 |
|------|---------|
| "即使 UID=0，如果进程 SELinux 标签不是 su 或 shell（且策略不允许），remount 仍会被拒绝。这是为什么 adb root 之后还需要 adb remount，而不是 root 之后自动可写。" | `adb root` 和 `adb remount` 解决的是两个独立问题，没有因果关系。root 之后不能自动可写，原因是分区仍然是 `ro` 挂载——`ro` 是内核挂载属性，和进程身份无关，必须单独执行 remount 修改挂载属性。SELinux 是第三层独立检查，不构成这个现象的原因。 |

---

## 遗留问题

1. **Dynamic Partitions**：system / vendor / product 在 Android 10+ 实际存储在 super 分区里，通过 dm-linear 虚拟设备挂载——这影响 `adb remount` 的行为，也是理解物理分区结构的重要补充。
2. **system_ext 分区定位**：已知 system / vendor / product，但 system_ext 的职责和与 system 的边界尚未覆盖。
3. **Magisk overlay 机制原理**：知道"用 overlay 文件系统覆盖挂载"，但具体实现路径（overlayfs 如何挂载、init 阶段在哪里介入）未深入。
4. **AVB 细节**：vbmeta 的内部结构、签名格式、rollback protection 机制未覆盖。
5. **Recovery 分区**：在 A/B 设备上的角色变化（virtual A/B）未覆盖。

---

## 对话质量诊断

### 对话质量

| 维度 | 评级 | 说明 |
|------|------|------|
| 推导过程完整度 | 高 | 对话中有大量主动猜测（挂载是"入口"、dm-verity 校验时机、Root Hash 存储位置、bootloader 内存储公钥）、多次主动质疑（UID 是谁的身份、分区是物理还是逻辑、回答者逻辑混乱的那句话） |
| 结论直给比例 | 低 | 绝大多数结论都有推导过程，AI 多次以问题驱动用户先猜测再校准 |
| 关键转折覆盖度 | 高 | 所有主要转折（mount 本质、UID 语义重定义、共享 UID 的边界、dm-verity 不在 bootloader 阶段、Merkle Tree 的设计动机）均有对话记录 |

**综合评级**：高质量

### 模型适用性

**适用性**：完全适用

对话有明确的疑问→推导→落点主线，用户持续主动猜测和质疑，认知转折点密集且有对话记录。

## 附录

```
Android 分区结构
│
├── 1. 物理划分（分区是什么）
│       每块物理存储区域承载特定职责
│       boot / system / vendor / product / userdata...
│
├── 2. 启动时序（分区怎么用）
│       bootloader → boot → init_boot → 挂载其余分区
│       A/B slot 保障更新安全回退
│       Treble 定义分区间接口边界
│
└── 3. 隔离机制（分区为何安全）
            Mount Flags            → 分区粒度
            DAC(Linux 文件权限)     → 文件粒度
            MAC(SELinux)           → 标签粒度
            dm-verity              → 启动前完整性（未完成）
```