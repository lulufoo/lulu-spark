# system prompt 一级知识索引模板

> 本文件由 `update_topics_from_github.py` 根据 `topics.json` 自动生成。
> 使用方式：将下方文本块**整体复制**，粘贴到 system prompt 或 instruction 的合适位置。

---

## 使用方式

```
当用户话题涉及以下领域时，主动查阅对应知识库：
- Android 技术实现（Binder / AQS / 协程 / View）→ android-dev-docs
- AI 学习方法 / DDM / LCCM / 领域学习 → ai-assisted-domain-learning
- AI 协作思维框架 / 解题模型 / 画像 → ai-thinking-framework
- AI 协作机制 / 补丁倾向 / 意图约束 → ai-collaboration-framework
- 用户明确要求"查我的知识库" → 根据话题选择仓库

查阅步骤：
1. 从下方一级索引确认目标仓库
2. 用 gh api 读取该仓库 _index.md，定位具体主题
3. 用 gh api 读取目标文档
```

---

## 一级索引文本块（直接粘贴到 prompt）

```
## LuLu 知识库

ai-thinking-framework: AI 思维框架，解题模型/画像模型/目标拆解/TPM; https://github.com/lulufoo/ai-thinking-framework/blob/main/_index.md
ai-assisted-domain-learning: AI 辅助学习方法，DDM/LCCM/领域深化/对话蒸馏; https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/_index.md
ai-collaboration-framework: AI 协作机制，补丁倾向/意图约束/协作反模式; https://github.com/lulufoo/ai-collaboration-framework/blob/main/_index.md
ai-software-dev: AI 工程实践，Agent/Harness/工具链/Copilot; https://github.com/lulufoo/ai-software-dev/blob/main/_index.md
ai-authored-learning: AI 生成学习内容，产品分析/领域洞察; https://github.com/lulufoo/ai-authored-learning/blob/main/_index.md
learning-with-ai: AI 学习方法论，认知模型/学习经验; https://github.com/lulufoo/learning-with-ai/blob/main/_index.md
learning-ai-agent: AI Agent 学习，Cursor/Skill/Agent模式; https://github.com/lulufoo/learning-ai-agent/blob/main/_index.md
android-dev-docs: Android 技术知识，Binder/AMS/协程/View绘制/AQS; https://github.com/lulufoo/android-dev-docs/blob/main/_index.md
social-sciences: 社会科学与认知心理，感知/意义/行为; https://github.com/lulufoo/social-sciences/blob/main/_index.md
```

---

## topics.json 扩展格式说明

当前 `topics.json` 需在各条目增加以下字段，供脚本生成上方文本块：

```json
{
  "repo": "lulufoo/ai-thinking-framework",
  "description": "AI 思维框架，解题模型/画像模型/目标拆解/TPM",
  "keywords": ["思维框架", "解题", "画像模型", "TPM", "目标拆解"]
}
```

| 字段 | 要求 |
|------|------|
| `description` | ≤ 25 字，包含 3-5 个核心关键词，用 `/` 分隔；AI 匹配触发依赖此字段 |
| `keywords` | 关键词数组，用于后续向量化检索升级 |

---

## 维护说明

- 每次 `topics.json` 新增仓库或修改 `description` 后，运行 `update_topics_from_github.py` 重新生成本文件
- 本文件仅作为"生成产物存档"，实际使用时直接复制文本块内容到 prompt
