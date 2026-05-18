# JMM 原子性边界、volatile 的真实作用与 as-if-serial — 要点摘要

> **导航**：[raw](../../../raw/common-tech/jmm-atomicity-happens-before-serial/202604231831-jmm-atomicity-happens-before-and-as-if-serial.md)


## 要点

1. **JMM 原子性保证以"单次内存读/写"为单位；volatile 对原子性的贡献极窄**
   基本类型（除 `long`/`double`）的单次读写、引用类型读写，JMM 本来就保证原子，不需要 volatile。volatile 在原子性上的实质贡献仅限于：规范层要求 `long`/`double` 作整体原子处理（主流 JVM 已实现但不是规范保证）。volatile 的核心价值是可见性和有序性。

2. **DCL 漏洞的机制：new 操作可被重排为"分配→赋引用→初始化"**
   `new Singleton()` 在字节码层面可被重排为 step1（分配内存）→step3（写引用）→step2（初始化）。B 线程读到非空引用时对象字段可能尚未初始化。加 `volatile` 后，volatile 写规则禁止 step3 重排到 step2 前，并建立跨线程 happens-before 关系，B 线程读到引用时保证看到完整初始化的对象。

3. **happens-before 程序顺序规则保证单线程结果一致，不禁止重排序本身**
   happens-before 的保证是"单线程视角下结果与代码顺序一致"，不保护其他线程看到的中间状态。as-if-serial 同理：只保证单线程不感知重排序，线程间可见性需显式建立 happens-before 关系（volatile 写读、synchronized 解锁加锁等）。
