# ReentrantLock：从可重入命名到 Mesa 骨架与 AQS 家族

> 本文档基于一次 LCCM 引导对话整理，目标是重建理解的过程——跟着推导走一遍，而不是直接读结论。

## 本次对话在追什么

对话从「知道 `ReentrantLock` 在 Java 并发包里」出发，按一条可重走的主线推进：

- 它与 `synchronized` 的分工：不是「谁才可重入」，而是**表达能力**（多 `Condition`、可中断等待、限时尝试、公平策略可选等）。
- AQS 上如何用少量字段支撑**可重入**与**并发安全**：`state` 与 `exclusiveOwnerThread`、CAS、CLH 队列与 `park`/`unpark` 的混合路径。
- **公平性**卡在哪一步：FIFO 队列仍在，差别在「新线程首次抢锁是否允许绕开已等待者」；并拆开 `synchronized` 里「新到者竞争」与 `_cxq` 次序两层。
- 为何自然走到 **Mesa** 与双队列：`ReentrantLock` + `Condition` 与 `while` 重检谓词的理由。
- 用 **Android 日常对象**做归类练习（原子类、协程 `Mutex`、SQLite 事务），把「尺子」落到你熟悉的栈。
- **理发店**跨域映射与一次 API 纠偏（`tryLock` vs `getQueueLength`），再从「三座位」跳到 **`Semaphore` 与 `state` 语义家族**。

文末保留对话里未展开的实现细节与练习方向，便于你单独补课。

---

## 它是什么：命名、可重入，以及「为什么还要一把锁」

一开始的直觉很合理：`Reentrant` 表示同一线程持锁进入临界区后还能再进（递归就是典型场景）；类名把这件事写进名字里，像是在强调主要特征。接着自然会把 `synchronized` 拉进来对比——两者都可重入，因此「可重入」并不是 `ReentrantLock` 相对 `synchronized` 的独占卖点。

这里曾出现一个需要当场校准的猜测：以为 `ReentrantLock` **默认公平**、而 `synchronized` 非公平。准确说法是：`ReentrantLock` **默认非公平**；只有 `new ReentrantLock(true)` 才是公平构造；`synchronized` 确实偏非公平一侧。把「公平与否」理解成**可选策略**，比理解成类的固定属性，更贴近设计意图。

与 `synchronized` 的真正对比轴，更接近「**表达力**」：`synchronized` 与对象监视器绑定，写法省事，但在「多种等待语义、可中断的等锁、先试再决定要不要排队」等组合需求上，会显得别扭。顺着这个方向，对话里归纳出：`ReentrantLock` 可以挂多个 `Condition`，把不同业务谓词的等待拆开；同时补充了 `tryLock` / 限时 `tryLock`、`lockInterruptibly` 等 `synchronized` 不易直接表达的能力。一句收束：`synchronized` 更像语法层面的「要 / 不要一把监视器锁」；`ReentrantLock` 把锁做成对象，让你能组合「试、等、中断、观测队列、选公平策略」。

公平策略在 API 上是显式的（默认非公平，传入 `true` 才是公平）：

```java
Lock nonFair = new ReentrantLock();
Lock fair    = new ReentrantLock(true);
```

`tryLock` 与「等锁可被中断」的最小形状（语义对照用，非生产模板）：

```java
if (lock.tryLock()) {
    try { /* 临界区 */ }
    finally { lock.unlock(); }
} else {
    /* 拿不到：降级、重试、记指标等 */
}

// 或带超时
lock.tryLock(timeout, unit);

lock.lockInterruptibly(); // 等锁过程中可响应 interrupt
```

---

## 内部靠什么站得住：持有者、重入深度、CAS 与等待路径

从「可重入」反推记录项，对话里落到两件必须记住的事：**当前持有者的标识**，以及**重入深度**。在 AQS 语境里，这对应 `exclusiveOwnerThread` 与 `state`：`state == 0` 表示无人占用；`state > 0` 时，数值本身就可以同时表达「被占用」与「嵌套了几层」——用同一整型字段兼表两种信息，有利于用一次 CAS 完成关键变迁，这是实现上的常见取舍。

