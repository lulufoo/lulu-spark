# Java Monitor 机制与 Mesa 模型：从 synchronized 到 AQS — 要点摘要

> **导航**：[distilled](../../../distilled/common-tech/ddm-mesa-model-to-aqs/202604231831-ddm-monitor-mesa-model-to-aqs.md)


## 要点

1. **wait() 必须原子地"释放锁 + 入 WaitSet"，不可分割**
   若拆成两步，`notify()` 可能在线程进入 WaitSet 之前就发出，导致永久丢失唤醒（Lost Wakeup）。原子性由 JVM 的 `ObjectMonitor::wait()` 在内部锁保护下先入 WaitSet 再释放 monitor 来保证，不是 `synchronized` 关键字本身。

2. **ObjectMonitor 三结构：_WaitSet、_EntryList、_cxq 各有分工**
   `_WaitSet`：wait() 后等 notify；`_EntryList`：FIFO 等锁队列；`_cxq`：新竞争者先进（LIFO），由锁持有者退出时合并进 `_EntryList`。`notify()` 唤醒的线程直接进 `_EntryList`，不经过 `_cxq`。

3. **synchronized 不公平性来自三处叠加，不是单一原因**
   ① `_cxq` 是 LIFO（后入先合并）；② 新线程自旋直接 CAS 可绕开已排队线程；③ `_cxq` 合并时机不确定（由锁退出者触发，无上限保证）。`ReentrantLock(true)` 用严格 FIFO 解决这个问题，代价是性能下降。
