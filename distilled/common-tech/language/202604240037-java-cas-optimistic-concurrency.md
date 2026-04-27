# 从一次错误的 CAS 实现出发，直到乐观并发

> 本文档基于一次引导式学习（RAPM + LCCM）整理。
> 目标是重建理解过程——跟着推导走一遍，而不是直接读结论。


> **导航**：[digest](../../../digest/common-tech/language/202604240037-java-cas-optimistic-concurrency.md)

## 本次对话在追什么

对话从一份缺少重试循环的 CasCounter 实现出发，沿一条可重走的主线推进：

- 从"CAS 调一次够吗"→ 识别自旋语义，理解失败时发生了什么
- 从"自旋能用多久"→ 触及乐观并发的高冲突失效边界
- 从"多个变量怎么更新"→ 推导 AtomicReference<不可变对象> 的必要性，并独立给出不可变的原因
- 从"值转了一圈回来"→ 追问 ABA 问题，拆解 AtomicStampedReference 的内部 Pair 机制
- 从"CAS 背后有没有规律"→ 迁移到电商库存、Android 主线程模型、Compose Snapshot，提炼乐观并发的跨域结构

文末保留 synchronized JMM 可见性/有序性机制、ABA 在 C/C++ 内存复用场景下的细节，便于单独补课。

---

## 一次 compareAndSet 不够吗？

最初提交的 CasCounter 实现是这样的：

顺着 RAPM 任务的 CasCounter 框架，提交的自增方法：

```java
public void increment() {
    int oldValue = value.get();
    value.compareAndSet(oldValue, oldValue + 1);
}
```

这段代码的问题不是语法，而是语义——`compareAndSet` 只尝试一次，失败时什么都不做，自增悄悄丢失了。高并发场景下，多个线程同时读到相同的 oldValue，只有一个能 CAS 成功，其余全部静默失败，最终计数必然小于预期。

CAS 本身不保证成功，它只保证：**比较和交换这个操作不被打断**。要用 CAS 实现"最终必须完成一次自增"，需要在失败时反复重试：

```java
public void increment() {
    while (true) {
        int oldValue = value.get();
        boolean success = value.compareAndSet(oldValue, oldValue + 1);
        if (success) break;
        Thread.yield();  // 建议性让出 CPU，JVM/OS 可忽略
    }
}
```

在这个方向上，你主动识别出：这个 `while` 就是**自旋**——线程不睡眠，不让出 CPU，一直在原地转圈重试，直到条件成立。这和"阻塞"的差别正是下一个问题的入口。

---

## 自旋打满 CPU 的条件是什么？

自旋的代价是：持续消耗 CPU，但不推进有用的工作。`Thread.yield()` 是一个建议性的让步，JVM 和 OS 可以忽略它；更彻底的让出是 `LockSupport.parkNanos()`，这是阻塞的方向了。

顺着"自旋 vs 阻塞"的追问，你给出的判断是：自旋适合**等待时间短**的场景——CAS 失败后很快能成功，这时候自旋划算；高并发持续争用同一个变量时，大量线程同时自旋，CPU 全在空转，有效工作趋近于零。

对比 `synchronized`：拿不到锁的线程被操作系统**挂起**，不占 CPU，等通知再唤醒——代价是线程切换开销，但 CPU 得以释放。

"高并发下没有更合适的方式了吧"——这里需要一个校准：Java 8 引入的 `LongAdder` 专为高冲突计数设计，把一个计数器拆成多个分段，各线程打不同的桶，读取时合并，从根本上降低争用——化解冲突，而不是换更能抗冲突的机制硬扛。电商大促时的真实工程方案（库存分桶、Redis 原子扣减、请求队列）都在做同一件事：让高冲突问题变成低冲突问题，而不是直接切悲观锁。

---

## 多个变量怎么同时更新？

AtomicInteger 的 CAS 只对一个变量有效。你提出：把 x 和 y 封装进一个对象，比较对象的 hashcode——**封装成对象**这个方向是对的，但 hashcode 不行：不同的 (x, y) 组合可能产生相同的 hashcode（哈希碰撞），而 `AtomicReference.compareAndSet` 用的是 `==`（引用地址比较），不是 `equals()`。

正确的做法是创建**不可变对象**，然后 CAS 替换整个引用：

顺着"封装成对象，CAS 操作引用"的方向：

```java
record Point(int x, int y) {}  // Java 16+ 正式语法，类似 Kotlin data class

AtomicReference<Point> ref = new AtomicReference<>(new Point(1, 2));

Point oldVal = ref.get();
Point newVal = new Point(oldVal.x() + 1, oldVal.y() + 1);
ref.compareAndSet(oldVal, newVal);  // 比较引用地址
```

