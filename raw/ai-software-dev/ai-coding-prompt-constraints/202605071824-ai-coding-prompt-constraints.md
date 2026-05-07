# AI Coding — 编码提示词约束

> 创建时间：2026年5月7日 18:24

> 导航：[distilled](../../../distilled/ai-software-dev/ai-coding-prompt-constraints/202605071824-ai-coding-prompt-constraints.md) · [digest](../../../digest/ai-software-dev/ai-coding-prompt-constraints/202605071824-ai-coding-prompt-constraints.md) · [trace](../../../trace/ai-software-dev/ai-coding-prompt-constraints/202605071824-ai-coding-prompt-constraints.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

我的对话过程的Rules和编码的Rules，可以区分开吗？ 例如：对话过程，可能是方案设计，我需要专门的提示词约束对话过程。编码的构成，需要代码相关的约束。 可以做到自动切换吗？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn1
CHAT_ID-a7f3e2d9b14c8f6e0a2d5c3b7e9f1a4d

用户意图：了解 Cursor 是否支持将对话规范与编码规范分开管理，并自动切换。

先读 create-rule 技能，再给出准确答案。

The user is asking whether they can separate their conversation rules (for things like planning, design discussions) from coding rules, and whether Cursor can automatically switch between them.

This is about Cursor rules - specifically about creating different rule sets that apply in different contexts. Let me read the create-rule skill to understand what's possible.✅ 已验证（Cursor rules 机制）

**可以区分，且支持自动切换。**核心机制是 `.cursor/rules/` 下的 `.mdc` 文件，通过两个 frontmatter 字段控制激活时机：

| 字段 | 行为 |
|------|------|
| `alwaysApply: true` | 每次会话都生效 |
| `globs: **/*.ts` | 当匹配的代码文件被打开时自动生效 |
| 两者都不设 | 手动引用才生效 |

---

**你的场景对应方案：**

**对话/方案设计规范** → `alwaysApply: true`（全局生效，始终在场）

```yaml
---
description: 对话过程规范（方案设计、需求分析）
alwaysApply: true
---
# 对话规范
- 先暴露假设，再给出方案
- 输出所有合理解读，禁止默默选一个执行
- 不确定时如实标注 ⚠️
```

**编码规范** → `globs` 绑定文件类型（打开代码文件时自动叠加）

```yaml
---
description: 编码约束（TypeScript / Python）
globs: **/*.ts,**/*.tsx,**/*.py
alwaysApply: false
---
# 编码规范
- 禁止空 catch
- 函数不超过 40 行
- ...
```

---

**自动切换的实际效果：**
- 纯聊天/方案设计：只有对话规范生效
- 打开 `.ts` / `.py` 等文件后：编码规范**自动叠加**进来

**一个限制**：Cursor 不支持"仅当没有代码文件时才生效"的条件，所以对话规范如果设为 `alwaysApply: true`，写代码时它也在场。解决方式是让对话规范写得足够轻量，不干扰编码上下文，或者改用手动引用模式（不设任何激活条件，需要时用 `@rule名` 调用）。

---

需要我现在帮你把现有的 user_rules 拆分为这两个文件吗？

Turn:1～1

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

但是，我需要的是全局的Users 设置，不是放到项目里面的。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn2

用户意图：确认全局（跨项目）的 User Rules 是否支持对话/编码规范自动切换。

先查证 Cursor 全局规则的实际能力，再给结论。

The user wants global user-level rules, not project-level rules. In Cursor, there are User Rules in the settings (Settings > General > Rules for AI / User Rules). These are global and apply across all projects. Let me think about what options exist for global rules in Cursor.

Cursor has:
1. Project rules: `.cursor/rules/*.mdc` - project specific
2. User rules: In Cursor Settings > General > Rules for AI - these are plain text, global

The user rules in settings are a single text field - you can't have multiple rule files or glob-based activation there. So the auto-switching based on file type isn't available at the global user level.


I'm trying to recall if Cursor has a global rules directory like `~/.cursor/rules/` that would work similarly to project rules, but I'm not confident about that. It seems Cursor only supports the single text block for user rules in settings, with no glob-based switching at the global level.✅ 已验证（Cursor 官方文档）

**结论：在全局（User Rules）层面，不支持自动切换。**

---

**三种全局规则的能力对比：**

| 类型 | 全局？ | 支持 glob 条件？ | 说明 |
|------|--------|-----------------|------|
| **User Rules**（Settings → Rules） | ✅ | ❌ | 单一文本块，永远生效 |
| **Project Rules**（`.cursor/rules/`） | ❌ | ✅ 支持 glob | 仅限项目内 |
| **Team Rules** | ✅ | ✅ 支持 glob | 需要 Team/Enterprise 付费套餐 |

---

**对你的直接影响：**

你现在的 User Rules 是一个**纯文本块、始终生效、无条件触发**。没有办法在全局层面配置"方案设计时用这段，写代码时用那段"的自动切换。

**现实可行的替代方案：**

1. **接受"叠加"而非"切换"**：User Rules 写对话规范（始终在场），每个项目的 `.cursor/rules/` 写编码规范（打开代码文件时自动叠加）。跨项目复用就在每个项目里放同一份 `.mdc` 文件。

2. **手动引用模式**：把编码规范单独写成 `@coding-rules` 的项目规则（不设 alwaysApply，不设 globs），需要时手动 `@` 引用。

3. **升级 Team 套餐**：Team Rules 支持 glob，可以全局生效且按文件类型区分——这是唯一能做到你要求的原生方案。

---

目前你的 User Rules 已经很详细，如果想在现有条件下尽量减少干扰，可以把编码规范部分**从 User Rules 里剥离出来**，下沉到各项目的 Project Rules。需要我帮你做这个拆分吗？

Turn:1～2

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

请调研一下，Claude Code 怎么解决这个问题的？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn3

用户意图：了解 Claude Code 如何区分对话规范与编码规范（对应 Cursor User Rules 问题的参考方案）。

The user wants to know how Claude Code handles the separation of conversation rules vs coding rules. Let me research this.✅ 已验证（Anthropic 官方文档）

Claude Code 的方案比 Cursor 完善得多，**在全局层面原生支持按文件类型自动切换**。

---

## Claude Code 的解法

核心机制：**`~/.claude/rules/` 全局规则目录 + `paths` frontmatter**

```text
~/.claude/
├── CLAUDE.md          # 全局始终生效（等同 Cursor User Rules）
└── rules/
    ├── conversation.md    # 对话规范，无 paths → 始终生效
    └── coding.md          # 编码规范，有 paths → 自动切换
```

**对话规范**（无 `paths`，始终加载）：

```markdown
# 对话规范
- 先暴露假设，再给出方案
- 不确定时标注 ⚠️
```

**编码规范**（有 `paths`，仅当读取匹配文件时触发）：

```markdown
---
paths:
  - "**/*.ts"
  - "**/*.py"
  - "**/*.go"
---
# 编码规范
- 禁止空 catch
- 函数不超过 40 行
```

---

## 与 Cursor 的对比

| 能力 | Cursor | Claude Code |
|------|--------|-------------|
| 全局始终生效的规则 | ✅ User Rules（单文本块） | ✅ `~/.claude/CLAUDE.md` |
| **全局 + 按文件类型自动切换** | ❌ 不支持 | ✅ `~/.claude/rules/` + `paths` |
| 层级覆盖（org → user → project） | ❌ 无 | ✅ 四层（managed / user / project / local） |
| 规则按需加载（Skills） | ❌ 无全局对应 | ✅ `~/.claude/skills/` |

---

**结论**：Claude Code 通过 `~/.claude/rules/` 目录 + `paths` frontmatter，在个人用户层面就实现了你想要的效果——对话规范全局生效，编码规范打开代码文件时自动叠加。这个能力在 Cursor 里目前只有 Team/Enterprise 套餐才有对应方案。

Turn:1～3

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 4）

你查一下 Visual Studio Code实现了这个吗

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn4

The user wants to know if Visual Studio Code has implemented a similar mechanism for separating conversation rules from coding rules with auto-switching. Let me search for this.✅ 已验证（VS Code 官方文档）

**VS Code 实现了，而且能力与 Claude Code 几乎对齐。**

---

## VS Code 的方案

核心文件类型：`*.instructions.md`，通过 `applyTo` frontmatter 控制激活范围。

**对话规范**（`applyTo: "**"` → 始终生效）：

```markdown
---
applyTo: "**"
---
# 对话规范
- 先暴露假设，再给出方案
```

**编码规范**（`applyTo: "**/*.py"` → 仅匹配文件时生效）：

```markdown
---
name: 'Python Standards'
applyTo: '**/*.py'
---
# Python 编码规范
- 使用 type hints
- 遵循 PEP 8
```

**全局（跨所有工作区）**放在：

```
~/.copilot/instructions/
~/.claude/rules/          ← VS Code 同样识别这个目录！
```

---

## 三者横向对比

| 能力 | Cursor | Claude Code | VS Code (Copilot) |
|------|--------|-------------|-------------------|
| 全局始终生效 | ✅ User Rules（单块） | ✅ `~/.claude/CLAUDE.md` | ✅ `~/.copilot/instructions/` |
| **全局 + 按文件类型自动切换** | ❌ | ✅ `~/.claude/rules/` + `paths` | ✅ `~/.copilot/instructions/` + `applyTo` |
| 用户级规则目录（多文件） | ❌ | ✅ | ✅ |
| 识别 `~/.claude/rules/` | ❌ | ✅ | ✅（兼容 Claude 格式） |
| 无需项目文件即可使用 | ❌ | ✅ | ✅ |

---

**结论**：VS Code 和 Claude Code 都原生支持"对话规范全局生效、编码规范按文件类型自动叠加"。**Cursor 在 User Rules 层面是这三者中唯一未实现此能力的**，目前只能靠项目级 `.cursor/rules/` 补偿，或等待 Team 套餐/功能更新。

Turn:1～4

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 5）

能不能在项目中，加一个.rules_sync.json 文件，文件记录rules规则，以及同步输出的目录。项目启动之后，llm主动执行一次同步到对应目录下？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn5

用户意图：评估通过 `.rules_sync.json` + LLM 自动同步的方式，将 Cursor 项目规则同步到全局目录（绕过 Cursor User Rules 不支持 glob 的限制）。

**可以实现。** 有两条路，关键差别在触发方式：

---

## 方案设计

**`.rules_sync.json` 结构：**

```json
{
  "version": "1.0",
  "sync": [
    {
      "source": ".cursor/rules/conversation.mdc",
      "targets": [
        "~/.claude/rules/conversation.md",
        "~/.copilot/instructions/conversation.instructions.md"
      ]
    },
    {
      "source": ".cursor/rules/coding.mdc",
      "targets": [
        "~/.claude/rules/coding.md",
        "~/.copilot/instructions/coding.instructions.md"
      ]
    }
  ]
}
```

---

## 两种触发方式的权衡

| | **Path A：Hook 触发**（Shell） | **Path B：Rule 触发**（LLM） |
|--|--|--|
| 触发方式 | Cursor Hook 在会话开始时自动执行 Shell 脚本 | `alwaysApply: true` 的 Rule 指示 LLM 在每次会话开始时执行同步 |
| 可靠性 | ✅ 100% 确定执行 | ⚠️ LLM 可能跳过或合并到其他操作 |
| 对 Token 的影响 | ✅ 无 | ⚠️ 每次会话消耗少量 Token |
| 可见性 | 静默执行 | LLM 会主动告知"已同步" |
| 实现复杂度 | 需要写 Hook + Shell 脚本 | 只需一条 Rule |

你说的"LLM 主动执行"对应 **Path B**，它的 Rule 大致是：

```markdown
---
alwaysApply: true
---
# 规则同步
每次会话开始时：
1. 检查项目根目录是否存在 `.rules_sync.json`
2. 若存在，读取并将每个 `source` 文件复制到对应 `targets` 路径
3. 用 Shell 执行，完成后简短告知结果
```

---

## 一个潜在问题

Path B 中，LLM 是"被建议执行"而非"被强制执行"——如果会话一开始用户就发了很长的消息，LLM 可能直接进入任务而跳过同步。

**更健壮的组合**：Path A（Hook）做保底执行，Path B（Rule）可选做可见性提示。

---

需要我帮你实现哪条路？或者两条都做？

Turn:1～5

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 6）

或者这个 `.rules_sync.json` 做成一个prompt，主动找LLM同步一下。对话中要求

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn6

用户意图：用 `.rules_sync.json` 作为配置，用户在对话中主动要求 LLM 执行同步，而非自动触发。

这条路最简单，不需要 Hook，不消耗每次会话的 Token。实质上就是一个**按需调用的 Skill**。

---

## 设计

**三个文件：**

```
.rules_sync.json       ← 同步配置（source → targets）
.cursor/skills/
  sync-rules/
    SKILL.md           ← LLM 执行同步的指令
```

**触发方式**：在对话中说任意自然语言，例如：
- "帮我同步规则"
- "sync rules"
- "把规则同步到 Claude 和 VSCode"

---

**与 Cursor Skill 机制完全契合**，Skill 只在用户主动调用时才加载，零常驻开销。

---

需要我现在帮你实现这两个文件吗？需要先确认一下：

1. 你的 `.cursor/rules/` 里现有哪些规则文件需要同步？
2. 同步目标是哪些工具？（Claude Code、VS Code Copilot、还是其他？）

或者直接告诉我你的工作区路径，我去读一下现有结构。

Turn:1～6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 7）

这个skill，需要支持多平台，你思考一下，如何实现？ 目前有对话 / 编码 / ... 未来还有不同的场景。https://github.com/lulufoo/ai-software-dev/tree/main/sys-prompt/dialogure

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn7

先读仓库结构，再设计方案。

The user wants me to think about how to implement a multi-platform rule sync skill that supports:
1. Multiple platforms (Cursor, Claude Code, VS Code Copilot)
2. Multiple rule categories: conversation, coding, and future scenarios
3. References a GitHub repo for context

Let me first fetch the GitHub repo content to understand the existing structure, then design the skill.

The URL is: https://github.com/lulufoo/ai-software-dev/tree/main/sys-prompt/dialogure

According to the user rules, I need to use `gh api` to fetch GitHub content.

Let me parse the URL:
- owner: lulufoo


- repo: ai-software-dev
- ref: main
- path: sys-prompt/dialogure

Now I'll query the GitHub API to list the contents of that directory and see what rule structures already exist.✅ 已读取源文件。现在我有足够上下文了。

---

## 设计思考

**核心问题**：三个平台的规则文件格式不同，但内容是同一份。

| 平台 | 触发字段 | 示例 |
|------|----------|------|
| Cursor | `alwaysApply: true` / `globs: **/*.ts` | `.cursor/rules/*.mdc` |
| Claude Code | 无字段（全局）/ `paths: ["**/*.ts"]` | `~/.claude/rules/*.md` |
| VS Code | `applyTo: "**"` / `applyTo: "**/*.ts"` | `~/.copilot/instructions/*.instructions.md` |

三个平台的**内容相同，frontmatter 不同**。因此：**内容与格式分离**，`rules_sync.json` 只描述映射关系，Skill 负责转换并输出。

---

## 推荐结构

```
.rules_sync.json       ← 唯一配置文件
rules/
  conversation.md      ← 纯内容，无 frontmatter（source of truth）
  coding.md
  ...                  ← 未来场景直接添加文件 + json 条目
```

**`.rules_sync.json` schema：**

```json
{
  "version": "1.0",
  "source_base": "rules/",
  "scenarios": {
    "conversation": {
      "source": "conversation.md",
      "scope": "always",
      "platforms": {
        "cursor":     { "output": ".cursor/rules/conversation.mdc" },
        "claude-code":{ "output": "~/.claude/rules/conversation.md" },
        "vscode":     { "output": "~/.copilot/instructions/conversation.instructions.md" }
      }
    },
    "coding": {
      "source": "coding.md",
      "scope": "code",
      "glob": "**/*.{ts,tsx,py,go,java,rs}",
      "platforms": {
        "cursor":     { "output": ".cursor/rules/coding.mdc" },
        "claude-code":{ "output": "~/.claude/rules/coding.md" },
        "vscode":     { "output": "~/.copilot/instructions/coding.instructions.md" }
      }
    }
  }
}
```

`scope` 字段驱动 frontmatter 生成逻辑：

| scope | Cursor frontmatter | Claude Code frontmatter | VS Code frontmatter |
|-------|--------------------|------------------------|---------------------|
| `always` | `alwaysApply: true` | 无 | `applyTo: "**"` |
| `code` | `globs: {glob}` | `paths: [...]` | `applyTo: {glob}` |

---

## Skill 执行流程

```
用户说「同步规则」
  → 读 .rules_sync.json
  → 对每个 scenario：
      读 source 文件内容
      对每个 platform：
          根据 scope + glob 生成对应 frontmatter
          拼接 frontmatter + 内容
          写入 output 路径（自动展开 ~/）
  → 输出同步结果摘要
```

---

## 可扩展性

- **新场景**（如 `review`、`architecture`）：加一个 `rules/review.md` + json 条目，Skill 零修改。
- **新平台**（如 Windsurf、JetBrains）：在 `platforms` 下加条目，Skill 的 frontmatter 生成逻辑加一个分支。
- **远程 source**：`source` 字段可扩展为 GitHub 路径，Skill 用 `gh api` 拉取，而不是读本地文件。

---

你看这个方向是否对？确认后我来实现 `SKILL.md` 和 `.rules_sync.json` 模板。另外需要确认一下：规则内容的 source of truth 放本地 `rules/` 目录，还是直接指向你的 GitHub 仓库（`lulufoo/ai-software-dev`）？

Turn:1～7

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 8）

那我的规则，需要放到skill里面去吗？ 例如：https://github.com/lulufoo/ai-software-dev/tree/main/sys-prompt/dialogure

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn8

用户意图：确认规则内容是否需要放进 Skill 文件里，还是可以保留在 GitHub 仓库中。

**不需要放进 Skill。** Skill 只负责逻辑，规则内容留在 GitHub 仓库里是最干净的设计。

---

## 两种方式的对比

**❌ 规则放进 Skill（耦合）**
```
SKILL.md
  ├── 同步逻辑
  └── 规则内容（对话规范、编码规范...）← 每次改规则要改 Skill
```

**✅ 规则留在 GitHub，Skill 只做搬运（解耦）**
```
GitHub repo（source of truth）
  └── lulufoo/ai-software-dev/sys-prompt/dialogure/

.rules_sync.json（本地映射配置）
  └── 指向 GitHub 路径 + 各平台输出目录

SKILL.md（纯逻辑）
  └── 读 json → gh api 拉取 → 转换 frontmatter → 写入目标目录
```

---

## `.rules_sync.json` 引用 GitHub 的样子

```json
{
  "version": "1.0",
  "scenarios": {
    "conversation": {
      "source": "github:lulufoo/ai-software-dev@main/sys-prompt/dialogure/global-conversation.mdc",
      "scope": "always",
      "platforms": {
        "cursor":      { "output": ".cursor/rules/conversation.mdc" },
        "claude-code": { "output": "~/.claude/rules/conversation.md" },
        "vscode":      { "output": "~/.copilot/instructions/conversation.instructions.md" }
      }
    },
    "coding": {
      "source": "github:lulufoo/ai-software-dev@main/sys-prompt/coding/global-coding.mdc",
      "scope": "code",
      "glob": "**/*.{ts,tsx,py,go}",
      "platforms": {
        "cursor":      { "output": ".cursor/rules/coding.mdc" },
        "claude-code": { "output": "~/.claude/rules/coding.md" },
        "vscode":      { "output": "~/.copilot/instructions/coding.instructions.md" }
      }
    }
  }
}
```

**好处**：
- GitHub 里的规则随时更新，下次执行同步自动拉最新版
- Skill 文件永远不需要因为规则内容变化而修改
- 新增场景只需在 GitHub 加文件 + `.rules_sync.json` 加条目

---

要开始实现吗？先确认一个细节：你的 `coding` 相关规则，GitHub 仓库里目前有吗？我去检查一下现有的 `sys-prompt` 目录结构。

Turn:1～8

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 9）

我们先把Coding 规则整理一下，可以基于 ai-coding-discipline 整理为Java / Kotlin版本。  /Users/lulu/Code/lulu-workbench/raw/ai-software-dev/ai-coding-discipline-norms-dialogue/202605061157-ai-coding-discipline-norms-dialogue.md

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn9

The user wants to organize coding rules from a local file, creating Java/Kotlin versions based on it. Let me read the file first.先读原版 SKILL.md，再整理为 Java/Kotlin 版本。

Now I have the context. The user wants to take the `ai-coding-discipline` rules (6 rules for preventing common AI coding anti-patterns) and create Java/Kotlin versions.

The six rules are:
1. No Silent Fallbacks - 禁止用 `??` / 默认值掩盖错误
2. No Catch-All try/catch - 业务逻辑让错误向上传播，只在 API 边界捕获
3. Tests Must Fail When Code Breaks - 验证具体结果，而不只是判断「是否存在」


4. No Hardcoded Lookup Tables - 实现真实逻辑，不用硬编码表来凑测试用例
5. Red-Green Testing (TDD) - 先写失败的测试，再修复代码
6. Don't Remove Debug Logs During Fix - 修复期间保留日志，由人工确认修复后再移除

I need to fetch the original SKILL.md from GitHub to understand the full format and TypeScript examples, then adapt them for Java/Kotlin.Now I have the full original SKILL.md. I need to create Java/Kotlin versions of this. The key differences from TypeScript:

1. **Rule 1: No Silent Fallbacks**
   - Java: `Optional.orElse(null)`, `Optional.orElseGet(() -> null)`, ternary with null
   - Kotlin: `?: null`, `?.let { }` missing the null check, Elvis operator masking

2. **Rule 2: No Catch-All try/catch**
   - Java: `catch (Exception e)` swallowing errors, returning null
   - Kotlin: Same but with `catch (e: Exception)`

3. **Rule 3: Tests Must Fail When Code Breaks**
   - Java: JUnit 5 examples, `assertNotNull` as sole assertion
   - Kotlin: Kotest or JUnit 5 examples

4. **Rule 4: No Hardcoded Lookup Tables**
   - Same concept, Java/Kotlin syntax

5. **Rule 5: Red-Green Testing (TDD)**
   - Language agnostic, just show Java/Kotlin test tools (JUnit 5, Kotest)

6. **Rule 6: Debug Logs** - I need to continue with the remaining rules and show how they apply to Java/Kotlin, covering logging best practices, avoiding debug-only assertions, and ensuring tests remain deterministic across both languages.

For the file structure, I'm thinking a single markdown document makes sense since the user asked for Java/Kotlin versions together. I'll organize it by rule with side-by-side examples where the syntax differs. Since there's no existing `sys-prompt/coding/` directory in their repo, I should create this as a local file that can be added to their GitHub structure later. I'm thinking through where to save this Kotlin coding rules file—it needs to work across different platforms like Cursor, Claude, and VS Code, but I should start with a clean markdown version as the source of truth. I'll create it in a sensible location and let the user decide where to integrate it into their workflow. Users/lulu/Code/lulu-workbench/` as the base since that's where their files are.

Actually, let me create it as a clean draft. I'll put it in a `.cache` directory under the workspace, or since there's no open workspace, I'll use the user's home cache.

Wait, I should check if there's a local copy of the ai-software-dev repo first. The user has files at `/Users/lulu/Code/lulu-workbench/raw/ai-software-dev/`. Let me check if there's a `sys-prompt` directory in the local workspace.

Actually, I'll just create the file and tell the user where it is. The best location would be somewhere in their local workspace that maps to the GitHub repo structure.

Let me create it at: `/Users/lulu/Code/lulu-workbench/.cache/coding-jvm-discipline.md`, but I need to verify that directory exists first.Now I'll create the Java/Kotlin version of the ai-coding-discipline rules. I'll save it to `/Users/lulu/Code/lulu-workbench/.cache/coding-jvm-discipline.md`.

Let me write a comprehensive Java/Kotlin version of the 6 rules with appropriate examples in both languages.已创建：`/Users/lulu/Code/lulu-workbench/.cache/coding-jvm-discipline.md`

---

## 整理说明

原版 6 条规则全部保留，语言无关的原则未改动，做了三处 JVM 适配：

| 规则 | 原版（TypeScript） | Java/Kotlin 对应 |
|------|-------------------|-----------------|
| Rule 1 | `??` / `\|\|` | `Optional.orElse()` / Kotlin `?:` |
| Rule 2 | `async/await` + `catch` | Java checked exception + `catch (Exception e)` |
| Rule 3 | Jest `toBeDefined()` | JUnit `assertNotNull()` / Kotest `shouldNotBeNull()` |
| Rule 4 | TS 对象字面量 | `Map.of()` / Kotlin `mapOf()` |
| Rule 5 | 语言无关，保留 | 提到 JUnit 5 / Kotest |
| Rule 6 | 语言无关，保留 | 提到 `log.debug` / `println` |

---

**下一步确认两件事：**
1. 内容是否符合预期？有需要调整的细节吗？
2. 这个文件的最终落地位置：放到 `lulufoo/ai-software-dev` 的 `sys-prompt/coding/` 目录，还是另建一个位置？

Turn:1～9

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 10）

上述是语言层面的，还有一个coding的全局性约束，需要加一下： Karpathy 的 这个 https://github.com/forrestchang/andrej-karpathy-skills/blob/main/README.zh.md

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn10

先拿原文，再整合进文件。

The user wants to add Karpathy's coding principles (from forrestchang/andrej-karpathy-skills) as global coding constraints to the coding rules file. The content is already provided in the external links section.

The four principles are:
1. 编码前思考 (Think Before Coding) - No assumptions, surface confusion, present tradeoffs
2. 简洁优先 (Simplicity First) - Minimal code, no over-engineering
3. 精准修改 (Precise Changes) - Only touch what's necessary
4. 目标驱动执行 (Goal-Driven Execution) - Define success criteria, verify in loops


I need to fetch the exact wording from the CLAUDE.md file in that repository, then decide whether to add this as a new section to the existing coding discipline file or create a separate global constraints file. Since the user specified "global coding constraints," a separate file might be more appropriate than extending the Java-specific discipline document.结构上，这两层是分开的：

- `coding-global.md` → Karpathy 四原则，**所有语言通用**（`glob: **/*.{ts,py,java,kt,go,...}`）
- `coding-jvm.md` → ai-coding-discipline 六条，**仅限 Java/Kotlin**（`glob: **/*.{java,kt}`）

分两个文件，未来加语言也不需要动 global。

Now I have the Karpathy CLAUDE.md. The user wants to add these global coding constraints. I need to decide how to structure this:

Option 1: Add a new section to the existing `coding-jvm-discipline.md`
Option 2: Create a separate `coding-global.md` (language-agnostic global constraints)

Given the user said "coding的全局性约束，需要加一下" - they want to add it to the existing file as a global section. The JVM-specific rules cover "what to do in Java/Kotlin", while the Karpathy principles cover "how to think and work when coding in general".
 
Re-reading their intent more carefully, they're distinguishing between language-specific constraints and universal coding principles. I should create a separate `coding-global.md` file with the Karpathy principles, keeping the JVM discipline file focused on language-specific rules.

This aligns perfectly with their `.rules_sync.json` system—the global file applies across all projects and languages, while the JVM file targets only Java and Kotlin files. I'll write the global file in Chinese to match their documentation style.两个文件都在 `.cache/` 下了：

```
.cache/
  coding-global.md     ← Karpathy 四原则，所有语言
  coding-jvm-discipline.md  ← ai-coding-discipline 六条，Java/Kotlin
```

---

**对应到 `.rules_sync.json` 的 scenario 结构：**

```json
"scenarios": {
  "coding-global": {
    "source": "...",
    "scope": "code",
    "glob": "**/*.{ts,tsx,js,py,java,kt,go,rs,rb,swift}"
  },
  "coding-jvm": {
    "source": "...",
    "scope": "code",
    "glob": "**/*.{java,kt}"
  }
}
```

两层叠加：写 Java/Kotlin 时，AI 同时受 global 四原则 + JVM 六条约束。

---

内容确认后，下一步是把这两个文件推到你的 GitHub 仓库（`sys-prompt/coding/` 目录）。需要我现在做吗？还是你想先审阅一遍内容？

Turn:1～10

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 11）

你先推送上去，我再看下。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn11

The user wants me to push the two files to their GitHub repository. I need to:1. Check if there's a local clone of the `lulufoo/ai-software-dev` repo
2. If yes, add the files and push
3. If no, use `gh api` to create the files directly via the GitHub API

Let me check if there's a local clone first.The local repo doesn't have a `sys-prompt/` directory, but the GitHub repo does (I saw it earlier). The local repo might be out of sync or have a different structure than the remote. Let me check the remote status and then pull, or use `gh api` to push directly.

Actually, let me check the git status first and see if local is behind remote.The local repo doesn't have `sys-prompt/` directory but the GitHub remote does. This means the remote has content that the local doesn't have - perhaps those files were pushed directly via GitHub API or another machine. Let me pull first to sync, then add the new files.已推送。两个文件在：

- [`sys-prompt/coding/coding-global.md`](https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/coding/coding-global.md) — Karpathy 四原则
- [`sys-prompt/coding/coding-jvm-discipline.md`](https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/coding/coding-jvm-discipline.md) — Java/Kotlin 六条规则

你去看一下内容，有需要调整的直接说。

Turn:1～11

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 12）

https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/coding/coding-jvm-discipline.md  这个规则太过冗长了，给我一个和原始方案同样简介的方案。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn12

先对比原文行数，再精简。

The user wants a more concise version of the coding-jvm-discipline.md file, similar in brevity to the original ai-coding-discipline SKILL.md. Let me re-read both to understand what "concise" means here.

The original SKILL.md is about ~140 lines with TypeScript examples. My JVM version is much longer because I added both Java and Kotlin examples for each rule. The user wants it trimmed down to be similarly concise.

Key principle: Keep the rule statement + one pair of examples (either Java or Kotlin, or a compact dual example), remove verbose explanations.Let me create a leaner version. I'll keep:
- Rule name + one-line principle


- One BAD + one GOOD example (choose whichever is more illustrative - Kotlin is more concise)
- Brief checklist where it adds value
- Remove wordy prose

Actually, looking at the original SKILL.md more carefully, it's actually fairly detailed. The issue with my JVM version is I have BOTH Java AND Kotlin examples for every rule, which doubles the length. The solution is to pick ONE language per example or combine them.

I'll write a version that uses Kotlin as the primary example since it's more concise, adds Java notes only where syntax differs meaningfully, removes redundant prose, and keeps the quick reference format intact.

Generating the reference table...272 行 vs 原版约 140 行。问题是每条规则都放了 Java + Kotlin 双份示例。解法：以 Kotlin 为主，Java 仅在语法差异显著时补一行注释。

The original SKILL.md was about 140 lines in TypeScript. My JVM version is 272 lines - almost double. The main reason is I have both Java AND Kotlin examples for every rule.

The fix: Use Kotlin as primary (more concise), add Java only where they differ significantly. Remove verbose explanations - keep rule + bad/good pair + checklist if needed.

Target: ~130-150 lines.147 行，与原版 TypeScript 版本接近。

**精简策略**：
- 每条规则保留 Kotlin 为主示例，Java 差异处仅用单行注释注明
- 去掉重复的散文解释，规则描述用一句话
- 保留所有原版的逻辑结构和快速自查表

[查看更新后的文件](https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/coding/coding-jvm-discipline.md)

Turn:1～12

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 13）

在生成代码的时候，对话原则是否需要默认加上？ https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/dialogure/global-conversation-zh.md  vs  https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/coding/coding-global.md

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn13

先读两个文件内容再分析重叠。

The user is asking whether the conversation principles should be included by default when generating code. They're comparing:
1. `sys-prompt/dialogure/global-conversation-zh.md` - conversation principles
2. `sys-prompt/coding/coding-global.md` - coding global constraints (Karpathy)

Let me first fetch both files to understand their content, then give a clear answer.

Actually, I already know the content of `global-conversation.mdc` from earlier (it has things like "Turn1", "CHAT_ID", etc.). But the user references `global-conversation-zh.md` which might be different from `global-conversation.mdc`. Let me check.
 
Now I'm thinking through whether conversation principles should apply during coding tasks. There's a real tension here: conversation principles like turn numbering, evidence-first ordering, and authorization requirements are fundamentally about dialogue flow, while coding principles focus on code quality and structure. Some elements like turn numbering might carry over, but others like CHAT_ID are only relevant at chat initialization. I need to evaluate which principles actually add value in a coding context versus which ones would just create friction.

Looking closer, I notice significant conceptual overlap—"think before acting" maps to Karpathy's "think before coding," and "action requires authorization" aligns with making surgical changes. The real question is whether loading both frameworks simultaneously creates redundancy or useful reinforcement. Let me check the Chinese version to see if there's a different approach to this problem.✅ 两份文件都读到了，可以做精确的重叠分析。

---

## 重叠分析

| 对话原则 | Karpathy 编码约束 | 关系 |
|---------|-----------------|------|
| 行动前先思考（暴露假设、不默默选一个）| 原则 1：编码前先思考 | **高度重叠** |
| 行动需要授权（只做指令范围内的事）| 原则 3：精准修改 | **高度重叠** |
| 意图优先（最小信息范围）| 原则 2：简洁优先 | **部分重叠** |
| 先查证再结论（工具验证，✅/⚠️/❌）| — | **编码中有价值，无对应** |
| Turn 编号 / CHAT_ID / 模型描述 | — | **纯对话元信息，编码无关** |
| 文档规范（文件名格式、链接）| — | **文档场景，编码无关** |

---

## 结论

**不建议在编码场景默认加载完整对话原则。**

理由：
1. 核心思维约束（行动前先思考、精准修改）**已被 Karpathy 四原则覆盖**，重复加载只是浪费 token
2. Turn 编号、CHAT_ID、文档规范这些**对代码生成无任何帮助**，只增加噪声

**真正值得在编码场景保留的**，只有这一条：

> 「先查证再结论」——不确定 API / 行为时，必须工具验证，不得推断后直接写代码

这条在 Karpathy 里没有对应，可以作为单独一条合并进 `coding-global.md`。

---

## 建议操作

在 `coding-global.md` 末尾加一条原则 5：

```markdown
## 原则 5：不确定时先验证

**不推断 API / 行为 / 文件结构。先用工具查证，再写代码。**

- 不确定某个方法签名 → 先读源码或文档，再调用
- 不确定文件是否存在 → 先检查，再修改
- 不确定依赖版本 → 先查 pom.xml / build.gradle，再引用
```

需要我加进去并推送吗？

Turn:1～13

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 14）

可以，你给我加一个原则 5

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 15）

不过你说的 ## 原则 5：不确定时先验证 ，我觉得是否太狭隘了？ 例如：怎么约束：AI必须确认清楚外部API的真实含义？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn15

用户意图：把原则 5 从「操作层面的查证」升级为「认知层面的 API 真实含义确认」。

你说得对，现在的原则 5 描述的是操作行为（先读文件再改），但更深层的问题是：

**AI 对外部 API 的认知可能是错的——来自训练数据的幻觉，而非真实的当前行为。**

---

## 更宽的框架

这个问题的本质是 **LLM 的 API 幻觉**：

- 方法签名记错（参数顺序、类型、返回值）
- API 在某版本后已变更，AI 还在用旧的
- 方法存在于训练数据里某个相似库，但当前库没有
- 方法行为的边界条件与文档描述不符

这不是「是否查证」的问题，而是「**AI 的记忆不可作为 API 的信源**」。

---

## 重写建议

```markdown
## 原则 5：外部接口必须以真实来源为准

**AI 对 API 的记忆不可信。任何外部接口的使用，
必须以实际文档、源码或当前依赖为依据，不得依赖训练数据中的印象。**

适用范围：
- 第三方库方法（Spring、Hibernate、Jackson、Guava…）
- 框架行为（注解语义、生命周期、自动配置）
- 数据库 / 消息队列客户端 API
- 内部公共模块的接口

行动规则：
- 写调用前，先确认当前版本的实际签名（读 pom.xml / 源码 / Javadoc）
- 不确定某个行为 → 说出来，而不是凭印象写代码
- 发现 API 与预期不符 → 停下来重新确认，不要「猜一个试试」
- 不用「应该是这样」—— 要么确认，要么明确标注为待验证
```

---

这个版本的核心从「查文件」变成了「**AI 记忆不是信源，真实文档才是**」。

要我用这个版本替换进去并推送吗？

Turn:1～15

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 16）

原则 5 对应的是证据优先， 需要依然保持这个抽象粒度。

1 编写任何代码，都需要有证据支撑。 
2 编写任何API调用，必须查证其用法、边界。
3 引入任何依赖，必须有明确要求。

这个是全局的原则约束，不要太细化了。请你审查，有没有遗漏。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn16

用户意图：审查原则 5 的三条子规则是否完整，不遗漏，抽象层级一致。

---

三条覆盖的范围：

| 条目 | 覆盖域 |
|------|--------|
| 1. 编写任何代码，需有证据支撑 | 新增逻辑 |
| 2. 任何 API 调用，必须查证用法和边界 | 外部接口 |
| 3. 引入任何依赖，必须有明确要求 | 新增依赖 |

**审查结论：有一个对称缺口。**

「编写」和「引入」对应创建行为，但**删除**没有被显式覆盖：

> **删除任何代码或依赖，必须确认无调用方。**

这与原则 3（精准修改）不同——原则 3 约束的是「不要多改」，这里约束的是「删除前必须有证据证明安全」。删除一个被依赖的方法 / 移除一个还在使用的依赖，是同等严重的代码错误，需要同等级别的证据要求。

---

**建议修订为四条：**

```
1. 编写任何代码，必须有证据支撑
2. 调用任何 API，必须查证其用法与边界
3. 引入任何依赖，必须有明确要求
4. 删除任何代码或依赖，必须确认无调用方
```

其余无遗漏。你看是否需要加第 4 条？

Turn:1～16

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 17）

删除任何代码或依赖，必须确认无调用方。这个不是特别重要，因为有编译支持，不会无法发现。 真正需要避免的是 私自删除了代码和依赖。 

1. 新增、修改、删除任何代码，必须有证据支撑
2. 新增、修改任何 API调用，必须查证其用法与边界
3. 新增、修改、删除任何依赖，必须有明确要求

应该可以兼顾到你说的第四点了，请你审查遗漏，审查表述问题。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn17

**审查结论：逻辑完整，有一处表述不一致需要确认。**

---

**覆盖完整性：✅ 无遗漏**

三条分别覆盖了三个对象（代码 / API调用 / 依赖）的完整生命周期操作，第 4 点已被吸收进去。

---

**表述问题：第 1 条和第 3 条的约束强度不对称**

| 条目 | 约束词 | 含义 |
|------|--------|------|
| 1. 代码 | **必须有证据支撑** | 需要来源依据（需求、文档、问题描述均可） |
| 3. 依赖 | **必须有明确要求** | 只能来自用户的显式指令 |

第 3 条比第 1 条更严格——这是**有意为之的**（依赖变更影响范围更广），逻辑上正确，但需要确认是否符合你的意图。如果是有意区分，建议保持不动；如果是笔误，可以统一为「明确要求」。

---

**一个边界场景供参考**

「配置变更」（如 `application.yml`、Spring Bean 定义）严格来说不是代码，但影响与代码等同。第 1 条用「代码」作为表述是否足够宽？如果不是，可以改为「任何代码或配置变更」。同样，是否细化取决于你的意图。

---

其他无问题。两处均是**需要你确认意图**，不是我认定的错误。

Turn:1～17

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 18）

是的，改成 「任何代码或配置变更」 ，可以兼顾到代码 和 配置， 变更 兼顾到了 ：新增、修改、删除。请你优化

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn18已推送。原则 5 最终版：

```markdown
## 原则 5：证据优先

**没有证据，不做变更。**

1. 任何代码或配置变更，必须有证据支撑
2. 新增、修改任何 API 调用，必须查证其用法与边界
3. 任何依赖变更，必须有明确要求
```

「证据优先」与前四条原则保持了同一抽象层级，三条子规则分别覆盖变更对象的完整范围。

Turn:1～18

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 19）

这里 有证据支撑， 其实是说的有明确的需求， 但是证据支撑，可能会混淆意思。 同样的，第三点的 必须有明确要求 也是一样。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn19

用户意图：统一三条的约束词语，避免「证据支撑」含义模糊，对齐到「明确需求」这个核心语义。

你说得对。「证据支撑」暗示可以来自任何地方，而实际意图是：**必须有来自用户或需求的明确指示，才能做变更**。「明确需求」更准确。

建议统一为：

```markdown
## 原则 5：证据优先

**没有明确需求，不做变更。**

1. 任何代码或配置变更，必须有明确需求
2. 新增、修改任何 API 调用，必须查证其用法与边界
3. 任何依赖变更，必须有明确需求
```

第 2 条约束的是 API **用法的准确性**，与需求无关，保持「查证」不变。

这样改吗？

Turn:1～19

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 20）

你发散思考一下，证据优先还需要考虑哪些点？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn20

用户意图：发散思考「证据优先」原则的完整覆盖范围，看是否还有遗漏的维度。

---

当前三条覆盖的维度是：**变更的授权来源**（需求）和 **API 用法的准确性**。

发散来看，「证据优先」的完整语义应该包含以下几个独立维度：

---

**维度 1：变更授权** ← 已覆盖
> 任何代码、配置、依赖变更，必须有明确需求

**维度 2：接口准确性** ← 已覆盖
> 调用 API 前，必须查证用法与边界

**维度 3：假设显式化** ← 未覆盖
> AI 对业务逻辑、数据结构、系统行为做出的任何假设，必须明确说出来，不得隐式带入代码

这是「幻觉陷阱」最常见的发生地：AI 假设某个字段一定存在、某个状态只有两种、某个操作是幂等的——然后把假设直接编码进去，没有任何标注。

**维度 4：破坏性操作需二次确认** ← 未覆盖
> 删除、覆盖、清理等不可逆操作，必须在执行前明确告知并等待确认

这与「变更需要明确需求」有所不同——即使有需求，破坏性操作仍需要执行前确认，不能静默执行。

**维度 5：推断需标注** ← 未覆盖
> 当无法查证而必须推断时，必须显式标注为待验证，不得将推断当作事实写入代码

---

整理后的五个维度：

| 维度 | 核心问题 |
|------|---------|
| 变更授权 | 凭什么改？ |
| 接口准确性 | API 用法是否正确？ |
| 假设显式化 | 有没有把假设偷偷藏进代码？ |
| 破坏性操作确认 | 不可逆操作是否已告知？ |
| 推断标注 | 推断是否被当作事实？ |

---

维度 3（假设显式化）和维度 4（破坏性操作确认）是否值得加进去？你来判断——它们是否在你实际使用 AI 编码时真实发生过？

Turn:1～20

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 21）

第五条，推断需标注 是否应该直接停止呢？ 还是 标注之后，用户验证？ 你觉得呢？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn21

用户意图：讨论「推断需标注」的处理策略——停止还是标注后继续。

---

这是个**工作流设计选择**，取决于推断的影响范围。我的看法：

**直接停止的代价太高。**

代码中的推断无处不在——函数参数的业务含义、字段的合理范围、并发安全假设——如果每个推断都停下来等确认，AI 将几乎无法连续工作。

**但不加区分地「标注后继续」也有问题。**

如果推断影响了核心路径（数据模型、权限逻辑、状态机转换），带着错误假设写完整个实现，比停下来等确认的代价大得多。

---

**更合理的策略：按影响范围区分**

| 推断类型 | 策略 |
|---------|------|
| 影响核心路径（数据模型、权限、状态流转） | **停止，显式提问** |
| 影响局部实现（某个字段的默认值、日志格式） | **标注后继续，等用户 review** |

对应到原则表述：

```
无法查证时，必须标注推断。
若推断影响核心路径，停止并明确提问；
否则标注后继续，由用户在 review 时确认。
```

---

这样「停止」和「继续」都有依据，而不是一刀切。你觉得这个分层合适吗？还是你倾向于强制要求所有推断都停止？

Turn:1～21

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 22）

你说的不太具备可执行性，统一标注吧  

当无法查证而必须推断时，必须显式标注为待验证，不得将推断当作事实写入代码

 你这里说的，标注为待验证，标注在哪里？ “不得将推断当作事实写入代码” 意思是写入待验证，然后停止吗？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn22

用户意图：明确「推断标注」的操作语义——标注在哪里、执行后是否停止。

---

你问的是两个操作问题：

**1. 标注在哪里？**

不是标注在代码里（注释难以被关注），而是**标注在当前对话输出中**：AI 生成代码或方案时，在回复里显式写出：

> ⚠️ 以下推断待验证：`xxx 字段假设为非空` / `假设该接口幂等`

**2. 「不得将推断当作事实写入」是什么意思？**

有两种解读：
- A：可以写代码，但必须把推断标注出来，然后继续
- B：不写代码，先停下来等确认

你之前说「统一标注」，意味着选 A——**写出来，但标注清楚，不掩盖**。

---

**修订后的表述：**

```
当无法查证而必须推断时，在回复中显式列出所有推断，
标注为 ⚠️ 待验证，由用户确认后方可视为有效。
```

去掉「不得将推断当作事实写入代码」——它含义模糊。改成「列出推断 + 等用户确认」，动作清晰。

这样改吗？

Turn:1～22

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 23）

我觉得，还不如代码中，增加TODO 待验证的注释。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn23

这个方案更好，因为：**TODO 注释和代码在同一位置，不会被对话淹没**，也符合开发者的日常习惯，代码 review 时自然可见。

修订后的表述：

```
当无法查证而必须推断时，在推断所在位置添加 TODO 注释，
标注推断内容及待验证的具体问题，不得将推断静默写入代码。
```

示例：

```java
// TODO [待验证] 假设 userId 在此处一定非空，若上游可能传 null 需补充校验
String userId = order.getUserId();
```

这样改进到文件里推上去吗？

Turn:1～23

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 24）

证据优先，还有一个缺陷，只提了要求，没有给出反向指引。 例如： 任何代码或配置变更，必须有证据支撑。 没有证据怎么办？ 应该要求。只有推断这里给出了反向机制

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn24

用户意图：补全「证据优先」各条规则的反向指引——当条件不满足时，该怎么做。

好的观察。当前三条只说了「必须」，没说「不满足时怎么办」，只有推断那条给了行动指引（加 TODO）。

---

**三条的反向指引对称补全：**

| 条目 | 无法满足时 |
|------|---------|
| 1. 代码/配置变更无明确需求 | **停止，明确提问** |
| 2. API 用法无法查证 | **停止，明确说明无法确认，等用户提供** |
| 3. 依赖变更无明确要求 | **停止，不得自行引入** |
| 推断 | 继续，但加 TODO 标注 |

注意三条的反向指引都是**停止**，推断是唯一可以继续的例外——因为推断通常是局部的，而需求不明、API 不确定时继续写代码是系统性风险。

---

**修订后完整原则 5：**

```markdown
## 原则 5：证据优先

**没有明确需求，不做变更。**

1. 任何代码或配置变更，必须有明确需求；无明确需求时，停止并提问
2. 新增、修改任何 API 调用，必须查证其用法与边界；无法查证时，停止并说明
3. 任何依赖变更，必须有明确需求；无明确需求时，停止，不得自行引入
4. 当无法查证而必须推断时，在推断所在位置加 TODO 注释标注待验证内容
```

要我更新推送吗？

Turn:1～24

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 25）

这里有一个疑惑，证据优先是在AI执行过程中的约束。目的是禁止没有明确需求的变更引入。

那是否意味着，没有需求的情况，禁止变更即可？ 但是，这里还有一个考虑，当AI考虑引入，是否是因为有一些理解模糊的地方？ 

所以，对应的决策就变成了2个选择：不变更 or 把问题提出来。 你怎么思考这个问题的？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn25

用户意图：厘清「无明确需求时」的正确决策逻辑——是直接不做，还是先把问题提出来。

---

你描述了两种情形，它们的根因不同：

**情形 A：需求清晰，但 AI 想多做**
> 用户让我加一个登录接口，AI 顺手把密码加密逻辑也重构了

这里没有理解模糊，是 AI 主动越界。正确决策：**直接不做**，不需要提问。

**情形 B：AI 想变更，是因为理解有歧义**
> 用户让我实现「支付流程」，AI 不确定是否包含退款逻辑，于是把退款也写进去了

这里 AI 是因为不确定才做了额外的事。如果不做，可能做出残缺的实现；如果做了，可能引入不需要的代码。正确决策：**提出问题，等用户澄清**。

---

**两种情形的识别标准：**

> AI 想做这件事，是因为「感觉应该做」，还是因为「不做的话不知道该怎么继续」？

- 前者 → 直接不做
- 后者 → 提问

---

**对应到原则表述：**

```
无明确需求时，不做变更。
若不做变更导致实现有歧义，停止并明确提问，而非自行填补。
```

两句话分别对应两个情形，合并为一条反向指引。这样比单说「停止并提问」更精确——因为「直接不做」和「提问后等待」是两个不同的出口。

要按这个方向重写原则 5 的反向指引吗？

Turn:1～25

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 26）

需求清晰，本身是一个比较模糊的准则，不好判断，而且执行容易走偏。我觉得需要思考一个前提，AI依据什么来执行代码的？

我理解输入的应该是一个比较清楚的 施工执行单，这个施工单本身已经把应该怎么做，比较细化的步骤列出来的。不应该存在很大的模糊空间。

所以，基于这个前提，证据优先应该是硬性要求。

## 原则 5：证据优先

**没有明确需求，禁止引入变更。**

1. 任何代码或配置变更，必须有明确需求；无明确需求时，停止并提问。
2. 新增、修改任何 API 调用，必须查证其用法与边界；无法查证时，停止并说明。
3. 任何依赖变更，必须有明确需求；无明确需求时，停止，不得自行引入。
4. 当无法查证而必须推断时，停止并提问。

这个要求，就约束了输入施工单的质量，避免风险引入生成的代码中。一旦引入，暴露问题成本很高。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn26

用户意图：确认原则 5 的最终表述，并理解其背后的设计前提。

你的推导逻辑是自洽的：

**前提确立了边界**：AI 的输入是施工单，施工单本身应足够清晰。因此 AI 遇到模糊时，问题出在施工单质量上，而不是 AI 需要自己填补。这样「证据优先」就可以是硬性规则，不需要区分「主动越界」和「理解歧义」——两种情况的处置都是停止并提问。

**原则 4（推断）的处理也因此统一**：如果有施工单，理论上推断空间很小；若仍然遇到，停止提问比加 TODO 更符合硬性要求的精神。第 4 条修改是对的。

---

最终版本看起来是这样：

```markdown
## 原则 5：证据优先

**没有明确需求，禁止引入变更。**

1. 任何代码或配置变更，必须有明确需求；无明确需求时，停止并提问
2. 新增、修改任何 API 调用，必须查证其用法与边界；无法查证时，停止并说明
3. 任何依赖变更，必须有明确需求；无明确需求时，停止，不得自行引入
4. 当无法查证而必须推断时，停止并提问
```

无遗漏，表述清晰，四条反向指引对称完整。

要我更新文件推送吗？

Turn:1～26

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 27）

**没有明确需求，禁止引入变更。* 这一句话，表述还是有一点模糊。 可以说清楚一点吗？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn27

用户意图：把「没有明确需求，禁止引入变更」改为表述更清晰的一句话。

---

模糊点在于：「需求」和「变更」都没有指向明确的主体。

**候选方案：**

**A** 「施工单之外的任何变更，一律禁止。」
→ 清楚，但依赖「施工单」这个概念，需要读者已理解前提。

**B** 「只执行被明确要求的操作，其余一律不做。」
→ 正向表述，清晰，无歧义。

**C** 「未在需求中出现的代码、配置、依赖，禁止引入。」
→ 把「变更」拆成具体对象，更精确，但稍显冗长。

---

我倾向 **B**，它直接说清楚了 AI 的行动边界，不需要解释什么是「施工单」，也不需要枚举变更类型：

```markdown
## 原则 5：证据优先

**只执行被明确要求的操作，其余一律不做。**
```

你觉得哪个更符合你的意图？

Turn:1～27

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 28）

对于：1. 任何代码或配置变更，必须有明确需求；无明确需求时，停止并提问。 

提出另外一个问题：会不会导致AI一直在提问？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn28

用户意图：评估「无明确需求时停止并提问」是否会导致 AI 过度提问，影响执行效率。

---

**这个担忧是合理的，但前提假设解决了这个问题。**

如果施工单足够清晰（你的前提），AI 遇到「无明确需求」的情况应该很少——不会频繁触发提问。提问频繁，意味着施工单质量不达标，问题出在输入端，而不是原则本身过严。

所以这个原则在设计上形成了一个**反压机制**：

> AI 频繁提问 → 说明施工单写得不够细 → 倒逼施工单质量提升

---

**但有一个真实风险：**

AI 可能把「无法确定」的情况误判为「无明确需求」，从而产生不必要的提问。比如一个技术实现细节（用 HashMap 还是 LinkedHashMap），需求里不会写，但 AI 不应该停下来问。

这类「实现层面的合理判断」应该属于 AI 的执行权限范围，不受证据优先约束。

---

**因此，约束的边界需要明确：**

「证据优先」约束的是**需求范围**（做什么、改什么），不约束**技术实现方式**（怎么做）。

```markdown
**只执行被明确要求的操作，其余一律不做。**

（约束变更范围，不约束实现方式的技术选择。）
```

加这一行说明，可以有效避免 AI 在技术细节上过度提问。需要加进去吗？

Turn:1～28

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 29）

这一行，引入可其他风险，先不加。 你将刚刚讨论的最终方案发给我审查。

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn29

```markdown
## 原则 5：证据优先

**只执行被明确要求的操作，其余一律不做。**

1. 任何代码或配置变更，必须有明确需求；无明确需求时，停止并提问
2. 新增、修改任何 API 调用，必须查证其用法与边界；无法查证时，停止并说明
3. 任何依赖变更，必须有明确需求；无明确需求时，停止，不得自行引入
4. 当无法查证而必须推断时，停止并提问
```

确认后我更新文件推送。

Turn:1～29

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 30）

是的

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn30已推送。[查看最终文件](https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/coding/coding-global.md)

Turn:1～30

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 31）

证据优先 是否应该做成 硬门控？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn31

用户意图：评估「证据优先」是否应从行为准则升级为硬性门控机制。

---

**先区分两个层级：**

**软约束（现在的状态）**：原则写在规则文件里，AI 读到后尽量遵守，但没有结构性保障——在任务压力下 AI 仍可能跳过。

**硬门控**：在执行之前，AI 必须完成一个确认步骤，否则无法进入实现阶段。门控是结构性的，不依赖 AI 的自觉。

---

**我的判断：应该做成硬门控，但不是在规则层实现，而是在工作流层实现。**

规则文件本质上是「告诉 AI 应该怎么做」，无法强制拦截。真正的硬门控需要放在**施工单的格式约束**里：

```
施工单必须包含：
- 变更清单（每一项变更的明确来源）
- 需要引入/修改的 API 列表（已查证）
- 依赖变更列表（有明确要求）

AI 收到施工单后，先逐项确认上述内容完整，再开始执行。
```

这样「证据优先」就从 AI 的行为准则变成了**输入格式的验收标准**——没有完整的证据清单，施工单本身就不合格，AI 不应开始执行。

---

**结论：**

| 层级 | 方式 | 强度 |
|------|------|------|
| 规则文件（现在） | 告知 AI 应停止 | 软，可被跳过 |
| 施工单格式约束 | 输入不完整则拒绝执行 | 硬，结构性保障 |

「证据优先」值得做成硬门控，但实现位置在施工单设计上，而不是在编码规则里。这涉及施工单的格式规范，是否要往这个方向推进？

Turn:1～31

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 32）

但是，施工单也是可能遗漏的，证据优先本身就是发现问题，有问题遗漏到了代码执行阶段是正常的。所以，代码阶段的 「证据优先」，在于如何做到发现问题，并提出问题，同时退出。这个动作本身要做成强制执行

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn32

这个理解很关键。「证据优先」在代码执行阶段的本质是：

> **施工单的最后一道安全网——发现遗漏，强制退出。**

不是预防，而是检测+中止。

---

**强制执行的关键是：把它写成触发-动作模式，而不是原则。**

原则写法（弱）：
> 无明确需求时，停止并提问

触发-动作写法（强）：
> 当执行过程中遇到以下任一情况，**立即停止当前任务，禁止继续生成代码**，列出问题并等待指令：
> - 发现需要变更但施工单未提及的代码或配置
> - 需要调用的 API 无法确认用法或边界
> - 需要引入未被明确要求的依赖
> - 遇到必须推断才能继续的决策点

**「禁止继续生成代码」** 是强制退出的关键词——不是「建议停止」，而是硬性中止条件。

---

**修订后的原则 5：**

```markdown
## 原则 5：证据优先

**只执行被明确要求的操作，其余一律不做。**

执行过程中，遇到以下任一情况，立即停止并列出问题，禁止继续生成代码：

1. 需要变更但施工单未明确要求的代码或配置
2. 需要调用但用法或边界无法查证的 API
3. 需要引入但未被明确要求的依赖
4. 需要推断才能继续的决策点
```

四条从「必须有…」改成了「遇到…立即停止」，语义从约束变成了触发条件。要我更新推送吗？

Turn:1～32

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 33）

1 同意
2 另外几条约束，都是语义约束。和第五条不是统一的约束。 你这么看这个不一致？

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn33

用户意图：识别原则 1-4（语义约束）与原则 5（触发-动作）之间的结构性不一致，判断是否需要统一。

---

**不一致是真实存在的，但我认为不需要统一——这是有意义的分层。**

| 原则 | 类型 | 作用时机 |
|------|------|---------|
| 1-4 | 语义约束（告知 AI 应该怎么做） | 编码过程中持续生效 |
| 5 | 触发-动作（遇到条件立即中止） | 编码过程中检测异常 |

这两种约束解决的是不同问题：

- **原则 1-4** 约束的是 AI 的**主动行为**：别过度复杂、别乱改、别假设、先验证思路
- **原则 5** 约束的是 AI 的**退出条件**：发现边界问题时必须停下来

如果把原则 5 改成和 1-4 同样的语义约束写法，它就变成了「建议」而不是「门控」，失去了强制退出的效果——这是你刚才明确要求的。

**所以这个不一致是值得保留的结构差异，不是问题。**

如果要显式化这个分层，可以在文件里加一行区分：

```markdown
> 原则 1–4 约束编码行为；原则 5 是执行中的强制退出条件。
```

这样读者能理解为什么第五条写法不同。要加这行说明吗？

Turn:1～33

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 34）

同意

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn34

已推送。[查看更新](https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/coding/coding-global.md)

Turn:1～34

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 35）

执行：dtd_normalize，主题： AI Coding — 编码提示词约束

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn35

用户意图：执行 `dtd_normalize`（P0 规范化），主题「AI Coding — 编码提示词约束」，将本会话归档至 lulu-workbench。

> ✅ config.json 读取完成 · archive_root: `/Users/lulu/Code/lulu-workbench`

topic-path 选自 topics.json：`lulufoo/ai-software-dev` → project `ai-software-dev`，doc-theme `ai-coding-prompt-constraints`（3–5 词 kebab-case）。

> ✅ dtd_normalize 完成
> 📄 raw：`raw/ai-software-dev/ai-coding-prompt-constraints/202605071824-ai-coding-prompt-constraints.md`
> 🗂 index.json 已更新（layers 新增 raw · id `c2c3cd66531ea258715c0e832c1ed73f`）

Turn:1～35

<!-- DDM:TURN_SEP:v1 -->
