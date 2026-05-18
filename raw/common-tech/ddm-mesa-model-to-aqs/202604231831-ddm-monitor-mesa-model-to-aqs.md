# Java Monitor 机制与 Mesa 模型：从 synchronized 到 AQS

> 本文档基于一次 LCCM 引导对话整理。
> 目标是重建理解的过程——跟着推导走一遍，而不是直接读结论。


> **导航**：[digest](../../../digest/common-tech/ddm-mesa-model-to-aqs/202604231831-ddm-monitor-mesa-model-to-aqs.md)
---

## wait() 调用时到底发生了什么

进入 `synchronized` 临界区的线程是持有锁的——这一点没有歧义。但当线程在临界区内调用 `obj.wait()` 时，一个直觉上容易忽略的问题出现了：它在等什么？谁来唤醒它？其他线程要进入同一个 `synchronized(obj)` 临界区，必须拿到 `obj` 的锁，但锁现在在 A 手里——B 怎么进来执行 `notify()`？

这里的关键是：`wait()` 不是普通的睡眠，它做了两件事，而且是**原子地**同时完成：

```
obj.wait() = 释放 obj 的锁 + 把当前线程放入 WaitSet 睡眠
```

正是这个"释放锁"的动作，让其他线程得以进入临界区。B 之所以能进来执行 `notify()`，前提是 A 已经通过 `wait()` 主动让出了锁。

---

## 多个线程等待时的唤醒机制

顺着这个理解，进一步的问题是：如果有多个线程都在等，`notify()` 只唤醒一个，那另一个去哪了？被唤醒的线程拿到锁了吗？

这里需要区分两个队列：

- **WaitSet**：调用 `wait()` 后进入，在这里等 `notify()` 把自己移出来
- **EntryQueue**：准备竞争锁的线程在这里等锁释放

`notify()` 只做一件事：把 WaitSet 里的某个线程移入 EntryQueue。调用 `notify()` 的线程**不会退出临界区、不会释放锁**，它继续持锁执行，直到自己退出 `synchronized` 块。

如果有线程 A 和 C 都在 WaitSet 等待：
- `notify()` 只移动一个（JVM 选择，不保证顺序），另一个继续在 WaitSet 睡着
- `notifyAll()` 把所有线程都移入 EntryQueue，然后它们竞争锁，一个拿到执行，其余在 EntryQueue 等

在 EntryQueue 里等待的线程**不需要再次 notify**——锁一释放，它们自动参与竞争。

---

## wait() 原子性：释放锁和入队为什么不可分割

`wait()` 必须原子地完成"释放锁 + 入 WaitSet"。如果这两步可以被打断，会发生什么？

假设拆成两步：

```
A：第①步 —— 决定释放锁
                        ← CPU 切换给 B
                           B：进入 synchronized，执行 notify()
                              WaitSet 为空，notify 什么都没做
                        ← CPU 切回 A
A：第②步 —— 进入 WaitSet 睡眠
```

B 的 `notify()` 在 A 进入 WaitSet **之前**就发出去了，A 永远睡着，没有人再唤醒它。这就是经典的 **Lost Wakeup（通知丢失）** 问题。

所以 `wait()` 的原子性不是实现细节，而是正确性的前提：不给任何线程在"释放锁"和"入 WaitSet"之间插入的窗口。

这个原子性由 **JVM + OS** 两层共同保证：JVM 的 `ObjectMonitor::wait()` 在内部锁保护下先加入 WaitSet 再释放 monitor；底层依赖操作系统的 mutex + condition variable 原语，释放与挂起不可分割。Java 语言层的 `synchronized` 关键字本身不提供这个保证，原子性实现在 `ObjectMonitor` 里。

---

## ObjectMonitor 的真实结构

知道了行为，自然想问 JVM 里实际用什么数据结构支撑。WaitSet 和 EntryQueue 是真实存在的字段，但实际结构比 Mesa 模型的概念图多了一个：

| 字段 | 对应概念 | 说明 |
|---|---|---|
| `_WaitSet` | Wait Set | `wait()` 后进入，等 notify 移出 |
| `_EntryList` | Entry Queue | 准备抢锁的线程，FIFO 出队 |
| `_cxq` | Entry Queue 的入口 | 新到来的竞争者先进 `_cxq`（LIFO），再合并进 `_EntryList` |