为什么 Point 必须是不可变的——你独立推导出了这一点：CAS 用引用地址作为"值未被修改"的代理判断。如果允许原地修改字段，引用没变但内容已改，CAS 会误判为"没人动过"，用旧逻辑覆盖别人的修改。不可变对象保证了"同一个引用 = 同样的内容"，CAS 的代理判断才成立。

---

## 值转了一圈回来，CAS 感知不到吗？

ABA 问题的核心是：CAS 只问"现在的值等于我记录的旧值吗"，感知不到"值是否在中间经历过变化又回来了"。

这在大多数场景下不构成问题——计数器、状态标志、简单替换，A→B→A 没什么危害。**真正有害**的条件是：同一个引用对应的对象是可变的且被复用——一个可变对象被取出、内容修改、又修改回原值（同一个实例），持有旧引用的线程做 CAS 会成功，但对象的"语义身份"已经不是当时读到的那个了，操作在过时快照上继续执行，无异常，静默出错。在 Java 的 GC 环境下，这需要对象池/复用才能触发，不是普遍风险。

Java 提供 `AtomicStampedReference` 作为防御工具，在引用旁边附加一个**版本号**。你问：它如何原子地同时比较引用和 stamp——两个值，难道要加锁或者特殊的双 CAS 硬件指令？

答案是：用的正是上一节的模式——把 reference 和 stamp 打包进一个不可变 Pair，CAS 操作那个 Pair 的引用地址：

JDK 源码印证了这个结构：

```java
// AtomicStampedReference 内部
private static class Pair<T> {
    final T reference;
    final int stamp;
}

private volatile Pair<V> pair;

public boolean compareAndSet(V expectedRef, V newRef,
                              int expectedStamp, int newStamp) {
    Pair<V> current = pair;                    // 读 volatile，拿本地快照
    return expectedRef == current.reference    // 检查快照（快速失败优化）
        && expectedStamp == current.stamp      // 检查快照（快速失败优化）
        && casPair(current, Pair.of(newRef, newStamp));  // 真正的原子操作
}
```

`current` 是本地变量——即使另一个线程在两次检查之间换掉了 `pair`，`casPair` 时发现 `pair != current`，CAS 失败，方法返回 false。前两个 `==` 是快速失败的优化，原子性保证只来自最后那一个 CAS。

版本号的递增是调用方的责任——`compareAndSet` 接受 `newStamp` 参数，通常传 `stamp + 1`。JDK 选择了灵活性而非强制：你可以用时间戳、跳跃式编号，或任何规则。代价是保护能否生效完全靠调用方的自觉——这也是它在实际代码里用得不多的原因。

---

## CAS 背后是一种哲学

你把乐观并发总结为"先做，有问题再说"——方向对，精确版多了一层：**验证是设计的一部分，不是事后补救**。CAS 的重试、AtomicStampedReference 的版本号比对、数据库的 `WHERE version = expected`，都是"结构化的冲突处理机制"。

> 乐观并发：假设冲突是小概率事件，先做，用验证代替预防，代价只在冲突发生时才支付。

你主动识别出这个模式在电商库存里的实例：

顺着"乐观读取 → 验证旧值 → 原子替换"的结构，DB 乐观锁和 CAS 并排：

```
CAS（内存层）                     DB 乐观锁（库存扣减）
  读 oldValue                       读 stock=100, version=5
  计算 newValue                     计算 newStock=99
  compareAndSet(old, new)           UPDATE ... WHERE id=1 AND version=5
  失败 → 重试                        affected rows=0 → 重试或报错
```

结构完全相同：差别只在"原子性"的载体——内存层靠 CPU 指令，数据库层靠事务 + WHERE 条件。

你也主动识别出：**Android 主线程模型是悲观的极限**——不是"先加锁再进入"，而是直接规定只有一个线程能碰 UI，从根本上消灭并发。这在 UI 场景成立：帧率有上限（60fps = 16ms/帧），不是高吞吐场景，一致性比吞吐量更重要。

顺着这一端，Jetpack Compose 的 Snapshot 系统走向了另一端。你直接识别出它和 CAS 的结构同构：

```
Compose Snapshot
  进入快照：取当前所有 State 的隔离视图
  在快照内修改：外界不可见
  apply()：检查你读过的 State 是否被别人改过
            有冲突 → 冲突解决器或丢弃
            无冲突 → 原子应用所有变更
```

高层是 MVCC（多版本并发控制），底层 StateRecord 链表的原子更新仍然靠 CAS。