并发下若两个线程都读到 `state == 0` 并各自写入，就会破坏互斥。对话里独立推到：需要 CAS 这类「先比较再写、整体原子」的语义，才能保证只有一个赢家。

`ReentrantLock` 并不是「纯自旋锁」叙事：更贴近工程现实的路径是「先 CAS 抢一次 → 失败则进入 CLH 变体队列 → 在队列头附近有限重试 → 最终 `park` 挂起」。`tryLock` 往往只走「快速尝试」那一截，所以能做到「拿不到就立刻返回或限时返回」，这与无限阻塞的默认 `lock()` 是不同的心智模型。

实现上，`ReentrantLock` 是薄外壳，核心在内部 `Sync`（`FairSync` / `NonfairSync`）与 **AQS**（`AbstractQueuedSynchronizer`）：

```text
ReentrantLock
    → Sync (FairSync / NonfairSync)
        → AQS：state、exclusiveOwnerThread、CLH 变体队列、park/unpark 协作
```

---

## 公平：FIFO 队列为什么仍可能「不公平」

这里曾出现一个很好的结构性质疑：CLH 等待队列是 FIFO，那「非公平」从何而来？是不是双向链表也可以做成 LIFO？

校准后的主线是：公平版与非公平版**共用同一种 FIFO 队列**；差别主要在**新线程第一次抢锁**时，是否允许在「已经有人在排队挂起」的情况下仍然直接 CAS 成功——非公平允许「绕开队列」；公平实现则会用「是否存在前驱等待者」这类检查来抑制插队。因此，「非公平」的主语更接近**新到者与已等待者之间的次序**，而不是「链表从哪一头进出」。

公平分支在思路上多了一道闸门（示意，非逐字源码）：

```java
// FairSync：先问「我前面还有没有人在等？」，再 CAS
if (!hasQueuedPredecessors() && compareAndSetState(0, 1)) {
    setExclusiveOwnerThread(current);
    return true;
}

// NonfairSync：常常允许先 CAS，再谈入队
if (compareAndSetState(0, 1)) {
    setExclusiveOwnerThread(current);
    return true;
}
```

`synchronized` 里 `_cxq` 的 LIFO 等细节，会牵动**已等待者之间**的次级次序；但对话里把两层拆开：**主要不公平**仍来自「新进入者与已排队者的直接竞争」这一机制层，与 `ReentrantLock` 非公平在「绕队列」意义上是同构的。

后来补上的一个反向直觉也很有用：释放锁的瞬间，队头线程从 `park` 回到可运行态有路径成本；若此时另一个已在 CPU 上跑着的线程刚好进门，它一次 CAS 的成本更低——于是默认非公平往往吞吐更好，代价是极端情况下可能出现饥饿。这不是道德判断，是调度与锁实现之间的工程权衡。

---

## Mesa 与 `Condition`：双队列如何落到 `await` / `signal`

在尝试「从零搭一个阻塞互斥」时，对话里主动引入了 Mesa 与「入口等待队列 / 资源等待队列」的划分，并把 `ReentrantLock` + `Condition` 接到这条思路上。

Mesa 与 Hoare 的关键分野在 `signal` 之后：**会不会立刻把锁和执行权交给被唤醒方**。Mesa 下，被唤醒者通常回到**入口（同步）队列**里重新竞争锁；因此从「被 signal」到「真正再次进入临界区」之间，可能被别的线程插入，条件可能再次变假——所以 `await` 必须放在 **`while` 重检谓词** 的模式里，而不能指望一次 `if`。

落到 API 行为：`await` 会释放互斥所对应的全部重入深度（`state` 回到 0）、当前节点离开同步队列、进入条件队列；`signal` 把节点迁回同步队列，再参与抢锁；语义上还要保证「恢复执行后重入计数与锁的归属」自洽。这样，纸面上的「双队列」才和代码里的等待结构对齐。

Mesa 语义在写法上的硬约束（示意）：

```java
lock.lock();
try {
    while (!predicateHolds()) {
        condition.await();
    }
    // 临界区：依赖谓词为真
} finally {
    lock.unlock();
}
```

---

## 一把尺子量不同「锁式物体」：从 JVM 到 Android 日常

当例子暂时落在不熟悉的栈（Redis 等）上时，对话把练习切回 Android 侧：用三个对象做「归类练习」。

