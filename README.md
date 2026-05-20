# lulu-workbench

个人 AI 工作台。包含对话知识归档库（DDM 流程）与个人 AI skill 集合。

## 双仓库 Setup

程序与语料分属两个 Git 仓库：

1. **Clone 两个仓库**
   - `git clone https://github.com/lulufoo/lulu-workbench.git`
   - `git clone https://github.com/lulufoo/lulu-workbench-knowledge.git`
2. **配置语料路径** — 在 `lulu-workbench/meili.env` 中设置：
   ```ini
   KNOWLEDGE_CORPUS_DIR=/你的本地路径/lulu-workbench-knowledge
   KNOWLEDGE_CORPUS_GITHUB=https://github.com/lulufoo/lulu-workbench-knowledge/blob/main
   ```
3. **外置 Meilisearch** — 见 [docs/meilisearch-dev.md](docs/meilisearch-dev.md)（App / `server.py` 不启动 Meili）
4. **启动服务** — 在 workbench 根目录执行 `python3 server.py` 或 `cargo tauri dev`，浏览器打开 `http://localhost:8765`

**⇕ 同步**（↑ 提交变更 / ↓ 更新项目）针对 `KNOWLEDGE_CORPUS_DIR` 对应的 `lulu-workbench-knowledge` 仓库；workbench 程序本身的变更请在该程序仓库目录内自行 `git` 提交。
