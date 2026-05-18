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
3. **启动服务** — 在 workbench 根目录执行 `python3 server.py`，浏览器打开 `http://localhost:8765`
