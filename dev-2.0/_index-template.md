# {仓库名} 知识索引

> 仓库：`https://github.com/lulufoo/{repo}`
> 本文件由同步脚本自动维护，每次新增沉淀文档时追加一行。

---

| 主题 | 一句话描述 | GitHub URL |
|------|-----------|------------|
| {doc-theme} | {核心结论，不超过30字，不是标题的重复} | https://github.com/lulufoo/{repo}/blob/main/{doc-theme}/{ts}-{slug}.md |

---

## 填写规范

**主题**（`doc-theme`）
- 与目录名一致，kebab-case，英文
- 例：`dialogue-distillation-model`、`binder-ipc-internals`

**一句话描述**
- 概括文档核心结论或核心观点，不超过 30 字
- ❌ 不要重复标题：`DDM 工作流设计` → 等于没写
- ✅ 写出结论：`DDM 将对话归档分为4个阶段，P2 是核心产出点`

**GitHub URL**
- 使用绝对路径：`https://github.com/lulufoo/{repo}/blob/main/{doc-theme}/{ts}-{slug}.md`
- 通过 `gh api` 读取，跨本地/远程环境均可用
- 如果一个主题有多个文档，每个文档单独一行

---

## 示例（ai-assisted-domain-learning）

| 主题 | 一句话描述 | GitHub URL |
|------|-----------|------------|
| dialogue-distillation-model | DDM 将对话处理分 P0-P3 四阶段，P2 distilled 是核心可读产出 | https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/dialogue-distillation-model/202604231831-dialogue-distillation-model.md |
| layered-cognitive | LCCM 用"层间跨越"替代平铺，每层用具体问题驱动认知上升 | https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/layered-cognitive/202604231831-lccm-layered-concept-cognitive-model.md |
| domain-deepening | 领域深化三视图：框架视图→知识图谱→角色视图，依次递进 | https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/domain-deepening/202604231831-domain-framework-view-model.md |
