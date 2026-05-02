# DDM Skill 构建与 Size 优化

> 创建时间：2026年5月2日 15:29

> 导航：[distilled](../../../distilled/ai-assisted-domain-learning/dialogue-distillation-model/202605021529-ddm-skill-build-optimize.md) · [digest](../../../digest/ai-assisted-domain-learning/dialogue-distillation-model/202605021529-ddm-skill-build-optimize.md) · [trace](../../../trace/ai-assisted-domain-learning/dialogue-distillation-model/202605021529-ddm-skill-build-optimize.md)

<!-- DDM:TURN_SEP:v1 -->

## 先行上下文（会话摘要，前期工作）

本对话为多会话续接，以下为压缩摘要，非逐字记录。

前期工作：

1. 从 GitHub 私有仓库拉取 DDM 文档（通过 `gh api`），构建 `skills/ddm/` 完整目录结构：`SKILL.md`、`config.json`、7 个 reference 文件（`dialogue-to-doc.md`、`ddm-concepts.md`、`ddm-p0-normalize.md` 至 `ddm-p4-digest.md`）
2. 新增 diagnose 层（`diagnose/<COMMON_PATH>`），同步更新 index.json v4 schema（54 条目加 `"diagnose": false`）
3. `ddm-concepts.md` 瘦身：理论部分移入各 p[x] PART 1，concepts 保留 intro + 路径符号配置
4. P1/P2/P3/P4 增加 PART 1（理论）/ PART 2（执行）结构
5. `ddm-p4-digest-archive.md` → `ddm-p4-digest.md` 重命名，旧文件留 stub redirect，3 处引用更新
6. 各 p[x] 文件添加 `> **配置参考**：[ddm-concepts.md](ddm-concepts.md)` 头部
7. Skill symlink 安装：`~/.copilot/skills/ddm → /Users/lulu/Code/cognitive-trace-archive/skills/ddm`
8. 全文件冗余审查，产出优化候选列表（A 类 6 项直接删、B 类 5 项可压缩、C 类 2 项结构重构）

当前 skill 总大小（优化前）：31,506B

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

执行优化吧 优化完了重新安装skill

（附件：ddm-p1-diagnose.md 第 3–4 行，配置参考行选中）

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

执行 A1–A6 + B1–B3 共 8 项优化：

- **A1+A2**（ddm-p2-generate.md）：删除 [P2-4] 格式规则整节（与 [P2-1]/[P2-2] 重复），[P2-5] 导航前缀公式改为 inline 引用 P0 Step 2
- **A3**（ddm-p4-digest.md）：删除 Phase 4 完成后整体汇总输出块（dialogue-to-doc.md DTD 完成汇总已覆盖）
- **A4**（dialogue-to-doc.md）：删除 config.json JSON 示例块，改为单行说明
- **A5**（ddm-p1-diagnose.md）：P1-7 模板顶部加"可选字段无内容时写「无」"，删除各字段末尾 6 处 `/ 无则写「无」`
- **A6**（ddm-p4-digest.md）：[U/U]/[U/AI] 高自主性事件两条定义改为引用 ddm-p3-trace.md PART 1
- **B1**（ddm-p0-normalize.md）：Step 2 示例从 4 轮缩减为 2 轮
- **B2**（ddm-p1-diagnose.md）：象限表删除"文档叙述重点"列
- **B3**（ddm-p1-diagnose.md）：P1-7 入口假设 + 跃迁点的括号注释删除（与 P3 PART 1 重复）

结果：31,506B → 29,801B（-5.4%）

重新安装：`rm -f ~/.copilot/skills/ddm && ln -s ... ` → symlink 重建 ✅

各文件变化：

| 文件 | 优化前 | 优化后 |
|------|--------|--------|
| dialogue-to-doc.md | 2144B | 2072B |
| ddm-p4-digest.md | 3607B | 3023B |
| ddm-p2-generate.md | 6910B | 6438B |
| ddm-p1-diagnose.md | 7628B | 7168B |
| ddm-p0-normalize.md | 3541B | 3424B |

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

如何执行你刚刚安装的skill？ 执行 acn 吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 2）

在 VS Code Copilot Chat 里，直接用自然语言触发即可：

```
@workspace 蒸馏 acn
```

或者更明确：

```
执行 ACN：[在此粘贴或描述你要归档的对话]
```

Copilot 会加载 SKILL.md，读取 config.json，然后走 P0 流程（选 topic-path → 生成 CACHE_RAW → 写入 raw/ → 更新 index.json）。

触发关键词（来自 SKILL.md description）：`蒸馏 distill ddm normalize archive 归档 acn dtd`，命中任意一个即可激活 skill。

如果要完整走全流程（P0→P4），用：

```
执行 DTD：[对话内容]
```

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

当前对话，执行 ACN

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 3）

（执行 ACN P0：归一化当前对话，写入 raw/，更新 index.json）
