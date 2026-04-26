# Android 分区结构与隔离机制的完整链条 — 要点摘要

> **导航**：[distilled](../../../distilled/android-dev-docs/system-principles/android-partition-isolation-mechanism-complete-chain-distilled.md)

## 要点

1. **Mount Flags 在 VFS 层生效，早于 DAC 和 SELinux，是最先触发的一道墙**
   物理块通过挂载映射为目录路径，挂载时附加的 Flags（ro/noexec/nosuid/nodev）直接约束访问入口。`system`/`vendor`/`product` 挂载时同时带 `ro, nodev, nosuid`——不只只读，连提权路径也一并堵死，内核在进入 DAC/SELinux 检查之前就已拒绝。

2. **A/B slot：bootloader 和 userdata 不进 slot，原因截然不同**
   bootloader 是"决定切换哪个 slot 的决策者"，若自身进 slot 则出问题时无救；userdata 存用户数据，不随系统版本回滚或切换，设计语义上不属于 slot。所有系统分区（boot/init_boot/system/vendor/product）有 _a/_b 两份，整套切换保证版本一致性。

3. **DAC 权限记录在文件上，不在进程上**
   每个文件自身存储 owner UID/GID 和三组 rwx（owner/group/other），内核在访问时检查 caller 的 UID/GID 能否匹配文件的权限位。进程没有"这个进程对这些文件有权限"的自我记录，权限判断的依据在被访问的文件侧。