- `AtomicInteger.compareAndSet`：只有原子变更、没有「谁在等、谁持有」的完整故事，更像**底层砖块**。
- `Room` / SQLite 事务：持有者是**事务**而不是线程，死锁往往走**全局检测与回滚**这一路，更像**事务侧的锁管理**。
- `kotlinx.coroutines.sync.Mutex`：大类上仍属「执行主体 + 互斥」这一支，但有两个精微校准点：其一，「持有者」更贴近**协程**（可用 `owner` 表达），而不是固定绑定到某个 OS 线程；其二，`suspend` 描述的是**挂起机制**，不等于 Mesa 里的**条件变量**——标准库 `Mutex` 本身不带 `Condition`，条件等待通常用 `Channel`、`StateFlow` 等组合出来。于是它更接近「**裁掉条件队列后的互斥子骨架**」：规律相同，部件可以省略。

对话里还曾用另一组「不熟悉栈」的例子练同一套尺子（你当时表示不熟，这里仅作对照存档，便于以后对照官方文档）：

| 例子 | 更像哪一类 | 一句话理由 |
|------|-------------|-----------|
| Go `sync.Mutex` + `sync.Cond` | 监视器族（Mesa 语义；文档要求 `for !cond { c.Wait() }`） | 有互斥 + 独立条件原语 |
| Linux `futex` | 更接近**底层砖块**（CAS + 内核等待/唤醒），不是完整监视器 | 不自带「持有者故事」与条件队列，常被 libc / 运行时拿来拼锁 |
| InnoDB 行锁 | **事务锁管理** | 持有者是事务；谓词等待不是 Mesa `Condition` 那条故事线；死锁走检测与回滚 |

把不同「锁式物体」放在一张鸟瞰里（与上文三问一致，只是换视角）：

```text
                并发里叫「锁」的东西
                          |
     +--------------------+--------------------+
     |                    |                    |
  底层砖块            监视器族              事务侧锁管理
 CAS / futex         互斥 +（可选）条件      2PL / 行锁 / 等
  park 片段          ReentrantLock 等        全局等待图

持有者：常无        持有者：线程/协程        持有者：事务
条件队列：无        条件队列：可有可裁剪     条件队列：不适用同一模型
```

---

## 理发店：跨域映射、一次 API 纠偏、以及从「一把椅子」到「三把椅子」

用理发店做映射时，等候区对应「竞争失败后的入口队列」；「下一位」更接近**唤醒**而不是 CAS；共享状态更接近「空位 / 占用数」；「坐下抢座」才是需要原子化的竞争瞬间。曾把「等候区满了就离开」误贴到 `getQueueLength`——这里应校准为 **`tryLock` 或带超时的 `tryLock`**：核心语义是**不排队或只短暂排队就放弃**；`getQueueLength` 只是观测队列长度，不改变获取语义。

题目里若出现「三个座位」而不是「一把椅子」，互斥不再是 0/1 二元，而是 **N 个许可**——这与 `Semaphore` 同构。进一步收束：`java.util.concurrent` 里许多工具都可以读成**同一套 AQS 骨架**，只是 **`state` 的解释不同**（互斥加重入深度、剩余许可数、`CountDownLatch` 的剩余次数、读写锁的位域打包等）。这是整段对话里把「具体类名」往上抽的最后一跳。

| 工具 | `state` 在读者的直觉里更像什么 |
|------|--------------------------------|
| `ReentrantLock` | 0 表示空闲；大于 0 表示被占用且值等于重入深度 |
| `Semaphore` | 剩余许可数（初始化为 N） |
| `CountDownLatch` | 还剩几次 countDown 才会开门 |
| `ReentrantReadWriteLock` | 高位/低位拆分表达读锁计数与写锁占用（实现细节以源码为准） |

---

## 对话中出现的误解

> 以下是本次对话推导过程中出现的明确偏差，记录在此供后续参考。