`_cxq` 是概念模型里没有画出来的部分。新线程到来时，用 CAS 写入 `_cxq`，不需要争抢 `_EntryList` 的操作权，减少入队时的锁竞争。`_cxq` 自己**不直接参与抢锁**——只有被合并进 `_EntryList` 之后，才有资格出队拿锁。合并时机由锁持有者退出时触发。

你主动识别出了这个细节：`_cxq` 不和 `_EntryList` 竞争拿锁，它只是定期进入 `_EntryList`，`_EntryList` FIFO 出队才是真正的持锁入口。

`notify()` 唤醒的线程从 `_WaitSet` 直接移入 `_EntryList`，不经过 `_cxq`——被唤醒线程和新来竞争者走的是不同入口：

```
新竞争者   → _cxq → (合并触发) → _EntryList → 持锁
notify唤醒 ─────────────────→ _EntryList → 持锁
```

关于猜测中的 `_NotifySet`：它不存在。`notify()` 是即时操作，直接移队列，不需要中间暂存。

---

## synchronized 的不公平性从何而来

`_cxq` 是 LIFO，意味着后来的线程反而先合并进 `_EntryList`，等待已久的线程可能排在后面——这是不公平的一个来源。但不公平是结构性的，来自三处叠加：

**① `_cxq` 是 LIFO**：后入先合并，破坏等待顺序。

**② 自旋插队**：新线程进入时，JVM 先让它自旋尝试 CAS 抢锁，不直接进 `_cxq`。如果自旋成功，直接持锁，完全绕过已在 `_EntryList` 排队的线程。

**③ `_cxq` 合并时机不确定**：合并由锁退出者触发，不是定时触发，`_cxq` 里的线程等多久取决于锁的释放节奏，没有上限保证。

`synchronized` 不保证公平性——线程获得锁的顺序不等于它们等待的顺序。这正是 `ReentrantLock(true)`（公平锁）存在的原因，它用严格 FIFO 队列替换了这套机制，代价是性能下降。

---

## Mesa 模型在哪个层次上

到这里可以退一步问：`synchronized + wait/notify` 是 Mesa 模型，`ReentrantLock` 也是 Mesa 模型吗？Mesa 到底对应什么维度？

Mesa 模型是**并发控制的设计抽象**，不绑定任何具体实现。它定义的是一种语义：条件变量 + 互斥锁的协作方式——被唤醒的线程**不直接持锁，需重新竞争**。凡满足这个结构的，都是 Mesa 模型的实现。

Java 里两种实现都是 Mesa：`synchronized + wait/notify` 用的是对象内置锁和隐式的单条件变量；`ReentrantLock + Condition` 用的是显式锁和可以有多个的条件变量。`Condition.await()` 对应 `wait()`，`Condition.signal()` 对应 `notify()`，唤醒后同样要回队列重新抢锁，同样必须用 `while`。

Java 之外，`pthread_mutex + pthread_cond_wait`、Python 的 `threading.Condition`、Go 的 `sync.Cond`、C# 的 `Monitor.Wait/Pulse` 也都是 Mesa 模型的实现。

与之对比的 Hoare 模型（signal 后立即交出锁，被通知者直接执行）在理论上承诺更强，但实现复杂、性能差，几乎没有主流实现。Mesa 是并发控制的事实标准。

---

## wait/notify 为什么属于 Object，而不是 Thread

确认了 Mesa 模型不绑定具体实现之后，一个关于 API 设计的疑问自然出现：`wait()` 和 `notify()` 为什么是 `Object` 的方法，而不是 `Thread` 的方法？

反向推一下：如果放在 `Thread` 上，会是什么语义？

```java
Thread.currentThread().wait();  // 当前线程在等什么？等谁通知？
someThread.notify();            // 通知某个线程？通知它什么条件？
```

问题立刻暴露：等待和通知缺少共同的**条件锚点**。两个线程必须约定在同一个对象上等待和通知，才能形成有意义的信号传递——"你在 `obj` 上等，我在 `obj` 上通知你"，`obj` 是双方的共同协调点。

Mesa 模型里，Monitor = 锁 + 条件变量，两者绑定在同一个对象上：

