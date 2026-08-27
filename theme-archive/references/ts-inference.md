# ts 推断 — 仅 GitHub URL

由 `theme-archive` Standalone 在确定 `COMMON_PATH` 前调用（若输入为 GitHub URL）。`ts` 与 `index.created_at`、raw/digest 的 `> 创建时间：` 一致（UTC+8，`YYYYMMDDHHMM`）。

## 适用条件

用户输入匹配其一：

- `github.com/{owner}/{repo}/blob/{ref}/{path}`
- `raw.githubusercontent.com/{owner}/{repo}/{ref}/{path}`

不匹配 → **不适用**；`theme-archive` 使用归档时刻作为 `ts`。

## 推断规则

1. 从 URL 解析 `owner` / `repo` / `ref` / `path`。
2. 用 `gh api` 查询该文件历史（禁止 HTTP 拉 blob 页；见项目 `docs/git/git-gh-operations.md`）。
3. 取**最后一次非 Delete 提交**的 `commit.committer.date`，转为 UTC+8 的 `ts`（分钟精度）。
4. 查询失败或为空 → `ts` = 归档时刻（UTC+8）。

## 命令模板

```bash
gh api "repos/{owner}/{repo}/commits?path={path}&sha={ref}&per_page=100" \
  --jq '[.[] | select(.commit.message | test("Delete"; "i") | not)] | last.commit.committer.date'
```

```bash
python3 -c "
from datetime import datetime, timezone, timedelta
iso = '''<jq输出>'''.strip()
dt = datetime.fromisoformat(iso.replace('Z', '+00:00'))
cn = dt.astimezone(timezone(timedelta(hours=8)))
print(cn.strftime('%Y%m%d%H%M'))
"
```
