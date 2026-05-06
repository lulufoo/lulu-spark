# 代码规范：ai-coding-discipline

> 创建时间：2026年5月6日 11:57

> 导航：[distilled](../../../distilled/ai-software-dev/ai-coding-discipline-norms-dialogue/202605061157-ai-coding-discipline-norms-dialogue.md) · [digest](../../../digest/ai-software-dev/ai-coding-discipline-norms-dialogue/202605061157-ai-coding-discipline-norms-dialogue.md) · [trace](../../../trace/ai-software-dev/ai-coding-discipline-norms-dialogue/202605061157-ai-coding-discipline-norms-dialogue.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

介绍一下，这个skill 是做什么的？ https://github.com/luoling8192/ai-coding-principles

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn1

CHAT_ID-7f3a2b1c9e4d8f0a6b5c3e2d1f9a4b7c

用户意图：了解 `luoling8192/ai-coding-principles` 这个 skill 是做什么的。

根据已有信息（✅ 已验证，来源：[GitHub 页面](https://github.com/luoling8192/ai-coding-principles)），该项目包含两个 skill：

---

## ai-coding-principles 是什么

一个给 Claude Code 使用的 **AI 编码规范 skill 集合**，目标是约束 AI 写代码时的常见反模式，强制执行编码纪律。

---

### Skill 1：`ai-coding-discipline`

在所有代码编写任务中强制加载，防止 6 类常见 AI 编码反模式：

| # | 规则 | 说明 |
|---|------|------|
| 1 | No Silent Fallbacks | 禁止用 `??` / 默认值掩盖错误 |
| 2 | No Catch-All try/catch | 业务逻辑让错误向上传播，只在 API 边界捕获 |
| 3 | Tests Must Fail When Code Breaks | 验证具体结果，而不只是判断「是否存在」 |
| 4 | No Hardcoded Lookup Tables | 实现真实逻辑，不用硬编码表来凑测试用例 |
| 5 | Red-Green Testing (TDD) | 先写失败的测试，再修复代码 |
| 6 | Don't Remove Debug Logs During Fix | 修复期间保留日志，由人工确认修复后再移除 |

---

### Skill 2：`ddia-principles`

基于 Martin Kleppmann 的《Designing Data-Intensive Applications》提炼的参考指南，在设计数据库 schema、选存储引擎、实现复制/分区、处理分布式事务、构建批流处理时加载。

---

### 安装方式

```bash
npx skills add luoling8192/ai-coding-principles
```

---

**一句话总结**：这是一套给 AI 编码 Agent 用的「约束规则包」——不让 AI 写出那些看起来能跑、实则埋坑的代码。

Turn:1～1

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

「我根据响马老师的 PPT 写了一个对应的 SKILL 方便大家去 harness。」 这个skill就是根据这个ppt做出来的，你帮我做增强检索，找一下这个PPT。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn2

用户意图：定位「响马老师」那份被说成某 Harness/Cursor SKILL 来源的 **PPT**，并核实它和 [`luoling8192/ai-coding-principles`](https://github.com/luoling8192/ai-coding-principles) 的关系。

---

### 检索结论（先说结论）

1. **❌ 未解决**：在公开网页/GitHub 检索中，**没有找到可核验为「响马老师原创、且即为你所指 SKILL 来源」的单一 PPT 文件**。
2. **✅ 已验证**：仓库 [`luoling8192/ai-coding-principles`](https://github.com/luoling8192/ai-coding-principles) 的 **中文版 README 与 `ai-coding-discipline/SKILL.md` 正文里均未出现「响马」或「PPT」溯源**；对该仓库的 GitHub code 搜索 **`repo:luoling8192/ai-coding-principles 响马` 结果为 0**。
3. **⚠️ 推断**：听到的「根据响马老师 PPT 写的 SKILL」更可能指向 **Harness / 驾驭工程** 主题的分享，而不是「编码纪律六条 + DDIA」那条线；也可能 SKILL 不是 `ai-coding-principles`，而是别的 Harness 类 skill。

---

### 与「Harness / 驾驭工程」高度相关的公开材料（可作对照，但不等于「响马 PPT」）

| 材料 | 说明 |
|------|------|
| [搜狐转载：清华大学清新研究团队《2026 驾驭工程（Harness Engineering）研究》](https://www.sohu.com/a/1003439719_121864818) | 摘要称约 78 页报告 |
| [原创力文档：《驾驭（Harness）工程研究 PPT.pptx》](https://m.book118.com/html/2026/0329/8014114036010057.shtm) | 页面预览与「操作系统层 / 四层链条」等框架一致 |

---

### 建议下一步

- 把那条动态的 **原文截图或链接**（谁说的、指向哪个 SKILL 仓库）发来，可按仓库 README/issue/commit 再做针对性溯源。
- 若关心的是 **Harness 编排类 SKILL**，需单独核对名称，避免和 **`ai-coding-principles`（编码纪律 + DDIA）** 混为一谈。

Turn:1～3

基于 Claude 的 Cursor 对话助手（Composer 驱动）。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

ai-coding-discipline 是通用的规范吗？ 例如：Java 、 Kotlin都适用吗？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn3

用户意图：确认 `ai-coding-discipline` 的六条规则是否适用于 Java / Kotlin 等语言。

---

## 结论：规则通用，示例代码不通用

**六条规则本身是语言无关的工程原则**，但 **示例代码全部是 TypeScript**。

| 规则 | 通用性 | Java/Kotlin 对应说法 |
|------|--------|---------------------|
| **1. 禁止静默回退** | ✅ 通用 | Java：慎用 `Optional.orElse` 掩盖契约；Kotlin：慎用 `?:` 掩盖不该为 null 的字段 |
| **2. 禁止业务逻辑中的 catch-all** | ✅ 通用 | 宽泛 `catch` 吞错、返回 null |
| **3. 测试必须能发现代码缺陷** | ✅ 通用 | JUnit/Kotest 断言具体业务结果 |
| **4. 禁止硬编码查找表** | ✅ 通用 | 不用枚举式假实现蒙测试 |
| **5. 红绿测试（TDD）** | ✅ 通用 | 先失败测试再修复 |
| **6. 修复时不删调试日志** | ✅ 通用 | 日志保留到人工确认 |

SKILL 的 `description` 触发词偏向 Claude Code，且示例全是 TypeScript。**AI 在 Java/Kotlin 场景下仍可能优先类比 TS 语法**；若在 JVM 项目上使用，建议在 SKILL 里补充 **Java/Kotlin 示例对照**。

Turn:1～4

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 4）

这个原则，是在AI生成代码的时候，作为提示词约束导入的吗

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn4

用户意图：理解 `ai-coding-discipline` 的运行机制——是否以提示词的形式在 AI 生成代码时注入约束。

---

## 是的，本质上就是「按需注入的提示词约束」

工作机制是 Claude Code 的 **Skills 系统**：先 **发现**（frontmatter 元数据常驻），任务匹配时在 **激活阶段** 把完整 `SKILL.md` **加载进上下文**，六条规则作为约束影响生成代码。

frontmatter 写明必须在 writing code / fixing bugs / writing tests 等场景加载。

---

## 与手动写提示词的区别

| 对比项 | 手动在对话里写规则 | Skills 系统 |
|--------|------------------|-------------|
| 每次都要重复 | 是 | 否 |
| token | 每轮全量 | 元数据小，正文按需 |
| 跨项目复用 | 需复制 | `npx skills add` |

**一句话**：Skills 是把规范做成可安装、懒加载的模块；`ai-coding-discipline` 是其中管编码纪律的那一个。

Turn:1～5

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 5）

执行：dtd_normalize， 主题是  代码规范：ai-coding-discipline

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn5

用户意图：执行 `dtd_normalize`，主题为「代码规范：ai-coding-discipline」，归档本轮对话。

> ✅ config.json 读取完成 · archive_root: `/Users/lulu/Code/cognitive-trace-archive`
>
> topic-path：`ai-software-dev/agentic-coding`（选自 topics.json · `lulufoo/ai-software-dev` → agentic-coding）

---

> ✅ dtd_normalize 完成  
> 📄 raw：`raw/ai-software-dev/ai-coding-discipline-norms-dialogue/202605061157-ai-coding-discipline-norms-dialogue.md`  
> 🗂 index.json 已更新（layers 新增 raw · id `9516be4e728e5c91d810b2fd34dff201`）

Turn:1～6

模型：Claude Sonnet 4.6（Anthropic）
