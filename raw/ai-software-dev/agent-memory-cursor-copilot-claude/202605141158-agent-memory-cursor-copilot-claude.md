# Agent 记忆系统：Cursor / Copilot / Claude Code 调研分析

> 创建时间：2026年5月14日 11:58

> 导航：[distilled](../../../distilled/ai-software-dev/agent-memory-cursor-copilot-claude/202605141158-agent-memory-cursor-copilot-claude.md) · [digest](../../../digest/ai-software-dev/agent-memory-cursor-copilot-claude/202605141158-agent-memory-cursor-copilot-claude.md) · [trace](../../../trace/ai-software-dev/agent-memory-cursor-copilot-claude/202605141158-agent-memory-cursor-copilot-claude.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

<timestamp>Wednesday, May 13, 2026, 11:42 PM (UTC+8)</timestamp>
<user_query>
/memories/ 目录内容：

/memories/skill-file-reading.md（本次新建）
/memories/session/（空）
/memories/repo/（空）

上述是VS Copilot的 memories 设计，请调查分析一下，分别是什么含义？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn1

CHAT_ID-a7f3e92b1c4d58f6e0b2a9d3c7f1e845

用户意图：调查分析 VS Copilot 的 `/memories/` 目录设计，弄清三个路径各自的含义。

The user is asking about the VS Copilot memories directory structure. Let me think about what skills might apply here before responding.

The user wants me to investigate and analyze the VS Copilot memories directory structure. This seems like a research/analysis task. Let me check if any skills apply - this is an informational/research question about VS Copilot's design. Let me check if there's a relevant skill.

Looking at the available skills:
- repo-analyzer: for analyzing projects/repos
- brainstorming: for creative work
- canvas: for analytical artifacts

This seems like a factual research question. The user wants to understand what `/memories/skill-file-reading.md`, `/memories/session/`, and `/memories/repo/` mean in VS Copilot's design.


I need to search for information about VS Copilot's memories feature since I may not have current details about this architecture. Let me look for documentation or relevant files that explain how these directories are structured and what they're used for.

现在有了官方文档的一手资料，可以给出准确分析。

---

## VS Copilot `/memories/` 目录设计分析

✅ 已验证（出处：[VS Code 官方文档 Memory in VS Code agents](https://code.visualstudio.com/docs/copilot/agents/memory)，抓取于 2026-05-13）

### 整体设计：三作用域本地记忆系统

Memory Tool 是 VS Code Copilot 的内置 Agent 工具，**所有数据存储在本机**，用于跨会话保留上下文。三个目录对应三种不同的持久化作用域：

| 路径 | 作用域 | 跨 session 持久 | 跨 workspace 持久 | 用途定位 |
|------|--------|----------------|-------------------|---------|
| `/memories/` | **User（用户级）** | ✅ | ✅ | 个人偏好、通用习惯、常用命令 |
| `/memories/repo/` | **Repository（仓库级）** | ✅ | ❌（仅限当前 workspace） | 项目架构约定、命名规范、构建命令 |
| `/memories/session/` | **Session（会话级）** | ❌（chat 结束即清除） | ❌ | 临时任务上下文、进行中的计划 |

---

### 三个路径的具体含义

**`/memories/`（User 级）**

- 所有 workspace、所有 session 均生效
- 每次 session 启动时，**前 200 行自动注入** Agent 上下文
- 适合存：`我偏好 tabs 而非 spaces`、`JS 用单引号` 等全局性偏好

**`/memories/repo/`（Repository 级）**

- 仅在当前 workspace 内跨 session 持久
- 适合存：`该项目使用 Repository Pattern`、`所有 API 需要鉴权` 等项目特定事实
- 你的 `/memories/repo/` 目前为空，说明当前项目尚无 Agent 积累的项目记忆

**`/memories/session/`（Session 级）**

- 仅限当前对话，关闭 chat 即清除
- Plan Agent 会把实施计划写入此目录下的 `plan.md`
- 你的 `/memories/session/` 目前为空，符合预期（新 session 开始）

---

### `/memories/skill-file-reading.md`（本次新建）

这是一个 **User 级记忆文件**，路径在 `/memories/` 根目录下，属于用户作用域。

文件名 `skill-file-reading.md` 表明：Agent 在本次 session 中读取了某个 Skill 文件后，将"如何读取/使用 Skill 文件"这条洞察写入了 User 级记忆，以便后续所有 session 复用这个认知——这正是 Memory Tool 的典型行为模式：**Agent 自主判断值得记住的信息，选择合适的作用域写入**。

---

### 补充：Memory Tool vs Copilot Memory

这两个是**不同的系统**，容易混淆：

| | Memory Tool（本地） | Copilot Memory（GitHub 托管） |
|--|--|--|
| 存储位置 | 本机 `/memories/` | GitHub 服务端 |
| 作用域 | User / Repo / Session | 仅 Repository |
| 跨 Copilot 产品共享 | 否（仅 VS Code） | 是（Cloud Agent、Code Review、CLI） |
| 创建方式 | 用户/Agent 在 chat 中主动写入 | Copilot Agent 自动捕获 |
| 过期机制 | 手动管理 | 自动 28 天过期 |

你描述的 `/memories/` 目录结构属于前者（本地 Memory Tool）。

Turn:1～1

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have comprehensive information from the official VS Code documentation. Let me also look into what `/memories/skill-file-reading.md` specifically means - the user mentioned it was newly created in this session. This is likely related to the memory tool reading a skill file, and it's storing a user-level memory about skill file reading.

Let me now compose a comprehensive analysis based on the official documentation.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

<timestamp>Thursday, May 14, 2026, 12:50 AM (UTC+8)</timestamp>
<user_query>
1 /memories/（User 级） 如果有多个文件，是每个文件的前200行自动注入吗？ 
2 /memories/（User 级） 是AI自己添加的吗？ 我遇到一个AI读取SKILL只读了100行的问题，AI自己生成了一个skill-file-reading.md放到这个目录下。
3 那  /memories/（User 级） 用户自己可以添加吗 ？ 需要通过 Memory Tool 触发吗？ 

</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn2

用户意图：针对 `/memories/`（User 级）的三个具体机制问题求证。

先查证官方文档和 issue，不推断。

The user has three specific questions about VS Copilot's `/memories/` (User level) directory. Let me check the official documentation more carefully, as I need to verify these specific details. Let me re-read the official docs or search for more specific information.

From what I already fetched from the VS Code docs:
1. "The first 200 lines are automatically loaded into the agent's context at the start of every session." - This says the first 200 lines, but doesn't clarify if it's per file or total. I need to verify this.
2. Yes, AI can add memories autonomously
3. Need to check if users can manually add files


I should search for more detailed documentation or examples that specifically address these three questions.

搜索结果对多文件200行的细节未明确，直接读源码确认。

The search results didn't give me definitive answers about the multi-file 200-line behavior. Let me read the actual source code of memoryTool.tsx which was saved to a file. That should give me the definitive answer about how the 200-line limit works with multiple files.

The 200-line loading behavior isn't in the `memoryTool.tsx` file - it's likely in a separate file that handles how memories are injected into the system prompt. Let me search for this in the broader VS Code Copilot chat codebase, or look at the GitHub issue/PR that introduced this.

Let me also look for the actual initialization/system prompt related code.

源码已经给出了确定性答案，直接回答三个问题。

---

## 三个问题的确认答案

✅ 已验证（出处：[microsoft/vscode-copilot-chat `memoryContextPrompt.tsx`](https://github.com/microsoft/vscode-copilot-chat/blob/main/src/extension/tools/node/memoryContextPrompt.tsx)，`gh api` 直接读取）

---

### Q1：多文件时，200行是每文件还是总计？

**是所有文件合计不超过200行。**

源码关键逻辑（`getUserMemoryContent()`）：

```typescript
const MAX_USER_MEMORY_LINES = 200;

for (const [name] of fileEntries) {
    if (lines.length >= MAX_USER_MEMORY_LINES) {
        break;   // 总行数达到200就停止，后续文件不再读取
    }
    lines.push(`## ${name}`, ...text.split('\n'));  // 文件名作为标题 + 内容
}
return lines.slice(0, MAX_USER_MEMORY_LINES).join('\n');  // 最终截断到200行
```

所以：

- 文件按目录顺序遍历（文件系统顺序，通常字母序）
- 每个文件内容前会加一行 `## 文件名` 作为标题（占用行数）
- 累计行数达到 200 立即停止，后续文件**完全忽略**
- 多文件时靠后的文件有可能完全读不到

---

### Q2：`/memories/` 是 AI 自己添加的吗？

**是的，AI 可以自主写入**，这是 Memory Tool 的设计意图之一。

源码中 `MemoryInstructionsPrompt` 的系统指令明确告诉 AI：

> "As you work, consult your memory files to build on previous experience. When you encounter a mistake that seems like it could be common, check your memory for relevant notes — and if nothing is written yet, **record what you learned.**"

指导原则还包括：

> "Organize by topic in **separate files** (e.g., `debugging.md`, `patterns.md`)"

所以你遇到的情况完全符合设计：AI 在读 SKILL 文件时只读了 100 行，发现这是一个值得记忆的教训，于是自主调用 Memory Tool，创建了 `/memories/skill-file-reading.md` 写入这条注意事项。

---

### Q3：用户自己能手动添加吗？需要 Memory Tool 触发吗？

**两种方式都可以：**

| 方式 | 是否支持 | 说明 |
|------|---------|------|
| 自然语言让 AI 写 | ✅ | `"记住我的偏好是..."` → AI 调用 Memory Tool 写入 |
| 用户手动编辑文件 | ✅ | 直接编辑文件系统上的文件即可，`getUserMemoryContent()` 只是读目录 |
| VS Code UI 直接创建 | ❌ | 目前不支持（仅有 `Chat: Show Memory Files` 查看，无创建 UI） |

文件实际存储在 VS Code 的 `globalStorageUri` 下（通常是 `~/Library/Application Support/Code/User/globalStorage/github.copilot-chat/memory-tool/memories/`），找到路径后可直接新建 `.md` 文件，下次 session 启动时会自动注入。

> ⚠️ 注意：`/memories/session/` 和 `/memories/repo/` **不会**自动注入内容到上下文，只列出文件名——AI 需要主动调用 Memory Tool 读取。只有 `/memories/`（User 级）的内容是自动注入的。

Turn:1～2

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have the definitive source code answer. Let me analyze the key findings:

1. **Multi-file 200-line behavior**: From the `getUserMemoryContent()` method:
```javascript
const MAX_USER_MEMORY_LINES = 200;
// ...
for (const [name] of fileEntries) {
    if (lines.length >= MAX_USER_MEMORY_LINES) {
        break;  // stops adding files once 200 lines total reached
    }
    const fileUri = URI.joinPath(memoryDirUri, name);
    const text = new TextDecoder().decode(content);


```

Still writing parser...

So **200 lines is a TOTAL limit across all files**, not per file. Files are processed in directory order, and once 200 lines total are accumulated, processing stops. The final result is also sliced to 200. Each file gets a `## filename` header prepended, so the actual content lines per file would be slightly less than 200.

2. **Who creates memories**: The AI can autonomously write to `/memories/` using the Memory Tool, but users can also manually create or edit files in this directory since it's just a local filesystem path. The Memory Tool is essentially the agent's API for reading and writing these files.

3. **`/memories/skill-file-reading.md` created by AI**: Yes, the AI autonomously created this file when working and wanting to remember something about reading skill files. This is the Memory Tool writing on its own.

4. **Can users manually add files?**: Looking at the code, `getUserMemoryContent()` just reads from the filesystem directory with no special locking or validation, so users can manually add files to the directory if they know the actual path in `globalStorageUri`. The VS Code docs mention a "Manage memory files" option under Chat.

Show Memory Files that opens a list of all memory files across scopes, so users can view them, but the UI doesn't provide a way to manually create them—you'd need to navigate to the actual file path or ask the agent to create them via natural language.

For session and repo memory, the docs and source code show that both are listed in context but not auto-loaded—you need to use the Memory Tool to read them. Only user memory is auto-loaded, and even then just the first 200 lines total.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

<timestamp>Thursday, May 14, 2026, 1:04 AM (UTC+8)</timestamp>
<user_query>
这和写入.github目录的指令，有什么区别？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn3

用户意图：对比 Memory Tool（写入 `/memories/`）与写入 `.github` 目录的 Copilot 指令，理解两者的区别。

The user is asking about the difference between writing to the `/memories/` directory (Memory Tool) and writing instructions to the `.github` directory (which likely refers to `.github/copilot-instructions.md` or similar custom instructions files). Let me think about what I know about this and whether I need to look it up.

The `.github` directory context likely refers to:
1. `.github/copilot-instructions.md` - Custom instructions for GitHub Copilot
2. Possibly `.github/skills/` for agent skills

Let me search for the official documentation on this to verify before answering.

---

## Memory Tool vs `.github/` 指令文件的本质区别

✅ 已验证（出处：[VS Code Customization 官方文档](https://code.visualstudio.com/docs/copilot/copilot-customization)，[Custom Instructions 文档](https://code.visualstudio.com/docs/copilot/customization/custom-instructions)）

---

### 一句话核心区别

| | `.github/` 指令文件 | `/memories/` Memory Tool |
|--|--|--|
| **作者** | 人类手动编写 | AI 自主写入（或用户用自然语言触发） |
| **性质** | 声明式规则（静态） | 经验性笔记（动态） |
| **存储位置** | 项目仓库内（可 git 提交） | 本机本地（不进仓库） |
| **共享范围** | 团队所有成员 | 仅当前机器的当前用户 |
| **注入方式** | 每次请求全量注入 | User 级前200行自动注入；其余仅列出文件名 |

---

### `.github/` 目录能放什么

```
.github/
├── copilot-instructions.md      # 全局始终生效的编码规范
├── instructions/
│   └── react.instructions.md   # 特定文件类型 / 目录的专项规则
├── prompts/
│   └── review.prompt.md        # 可复用的提示词模板
├── agents/
│   └── reviewer.agent.md       # 自定义 Agent（专属角色+工具配置）
└── skills/
    └── my-skill/                # Agent Skill（脚本+指令打包）
```

这些都是**人类维护的项目规范**，版本控制、可 PR 审查、团队共享。

---

### 两者定位的本质差异

**`.github/copilot-instructions.md`** → 相当于团队的"代码规范文档"  
> "所有人都要遵守，所有场景都要应用，人工维护，不会变化"

**`/memories/`** → 相当于 AI 的"工作笔记本"  
> "AI 在干活过程中自己记下来的教训和偏好，只属于这台机器上的这个用户"

---

### 你遇到的 `skill-file-reading.md` 场景还原

```
AI 读取 SKILL 文件时只读了100行
    ↓
AI 判断：这是一个我以后可能重犯的错误
    ↓
AI 调用 Memory Tool，写入 /memories/skill-file-reading.md
    ↓
下次 session 启动时，这条笔记自动出现在上下文前200行里
    ↓
AI 在下次读 SKILL 时就会"想起来"要完整读取
```

如果你想让这条规则**对所有使用该仓库的人生效**，应该写进 `.github/copilot-instructions.md`；  
如果只是 AI 给自己记的私人备忘，就是 `/memories/` 的用途。

---

### 已知冲突（⚠️ 推断+已有 issue 佐证）

[Issue #308669](https://github.com/microsoft/vscode/issues/308669) 记录了一个问题：Memory Tool 的系统指令是硬编码在 system prompt 中的，`.github/copilot-instructions.md` **无法覆盖**它，导致用户试图让 AI 把知识写到项目文档而非内部 memory 时，AI 仍然优先写 memory，造成"知识漂移"和重复文档。

Turn:1～3

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have enough information to give a comprehensive comparison. Let me organize the key differences:

1. **`.github/copilot-instructions.md`** (and related `.github/` customization files):
   - Static, manually-written files
   - Stored in the project repo (version-controlled, team-shareable)
   - Always injected into every chat request for that workspace
   - Human-authored, human-maintained
   - For project-wide coding standards, conventions, architecture rules
   - Goes into `.github/` directory which is part of the repo

2. **Memory Tool (`/memories/`):**
   - Dynamic, AI-written (or user-written via natural language)
   - Stored locally on the machine, not version-controlled or shared with the team
   - User-level memories auto-inject their first 200 lines, while repo and session-level memories require explicit reading
   - Designed for personal preferences, lessons learned during work, and task-specific context

The core difference is that `.github/` files represent declarative, static team rules that humans establish upfront, whereas memories are experiential and dynamic—they capture what I learn as I work. There's also a documented conflict in issue #308669 related to memory tool instructions that's worth investigating.

The `.github/` directory itself can house various instruction types: the main `copilot-instructions.md` file, organized instruction files, prompt templates, custom agents, and agent skills.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 4）

<timestamp>Thursday, May 14, 2026, 8:21 AM (UTC+8)</timestamp>
<user_query>
/memories/session/  和 /memories/repo/ 是什么写入逻辑？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn4

用户意图：了解 `/memories/session/` 和 `/memories/repo/` 的写入触发逻辑。

已有 `memoryTool.tsx` 源码，再补充读取 `agentMemoryService.ts`。

The user wants to understand the write logic for `/memories/session/` and `/memories/repo/`. I already have the source code from `memoryContextPrompt.tsx` and `memoryTool.tsx`. Let me also fetch the `agentMemoryService.ts` and other relevant files to get the full picture of how these are written.

From what I already know from the source code:

1. For session memory: it's written when the AI calls Memory Tool with a path under `/memories/session/`. The Plan agent uses it for `plan.md`.

2. For repo memory: there are two modes:
   - Local repo memory (when Copilot Memory/CAPI is disabled): written locally to workspace storage
   - CAPI-backed repo memory (when Copilot Memory is enabled): written to GitHub servers

Now I need to examine the actual write implementation in `memoryTool.tsx` and check `agentMemoryService.ts` to understand the full flow.

源码已完整，直接分析。

---

## `/memories/session/` 和 `/memories/repo/` 写入逻辑

✅ 已验证（出处：`memoryTool.tsx` + `agentMemoryService.ts`，`gh api` 直接读取）

---

### 路由判断：三者怎么区分

`_dispatch()` 方法是唯一入口，根据路径前缀分流：

```typescript
// 1. /memories/repo/* → CAPI（云端）或本地 repo 存储
if (isRepoPath(path)) {
    if (capiEnabled) return _dispatchRepoCAPI(params);   // Copilot Memory 开启
    return _dispatchLocal(params, 'repo');               // fallback 本地
}

// 2. /memories/session/* → 本地 session 存储
// 3. /memories/* → 本地 user 存储
const scope = isSessionPath(path) ? 'session' : 'user';
return _dispatchLocal(params, scope, sessionResource);
```

---

### `/memories/session/` 的写入逻辑

**存储位置：**

```
storageUri（workspace 本地）
  └── memory-tool/memories/
        └── {sessionId}/       ← 从 vscode-chat-session://local/<sessionId> 提取
              └── plan.md      ← Plan Agent 写入
              └── 其他临时文件
```

**写入触发：**

- AI 主动调用 Memory Tool，路径带 `/memories/session/` 前缀时写入
- **Plan Agent 默认写入 `plan.md`**，记录当前实施计划，全程维护
- 支持完整 CRUD（create / str_replace / insert / delete / rename）

**注入上下文的方式：**

不自动加载文件内容，只列出文件名（让 AI 知道文件存在）：

> "The following files exist in your session memory. Use the memory tool to read them if needed."

**清除时机：** chat 关闭时 sessionId 失效，文件不再可见（数据留在磁盘但不会被下次 session 读到）

---

### `/memories/repo/` 的写入逻辑（两套模式）

#### 模式 A：Copilot Memory 关闭（默认）→ 本地文件

```
storageUri（workspace 本地）
  └── memory-tool/memories/
        └── repo/
              └── conventions.md
              └── 其他项目规范文件
```

- 支持完整 CRUD
- 仅在当前 workspace 可见
- 注入上下文方式同 session：只列文件名，AI 需主动读取

#### 模式 B：Copilot Memory 开启 → GitHub 云端 CAPI

写入时调用 `storeRepoMemory()`，走 **GitHub CAPI REST API**（PUT 请求）：

```typescript
// 写入格式：JSON 结构体，不是普通 Markdown
{
    subject: "命名规范",          // 必填：记忆标题
    fact: "所有 API 需要鉴权",    // 必填：具体事实
    citations: ["src/api/auth.ts:12"],  // 引用代码位置（用于验证）
    reason: "从代码审查中学到",    // 写入原因
    category: "conventions",     // 分类
    source: { agent: "vscode" }  // 写入来源
}
```

关键约束：

| 约束 | 说明 |
|------|------|
| **仅支持 `create`** | 不能 edit / delete / rename，源码明确报错 |
| **必须是 JSON** | 不是 Markdown，是结构化记忆条目 |
| **28天自动过期** | GitHub 服务端自动清理 |
| **读取前先验证** | AI 使用前会对 `citations` 引用的代码位置做实时校验 |
| **跨 Agent 共享** | Code Review、Cloud Agent、CLI 均可读取 |

---

### 三者对比总结

| | `/memories/`（User） | `/memories/session/` | `/memories/repo/` |
|--|--|--|--|
| 存储 | 本机全局 | 本机 workspace + sessionId | 本机 workspace（或 GitHub 云端） |
| 上下文注入 | **全文自动注入（前200行）** | 仅列文件名 | 仅列文件名（或云端直接注入摘要） |
| 支持操作 | 全 CRUD | 全 CRUD | 全 CRUD（本地）/ 仅 create（云端） |
| 格式 | Markdown | Markdown | Markdown（本地）/ JSON（云端） |
| 谁来写 | AI 自主或用户委托 | AI 自主（Plan Agent 必写） | AI 发现项目规律时写入 |
| 生命周期 | 永久（手动管理） | Chat 结束不可见 | 永久（本地）/ 28天（云端） |

Turn:1～4

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have a very clear picture from the source code. Let me analyze the write logic for both `/memories/session/` and `/memories/repo/`:

**`/memories/session/` write logic:**
- Routed by path: if path starts with `/memories/session`, scope = 'session'
- Stored in `storageUri` (workspace-local storage), under `MEMORY_BASE_DIR/{sessionId}/` where sessionId is extracted from the chat session resource URI
- Supports all operations: create, str_replace, insert, delete, rename, view
- Cleared when chat ends (because the session-scoped storage is tied to the session ID)


- The plan agent writes `plan.md` here during planning

**`/memories/repo/` write logic:**
For repo-scoped memories, the behavior splits based on whether Copilot Memory is enabled. If it's on, the system routes through CAPI to store data in GitHub's cloud, but only allows create operations. When disabled, it falls back to local workspace storage with full operation support.

**CAPI repo memory specifics:**
The data must conform to a JSON schema with `subject`, `fact`, `citations`, `reason`, and `category` fields. Since only creates are supported, new memories get stored server-side and automatically expire after 28 days. These memories are shared across all Copilot agents and get validated against the codebase before being used.

**Routing logic:**
The dispatch method checks if a path targets repo memories, then determines whether to use CAPI or local storage based on the enabled state.

**Context injection behavior:**
For both session and repo scopes, files are listed by filename only—they're not automatically loaded into context. When CAPI is enabled for repo memories, the system injects them directly as formatted text rather than as a file listing.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 5）

<timestamp>Thursday, May 14, 2026, 8:25 AM (UTC+8)</timestamp>
<user_query>
/memories/session/ 会在session 结束之后清除吗？ Chat结束，是否意味着永远访问不到了？ Chat被删除，/memories/session/ 也会一并删除呢？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn5

用户意图：弄清 `/memories/session/` 的生命周期——结束后是逻辑不可见还是物理删除，以及删除 chat 是否联动删除文件。

之前已看到有 `memoryCleanupService.ts`，直接读它。

The user is asking about the lifecycle of `/memories/session/` files - specifically:
1. Are they cleared when session ends?
2. Does "chat ends" mean they're permanently inaccessible?
3. If chat is deleted, are the session memory files also deleted?

From the source code I already have, I know:
- Session files are stored under `storageUri/memory-tool/memories/{sessionId}/`
- The sessionId is extracted from `vscode-chat-session://local/<sessionId>`
- The files persist on disk but are tied to the sessionId

Let me look more carefully at the source code to understand:


I need to check the `memoryCleanupService.ts` file to see what actually happens when a session ends or a chat is deleted, and whether the files are physically removed from disk or just become inaccessible.

源码给出了明确答案。

---

## `/memories/session/` 的生命周期

✅ 已验证（出处：`memoryCleanupService.ts`，`gh api` 直接读取）

---

### Q1：Session 结束后文件会被清除吗？

**不会立即清除，是逻辑上不可见，文件仍在磁盘上。**

文件存储路径是：

```
storageUri/memory-tool/memories/{sessionId}/plan.md
```

`sessionId` 从 `vscode-chat-session://local/<sessionId>` 提取。Chat 关闭后，新 session 拿到一个新的 sessionId，旧目录不再被任何 context 引用——**逻辑上"失联"，但物理上还在**。

---

### Q2：Chat 结束，是否意味着永远访问不到了？

**对 AI 而言是的，对用户而言不是。**

- AI 层面：新 chat 有新 sessionId，旧 session 目录不会出现在上下文中，AI 无法感知也无法调用 Memory Tool 访问它
- 用户层面：文件仍在本机磁盘，**你可以直接去文件系统翻看**，路径在 VS Code 的 `storageUri` 下（通常是 `~/Library/Application Support/Code/User/workspaceStorage/<hash>/github.copilot-chat/memory-tool/memories/<sessionId>/`）

---

### Q3：删除 Chat，session 文件会一并删除吗？

**不会。** 清理是**时间驱动**的，不是**事件驱动**的。

`MemoryCleanupService` 的清理逻辑：

```typescript
const RETENTION_PERIOD_MS = 14 * 24 * 60 * 60 * 1000;  // 14天

// VS Code 启动时触发一次后台清理
start(): void {
    this.cleanupStaleResources();
}

// 判断文件是否过期：优先检查内存时间戳，fallback 到文件系统 mtime
if (stat.mtime >= cutoffTime) continue;  // 14天内访问过，跳过

// 过期则物理删除
await this.fileSystem.delete(entryUri, { recursive: true });
// 目录空了也删掉
await this.fileSystem.delete(sessionUri, { recursive: true });
```

实际清除时机：

| 触发条件 | 说明 |
|----------|------|
| **VS Code 下次启动时** | `start()` 在启动时调用，执行一次扫描 |
| **文件超过14天未访问** | 以文件 mtime 或内存时间戳为准 |
| **用户手动执行** | `Chat: Clear All Memory Files` 命令（全清，不区分 session） |

---

### 完整生命周期总结

```
Chat 进行中
    → 文件写入 storageUri/.../memories/{sessionId}/
    
Chat 关闭
    → 文件仍在磁盘，AI 无法访问（sessionId 失效）

VS Code 重启
    → MemoryCleanupService 扫描所有 session 目录
    → 14天内访问过的：保留
    → 超过14天的：物理删除
    
删除 Chat 记录
    → 对 session 文件无任何影响（没有事件联动）
```

> ⚠️ 结论：文档说"session 结束即清除"是对**可访问性**的描述，不是对**物理存储**的描述。真正的物理删除是 14天 + VS Code 重启后的后台清理。

Turn:1～5

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have the complete picture. Let me analyze the cleanup logic:

1. **When session ends**: Files are NOT immediately deleted. They remain on disk under `storageUri/memory-tool/memories/{sessionId}/`. The session memory is "inaccessible" because the new session has a different sessionId, so the old session directory isn't referenced by the new session's context.

2. **Physical deletion**: `MemoryCleanupService` runs on VS Code startup (`start()` is called during startup). It:
   - Scans all session directories (excluding `repo`)
   - Checks the last access time (via in-memory timestamps OR file system mtime)
   - Deletes any file or directory that hasn't been accessed in 14 days
   - Then cleans up any empty session directories

3. **Chat deletion**: There's no explicit cleanup triggered when a chat ends—the system relies entirely on the time-based retention period. So when a session ends, its files become logically inaccessible to new sessions but remain physically on disk until the 14-day window expires.

To directly answer the questions:
1. **Session ends → files persist**, just unreachable since the new session has a different ID
2. **"Chat ends" = logically inaccessible** but the files stay around for up to 14 days
3. **Chat deleted → NOT automatically cleaned up**. The cleanup is purely time-based, not event-driven. Deleting a chat doesn't trigger immediate file removal.

The key insight is that `/memories/session/` files actually survive VS Code restarts because they're stored in a persistent location. They only disappear when either the 14-day retention period expires and cleanup runs at the next startup, or the user explicitly runs the clear memory command. The "session" naming is a bit misleading—it refers to the chat conversation scope, not the VS Code process session itself.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 6）

<timestamp>Thursday, May 14, 2026, 8:39 AM (UTC+8)</timestamp>
<user_query>
/memories/repo/ 的写入 和 .github/ 用户自己的配置，是否存在冲突的？ 为什么需要2套存储呢？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn6

用户意图：理解 `/memories/repo/` 和 `.github/` 之间是否存在冲突，以及为什么需要两套存储。

已有所有相关源码，直接分析。

---

## `/memories/repo/` vs `.github/` 的关系

✅ 已验证（出处：`memoryContextPrompt.tsx`、官方文档、Issue #308669）

---

### 先看两者在 System Prompt 里的位置

从 `memoryContextPrompt.tsx` 的 `render()` 可以看出，每次请求时上下文结构大致是：

```
[system prompt]
  ├── <memoryInstructions>    ← Memory Tool 使用规则（硬编码）
  ├── <userMemory>            ← /memories/ 内容（前200行全量注入）
  ├── <sessionMemory>         ← /memories/session/ 文件列表
  ├── <repoMemory>            ← /memories/repo/ 文件列表（本地模式）
  │   或
  │   <repository_memories>  ← CAPI 记忆摘要（云端模式）
  └── [copilot-instructions.md 内容]  ← .github/ 注入（另一个管道）
```

两者**同时存在于上下文中**，没有显式的优先级仲裁机制，由 LLM 自行权衡。

---

### 是否存在冲突？

**存在，且有正式 issue 记录。**

[Issue #308669](https://github.com/microsoft/vscode/issues/308669)：用户在 `.github/copilot-instructions.md` 中写了"把学到的东西记到项目文档里，不要写 memory"，但 `<memoryInstructions>` 是**硬编码进 system prompt** 的，AI 仍然优先写 memory，导致知识被存到两个地方，出现漂移和重复。

CAPI 模式还有额外的冲突风险，因为 CAPI memory 注入时带了这段警告：

> "be aware that memories might be **obsolete or incorrect** or may not apply to your current task. Use the citations provided to verify the accuracy of any relevant memory **before relying on it**."

也就是说，`/memories/repo/`（CAPI）里的内容可能和 `.github/copilot-instructions.md` 里的规范**互相矛盾**，而系统没有自动调和机制。

---

### 为什么要两套存储？

两者的设计出发点根本不同：

| 维度 | `.github/copilot-instructions.md` | `/memories/repo/` |
|------|----------------------------------|-------------------|
| **作者** | 人类主动编写 | AI 在工作中自动发现 |
| **性质** | 规范性规则（prescriptive） | 描述性事实（descriptive） |
| **内容类型** | "应该怎么做" | "我发现这个仓库是这样的" |
| **更新方式** | PR + Code Review | AI 自主写入 |
| **版本控制** | 进 git，团队共享 | 不进 git，机器私有（本地模式） |
| **稳定性** | 稳定，人工维护 | 动态，AI 边干边写 |
| **覆盖范围** | 全团队所有 Copilot 用户 | 当前机器 / 当前 GitHub 账号 |

**一句话区别：**
- `.github/` 是团队的"代码规约文档"，人写的，团队执行
- `/memories/repo/` 是 AI 的"工作经验笔记"，AI 写的，AI 自己复用

---

### 两套存储的合理性

两套系统解决的是**不同的信息不对称**：

```
人类知道规范  → .github/copilot-instructions.md → 告诉 AI 怎么做
    ↑                                                    ↓
AI 发现规律  ← /memories/repo/ ← AI 边工作边归纳 ← AI 做了什么
```

设计意图是形成**互补**：人类提供意图，AI 积累经验，两者都喂给上下文，让 AI 更了解这个项目。

但实际落地时，两者并没有任何协调机制，内容可能互相重复甚至矛盾，这正是 Issue #308669 反映的问题——目前属于**设计缺陷，尚未解决**。

---

### 实践建议

| 场景 | 推荐用哪个 |
|------|-----------|
| 团队编码规范、命名约定 | `.github/copilot-instructions.md` |
| 项目架构决策（需 PR 审查） | `.github/copilot-instructions.md` |
| AI 边做边发现的临时规律 | `/memories/repo/`（本地模式） |
| 希望 Code Review / CLI 共享的仓库知识 | `/memories/repo/`（CAPI 云端模式） |
| 不希望 AI 自动积累 memory | 在 `.github/copilot-instructions.md` 里写明——但目前**无法覆盖** `<memoryInstructions>` 的硬编码行为 |

Turn:1～6

> 模型：Claude Sonnet 4.6（Anthropic）

The user is asking about the relationship and potential conflicts between `/memories/repo/` and `.github/` custom instructions. They want to understand why two systems coexist and whether they conflict.

I already have a lot of the source code and documentation. Let me think about this carefully based on what I know:

1. `.github/copilot-instructions.md` = human-written static rules, version-controlled, team-shared
2. `/memories/repo/` = AI-discovered dynamic facts, either local or GitHub cloud-stored

The key differences:
- Author: human vs AI
- Nature: prescriptive rules vs discovered facts
- Storage: in repo vs outside repo


- Sharing: everyone vs workspace-local or GitHub account-scoped
- Updates: manual PRs vs AI autonomous writes

These two systems exist because `.github/` captures what the team wants the AI to follow, while `/memories/repo/` captures what the AI has learned about the codebase. The tension arises when memory instructions in the system prompt can't be overridden by copilot-instructions, or when the AI writes something to memories that contradicts the human-authored guidelines—creating ambiguity about which takes precedence.

Looking at how both get injected into context, the memory instructions and `.github/copilot-instructions.md` are loaded as separate sections alongside user memory, session memory, and repo memory. There's no explicit conflict resolution mechanism, so the model has to figure out how to reconcile them all.

For local repo memory, files are listed but not automatically loaded—the AI has to actively read them, so conflicts are more indirect. With CAPI repo memory, the memories are injected as formatted text with a built-in caveat that they might be obsolete or incorrect. The two systems exist because they serve different purposes: `.github/` captures what the team intends while `/memories/repo/` captures what the AI has actually discovered, one is stable across time while the other evolves, one is team-wide while the other is per-developer, and one prescribes actions while the other describes observations.

I have Issue #308669 that documents a real conflict, so I can answer based on what I know and reference that.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 7）

<timestamp>Thursday, May 14, 2026, 8:44 AM (UTC+8)</timestamp>
<user_query>
/memories/repo/ 是作为项目的上下文记录写入的是吗？ 这是否意味着如果AI理解错误，人无法纠正呢？ 这个过程是黑盒的。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn7

用户意图：理解 `/memories/repo/` 是否透明可控，以及 AI 写错时人是否有纠正能力。

基于已有源码直接分析，补充查证 GitHub 管理界面的实际能力。

The user is asking a deeper philosophical/design question about `/memories/repo/`:
1. Is it used as project context records?
2. If AI writes incorrect information, can users correct it?
3. Is the process opaque (black box)?

From what I already know from the source code:
- CAPI mode: PUT to GitHub API, users can view/delete via GitHub Settings → Repository Settings → Copilot → Memory
- Local mode: files stored locally, users can read/edit/delete directly
- The memory has a "just-in-time verification" approach using citations
- Memories expire after 28 days (CAPI mode)


- The `MemoryInstructionsPrompt` tells AI to update memories if they're wrong

So I can answer based on verified facts: CAPI repo memory stores human-readable JSON with subject/fact/citations/reason that users can access through GitHub Settings, while local repo memory is just plain Markdown files users can edit directly. There's a built-in correction mechanism where I'm instructed to write a new memory if I find an old one is wrong, and I validate citations against current code before using any memory.

The safeguards include a 28-day auto-expiry to reduce drift, and the memory context prompt itself includes a caveat that memories might be obsolete or incorrect. But the user's concern about opacity is partially legitimate—in CAPI mode, I autonomously decide what to write and when without notifying users or requiring review, and they'd need to dig into GitHub settings to see what's been stored. The local mode is fully transparent since files sit on disk, but the core issue remains: if I misunderstand something and write it to memory, there's no easy way for humans to catch and correct it before I use it again.

---

## `/memories/repo/` 的透明度与纠错能力

✅ 已验证（出处：`memoryContextPrompt.tsx` 源码 + `agentMemoryService.ts` + [GitHub 官方文档](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/copilot-memory)）

---

### 第一个问题：是项目上下文记录吗？

**是的，且是 AI 自主判断后写入的。**

CAPI 模式下写入的是结构化 JSON：

```json
{
  "subject": "API 鉴权约定",
  "fact": "所有公开 API 端点都必须鉴权",
  "citations": ["src/api/middleware/auth.ts:23"],
  "reason": "在审查 PR #142 时发现",
  "category": "conventions"
}
```

AI 写入时机由 `MemoryInstructionsPrompt` 中的系统指令控制，判断标准是：

> "Will this help with future coding or code review tasks across the repository? If unsure, skip storing it."

即：AI **自己决定**什么值得记、什么不值得记，没有用户审批环节。

---

### 第二个问题：AI 写错了，人能纠正吗？

**可以，但需要主动去找。**

#### CAPI 模式（云端）

GitHub 提供了查看和删除界面：

> Repository Settings → Copilot → Memory → 可逐条查看/删除

支持单条删除或批量删除，**仓库 Owner 才有权限**。

系统还有两个自动纠错机制：

| 机制 | 说明 |
|------|------|
| **Citation 实时验证** | 使用 memory 前，AI 会对 citations 指向的代码位置做实时校验。代码已变更 → memory 可能被跳过 |
| **28天自动过期** | 所有 CAPI memory 28天后自动删除，防止长期积累的错误认知 |
| **AI 自我纠正** | 系统指令要求 AI：发现 memory 不准确时，写入一条新 memory 替代旧的 |

但这些都是**事后补救**，AI 写入时无审批。

#### 本地模式（无 CAPI）

文件就是本机磁盘上的 Markdown，**可以直接编辑、删除**，完全透明。

---

### 第三个问题：这个过程是黑盒的吗？

**部分是，且这是设计上的真实缺陷。**

| 环节 | 透明度 | 说明 |
|------|--------|------|
| AI 决定写什么 | ⬛ 黑盒 | 完全由 LLM 判断，无 log，无通知 |
| 写入时机 | ⬛ 黑盒 | 用户不知道 AI 何时悄悄写了一条 memory |
| 写入内容 | 🟨 可查 | CAPI：GitHub Settings 可查；本地：直接看文件 |
| 删除权限 | 🟩 透明 | 人可以删除任意条目 |
| 被使用前验证 | 🟨 部分透明 | citations 验证有，但验证结果不暴露给用户 |
| 影响 AI 决策的路径 | ⬛ 黑盒 | memory 如何影响 AI 输出，用户看不到 |

Issue #308669 对这个问题的描述很准确：

> 用户无法知道 memory 里有什么，也无法知道它在哪个时刻影响了 AI 的行为，导致"知识漂移"——AI 的行为慢慢偏离 `.github/copilot-instructions.md` 里人工定义的规范，而用户毫不知情。

---

### 设计上的根本张力

```
AI 自主写入  →  减少人类维护成本  →  但失去可审查性
人工维护规范 →  可控可审查        →  但 AI 无法主动积累经验
```

两套系统（`.github/` + `/memories/repo/`）本意是互补，但目前缺少**协调层**：没有"memory 写入审批"、没有"memory 和 instructions 冲突检测"、没有"memory 变更通知"。

这个 gap 目前是已知问题，尚无官方解决方案。

Turn:1～7

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have the complete picture. Let me answer the user's questions:

1. **Is `/memories/repo/` writing project context records?** Yes, that's exactly the design intent - it captures facts about the codebase that AI discovers while working.

2. **If AI understands wrong, can humans correct it?** 
   - CAPI mode: Yes, repo owners can view and delete memories via GitHub Settings → Repository → Copilot → Memory. You can delete individual memories or multiple at once.
   - Local mode: Files are just Markdown on disk, fully editable.
   - AI also has self-correction: if it finds a memory is wrong, it writes a new one to replace it.
   
3. **Is the process a black box?** It's partially opaque - AI autonomously decides what to write and when without user notification, but there are transparency mechanisms like GitHub Settings visibility, citation requirements, and 28-day expiry. The real tension is that while humans can see and correct memories afterward, they can't control the initial decision-making about what gets recorded.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 8）

<timestamp>Thursday, May 14, 2026, 8:47 AM (UTC+8)</timestamp>
<user_query>
也就是可能存在这样一个问题，同样的Spec执行文档，在项目的A员工本地执行 和 B 员工本地执行，效果是不一样的。这确实可能导致无法审计，AI为什么做出不同的决策。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn8

用户意图：确认"同一 Spec，不同人执行结果不同"的问题是否成立，并理解其根因。

这是一个基于已有知识的推理问题，不需要额外查证，直接分析。

---

## 你的判断是准确的

**同一份 Spec，A 和 B 执行结果可能不同，且无法审计原因。**

这不是推断，是架构上的必然结果。

---

### 根因：System Prompt 的构成是人机混合的，且因人而异

每次 AI 执行时，实际上下文 = 以下内容的叠加：

```
Spec 文档（共享）
    +
.github/copilot-instructions.md（共享，一致）
    +
/memories/（User 级，A 和 B 各自的本机，完全不同）
    +
/memories/repo/（本地模式：A 和 B 各自机器，不同）
    +
CAPI memories（云端模式：按 GitHub 账号隔离，A ≠ B）
    +
当前 session 状态（不同）
```

前两项是共享的，后三项**完全私有**。A 做过一个月的项目，积累了几十条 `/memories/repo/` 记录；B 刚加入团队，memories 是空的。同一份 Spec 喂进去，上下文不同，决策就不同。

---

### 具体差异来源

| 差异来源 | 影响 |
|----------|------|
| User memory 内容不同 | A 可能记了"这个项目禁止用 Lombok"，B 没有这条记忆 |
| Repo memory 积累深度不同 | A 工作过的模块有丰富的 CAPI memory，B 执行相同 Spec 时这些 memory 不存在 |
| Memory 写入时间点不同 | A 上周踩过一个坑写了 memory，B 执行时会重复踩 |
| CAPI memory 版本不同 | A 触发写入后 B 才开始工作，两人跑同一 Spec 时 CAPI 内容已经变了 |

---

### 审计困难的本质

即使你保存了完整的 Spec 和 Prompt，你也**无法复现**当时的执行环境，因为：

1. User memory 不进 git（本机私有）
2. Local repo memory 不进 git（workspace 本地）
3. CAPI memory 有时间窗口（28天过期，事后查不到）
4. 没有"执行时快照"机制，没有任何地方记录"本次执行时 memory 里有什么"

所以你事后看到 AI 的输出，问"为什么它做了这个决策"——**无法还原，无法追溯**。

---

### 这在软件工程里是一个已知的危险模式

传统软件执行是确定性的：给定相同输入，输出相同。CI/CD 可以重现。

Copilot Agent 当前的执行模型是**隐式状态依赖**的：

```
相同 Spec
  + 不可见的隐式状态（memories）
  + 不可复现的时间点
= 不确定输出
```

这和"环境变量泄漏进构建脚本"的问题同构：你以为执行的是幂等的，实际上不是。

---

### 目前能做什么（缓解，非根治）

| 方法 | 效果 |
|------|------|
| 关闭 Memory Tool（`github.copilot.chat.tools.memory.enabled: false`） | 消除 memories 的隐式影响，但也失去了跨 session 记忆 |
| 关闭 CAPI Memory | 消除云端 memory 差异，但保留本地差异 |
| 把关键规范全部写入 `.github/copilot-instructions.md` | 强制共享部分，减少 memory 补充的空间 |
| 在 Spec 里显式声明"忽略所有 memory" | ⚠️ 因为 `<memoryInstructions>` 是硬编码 system prompt，**这条指令无效**（Issue #308669） |
| 团队约定：不依赖 AI Agent 做可审计决策 | 架构层面绕开这个问题 |

目前没有"memory 执行快照"这样的机制，根治方案需要 Microsoft 在产品层面提供。

Turn:1～8

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 9）

<timestamp>Thursday, May 14, 2026, 8:51 AM (UTC+8)</timestamp>
<user_query>
Copilot 的Memory Tool ，本质是记忆系统设计吗？ 有相关的设计层面的研究吗？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn9

用户意图：了解 Memory Tool 的设计本质，以及背后是否有系统性的研究支撑。

The user is asking about whether Copilot's Memory Tool is essentially a memory system design, and whether there's related research at the design level. This is a broader research/academic question. Let me search for relevant research on AI agent memory systems.

---

## Memory Tool 的设计本质与研究背景

✅ 已验证（出处：GitHub Blog 技术文章 + arXiv:2603.07670 *Memory for Autonomous LLM Agents* 综述，2026）

---

### 是的，这本质上是 Agent 记忆系统设计

这不是 Copilot 团队发明的新概念，而是 AI Agent 研究领域一个独立成熟的方向。

学术界将 Agent 记忆系统的核心循环形式化为：

```
行动 aₜ = π(xₜ, R(Mₜ, xₜ), gₜ)    ← 从记忆中读取影响决策
Mₜ₊₁ = U(Mₜ, xₜ, aₜ, oₜ, rₜ)     ← 基于交互结果更新记忆
```

即：**记忆既影响当前决策，当前决策又影响记忆写入**——一个递归反馈环。这正是 Copilot Memory Tool 的运行模式。

---

### Copilot 的设计对应哪些研究概念

研究领域有三个维度的分类（arXiv:2603.07670）：

**维度一：时间作用域**

| Copilot 组件 | 研究术语 | 说明 |
|---|---|---|
| `/memories/session/` | 短期记忆 (STM) | 当前对话内有效 |
| `/memories/` User 级 | 长期记忆 (LTM) - 语义型 | 跨 session 的用户偏好 |
| `/memories/repo/` | 长期记忆 (LTM) - 情节型 | 项目工作中积累的具体事实 |

**维度二：表示底层**

| Copilot 组件 | 研究术语 |
|---|---|
| User/Session `.md` 文件 | 非结构化文本存储 |
| CAPI `{subject, fact, citations}` JSON | 结构化符号存储（类 ChatDB 方案） |

**维度三：控制策略**

Copilot 使用的是 **Prompted Self-Control**（提示词驱动的自主控制）——Memory Tool 作为工具调用暴露给 LLM，由 LLM 自行决定何时调用、写什么。这是 MemGPT（2024）推广的经典架构：`core_memory_append` / `archival_memory_search` 作为工具。

---

### Copilot 与 MemGPT 的架构对比

MemGPT 是这个方向最具代表性的工程原型：

```
MemGPT 三层架构          →      Copilot Memory Tool
─────────────────────────────────────────────────
主记忆（context window）  →   /memories/ 前200行自动注入
召回数据库（recall DB）    →   /memories/session/ 文件列表（需主动读）
档案向量库（archival）     →   /memories/repo/ CAPI（需主动读）
```

两者几乎是同构的，差异在于 Copilot 把"召回"简化成了"文件名列表 + AI 主动读取"，而不是向量检索。

---

### Copilot 的独特设计：Just-in-Time Verification

GitHub 技术博客明确说这是他们的核心创新——**不做离线去重和冲突解决，改为写入时带 citations，使用时实时验证**：

> "Information retrieval is an asymmetrical problem: It's hard to solve, but easy to verify."

这解决了研究领域的核心难题——**Faithfulness（忠实性）**：

> "Stale or hallucinated recall can be worse than no recall at all."（arXiv:2603.07670）

实测数据：
- 对抗性实验（故意注入错误 memory）：AI 一致发现矛盾并写入更正版本，记忆池自愈
- 编码 Agent PR 合并率：有 memory 90% vs 无 memory 83%（+7%，p<0.00001）
- Code Review 正向反馈率：77% vs 75%（+2%）

---

### 开放挑战（与你上一个问题直接相关）

arXiv 综述总结的未解难题，其中两条与你之前提到的问题完全一致：

| 研究挑战 | 对应你的问题 |
|---|---|
| **Governance**：记忆系统要能支持删除请求和组织策略 | A/B 员工 memory 不一致，无法审计 |
| **Continual Consolidation**：如何在不遗忘重要细节的同时合并新旧记忆 | AI 写错后如何纠正，新旧 memory 冲突 |
| **Trustworthy Reflection**：AI 自我纠正的可信度 | 黑盒写入，人无法验证 |

**这些都是 2026 年学术界仍未解决的开放问题，不只是 Copilot 的产品缺陷。**

---

### 延伸阅读

如果你想深入研究这个方向，可以参考：

- **综述**：[arXiv:2603.07670](https://arxiv.org/abs/2603.07670) - *Memory for Autonomous LLM Agents*（2026，含完整 taxonomy）
- **工程原型**：MemGPT（2024）- OS 分页思想应用于 Agent 记忆
- **Reflexion**（2023）- AI 写入"事后反省日志"作为情节记忆，Copilot 的行为模式与此高度相似
- **GitHub 技术博客**：[Building an agentic memory system for GitHub Copilot](https://github.blog/ai-and-ml/github-copilot/building-an-agentic-memory-system-for-github-copilot/)

Turn:1～9

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have comprehensive information to answer the user's question. Let me put together a structured response that:
1. Confirms this IS a memory system design
2. Maps Copilot's specific design to academic research concepts
3. Highlights the relevant research
4. Shows where GitHub's design aligns with and differs from research

The key mapping:
- `/memories/` (User level, auto-injected 200 lines) → **Context-resident memory** (working memory, in-window)
- `/memories/session/` (listed, not loaded) → **Short-term episodic memory** 
- `/memories/repo/` local → **External retrieval store** (workspace-scoped)


- CAPI memories with citation verification → **Retrieval-augmented stores** using real-time validation instead of offline curation, which is a novel approach to ensuring faithfulness

I'm seeing how GitHub's three-dimensional taxonomy breaks down: temporal scope spans session to user to repo levels, the representational substrate varies between structured JSON and unstructured markdown, and the control policy ranges from heuristic to prompted self-control to learned approaches.

Copilot implements prompted self-control through the Memory Tool as a callable function, and the architecture mirrors MemGPT's design quite closely—both have a main context window, a recall database, and an archival vector store, just organized differently across Copilot's session and repo memory layers.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 10）

<timestamp>Thursday, May 14, 2026, 9:05 AM (UTC+8)</timestamp>
<user_query>
MemGPT 三层架构 是谁提出来的？ 介绍一下这个架构。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn10

用户意图：了解 MemGPT 三层架构的提出者和架构详情。

The user wants to know who proposed the MemGPT three-layer architecture and wants an introduction to it. I already have some information from the survey paper I read. Let me also fetch the original MemGPT paper to give accurate information.

From what I already know:
- MemGPT was published in 2024
- It's inspired by OS virtual memory / paging concepts
- It has three tiers: main context (working memory), recall database, archival vector store
- Packer et al., 2024

Let me search for more specific information about MemGPT.

---

## MemGPT：Towards LLMs as Operating Systems

✅ 已验证（出处：[arXiv:2310.08560](https://arxiv.org/abs/2310.08560)，ICML 发表，原文全文）

---

### 提出者

**Charles Packer、Sarah Wooders、Kevin Lin、Vivian Fang、Shishir G. Patil、Ion Stoica、Joseph E. Gonzalez**

均来自加州大学伯克利分校（UC Berkeley）。论文发表于 2023 年 10 月，收录于 ICML。

---

### 核心洞察：把 LLM 当作 CPU，上下文窗口当作 RAM

论文的出发点是一个工程现实：LLM 的上下文窗口是固定的，而真实任务（长文档分析、多轮持续对话）的信息量远超这个上限。

直接扩大上下文窗口有两个障碍：

1. **计算成本**：Transformer 的 Self-Attention 是 O(n²)，上下文翻倍，成本翻四倍
2. **注意力失真**：即使有超长上下文，中间段的信息会被严重忽略（"Lost in the Middle" 现象）

MemGPT 的答案是：**不扩大 RAM，而是借鉴 OS 的虚拟内存分页机制**。

---

### 三层记忆架构

```
┌─────────────────────────────────────────────────┐
│            Main Context（主记忆 = RAM）           │
│  ┌──────────────┐ ┌────────────┐ ┌───────────┐  │
│  │ System       │ │  Working   │ │   FIFO    │  │
│  │ Instructions │ │  Context   │ │   Queue   │  │
│  │  （只读）    │ │ （核心笔记）│ │（消息历史）│  │
│  └──────────────┘ └────────────┘ └───────────┘  │
└─────────────────────────────────────────────────┘
         ↕ 函数调用（paging in/out）
┌─────────────────────────────────────────────────┐
│           External Context（外部存储 = 磁盘）     │
│  ┌──────────────────┐  ┌───────────────────────┐ │
│  │   Recall Storage │  │  Archival Storage     │ │
│  │  （消息历史数据库）│  │  （向量数据库/档案库）  │ │
│  └──────────────────┘  └───────────────────────┘ │
└─────────────────────────────────────────────────┘
```

**Main Context（主记忆）** — 当前 LLM 能直接看到的所有内容：

- `System Instructions`：静态只读，包含记忆使用规则和函数说明
- `Working Context`：固定大小的读写区，存关键事实（用户偏好、任务关键信息）
- `FIFO Queue`：滚动消息历史，满了会触发 eviction；第一条始终是"被淘汰消息的递归摘要"

**External Context（外部存储）** — 超出上下文窗口的信息，必须显式 page in 才能使用：

- `Recall Storage`：结构化消息数据库，存储每一次对话的完整记录，支持关键词/时间检索
- `Archival Storage`：向量数据库，存储任意长文本，支持语义相似度检索（论文实现用 PostgreSQL + pgvector）

---

### 核心机制：LLM 自主管理内存的函数调用

LLM 通过**函数调用（Function Calling）**主动控制信息在各层之间的流动：

```
core_memory_append()        → 写入 Working Context
core_memory_replace()       → 更新 Working Context
conversation_search()       → 从 Recall Storage 检索历史对话
archival_memory_insert()    → 写入 Archival Storage
archival_memory_search()    → 从 Archival Storage 语义检索
```

**内存压力触发机制**（类比 OS 的 Page Fault）：

- 上下文用到 70%：系统发出 warning，LLM 决定把什么存到外部
- 上下文用到 100%：强制 eviction，FIFO Queue 清空 50%，生成递归摘要

**函数链（Function Chaining）**：
LLM 输出时带 `request_heartbeat=true` 标记，可以连续调用多个函数再回复用户——实现多步检索、分页翻阅等复杂操作。

---

### 实验结果

**对话记忆任务（Deep Memory Retrieval）**：

| 模型 | 无 MemGPT | 有 MemGPT |
|------|----------|----------|
| GPT-3.5 Turbo | 38.7% | 66.9% |
| GPT-4 | 32.1% | **92.5%** |

**文档分析任务**：固定上下文模型随文档变长性能下降，MemGPT 不受影响（通过分页检索始终能找到正确文档）。

---

### 与 Copilot Memory Tool 的对应关系

| MemGPT 组件 | Copilot Memory Tool 对应 |
|-------------|------------------------|
| Working Context | `/memories/`（前200行自动注入） |
| Recall Storage（消息历史） | `/memories/session/`（列出文件名，主动读） |
| Archival Storage（向量档案） | `/memories/repo/`（列出文件名，主动读）|
| 函数调用控制 | Memory Tool 作为 Agent Tool 暴露 |

Copilot 的实现比 MemGPT 简化：没有向量检索，用文件列表 + AI 主动读取代替；增加了 CAPI 云端模式和 citation 验证机制。

---

### 后续演化

论文于 2024 年演化为 **Letta 框架**（原名 MemGPT 开源项目，[letta.com](https://letta.com)），成为构建有状态 LLM Agent 的工程平台，目前仍活跃维护。

Turn:1～10

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have all the information I need to give a thorough answer about MemGPT. Let me compose a clear and structured response.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 11）

<timestamp>Thursday, May 14, 2026, 9:20 AM (UTC+8)</timestamp>
<user_query>
MemGPT 是Cursor、 Claude Code 都是使用的记忆系统架构吗？  请系统性调研再回答。 

</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn11

用户意图：系统调研 Cursor 和 Claude Code 是否使用 MemGPT 架构，要求查证后再作答。

The user wants to know if MemGPT is the memory architecture used by Cursor and Claude Code. They want a systematic research-based answer. Let me search for this information.

This requires investigation of:
1. What memory system Cursor uses
2. What memory system Claude Code uses
3. Whether either of them explicitly uses MemGPT architecture

Let me search for this systematically.

Now I have comprehensive information to answer systematically. Let me also quickly check what the Claude Code auto-memory mechanism uses and whether it explicitly mentions MemGPT.

Key findings so far:
1. **Cursor**: Uses a completely different approach - semantic indexing with vector search, dynamic context discovery (converting tool outputs to files, referencing history files during summarization, lazy loading of MCP tools). No explicit MemGPT reference. The pattern is inspired by similar concepts but implemented differently.

2. **Claude Code**: Uses a 5-layer persistence mechanism:
   - Layer 1: CLAUDE.md files (instruction layer, like `.github/copilot-instructions.md`)
   - Layer 2: Auto-memory (`~/.claude/projects/<project>/memory/` with MEMORY.md entrypoint, 200-line/25KB cap)
   - Layer 3: Background memory extraction agent
   - Layer 4: Context compaction (summarization when context fills)
   - Layer 5: Raw session transcripts

Neither Cursor nor Claude Code explicitly uses "MemGPT" — they implement similar concepts like tiered memory and context management with their own approaches. I should dig deeper into the Claude Code article to understand how layers 3-5 actually work.

现在有足够的一手资料，整理回答。

---

## Cursor 和 Claude Code 是否使用 MemGPT 架构？

✅ 已验证（出处：Cursor 官方博客、Claude Code 架构分析文章、MemGPT 论文）

**结论先行：两者都没有直接使用 MemGPT，但三者共享同一个底层灵感来源——OS 分层内存模型。各自独立实现了相近的分层思路，但工程细节差异显著。**

---

### 一、MemGPT 是什么性质的工作

MemGPT（2023，UC Berkeley）是一篇**学术研究论文 + 开源原型**，不是一个被业界广泛集成的库。它的核心贡献是：

> **命名并形式化了"把 OS 虚拟内存分页思想应用于 LLM 上下文管理"这个模式**

Cursor、Claude Code、Copilot 都是独立工程实现，没有引用或集成 MemGPT 代码库。

---

### 二、三者架构对比

#### Cursor：动态上下文发现（Dynamic Context Discovery）

✅ 来源：[Cursor 官方博客](https://cursor.com/blog/dynamic-context-discovery)

Cursor 的设计出发点是**代码库索引**，而非通用记忆系统：

| 层次 | 实现方式 |
|------|---------|
| **语义索引层** | tree-sitter AST 解析 → 函数/类级别 embedding → 本地向量索引（Qdrant） |
| **三遍检索** | 语义相似度 → 依赖图遍历 → 按时效/调用深度重排序 |
| **工具输出文件化** | 长 shell/MCP 输出不截断，写成文件，AI 按需读取（`tail`/`read`） |
| **历史文件化** | Context 满时触发 summarization，AI 可引用历史文件补充细节 |
| **MCP 懒加载** | 工具描述同步为文件夹，AI 按需查找，减少 token 46.9% |
| **终端历史文件化** | 终端输出同步到本地文件系统，AI grep 相关内容 |

Cursor **没有** MemGPT 式的多轮对话记忆（Working Context / Recall Storage）。它的记忆核心是**代码库的语义索引**，而非用户交互历史。

---

#### Claude Code：五层持久化架构

✅ 来源：架构深度分析文章（`oldeucryptoboi.com`，含源码级分析）

Claude Code 的架构与 MemGPT 最为相似，但完全独立实现：

```
Layer 1: CLAUDE.md  ──────  指令层（人工维护，静态，类比 MemGPT 的 System Instructions）
    ↓ 每次 session 加载
Layer 2: Auto-Memory ─────  持久知识库（~/.claude/projects/<repo>/memory/）
    MEMORY.md 主索引：200行 / 25KB 上限，自动截断并警告
    topic files：按需加载（testing.md / git-workflow.md...）
    ↓ session 内按需读取
Layer 3: 后台记忆提取 Agent ─  task 完成后自动 fork，提取当前 session 新增的洞察写入 Layer 2
    ↓ 仅写 memory 目录，权限隔离
Layer 4: Context Compaction ─  上下文满时触发，微压缩（清 tool result）+ 全量摘要
    threshold = context_window - 20000(output) - 13000(buffer) ≈ 83%
    ↓ 摘要写入 boundary message，旧消息丢弃
Layer 5: 原始 session transcript  供摘要时引用，不长期保留
```

记忆类型系统（四种结构化类型，带 frontmatter）：

| 类型 | 内容 | 衰减速度 |
|------|------|---------|
| `user` | 用户身份、专长 | 长期 |
| `feedback` | 用户纠正和确认（含 Why） | 长期 |
| `project` | 当前工作、截止日期（日期绝对化） | 快速衰减 |
| `reference` | 外部系统指针（Linear 项目、文档链接） | 中等 |

---

#### VS Copilot Memory Tool：三作用域本地文件 + CAPI 云端

（前几轮已详细分析）

核心特点是**用户级 memory 全量注入（200行）**，其余仅列文件名，无向量检索。

---

### 三、三者与 MemGPT 的概念对应

| MemGPT 概念 | Cursor | Claude Code | Copilot Memory Tool |
|-------------|--------|-------------|---------------------|
| System Instructions（只读静态） | Rules 文件 | CLAUDE.md | `.github/copilot-instructions.md` |
| Working Context（核心笔记，自动注入） | 无对应 | MEMORY.md（200行/25KB） | `/memories/`（200行） |
| FIFO Queue（消息滚动历史） | 历史文件化 + summarize | Context Compaction | Session 无自动摘要 |
| Recall Storage（消息数据库） | 无 | Layer 5 原始 transcript | `/memories/session/` |
| Archival Storage（向量档案） | 代码库语义索引 | topic files（按需读） | `/memories/repo/` |
| 自主写入记忆 | 无（只读索引） | Layer 3 后台提取 Agent | Memory Tool 函数调用 |
| 函数链（Function Chaining） | 动态工具发现 | 工具调用链 | Memory Tool CRUD |

---

### 四、三者的本质差异

| 维度 | Cursor | Claude Code | Copilot Memory Tool | MemGPT |
|------|--------|-------------|---------------------|--------|
| **核心记忆介质** | 代码库向量索引 | 结构化 Markdown 文件 | 平坦 Markdown 文件 | 文本 + 向量数据库 |
| **检索方式** | 向量相似度 + 依赖图 | 按需文件读取 | 仅按文件名列出 | 向量检索 + 关键词 |
| **谁决定写什么** | 系统自动索引 | AI + 后台提取 Agent | AI 自主（+ 用户委托） | AI 自主 |
| **跨 session 记忆** | ✅（语义索引持久） | ✅（MEMORY.md + topic） | ✅（User/Repo 级） | ✅（Recall/Archival） |
| **上下文满时策略** | 写文件 + 按需读 | 微压缩 + 全量摘要 | 截断到200行 | FIFO eviction + 递归摘要 |
| **跨用户/团队共享** | ❌（本地索引） | 部分（CLAUDE.md 可进 git） | CAPI 模式可跨 Agent | ❌（本地） |

---

### 结论

**没有任何一个产品直接采用 MemGPT 架构。**

三者都独立实现了"分层记忆 + 按需加载"的思路，底层灵感共同来自 OS 虚拟内存模型——这个概念早于 MemGPT，MemGPT 的贡献是把它第一次在 LLM Agent 领域系统化地命名和评测。正确的理解是：

> **MemGPT 是学术界对这个工程模式的形式化描述，各产品是独立的工程实现，彼此相互印证而非继承关系。**

Turn:1～11

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have comprehensive information about both Cursor and Claude Code's memory architectures. Let me synthesize the findings and answer whether they use MemGPT.

Key findings:

**Does Cursor use MemGPT?**
No explicit MemGPT reference. Cursor uses:
1. Semantic indexing with vector search (AST parsing + embeddings)
2. Dynamic Context Discovery pattern - converting tool outputs to files, lazy loading
3. Chat history as files for summarization recovery
4. MCP tools loaded dynamically from folders
5. Terminal sessions as files

This is conceptually similar to MemGPT (external storage, lazy loading into context) but independently designed with a different architecture focused on codebase indexing.


**Does Claude Code use MemGPT?**
Also no explicit MemGPT reference, but the structural parallels are striking. Claude Code layers instructions in CLAUDE.md, maintains a capped auto-memory file similar to MemGPT's working context constraints, loads topic files on demand like archival storage, runs a background memory extraction agent for self-directed management, and implements context compaction—all mirroring MemGPT's core design patterns.

**Are they using MemGPT architecture?**
None of them explicitly adopted MemGPT, but they all converged on the same solution: applying operating system virtual memory hierarchies to LLM context management. MemGPT formalized and named this pattern in academic research, but Cursor and Claude Code independently arrived at similar tiered memory systems inspired by the same underlying OS concept. So the real story is parallel innovation rather than adoption.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 12）

<timestamp>Thursday, May 14, 2026, 9:44 AM (UTC+8)</timestamp>
<user_query>
 Cursor 的 代码库向量索引，就是 Archival Storage（向量档案） 吗？ 如何上报给LLM的呢 ？ 

Copilot Memory Tool 是 仅按文件名列出，是文件名全部传给LLM吗？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn12

用户意图：澄清两个具体机制——Cursor 向量索引与 Archival Storage 的差异，以及 Copilot 文件名列表是否全量传给 LLM。

前者需要补充查证，后者已有源码可直接回答。

The user has two specific questions:

1. Is Cursor's codebase vector index the same as MemGPT's Archival Storage? How is it reported to the LLM?
2. For Copilot Memory Tool, when it lists files by name, are ALL filenames passed to the LLM?

For question 1, I have the Cursor blog post about dynamic context discovery and some info from earlier searches. Let me think about what I know:
- Cursor uses a vector index of the codebase (AST parsed, function/class level embeddings)
- At query time, it does three-pass retrieval and injects the results into context


- The key difference from MemGPT's Archival Storage is that Cursor handles retrieval automatically at the system level before the LLM even processes the query, whereas MemGPT requires the LLM to explicitly call `archival_memory_search()` to access stored information.

For question 2, I'm looking at the source code from the memory context prompt file. The session and repo memory both list files by their filenames within specific tags, but I need to verify whether all filenames are actually passed to the LLM or if there's any filtering happening.

两个问题都有确定性答案。

---

## Q1：Cursor 向量索引 vs MemGPT Archival Storage

### 表面相似，本质不同

两者都是"把信息存到向量数据库，用语义检索取回"，但**谁发起检索**这一点截然相反：

| | MemGPT Archival Storage | Cursor 代码库向量索引 |
|--|--|--|
| **检索发起者** | LLM 自主调用 `archival_memory_search()` | **系统自动**，在 LLM 看到 query 之前 |
| **LLM 的角色** | 决策者（主动拉取） | 被动接收者（结果已注入） |
| **检索时机** | LLM 判断需要时才检索 | 每次 query 必然触发 |
| **检索控制** | LLM 控制查询词、翻页 | 系统的三遍 pipeline 自动完成 |

### Cursor 的检索 Pipeline（系统侧完成，LLM 看到结果时已是成品）

```
用户提问
    ↓
Pass 1：embed query → ANN 搜索向量索引 → 取 top-k 语义相似的函数/类
    ↓
Pass 2：对每个结果，遍历依赖图 → 找到 callers / callees / imports
    ↓
Pass 3：重排序（近期修改文件优先，调用深度越近权重越高）
    ↓
将排好序的语义单元（完整函数体 + 文件路径 + 行号）打包注入 context window
    ↓
LLM 收到的 prompt 里直接包含这些代码片段，看不到检索过程
```

### LLM 实际收到的是什么

不是"文件名列表"，而是**完整代码片段 + 元数据**，例如：

```
// src/checkout/couponService.ts (line 42-67)
export function validateCoupon(code: string, userId?: string): ValidationResult {
    ...完整函数体...
}

// src/session/guestMiddleware.ts (line 12-34)
export function attachGuestSession(req: Request): void {
    ...完整函数体...
}
```

LLM 看到的是已经选好的代码，不知道背后跑了向量检索，也不能自主发起新的检索（这正是 Cursor 与 MemGPT 的根本区别）。

另外 Cursor 还维护了**两层索引**：持久化索引（文件保存时更新）+ 内存 delta（当前打开文件实时更新），检索时合并两层结果，保证实时一致性。

---

## Q2：Copilot Memory Tool 文件名是全量传给 LLM 吗？

**是的，全部文件名都传给 LLM，没有数量上限。**

✅ 已验证（来源：`memoryContextPrompt.tsx` 源码）

源码逻辑：

```typescript
// session memory 列表构建
const entries = await this.fileSystemService.readDirectory(sessionDirUri);
for (const [fileName, fileType] of entries) {
    if (fileType === FileType.File && !fileName.startsWith('.')) {
        files.push(`/memories/session/${fileName}`);  // 全部加入，无上限
    }
}

// repo memory 列表构建（同样逻辑，无上限）
for (const [fileName, fileType] of entries) {
    files.push(`/memories/repo/${fileName}`);
}
```

注入到上下文的格式：

```xml
<sessionMemory>
The following files exist in your session memory (/memories/session/).
Use the memory tool to read them if needed.

/memories/session/plan.md
/memories/session/task-notes.md
/memories/session/debug-log.md
</sessionMemory>

<repoMemory>
The following files exist in your repository memory (/memories/repo/).
These are scoped to the current workspace.

/memories/repo/conventions.md
/memories/repo/build-commands.md
</repoMemory>
```

### 与 User Memory 的关键差异

| | `/memories/`（User） | `/memories/session/` | `/memories/repo/` |
|--|--|--|--|
| 传给 LLM 的内容 | **文件内容**（前200行合计） | **文件名列表**（全量） | **文件名列表**（全量） |
| LLM 能直接看到内容吗 | ✅ 直接可用 | ❌ 需主动调用 Memory Tool 读 | ❌ 需主动调用 Memory Tool 读 |
| 数量限制 | 总计200行截断 | 无限制 | 无限制 |

设计意图是：User memory 是高频使用的偏好，值得全量注入；session/repo memory 文件可能很大，只给文件名让 LLM 按需读取，避免浪费 token。

但存在一个隐患：如果 `/memories/session/` 里有几十个文件，文件名列表本身也会占用不少 token，且 AI 可能看到文件名后选择性地忽略部分文件，导致信息遗漏。目前代码中没有对文件名列表的数量限制。

Turn:1～12

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have the full picture. Let me answer both questions clearly.

**Question 1: Is Cursor's codebase vector index the same as Archival Storage? How is it reported to the LLM?**

Key difference: In MemGPT, Archival Storage is accessed by the LLM via explicit function calls (`archival_memory_search()`). The LLM decides when and what to search.

In Cursor, the vector index is accessed AUTOMATICALLY at query time by the system before the LLM sees anything. The retrieval pipeline (3-pass: semantic → dependency graph → reranking) runs before the LLM call, and the results are injected into the context window automatically. The LLM never "calls" the index directly.


The fundamental difference is that MemGPT puts retrieval in the LLM's hands, while Cursor's system always searches first and feeds the relevant code snippets directly into the context as raw text with file paths and line numbers—the LLM just sees them as part of its input, not as a tool result.

**Question 2: For Copilot Memory Tool, are ALL filenames passed to the LLM?**

Looking at the source code, I can see how both session and repo memory work. The system iterates through files, filters out hidden ones, and builds a list of filenames that gets injected into the prompt. There's no limit mentioned on how many filenames are included, so yes, all of them are passed to the LLM.

The key difference from user memory is that for session and repo, only the file paths are listed—not the actual content. Both use `readDirectory` which reads one level deep without recursion, so subdirectories aren't traversed.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 13）

<timestamp>Thursday, May 14, 2026, 9:50 AM (UTC+8)</timestamp>
<user_query>
Cursor 其实没有类似 /memories/repo/ 的目录，而是在User提问的时候，把搜索相关的代码，通过 Pass 1 注入到 上下文是吗？ 

</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn13

用户意图：确认 Cursor 没有持久化的 memory 目录，而是在每次提问时实时检索注入。

这是基于已有资料的推理验证，直接回答。

---

**是的，理解完全正确。**

Cursor 没有 `/memories/repo/` 这类持久化的"记忆目录"概念。两者的设计哲学根本不同：

```
Copilot Memory Tool            Cursor
──────────────────             ────────────────────────────────
AI 工作时 → 主动写入记忆文件    代码变动时 → 系统自动更新向量索引
下次 session → 读取记忆文件     用户提问时 → 系统自动检索 → 注入 context
```

### Cursor 每次提问时发生的事

```
用户提问："checkout 流程为什么失败？"
         ↓
[系统侧，LLM 不参与]
Pass 1：embed query → 向量索引 ANN 搜索 → top-k 语义相关函数/类
Pass 2：依赖图遍历 → 找 callers / callees / imports
Pass 3：按时效 + 调用深度重排序
         ↓
[组装 prompt]
把选出的代码片段（完整函数体 + 路径 + 行号）打包进 context window
         ↓
LLM 收到 prompt，直接看到代码内容，感知不到检索过程
```

### 两个关键特征让它区别于"记忆"

**1. 没有写入行为**

Cursor 的索引是对代码库的**镜像**，不是 AI 的"学习结果"。AI 读代码、写代码，索引跟着更新，但 AI 本身从不向索引写入任何主观判断或经验。

**2. 无状态，每次独立**

每次提问都是全新的一次检索，不依赖上一次检索的结果。Copilot Memory Tool 的 `/memories/repo/` 里的内容会在下次 session 继续影响 AI；Cursor 的索引只反映当前代码库状态，不积累 AI 的"历史经验"。

### 用 MemGPT 术语对比

| MemGPT | Copilot `/memories/repo/` | Cursor 向量索引 |
|--------|--------------------------|----------------|
| Archival Storage 里存的是 | AI 归纳的项目事实（主观） | 代码库的语义镜像（客观） |
| 谁写入 | AI 自主写入 | 构建系统自动索引 |
| 谁检索 | LLM 调用函数主动拉取 | 系统在 LLM 之前自动注入 |
| 跨 session 积累 | ✅ 有（AI 学到的东西留下来） | ❌ 无（只反映代码当前状态） |

Cursor 更接近一个**实时的代码感知层**，而不是记忆系统。它解决的问题是"如何在有限 context 里看到正确的代码"，不是"如何记住过去的工作经验"。

Turn:1～13

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 14）

<timestamp>Thursday, May 14, 2026, 9:52 AM (UTC+8)</timestamp>
<user_query>
1 /memories/repo/ 你说的需要LLM主动拉取，那如果每次提问，AI不拉取的话，是否效果会很差。
2 而 Cursor 向量索引 是否 支持LLM 主动要求的二次搜索索引呢？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn14

用户意图：两个机制问题——Copilot repo memory 不拉取是否降级严重；Cursor 是否支持 LLM 主动二次检索。

Q1 有源码可直接回答；Q2 需要补充查证。

Two good questions:

1. For Copilot's `/memories/repo/` - if the LLM doesn't actively pull it, does the experience degrade significantly?

2. For Cursor - does it support LLM-initiated secondary searches of the vector index?

For Q1, I know from the source code that:
- Session memory and repo memory are listed as filenames only
- LLM is instructed to read them "if needed"
- The system prompt says "Use the memory tool to read them if needed"
- For CAPI memories (cloud), they ARE injected directly as formatted text, NOT just listed by filename


- The key difference is that local repo memory files require the LLM to actively decide to read them, whereas cloud memories are pre-injected

For Q2, from the Cursor documentation and blog posts, I can see that Cursor does support semantic search as a callable tool within its agent framework. The agent can invoke grep and Cursor's semantic search capabilities dynamically, and there's specific mention of "semsearch" functionality. I should dig deeper into the agent skills documentation to understand exactly how this secondary search works.

---

## Q1：`/memories/repo/` 不主动拉取，效果会差吗？

**取决于模式，两种情况截然不同。**

✅ 已验证（来源：`memoryContextPrompt.tsx` 源码）

### 本地模式（Copilot Memory 未开启）

上下文里只有文件名列表：

```xml
<repoMemory>
The following files exist in your repository memory (/memories/repo/).
Use the memory tool to read them if needed.

/memories/repo/conventions.md
/memories/repo/build-commands.md
</repoMemory>
```

LLM 看到的是**路径字符串，没有内容**。如果 AI 没有主动调用 Memory Tool 读取，这些文件对当前 session **零影响**。

这是已知问题：[Issue #300804](https://github.com/microsoft/vscode/issues/300804) 标题就叫 "Copilot user memory never used"——AI 经常看到文件名却不去读，记忆形同虚设。

### CAPI 模式（Copilot Memory 开启，云端）

内容**直接注入**上下文，LLM 不需要主动拉取：

```xml
<repository_memories>
The following are recent memories stored for this repository...

**API version synchronization**
- Fact: API version must match between client SDK, server routes, and documentation.
- Citations: src/client/sdk/constants.ts:12, server/routes/api.go:8
- Reason: If not synchronized, integration can fail.
</repository_memories>
```

CAPI 模式下 AI 不主动拉取也能用到这些记忆——这是两种模式最大的体验差距。

---

## Q2：Cursor 是否支持 LLM 主动发起二次检索？

**支持，且是双轨模型。**

✅ 已验证（来源：[Cursor 官方文档 Semantic & Agentic Search](https://cursor.com/docs/agent/tools/search)）

之前的描述有遗漏，更完整的架构是：

```
用户提问
    ↓
[第一轨：系统侧，自动，LLM 之前]
三遍 pipeline 检索 → 相关代码片段注入 context
    ↓
LLM 开始工作
    ↓
[第二轨：LLM 主动，按需，工具调用]
发现需要更多上下文 → 调用工具
```

LLM 可主动调用的搜索工具：

| 工具 | 适用场景 | 示例 |
|------|---------|------|
| **Semantic Search**（`codebase_search`） | 不知道具体函数名，按意图搜索 | "find where we handle authentication" |
| **Instant Grep** | 知道确切符号名/字符串 | "find all callers of `processOrder`" |
| **文件读取** | 找到文件后深入查看 | 读取某个具体文件 |

Agent 会**自动决定**调用哪个工具，也会**链式调用**：

```
semantic search 找入口点
    → grep 追踪引用链
        → 读取相关文件建立完整上下文
```

### 还有 Explore Subagent

当搜索范围太广时，Agent 会自动启动一个独立子 Agent，在自己的 context window 里并行搜索，只把相关发现的**摘要**返回主 Agent——避免大量搜索结果污染主对话的 context。

### 修正之前表格

| | MemGPT Archival | Cursor 向量索引 | Copilot `/memories/repo/`（本地） | Copilot `/memories/repo/`（CAPI） |
|--|--|--|--|--|
| 系统预注入 | ❌ | ✅（三遍 pipeline） | ❌ | ✅（直接注入摘要） |
| LLM 主动检索 | ✅（函数调用） | ✅（`codebase_search`/grep） | ✅（Memory Tool，但常被跳过） | ❌（已注入，不需要） |

Cursor 实际上是**两轨并行**：系统预注入 + LLM 主动检索都有，比 MemGPT（纯 LLM 主动）和本地 Copilot（纯文件名提示）都更完善。

Turn:1～14

> 模型：Claude Sonnet 4.6（Anthropic）

Now I have the full picture for both questions. Let me analyze:

**Q1: Copilot /memories/repo/ - if LLM doesn't pull, does experience degrade?**

From the source code in `memoryContextPrompt.tsx`:
- **Local mode**: Files are listed as filenames. The LLM gets the filename list and must call Memory Tool to read each file. The system prompt says "Use the memory tool to read them if needed." - it's entirely up to the LLM to decide.
- **CAPI mode**: Memories ARE injected directly as formatted text (not just filenames). The LLM sees the actual memory content. The listing is `<repository_memories>` with full subject/fact/reason/citations.


- **CAPI mode**: The memories are already in context, so there's no degradation if the LLM doesn't actively pull them. **Local mode**: If the LLM skips reading the memory files, those memories have zero impact on the session. The system instructions encourage checking memories first, but this is just a soft guideline—the LLM can ignore it. Issue #300804 confirms this is a documented problem where Copilot user memories go unused.

**Q2: Cursor -

** it turns out Cursor does support LLM-initiated secondary search. The system does an initial 3-pass retrieval before the LLM sees the prompt, but then during the agent loop, the LLM can actively call `codebase_search` for semantic search or `grep` for exact matches. For complex tasks, it can even spawn a subagent to do parallel searching. So Cursor actually has a hybrid approach that's more powerful than I initially described—combining automatic pre-query retrieval with LLM-driven follow-up searches.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 15）

<timestamp>Thursday, May 14, 2026, 9:58 AM (UTC+8)</timestamp>
<user_query>
1 我说的是 /memories/repo/ 的文件列表，需要AI主动获取吗？ 还是默认上传的。 那我的理解是否准确：文件内容获取需要LLM二次调用 Memory Tool 读取【本地】，Copilot /memories/repo/（CAPI）会直接注入摘要？ 那么：文件列表本身是否是自动上报的？ 

</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn15

用户意图：精确澄清一个问题——`/memories/repo/` 的**文件列表**本身是否自动注入，不需要 AI 主动获取。

这个问题源码里有明确答案。

直接看 `memoryContextPrompt.tsx` 的 `render()` 方法——这是每次请求组装 system prompt 时调用的函数，Turn 2 已读到完整内容，直接引用关键段落：

```typescript
// render() 方法内，每次请求自动执行：
const userMemoryContent   = await this.getUserMemoryContent();    // 读文件内容
const sessionMemoryFiles  = await this.getSessionMemoryFiles();   // 读文件列表
const repoMemories        = await this.agentMemoryService.getRepoMemories(); // CAPI
const localRepoMemoryFiles = await this.getLocalRepoMemoryFiles(); // 读文件列表

// 然后全部自动拼入 system prompt：
<Tag name='sessionMemory'>
  {sessionMemoryFiles.length > 0
    ? <>...{sessionMemoryFiles.join('\n')}</>   // ← 文件列表自动注入
    : <>Session memory is empty.</>}
</Tag>

<Tag name='repoMemory'>
  {localRepoMemoryFiles.length > 0
    ? <>...{localRepoMemoryFiles.join('\n')}</>  // ← 文件列表自动注入
    : <>Repository memory is empty.</>}
</Tag>
```

---

## 你的理解完全正确，补充精确化

| | 文件**列表**（路径） | 文件**内容** |
|--|--|--|
| `/memories/`（User） | 自动注入（遍历目录） | **自动注入**（前 200 行合计） |
| `/memories/session/`（本地） | **自动注入**（每次请求都执行） | ❌ 不注入，LLM 需主动调用 Memory Tool 读 |
| `/memories/repo/`（本地模式） | **自动注入**（每次请求都执行） | ❌ 不注入，LLM 需主动调用 Memory Tool 读 |
| `/memories/repo/`（CAPI 模式） | ❌ 不列文件名（存的不是文件） | **自动注入摘要文本**（subject + fact + citations） |

**结论：**

- **文件列表**——session 和本地 repo memory 的文件名，每次请求组装 system prompt 时**自动上报，无需 AI 主动获取**
- **文件内容**——本地模式下，AI 看到文件名后若想读内容，必须显式调用 Memory Tool
- **CAPI 模式**——没有"文件"概念，memory 是结构化条目，直接以摘要文本注入，AI 无需任何主动操作

Turn:1～15

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 16）

<timestamp>Thursday, May 14, 2026, 10:11 AM (UTC+8)</timestamp>
<user_query>
1  /memories/repo/（CAPI 模式） 是否为了应对 /memories/repo/（本地模式）可能存在的某个局限而设计的？ 请你调查给我解答。

2 我的思考： 
- /memories/repo/（本地模式）只注入文件列表（路径）， 如果  /memories/repo/  不存在和本次任务相关的文件，是否就会被LLM忽视了？  
- /memories/repo/（CAPI 模式）注入了 摘要文本，是否和上述同样的问题存在，摘要文本可能也和本次任务无关呢？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn16

用户意图：调查 CAPI 是否针对本地模式的局限设计；验证两种模式是否都存在"注入内容与当前任务无关"的问题。

已有源码和 GitHub 博客，再补充查证 CAPI 检索策略的细节。

The user has two questions:

1. Was CAPI mode designed to address limitations of local mode?
2. Their thinking about relevance filtering - both local (file list) and CAPI (summaries) might inject irrelevant information or be ignored.

For Q1, I should look at the GitHub blog post about building the agentic memory system, which I already have. Let me recall what it said about the design decisions.

From the GitHub blog:
- "One option would be to implement an offline curation service to deduplicate, resolve conflicts, track branch status, and expire stale information. At GitHub's scale, however, such an approach would introduce significant engineering complexity and LLM costs"


- They chose just-in-time verification instead

But this comparison is about offline curation versus just-in-time verification, not specifically about CAPI versus local mode. Looking at the actual design, CAPI mode (GitHub-hosted Copilot Memory) and local mode (filesystem-based Memory Tool) serve different purposes—local mode is per-developer and per-machine, while CAPI enables team-shared, cross-agent memory. The blog explicitly highlights that cross-agent memory lets agents learn from experiences across the development workflow without explicit user instructions, so CAPI was primarily designed for enabling that collaborative, persistent knowledge sharing across agents.

The user's second point is insightful: in local mode, if relevant files aren't in the file list, the LLM might skip them, though it does see all filenames and decides based on relevance. With CAPI, all memories get injected as text, which could become noise if they're not relevant to the current task—the system prompt even warns that memories might be obsolete or incorrect. Looking at the actual retrieval logic in the memory service...

Examining the memory retrieval mechanism, I see it's fetching the most recently stored memories rather than the most relevant ones to the current query—that's a meaningful constraint on how useful these memories actually are.

In local mode, the LLM has to infer relevance from filenames alone, so it might skip files that don't obviously signal their connection to the task. With CAPI, the system injects the 10 most recent memories regardless of whether they're actually relevant, and while the prompt tells the LLM to evaluate their usefulness, all 10 still consume context tokens either way.

Both approaches have relevance problems, just different ones. The blog post notes this as something to address in the future through better retrieval techniques. Looking back at Q1, CAPI solves several limitations of local mode—cross-agent sharing, cloud scalability, auto-expiry, citation verification—but it doesn't actually fix the relevance filtering issue. If anything, forcing all 10 memories into the context window might make the irrelevance problem worse than letting the LLM selectively pull files.

The two modes are mutually exclusive in the implementation: when CAPI is enabled, the local repo memory listing gets skipped entirely. They're designed to coexist for different use cases—CAPI for team and cross-agent knowledge, local mode for developer-specific or session-specific context—rather than one replacing the other. CAPI was designed to solve the core problem of local mode—the LLM not actually reading stored files—but it trades some flexibility for that. It's more collaborative across developers and has built-in safeguards like expiration and citation verification, though it's less granular and requires GitHub authentication. Each approach has tradeoffs depending on what matters most for the use case.

已在 `agentMemoryService.ts` 里看到关键代码，直接引用。

---

## Q1：CAPI 模式是否为了弥补本地模式的局限而设计？

✅ 已验证（来源：`agentMemoryService.ts` + `memoryContextPrompt.tsx` + GitHub Blog）

**是的，但不完全是。** 两者是**互斥的替代关系**（代码里是 `!enableCopilotMemory` 的分支），CAPI 针对性解决了本地模式的几个核心痛点，但也引入了新的局限。

### 本地模式的痛点 → CAPI 如何对应

| 本地模式痛点 | CAPI 的解法 |
|-------------|-----------|
| 文件内容不自动注入，LLM 经常跳过 | 直接注入摘要文本，不需要 LLM 主动读 |
| 仅限当前机器，换台电脑丢失 | 存在 GitHub 服务端，跨机器 |
| 仅当前用户可见，团队无法共享 | 跨 Agent 共享（Code Review、Coding Agent、CLI 共用） |
| 无过期机制，旧知识永远存在 | 28天自动过期，防止知识腐烂 |
| AI 写错无验证机制 | Citation 实时校验，引用代码变化则 memory 失效 |

### CAPI 引入的新局限

| 维度 | CAPI 的做法 | 产生的新问题 |
|------|-----------|------------|
| 存储格式 | 必须是 JSON（subject/fact/citations/reason） | 不能自由写 Markdown，结构化成本高 |
| 操作限制 | 仅支持 `create`，不能 edit / delete | 发现错误只能新建一条，旧条目继续污染直到过期 |
| 检索策略 | 取最近 10 条（`action: 'recent', limit: 10`） | **不按相关性排序**，与当前任务无关的也全部注入 |
| 可见性 | 只有 Repo Owner 能在 GitHub Settings 查看 | 普通开发者不知道 AI 写了什么 |

---

## Q2：两种模式都存在"注入内容与当前任务无关"的问题吗？

**两种模式都有这个问题，但表现形式不同。**

### 本地模式：相关性门槛在"文件名"

```
LLM 收到：
  /memories/repo/conventions.md
  /memories/repo/build-commands.md
  /memories/repo/database-schema.md

当前任务：修复一个 UI 渲染 bug
LLM 判断：这三个文件名看起来都不相关
结果：一个都不读 → 文件内容完全不生效
```

局限：LLM 靠文件名猜测相关性，容易误判（尤其文件名模糊时）。内容被读与否完全依赖 LLM 的主观决策，无系统保障。

### CAPI 模式：相关性问题在"注入但无用"

源码里 CAPI 的检索调用是：

```typescript
// agentMemoryService.ts
async getRepoMemories(limit: number = 10) {
    await this.capiClientService.makeRequest({
        action: 'recent',   // ← 取最近，不是取相关
        limit               // ← 默认10条
    });
}
```

**取的是最近写入的 10 条**，不是与当前任务最相关的 10 条。

```
LLM 收到（假设最近写入的）：
  [memory 1] API version 同步约定
  [memory 2] Winston 日志格式
  [memory 3] 数据库连接池配置
  ... 共10条

当前任务：修复 UI 渲染 bug
结果：10条全部注入 context，但可能全部无关 → 纯噪声
```

系统 prompt 里有一句话承认了这个问题：

> "be aware that memories might be obsolete or incorrect or **may not apply to your current task**. Consider whether any are relevant to your current task."

GitHub Blog 也坦承这是未来工作：

> "**Future implementations will enable additional retrieval techniques, such as a search tool and weighted prioritization.**"

即目前 CAPI 的相关性检索根本没有实现，是已知待解决的问题。

---

### 两种模式的相关性问题对比

| | 本地模式 | CAPI 模式 |
|--|--|--|
| 注入内容 | 文件名列表（不含内容） | 最近10条摘要文本（含内容） |
| 相关性判断者 | LLM（靠文件名猜） | 无判断（全量注入最近10条） |
| 无关内容的代价 | LLM 不读，零影响 | 占用 context token，可能引入噪声 |
| 有关内容被遗漏的代价 | LLM 不读，效果等于无 | 如果超出10条限制，被截断 |
| 相关性检索 | ❌ 无 | ❌ 无（`recent` 而非 `relevant`） |

**两种模式的相关性都是弱点**，只是弱在不同地方：本地模式靠 LLM 猜文件名，CAPI 靠时间顺序而非任务相关性。真正基于当前任务做相关性过滤的检索（类似 Cursor 的向量搜索）目前两种模式都没有实现。

Turn:1～16

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 17）

<timestamp>Thursday, May 14, 2026, 10:16 AM (UTC+8)</timestamp>
<user_query>
/memories/repo/（CAPI 模式） 和 /memories/repo/（CAPI 模式） 的 相关性弱点，有补救手段吗？ 类似：  Cursor  支持 LLM 主动发起二次检索。
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn17

用户意图：了解两种 Copilot repo memory 模式是否有类似 Cursor 二次检索的补救手段。

基于已有源码直接分析，补充查证 Memory Tool 的 view 命令能力范围。

The user is asking about whether there are remediation strategies for the relevance weaknesses in both Copilot Memory modes (local and CAPI), similar to how Cursor supports LLM-initiated secondary search.

From what I already know:
1. Local mode: LLM can call Memory Tool with `view` command on specific files
2. CAPI mode: Currently only `create` is supported, no search/query API exists yet

Let me check the memoryTool.tsx source code for any search/query functionality, and also check if there's any search capability planned or mentioned in the GitHub blog.

From what I've seen in the code:- Memory Tool commands: `create`, `str_replace`, `insert`, `delete`, `rename`, `view`


- The `view` command can list files when pointed at a directory or read content from a specific file, but there's no dedicated search functionality
- CAPI mode is even more restricted—it only allows `create` operations, rejecting everything else with an error
- However, the LLM can work around this in local mode by viewing the `/memories/repo/` directory to get a file list, then viewing specific files to read their content
- For CAPI mode, memories are only accessible through system prompt injection since the LLM can't query the store directly
- The system prompt does instruct the AI to use the Memory Tool to read specific files when it needs more detail, but this isn't true semantic search
- The GitHub blog explicitly mentions that search tools and weighted prioritization are planned for future implementations
- So right now: local mode offers limited search through file viewing, while CAPI mode offers no query capability at all
- For local mode, there are some workarounds like using descriptive filenames and maintaining an index file, but these aren't as systematic as what Cursor provides
- The honest assessment is that neither implementation currently has a proper relevance-based secondary search mechanism

, with CAPI explicitly marking this as future work

源码已验证。Memory Tool 支持的命令只有：`view`、`create`、`str_replace`、`insert`、`delete`、`rename`，**没有 `search` 命令**。

---

## 补救手段分析

### 本地模式：有有限的补救，但不是检索

LLM 可以用 Memory Tool 的 `view` 命令主动读取特定文件：

```
# 读目录 → 确认有哪些文件（已自动注入，不需要这步）
view /memories/repo/

# 读具体文件 → 获取内容
view /memories/repo/conventions.md

# 读文件的特定行范围
view /memories/repo/conventions.md  view_range=[1, 50]
```

但这是**按文件名定向读取**，不是语义检索。LLM 必须先从文件名判断"这个文件可能与任务相关"，才会去读——**相关性问题还是落在文件名猜测上**，没有根本解决。

### CAPI 模式：目前无任何补救手段

源码明确：CAPI 路径下只允许 `create`，其他命令一律报错：

```typescript
private async _dispatchRepoCAPI(params, path): Promise<MemoryToolResult> {
    switch (params.command) {
        case 'create':
            return this._repoCreate(params);
        default:
            return { text: `Error: Only 'create' is allowed for /memories/repo/.`, outcome: 'error' };
    }
}
```

LLM **无法查询、无法检索、无法主动拉取** CAPI memories。它只能接收系统自动注入的最近 10 条，没有任何二次获取的通道。

GitHub Blog 已经把这个能力列为明确的 roadmap：

> "**Future implementations will enable additional retrieval techniques, such as a search tool and weighted prioritization.**"

---

### 与 Cursor 的对比

| 能力 | Cursor | Copilot 本地 repo memory | Copilot CAPI memory |
|------|--------|--------------------------|---------------------|
| 系统预注入相关内容 | ✅（三遍 pipeline，语义相关） | ❌（只注入文件名） | ⚠️（注入最近10条，非相关） |
| LLM 主动语义检索 | ✅（`codebase_search`） | ❌ | ❌ |
| LLM 主动读指定内容 | ✅（读文件） | ✅（Memory Tool `view`） | ❌（CAPI 只支持 create） |
| 相关性过滤机制 | ✅（向量 + 依赖图 + 重排序） | ❌（靠文件名猜） | ❌（靠时间顺序） |

---

### 当前可用的缓解手段（非系统级）

**本地模式可以做的：**

- 文件命名要语义清晰（让 LLM 从文件名能准确判断相关性）
- 参考 Claude Code 的 MEMORY.md 模式：主文件做一行一条的索引摘要，LLM 先读索引再按需读详情
- 文件数量控制少而精，减少 LLM 需要判断的噪声

**CAPI 模式可以做的：**

- 实际上没有可操作的缓解手段，只能等 Microsoft 实现 search API
- 最多通过 GitHub Settings 手动删除明显无关的旧 memory，但这是事后补救

**两种模式共同的根本局限：**

都缺少"知道当前任务是什么，再去匹配最相关的 memory"这一步——而这正是 Cursor 向量检索做的事情。Memory Tool 的设计是静态注入，Cursor 的设计是动态匹配，两者是不同的架构选择。

Turn:1～17

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 18）

<timestamp>Thursday, May 14, 2026, 11:25 AM (UTC+8)</timestamp>
<user_query>
请将本次对话，提炼成话题列表给我，按照turn顺序聚合。
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn18

用户意图：将本次对话提炼为按 Turn 顺序聚合的话题列表。

---

## 本次对话话题列表

**Turn 1｜VS Copilot `/memories/` 目录设计概览**
三作用域（User / Repo / Session）的含义、持久化范围、用途定位，Memory Tool vs Copilot Memory 的区别。

**Turn 2｜User 级 memory 的三个机制细节**
① 200行是所有文件合计（非每文件）；② AI 自主写入的触发机制；③ 用户可手动添加文件，不强制依赖 Memory Tool。

**Turn 3｜`/memories/` vs `.github/` 指令文件的本质区别**
人写规范 vs AI 经验笔记；版本控制 vs 本机私有；静态声明 vs 动态积累；已知冲突 Issue #308669。

**Turn 4｜`/memories/session/` 和 `/memories/repo/` 的写入逻辑**
路由判断机制；session 按 sessionId 存本地；repo 分本地模式和 CAPI 云端模式（仅支持 create，JSON 格式）。

**Turn 5｜`/memories/session/` 的生命周期**
Chat 结束后逻辑不可见但文件物理存在；14天 + VS Code 重启后才触发清理；删除 Chat 不联动删除文件。

**Turn 6｜`/memories/repo/` vs `.github/` 的冲突与共存设计**
两套存储解决不同信息不对称；两者同时进入 system prompt，无仲裁机制；Issue #308669 记录的实际冲突。

**Turn 7｜AI 写入 `/memories/repo/` 的透明度与纠错能力**
CAPI 可在 GitHub Settings 查看/删除；citation 验证 + 28天过期 + AI 自我纠正；写入决策是黑盒，无审批流程。

**Turn 8｜同一 Spec，A/B 员工执行结果不同的问题**
User/Repo memory 因人因机器而异，导致相同输入产生不同 AI 决策；执行时上下文无快照，事后无法审计还原。

**Turn 9｜Memory Tool 的学术背景：Agent 记忆系统研究**
三维分类（时间作用域 / 表示底层 / 控制策略）；Copilot 架构对应关系；Just-in-Time Verification 的创新点；相关论文（arXiv:2603.07670）和实测数据。

**Turn 10｜MemGPT 架构介绍**
UC Berkeley Packer 等人提出；以 OS 虚拟内存分页为灵感；Main Context（三分区）+ External Context（Recall / Archival Storage）；函数调用自主管理记忆；实验结果；演化为 Letta 框架。

**Turn 11｜Cursor 和 Claude Code 是否使用 MemGPT 架构**
两者都未直接使用；Cursor 是动态上下文发现（代码库向量索引 + 按需注入）；Claude Code 是五层持久化（CLAUDE.md / Auto-Memory / 后台提取 Agent / Context Compaction / 原始 transcript）；三者共享 OS 分层内存灵感但独立实现。

**Turn 12｜Cursor 向量索引 vs Archival Storage；Copilot 文件名全量传递**
Cursor 是系统预注入（LLM 不参与检索），MemGPT 是 LLM 主动拉取；Copilot session/repo 文件名无数量上限，全量注入文件名列表；User memory 注入内容，session/repo 仅注入路径。

**Turn 13｜Cursor 没有持久化 memory 目录，是实时检索注入**
确认 Cursor 向量索引是代码库的客观镜像，不积累 AI 主观经验；无写入行为，每次提问独立检索，与 Copilot memory 的持久积累模式根本不同。

**Turn 14｜两个补充澄清**
① Copilot 本地 repo memory 若 LLM 不主动读文件则零效果；CAPI 模式内容自动注入无需主动读。
② Cursor 支持 LLM 主动二次检索（`codebase_search` + grep + Explore subagent），是系统预注入 + LLM 主动检索双轨并行。

**Turn 15｜文件列表是自动注入还是需要 AI 主动获取**
确认：session 和本地 repo 的文件名列表在每次请求组装 system prompt 时自动注入，无需 AI 主动获取；文件内容才需要 AI 二次调用 Memory Tool。

**Turn 16｜CAPI 是否为弥补本地模式局限而设计；两种模式的相关性弱点**
CAPI 解决了"LLM 不读文件"等问题但引入新局限；本地模式靠文件名猜相关性；CAPI 取最近10条（`action: 'recent'`）非相关性检索，两者均缺乏任务相关性过滤。

**Turn 17｜两种模式的相关性弱点有无补救手段**
本地模式：LLM 可用 Memory Tool `view` 定向读文件，但仍依赖文件名判断，非语义检索；CAPI 模式：`create` 以外的命令全部报错，无任何查询通道，补救需等 Microsoft 实现 search API（已列入 roadmap）。

Turn:1～18

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 19）

<timestamp>Thursday, May 14, 2026, 11:29 AM (UTC+8)</timestamp>
<user_query>
将所有上下文， 整理四个文档框架。 

1、 从MemGPT 架构介绍 开始，到 工具横向对比。
2、 cursor / VS Copilot / claude分别介绍。 

请给我四个文档的章节框架
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn19

用户意图：设计四个文档的章节框架（非生成内容，仅框架）。

---

## 四个文档章节框架

---

### Doc 1｜Agent 记忆系统：从 MemGPT 到工具横向对比

```
1. 背景：为什么 LLM 需要记忆系统
   1.1 上下文窗口的根本局限
   1.2 "无记忆"的代价：重复踩坑、无法积累

2. MemGPT：OS 分层内存模型的形式化
   2.1 提出者与论文背景（UC Berkeley，ICML 2023）
   2.2 核心思想：把 LLM 当 CPU，上下文窗口当 RAM
   2.3 三层架构
       - Main Context（System Instructions / Working Context / FIFO Queue）
       - Recall Storage（消息历史数据库）
       - Archival Storage（向量档案库）
   2.4 函数链机制：LLM 自主管理内存
   2.5 实验结果与演化（→ Letta 框架）

3. 学术分类框架（arXiv:2603.07670）
   3.1 三维分类：时间作用域 / 表示底层 / 控制策略
   3.2 五大机制家族
   3.3 代表系统时间线（RAG → Reflexion → MemGPT → Agentic Memory）

4. 工具横向对比
   4.1 架构对比矩阵（Cursor / VS Copilot / Claude Code / MemGPT）
   4.2 记忆写入：谁决定写什么
   4.3 记忆读取：系统预注入 vs LLM 主动检索
   4.4 相关性机制：向量检索 vs 时间排序 vs 文件名猜测
   4.5 跨 session / 跨用户 / 跨 Agent 能力对比
   4.6 可审计性与团队协作问题

5. 共同局限与开放问题
   5.1 相关性过滤缺失
   5.2 执行不确定性（A/B 员工问题）
   5.3 黑盒写入与审计缺口
```

---

### Doc 2｜Cursor 上下文管理机制

```
1. 设计哲学：代码库感知而非记忆积累
   1.1 Cursor 解决的核心问题（代码库远大于 context window）
   1.2 与记忆系统的本质区别：客观镜像 vs 主观经验

2. 代码库语义索引（静态层）
   2.1 索引 Pipeline：File Watching → AST 解析 → Embedding → 向量索引
   2.2 语义单元粒度：函数 / 类 / 模块，非任意 token 截断
   2.3 依赖图：向量相似度之外的关系层
   2.4 两层索引：持久化索引 + 内存 delta（实时一致性）
   2.5 隐私与安全：路径加密，代码不明文存储

3. 查询时三遍 Pipeline（系统预注入）
   3.1 Pass 1：语义相似度检索
   3.2 Pass 2：依赖图遍历
   3.3 Pass 3：时效 + 调用深度重排序
   3.4 注入格式：完整函数体 + 文件路径 + 行号

4. 动态上下文发现（Dynamic Context Discovery）
   4.1 工具输出文件化：长输出写文件，LLM 按需读取
   4.2 历史文件化：summarization 时可引用历史文件补细节
   4.3 MCP 工具懒加载：按需查找，减少 46.9% token
   4.4 终端历史文件化：AI 可 grep 终端输出

5. LLM 主动二次检索（动态层）
   5.1 Semantic Search（codebase_search）：语义搜索
   5.2 Instant Grep：精确符号匹配
   5.3 工具链式调用：semantic → grep → 文件读取
   5.4 Explore Subagent：独立 context 并行搜索，只返回摘要

6. Context 管理策略
   6.1 Summarization 触发时机
   6.2 历史文件引用机制
```

---

### Doc 3｜VS Copilot Memory Tool 深度解析

```
1. 概述与启用状态
   1.1 Memory Tool（本地）vs Copilot Memory（CAPI 云端）两套系统
   1.2 默认开启状态与配置项

2. 三作用域设计
   2.1 User 级（/memories/）：跨 session / 跨 workspace 持久化
   2.2 Session 级（/memories/session/）：对话内临时上下文
   2.3 Repository 级（/memories/repo/）：项目范围知识

3. User 级记忆机制（/memories/）
   3.1 文件内容自动注入：200行合计上限的实现逻辑
   3.2 多文件时的截断行为（按目录顺序，总行数达到200停止）
   3.3 AI 自主写入的触发机制
   3.4 用户手动添加的方式与路径

4. Session 级记忆机制（/memories/session/）
   4.1 存储路径：storageUri/{sessionId}/
   4.2 注入方式：仅文件名列表自动注入，内容需主动读取
   4.3 Plan Agent 写入 plan.md 的典型用法
   4.4 生命周期：逻辑失联 vs 物理删除（14天 + 重启清理）

5. Repository 级记忆机制（/memories/repo/）
   5.1 本地模式：文件名列表注入，内容需 Memory Tool 读取
   5.2 CAPI 模式：最近10条摘要直接注入，写入格式（JSON）
   5.3 两种模式的相关性弱点对比
   5.4 CAPI 的技术创新：Just-in-Time Verification（citation 实时校验）
   5.5 CAPI 的当前局限：仅支持 create，无 search API

6. Memory Tool 命令集
   6.1 支持的命令（view / create / str_replace / insert / delete / rename）
   6.2 路由逻辑：路径前缀决定作用域

7. /memories/ vs .github/ 指令文件
   7.1 设计定位对比
   7.2 冲突问题（Issue #308669）
   7.3 实践建议

8. 已知局限与开放问题
   8.1 A/B 员工执行不一致问题
   8.2 黑盒写入，无审计机制
   8.3 相关性检索缺失（roadmap）
```

---

### Doc 4｜Claude Code 记忆与持久化架构

```
1. 设计哲学：五层持久化，选择性而非全量保存

2. Layer 1：CLAUDE.md 指令层
   2.1 发现顺序：Managed → User → Project → Local
   2.2 优先级：Local 覆盖 Project 覆盖 User
   2.3 @include 指令与循环引用防护
   2.4 40,000字符截断上限
   2.5 Trust Boundary：项目级文件不可修改 memory 路径等安全配置

3. Layer 2：Auto-Memory 持久知识库
   3.1 存储路径：~/.claude/projects/<git-root>/memory/
   3.2 MEMORY.md 主索引：200行 / 25KB 双上限，截断算法细节
   3.3 Topic Files：按需读取，不全量注入
   3.4 四种记忆类型（user / feedback / project / reference）及 frontmatter 规范
   3.5 禁止保存的信息类型（防止 memory 成为垃圾桶）
   3.6 Path Security 验证（防止 memory 目录被重定向到 ~/.ssh 等敏感路径）
   3.7 Worktree 共享：同一 git root 的所有 worktree 共用 memory

4. Layer 3：后台记忆提取 Agent
   4.1 触发时机：主 Agent 完成任务后 fork
   4.2 工具权限隔离：只能写 memory 目录
   4.3 与主 Agent 的去重逻辑（避免重复写入）
   4.4 Feature Gate：非交互模式默认关闭

5. Layer 4：Context Compaction（上下文压缩）
   5.1 Microcompact：清理已处理的 tool result（不丢语义）
   5.2 Auto-compact 触发阈值计算公式
   5.3 Full Compaction Pipeline：fork summary agent → 生成 boundary message
   5.4 压缩的信息损失与 history 文件补救

6. Layer 5：原始 Session Transcript
   6.1 用途：为 compaction 提供完整历史引用
   6.2 保留时限

7. 与其他工具的对比
   7.1 vs MemGPT：结构最相似，但完全独立实现
   7.2 vs Cursor：静态文件 vs 动态向量索引
   7.3 vs Copilot Memory Tool：有后台自动提取 Agent，Copilot 无此机制
```

Turn:1～19

> 模型：Claude Sonnet 4.6（Anthropic）

The user wants me to organize the conversation content into four document frameworks. Let me think about what four documents would make sense:

1. From MemGPT architecture introduction to cross-tool comparison → this seems like one document
2. Cursor / VS Copilot / Claude Code separately introduced → this seems like three documents

So:
1. Doc 1: MemGPT架构 + 横向对比
2. Doc 2: Cursor 记忆系统
3. Doc 3: VS Copilot Memory Tool
4. Doc 4: Claude Code 记忆系统

Let me design chapter frameworks for each.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 20）

<timestamp>Thursday, May 14, 2026, 11:31 AM (UTC+8)</timestamp>
<user_query>
审查对话内容，哪些没有在框架里面体现，给我一个列表，列表按照文档分组。
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn20

用户意图：审查四个文档框架，找出对话中有但框架未体现的内容，按文档分组列出。

---

## 框架遗漏内容清单

---

### Doc 1｜MemGPT + 横向对比

- **POMDP 形式化**：Agent 记忆的数学表达（`aₜ = π(xₜ, R(Mₜ, xₜ), gₜ)` / `Mₜ₊₁ = U(…)`），及与 Belief State 的关系
- **MemGPT Queue Manager 细节**：70% 触发 warning、100% 触发 flush、递归摘要生成机制
- **函数链（Function Chaining）**：`request_heartbeat=true` 标记，AI 连续调用函数不回复用户
- **CAPI 实测数据**：PR 合并率 90% vs 83%（+7%），Code Review 正向反馈 77% vs 75%（+2%），p<0.00001
- **"均未直接使用 MemGPT"的定性结论**：三者独立实现，MemGPT 是学术命名而非工业集成，需作为独立结论段
- **"隐式状态依赖"类比**：A/B 员工问题类比于"环境变量泄漏进构建脚本"的确定性破坏

---

### Doc 2｜Cursor

- **MemGPT 概念映射对照表**：Working Context / Recall / Archival 与 Cursor 组件的明确映射（作为独立小节）
- **Cursor 无写入行为的设计含义**：强调"代码索引是客观镜像，不积累 AI 主观经验"需作为独立设计原则段
- **6 周无活动删除策略**：索引的保留时限与团队 index 共享机制
- **隐私实现细节**：路径加密、代码不明文存储、客户端解密流程

---

### Doc 3｜VS Copilot Memory Tool

- **`.github/` 目录的完整内容类型**：`copilot-instructions.md` / `instructions/` / `prompts/` / `agents/` / `skills/`（Turn 3 涉及，框架只提到 copilot-instructions.md）
- **System Prompt 结构全貌**：`<userMemory>` / `<sessionMemory>` / `<repoMemory>` / `<repository_memories>` / `<memoryInstructions>` 各 Tag 的位置关系
- **MemoryCleanupService 机制细节**：`markAccessed()`、mtime fallback、空目录清理逻辑
- **Issue #300804**："Copilot user memory never used"——LLM 看到文件名却经常不读的已知 Bug
- **CAPI 检索策略的具体实现**：`action: 'recent', limit: 10`，取最近而非相关，非 relevance-based
- **透明度审计表**：6 个环节（决定写什么 / 写入时机 / 写入内容 / 删除权限 / 验证可见性 / 影响路径）的黑盒/半透明/透明分级
- **A/B 员工问题的缓解手段清单**：关闭 Memory Tool / 关闭 CAPI / 写入 .github / 无法覆盖 `<memoryInstructions>` 硬编码
- **本地 repo memory 相关性缓解手段**：描述性文件名 / MEMORY.md 索引模式 / 控制文件数量

---

### Doc 4｜Claude Code

- **"为什么不全量保存"的设计推导**：全量 = context 浪费；零保存 = 规模崩溃；五层架构是中间路径的设计动机
- **mkdir 问题与解法**：早期 AI 每次浪费 turn 检查目录是否存在；`ensureMemoryDirExists()` + prompt 明确告知解法
- **Path 三步解析链**：`CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` → `settings.json` → 默认路径，及 Cowork/SDK 场景的覆盖原因
- **Worktree 共享机制**：所有 worktree 共用同一 memory 目录，feature-branch 写的 memory 在 main worktree 可见
- **Microcompact 的 tool 白名单**：`COMPACTABLE_TOOLS = {FileRead, Bash, Grep, Glob, WebSearch, WebFetch, FileEdit, FileWrite}`
- **Auto-compact 触发阈值公式**：`threshold = context_window - 20000 - 13000`，非简单 80% 百分比
- **4/3 token 估算乘数**：图片固定估算 2000 token 的处理逻辑
- **Feature Gate 多层控制**：`EXTRACT_MEMORIES` 编译期 flag + GrowthBook 运行期 gate + 节流 gate，非交互模式默认关闭
- **Plans vs Tasks vs Memory 的三分区**：系统 prompt 明确三者适用场景，防止 AI 把计划和任务列表写进 memory
- **Layer 5 保留时限**：原始 transcript 的实际保留策略（框架目前仅写"不长期保留"，细节缺失）

Turn:1～20

> 模型：Claude Sonnet 4.6（Anthropic）

The user wants me to review the conversation and identify what's missing from the four document frameworks. Let me go through each turn and check what's covered and what's not.

Turn 1: VS Copilot memories directory overview
- Three scopes explained ✅ (Doc 3, Section 2)
- Memory Tool vs Copilot Memory distinction ✅ (Doc 3, Section 1)
- User memory first 200 lines auto-loaded ✅ (Doc 3, Section 3)
- Session/repo empty initially ✅ covered

Turn 2: Three specific mechanism questions
- 200 lines is total across files ✅ (Doc 3, Section 3.2)


- AI autonomously writes ✅ (Doc 3, Section 3.3)
- Users can manually add files ✅ (Doc 3, Section 3.4)

Turn 3: .github/ vs /memories/ comparison
- Design positioning comparison ✅ (Doc 3, Section 7.1)
- .github/ directory structure with copilot-instructions.md, instructions/, prompts/, agents/, and skills/ subdirectories isn't documented in the frameworks
- Issue #308669 conflict ✅ (Doc 3, Section 7.2)
- Practice recommendations ✅ (Doc 3, Section 7.3)

Turn 4: session/ and repo/ write logic
- Routing logic ✅ (Doc 3, Section 6.2)
- Session stored in storageUri/{sessionId}/ ✅ (Doc 3, Section 4.1)
- Local repo mode ✅ (Doc 3, Section 5.1)
- CAPI mode with JSON format ✅ (Doc 3, Section 5.2)
- CAPI only supports create ✅ (Doc 3, Section 5.5)

Turn 5: /memories/session/ lifecycle
- Chat ends → logically inaccessible but physically present ✅ (Doc 3, Section 4.4)
- 14-day + restart cleanup ✅ (Doc 3, Section 4.4)
- Deleting chat doesn't delete files ✅ (Doc 3, Section 4.4)
- The actual cleanup mechanism implementation details aren't covered in the framework docs

Turn 6: /memories/repo/ vs .github/ conflict and coexistence
- These address different information gaps ✅ (Doc 3, Section 7.1)
- Both get injected into the system prompt without any conflict resolution ✅ (Doc 3, Section 7.2)
- Issue #308669 ✅
- The .github/ directory structure and system prompt injection specifics aren't explicitly detailed in the framework

Turn 7: Transparency and correction
- CAPI visibility in progress through GitHub Settings ✅ (Doc 3, Section 5.4 partially)
- Citation verification with 28-day expiry and self-correction mechanisms ✅ (Doc 3, Section 5.4)
- Writes happen without approval in a black box ✅ (Doc 3, Section 8.2)
- The decision matrix around who controls writes, timing, and visibility isn't explicitly laid out in the framework

Turn 8: A/B employee problem
- Memory varies by person and machine, leading to divergent decisions ✅ (Doc 3, Section 8.1, Doc 1, Section 4.6)
- No captured snapshot of the execution context ✅ (Doc 1, Section 4.6 and Doc 3, Section 8.1)
- This mirrors implicit state dependencies like environment variable leakage in build systems

Turn 9: Academic background
The mitigation strategies—disabling Memory Tool, disabling CAPI, writing to .github/copilot-instructions.md, and the inability to override memoryInstructions—are only partially covered in the docs. Moving to the academic framing, I'm seeing the three-dimension taxonomy, five mechanism families, system timeline, and how Copilot maps to research concepts all documented, plus GitHub's JIT verification innovation. Now I'm looking at the performance metrics comparing merge rates and code review outcomes.

Turn 10: MemGPT architecture
The POMDP formalization isn't explicitly laid out in the frameworks. For MemGPT itself, the three-part context structure is documented, but the Queue Manager and Function Executor details, memory pressure thresholds at 70%/100%, and function chaining with request_heartbeat aren't explicitly covered in the framework sections.

Turn 11: Whether Cursor/Claude Code use MemGPT
I'm checking the architecture comparison matrices to see if there's clarity on this question.

Turn 12: Cursor vector index vs Archival Storage; Copilot filename transmission
Cursor uses system pre-injection rather than MemGPT's LLM-initiated retrieval, and the LLM never participates in the actual retrieval process—it just receives ready code. Copilot transmits all session and repo filenames without limits, which creates a potential noise problem as files accumulate, though this risk isn't explicitly discussed in the framework.

Turn 13: Cursor's lack of persistent memory
Cursor's index functions as an objective mirror rather than subjective memory, with no write behavior and each query treated independently. The comparison between MemGPT's terminology and Cursor's approach isn't formally laid out in a dedicated section.

Turn 14: Copilot repo memory and Cursor's dual-track search
Copilot's auto-injection happens at the API level, while local mode LLMs often skip files entirely—a behavior documented in Issue #300804 but not explicitly covered in the framework. Cursor supports a dual-track retrieval model that handles both primary and secondary search paths.

Turn 15: Automatic file list injection
Session and local repo file lists are automatically injected into queries, whereas CAPI operates with structured memory entries rather than a traditional file concept.

Turn 16: CAPI's design tradeoffs
CAPI was built to address specific pain points in local mode retrieval, though it introduces its own set of limitations. The action parameter defaults to 'recent' with a fixed limit rather than using relevance-based ranking, which isn't explicitly detailed in the framework documentation.

Turn 17: Addressing relevance gaps across modes
Local mode can leverage view commands and naming conventions for better context discovery, while CAPI currently supports only creation operations with search functionality still on the roadmap. I'm noticing that comparison tables between Cursor, Copilot local, and Copilot CAPI—along with practical mitigation strategies like file naming patterns and memory indexing—fall outside what's documented in the frameworks.

Now I'm identifying what's missing across the documentation. Doc 1 is lacking performance metrics and POMDP formalization details.

Doc 2 needs explicit MemGPT concept mapping, clearer framing around independent query handling, the contrast with local mode capabilities, and policies around index retention and team sharing.

Doc 3 is missing coverage of the .github directory structure.

Doc 4 requires details on how the mkdir issue gets resolved, the three-step path resolution chain from environment variables through settings to defaults, and how worktree sharing actually works.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 21）

<timestamp>Thursday, May 14, 2026, 11:34 AM (UTC+8)</timestamp>
<user_query>
遗漏的内容，分为 重要 / 中等 / 一般 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn21

用户意图：对遗漏内容按重要性分级。

分级标准：**重要** = 影响读者理解核心结论或架构；**中等** = 重要细节，丢失会降低文档可信度；**一般** = 实现细节，锦上添花。

---

### Doc 1｜MemGPT + 横向对比

| 重要性 | 遗漏内容 |
|--------|---------|
| 🔴 重要 | **"均未直接使用 MemGPT"的定性结论**——这是 Doc 1 的核心论点，缺失会让读者误解关系 |
| 🔴 重要 | **"隐式状态依赖"类比**——A/B 员工问题的本质，是横向对比章节的关键洞察 |
| 🟡 中等 | **CAPI 实测数据**（PR 合并率 +7%，Code Review +2%）——支撑记忆系统有效性的唯一量化证据 |
| 🟡 中等 | **POMDP 形式化**——`aₜ = π(xₜ, R(Mₜ, xₜ), gₜ)` 公式，是 Section 3 学术分类的理论基础 |
| 🟢 一般 | **Queue Manager 细节**（70%/100% 阈值、递归摘要）——MemGPT 内部实现细节，理解架构不强依赖 |
| 🟢 一般 | **函数链 `request_heartbeat=true`**——MemGPT 的工程细节，可作为补充说明 |

---

### Doc 2｜Cursor

| 重要性 | 遗漏内容 |
|--------|---------|
| 🔴 重要 | **"无写入行为 / 每次独立检索"作为独立设计原则**——这是 Cursor 与 memory 系统的根本区别，必须在 Section 1 清晰立论 |
| 🔴 重要 | **MemGPT 概念映射对照表**——Doc 2 是三个工具文档中最需要此对比的，因为 Cursor 与 MemGPT 表面最像，实质差异最大 |
| 🟡 中等 | **隐私实现细节**（路径加密、代码不明文、客户端解密）——企业用户的关键决策因素 |
| 🟢 一般 | **6 周无活动删除策略**——运维细节，不影响理解架构 |

---

### Doc 3｜VS Copilot Memory Tool

| 重要性 | 遗漏内容 |
|--------|---------|
| 🔴 重要 | **CAPI 检索策略**（`action: 'recent', limit: 10`，非相关性）——这是 CAPI"注入但无关"问题的根因，是 Section 5.3 相关性弱点分析的核心证据 |
| 🔴 重要 | **Issue #300804**（"Copilot user memory never used"）——LLM 经常跳过本地 repo memory 的已知 Bug，是 Section 5.3 不可缺少的现实依据 |
| 🔴 重要 | **透明度审计表**（6 环节的黑/灰/白分级）——Turn 7 的核心产出，支撑 Section 8.2 黑盒写入问题 |
| 🔴 重要 | **A/B 员工缓解手段清单**——Section 8.1 若只提问题不给应对方式，实用性归零 |
| 🟡 中等 | **System Prompt 结构全貌**（各 Tag 位置关系）——理解注入顺序和优先级的必要背景 |
| 🟡 中等 | **`.github/` 目录完整内容类型**（instructions/ / prompts/ / agents/ / skills/）——Section 7 对比不完整 |
| 🟡 中等 | **本地 repo memory 相关性缓解手段**（命名规范 / MEMORY.md 索引模式 / 文件数量控制）——Section 5 的实践建议 |
| 🟢 一般 | **MemoryCleanupService 机制细节**（markAccessed / mtime fallback）——Turn 5 的源码级细节，文档中作为补充注脚即可 |

---

### Doc 4｜Claude Code

| 重要性 | 遗漏内容 |
|--------|---------|
| 🔴 重要 | **"为什么不全量保存"的设计推导**——五层架构的设计动机，是 Section 1 的必要铺垫，缺少则架构设计显得无缘由 |
| 🔴 重要 | **Plans vs Tasks vs Memory 三分区**——防止 AI 把计划写进 memory 的系统约束，是 Layer 3 后台提取 Agent 正确理解的前提 |
| 🟡 中等 | **Auto-compact 触发阈值公式**（`threshold = context_window - 20000 - 13000`）——Section 5.2 若只写"约 83%"而不给公式，失去技术严谨性 |
| 🟡 中等 | **Worktree 共享机制**（所有 worktree 共用同一 memory）——实际使用中的重要副作用，Section 3.7 需明确 |
| 🟡 中等 | **Feature Gate 多层控制**（编译期 + 运行期 + 节流 + 非交互默认关闭）——Section 4 Layer 3 若不提此点，读者不知道后台提取 Agent 的实际开启条件 |
| 🟡 中等 | **Path 三步解析链**（含 Cowork/SDK 覆盖场景）——Section 3.2 的路径安全若不给完整解析链，验证路径来源无法自洽 |
| 🟢 一般 | **mkdir 问题与解法**——工程实现趣闻，作为 Section 3 的脚注 |
| 🟢 一般 | **Microcompact tool 白名单**（8 个工具）——Section 5.1 的枚举细节 |
| 🟢 一般 | **4/3 token 估算乘数**——Section 5.2 的计算细节 |
| 🟢 一般 | **Layer 5 保留时限**——官方文档未给明确时限，当前只能标注"待查" |

Turn:1～21

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 22）

<timestamp>Thursday, May 14, 2026, 11:36 AM (UTC+8)</timestamp>
<user_query>
请基于遗漏内容，完善 框架。并标注完善的框架修改点，以及对应的遗漏内容
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn22

用户意图：在原框架基础上补入遗漏内容，并标注每个修改点对应的遗漏条目。

---

## Doc 1｜Agent 记忆系统：从 MemGPT 到工具横向对比（完善版）

```
1. 背景：为什么 LLM 需要记忆系统
   1.1 上下文窗口的根本局限
   1.2 "无记忆"的代价：重复踩坑、无法积累

2. MemGPT：OS 分层内存模型的形式化
   2.1 提出者与论文背景（UC Berkeley，ICML 2023）
   2.2 核心思想：把 LLM 当 CPU，上下文窗口当 RAM
   2.3 三层架构
       - Main Context（System Instructions / Working Context / FIFO Queue）
       - Recall Storage（消息历史数据库）
       - Archival Storage（向量档案库）
   2.4 函数链机制：LLM 自主管理内存
       [+新增] request_heartbeat=true 标记与连续函数调用行为
       ← 遗漏：函数链（Function Chaining）
   2.5 [↑扩展] Queue Manager 细节：70%/100% 阈值、递归摘要生成
       ← 遗漏：Queue Manager 细节
   2.6 实验结果与演化（→ Letta 框架）

3. 学术分类框架（arXiv:2603.07670）
   3.1 [+新增] 形式化定义：Agent 记忆作为 POMDP Belief State
       - aₜ = π(xₜ, R(Mₜ, xₜ), gₜ) / Mₜ₊₁ = U(Mₜ, xₜ, aₜ, oₜ, rₜ)
       ← 遗漏：POMDP 形式化
   3.2 三维分类：时间作用域 / 表示底层 / 控制策略
   3.3 五大机制家族
   3.4 代表系统时间线（RAG → Reflexion → MemGPT → Agentic Memory）

4. 工具横向对比
   4.1 架构对比矩阵（Cursor / VS Copilot / Claude Code / MemGPT）
   4.2 记忆写入：谁决定写什么
   4.3 记忆读取：系统预注入 vs LLM 主动检索
   4.4 相关性机制：向量检索 vs 时间排序 vs 文件名猜测
   4.5 跨 session / 跨用户 / 跨 Agent 能力对比
   4.6 [↑扩展] 可审计性与团队协作问题
       [+新增] 执行确定性破坏的"隐式状态依赖"类比：
               环境变量泄漏进构建脚本
       ← 遗漏："隐式状态依赖"类比

5. [+新增] 记忆系统有效性的量化证据
   - CAPI 实测：PR 合并率 90% vs 83%（+7%，p<0.00001）
   - CAPI 实测：Code Review 正向反馈 77% vs 75%（+2%）
   ← 遗漏：CAPI 实测数据

6. [+新增] 核心结论：三者独立实现，MemGPT 是学术命名
   - 无一产品直接集成 MemGPT 代码库
   - MemGPT 的贡献是"形式化"，各产品基于同一 OS 分层灵感独立设计
   ← 遗漏："均未直接使用 MemGPT"的定性结论

7. 共同局限与开放问题
   7.1 相关性过滤缺失
   7.2 执行不确定性（A/B 员工问题）
   7.3 黑盒写入与审计缺口
```

---

## Doc 2｜Cursor 上下文管理机制（完善版）

```
1. 设计哲学：代码库感知而非记忆积累
   1.1 Cursor 解决的核心问题（代码库远大于 context window）
   1.2 [↑扩展] 与记忆系统的本质区别：三个维度
       [+新增] ① 无写入行为：索引是客观镜像，不积累 AI 主观经验
       [+新增] ② 每次独立检索：无跨 session 的经验积累
       [+新增] ③ LLM 是被动接收者，不是检索发起者（预注入阶段）
       ← 遗漏："无写入行为 / 每次独立检索"作为独立设计原则

2. 代码库语义索引（静态层）
   2.1 索引 Pipeline：File Watching → AST 解析 → Embedding → 向量索引
   2.2 语义单元粒度：函数 / 类 / 模块，非任意 token 截断
   2.3 依赖图：向量相似度之外的关系层
   2.4 两层索引：持久化索引 + 内存 delta（实时一致性）
   2.5 [↑扩展] 隐私与安全
       [+新增] 路径加密、代码不明文存储、检索时客户端解密
       [+新增] 6 周无活动自动删除；团队 index 共享机制
       ← 遗漏：隐私实现细节 / 6 周删除策略

3. 查询时三遍 Pipeline（系统预注入）
   3.1 Pass 1：语义相似度检索
   3.2 Pass 2：依赖图遍历
   3.3 Pass 3：时效 + 调用深度重排序
   3.4 注入格式：完整函数体 + 文件路径 + 行号

4. 动态上下文发现（Dynamic Context Discovery）
   4.1 工具输出文件化：长输出写文件，LLM 按需读取
   4.2 历史文件化：summarization 时可引用历史文件补细节
   4.3 MCP 工具懒加载：按需查找，减少 46.9% token
   4.4 终端历史文件化：AI 可 grep 终端输出

5. LLM 主动二次检索（动态层）
   5.1 Semantic Search（codebase_search）：语义搜索
   5.2 Instant Grep：精确符号匹配
   5.3 工具链式调用：semantic → grep → 文件读取
   5.4 Explore Subagent：独立 context 并行搜索，只返回摘要

6. [+新增] 与 MemGPT 概念映射对照表
   - Working Context   ↔  无对应（Cursor 无 AI 写入的持久笔记）
   - Recall Storage    ↔  历史文件化（summarization 时引用）
   - Archival Storage  ↔  代码库向量索引（但检索由系统发起，非 LLM）
   - 函数调用检索      ↔  codebase_search / grep（LLM 主动层）
   ← 遗漏：MemGPT 概念映射对照表

7. Context 管理策略
   7.1 Summarization 触发时机
   7.2 历史文件引用机制
```

---

## Doc 3｜VS Copilot Memory Tool 深度解析（完善版）

```
1. 概述与启用状态
   1.1 Memory Tool（本地）vs Copilot Memory（CAPI 云端）两套系统
   1.2 默认开启状态与配置项

2. 三作用域设计
   2.1 User 级（/memories/）：跨 session / 跨 workspace 持久化
   2.2 Session 级（/memories/session/）：对话内临时上下文
   2.3 Repository 级（/memories/repo/）：项目范围知识

3. [+新增] System Prompt 注入结构全貌
   - <userMemory>：User memory 内容（前200行）
   - <sessionMemory>：Session 文件名列表
   - <repoMemory>：本地 repo 文件名列表（仅本地模式）
   - <repository_memories>：CAPI 摘要文本（仅 CAPI 模式）
   - <memoryInstructions>：硬编码的 Memory Tool 使用规则
   - 两者互斥：CAPI 开启时 <repoMemory> 不注入
   ← 遗漏：System Prompt 结构全貌

4. User 级记忆机制（/memories/）
   4.1 文件内容自动注入：200行合计上限的实现逻辑
   4.2 多文件时的截断行为（按目录顺序，总行数达到200停止）
   4.3 AI 自主写入的触发机制
   4.4 用户手动添加的方式与路径

5. Session 级记忆机制（/memories/session/）
   5.1 存储路径：storageUri/{sessionId}/
   5.2 注入方式：仅文件名列表自动注入，内容需主动读取
   5.3 Plan Agent 写入 plan.md 的典型用法
   5.4 生命周期：逻辑失联 vs 物理删除（14天 + 重启清理）
       [+新增] MemoryCleanupService：markAccessed()、mtime fallback、
               空目录清理逻辑
       ← 遗漏：MemoryCleanupService 机制细节

6. Repository 级记忆机制（/memories/repo/）
   6.1 本地模式：文件名列表注入，内容需 Memory Tool 读取
       [+新增] Issue #300804："Copilot user memory never used"
               ——LLM 经常看到文件名但不去读的已知问题
       ← 遗漏：Issue #300804
   6.2 CAPI 模式：最近10条摘要直接注入，写入格式（JSON）
       [+新增] 检索策略：action: 'recent', limit: 10（非 relevance-based）
       ← 遗漏：CAPI 检索策略具体实现
   6.3 [↑扩展] 两种模式的相关性弱点对比
       [+新增] 本地：文件名猜测，易误判；内容若不读零影响
       [+新增] CAPI：时间顺序而非任务相关，全量注入可能全部无关
       ← 遗漏：两种模式相关性对比表
   6.4 CAPI 的技术创新：Just-in-Time Verification（citation 实时校验）
   6.5 CAPI 的当前局限：仅支持 create，无 search API（已列 roadmap）
   6.6 [+新增] 相关性弱点的缓解手段
       - 本地模式：语义文件命名 / MEMORY.md 索引模式 / 控制文件数量
       - CAPI 模式：暂无系统级手段，手动在 GitHub Settings 删除旧条目
       ← 遗漏：本地 repo memory 相关性缓解手段

7. Memory Tool 命令集
   7.1 支持的命令（view / create / str_replace / insert / delete / rename）
   7.2 路由逻辑：路径前缀决定作用域

8. /memories/ vs .github/ 指令文件
   8.1 [↑扩展] .github/ 目录完整内容类型
       [+新增] copilot-instructions.md / instructions/ / prompts/
               / agents/ / skills/ 各类型的定位
       ← 遗漏：.github/ 目录完整内容类型
   8.2 冲突问题（Issue #308669）及 <memoryInstructions> 硬编码无法覆盖
   8.3 实践建议

9. [↑扩展] 已知局限与开放问题
   9.1 [↑扩展] A/B 员工执行不一致问题
       [+新增] 透明度审计表（6 环节：写入决策 / 写入时机 / 内容可查性
               / 删除权限 / 验证可见性 / 影响路径的黑/灰/白分级）
       ← 遗漏：透明度审计表
       [+新增] 缓解手段清单：关闭 Memory Tool / 关闭 CAPI /
               规范写入 .github / 无法覆盖硬编码 memoryInstructions
       ← 遗漏：A/B 员工问题缓解手段清单
   9.2 黑盒写入，无审计机制
   9.3 相关性检索缺失（roadmap）
```

---

## Doc 4｜Claude Code 记忆与持久化架构（完善版）

```
1. [↑扩展] 设计哲学：五层持久化，选择性而非全量保存
   [+新增] 为什么不全量保存：context 浪费 vs 规模崩溃的权衡推导
   [+新增] 为什么不零保存：不可再生的隐性知识（架构决策、放弃的方案）
   [+新增] 五层架构是"中间路径"的设计动机
   ← 遗漏："为什么不全量保存"的设计推导

2. Layer 1：CLAUDE.md 指令层
   2.1 发现顺序：Managed → User → Project → Local
   2.2 优先级：Local 覆盖 Project 覆盖 User
   2.3 @include 指令与循环引用防护
   2.4 40,000字符截断上限
   2.5 Trust Boundary：项目级文件不可修改 memory 路径等安全配置

3. Layer 2：Auto-Memory 持久知识库
   3.1 [↑扩展] 存储路径解析：三步优先链
       [+新增] CLAUDE_COWORK_MEMORY_PATH_OVERRIDE（Cowork/SDK 场景）
               → settings.json（用户级配置）
               → 默认 ~/.claude/projects/<git-root>/memory/
       ← 遗漏：Path 三步解析链
   3.2 MEMORY.md 主索引：200行 / 25KB 双上限，截断算法细节
       [+新增] mkdir 问题：早期 AI 每次浪费 turn 检查目录；
               ensureMemoryDirExists() + prompt 明确告知解法
       ← 遗漏：mkdir 问题与解法
   3.3 Topic Files：按需读取，不全量注入
   3.4 四种记忆类型（user / feedback / project / reference）及 frontmatter 规范
   3.5 禁止保存的信息类型（防止 memory 成为垃圾桶）
   3.6 Path Security 验证（防止 memory 目录被重定向到 ~/.ssh 等敏感路径）
   3.7 [↑扩展] Worktree 共享
       [+新增] 所有 worktree 共用同一 memory 目录（以 git root 为 key）
       [+新增] 副作用：feature-branch 写的 memory 在 main worktree 可见
       ← 遗漏：Worktree 共享机制

4. Layer 3：后台记忆提取 Agent
   4.1 触发时机：主 Agent 完成任务后 fork（无工具调用的最终响应）
   4.2 工具权限隔离：只能写 memory 目录
   4.3 与主 Agent 的去重逻辑（避免重复写入）
   4.4 [↑扩展] Feature Gate 多层控制
       [+新增] 编译期 EXTRACT_MEMORIES flag
               + GrowthBook 运行期 gate（tengu_passport_quail）
               + 节流 gate（tengu_bramble_lintel）
               + 非交互模式（SDK/CI）默认关闭
       ← 遗漏：Feature Gate 多层控制
   4.5 [+新增] Plans vs Tasks vs Memory 三分区
       - Plans：实现前对齐方案，session 作用域
       - Tasks：追踪当前对话进度，session 作用域
       - Memory：仅存未来对话有用的知识
       系统 prompt 明确禁止把计划和任务列表写进 memory
       ← 遗漏：Plans vs Tasks vs Memory 三分区

5. Layer 4：Context Compaction（上下文压缩）
   5.1 [↑扩展] Microcompact：清理已处理的 tool result（不丢语义）
       [+新增] 白名单：FileRead / Bash / Grep / Glob / WebSearch /
               WebFetch / FileEdit / FileWrite
       ← 遗漏：Microcompact tool 白名单
   5.2 [↑扩展] Auto-compact 触发阈值
       [+新增] 精确公式：threshold = context_window - 20000 - 13000
       [+新增] CLAUDE_AUTOCOMPACT_PCT_OVERRIDE 环境变量覆盖
       [+新增] token 估算：4/3 padding 乘数，图片固定 2000 token
       ← 遗漏：Auto-compact 触发阈值公式 / 4/3 估算乘数
   5.3 Full Compaction Pipeline：fork summary agent → 生成 boundary message
   5.4 压缩的信息损失与 history 文件补救

6. Layer 5：原始 Session Transcript
   6.1 用途：为 compaction 提供完整历史引用
   6.2 [↑扩展] 保留时限
       [+新增] ⚠️ 官方文档未给出明确时限，标注为"待查"
       ← 遗漏：Layer 5 保留时限（原框架仅写"不长期保留"）

7. 与其他工具的对比
   7.1 vs MemGPT：结构最相似，但完全独立实现
   7.2 vs Cursor：静态文件 vs 动态向量索引
   7.3 vs Copilot Memory Tool：有后台自动提取 Agent，Copilot 无此机制
```

Turn:1～22

> 模型：Claude Sonnet 4.6（Anthropic）

The user wants me to update the four document frameworks to include the missing content (especially the important and medium ones), and mark where changes were made and what missing content they correspond to. Let me go through each document and add the missing items.

I'll mark changes with indicators like [+新增] or [↑扩展] to show what was modified.

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 23）

<timestamp>Thursday, May 14, 2026, 11:40 AM (UTC+8)</timestamp>
<user_query>
请按照框架，生成四个文档。 放到 .cache 目录。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn23

用户意图：按完善后的框架，生成四个完整文档写入 `.cache` 目录。

The user wants me to generate four documents based on the finalized frameworks and save them to the `.cache` directory. Let me create all four documents with full content based on our conversation.

Files to create:
1. `.cache/agent-memory-system-memgpt-comparison.md`
2. `.cache/cursor-context-management.md`
3. `.cache/vscode-copilot-memory-tool.md`
4. `.cache/claude-code-memory-architecture.md`

Let me think about the titles based on the framework:
1. Doc 1: Agent 记忆系统：从 MemGPT 到工具横向对比


2. Doc 2: Cursor 上下文管理机制
3. Doc 3: VS Copilot Memory Tool 深度解析
4. Doc 4: Claude Code 记忆与持久化架构

Now I'll write the full content for each document based on all the information from this conversation, making sure to cover the key frameworks and comparisons we've discussed.

四个文档已生成到 `.cache` 目录：

| 文件 | 内容 |
|------|------|
| `agent-memory-system-memgpt-comparison.md` | MemGPT 架构、学术分类框架、工具横向对比、量化证据、核心结论 |
| `cursor-context-management.md` | 语义索引 pipeline、三遍检索、动态上下文发现、LLM 主动二次检索、MemGPT 对照表 |
| `vscode-copilot-memory-tool.md` | 三作用域机制、System Prompt 注入结构、CAPI vs 本地模式、透明度审计表、A/B 员工问题缓解手段 |
| `claude-code-memory-architecture.md` | 五层持久化设计动机、CLAUDE.md、Auto-Memory（含双上限算法）、后台提取 Agent、Context Compaction 精确公式 |

Turn:1～23

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 24）

<timestamp>Thursday, May 14, 2026, 11:51 AM (UTC+8)</timestamp>
<user_query>
将文档提交到：https://github.com/lulufoo/ai-software-dev/agent-memory-system  目录
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

Turn24

用户意图：将四个文档提交到 GitHub 仓库 `lulufoo/ai-software-dev` 的 `agent-memory-system` 目录。

The user wants to commit the four documents to a GitHub repository. Let me check the repository structure and then do the necessary git operations. But wait - this is a different repository than the current workspace. The current workspace is `/Users/lulu/Code/android-dev-docs` which is a git repo at that path. The user wants to commit to `https://github.com/lulufoo/ai-software-dev` in the `agent-memory-system` directory.

I need to:
1. Check if the `ai-software-dev` repo exists locally
2. If not, clone it or find it
3. Copy the files to the right directory


4. Commit and push

Let me first check if this repo exists locally.

已完成。四个文档已提交并推送到：

**[github.com/lulufoo/ai-software-dev](https://github.com/lulufoo/ai-software-dev)** → `agent-memory-system/` 目录

提交：`057db86`，共新增 1344 行，4 个文件。

Turn:1～24

> 模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 25）

<user_query>
执行：dtd_normalize  ，主题：Agent 记忆系统：Curor / Copilot / Claude Code 调研分析
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

_(无文本回复)_

<!-- DDM:TURN_SEP:v1 -->
