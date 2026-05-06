# Binder 线程池完整机制链 — 摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/binder-thread-pool-complete-mechanism/202604231831-binder-thread-pool-complete-mechanism.md)


## 概述

本文从「Binder 线程池是 Zygote fork 时建立的吗」这一直觉误解出发，推导出线程池由子进程自己在启动阶段初始化——ProcessState::self() 打开 /dev/binder 建立驱动连接，startThreadPool() 创建第一条线程。线程池扩容由内核驱动主动触发：当所有线程都在处理请求时，驱动向用户空间发 BR_SPAWN_LOOPER，用户空间响应创建新线程，上限默认 15 条；main 线程与普通线程的唯一区别是永不超时退出，不具备管理职责。
