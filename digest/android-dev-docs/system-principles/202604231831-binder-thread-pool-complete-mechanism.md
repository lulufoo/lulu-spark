# Binder 线程池完整机制链 — 要点摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/system-principles/202604231831-binder-thread-pool-complete-mechanism.md)


## 要点

1. **线程池由子进程自己初始化，不是 Zygote 建的**
   启动链路：`ProcessState::self()`（打开 /dev/binder，进程级单例）→ `startThreadPool()` → `spawnPooledThread(isMain=true)` → `joinThreadPool()`。`IPCThreadState` 是线程级的，每条 Binder 线程各有一个实例，负责收发逻辑。

2. **ioctl(BINDER_WRITE_READ) 是双向通道；线程在内核等待队列挂起后自动循环**
   发送方和接收方都调用同一个 ioctl，区别只在参数（write_size>0 或 read_size>0）。等待中的线程通过 `read_size>0` 陷入内核挂起；有请求时驱动唤醒线程，处理完后自动进入下一轮循环再次陷入内核，不需要外部调用触发。

3. **调度权在内核驱动侧；main 线程和普通线程无管理等级之分**
   驱动维护等待队列并负责唤醒（决策者），用户态 ProcessState 创建/销毁线程（执行者）。main 线程（`isMain=true`）与普通线程的区别只是永不超时退出，不具备管理职责。线程上限由驱动通过 `BINDER_SET_MAX_THREADS` 控制。