```
obj.wait()    ── 在 obj 这个 Monitor 的条件变量上等待
obj.notify()  ── 通知在 obj 这个 Monitor 上等待的线程
```

`wait/notify` 是 Monitor 的操作，Monitor 绑在对象上，所以方法自然属于 `Object`。这是 Java"每个对象天生是一个 Monitor"这个设计决策的直接推论——不是 API 设计的随意选择，而是模型本身决定的。

---

## while 而非 if：一个被迫的选择

Mesa 模型有一个对调用方的强制要求：等待条件必须用 `while`，不能用 `if`。这不是编码风格，背后有结构性原因。

假设 A 和 C 都在 WaitSet 等待"队列不为空"，B 生产了一个 item 并调用 `notify()`，A 被移入 EntryQueue。此时 C 也因某次 `notifyAll` 被唤醒并**先抢到锁**，消费了那个 item，队列又空了，C 释放锁。A 拿到锁，从 `wait()` 返回——但此时队列已经空了，条件不再成立。

用 `if` 的话，A 会直接执行消费操作，在空队列上崩溃。用 `while` 的话，A 醒来重新进入循环检查，发现条件不成立，继续等。

根本原因是：**Mesa 模型不保证"被唤醒时条件成立"**。从 `notify()` 到被通知线程真正持锁，之间存在时间窗口，任何线程都可以在这个窗口内改变条件。`while` 是对这个窗口的防御。

你在这里主动识别出这个规律与 DCL（双重检查锁）的相似之处——两者都是"判断到执行之间有时间窗口，窗口内条件可能被别人改掉"。区别在于：DCL 的窗口在锁外到锁内之间，针对新线程；Mesa `while` 的窗口在 notify 到持锁之间，针对被唤醒线程。两种场景，同一条并发规律：

> 在多线程环境下，任何"判断"和"执行"之间的时间窗口，都可能让条件失效。

---

## 多条件变量解决了什么

`synchronized` 每个对象只有一个隐式条件变量，所有等待线程都在同一个 WaitSet 里。这在生产者/消费者场景里暴露出问题：

```java
// 生产者
synchronized (lock) {
    while (queue.isFull()) lock.wait();
    queue.add(item);
    lock.notifyAll();  // 既唤醒消费者，也唤醒其他生产者
}
```

`notifyAll()` 把所有等待线程全部唤醒——包括同样在等"不满"的其他生产者。它们醒来发现队列还是满的，白白竞争一轮锁，再睡回去。线程越多，无效唤醒越多。

`ReentrantLock` 支持多个 Condition，可以精准唤醒：

```java
ReentrantLock lock = new ReentrantLock();
Condition notFull  = lock.newCondition();  // 生产者等这个
Condition notEmpty = lock.newCondition();  // 消费者等这个

// 生产者
lock.lock();
while (queue.isFull()) notFull.await();
queue.add(item);
notEmpty.signal();  // 只唤醒消费者
lock.unlock();

// 消费者
lock.lock();
while (queue.isEmpty()) notEmpty.await();
queue.poll();
notFull.signal();   // 只唤醒生产者
lock.unlock();
```

这正是 `java.util.concurrent.ArrayBlockingQueue` 的实际实现方式。

在这里，你主动写出了用嵌套 `synchronized` 模拟多条件的方案，并追问它是否有 BUG：

```java
synchronized (LOCK) {
    synchronized (notFull) {
        while (queue.isFull()) notFull.wait();
        ...
    }
}
```

你随后独立识别出问题所在：`notFull.wait()` 只释放 `notFull` 的锁，**外层 LOCK 依然被持有**。消费者尝试进入 `synchronized(LOCK)` 时永远被阻塞，而生产者在 `notFull.wait()` 里等消费者消费——两个线程互等，必然死锁。这不是性能问题，是结构性的必然死锁。

`wait()` 的释放规则是：**只释放调用它的那个对象的锁，持有的其他锁全部保留**。这是 Java 语言规范的明确规定。多锁嵌套下，这个规则使 `wait()` 几乎无法安全使用。

`Condition` 正是把"多个条件队列绑在同一把锁上"这个需求封装成了一等公民——`Condition.await()` 释放的是它绑定的那把 `ReentrantLock`，结构清晰，没有多锁嵌套的歧义。

---

