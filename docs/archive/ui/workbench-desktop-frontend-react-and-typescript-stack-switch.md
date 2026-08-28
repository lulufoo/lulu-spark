# Workbench 桌面前端：React 与 TypeScript 技术栈切换

官方参考：[Vite | Tauri](https://v2.tauri.app/start/frontend/vite/)

关联 todo：`task_95c6c41562a4`

决策来源：本会话调研 + 方案 + `/converge`（Q1 已锁 TypeScript；Q2 授权 P0 **未锁**）。

---

## 1. 目标

把桌面 L2 从「静态 HTML + vanilla ESM」换成 **React + TypeScript**。路径是绞杀：新壳包住旧页，一页一换，应用始终能开。

不改 L3 invoke map，不改 Rust，不与 Android 共用 UI。

---

## 2. 已锁决定

| 项 | 决定 | 标记 |
|----|------|------|
| 终点 | 整站 React | 用户授权写方案 |
| 语言 | 这一轮上 TypeScript | `/converge` Q1 = B |
| 路径 | 绞杀，不是停机重写 | 方案默认；Q2 未改 |
| 打包 | Vite；产出 `frontend/dist` | 方案默认；见官方 Vite+Tauri |
| 路由 | 留下现有 `#` / `parseHash`；不上 react-router | 方案默认 |
| 状态 | 留下 `host/state.js`，React 用 `useSyncExternalStore` 订阅 | 方案默认 |
| 样式 | 留下 `frontend/app.css` | 方案默认 |
| Host | `frontend/js/host/` 等到 P8 再搬进 `src/host/` | 方案默认 |
| P0 落地 | 已授权并开工 | 用户 2026-08-28：开始落地 / 继续直到完成 |

同一轮不上：Redux / Zustand / React Query、Tailwind / CSS-in-JS、react-router。

---

## 3. 现状（2026-08-28 绞杀后重核）

| 项 | 现状 | 标记 |
|----|------|------|
| 打包器 / React / TS | Vite + React + TypeScript；入口 `frontend/src/main.tsx` | ✅ Verified（`package.json`；`frontend/src/main.tsx`） |
| Tauri 前端 | `devUrl: http://localhost:5173`；`frontendDist: "../frontend/dist"` | ✅ Verified（`src-tauri/tauri.conf.json`） |
| 入口 HTML | `index.html` 只留 `#root`；module 指向 `src/main.tsx`；marked 走 npm；mermaid / qrcode 仍在 `vendor/` | ✅ Verified（`frontend/index.html` 16 行） |
| 壳 | `App` 画 `Shell`（header + 对话框）和 `HashRouter`（五个页面槽） | ✅ Verified（`frontend/src/App.tsx`；`shell.tsx`；`hash-router.tsx`；`shell-pages.tsx`） |
| 对话框 | Settings / Bind / Skills / Commit / Comment / Move / Settle / Todo / Convert / Read Later 等由 `Shell` 挂组件，不再内联静态盒子 | ✅ Verified（`frontend/src/shell.tsx` 的 `<XxxDialog />`） |
| vanilla 目录 | `frontend/js/` 已不存在；`frontend/src` 无 `.js` | ✅ Verified（`test -d frontend/js`；`find frontend/src -name '*.js'`） |
| 源码规模 | 125 个 `.ts`/`.tsx`，22,208 行；53 个文件仍有 `@ts-nocheck` | ✅ Verified（`find` + `rg` + `wc`，2026-08-28） |
| CSS | `app.css` 7,410 行 | ✅ Verified（`wc -l`） |
| 测试 | 126 个 vitest 文件 | ✅ Verified（`find tests -name '*.test.js'`） |
| 路由 | 仍是 `#` / `parseHash`：home / workbench / corpus-doc / read-later / todo-tasks。`HashRouter` 按路由显隐五个槽；`routes.ts` 的 `mount*` 仍往这些槽里挂岛屿 | ✅ Verified（`frontend/src/router/index.ts`；`hash-router.tsx`；`app-shell/routes.ts`） |
| 状态 | `host/state.ts` 仍是可变对象；已有 `useHostState` / `notifyState`。对话框另用 `createModuleStore` | ✅ Verified（`frontend/src/host/state.ts`；`shared/module-store.ts`） |
| Host | 已在 `frontend/src/host/`（不再等 P8 从 `js/host` 搬） | ✅ Verified（`frontend/src/host/`） |
| 分层 | L2 只经 L3 | ✅ Verified（`docs/architecture/arch-layer-constraints.md`） |
| Tauri import | `frontend/src/**` 禁止静态 `@tauri-apps/*`，仅 `apiClient` 可动态 import | ✅ Verified（`docs/coding/coding-workbench-discipline.md`） |

还没做完的绞杀（页面仍是嵌套 `createRoot` 岛屿，不是路由树里的子组件）：

- Home / Todos / Corpus 仍由 `routes.ts` 调用 `mountHomeHub` / `mountTodoTaskSplit` / `mountCorpusDocList`
- Notes 阅读器 / 侧栏仍改 `ShellPages` 里的 DOM id
- `boot.ts` 仍负责拉 index、注册 hash handler、挂 home-entry overlay
- `@ts-nocheck` 尚未逐文件拆掉

✅ Verified（本轮 `tsc`、vitest 123/126 文件、浏览器 `#/home` Settings / Bind / `#/todo-tasks` / `#/workbench`）

---

## 4. 目标模型

### 4.1 运行时

```text
frontend/index.html     # 只剩 #root
frontend/src/main.tsx   # createRoot → App → boot.ts
frontend/src/App.tsx    # Shell（header + 对话框）+ HashRouter（页面槽）
frontend/src/host/      # 已从 js/host 搬来
frontend/dist/          # Vite 产出；Tauri frontendDist 指向这里
```

✅ Verified（`frontend/index.html`；`frontend/src/main.tsx`；`test -d frontend/js` 为否）

⚠️ Inferred：Vite 配置放仓库根、`root`/`outDir` 指向 `frontend/`，与现有「`package.json` 在仓库根」一致。官方示例的 `frontendDist` 是 `../dist`；本仓库应对 `../frontend/dist`。

### 4.2 Tauri 接 Vite

按官方字段改 `src-tauri/tauri.conf.json`：

- `beforeDevCommand`：`npm run dev`
- `beforeBuildCommand`：`npm run build`
- `devUrl`：与 Vite `strictPort` 一致（官方示例 `http://localhost:5173`）
- `frontendDist`：`../frontend/dist`

Vite 须 `clearScreen: false`，`watch.ignored` 含 `**/src-tauri/**`。

✅ Verified（官方文档 [Vite \| Tauri](https://v2.tauri.app/start/frontend/vite/)）

### 4.3 绞杀岛屿

P1 起：React 负责 header 与路由切换。每个尚未迁完的页面是一个空 `div`，仍调用现有 `mount*`（含已抽出的 `app-shell/routes.js`）。迁完一页，删掉该页的 vanilla 挂载。

### 4.4 什么留下、什么换成 TSX

| 留下（可 import，P8 再搬） | 迁到哪一页才换成 TSX |
|---------------------------|----------------------|
| `host/api*`、`apiClient`、invoke map | 碰 DOM 的阅读器、列表、对话框、壳 |
| `router/index.js` 的 parse / navigate | `todo-task/detail-render.js` 等拼 HTML 字符串的渲染 |
| `state.js`、FSM、format、comment-reorder / comment-markdown | |

L2 仍只经 L3。`frontend/src/**` 同样禁止静态 `@tauri-apps/*`；门闩在 P0 起把扫描根扩到 `frontend/src`。

---

## 5. 阶段

后续收岛与块内分层以 `docs/archive/ui/workbench-desktop-l2-in-block-layering-and-page-lift-tech-plan.md` 为准。下表 P3–P8 是栈切换当时的阶段划分，不再当实施顺序。✅ Verified（`3e09485` 已做完对话框与五个槽；vanilla `frontend/js/` 已不存在）

每一阶段结束应用都能开。迁哪一页，就改哪一页的源码扫描测试。

| 阶段 | 做什么 | 完成条件 |
|------|--------|----------|
| P0 | Vite + React + TypeScript + Tauri hooks。界面仍是旧 JS。 | ✅ 管道已接；`vite build` 通过。❌ 本机未开 `tauri dev` 窗口 |
| P1 | React 管 hash 路由；旧页当岛屿。Header 仍在 `index.html`（源码门闩锁着这些 id） | ✅ `mountHashRouter`；5 条路由仍走 `wrapRouteMount` |
| P2 | Toast 改 React（`showToast` 合同不变）。Convert/Bind 对话框仍 vanilla，放到 P7 | ✅ `toast.test.js` 通过 |
| P3 | Home + Read Later | hub、覆盖层、Read Later 列表 |
| P4 | Todos（page / list / detail 的渲染改 TSX；host / lifecycle / binding 留下） | 列表、详情、创建、绑定 |
| P5 | Notes（阅读器、评论、侧栏、搜索、创建会话） | 打开 / 编辑 / 评论 / 新建 / 返回 |
| P6 | Corpus（复用 P5 组件，换 API 动词） | 打开知识库文档 + 评论 |
| P7 | Settings 与剩余对话框 | 设置能保存；绑定 / 转换 / 提交可用 |
| P8 | 删除 vanilla；`index.html` 只留 `#root`；marked 改 npm；CSP 去掉 jsDelivr | `frontend/js/` 不存在；`npm test` 全绿 |

P0 **须再次授权**后再做。

P5 的 Mermaid / 高亮 overlay 仍放在 `useEffect` 里，不改成组件树。

⚠️ Inferred：`apiClient` 里「外置浏览器看 1430 端口」的判断，接 Vite 后可能要对齐 `devUrl` 端口。P0 核。

---

## 6. 与并行拆分的关系

当前工作区正在把胖文件拆成模块。React 迁入时：

1. **对接拆后模块**，不要把 `host/api/`、`routes.js`、`*-render.js` 合并回去。
2. 未完成的拆分可以继续；P0 只动管道，不碰这些文件。
3. 某一页开始改 TSX 时，以当时仓库里的模块边界为准，不以本文件第 3 节的行数为冻结合同。

---

## 7. 验收（整站）

1. 桌面 UI 入口是 `frontend/src/main.tsx`，无 vanilla `js/main.js`。
2. L3 仍只经 `apiClient` + invoke map；无静态 `@tauri-apps/*`（除允许的动态 import）。
3. UI 文案仍为英文（`docs/biz/ui-build-constraints.md`）。
4. 5 条 hash 路由行为与迁之前一致（打开、返回、创建、覆盖层收回）。
5. `npm test`（含 cargo lib + vitest）全绿。

---

## 8. 开放

| 项 | 状态 |
|----|------|
| 是否授权 P0 | ✅ Verified（用户 2026-08-28：开始落地） |
| `devUrl` 端口 | ✅ Verified：5173（官方 Vite 示例；`tauri.conf.json` `devUrl`） |
| 设计文档是否再归档进语料仓 | ❌ Unresolved（本文先落仓库） |
| 收岛与块内分层 | 见 `workbench-desktop-l2-in-block-layering-and-page-lift-tech-plan.md` |
