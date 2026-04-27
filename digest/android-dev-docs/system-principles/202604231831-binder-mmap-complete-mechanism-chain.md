# Binder mmap：从"减少拷贝"到"安全边界"的完整机制链 — 摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/system-principles/202604231831-binder-mmap-complete-mechanism-chain.md)


## 概述

本文从「mmap 是 Binder 使用的内存技术」这一模糊直觉出发，推导出 mmap 省掉的具体是接收端那次拷贝——发送方通过 copy_from_user 把数据推入内核缓冲区（一次拷贝），接收方用户空间与该内核缓冲区映射到同一物理页，直接读取无需第二次拷贝；常见误解是认为发送方也参与了映射，实际上映射在接收进程初始化时一次性建立。内核用两棵红黑树管理缓冲区，接收方处理完必须显式发送 BC_FREE_BUFFER 释放，否则约 1MB 区域会被耗尽。