## AQS：把 Mesa 模型泛化成通用框架

`ReentrantLock`、`CountDownLatch`、`Semaphore` 行为差异很大，但它们共享同一套底层结构——`AbstractQueuedSynchronizer`（AQS）。

AQS 把 Mesa 模型拆成两层：

```
AQS 固化的部分（机制）：
  · CLH 双向队列——线程排队、挂起、唤醒
  · LockSupport.park/unpark——线程挂起原语

AQS 留给子类的部分（策略）：
  · state（一个 int）——你定义它的语义
  · tryAcquire()——你定义"何时获取成功"
  · tryRelease()——你定义"何时完全释放"
```

同一套结构，state 语义不同，就变成了不同的工具：

| 实现 | state 语义 | 获取成功条件 |
|---|---|---|
| `ReentrantLock` | 0=未锁，>0=重入次数 | CAS 把 0 改成 1 |
| `Semaphore` | 剩余许可数 | state > 0，减一 |
| `CountDownLatch` | 倒计数 | state == 0 |
| `ReadWriteLock` | 高16位=读锁数，低16位=写锁数 | 按读写规则判断 |

AQS 对应 Mesa 模型的结构映射：

```
Mesa 模型          AQS 实现
─────────────────────────────────
互斥锁          →  state + CAS
EntryQueue      →  CLH 双向队列（Node 链表）
WaitSet         →  ConditionObject 内部队列
wait()          →  ConditionObject.await()
notify()        →  ConditionObject.signal()
重新竞争锁       →  从 Condition 队列移回 CLH 队列
```

Mesa 的 `while` 仍然必须——`await()` 返回后条件可能失效，AQS 没有改变这个根本约束。

你提出了"AQS 是 lib 层，没有 JVM 层支持吗"的疑问。修正如下：AQS 不是 ObjectMonitor 的上层包装，而是**平行的另一套实现**。它用纯 Java 代码管理队列，但线程挂起和唤醒这件事没有纯 Java 的办法做到——AQS 通过 `LockSupport.park/unpark` 借助 JVM 原语完成这一步，底层同样下穿到 OS。

AQS 存在的根本原因是 `synchronized` 有三个语义层的硬伤：

| 问题 | synchronized | AQS/ReentrantLock |
|---|---|---|
| 不可中断 | 等锁期间无法响应中断 | `lockInterruptibly()` |
| 无超时 | 要么拿到锁要么永远等 | `tryLock(timeout)` |
| 单条件变量 | 每个对象只有一个 WaitSet | 多个 `Condition` |

---

## CLH 队列 vs ObjectMonitor EntryList

两者都是"等待线程的队列"，但设计哲学不同。

ObjectMonitor 的 `_EntryList` 里，线程自旋等待**锁本身**——当锁释放时，多个线程同时尝试 CAS 抢同一个变量，大量失败，缓存行失效风暴随线程数增长。

AQS 的 CLH 队列让每个线程只盯着**前一个节点的 waitStatus**：

```
head → [Node A] → [Node B] → [Node C]

B 只看 A 的 waitStatus
C 只看 B 的 waitStatus
互不干扰
```

锁释放时，只有队头线程被唤醒，其他线程继续各自盯着自己的前驱。缓存行竞争从 O(N) 降到 O(1)。

| | ObjectMonitor EntryList | AQS CLH 队列 |
|---|---|---|
| 队列结构 | 单向链表 | 双向链表（FIFO） |
| 自旋目标 | 等锁本身 | 等前驱节点状态 |
| 挂起方式 | OS 原语（pthread） | `LockSupport.park()` |
| 公平性 | 不保证 | 可选（公平锁严格 FIFO） |
| 可见性 | JVM 内部，Java 不可见 | Java 层，完全可控 |

ObjectMonitor 是 JVM 为 `synchronized` 定制的实现，简单但粗放；AQS CLH 是 Java 层精心设计的通用框架，通过"自旋等前驱"显著减少高竞争场景下的缓存竞争。

---

## 对话中出现的事实性偏差（回答者视角）

> 以下是本次对话推导过程中出现的明确事实性偏差，记录在此供后续参考。

