# GitHub Link Reading (❌ non-mechanizable — action triggers)

当用户消息里出现 `github.com/{owner}/{repo}/blob/{ref}/{path}`
或 `raw.githubusercontent.com/{owner}/{repo}/{ref}/{path}` 时：

1. **禁止直接拉取**：不得通过任何 HTTP(S) 方式拉取该 URL 的内容——
   无论是内置的网页抓取能力、浏览器、IDE 扩展，还是 MCP 工具，均不得使用。

2. **改走 gh api**：在可执行 `gh` 的 Shell 环境中运行：

   ```bash
   gh api "repos/{owner}/{repo}/contents/{path}?ref={ref}" \
     --jq '.content' | base64 -d
   ```

3. **URL 解析**：从 URL 直接解析 `owner` / `repo` / `ref` / `path`，无需映射表。

4. **失败处理**：

   | 状态码 | 处理 |
   |--------|------|
   | 404 | 告知路径不存在或无访问权限 |
   | 403 | 告知需执行 `gh auth login` 或申请仓库权限 |
   | 其他失败 | 不得静默回退到任何 URL/HTTP 方式重试 |