| 误解或误判 | 校准后的表述 |
|-----------|-------------|
| 认为 `ReentrantLock` 默认为是公平锁、`synchronized` 非公平 | `ReentrantLock` **默认非公平**；`new ReentrantLock(true)` 才是公平；`synchronized` 非公平 |
| 将「等候区满则离开」对应到 `getQueueLength` | 对应 **`tryLock` / 带超时的 `tryLock`**；`getQueueLength` 仅查询等待队列长度，不改变加锁语义 |
| 以为「非公平」主要来自 CLH 可否 LIFO，或与 `_cxq` 的 LIFO 简单等同 | 对 `ReentrantLock`：主要非公平来自**新线程绕队列直抢**；`_cxq` 的 LIFO 等更像**已等待者之间的次级次序**因素 |

---

## 遗留问题

按对话中曾显式维护的清单与收束时的缺口，合并为下列优先顺序（每条一句话指方向）：

1. **`unlock()` 调用链**：`state` 递减、何时唤醒后继、`unpark` 与 `park` 如何配对，尚未按源码路径逐步展开。
2. **常见误用面**：`unlock` 与 `finally` 的配对、`Condition.await` 与谓词检查边界等，尚未系统罗列成检查清单。
3. **手撸新同步工具或替代范式**：基于 AQS 自定义一种 `state` 语义，或对比 Actor / STM 等「弱化互斥假设」的路线，对话未继续。

---

## 附录

### 对照表：Mesa 与 Hoare（条件语义）

| 侧面 | Hoare | Mesa |
|------|-------|------|
| `signal` 后 | 常立即移交锁与 CPU 给被唤醒者 | 被唤醒者回入口队列重抢锁 |
| 醒来时条件 | 可假定仍为真 | 可能已变假，必须 `while` 重检 |
| 工程采用 | 少见 | Java `wait/notify`、`ReentrantLock`+`Condition`、多数主流运行时 |

### 框架骨架：用户态阻塞互斥的最小部件（与语言无关）

| 部件 | 职责 |
|------|------|
| 共享状态 | 是否占用、占用深度或许可数 |
| 持有者标识 | 当前执行主体（线程 / 协程 / 事务 ID 等） |
| 入口等待队列 | 竞争失败者的排队与唤醒次序 |
| 条件等待队列（可选） | 谓词不满足时的等待与 `signal` 迁移 |
| 原子状态变更 | CAS 或等价硬件原子序列 |
| 挂起 / 唤醒 | `park`/`unpark`、协程调度、或内核 `futex` 等 |

### 决策准则：识别不同「锁式物体」的三问

1. 持有者是谁？（无独立持有者故事 → 砖块；执行主体 → 监视器族；事务 → 锁管理器）
2. 是否存在独立的条件等待语义？（无 → 裁剪版互斥；有 → 完整 Mesa）
3. 死锁如何处理？（不处理 / 由调用方避免 / 全局检测与回滚）

### 源码与 API 入口标识（便于查证）

- `java.util.concurrent.locks.ReentrantLock`
- `java.util.concurrent.locks.AbstractQueuedSynchronizer`（`state`、`exclusiveOwnerThread`、CLH 变体队列）
- `java.util.concurrent.locks.ReentrantLock.FairSync` / `NonfairSync` 与 `hasQueuedPredecessors()` 公平路径差异
- `java.util.concurrent.locks.AbstractQueuedSynchronizer.ConditionObject`

---

## 对话质量诊断

### 对话质量

| 维度 | 评级 | 说明 |
|------|------|------|
| 推导过程完整度 | 高 | 多次主动对比（`synchronized` / `_cxq`）、引入 Mesa、完成 Android 三案例与理发店迁移 |
| 结论直给比例 | 低 | 关键结论多经提问、纠偏或类比后落地 |
| 关键转折覆盖度 | 高 | 公平误解、CLH 与绕队列、Kotlin `Mutex` 精微点、`tryLock` 纠偏、信号量泛化均有记录 |
| 用户主体性 | 中 | 有自发质疑与跨模型联想；整体仍在引导式路径内由对话框架推进 |

**综合评级**：高质量

### 路径来源

路径来源：混合（既有自发质疑与模型引入，也有框架化追问与补全）。

复用建议：可自主重走「骨架 → CAS → 公平分界 → Mesa → 三问分类 → 理发店」链条；涉及具体 JVM 源码行级或 `unlock` 逐步展开时，建议配合源码或官方文档再走一遍。

### 模型适用性

适用性：完全适用（疑问 → 推导 → 收束主线清晰）。