**规律**：乐观并发不是一个算法，是一种设计哲学——从单变量（CAS）到多变量（Snapshot/MVCC）到分布式（数据库乐观锁），同一个结构在不同粒度上反复出现。

---

## 对话中出现的事实性偏差（回答者视角）

> 以下记录本次推导中出现的明确事实性偏差。
> 每条保留「触发语境」，避免把路径事件扁平化为知识勘误。

| 触发语境 | 偏差描述 | 校准后的表述 |
|---------|---------|-------------|
| 讨论 CAS 与 synchronized 的区别时说"synchronized 解决 JMM 的原子性问题" | 认为 synchronized 只覆盖原子性 | synchronized 覆盖 JMM 三个保证：原子性、可见性、有序性 |
| 讨论高并发计数时说"不用 CAS 也没有更合适的方式了吧" | 认为 CAS 是高并发计数的唯一选择 | Java 8 引入的 `LongAdder` 通过分段降低争用，是高并发下更优的选择 |

---

## 对话中出现的引导质量问题（回答者视角）

> 以下记录本次对话中出现的引导策略失效或方向偏移，供路径复用时参考。

| 问题描述 | 影响范围 | 对话中的处理结果 |
|---------|---------|----------------|
| ABA 问题的栈例子设计有误：把"B 丢失"当成危害，但 B 是线程 Y 主动 pop 掉的，X 的 pop 操作本不关心 B | 用了两轮才给出清晰解释，用户质疑有效打断了偏离方向 | AI 承认例子有问题，重新用可变对象复用场景给出准确危害说明 |

---

## 遗留问题

1. `synchronized` 对 JMM **可见性**和**有序性**的保证机制——本次只覆盖了原子性，可见性（其他线程立刻看到修改）和有序性（禁止指令重排）的实现细节未展开
2. **ABA 在 C/C++ 内存复用场景**下的具体危害——本次场景局限于 Java/GC 环境，相同内存地址被复用给新对象才是真实痛点，与 Java 场景机制不同

---

## 附录

### CasCounter 最终实现

经本次学习校准后的正确版本：

```java
import java.util.concurrent.atomic.AtomicInteger;

public class CasCounter {
    private AtomicInteger value = new AtomicInteger(0);

    public void increment() {
        while (true) {
            int oldValue = value.get();
            boolean success = value.compareAndSet(oldValue, oldValue + 1);
            if (success) break;
            Thread.yield();
        }
    }

    public int get() {
        return value.get();
    }

    public static void main(String[] args) throws InterruptedException {
        CasCounter counter = new CasCounter();
        Thread[] threads = new Thread[10];
        for (int i = 0; i < threads.length; i++) {
            threads[i] = new Thread(() -> {
                for (int j = 0; j < 1000; j++) {
                    counter.increment();
                }
            });
            threads[i].start();
        }
        for (Thread t : threads) t.join();  // join() 而不是 sleep()
        System.out.println("最终值：" + counter.get());
        System.out.println(counter.get() == 10000 ? "✓ 通过" : "✗ 未通过");
    }
}
```

### CAS vs synchronized 核心对比

从本次对话推导出的对比维度：

| 维度 | CAS | synchronized |
|------|-----|-------------|
| 并发模型 | 乐观——先做，失败重试 | 悲观——先锁，操作后释放 |
| 冲突处理 | 自旋重试 | 线程挂起，等通知 |
| 操作粒度 | 单个变量 | 任意代码块 |
| 适用场景 | 低冲突、操作短 | 高冲突、或需多变量原子更新 |
| CPU 占用 | 冲突时持续占用 | 冲突时挂起，释放 CPU |
| JMM 保证 | 原子性（单变量） | 原子性 + 可见性 + 有序性 |

---

## 对话质量诊断

### 对话质量

| 维度 | 评级 | 说明 |
|------|------|------|
| 推导过程完整度 | 高 | 用户在 6 处主动质疑、猜测或提出反例，推导链完整 |
| 结论直给比例 | 低 | 80%+ 的结论有推导过程，AI 多数通过提问推进 |
| 关键转折覆盖度 | 高 | 所有主要转折均有记录（retry loop、ABA 质疑、不可变性推导、跨域迁移）|
| 用户主体性 | 高 | [U/U] 事件占多数，多处主动识别、质疑、连接 |

**综合评级**：高质量

### 路径来源

路径来源：用户主导（[U/U] 占多数）

复用建议：可直接重走，文档高度自足。ABA 部分建议配合「引导质量问题」章节一起阅读，避免被原始错误例子带偏。

### 模型适用性

适用性：完全适用（有疑问 → 推导 → 落点的主线清晰，覆盖感知层到洞察层）
