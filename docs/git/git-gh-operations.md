# Git gh Operations

When a user message contains `github.com/{owner}/{repo}/blob/{ref}/{path}`
or `raw.githubusercontent.com/{owner}/{repo}/{ref}/{path}`:

1. **Do not fetch directly**: Do not retrieve that URL's content via any HTTP(S) method —
   not built-in web fetch, browser, IDE extension, or MCP tools.

2. **Use `gh api` instead**: In a Shell where `gh` is available, run:

   ```bash
   gh api "repos/{owner}/{repo}/contents/{path}?ref={ref}" \
     --jq '.content' | base64 -d
   ```

3. **URL parsing**: Parse `owner` / `repo` / `ref` / `path` directly from the URL. No mapping table.

4. **Failure handling**:

   | Status | Action |
   |--------|--------|
   | 404 | Tell the user the path does not exist or is not accessible |
   | 403 | Tell the user to run `gh auth login` or request repository access |
   | Other failures | Do not silently fall back to any URL/HTTP retry |
