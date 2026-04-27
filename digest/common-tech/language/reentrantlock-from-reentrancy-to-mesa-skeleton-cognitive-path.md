# ReentrantLock：从可重入命名到 Mesa 骨架与 AQS 家族 — 要点摘要

> **导航**：[distilled](../../../distilled/common-tech/language/reentrantlock-from-reentrancy-to-mesa-skeleton-cognitive-path-distilled.md)

## 要点

1. **ReentrantLock 默认非公平；相比 synchronized 的真正优势是"表达力"**
   `new ReentrantLock(true)` 才是公平构造，默认非公平。synchronized 与 ReentrantLock 都可重入，"可重入"不是后者的独占卖点。后者的价值在于：多 `Condition`（拆分不同业务谓词的等待）、可中断等待（`lockInterruptibly`）、限时尝试（`tryLock(timeout)`）、公平策略可选。

2. **AQS 的 state 字段同时表达"被占用"和"重入深度"；lock() 是多段路径**
   `state=0` 无人占用，`state>0` 时数值即嵌套深度，一次 CAS 完成状态变更。`lock()` 路径不是纯自旋：先 CAS 尝试→失败则进 CLH 变体队列→队头有限自旋→最终 `park` 挂起。`tryLock` 只走"快速尝试"那一截，拿不到立即返回，是不同的心智模型。

3. **公平 vs 非公平的差别在"新线程首次抢锁时是否允许绕开已等待者"**
   公平版在 CAS 前用 `hasQueuedPredecessors()` 检查；非公平版直接 CAS。FIFO 队列共用，不是链表方向不同。默认非公平吞吐更高（新到线程在 CPU 上，切换成本低），极端情况下可能出现饥饿。

4. **Condition 实现 Mesa 模型：必须用 while 而非 if 检查谓词**
   每个 `Condition` 对象有独立等待队列，`signal()` 将线程移入 AQS 主队列参与锁竞争。由于 spurious wakeup（虚假唤醒）和多消费者竞争，线程被唤醒后不保证谓词仍成立，必须用 `while` 重检。
