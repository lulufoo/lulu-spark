# LuLu Workbench C/S 模式重构方案讨论 — 总结

> 创建时间：2026年5月18日 09:18
> 来源：summary · 归档模式 dtd_archive_summary
> 导航：[distilled](../../../distilled/ai-software-dev/lulu-workbench-cs-refactor/202605180918-lulu-workbench-cs-refactor-discussion.md) · [digest](../../../digest/ai-software-dev/lulu-workbench-cs-refactor/202605180918-lulu-workbench-cs-refactor-discussion.md) · [trace](../../../trace/ai-software-dev/lulu-workbench-cs-refactor/202605180918-lulu-workbench-cs-refactor-discussion.md)
> 对话范围：Turn 1～14
> 主题标签：LuLu Workbench · C/S 重构 · Electron · Cursor Skill · MCP

---

## 一、讨论背景与目标

**现状**

- LuLu Workbench 是个人知识管道 **层2 中枢**：`raw/` → 标注/加工 → 沉淀到层3 知识库。
- 当前形态：**浏览器 + `python3 server.py`（~2500 行）+ 静态 Web UI（`index.html` + `js/`）**。
- 依赖：本地 Markdown/git、`gh` CLI、Meilisearch、硬编码路径（如知识库根目录）。
- **层1 加工**在 **Cursor Skill**（如 DDM）中完成，Skill 通过 `archive_root` 写入仓库。

**重构方向（讨论中逐步形成）**

1. 做成 **Mac 桌面客户端**（优先考虑 Electron）。
2. **去掉 Python 后端**，逻辑迁入客户端主进程（TypeScript）。
3. 后续可能 **接入 LLM**，但与 Cursor Skill 分工需明确。
4. **Cursor 仍负责对话与 Skill 执行**；与 App 的协作希望 **不用「文件系统当邮箱」**，改为 **本地服务 / MCP 直接通信**。

---

## 二、现状架构（讨论共识）

```text
层1  Cursor + Skills（DDM / theme-line …）
         │  写入 / 约定
         ▼
层2  lulu-workbench（raw、distilled、annotations、index.json）
         │  settle / 搜索
         ▼
层3  多个 GitHub 知识库（本地 clone + Meilisearch）
```

- 工作台 **Local-first**，数据以 Markdown + `index.json` 为主。
- 前端约 **35 个 `/api/*`** 由 `server.py` 提供；UI 与 API 耦合清晰，**适合迁移而非推倒重做 UI**。

---

## 三、客户端化：难度与工作量（结论）

| 范围 | 难度 | 粗估 |
|------|------|------|
| 仅 Electron 壳 + 内嵌现有 Web UI | 低～中 | 数天～2 周 |
| 去掉 Python，API 迁入主进程 | 中～中高 | 约 4～8 周 |
| 完整产品（配置、Meili 打包、GitHub REST 替代 `gh`、签名分发） | 中高 | 1～2 月+ |

**主要工作量不在 UI**，而在：`server.py` 移植、git/`gh`/Meilisearch、后台 reindex、路径可配置化。

**真正的产品分界**：是否与 Cursor **重复做完整 Agent/DDM**；轻量 LLM（摘要、RAG 问答）可放 App，重蒸馏仍放 Cursor Skill。

---

## 四、技术栈选型（结论）

### 4.1 桌面壳

| 方案 | 建议场景 |
|------|----------|
| **Electron + TypeScript** | 要尽快甩掉 Python、与现有 JS 栈统一、接受包体较大 |
| **Tauri 2 + Rust** | 长期 Mac 产品、要小体积；可先 loopback 保留 `/api/*` 再逐步 `invoke` |
| **Swift 原生** | 性价比低（前端虽可 WebView，后端需重写，与现有资产不匹配） |

**讨论倾向**：务实路径 **Electron + TS**；若强调 Mac 质感与包体，**Tauri** 为备选。

### 4.2 Electron 是什么（共识）

- **不是**「薄薄一层 WebView」，而是 **Chromium（渲染 UI）+ Node 主进程（本机能力）+ 桌面 API**。
- **开源**：GitHub `electron/electron`，MIT，源于 Atom，现由 **OpenJS Foundation** 托管；使用度很高（VS Code、Slack 等）。

### 4.3 后端迁移（替代 `server.py`）

- 主进程实现原 `/api/*`；**GitHub** 长期建议 REST/Octokit + Keychain；**Meilisearch** 继续 sidecar。

### 4.4 后续 LLM

- **Electron 不阻碍接 LLM**；**Cursor + Skill** 负责重蒸馏；**App 内 LLM** 可选做 RAG/摘要。

---

## 五、Cursor Skill 与 Electron 的协作（讨论演进）

| 环节 | 位置 |
|------|------|
| 聊天、推理、执行 DDM | **仅在 Cursor** |
| 浏览、标注、搜索、settle | **Electron** |

**协作方式演进**：从文件系统总线 → 倾向 **Electron 本地 HTTP / MCP `workbench_submit`**，由 App 统一落盘并刷新 UI；Skill 与 App 之间不以「互相盯文件夹」为契约。

**澄清**：蒸馏 **不在 Electron 发起执行**；可选按钮仅为跳转 Cursor。主路径为 Cursor 跑 Skill → 提交 payload 或（过渡期）写盘。

---

## 六、推荐 C/S 架构（讨论终点）

```text
Electron App（UI + Main API + 可选 MCP）
         ▲
         │ HTTP / MCP tools/call
Cursor · Agent · DDM / Skills
```

- 本地服务：`127.0.0.1` + `/api/ingest` 或 MCP `workbench_submit`。
- **MCP 形态 A**：Cursor spawn `workbench-mcp.js` 转发 API；**形态 B**：Electron Main 内嵌 SSE。

---

## 七、分阶段实施建议（汇总）

| 阶段 | 内容 |
|------|------|
| **P0** | Electron 壳 + 核心 API 移植 |
| **P1** | 路径配置；Meilisearch sidecar |
| **P2** | GitHub REST；下线 Python |
| **P3** | `/api/ingest` + MCP；DDM 改为 submit 契约 |
| **P4（可选）** | App 内轻量 LLM + RAG |

---

## 八、风险与约束（需记住）

1. **App 需常开**（MCP/HTTP 模式）。
2. **协议版本**与 schema 维护。
3. **大包体**与超时。
4. **双写过渡期**避免 Skill 写盘与 ingest 冲突。
5. **Electron 包体/内存** vs Tauri。

---

## 九、尚未拍板、可后续细化的事项

- [ ] Electron 还是 Tauri
- [ ] MCP 形态 A 或 B
- [ ] 端口与 Cursor `mcp.json` 自动配置
- [ ] `workbench_submit` JSON schema
- [ ] 过渡期是否保留 Skill 直接写盘
- [ ] App 内 LLM 边界

**本次会话新增**：`dtd_archive_summary` 模式，将讨论总结（如本文）归档为 `raw/`，`entry_kind: summary`。

---

## 十、一句话结论

**LuLu Workbench C/S 重构** = **Electron（或 Tauri）主进程承载原 Python API** + 现有 Web 前端；**Cursor 仍负责对话与 Skill**，通过 **本地 HTTP / MCP submit** 与 App 协作，摆脱文件系统邮箱；持久化仍为 Markdown/git，但 **跨进程契约改为 API**。总结类文本用 **`dtd_archive_summary`** 归档，不用 `dtd_normalize`。
