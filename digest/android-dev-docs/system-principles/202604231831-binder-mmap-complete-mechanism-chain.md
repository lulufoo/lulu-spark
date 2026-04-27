# Binder mmap：从"减少拷贝"到"安全边界"的完整机制链 — 要点摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/system-principles/binder-mmap-complete-mechanism-chain-distilled.md)

## 要点

1. **mmap 省掉的是"接收端的第二次拷贝"，发送进程不参与映射**
   发送进程通过 `copy_from_user` 把数据推入接收进程的内核缓冲区（一次拷贝）；接收进程的用户空间与该内核缓冲区映射到同一物理页，接收方直接读，无需再拷贝（省掉第二次）。发送进程的角色是"推数据进去"，mmap 的价值全在接收端。

2. **映射在进程初始化时一次性建立，每个进程都有自己的 mmap 区域**
   `open(/dev/binder) + mmap()` 在进程加入 Binder 体系时就完成，后续所有 IPC 复用这块区域。"发送方"和"接收方"只是每次 IPC 的角色，不是固定身份——因此每个进程都必须有自己的 mmap 区域。

3. **内核用两棵红黑树管理缓冲区；接收方必须主动释放**
   一棵按大小排空闲块，一棵按地址排已分配块，每次 IPC 做 best-fit 分配。接收方处理完毕后必须显式发送 `BC_FREE_BUFFER`，否则 ~1MB 区域会被逐渐耗尽，导致后续 IPC 失败。
