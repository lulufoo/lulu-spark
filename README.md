# lulu-workbench

个人 AI 工作台。包含对话知识归档库（DDM 流程）与个人 AI skill 集合。

## 双仓库 Setup

程序与语料分属两个 Git 仓库：

1. **Clone 两个仓库**
   - `git clone https://github.com/lulufoo/lulu-workbench.git`
   - `git clone https://github.com/lulufoo/lulu-workbench-knowledge.git`
2. **配置路径** — 首次启动 App 打开 **设置**，或编辑 `~/.config/lulu-workbench/config.toml`：
   ```toml
   workbench_knowledge_root = "/你的本地路径/lulu-workbench-knowledge"
   knowledge_corpus_root = "/你的本地路径/Code"   # 沉淀知识库各 topic 仓库 clone 根目录
   cache_dir = "/你的本地路径/lulu-workbench/.cache"
   github_user_url = ""   # 可选；个人 GitHub 主页，如 https://github.com/lulufoo（结合 workbench_knowledge_root 目录名生成 blob 链接）
   meili_url = "http://localhost:7700"
   ```
   GitHub Token、Meili Master Key 在设置页写入 Keychain。
3. **外置 Meilisearch** — 见 [docs/meilisearch-dev.md](docs/meilisearch-dev.md)（App 不启动 Meili）
4. **启动 App** — 在 workbench 根目录执行 `cargo tauri dev`（或安装 release 后从启动台打开）。**无需** Python 或 `server.py`。

**⇕ 同步**（↑ 提交变更 / ↓ 更新项目）针对语料仓库；程序仓库变更在 `lulu-workbench` 目录内 `git` 提交。

## 开发

```bash
npm install          # 前端单测（vitest）
cd src-tauri && cargo test --lib -- --test-threads=1
cargo tauri dev      # 官方运行时
```
