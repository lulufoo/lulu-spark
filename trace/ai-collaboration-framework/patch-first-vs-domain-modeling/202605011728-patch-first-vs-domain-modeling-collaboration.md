# 领域建模驱动 AI：编程协作设计(从AI打补丁说起） — 认知轨迹

> 创建时间：2026年5月1日 17:28

> 导航：[raw](../../../raw/ai-collaboration-framework/patch-first-vs-domain-modeling/202605011728-patch-first-vs-domain-modeling-collaboration.md) · [distilled](../../../distilled/ai-collaboration-framework/patch-first-vs-domain-modeling/202605011728-patch-first-vs-domain-modeling-collaboration.md) · [digest](../../../digest/ai-collaboration-framework/patch-first-vs-domain-modeling/202605011728-patch-first-vs-domain-modeling-collaboration.md)

---

## 入口假设

AI 打补丁的问题可以在协作层通过设计更好的规则/约束来修正，不需要改变 AI 本身

---

## 认知结构变化

**学习起点**：AI 打补丁的问题可以在协作层通过设计更好的规则/约束来修正；协作设计的核心是粒度拆分和任务分配

**关键跃迁**：Turn8 User 从 DbC/契约的复杂化方向突然回归，将形式化约束、架构设计、代码形状控制三线统一到"领域建模"这一框架下，多个前序疑问（形式化约束如何系统化、架构约束和代码形状的关系）被同时解答

**落点**：AI + 人类协作编程的正确目标不是"修正 AI 的打补丁行为"，而是"用领域建模定义代码形状，AI 在形状内施工"——这一框架下，避免打补丁是自然结果，不是需要单独追求的目标；人类的认知代价（架构判断前置）是真实的，协作层让它变得可管理，不能消除它

---