| 偏差描述 | 准确表述 |
|---|---|
| 猜测 ObjectMonitor 存在 `_NotifySet` 字段 | 该字段不存在；`notify()` 是即时操作，直接将线程从 `_WaitSet` 移入 `_EntryList`，不需要中间暂存队列 |
| 认为嵌套 synchronized 方案"不考虑性能可以使用" | 该方案在队列满时必然死锁（生产者持有外层 LOCK 不释放，消费者永远无法进入临界区），不是性能问题，是正确性问题 |

---

## 遗留问题

1. AQS CLH 队列的"自旋等前驱"在高竞争场景下的具体性能表现，与 ObjectMonitor 自旋的对比数据
2. JVM 锁升级路径（偏向锁 → 轻量级锁 → 重量级锁）与 ObjectMonitor 的关系——本次对话未覆盖
3. `LockSupport.park/unpark` 与 `pthread_cond_wait/signal` 的映射关系及平台差异
4. AQS 公平锁与非公平锁在 CLH 队列层面的实现差异

---

## 附录

### Mesa 模型主流实现对照

| 语言/平台 | 互斥锁 | 条件变量 |
|---|---|---|
| Java（内置） | `synchronized` | `Object.wait/notify` |
| Java（显式） | `ReentrantLock` | `Condition.await/signal` |
| C/C++ | `pthread_mutex` | `pthread_cond_wait/signal` |
| Python | `threading.Lock` | `threading.Condition` |
| Go | `sync.Mutex` | `sync.Cond` |
| C# | `Monitor.Enter` | `Monitor.Wait/Pulse` |

来源：Mesa 模型在哪个层次上

### ObjectMonitor 三队列结构

| 字段 | 类型 | 谁在里面 | 进入条件 | 退出条件 |
|---|---|---|---|---|
| `_WaitSet` | 双向链表 | 调用 `wait()` 的线程 | 调用 `obj.wait()` | `notify()` / `notifyAll()` |
| `_cxq` | 单向链表（LIFO） | 新到来的竞争者 | 进入 `synchronized` 竞争失败 | 锁退出者触发合并 |
| `_EntryList` | 单向链表（FIFO） | 等待持锁的线程 | 从 `_cxq` 合并 / 从 `_WaitSet` notify | 出队成功持锁 |

来源：ObjectMonitor 的真实结构

### AQS state 语义对照

| 实现类 | state 含义 | acquire 成功条件 | release 操作 |
|---|---|---|---|
| `ReentrantLock`（非公平） | 0=未锁，N=重入次数 | CAS 0→1 或已是持有者 | state 减至 0 |
| `Semaphore` | 剩余许可数 | state > 0，CAS 减一 | state 加一 |
| `CountDownLatch` | 倒计数 | state == 0（共享模式） | countDown() 减一至 0 |
| `ReentrantReadWriteLock` | 高16位=读锁数，低16位=写锁数 | 按读写互斥规则判断 | 分别操作对应位 |

来源：AQS

---

## 对话质量诊断

### 对话质量

| 维度 | 评级 | 说明 |
|---|---|---|
| 推导过程完整度 | 高 | 多处主动猜测（_NotifySet、JVM 保障、嵌套方案）、质疑（_cxq 是否竞争锁、AQS 是否无 JVM 支持）、类比（DCL） |
| 结论直给比例 | 低 | 80% 以上有推导路径，结论多从疑问中引出 |
| 关键转折覆盖度 | 高 | wait() 原子性、多锁嵌套死锁、Mesa while 必要性、CLH vs EntryList 均有完整推导记录、wait/notify 属于 Object 的 API 设计逻辑 |
| 用户主体性 | 中 | [U/U] 5个，[U/AI] 高自主 3个，有真实认知输出，但主线方向多由引导者设计 |

**综合评级**：高质量

### 路径来源

路径来源：混合（主线方向由 AI 引导，关键转折点和类比识别由用户主导）

复用建议：
- "wait() 原子性"、"while 必要性"、"多锁嵌套死锁" 三章可自主复用，用户认知事件完整
- "AQS 结构"、"CLH vs EntryList" 两章建议配合引导者重走，主线由 AI 设计
- DCL 类比章节高度自足，用户独立完成了跨域规律抽象

### 模型适用性

完全适用——对话有清晰的疑问→推导→落点主线，从 `wait()` 行为一路推进到 AQS 框架设计，推导链完整闭合。